//! Executes a build script under the resolved policy.
//!
//! The only confinement so far is environment scrubbing: the child starts with
//! an empty environment and receives only the allowed variables. Filesystem,
//! network, and syscall confinement are not yet applied.

use std::ffi::OsStr;
use std::io;
use std::process::{Command, ExitStatus};

/// Run `program` with `args`, passing through only the `allowed_env` variables.
pub fn exec<S: AsRef<OsStr>>(
    program: &S,
    args: &[S],
    allowed_env: &[String],
) -> io::Result<ExitStatus> {
    let mut cmd = Command::new(program.as_ref());
    cmd.args(args).env_clear();
    for key in allowed_env {
        if let Some(value) = std::env::var_os(key) {
            cmd.env(key, value);
        }
    }
    cmd.status()
}
