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

use std::path::PathBuf;
use std::process::Command;

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

/// A temporary build layout: a `sandbox.toml` plus a fake `CARGO_HOME`.
struct Build {
    dir: tempfile::TempDir,
}

impl Build {
    fn new(sandbox: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sandbox.toml"), sandbox).unwrap();
        Self { dir }
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
        let mut cmd = Command::new(crabshell());
        cmd.arg(probe())
            .args(args)
            .env("CARGO_PKG_NAME", pkg)
            .env("CARGO_MANIFEST_DIR", self.manifest_dir(pkg, &origin))
            .env("CARGO_HOME", self.cargo_home())
            .env("CRABSHELL_CONFIG", self.dir.path().join("sandbox.toml"));
        for (key, value) in env {
            cmd.env(key, value);
        }
        cmd.status().unwrap().success()
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

    assert!(build.check("special", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
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
    let env = [("OUT_DIR", "/tmp")];

    assert!(build.check("dep", Origin::External, &env, &["env-present", "OUT_DIR"]));
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
        .env_remove("CRABSHELL_CONFIG")
        .env("CRABSHELL_GRANTED", "yes")
        .status()
        .unwrap();

    assert!(status.success());
}

/// Without `CRABSHELL_CONFIG`, crabshell walks up from the manifest directory.
#[test]
fn config_is_discovered_by_walking_up() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("sandbox.toml"), "[local]\nenv = [\"CRABSHELL_GRANTED\"]\n")
        .unwrap();
    let manifest = dir.path().join("nested/pkg");
    std::fs::create_dir_all(&manifest).unwrap();

    let status = Command::new(crabshell())
        .arg(probe())
        .args(["env-present", "CRABSHELL_GRANTED"])
        .env("CARGO_PKG_NAME", "mine")
        .env("CARGO_MANIFEST_DIR", &manifest)
        .env("CARGO_HOME", dir.path().join("cargo-home"))
        .env_remove("CRABSHELL_CONFIG")
        .env("CRABSHELL_GRANTED", "yes")
        .status()
        .unwrap();

    assert!(status.success());
}
