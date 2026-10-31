//! Builds each workspace under `fixtures/` through crabshell.
//!
//! Each fixture runs a matrix of crate requests × consumer grants (see
//! [`Scenario`]): without requests, or with the precise requests in its
//! `requests.toml`; under the floor alone, its `sandbox.tight.toml`, or its
//! `sandbox.loose.toml`. A scenario that should work must succeed with every
//! build script printing the same directives as in a baseline build without
//! crabshell. A build script denied an optional env var or file often
//! degrades silently rather than failing, so success alone proves little.
//!
//! The crates under test may be untrusted, so every build, the baseline
//! included, runs inside an outer jail (the `jail` helper from the `probe`
//! package). It confines all of `cargo build`, including proc-macros, which
//! crabshell never sees: no network, and only the toolchain, the fixture and
//! the run's scratch dir are accessible (see [`Scratch`]). Crates are fetched
//! into a fresh `CARGO_HOME` before the jail starts, so no credentials exist
//! to steal.
//!
//! Build scripts can also echo their environment into build output (cc-rs
//! prints each compiler invocation with its full environment), so builds see
//! only a curated environment (see [`outer_env`]) carrying a fake token. The
//! baseline is expected to leak it only where a build script dumps its
//! environment; sandboxed builds must never leak it. Every build is also
//! checked for real secrets from the test's own environment.
//!
//! For trusted fixtures, an unjailed reference build must match the jailed
//! baseline, showing that the jail itself doesn't change behavior.
//!
//! The fixtures build with the nightly pinned in `fixtures/rust-toolchain.toml`
//! and fetch crates. Requests are injected through the `test-requests`
//! feature, and these tests are ignored without it: run them with
//! `cargo test --features test-requests`.

// Without `test-requests`, `check` only explains how to run the tests.
#![cfg_attr(not(feature = "test-requests"), allow(dead_code))]
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

/// Whether crates request permissions, standing in for their own declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Requests {
    /// No crate declares anything: the consumer must work out what's needed.
    None,
    /// Every crate requests exactly what it uses (`requests.toml`).
    Precise,
}

/// The consumer policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grants {
    /// No policy: only the default grant (crate-root read).
    Default,
    /// The union of the precise requests (`sandbox.tight.toml`).
    Tight,
    /// A broad policy, e.g. any env var (`sandbox.loose.toml`).
    Loose,
}

/// One cell of the requests × grants matrix.
#[derive(Debug, Clone, Copy)]
struct Scenario {
    requests: Requests,
    grants: Grants,
}

impl Scenario {
    const ALL: [Scenario; 6] = {
        use Grants::*;
        use Requests::*;
        [
            Scenario { requests: None, grants: Default },
            Scenario { requests: None, grants: Tight },
            Scenario { requests: None, grants: Loose },
            Scenario { requests: Precise, grants: Default },
            Scenario { requests: Precise, grants: Tight },
            Scenario { requests: Precise, grants: Loose },
        ]
    };

    fn name(&self) -> String {
        format!("{:?}-{:?}", self.requests, self.grants).to_lowercase()
    }

    /// The `CRABSHELL_CONFIG` for `fixture`. The default grant is an empty config.
    fn config(&self, fixture: &Fixture) -> PathBuf {
        match self.grants {
            Grants::Default => PathBuf::from("/dev/null"),
            Grants::Tight => fixture.dir().join("sandbox.tight.toml"),
            Grants::Loose => fixture.dir().join("sandbox.loose.toml"),
        }
    }

    /// Whether the build should work. Requests only narrow, so the default
    /// column works only for fixtures the default grant covers.
    fn succeeds(&self, fixture: &Fixture) -> bool {
        self.grants != Grants::Default || fixture.default_suffices
    }

    /// Whether `fixture`'s build should leak [`FAKE_TOKEN`]: only where a
    /// build script dumps its environment and nothing narrows the loose `*`.
    fn leaks(&self, fixture: &Fixture) -> bool {
        fixture.dumps_env && self.requests == Requests::None && self.grants == Grants::Loose
    }
}

/// A workspace under `fixtures/`.
struct Fixture {
    name: &'static str,
    /// Whether the fixture's pinned crates are reviewed, so they may also be
    /// built outside the jail.
    trusted: bool,
    /// Whether some build script prints its whole environment, so the
    /// baseline leaks [`FAKE_TOKEN`].
    dumps_env: bool,
    /// Whether the default grant alone covers every build script's needs.
    default_suffices: bool,
}

impl Fixture {
    fn dir(&self) -> PathBuf {
        e2e_tests().join(self.name)
    }
}

fn e2e_tests() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures")
}

/// A fake credential in every build's environment, to detect leaks.
///
/// Its name matches no grant in the fixtures, so a sandboxed build script can't
/// see it.
const FAKE_TOKEN: (&str, &str) = ("CRABSHELL_FAKE_TOKEN", "crabshell-fake-token-5c1f0e7a");

