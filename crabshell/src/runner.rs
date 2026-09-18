//! Executes a build script, forwarding stdio and its exit status.
//!
//! Currently a passthrough with no confinement; the sandbox (namespaces /
//! mounts / seccomp) is not yet applied.

use std::ffi::OsStr;
use std::process::{Command, ExitCode};

/// Run `program` with `args`, returning its exit status.
pub fn exec<S: AsRef<OsStr>>(program: &S, args: &[S]) -> ExitCode {
    let program = program.as_ref();
    match Command::new(program).args(args).status() {
        Ok(status) => match status.code() {
            Some(code) => ExitCode::from(code as u8),
            None => ExitCode::FAILURE, // terminated by a signal
        },
        Err(err) => {
            eprintln!("crabshell: failed to run {}: {err}", program.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
