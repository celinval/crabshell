//! End-to-end checks that drive the `crabshell` binary the way Cargo does.
//!
//! Cargo describes the package through the build script's environment, so these
//! tests set `CARGO_PKG_NAME`, `CARGO_MANIFEST_DIR`, `CARGO_HOME`, and
//! `CRABSHELL_CONFIG` themselves. Placing the manifest inside or outside
//! `CARGO_HOME` is what makes a package external or local. The build script
//! under test is the `probe` fixture, which reports its verdict through its
//! exit status.
//!
//! Both binaries land in the same directory, so `probe` is resolved as a
//! sibling of `crabshell`. Run these with `cargo test` from the workspace root
//! so that both have been built.
//!
//! `probe` is dynamically linked like any build script, so these tests also
//! cover the floor's runtime libraries: no policy here grants them.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::OnceLock;

/// The host triple, which Cargo passes to build scripts as `HOST`.
fn host() -> &'static str {
    static HOST: OnceLock<String> = OnceLock::new();
    HOST.get_or_init(|| {
        let output = Command::new("rustc").arg("-vV").output().unwrap();
        let info = String::from_utf8(output.stdout).unwrap();
        let host = info.lines().find_map(|line| line.strip_prefix("host: "));
        host.expect("rustc -vV prints the host").to_string()
    })
}

fn crabshell() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_crabshell"))
}

fn probe() -> PathBuf {
    let probe = crabshell().with_file_name("probe");
    assert!(
        probe.is_file(),
        "missing {}: run `cargo test` from the workspace root",
        probe.display()
    );
    probe
}

/// Where a package's manifest sits, which decides its category.
enum Origin {
    /// Outside `CARGO_HOME`, like a workspace member or path dependency.
    Local,
    /// Unpacked under `CARGO_HOME`, like a registry dependency.
    External,
}

/// A temporary build layout: a `sandbox.toml`, a fake `CARGO_HOME`, and an
/// `OUT_DIR`.
struct Build {
    dir: tempfile::TempDir,
}

impl Build {
    fn new(sandbox: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sandbox.toml"), sandbox).unwrap();
        std::fs::create_dir(dir.path().join("out")).unwrap();
        Self { dir }
    }

    /// An `XDG_CONFIG_HOME`, so the real user config can't affect tests.
    fn xdg(&self) -> PathBuf {
        self.dir.path().join("xdg")
    }

    /// Write the user's own config, which caps the project's policy.
    fn user_config(&self, text: &str) {
        let dir = self.xdg().join("crabshell");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.toml"), text).unwrap();
    }

    fn out_dir(&self) -> PathBuf {
        self.dir.path().join("out")
    }

    /// Create a file outside every granted path, like a credential in `$HOME`.
    fn secret(&self) -> PathBuf {
        let path = self.dir.path().join("secret");
        std::fs::write(&path, "hunter2").unwrap();
        path
    }

    fn cargo_home(&self) -> PathBuf {
        self.dir.path().join("cargo-home")
    }