/// The scratch dir of one test run, recreated when the run starts:
///
/// ```text
/// e2e-run/
///   cargo-home/                  CARGO_HOME, filled by `cargo fetch`
///   home/                        HOME, holding only this distro's machine
///                                facts as the user config, if any
///   tmp/                         TMPDIR
///   <fixture>/baseline-target/   jailed, without crabshell
///   <fixture>/reference-target/  unjailed, without crabshell (trusted only)
///   <fixture>/<variant>-target/  jailed, through crabshell
/// ```
///
/// Each target dir is built once per run, so Cargo never reuses build-script
/// results from another policy.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn get() -> &'static Scratch {
        static SCRATCH: OnceLock<Scratch> = OnceLock::new();
        SCRATCH.get_or_init(|| {
            let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("e2e-run");
            if root.exists() {
                std::fs::remove_dir_all(&root).unwrap();
            }
            let scratch = Scratch { root };
            for dir in [scratch.cargo_home(), scratch.home(), scratch.tmp()] {
                std::fs::create_dir_all(dir).unwrap();
            }
            scratch.install_host_facts();
            scratch
        })
    }

    fn cargo_home(&self) -> PathBuf {
        self.root.join("cargo-home")
    }

    /// Install this distro's machine facts, if any, as the user config.
    ///
    /// `fixtures/hosts/<ID>.toml`, keyed by `/etc/os-release`'s `ID`, holds
    /// what built-in grants can't know about a distro, such as Amazon Linux's
    /// `pkg-config` wrapper. Keeping it out of the fixtures' policies keeps
    /// those portable.
    fn install_host_facts(&self) {
        let os_release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let id = os_release.lines().find_map(|line| line.strip_prefix("ID="));
        let Some(id) = id.map(|id| id.trim_matches('"')) else { return };
        let facts = e2e_tests().join("hosts").join(format!("{id}.toml"));
        if facts.is_file() {
            let dir = self.home().join(".config/crabshell");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::copy(&facts, dir.join("config.toml")).unwrap();
        }
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn tmp(&self) -> PathBuf {
        self.root.join("tmp")
    }

    fn target(&self, fixture: &Fixture, name: &str) -> PathBuf {
        self.root.join(fixture.name).join(format!("{name}-target"))
    }
}

/// The pinned toolchain's binaries, called directly so rustup stays out of
/// the jail.
struct Toolchain {
    cargo: PathBuf,
    rustc: PathBuf,
}

impl Toolchain {
    fn get() -> &'static Toolchain {
        static TOOLCHAIN: OnceLock<Toolchain> = OnceLock::new();
        TOOLCHAIN.get_or_init(|| Toolchain { cargo: which("cargo"), rustc: which("rustc") })
    }

    /// The toolchain's root dir, holding `bin/` and `lib/`.
    fn root(&self) -> &Path {
        self.cargo.parent().and_then(Path::parent).unwrap()
    }
}

/// Ask rustup for `tool` in the toolchain pinned for `fixtures/`.
fn which(tool: &str) -> PathBuf {
    let output = Command::new("rustup")
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("RUSTUP_HOME", rustup_home())
        .args(["which", tool])
        .current_dir(e2e_tests())
        .output()
        .unwrap();
    assert!(output.status.success(), "rustup which {tool}:\n{}", output_stderr(&output));
    PathBuf::from(String::from_utf8(output.stdout).unwrap().trim())
}

fn rustup_home() -> OsString {
    std::env::var_os("RUSTUP_HOME").unwrap_or_else(|| {
        PathBuf::from(std::env::var_os("HOME").expect("HOME is set")).join(".rustup").into()
    })
}

/// The only variables Cargo sees, for every build alike.
///
/// `PATH` is kept for the tools build scripts run (`cc`, `pkg-config`), and
/// `RUSTUP_HOME` for policies that name the toolchain through it.
fn outer_env() -> Vec<(&'static str, OsString)> {
    let scratch = Scratch::get();
    vec![
        ("PATH", std::env::var_os("PATH").unwrap_or_default()),
        ("CARGO_HOME", scratch.cargo_home().into()),
        ("RUSTUP_HOME", rustup_home()),
        ("HOME", scratch.home().into()),
        ("TMPDIR", scratch.tmp().into()),
        ("RUSTC", Toolchain::get().rustc.clone().into()),
        (FAKE_TOKEN.0, FAKE_TOKEN.1.into()),
    ]
}

/// Download `fixture`'s locked dependencies into the scratch `CARGO_HOME`.
///
/// This runs outside the jail, since it needs the network; fetching runs no
/// crate code.
fn fetch(fixture: &Fixture) {
    let output = Command::new(&Toolchain::get().cargo)
        .env_clear()
        .envs(outer_env())
        .args(["fetch", "--locked"])
        .current_dir(fixture.dir())
        .output()
        .unwrap();
    assert!(output.status.success(), "{}: fetch failed:\n{}", fixture.name, output_stderr(&output));
}

