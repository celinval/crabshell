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

    // TODO: resolve policy from sandbox.toml to build the env allowlist.
    // Until then we default-deny: no environment is propagated.
    match runner::exec(&program, &args, &[]) {
        Ok(status) => status.code().map_or(ExitCode::FAILURE, |code| ExitCode::from(code as u8)),
        Err(err) => {
            eprintln!("crabshell: failed to run {}: {err}", program.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}
