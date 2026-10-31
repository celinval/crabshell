//! Prints this machine's `run` and `read` for crabshell's built-in grants, as
//! user-config TOML.
//!
//! Run once per machine, e.g.
//! `crabshell-probe > ~/.config/crabshell/config.toml`. Each grant's `run` and
//! `read` are complete and override the built-in ones: the built-in entries
//! that exist here, plus what the host's tools report they use. `env` stays
//! built-in.
//!
//! A report on stderr says whether each grant was added, partial (added, but
//! likely incomplete), skipped (its tool isn't installed) or failed; only
//! added and partial grants are printed.
//!
//! The probe is static: it can't see what a wrapper script runs. For a tool
//! that's a script it grants the interpreter and reports the grant as partial; the rest has to be
//! added by hand.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crabshell::config::ListField;
use crabshell::grants;
use crabshell::policy::implicit_read;

fn main() -> ExitCode {
    let mut probe = Probe::new();
    probe.pkg_config();
    probe.cc();
    probe.rustc();
    print!("{}", probe.toml());
    eprint!("{}", probe.report());
    ExitCode::SUCCESS
}

/// One grant's probed fields.
#[derive(Default)]
struct Grant {
    run: Vec<String>,
    read: Vec<String>,
    /// Paths `read` already covers, including what's always readable.
    covered: Vec<String>,
    /// What the probe found but couldn't follow, making the grant partial.
    warnings: Vec<String>,
    /// Why the grant is skipped: its tool isn't installed.
    skipped: Option<String>,
    /// What the probe couldn't do.
    errors: Vec<String>,
}

struct Probe {
    grants: BTreeMap<String, Grant>,
}

impl Probe {
    fn new() -> Self {
        let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".into());
        let host = output(&rustc, &["-vV"])
            .and_then(|info| info.lines().find_map(|l| l.strip_prefix("host: ")).map(String::from))
            .unwrap_or_default();
        let sysroot = output(&rustc, &["--print", "sysroot"]).map(|s| normalize(s.trim()));
        let implicit: Vec<String> = implicit_read(&host).into_iter().map(String::from).collect();
        let grants = grants::builtin()
            .into_iter()
            .map(|(name, builtin)| {
                let mut grant = Grant { covered: implicit.clone(), ..Grant::default() };
                for entry in values(&builtin.run) {
                    let exists = match entry.starts_with('/') {
                        true => Path::new(&entry).exists(),
                        false => which(&entry).is_some(),
                    };
                    if exists {
                        grant.run.push(entry);
                    }
                }
                for entry in values(&builtin.read) {
                    let path = match (entry.as_str(), &sysroot) {
                        (":sysroot", Some(sysroot)) => Some(sysroot.clone()),
                        (entry, _) if entry.starts_with('/') => Some(entry.to_string()),
                        _ => None,
                    };
                    match path {
                        Some(path) if Path::new(&path).exists() => grant.covered.push(path),
                        Some(_) => continue,
                        None => {}
                    }
                    grant.read.push(entry);
                }
                (name, grant)
            })
            .collect();
        Self { grants }
    }

    fn grant(&mut self, name: &str) -> &mut Grant {
        self.grants.entry(name.to_string()).or_default()
    }

    /// Record that `grant` reads `path`, unless it's already covered.
    fn read(&mut self, grant: &str, path: &str) {
        // Compare canonical paths too: a symlinked home spells the sysroot
        // differently from what `ldd` reports.
        let canonical =
            |p: &str| std::fs::canonicalize(p).map(|p| p.to_string_lossy().into_owned());
        let path_canonical = canonical(path).unwrap_or_else(|_| path.to_string());
        let grant = self.grant(grant);
        let covered = grant
            .covered
            .iter()
            .any(|p| within(p, path) || canonical(p).is_ok_and(|p| within(&p, &path_canonical)));
        if !covered {
            grant.read.push(path.to_string());
            grant.covered.push(path.to_string());
        }
    }

