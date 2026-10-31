//! Test fixture that inspects its sandbox and reports via exit status.
//!
//! Run as `probe <check> <arg>`, it performs one check against the
//! environment it was launched in and exits 0 on pass, non-zero on fail. This
//! lets e2e tests drive it through the runner and assert on the exit status.
//!
//! Checks:
//! - `env-present <NAME>` — the variable is set.
//! - `env-absent <NAME>`  — the variable is unset.
//! - `readable <PATH>`    — the file can be read.
//! - `unreadable <PATH>`  — the file cannot be read.
//! - `writable <DIR>`     — a new file can be created in the directory.
//! - `unwritable <DIR>`   — a new file cannot be created in the directory.
//! - `write-file <PATH>`  — the existing file can be opened for writing.
//! - `read-back <DIR>`    — a file created in the directory can be read back.
//! - `write-only <DIR>`   — a file can be created in the directory, but not
//!   read back.
//! - `private-tmp <DIR>`  — the temp dir is writable and isn't `DIR`.
//! - `can-exec <PATH>`    — the program can be spawned.
//! - `cannot-exec <PATH>` — the program cannot be spawned.
//! - `connect <ADDR>`     — a TCP connection to `ADDR` succeeds.
//! - `no-connect <ADDR>`  — a TCP connection to `ADDR` fails.
//! - `udp <ADDR>`         — a UDP socket bound to `ADDR` can be created.
//! - `no-udp <ADDR>`      — a UDP socket bound to `ADDR` can't be created.
//! - `no-io-uring _`      — `io_uring_setup` is denied (Linux).
//! - `no-x32-socket _`    — an x32-ABI `socket` syscall is denied (x86-64 Linux).

use std::net::{TcpStream, UdpSocket};
use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let (Some(check), Some(arg)) = (args.next(), args.next()) else {
        eprintln!("probe: usage: probe <check> <arg>");
        return ExitCode::FAILURE;
    };

    let passed = match check.as_str() {
        "env-present" => std::env::var_os(&arg).is_some(),
        "env-absent" => std::env::var_os(&arg).is_none(),
        "readable" => std::fs::read(&arg).is_ok(),
        "unreadable" => std::fs::read(&arg).is_err(),
        "writable" => can_create(Path::new(&arg)),
        "unwritable" => !can_create(Path::new(&arg)),
        "write-file" => std::fs::OpenOptions::new().write(true).open(&arg).is_ok(),
        "read-back" => read_back(Path::new(&arg)) == Some(true),
        "write-only" => read_back(Path::new(&arg)) == Some(false),
        "private-tmp" => {
            let tmp = std::env::temp_dir();
            tmp != Path::new(&arg) && can_create(&tmp)
        }
        "can-exec" => Command::new(&arg).status().is_ok(),
        "cannot-exec" => Command::new(&arg).status().is_err(),
        "connect" => TcpStream::connect(&arg).is_ok(),
        "no-connect" => TcpStream::connect(&arg).is_err(),
        "udp" => UdpSocket::bind(&arg).is_ok(),
        "no-udp" => UdpSocket::bind(&arg).is_err(),
        "no-io-uring" => syscall_fails(Syscall::IoUringSetup),
        "no-x32-socket" => syscall_fails(Syscall::X32Socket),
        _ => {
            eprintln!("probe: unknown check `{check}`");
            return ExitCode::FAILURE;
        }
    };

    if passed { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn can_create(dir: &Path) -> bool {
    let path = dir.join("probe-write-check");
    let created = std::fs::write(&path, b"").is_ok();
    let _ = std::fs::remove_file(&path);
    created
}

/// Create a file in `dir` and report whether it reads back; `None` if it
/// can't be created.
fn read_back(dir: &Path) -> Option<bool> {
    let path = dir.join("probe-read-back");
    std::fs::write(&path, b"x").ok()?;
    let readable = std::fs::read(&path).is_ok();
    let _ = std::fs::remove_file(&path);
    Some(readable)
}

/// A raw syscall a sandbox should deny.
enum Syscall {
    IoUringSetup,
    X32Socket,
}

/// Whether a sandbox denied `syscall`, i.e. it failed with `EACCES`. Off Linux,
/// or for x32 off x86-64, there's no such syscall, so it counts as denied.
fn syscall_fails(syscall: Syscall) -> bool {
    #[cfg(target_os = "linux")]
    {
        let rc = match syscall {
            // SAFETY: a null params pointer makes the kernel fail with EFAULT
            // unless something rejects the syscall first; nothing is set up.
            Syscall::IoUringSetup => unsafe {
                libc::syscall(libc::SYS_io_uring_setup, 1, std::ptr::null_mut::<libc::c_void>())
            },
            #[cfg(target_arch = "x86_64")]
            // SAFETY: `socket` takes no pointers; a created socket is closed below.
            Syscall::X32Socket => unsafe {
                libc::syscall(0x4000_0000 | libc::SYS_socket, libc::AF_INET, libc::SOCK_STREAM, 0)
            },
            #[cfg(not(target_arch = "x86_64"))]
            Syscall::X32Socket => return true,
        };
        if rc >= 0 {
            // SAFETY: closing the fd the syscall just returned.
            unsafe { libc::close(rc as libc::c_int) };
            return false;
        }
        // A sandbox denies with EACCES. Unconfined, io_uring with a null
        // pointer fails with EFAULT, and x32 on kernels without it with
        // ENOSYS, so other errors don't count.
        std::io::Error::last_os_error().raw_os_error() == Some(libc::EACCES)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = syscall;
        true
    }
}
