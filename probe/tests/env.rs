//! End-to-end checks that the runner only propagates allowed env variables.

use crabshell::runner;

const PROBE: &str = env!("CARGO_BIN_EXE_probe");

/// Run the probe `check` through the runner, allowing only `allowed_env`, and
/// report whether the check passed.
fn probe(check: &[&str], allowed_env: &[String]) -> bool {
    let program = PROBE.to_string();
    let args: Vec<String> = check.iter().map(|s| s.to_string()).collect();
    runner::command(&program, &args, allowed_env).status().unwrap().success()
}

#[test]
fn propagates_only_allowed_env() {
    // SAFETY: single-threaded test; no other test mutates these variables.
    unsafe {
        std::env::set_var("CRABSHELL_TEST_ALLOWED", "yes");
        std::env::set_var("CRABSHELL_TEST_DENIED", "yes");
        std::env::set_var("CRABSHELL_PREFIX_ONE", "yes");
    }
    let allowed = ["CRABSHELL_TEST_ALLOWED".to_string(), "CRABSHELL_PREFIX_*".to_string()];

    assert!(probe(&["env-present", "CRABSHELL_TEST_ALLOWED"], &allowed));
    assert!(probe(&["env-absent", "CRABSHELL_TEST_DENIED"], &allowed));
    // A trailing `*` allows every variable with that prefix.
    assert!(probe(&["env-present", "CRABSHELL_PREFIX_ONE"], &allowed));
}
