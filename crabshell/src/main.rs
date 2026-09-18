//! Cargo invokes us as `crabshell <build-script-exe> [args...]`.

use std::process::ExitCode;

use crabshell::runner;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);

    let Some(program) = args.next() else {
        eprintln!("crabshell: expected a build-script executable to run");
        return ExitCode::FAILURE;
    };
    let args: Vec<_> = args.collect();

    // TODO: resolve policy and set up the sandbox before running.
    runner::exec(&program, &args)
}
