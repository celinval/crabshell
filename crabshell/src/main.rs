//! Thin `host.runner` shell: parse the invocation, then drive the workflow.
//!
//! Cargo invokes us as `crabshell <build-script-exe> [args...]`, so the first
//! argument is the executable to run and the rest are forwarded to it verbatim.

use std::process::ExitCode;

use crabshell::runner;

fn main() -> ExitCode {
    // args[0] is our own name; args[1] is the build-script executable, and the
    // rest are the arguments Cargo wants to pass to it.
    let mut args = std::env::args_os().skip(1);

    let Some(program) = args.next() else {
        eprintln!("crabshell: expected a build-script executable to run");
        return ExitCode::FAILURE;
    };
    let args: Vec<_> = args.collect();

    // TODO: resolve the effective policy from sandbox.toml + the package's
    // own declaration, then set up the sandbox per that policy.
    runner::exec(&program, &args)
}
