//! Executes a build script, eventually inside a sandbox.
//!
//! At this prototype stage execution is a transparent passthrough: we spawn the
//! build script with no confinement, forwarding stdio and its exit status. This
//! validates the `host.runner` invocation contract end to end before we layer
//! on the namespace-based sandbox.

use std::ffi::OsStr;
use std::process::{Command, ExitCode};

/// Run `program` with `args`, forwarding stdio and its exit status.
///
/// TODO: set up the sandbox (namespaces / mounts / seccomp) for the resolved
/// policy before spawning. For now the script runs unconfined.
pub fn exec<S: AsRef<OsStr>>(program: &S, args: &[S]) -> ExitCode {
    let program = program.as_ref();
    match Command::new(program).args(args).status() {
        Ok(status) => match status.code() {
            Some(code) => ExitCode::from(code as u8),
            // Terminated by a signal: report a generic failure.
            None => ExitCode::FAILURE,
        },
        Err(err) => {
            eprintln!("crabshell: failed to run {}: {err}", program.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
