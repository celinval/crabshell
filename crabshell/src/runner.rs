//! Executes a build script under the resolved policy.
//!
//! The child starts with an empty environment and receives only the allowed
//! variables. On Linux, crabshell first confines its own process and then
//! replaces itself with the build script, so the confinement is inherited:
//!
//! - **Filesystem and exec** through Landlock: only granted paths are
//!   accessible, plus implicit access (see [`crate::policy::IMPLICIT_WRITE`]
//!   and [`crate::policy::implicit_runtime`]) and a private `TMPDIR`.
//! - **Network**, unless granted, through an empty network namespace, with
//!   Landlock's TCP rules as a second layer.
//!
//! Other platforms only get environment scrubbing.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::paths;
use crate::policy::Permissions;

/// Build the command for `program` with only the `allowed_env` variables.
///
/// An entry ending in `*` allows every variable with that prefix, e.g.
/// `CARGO_CFG_*`. The command is not confined; [`exec`] confines the current
/// process first.
pub fn command<S: AsRef<OsStr>>(program: &S, args: &[S], allowed_env: &[String]) -> Command {
    let mut cmd = Command::new(program.as_ref());
    cmd.args(args).env_clear();
    let allowed = |key: &str| {
        allowed_env.iter().any(|entry| match entry.strip_suffix('*') {
            Some(prefix) => key.starts_with(prefix),
            None => key == entry,
        })
    };
    for (key, value) in std::env::vars_os() {
        if key.to_str().is_some_and(allowed) {
            cmd.env(key, value);
        }
    }
    cmd
}

/// Confine the current process under `permissions`, then replace it with `program`.
///
/// Returns only on failure. Setting `CRABSHELL_LOG` prints the resolved
/// permissions and the granted paths to stderr.
pub fn exec<S: AsRef<OsStr>>(program: &S, args: &[S], permissions: &Permissions) -> io::Error {
    let log = std::env::var_os("CRABSHELL_LOG").is_some();
    if log {
        eprintln!("crabshell: {permissions:?}");
    }
    let grants = match Grants::new(Path::new(program.as_ref()), permissions) {
        Ok(grants) => grants,
        Err(err) => return err,
    };
    if log {
        eprintln!("crabshell: {grants:#?}");
    }
    if let Err(err) = confine(&grants, permissions.network) {
        return err;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut cmd = command(program, args, &permissions.env);
        if let Some(tmp) = &grants.tmp {
            cmd.env("TMPDIR", tmp);
        }
        cmd.exec()
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        io::Error::other("crabshell only supports Unix hosts")
    }
}

/// The concrete paths a build script may access.
#[derive(Debug)]
struct Grants {
    /// Readable (and executable) trees: `read` entries and the floor.
    read: Vec<PathBuf>,
    /// Executable files: `run` entries and the build script itself.
    exec: Vec<PathBuf>,
    /// Writable trees: `write` entries, `OUT_DIR` and the private temp dir.
    write: Vec<PathBuf>,
    /// Trees whose directories may only be listed.
    list: Vec<PathBuf>,
    /// The private temp dir, exported as `TMPDIR`.
    tmp: Option<PathBuf>,
}

impl Grants {
    fn new(program: &Path, permissions: &Permissions) -> io::Result<Self> {
        let mut read = Vec::new();
        for entry in &permissions.read {
            read.extend(expand(entry)?);
        }
        let mut exec = vec![program.to_path_buf()];
        for entry in &permissions.run {
            exec.push(which(entry)?);
        }
        let out_dir = std::env::var_os("OUT_DIR").map(PathBuf::from);
        let tmp = out_dir.as_deref().map(private_tmp).transpose()?;
        let mut write: Vec<PathBuf> = out_dir.into_iter().chain(tmp.clone()).collect();
        for entry in &permissions.write {
            write.extend(expand(entry)?);
        }
        let mut list = Vec::new();
        for entry in &permissions.list {
            list.extend(expand(entry)?);
        }
        Ok(Self { read, exec, write, list, tmp })
    }
}