    fn run(&mut self, grant: &str, program: &str) {
        let grant = self.grant(grant);
        if !grant.run.iter().any(|p| p == program) {
            grant.run.push(program.to_string());
        }
    }

    /// Record what running `program` takes: its libraries, or for a script its
    /// interpreter and a note.
    fn program(&mut self, grant: &str, name: &str, program: &Path) {
        match interpreter(program) {
            Some(interpreter) => {
                let note = format!(
                    "`{name}` ({}) is a `{interpreter}` script; what it runs isn't probed",
                    program.display()
                );
                self.grant(grant).warnings.push(note);
                self.run(grant, &interpreter);
                self.libraries(grant, Path::new(&interpreter));
            }
            None => self.libraries(grant, program),
        }
    }

    fn libraries(&mut self, grant: &str, program: &Path) {
        let Some(libraries) = shared_libraries(program) else {
            let problem = format!("`ldd {}` failed", program.display());
            return self.error(grant, problem);
        };
        for library in libraries {
            self.read(grant, &library);
        }
    }

    fn error(&mut self, grant: &str, error: String) {
        self.grant(grant).errors.push(error);
    }

    fn skip(&mut self, grant: &str, reason: String) {
        self.grant(grant).skipped = Some(reason);
    }

    fn pkg_config(&mut self) {
        let Some(program) = which("pkg-config") else {
            return self.skip("pkg-config", "`pkg-config` not found on PATH".into());
        };
        self.program("pkg-config", "pkg-config", &program);
        if let Some(pkgconf) = which("pkgconf") {
            self.program("pkg-config", "pkgconf", &pkgconf);
        }
        let Some(paths) = output("pkg-config", &["--variable", "pc_path", "pkg-config"]) else {
            let problem = "`pkg-config --variable pc_path pkg-config` failed".into();
            return self.error("pkg-config", problem);
        };
        for dir in paths.trim().split(':').filter(|dir| Path::new(dir).is_dir()) {
            self.read("pkg-config", dir);
        }
    }

    fn cc(&mut self) {
        if which("cc").is_none() {
            return self.skip("cc", "`cc` not found on PATH".into());
        }
        for helper in ["cc1", "collect2"] {
            let path = output("cc", &[&format!("-print-prog-name={helper}")])
                .map(|path| PathBuf::from(path.trim()))
                .filter(|path| path.is_absolute() && path.is_file());
            let Some(path) = path else {
                self.error("cc", format!("`cc -print-prog-name={helper}` found no program"));
                continue;
            };
            if let Some(dir) = path.parent() {
                self.read("cc", &dir.to_string_lossy());
            }
            self.program("cc", helper, &path);
        }
        for tool in ["cc", "as", "ld"] {
            match which(tool) {
                Some(program) => self.program("cc", tool, &program),
                None => self.error("cc", format!("`{tool}` not found on PATH")),
            }
        }
        for dir in include_dirs() {
            self.read("cc", &dir);
        }
    }

    fn rustc(&mut self) {
        let rustc = std::env::var_os("RUSTC")
            .map(PathBuf::from)
            .or_else(|| output("rustup", &["which", "rustc"]).map(|p| PathBuf::from(p.trim())));
        match rustc.filter(|path| path.is_file()) {
            Some(rustc) => self.libraries("rustc", &rustc),
            None => {
                self.skip("rustc", "neither `RUSTC` nor `rustup which rustc` found rustc".into())
            }
        }
    }

