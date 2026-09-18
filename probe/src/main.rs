//! Test fixture that inspects its sandbox and reports via exit status.
//!
//! Run as `probe <check> [args...]`, it performs one check against the
//! environment it was launched in and exits 0 on pass, non-zero on fail. This
//! lets e2e tests drive it through the runner and assert on the exit status.
//!
//! Checks:
//! - `env-present <NAME>` — the variable is set.
//! - `env-absent <NAME>`  — the variable is unset.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let check = args.next();
    let name = args.next();

    let passed = match (check.as_deref(), name.as_deref()) {
        (Some("env-present"), Some(name)) => std::env::var_os(name).is_some(),
        (Some("env-absent"), Some(name)) => std::env::var_os(name).is_none(),
        _ => {
            eprintln!("probe: usage: probe <env-present|env-absent> <NAME>");
            return ExitCode::FAILURE;
        }
    };

    if passed { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