/// Create an empty temp dir for this run, next to `out_dir`.
///
/// A shared `/tmp` would let build scripts leave files for one another.
/// crabshell replaces itself with the build script and can't clean up
/// afterwards, so the dir lives in Cargo's per-unit build dir: it's emptied on
/// the next run and removed by `cargo clean`.
fn private_tmp(out_dir: &Path) -> io::Result<PathBuf> {
    let parent = out_dir.parent().ok_or_else(|| io::Error::other("OUT_DIR has no parent"))?;
    let tmp = parent.join("crabshell-tmp");
    match std::fs::remove_dir_all(&tmp) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
        _ => {}
    }
    std::fs::create_dir(&tmp)?;
    Ok(tmp)
}

/// Expand a policy entry, whose variables are already expanded, into paths.
fn expand(entry: &str) -> io::Result<Vec<PathBuf>> {
    paths::glob(entry).map_err(io::Error::other)
}

/// Resolve a `run` entry: a path is expanded, a bare name is searched on `PATH`.
fn which(entry: &str) -> io::Result<PathBuf> {
    if entry.contains('/') {
        return expand(entry)?
            .pop()
            .ok_or_else(|| io::Error::other(format!("`{entry}`: no such program")));
    }
    let path = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&path)
        .map(|dir| dir.join(entry))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| io::Error::other(format!("`{entry}`: not found on PATH")))
}

#[cfg(target_os = "linux")]
fn confine(grants: &Grants, network: bool) -> io::Result<()> {
    use landlock::{
        ABI, Access, AccessFs, AccessNet, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus,
        path_beneath_rules,
    };

    if !network && has_route() {
        isolate_network()?;
    }

    let abi = ABI::V6;
    let mut ruleset =
        Ruleset::default().handle_access(AccessFs::from_all(abi)).map_err(io::Error::other)?;
    if !network {
        ruleset = ruleset
            .handle_access(AccessNet::BindTcp | AccessNet::ConnectTcp)
            .map_err(io::Error::other)?;
    }
    let status = ruleset
        .create()
        .and_then(|r| r.add_rules(path_beneath_rules(&grants.read, AccessFs::from_read(abi))))
        .and_then(|r| {
            r.add_rules(path_beneath_rules(&grants.exec, AccessFs::Execute | AccessFs::ReadFile))
        })
        .and_then(|r| r.add_rules(path_beneath_rules(&grants.write, AccessFs::from_all(abi))))
        .and_then(|r| r.add_rules(path_beneath_rules(&grants.list, AccessFs::ReadDir)))
        .and_then(|r| r.restrict_self())
        .map_err(io::Error::other)?;
    if status.ruleset == RulesetStatus::NotEnforced {
        return Err(io::Error::other("Landlock is not supported by this kernel"));
    }
    Ok(())
}

/// Whether the current network namespace can route to the outside world.
///
/// `connect` on a UDP socket only looks up a route; it sends nothing. Without a
/// route there's nothing to isolate from, which is the case when crabshell
/// runs nested inside another sandbox's network namespace. That sandbox's
/// Landlock domain may also deny the `/proc` writes [`isolate_network`] needs.
#[cfg(target_os = "linux")]
fn has_route() -> bool {
    let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") else {
        return false;
    };
    // A TEST-NET-3 address (RFC 5737): routable syntax, never a real host.
    socket.connect("203.0.113.1:9").is_ok()
}

/// Move into a new user and network namespace, which has only a down loopback.
///
/// The user namespace is what lets an unprivileged process create the network
/// namespace; our own uid and gid map to themselves inside it.
#[cfg(target_os = "linux")]
fn isolate_network() -> io::Result<()> {
    use rustix::thread::{UnshareFlags, unshare_unsafe};

    let uid = rustix::process::getuid().as_raw();
    let gid = rustix::process::getgid().as_raw();
    // SAFETY: new user and network namespaces don't unshare file descriptors
    // or other state Rust's runtime relies on.
    unsafe { unshare_unsafe(UnshareFlags::NEWUSER | UnshareFlags::NEWNET)? };
    std::fs::write("/proc/self/setgroups", "deny")?;
    std::fs::write("/proc/self/uid_map", format!("{uid} {uid} 1"))?;
    std::fs::write("/proc/self/gid_map", format!("{gid} {gid} 1"))?;
    Ok(())
}

#[cfg(not(target_os = "linux"))]
fn confine(_grants: &Grants, _network: bool) -> io::Result<()> {
    Ok(())
}
