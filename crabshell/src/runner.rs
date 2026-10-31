//! Executes a build script under the resolved policy.
//!
//! The child starts with an empty environment and receives only the allowed
//! variables. On Linux, crabshell first confines its own process and then
//! replaces itself with the build script, so the confinement is inherited:
//!
//! - **Filesystem and exec** through Landlock: only granted paths are
//!   accessible, plus implicit access (see [`crate::policy::implicit_read`]
//!   and [`crate::policy::IMPLICIT_READ_WRITE`]) and a write-only `OUT_DIR`
//!   and private `TMPDIR`. `write` grants no read access; a path that needs
//!   both is listed in both.
//! - **Network**, unless granted, through a seccomp filter denying IP
//!   sockets, with Landlock's TCP rules as a second layer.
//!
//! Other platforms only get environment scrubbing.

use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// Write-only trees: `write` entries, `OUT_DIR` and the private temp dir.
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
    let tmp =
        crate::paths::tmp_dir(out_dir).ok_or_else(|| io::Error::other("OUT_DIR has no parent"))?;
    match std::fs::remove_dir_all(&tmp) {
        Err(err) if err.kind() != io::ErrorKind::NotFound => return Err(err),
        _ => {}
    }
    std::fs::create_dir(&tmp)?;
    Ok(tmp)
}

/// The path a resolved entry names; resolution already expanded patterns.
fn expand(entry: &str) -> io::Result<Vec<PathBuf>> {
    if !Path::new(entry).is_absolute() {
        return Err(io::Error::other(format!("`{entry}`: expected an absolute path")));
    }
    Ok(vec![PathBuf::from(entry)])
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
        .and_then(|r| r.add_rules(path_beneath_rules(&grants.write, AccessFs::from_write(abi))))
        .and_then(|r| r.add_rules(path_beneath_rules(&grants.list, AccessFs::ReadDir)))
        .and_then(|r| r.restrict_self())
        .map_err(io::Error::other)?;
    if status.ruleset == RulesetStatus::NotEnforced {
        return Err(io::Error::other("Landlock is not supported by this kernel"));
    }
    if !network {
        deny_network()?;
    }
    Ok(())
}

/// Deny creating network sockets, with seccomp: IPv4, IPv6 and raw packets.
///
/// Without an IP socket there's no TCP, UDP or DNS. Unlike a network
/// namespace, a seccomp filter needs no privileges beyond `no_new_privs`, so it
/// also works where unprivileged user namespaces are restricted (Ubuntu's
/// AppArmor since 23.10, Docker's default seccomp profile). `AF_UNIX` stays
/// allowed for local IPC such as the jobserver, and `AF_NETLINK` for local
/// routing queries.
///
/// Two gaps are closed explicitly: io_uring is denied, since
/// `IORING_OP_SOCKET` creates sockets without the `socket` syscall, and on
/// x86-64 every x32-ABI syscall is denied, since seccompiler's rules don't
/// match the x32 numbering (`nr | 0x40000000`).
#[cfg(target_os = "linux")]
fn deny_network() -> io::Result<()> {
    use std::collections::BTreeMap;

    use seccompiler::{
        BpfProgram, SeccompAction, SeccompCmpArgLen, SeccompCmpOp, SeccompCondition, SeccompFilter,
        SeccompRule,
    };

    let denied = SeccompAction::Errno(libc::EACCES as u32);
    let family = |af: libc::c_int| {
        SeccompCondition::new(0, SeccompCmpArgLen::Dword, SeccompCmpOp::Eq, af as u64)
            .and_then(|condition| SeccompRule::new(vec![condition]))
    };
    let sockets = [libc::AF_INET, libc::AF_INET6, libc::AF_PACKET]
        .into_iter()
        .map(family)
        .collect::<Result<Vec<_>, _>>()
        .map_err(io::Error::other)?;
    // An empty rule list matches the syscall unconditionally.
    let rules = BTreeMap::from([(libc::SYS_socket, sockets), (libc::SYS_io_uring_setup, vec![])]);
    let arch = std::env::consts::ARCH.try_into().map_err(io::Error::other)?;
    let filter =
        SeccompFilter::new(rules, SeccompAction::Allow, denied, arch).map_err(io::Error::other)?;
    let program: BpfProgram = filter.try_into().map_err(io::Error::other)?;
    seccompiler::apply_filter(&program).map_err(io::Error::other)?;

    #[cfg(target_arch = "x86_64")]
    seccompiler::apply_filter(&deny_x32()).map_err(io::Error::other)?;
    Ok(())
}

/// A filter denying every x32-ABI syscall, whose numbers have bit 30 set.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn deny_x32() -> seccompiler::BpfProgram {
    use seccompiler::sock_filter;

    const X32_SYSCALL_BIT: u32 = 0x4000_0000;
    let op = |code: u32, jt: u8, jf: u8, k: u32| sock_filter { code: code as u16, jt, jf, k };
    vec![
        // The syscall number is the first field of `seccomp_data`.
        op(libc::BPF_LD | libc::BPF_W | libc::BPF_ABS, 0, 0, 0),
        op(libc::BPF_JMP | libc::BPF_JSET | libc::BPF_K, 0, 1, X32_SYSCALL_BIT),
        op(libc::BPF_RET | libc::BPF_K, 0, 0, libc::SECCOMP_RET_ERRNO | libc::EACCES as u32),
        op(libc::BPF_RET | libc::BPF_K, 0, 0, libc::SECCOMP_RET_ALLOW),
    ]
}

#[cfg(not(target_os = "linux"))]
fn confine(_grants: &Grants, _network: bool) -> io::Result<()> {
    Ok(())
}
