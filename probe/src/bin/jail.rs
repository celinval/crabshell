//! Test helper that confines a whole command, such as `cargo build`.
//!
//! Run as:
//!
//! ```text
//! jail [--read PATH]... [--write PATH]... [--list PATH]... [--network] -- PROGRAM [ARGS]...
//! ```
//!
//! It applies crabshell's runner confinement to `PROGRAM`, so builds of
//! untrusted crates can't reach the host, including through proc-macros, which
//! crabshell's build-script runner never sees. `PROGRAM` keeps the whole
//! environment it was started with; callers curate it.
//!
//! `PATH` entries take the same syntax as policy paths, e.g. `/lib*`. Read
//! access also allows execution, and `--write` is write-only, so a path that
//! needs both is passed to both. `--list` only allows listing directories,
//! which a nested crabshell needs to expand globs. Nothing else is granted: no
//! floor, and no network unless `--network` is given.

use std::process::ExitCode;

use crabshell::paths::{Expander, Variables};
use crabshell::policy::{Expand, Permissions};
use crabshell::runner;

fn main() -> ExitCode {
    match parse(std::env::args().skip(1))
        .and_then(|(p, program, args)| Ok((expand(p)?, program, args)))
    {
        Ok((permissions, program, args)) => {
            let err = runner::exec(&program, &args, &permissions);
            eprintln!("jail: failed to run {program}: {err}");
        }
        Err(err) => eprintln!("jail: {err}"),
    }
    ExitCode::FAILURE
}

/// Parse the command line into permissions, a program and its arguments.
fn parse(
    mut args: impl Iterator<Item = String>,
) -> Result<(Permissions, String, Vec<String>), String> {
    let mut permissions = Permissions { env: vec!["*".into()], ..Permissions::default() };
    loop {
        match args.next().as_deref() {
            Some("--read") => permissions.read.push(value(&mut args, "--read")?),
            Some("--write") => permissions.write.push(value(&mut args, "--write")?),
            Some("--list") => permissions.list.push(value(&mut args, "--list")?),
            Some("--network") => permissions.network = true,
            Some("--") => break,
            Some(other) => return Err(format!("unexpected argument `{other}`")),
            None => return Err("missing `-- PROGRAM`".into()),
        }
    }
    let program = args.next().ok_or("missing PROGRAM after `--`")?;
    Ok((permissions, program, args.collect()))
}

/// Expand the glob patterns in `permissions`' paths.
fn expand(permissions: Permissions) -> Result<Permissions, String> {
    let x = Expander::new(Variables::new(&Default::default(), |_| None), Vec::new());
    let paths = |entries: Vec<String>| -> Result<Vec<String>, String> {
        let mut out = Vec::new();
        for entry in entries {
            out.extend(x.paths(&entry).map_err(|err| err.to_string())?);
        }
        Ok(out)
    };
    Ok(Permissions {
        read: paths(permissions.read)?,
        write: paths(permissions.write)?,
        list: paths(permissions.list)?,
        ..permissions
    })
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next().ok_or_else(|| format!("`{flag}` needs a PATH"))
}
