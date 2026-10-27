//! Builds each `e2e-tests/` workspace through crabshell.
//!
//! A sandboxed build passes when it succeeds and every build script printed
//! the same directives as in an unsandboxed baseline build. A build script
//! denied an optional env var or file often degrades silently rather than
//! failing, so success alone proves little.
//!
//! These tests need nightly Cargo and fetch crates, so they are ignored by
//! default. Run them with `cargo test -- --ignored`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What a policy variant is expected to do to the build.
#[derive(Debug, PartialEq, Eq)]
enum Expect {
    /// The build succeeds with the baseline's build-script output.
    Same,
    /// The build succeeds, but some build script's output differs: a silent
    /// degradation.
    Differs,
    /// The build fails.
    Fails,
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../e2e-tests").join(name)
}

/// Build `fixture` into `target_dir`, through crabshell under `sandbox` if set.
///
/// The target dir is recreated first: Cargo doesn't rerun a build script when
/// only the policy changed.
fn build(fixture: &Path, target_dir: &Path, sandbox: Option<&str>) -> Output {
    if target_dir.exists() {
        std::fs::remove_dir_all(target_dir).unwrap();
    }
    let mut cmd = Command::new("cargo");
    cmd.arg("+nightly");
    if let Some(sandbox) = sandbox {
        let runner = format!("host.runner=[{:?}]", env!("CARGO_BIN_EXE_crabshell"));
        cmd.args(["-Zhost-config", "-Ztarget-applies-to-host", "--config", &runner])
            .env("CRABSHELL_CONFIG", fixture.join(sandbox));
    }
    cmd.args(["build", "--locked", "--target-dir"])
        .arg(target_dir)
        .current_dir(fixture)
        .output()
        .unwrap()
}

/// Every build script's `cargo:` directives, by package.
///
/// Other output is diagnostics, e.g. cc-rs prints each compiler invocation
/// with its full environment, which differs by design under a sandbox.
fn directives(target_dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for pkg in std::fs::read_dir(target_dir.join("debug/build")).unwrap() {
        let pkg = pkg.unwrap().path();
        for unit in std::fs::read_dir(&pkg).unwrap() {
            let stdout = unit.unwrap().path().join("run/stdout");
            if let Ok(text) = std::fs::read_to_string(&stdout) {
                let name = pkg.file_name().unwrap().to_string_lossy().into_owned();
                let lines = out.entry(name).or_insert_with(String::new);
                for line in text.lines().filter(|line| line.starts_with("cargo:")) {
                    lines.push_str(line);
                    lines.push('\n');
                }
            }
        }
    }
    out
}

/// Build `name` under each `(sandbox, expect)` variant and check the outcome.
fn check(name: &str, variants: &[(&str, Expect)]) {
    let fixture = fixture(name);
    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("e2e-tests").join(name);

    let baseline_dir = scratch.join("baseline");
    let baseline = build(&fixture, &baseline_dir, None);
    assert!(baseline.status.success(), "{name}: baseline failed:\n{}", stderr(&baseline));
    let expected = directives(&baseline_dir);

    for (sandbox, expect) in variants {
        let target_dir = scratch.join(sandbox);
        let output = build(&fixture, &target_dir, Some(sandbox));
        match expect {
            Expect::Fails => {
                assert!(!output.status.success(), "{name}/{sandbox}: expected the build to fail");
            }
            Expect::Same => {
                assert!(output.status.success(), "{name}/{sandbox}: failed:\n{}", stderr(&output));
                assert_eq!(directives(&target_dir), expected, "{name}/{sandbox}: output differs");
            }
            Expect::Differs => {
                assert!(output.status.success(), "{name}/{sandbox}: failed:\n{}", stderr(&output));
                assert_ne!(directives(&target_dir), expected, "{name}/{sandbox}: output matches");
            }
        }
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
#[ignore = "needs nightly Cargo and network"]
fn cfg_only() {
    check(
        "cfg-only",
        &[
            ("sandbox.toml", Expect::Same),
            ("sandbox.category.toml", Expect::Same),
            ("sandbox.floor.toml", Expect::Fails),
        ],
    );
}

#[test]
#[ignore = "needs nightly Cargo and network"]
fn rustc_probe() {
    check(
        "rustc-probe",
        &[
            ("sandbox.toml", Expect::Same),
            ("sandbox.category.toml", Expect::Same),
            ("sandbox.limits.toml", Expect::Fails),
            ("sandbox.floor.toml", Expect::Fails),
        ],
    );
}

#[test]
#[ignore = "needs nightly Cargo and network"]
fn system_lib() {
    check(
        "system-lib",
        &[
            ("sandbox.toml", Expect::Same),
            ("sandbox.category.toml", Expect::Same),
            ("sandbox.no-linker.toml", Expect::Differs),
            ("sandbox.floor.toml", Expect::Fails),
        ],
    );
}

#[test]
#[ignore = "needs nightly Cargo and network"]
fn declared() {
    check(
        "declared",
        &[
            ("sandbox.toml", Expect::Same),
            ("sandbox.narrow.toml", Expect::Differs),
            ("sandbox.floor.toml", Expect::Fails),
        ],
    );
}
