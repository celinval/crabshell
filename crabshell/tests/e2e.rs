//! End-to-end checks that drive the `crabshell` binary the way Cargo does.
//!
//! Cargo passes the package through the build script's environment, so these
//! tests set `CARGO_PKG_NAME` and `CARGO_MANIFEST_DIR` and let crabshell
//! discover a `sandbox.toml` written into a temporary directory. The build
//! script under test is the `probe` fixture binary, which reports its verdict
//! through its exit status.
//!
//! Both binaries land in the same directory, so `probe` is resolved as a
//! sibling of `crabshell`. Run these with `cargo test` from the workspace root
//! so that both have been built.

use std::path::PathBuf;
use std::process::Command;

fn crabshell() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_crabshell"))
}

fn probe() -> PathBuf {
    let probe = crabshell().with_file_name("probe");
    assert!(probe.is_file(), "missing {}: run `cargo test` from the workspace root", probe.display());
    probe
}

/// Run a probe check through crabshell against `sandbox`, and report whether it
/// passed. `env` is placed in crabshell's own environment, standing in for what
/// Cargo would set for the build script.
fn check(sandbox: &str, pkg: &str, env: &[(&str, &str)], args: &[&str]) -> bool {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("sandbox.toml"), sandbox).unwrap();

    let mut cmd = Command::new(crabshell());
    cmd.arg(probe())
        .args(args)
        .env("CARGO_PKG_NAME", pkg)
        .env("CARGO_MANIFEST_DIR", dir.path());
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.status().unwrap().success()
}

#[test]
fn propagates_only_granted_env() {
    let sandbox = r#"
        [package."*"]
        env = ["CRABSHELL_GRANTED"]
    "#;
    let env = [("CRABSHELL_GRANTED", "yes"), ("CRABSHELL_UNGRANTED", "yes")];

    assert!(check(sandbox, "dep", &env, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(check(sandbox, "dep", &env, &["env-absent", "CRABSHELL_UNGRANTED"]));
}

#[test]
fn named_package_grant_overrides_catch_all() {
    let sandbox = r#"
        [package."*"]
        env = []

        [package.special]
        env = ["CRABSHELL_GRANTED"]
    "#;
    let env = [("CRABSHELL_GRANTED", "yes")];

    assert!(check(sandbox, "special", &env, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(check(sandbox, "other", &env, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn limits_clamp_a_package_grant() {
    let sandbox = r#"
        [limits]
        env = []

        [package."*"]
        env = ["CRABSHELL_GRANTED"]
    "#;
    let env = [("CRABSHELL_GRANTED", "yes")];

    assert!(check(sandbox, "dep", &env, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn without_a_sandbox_file_nothing_is_granted() {
    let dir = tempfile::tempdir().unwrap();
    let status = Command::new(crabshell())
        .arg(probe())
        .args(["env-absent", "CRABSHELL_GRANTED"])
        .env("CARGO_PKG_NAME", "dep")
        .env("CARGO_MANIFEST_DIR", dir.path())
        .env("CRABSHELL_GRANTED", "yes")
        .status()
        .unwrap();

    assert!(status.success());
}