/// How a build is confined.
enum Mode<'a> {
    /// Outside the jail, without crabshell. Only for trusted fixtures.
    Reference,
    /// In the jail, without crabshell.
    Baseline,
    /// In the jail, through crabshell.
    Sandboxed(&'a Scenario),
}

/// A finished build of a fixture.
struct Build {
    output: Output,
    /// Whether [`FAKE_TOKEN`]'s value appears in the build's output.
    leaks_fake_token: bool,
}

/// Build `fixture` into `target_dir` in the given mode.
fn build(fixture: &Fixture, target_dir: &Path, mode: &Mode) -> Build {
    let toolchain = Toolchain::get();
    // Landlock rules attach to existing files, so the jail can only grant a
    // target dir that already exists.
    std::fs::create_dir_all(target_dir).unwrap();
    let mut cmd = match mode {
        Mode::Reference => Command::new(&toolchain.cargo),
        Mode::Baseline | Mode::Sandboxed(_) => jail(fixture, target_dir),
    };
    cmd.env_clear().envs(outer_env());
    if let Mode::Sandboxed(scenario) = mode {
        let runner = format!("host.runner=[{:?}]", env!("CARGO_BIN_EXE_crabshell"));
        cmd.args(["-Zhost-config", "-Ztarget-applies-to-host", "--config", &runner])
            .env("CRABSHELL_CONFIG", scenario.config(fixture));
        if scenario.requests == Requests::Precise {
            cmd.env("CRABSHELL_REQUESTS", fixture.dir().join("requests.toml"));
        }
    }
    let output = cmd
        .args(["build", "--frozen", "--target-dir"])
        .arg(target_dir)
        .current_dir(fixture.dir())
        .output()
        .unwrap();
    let logs = logs(target_dir, &output);
    assert_no_secrets(target_dir, &logs);
    let leaks_fake_token = logs.iter().any(|log| log.contains(FAKE_TOKEN.1));
    Build { output, leaks_fake_token }
}

/// A `jail` command, ready for cargo's arguments, that confines a build of
/// `fixture` into `target_dir`.
///
/// The fixture, the fetched crates and `HOME` are read-only, so a build can't
/// tamper with what later builds in the run use.
fn jail(fixture: &Fixture, target_dir: &Path) -> Command {
    let scratch = Scratch::get();
    let toolchain = Toolchain::get();
    let crabshell = PathBuf::from(env!("CARGO_BIN_EXE_crabshell"));
    let jail = crabshell.with_file_name("jail");
    assert!(jail.is_file(), "missing {}: run `cargo test` from the workspace root", jail.display());

    let read = [
        PathBuf::from("/usr"),
        PathBuf::from("/lib*"),
        toolchain.root().to_path_buf(),
        scratch.cargo_home(),
        scratch.home(),
        fixture.dir(),
        crabshell,
    ];
    let write = [target_dir.to_path_buf(), scratch.tmp(), PathBuf::from("/dev/null")];

    let mut cmd = Command::new(jail);
    // Policies expand globs such as `/lib*` by listing the parent dir. Listing
    // reveals file names, not contents.
    cmd.args(["--list", "/"]);
    for path in read {
        cmd.arg("--read").arg(path);
    }
    // `--write` is write-only: cargo also reads back what it writes.
    for path in write {
        cmd.arg("--read").arg(&path).arg("--write").arg(path);
    }
    cmd.arg("--").arg(&toolchain.cargo);
    cmd
}