    fn manifest_dir(&self, pkg: &str, origin: &Origin) -> PathBuf {
        let dir = match origin {
            Origin::Local => self.dir.path().join(pkg),
            Origin::External => self.cargo_home().join("registry/src/index").join(pkg),
        };
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Run a probe check through crabshell and report whether it passed.
    fn check(&self, pkg: &str, origin: Origin, env: &[(&str, &str)], args: &[&str]) -> bool {
        self.run(pkg, origin, env, args).status.success()
    }

    fn run(&self, pkg: &str, origin: Origin, env: &[(&str, &str)], args: &[&str]) -> Output {
        let mut cmd = Command::new(crabshell());
        cmd.arg(probe())
            .args(args)
            .env("CARGO_PKG_NAME", pkg)
            .env("CARGO_MANIFEST_DIR", self.manifest_dir(pkg, &origin))
            .env("CARGO_HOME", self.cargo_home())
            .env("HOST", host())
            .env("OUT_DIR", self.out_dir())
            .env("XDG_CONFIG_HOME", self.xdg())
            .env("CRABSHELL_CONFIG", self.dir.path().join("sandbox.toml"));
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.output().unwrap()
    }
}

const GRANTED: [(&str, &str); 2] = [("CRABSHELL_GRANTED", "yes"), ("CRABSHELL_UNGRANTED", "yes")];

#[test]
fn propagates_only_granted_env() {
    let build = Build::new("[external]\nenv = [\"CRABSHELL_GRANTED\"]\n");

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_UNGRANTED"]));
}

#[test]
fn category_follows_the_manifest_location() {
    let build = Build::new(
        r#"
        [local]
        env = ["CRABSHELL_GRANTED"]

        [external]
        env = []
        "#,
    );

    assert!(build.check("mine", Origin::Local, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn named_package_overrides_its_category() {
    let build = Build::new(
        r#"
        [external]
        env = []

        [package.special]
        env = ["CRABSHELL_GRANTED"]
        "#,
    );

    assert!(build.check(
        "special",
        Origin::External,
        &GRANTED,
        &["env-present", "CRABSHELL_GRANTED"]
    ));
    assert!(build.check("other", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn limits_clamp_a_category_grant() {
    let build = Build::new(
        r#"
        [limits]
        env = []

        [external]
        env = ["CRABSHELL_GRANTED"]
        "#,
    );

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn out_dir_is_always_granted() {
    let build = Build::new("[external]\nenv = []\n");

    assert!(build.check("dep", Origin::External, &[], &["env-present", "OUT_DIR"]));
}

#[test]
fn a_rejected_config_fails_the_build() {
    let build = Build::new("[package.\"*\"]\nenv = []\n");

    assert!(!build.check("dep", Origin::External, &GRANTED, &["env-absent", "ANYTHING"]));
}

#[test]
fn without_a_config_nothing_is_granted() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = dir.path().join("pkg");
    std::fs::create_dir_all(&manifest).unwrap();

    let status = Command::new(crabshell())
        .arg(probe())
        .args(["env-absent", "CRABSHELL_GRANTED"])
        .env("CARGO_PKG_NAME", "dep")
        .env("CARGO_MANIFEST_DIR", &manifest)
        .env("HOST", host())
        .env("XDG_CONFIG_HOME", dir.path().join("xdg"))
        .env_remove("CRABSHELL_CONFIG")
        .env("CRABSHELL_GRANTED", "yes")
        .status()
        .unwrap();

    assert!(status.success());
}

/// crabshell never searches near a package for a policy: a dependency could
/// ship a `sandbox.toml` granting itself anything.
#[test]
fn a_sandbox_toml_near_the_package_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = "[local]\nenv = [\"CRABSHELL_GRANTED\"]\n";
    let manifest = dir.path().join("nested/pkg");
    std::fs::create_dir_all(&manifest).unwrap();
    std::fs::write(manifest.join("sandbox.toml"), sandbox).unwrap();
    std::fs::write(dir.path().join("sandbox.toml"), sandbox).unwrap();

    let output = Command::new(crabshell())
        .arg(probe())
        .args(["env-absent", "CRABSHELL_GRANTED"])
        .env("CARGO_PKG_NAME", "mine")
        .env("CARGO_MANIFEST_DIR", &manifest)
        .env("CARGO_HOME", dir.path().join("cargo-home"))
        .env("HOST", host())
        .env("XDG_CONFIG_HOME", dir.path().join("xdg"))
        .env_remove("CRABSHELL_CONFIG")
        .env("CRABSHELL_GRANTED", "yes")
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("CRABSHELL_CONFIG is not set"));
}

#[test]
fn ungranted_paths_are_unreadable() {
    let build = Build::new("[external]\n");
    let secret = build.secret();

    assert!(build.check("dep", Origin::External, &[], &["unreadable", secret.to_str().unwrap()]));
}

#[test]
fn read_grants_are_read_only() {
    let build = Build::new("");
    let secret = build.secret();
    let dir = build.dir.path().to_str().unwrap();
    std::fs::write(
        build.dir.path().join("sandbox.toml"),
        format!("[external]\nread = [\"{dir}\"]\n"),
    )
    .unwrap();

    assert!(build.check("dep", Origin::External, &[], &["readable", secret.to_str().unwrap()]));
    assert!(build.check("dep", Origin::External, &[], &["unwritable", dir]));
}

#[test]
fn the_crate_root_is_readable_and_out_dir_writable() {
    let build = Build::new("[external]\n");
    let manifest = build.manifest_dir("dep", &Origin::External);
    std::fs::write(manifest.join("Cargo.toml"), "").unwrap();
    let out_dir = build.out_dir();

    assert!(build.check(
        "dep",
        Origin::External,
        &[],
        &["readable", manifest.join("Cargo.toml").to_str().unwrap()]
    ));
    assert!(build.check("dep", Origin::External, &[], &["unwritable", manifest.to_str().unwrap()]));
    assert!(build.check("dep", Origin::External, &[], &["writable", out_dir.to_str().unwrap()]));
}

#[test]
fn only_granted_programs_can_run() {
    let build = Build::new("[external]\nrun = [\"true\"]\n");
    let path = std::env::var("PATH").unwrap();
    let env = [("PATH", path.as_str())];

    assert!(build.check("dep", Origin::External, &env, &["can-exec", "/usr/bin/true"]));
    assert!(build.check("dep", Origin::External, &env, &["cannot-exec", "/usr/bin/false"]));
}

#[test]
fn network_is_denied_unless_granted() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let build = Build::new(
        r#"
        [external]

        [package.online]
        network = true
        "#,
    );

    assert!(build.check("dep", Origin::External, &[], &["no-connect", &addr]));
    assert!(build.check("online", Origin::External, &[], &["connect", &addr]));
    // UDP too, which Landlock's TCP rules alone don't cover.
    assert!(build.check("dep", Origin::External, &[], &["no-udp", "127.0.0.1:0"]));
    assert!(build.check("online", Origin::External, &[], &["udp", "127.0.0.1:0"]));
}

/// Paths to a socket that bypass a `socket`-only filter are denied too.
#[cfg(target_os = "linux")]
#[test]
fn network_denial_covers_io_uring_and_x32() {
    let build = Build::new("[external]\n");

    assert!(build.check("dep", Origin::External, &[], &["no-io-uring", "_"]));
    assert!(build.check("dep", Origin::External, &[], &["no-x32-socket", "_"]));
}

#[test]
fn granting_network_to_external_warns() {
    let build = Build::new("[external]\nnetwork = true\n");

    let output = build.run("dep", Origin::External, &[], &["env-absent", "ANYTHING"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("cargo::warning=crabshell: [external] grants network")
    );
}

#[test]
fn a_declaration_tightens_the_grant() {
    let build = Build::new("[external]\nenv = [\"CRABSHELL_GRANTED\", \"CRABSHELL_UNGRANTED\"]\n");
    let manifest = build.manifest_dir("dep", &Origin::External);
    std::fs::write(
        manifest.join("Cargo.toml"),
        "[package]\nname = \"dep\"\n\n[package.metadata.build-rs]\nenv = [\"CRABSHELL_GRANTED\"]\n",
    )
    .unwrap();

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_UNGRANTED"]));
}

#[test]
fn the_floor_grants_dev_null_and_a_private_tmpdir() {
    let build = Build::new("[external]\n");

    assert!(build.check("dep", Origin::External, &[], &["write-file", "/dev/null"]));
    assert!(build.check("dep", Origin::External, &[], &["private-tmp", "/tmp"]));
    assert!(build.check("dep", Origin::External, &[], &["unwritable", "/tmp"]));
}

#[test]
fn out_dir_and_tmpdir_are_write_only() {
    let build = Build::new("[external]\n");
    let out_dir = build.out_dir();
    let tmp = build.dir.path().join("crabshell-tmp");

    assert!(build.check("dep", Origin::External, &[], &["write-only", out_dir.to_str().unwrap()]));
    assert!(build.check("dep", Origin::External, &[], &["write-only", tmp.to_str().unwrap()]));
}

#[test]
fn read_grants_reach_out_dir_and_tmpdir() {
    let build = Build::new("[external]\nread = [\":out_dir\", \":tmp_dir\"]\n");
    let out_dir = build.out_dir();
    let tmp = build.dir.path().join("crabshell-tmp");

    assert!(build.check("dep", Origin::External, &[], &["read-back", out_dir.to_str().unwrap()]));
    assert!(build.check("dep", Origin::External, &[], &["read-back", tmp.to_str().unwrap()]));
}

#[test]
fn write_grants_no_read_access() {
    let build = Build::new("[external]\nwrite = [\":manifest_dir\"]\nread = []\n");
    let manifest = build.manifest_dir("dep", &Origin::External);

    assert!(build.check("dep", Origin::External, &[], &["write-only", manifest.to_str().unwrap()]));
}

/// A requests file stands in for a crate's `[package.metadata.build-rs]`.
#[cfg(feature = "test-requests")]
#[test]
fn injected_requests_tighten_the_grant() {
    let build = Build::new("[external]\nenv = [\"CRABSHELL_GRANTED\", \"CRABSHELL_UNGRANTED\"]\n");
    let requests = build.dir.path().join("requests.toml");
    std::fs::write(&requests, "[package.dep]\nenv = [\"CRABSHELL_GRANTED\"]\n").unwrap();
    let mut env = GRANTED.to_vec();
    env.push(("CRABSHELL_REQUESTS", requests.to_str().unwrap()));

    assert!(build.check("dep", Origin::External, &env, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &env, &["env-absent", "CRABSHELL_UNGRANTED"]));
    // Packages the file doesn't list keep the consumer's grant.
    assert!(build.check("other", Origin::External, &env, &["env-present", "CRABSHELL_UNGRANTED"]));
}

#[cfg(not(feature = "test-requests"))]
#[test]
fn a_requests_file_needs_the_test_feature() {
    let build = Build::new("");
    let env = [("CRABSHELL_REQUESTS", "/dev/null")];

    assert!(!build.check("dep", Origin::External, &env, &["env-absent", "ANYTHING"]));
}

#[test]
fn variables_expand_in_paths() {
    let build = Build::new("");
    let secret = build.secret();
    let dir = build.dir.path().to_str().unwrap();
    let sandbox = "[external]\nread = [\":data/secret\"]\n\n[variables]\ndata = { env = \"CRABSHELL_DATA\" }\n";
    std::fs::write(build.dir.path().join("sandbox.toml"), sandbox).unwrap();
    let secret = secret.to_str().unwrap();

    assert!(build.check(
        "dep",
        Origin::External,
        &[("CRABSHELL_DATA", dir)],
        &["readable", secret]
    ));
    // Unset, the variable is undefined and the grant is dropped, not an error.
    assert!(build.check("dep", Origin::External, &[], &["unreadable", secret]));
}

#[test]
fn a_path_with_an_env_var_fails_the_build() {
    let build = Build::new("[external]\nread = [\"$HOME/.ssh\"]\n");

    assert!(!build.check("dep", Origin::External, &[], &["env-absent", "ANYTHING"]));
}

/// Entries compare after expanding, so an absolute `[limits]` cap applies to a
/// grant written with a variable.
#[test]
fn limits_compare_paths_after_expanding_variables() {
    let build = Build::new("");
    let manifest = build.manifest_dir("dep", &Origin::External);
    std::fs::write(manifest.join("Cargo.toml"), "").unwrap();
    let file = manifest.join("Cargo.toml");
    let file = file.to_str().unwrap();
    let write_policy = |cap: &str| {
        let policy = format!("[limits]\nread = [\"{cap}\"]\n");
        std::fs::write(build.dir.path().join("sandbox.toml"), policy).unwrap();
    };

    // A cap above the crate root keeps the default `:manifest_dir` grant.
    write_policy(build.cargo_home().to_str().unwrap());
    assert!(build.check("dep", Origin::External, &[], &["readable", file]));
    // A cap elsewhere removes it.
    write_policy("/usr");
    assert!(build.check("dep", Origin::External, &[], &["unreadable", file]));
}

#[test]
fn the_user_config_caps_the_project_policy() {
    let build = Build::new("[external]\nenv = [\"CRABSHELL_GRANTED\", \"CRABSHELL_UNGRANTED\"]\n");
    build.user_config("[external]\nenv = [\"CRABSHELL_GRANTED\"]\n");

    let output =
        build.run("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_UNGRANTED"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("narrows dep's policy: env CRABSHELL_UNGRANTED"), "{stdout}");
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
}

/// Fields the user config doesn't name stay as the project sets them.
#[test]
fn the_user_config_leaves_unnamed_fields_alone() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let build = Build::new("[external]\nenv = [\"CRABSHELL_GRANTED\"]\nnetwork = true\n");
    build.user_config("[limits]\nnetwork = false\n");

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["no-connect", &addr]));
}

/// The user config can't widen the project's policy.
#[test]
fn the_user_config_cannot_widen() {
    let build = Build::new("[external]\nenv = []\n");
    build.user_config("[external]\nenv = [\"CRABSHELL_GRANTED\"]\n");

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_GRANTED"]));
}
