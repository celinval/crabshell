//! Cargo invokes us as `crabshell <build-script-exe> [args...]`.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crabshell::config::SandboxConfig;
use crabshell::policy::{PackageContext, Permissions};
use crabshell::{config, declared, paths, policy, runner};

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
/// we inherit. Without a `sandbox.toml` only implicit access and the default
/// grant apply. The package's own declaration, if any, then tightens the
/// result. Entries expand to concrete paths and names before they're compared.
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
        host: std::env::var("HOST").unwrap_or_default(),
    };
    let variables = paths::Variables::new(&config.variables, |name| std::env::var_os(name));
    let names = std::env::vars_os().filter_map(|(name, _)| name.into_string().ok()).collect();
    let x = paths::Expander::new(variables, names);
    let permissions = policy::resolve(&config, &pkg, &x)?;
    let declared = match injected(&pkg.name)? {
        Some(declared) => Some(declared),
        None => declared::load(&manifest_dir)?,
    };
    let permissions = match declared {
        Some(declared) => policy::tighten(permissions, &declared, &pkg, &config, &x)?,
        None => permissions,
    };
    let dropped = x.dropped();
    if !dropped.is_empty() && std::env::var_os("CRABSHELL_LOG").is_some() {
        eprintln!("crabshell: dropped entries with undefined variables: {dropped:?}");
    }
    Ok(permissions)
}

/// The package's declaration from the `CRABSHELL_REQUESTS` test file, if any.
///
/// Lets tests model what crates we can't edit would declare. Only builds with
/// the `test-requests` feature honor it; others reject it rather than silently
/// running without the requests a test expects.
fn injected(package: &str) -> Result<Option<declared::Declared>, Box<dyn std::error::Error>> {
    let Some(path) = std::env::var_os("CRABSHELL_REQUESTS") else {
        return Ok(None);
    };
    #[cfg(feature = "test-requests")]
    return Ok(declared::injected(Path::new(&path), package)?);
    #[cfg(not(feature = "test-requests"))]
    {
        let _ = (path, package);
        Err("CRABSHELL_REQUESTS needs a crabshell built with the `test-requests` feature".into())
    }
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