    /// Whether each grant was added, partial, skipped or failed, then a
    /// summary.
    fn report(&self) -> String {
        let (mut added, mut partial, mut skipped, mut failed) = (0, 0, 0, 0);
        let mut out = String::new();
        for (name, grant) in &self.grants {
            let status = match (&grant.skipped, grant.errors.as_slice(), grant.warnings.as_slice())
            {
                (Some(reason), _, _) => {
                    skipped += 1;
                    format!("skipped, {reason}")
                }
                (None, [], []) => {
                    added += 1;
                    "added".to_string()
                }
                (None, [], warnings) => {
                    partial += 1;
                    format!("partial, {}", warnings.join("; "))
                }
                (None, errors, _) => {
                    failed += 1;
                    format!("failed, {}", errors.join("; "))
                }
            };
            out.push_str(&format!("grant {name} : {status}\n"));
        }
        out.push_str(&format!(
            "\n{added} added, {partial} partial, {skipped} skipped, {failed} failed\n\n\
             warning: audit grants before use; build scripts that request them can access \
             every path listed\n"
        ));
        out
    }

    fn added(&self) -> impl Iterator<Item = (&String, &Grant)> {
        self.grants.iter().filter(|(_, grant)| grant.skipped.is_none() && grant.errors.is_empty())
    }

    fn toml(&self) -> String {
        let os = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
        let name = os.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=")).unwrap_or("unknown");
        let mut out = format!(
            "# Generated by crabshell-probe on {}. Each `run` and `read`\n\
             # overrides the built-in grant's.\n",
            name.trim_matches('"')
        );
        for (name, grant) in self.added() {
            out.push_str(&format!("\n[grant.{name}]\n"));
            for (field, values) in [("run", &grant.run), ("read", &grant.read)] {
                if values.is_empty() {
                    continue;
                }
                out.push_str(&format!("{field} = [\n"));
                for value in values {
                    out.push_str(&format!("    {value:?},\n"));
                }
                out.push_str("]\n");
            }
        }
        out
    }
}

/// A list field's entries, ignoring whether it inherits.
fn values(field: &Option<ListField>) -> Vec<String> {
    match field {
        Some(ListField::Override(values) | ListField::Inherit { value: values, .. }) => {
            values.clone()
        }
        None => Vec::new(),
    }
}

/// Whether `path` is `outer` or under it.
fn within(outer: &str, path: &str) -> bool {
    match path.strip_prefix(outer) {
        Some(rest) => rest.is_empty() || rest.starts_with('/') || outer.ends_with('/'),
        None => false,
    }
}

/// A program's stdout, if it ran successfully.
fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|dir| dir.join(name)).find(|p| p.is_file())
}

/// The interpreter of a `#!` script, or `None` for anything else.
fn interpreter(program: &Path) -> Option<String> {
    let text = std::fs::read(program).ok()?;
    let line = text.strip_prefix(b"#!")?.split(|b| *b == b'\n').next()?;
    let line = String::from_utf8_lossy(line);
    line.split_whitespace().next().map(String::from)
}

/// The shared libraries `program` loads, as `ldd` resolves them.
fn shared_libraries(program: &Path) -> Option<Vec<String>> {
    let listing = output("ldd", &[&program.to_string_lossy()])?;
    let libraries = listing
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let path = match line.split_once("=>") {
                Some((_, target)) => target.trim(),
                None => line,
            };
            let path = path.split(" (").next()?.trim();
            path.starts_with('/').then(|| normalize(path))
        })
        .collect();
    Some(libraries)
}

/// `path` without `.` and `..` components, which `ldd` reports for libraries
/// found through `$ORIGIN` (`bin/../lib`). Symlinks stay as they are, so the
/// path still matches how grants name it.
fn normalize(path: &str) -> String {
    let mut out = PathBuf::new();
    for component in Path::new(path).components() {
        match component {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out.to_string_lossy().into_owned()
}

/// The C compiler's `#include <...>` search path.
fn include_dirs() -> Vec<String> {
    let Ok(out) = Command::new("cc").args(["-xc", "-E", "-v", "/dev/null"]).output() else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stderr);
    text.lines()
        .skip_while(|line| !line.starts_with("#include <...> search starts here:"))
        .skip(1)
        .take_while(|line| !line.starts_with("End of search list."))
        .map(|line| line.trim().to_string())
        .filter(|dir| Path::new(dir).is_dir())
        .collect()
}
