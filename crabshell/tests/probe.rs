//! Checks that `crabshell-probe` prints a config crabshell accepts.

use std::process::Command;

#[test]
fn output_is_a_valid_config() {
    let out = Command::new(env!("CARGO_BIN_EXE_crabshell-probe")).output().unwrap();
    assert!(out.status.success());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, &out.stdout).unwrap();

    let config = crabshell::config::load(&path).unwrap();
    let report = String::from_utf8(out.stderr).unwrap();
    for grant in ["cc", "pkg-config", "rustc"] {
        let added = report.contains(&format!("grant {grant} : added\n"))
            || report.contains(&format!("grant {grant} : partial, "));
        assert_eq!(config.grant.contains_key(grant), added, "{grant}:\n{report}");
    }
}
