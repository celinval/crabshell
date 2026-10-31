//! End-to-end checks that `jail` confines the whole command it runs.
//!
//! The e2e workspace tests rely on it to build untrusted crates, so a jail
//! that grants too much would put the host at risk.

use std::net::TcpListener;
use std::path::Path;
use std::process::Command;

const JAIL: &str = env!("CARGO_BIN_EXE_jail");
const PROBE: &str = env!("CARGO_BIN_EXE_probe");

/// Run the probe `check` in a jail that grants the loader, the probe and
/// `extra`, and report whether the check passed.
fn jailed(extra: &[&str], check: &[&str]) -> bool {
    let probe_dir = Path::new(PROBE).parent().unwrap();
    Command::new(JAIL)
        .args(["--read", "/lib*", "--read"])
        .arg(probe_dir)
        .args(extra)
        .arg("--")
        .arg(PROBE)
        .args(check)
        .status()
        .unwrap()
        .success()
}

#[test]
fn only_granted_paths_are_accessible() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("secret");
    std::fs::write(&secret, "hunter2").unwrap();
    let (dir, secret) = (dir.path().to_str().unwrap(), secret.to_str().unwrap());

    assert!(jailed(&[], &["unreadable", secret]));
    assert!(jailed(&["--read", dir], &["readable", secret]));
    assert!(jailed(&["--read", dir], &["unwritable", dir]));
    assert!(jailed(&["--write", dir], &["writable", dir]));
}

#[test]
fn listing_grants_no_reads() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("secret");
    std::fs::write(&secret, "hunter2").unwrap();

    assert!(jailed(&["--list", "/"], &["unreadable", secret.to_str().unwrap()]));
}

#[test]
fn network_is_denied_unless_granted() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();

    assert!(jailed(&[], &["no-connect", &addr]));
    assert!(jailed(&["--network"], &["connect", &addr]));
    assert!(jailed(&[], &["no-udp", "127.0.0.1:0"]));
    assert!(jailed(&["--network"], &["udp", "127.0.0.1:0"]));
}

#[test]
fn rejects_bad_arguments() {
    let status = |args: &[&str]| Command::new(JAIL).args(args).status().unwrap().success();

    assert!(!status(&["--bogus", "--", PROBE]));
    assert!(!status(&["--read"]));
    assert!(!status(&["--read", "/usr"]));
}
