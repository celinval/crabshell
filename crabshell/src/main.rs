//! Cargo invokes us as `crabshell <build-script-exe> [args...]`.

use std::path::PathBuf;
use std::process::ExitCode;

use crabshell::policy::{PackageContext, Permissions};
use crabshell::{config, policy, runner};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);

    let Some(program) = args.next() else {
        eprintln!("crabshell: expected a build-script executable to run");
        return ExitCode::FAILURE;
    };
    let args: Vec<_> = args.collect();

    let permissions = match permissions() {
        Ok(permissions) => permissions,
        Err(err) => {
            eprintln!("crabshell: {err}");
            return ExitCode::FAILURE;
        }
    };

    match runner::exec(&program, &args, &permissions.env) {
        Ok(status) => status.code().map_or(ExitCode::FAILURE, |code| ExitCode::from(code as u8)),
        Err(err) => {
            eprintln!("crabshell: failed to run {}: {err}", program.to_string_lossy());
            ExitCode::FAILURE
        }
    }
}

/// Resolve the permissions for the package Cargo is building.
///
/// Cargo identifies the package through the build script's environment, which
/// we inherit. Without a `sandbox.toml` the structural floor applies and grants
/// nothing.
fn permissions() -> Result<Permissions, config::ConfigError> {
    let start = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();

    let Some(path) = config::find(&start) else {
        return Ok(Permissions::default());
    };
    let config = config::load(&path)?;

    // TODO: determine workspace membership so `[workspace]` can apply; until
    // then every package resolves through `[package."*"]`.
    let pkg = PackageContext {
        name: std::env::var("CARGO_PKG_NAME").unwrap_or_default(),
        is_workspace_member: false,
    };
    Ok(policy::resolve(&config, &pkg))
}