/// Variables of the test's own environment that look like credentials.
fn secrets() -> Vec<(String, String)> {
    const MARKERS: &[&str] =
        &["TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "SESSION", "AUTH", "KEY"];
    std::env::vars()
        .filter(|(key, value)| {
            let key = key.to_uppercase();
            value.len() >= 8 && MARKERS.iter().any(|marker| key.contains(marker))
        })
        .collect()
}

/// Cargo's output and every build script's saved stdout and stderr.
fn logs(target_dir: &Path, output: &Output) -> Vec<String> {
    let mut logs = vec![
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ];
    for (_, run) in run_dirs(target_dir) {
        for log in ["stdout", "stderr"] {
            logs.extend(std::fs::read_to_string(run.join(log)));
        }
    }
    logs
}

/// Fail if a secret from the test's own environment reached `logs`.
///
/// Only the variable's name is reported, never its value.
fn assert_no_secrets(target_dir: &Path, logs: &[String]) {
    for (key, value) in secrets() {
        assert!(
            !logs.iter().any(|log| log.contains(&value)),
            "{}: the value of `{key}` leaked into build output",
            target_dir.display()
        );
    }
}

/// Every build script's `run` dir, with the package it belongs to.
fn run_dirs(target_dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(pkgs) = std::fs::read_dir(target_dir.join("debug/build")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for pkg in pkgs {
        let pkg = pkg.unwrap().path();
        let name = pkg.file_name().unwrap().to_string_lossy().into_owned();
        for unit in std::fs::read_dir(&pkg).unwrap() {
            let run = unit.unwrap().path().join("run");
            if run.is_dir() {
                out.push((name.clone(), run));
            }
        }
    }
    out
}

/// Every build script's `cargo:` directives, by package.
///
/// Other output is diagnostics, e.g. cc-rs prints each compiler invocation
/// with its full environment, which differs by design under a sandbox.
fn directives(target_dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (name, run) in run_dirs(target_dir) {
        if let Ok(text) = std::fs::read_to_string(run.join("stdout")) {
            let lines: &mut String = out.entry(name).or_default();
            for line in text.lines().filter(|line| line.starts_with("cargo:")) {
                lines.push_str(line);
                lines.push('\n');
            }
        }
    }
    out
}

/// Build `fixture` in every [`Scenario`] and check each outcome.
#[cfg(not(feature = "test-requests"))]
fn check(fixture: &Fixture) {
    panic!("{}: the precise-requests scenarios need `--features test-requests`", fixture.name);
}

/// Build `fixture` in every [`Scenario`] and check each outcome.
#[cfg(feature = "test-requests")]
fn check(fixture: &Fixture) {
    let name = fixture.name;
    let scratch = Scratch::get();
    fetch(fixture);

    let baseline_dir = scratch.target(fixture, "baseline");
    let baseline = build(fixture, &baseline_dir, &Mode::Baseline);
    assert!(baseline.output.status.success(), "{name}: baseline failed:\n{}", stderr(&baseline));
    assert_eq!(
        baseline.leaks_fake_token, fixture.dumps_env,
        "{name}: baseline leaks the fake token"
    );
    let expected = directives(&baseline_dir);
    assert_requests_cover(fixture, &expected);

    if fixture.trusted {
        let reference_dir = scratch.target(fixture, "reference");
        let reference = build(fixture, &reference_dir, &Mode::Reference);
        assert!(
            reference.output.status.success(),
            "{name}: reference failed:\n{}",
            stderr(&reference)
        );
        assert_eq!(directives(&reference_dir), expected, "{name}: the jail changes the output");
    }

    for scenario in &Scenario::ALL {
        let label = format!("{name}/{}", scenario.name());
        let target_dir = scratch.target(fixture, &scenario.name());
        let build = build(fixture, &target_dir, &Mode::Sandboxed(scenario));
        assert_eq!(build.leaks_fake_token, scenario.leaks(fixture), "{label}: fake token leak");
        if scenario.succeeds(fixture) {
            assert!(build.output.status.success(), "{label}: failed:\n{}", stderr(&build));
            assert_eq!(directives(&target_dir), expected, "{label}: output differs");
        } else {
            assert!(!build.output.status.success(), "{label}: expected the build to fail");
        }
    }
}

/// Fail unless `requests.toml` lists every package whose build script runs.
///
/// An unlisted package would keep the consumer's full grant, hiding a missing
/// request.
fn assert_requests_cover(fixture: &Fixture, scripts: &BTreeMap<String, String>) {
    #[derive(serde::Deserialize)]
    struct Requests {
        package: BTreeMap<String, toml::Value>,
    }
    let text = std::fs::read_to_string(fixture.dir().join("requests.toml")).unwrap();
    let requests: Requests = toml::from_str(&text).unwrap();
    for pkg in scripts.keys() {
        assert!(
            requests.package.contains_key(pkg),
            "{}: requests.toml misses `{pkg}`",
            fixture.name
        );
    }
}

fn stderr(build: &Build) -> String {
    output_stderr(&build.output)
}

fn output_stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
#[cfg_attr(not(feature = "test-requests"), ignore = "needs `--features test-requests`")]
fn cfg_only() {
    // Its scripts only read TARGET and CARGO_CFG_*, which the default covers.
    check(&Fixture { name: "cfg-only", trusted: true, dumps_env: false, default_suffices: true });
}

#[test]
#[cfg_attr(not(feature = "test-requests"), ignore = "needs `--features test-requests`")]
fn rustc_probe() {
    check(&Fixture {
        name: "rustc-probe",
        trusted: true,
        dumps_env: false,
        default_suffices: false,
    });
}

#[test]
#[cfg_attr(not(feature = "test-requests"), ignore = "needs `--features test-requests`")]
fn system_lib() {
    // libz-sys prints a cc-rs compiler command, whose `Debug` lists the whole
    // environment.
    check(&Fixture { name: "system-lib", trusted: true, dumps_env: true, default_suffices: false });
}
