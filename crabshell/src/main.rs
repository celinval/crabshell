//! Cargo invokes us as `crabshell <build-script-exe> [args...]`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crabshell::config::SandboxConfig;
use crabshell::policy::{PackageContext, Permissions};
use crabshell::{config, declared, policy, runner};

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

    let err = runner::exec(&program, &args, &permissions);
    eprintln!("crabshell: failed to run {}: {err}", program.to_string_lossy());
    ExitCode::FAILURE
}

/// Resolve the permissions for the package Cargo is building.
///
/// Cargo identifies the package through the build script's environment, which
/// we inherit. Without a `sandbox.toml` only the structural floor applies. The
/// package's own declaration, if any, then tightens the result.
fn permissions() -> Result<Permissions, Box<dyn std::error::Error>> {
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_default();

    let config = match config_path(&manifest_dir) {
        Some(path) => config::load(&path)?,
        None => SandboxConfig::default(),
    };
    if config.external.network == Some(true) {
        // Cargo shows `cargo::warning` lines from a build script's stdout.
        println!(
            "cargo::warning=crabshell: [external] grants network to every registry and git dependency"
        );
    }

    let pkg = PackageContext {
        name: std::env::var("CARGO_PKG_NAME").unwrap_or_default(),
        is_local: is_local(&manifest_dir),
    };
    let permissions = policy::resolve(&config, &pkg);
    Ok(match declared::load(&manifest_dir)? {
        Some(declared) => policy::tighten(permissions, &declared),
        None => permissions,
    })
}

/// Locate the `sandbox.toml` governing this build.
///
/// A dependency's manifest lives under `CARGO_HOME`, so walking up from it
/// never reaches the consumer's config. `CRABSHELL_CONFIG` carries the path
/// instead, which Cargo can supply for every build script at once:
///
/// ```toml
/// [env]
/// CRABSHELL_CONFIG = { value = "sandbox.toml", relative = true }
/// ```
fn config_path(manifest_dir: &Path) -> Option<PathBuf> {
    match std::env::var_os("CRABSHELL_CONFIG") {
        Some(path) => Some(PathBuf::from(path)),
        None => config::find(manifest_dir),
    }
}

/// Whether the package's source is on the filesystem rather than fetched.
///
/// Registry and git dependencies are unpacked under `CARGO_HOME`, so anything
/// outside it is a workspace member or a path dependency. Vendored
/// dependencies are a known gap: they sit in the project tree yet are
/// third-party, and the environment alone cannot distinguish them.
fn is_local(manifest_dir: &Path) -> bool {
    let Some(cargo_home) = cargo_home() else {
        return false;
    };
    !manifest_dir.starts_with(&cargo_home)
}

fn cargo_home() -> Option<PathBuf> {
    std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cargo")))
}
