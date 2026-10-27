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
//! `probe` is dynamically linked, so every policy that expects it to start
//! grants [`LOADER`].

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Output};

/// A `read` grant for the dynamic loader and libc.
const LOADER: &str = r#"read = ["/lib*"]"#;

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
            .env("OUT_DIR", self.out_dir())
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
    let build = Build::new(&format!("[external]\nenv = [\"CRABSHELL_GRANTED\"]\n{LOADER}\n"));

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_UNGRANTED"]));
}

#[test]
fn category_follows_the_manifest_location() {
    let build = Build::new(&format!(
        r#"
        [local]
        env = ["CRABSHELL_GRANTED"]
        {LOADER}

        [external]
        env = []
        {LOADER}
        "#,
    ));

    assert!(build.check("mine", Origin::Local, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn named_package_overrides_its_category() {
    let build = Build::new(&format!(
        r#"
        [external]
        env = []
        {LOADER}

        [package.special]
        env = ["CRABSHELL_GRANTED"]
        "#,
    ));

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
    let build = Build::new(&format!(
        r#"
        [limits]
        env = []

        [external]
        env = ["CRABSHELL_GRANTED"]
        {LOADER}
        "#,
    ));

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_GRANTED"]));
}

#[test]
fn out_dir_is_always_granted() {
    let build = Build::new(&format!("[external]\nenv = []\n{LOADER}\n"));

    assert!(build.check("dep", Origin::External, &[], &["env-present", "OUT_DIR"]));
}

#[test]
fn a_rejected_config_fails_the_build() {
    let build = Build::new("[package.\"*\"]\nenv = []\n");

    assert!(!build.check("dep", Origin::External, &GRANTED, &["env-absent", "ANYTHING"]));
}

/// The floor doesn't cover the dynamic loader, so a dynamic program can't start.
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

    assert!(!status.success());
}

/// Without `CRABSHELL_CONFIG`, crabshell walks up from the manifest directory.
#[test]
fn config_is_discovered_by_walking_up() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = format!("[local]\nenv = [\"CRABSHELL_GRANTED\"]\n{LOADER}\n");
    std::fs::write(dir.path().join("sandbox.toml"), sandbox).unwrap();
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

#[test]
fn ungranted_paths_are_unreadable() {
    let build = Build::new(&format!("[external]\n{LOADER}\n"));
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
        format!("[external]\nread = [\"/lib*\", \"{dir}\"]\n"),
    )
    .unwrap();

    assert!(build.check("dep", Origin::External, &[], &["readable", secret.to_str().unwrap()]));
    assert!(build.check("dep", Origin::External, &[], &["unwritable", dir]));
}

#[test]
fn the_crate_root_is_readable_and_out_dir_writable() {
    let build = Build::new(&format!("[external]\n{LOADER}\n"));
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
    let build = Build::new(&format!("[external]\nrun = [\"true\"]\n{LOADER}\n"));
    let path = std::env::var("PATH").unwrap();
    let env = [("PATH", path.as_str())];

    assert!(build.check("dep", Origin::External, &env, &["can-exec", "/usr/bin/true"]));
    assert!(build.check("dep", Origin::External, &env, &["cannot-exec", "/usr/bin/false"]));
}

#[test]
fn network_is_denied_unless_granted() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let build = Build::new(&format!(
        r#"
        [external]
        {LOADER}

        [package.online]
        network = true
        "#,
    ));

    assert!(build.check("dep", Origin::External, &[], &["no-connect", &addr]));
    assert!(build.check("online", Origin::External, &[], &["connect", &addr]));
}

#[test]
fn granting_network_to_external_warns() {
    let build = Build::new(&format!("[external]\nnetwork = true\n{LOADER}\n"));

    let output = build.run("dep", Origin::External, &[], &["env-absent", "ANYTHING"]);
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("cargo::warning=crabshell: [external] grants network")
    );
}

#[test]
fn a_declaration_tightens_the_grant() {
    let build = Build::new(&format!(
        "[external]\nenv = [\"CRABSHELL_GRANTED\", \"CRABSHELL_UNGRANTED\"]\n{LOADER}\n"
    ));
    let manifest = build.manifest_dir("dep", &Origin::External);
    std::fs::write(
        manifest.join("Cargo.toml"),
        "[package]\nname = \"dep\"\n\n[package.metadata.build-rs]\nread = [\"/lib*\"]\nenv = [\"CRABSHELL_GRANTED\"]\n",
    )
    .unwrap();

    assert!(build.check("dep", Origin::External, &GRANTED, &["env-present", "CRABSHELL_GRANTED"]));
    assert!(build.check("dep", Origin::External, &GRANTED, &["env-absent", "CRABSHELL_UNGRANTED"]));
}

#[test]
fn the_floor_grants_dev_null_and_a_private_tmpdir() {
    let build = Build::new(&format!("[external]\n{LOADER}\n"));

    assert!(build.check("dep", Origin::External, &[], &["write-file", "/dev/null"]));
    assert!(build.check("dep", Origin::External, &[], &["private-tmp", "/tmp"]));
    assert!(build.check("dep", Origin::External, &[], &["unwritable", "/tmp"]));
}
