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

use std::net::TcpStream;
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
