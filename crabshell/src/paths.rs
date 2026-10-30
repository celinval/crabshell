//! Turning policy path entries into concrete paths.
//!
//! A policy path is absolute, or a variable plus a subpath:
//! `:manifest_dir/build/probe.rs`. Variable names match `[a-z][a-z0-9_]*`.
//! Built-in variables come from what Cargo tells the build script; user and
//! project config define more in `[variables]`, importing environment
//! variables only there. Policy paths never read the environment directly, so
//! `$NAME` is an error.
//!
//! A variable's value is a list of paths, and `:name/rest` expands to
//! `path/rest` for each of them. An imported environment variable is read as
//! a path list like `PATH`, so a single path is a one-element list, and a path
//! containing `:` can't be imported.
//!
//! [`Variables::apply`] expands variables after resolution and narrowing,
//! which compare entries as written. The runner then expands globs with
//! [`glob`]: a trailing `*` in the last component matches every sibling with
//! that prefix (`/lib*`).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::config::{Source, VariableDef};
use crate::policy::Permissions;

/// Variables crabshell defines, from the build script's environment.
pub const BUILTIN: &[&str] = &["manifest_dir", "manifest_path", "out_dir", "cargo_home", "sysroot"];

/// Environment variables whose values a crate, not the user, chooses.
///
/// A definition importing one could let a crate widen grants: `DEP_*` comes
/// from dependencies' build scripts, the others from the crate's manifest.
pub fn is_crate_chosen(env: &str) -> bool {
    env.starts_with("DEP_") || env.starts_with("CARGO_PKG_") || env == "CARGO_MANIFEST_LINKS"
}

/// Whether `name` is a valid variable name: `[a-z][a-z0-9_]*`.
pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// The values of every variable a build script's policy may use.
///
/// A variable is undefined when nothing provides it, e.g. `:out_dir` without
/// `OUT_DIR`; entries using it are dropped rather than failing the build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variables {
    values: BTreeMap<String, Option<Vec<PathBuf>>>,
}

impl Variables {
    /// Evaluate the built-ins and `defs`, reading the environment through `env`.
    ///
    /// `defs` comes from validated config, so names are valid and imports are
    /// user-chosen.
    pub fn new(
        defs: &BTreeMap<String, VariableDef>,
        env: impl Fn(&str) -> Option<OsString>,
    ) -> Self {
        let path = |name: &str| env(name).filter(|v| !v.is_empty()).map(|v| vec![PathBuf::from(v)]);
        let mut values = BTreeMap::new();
        values.insert("manifest_dir".to_string(), path("CARGO_MANIFEST_DIR"));
        values.insert("manifest_path".to_string(), path("CARGO_MANIFEST_PATH"));
        values.insert("out_dir".to_string(), path("OUT_DIR"));
        values.insert("cargo_home".to_string(), path("CARGO_HOME"));
        values.insert("sysroot".to_string(), sysroot(env("RUSTC"), env("PATH")).map(|p| vec![p]));
        for (name, def) in defs {
            values.insert(name.clone(), evaluate(def, &env));
        }
        Self { values }
    }

    /// Expand a policy entry; `None` when its variable is undefined.
    ///
    /// Absolute paths and bare program names (`cc`) pass through unchanged.
    pub fn expand(&self, entry: &str) -> Result<Option<Vec<String>>, PathError> {
        if entry.starts_with('$') {
            return Err(PathError::EnvInPath(entry.to_string()));
        }
        let Some(rest) = entry.strip_prefix(':') else {
            return Ok(Some(vec![entry.to_string()]));
        };
        let (name, tail) = rest.split_once('/').unwrap_or((rest, ""));
        if !is_valid_name(name) {
            return Err(PathError::InvalidName(entry.to_string()));
        }
        let Some(value) = self.values.get(name) else {
            return Err(PathError::Unknown(entry.to_string()));
        };
        Ok(value.as_ref().map(|roots| {
            roots
                .iter()
                .map(|root| if tail.is_empty() { root.clone() } else { root.join(tail) })
                .map(|path| path.to_string_lossy().into_owned())
                .collect()
        }))
    }

    /// Expand every path entry in `permissions`.
    ///
    /// Returns the expanded permissions and the entries dropped because their
    /// variable is undefined.
    pub fn apply(&self, permissions: Permissions) -> Result<(Permissions, Vec<String>), PathError> {
        let mut dropped = Vec::new();
        let mut list = |entries: Vec<String>| -> Result<Vec<String>, PathError> {
            let mut out = Vec::new();
            for entry in entries {
                match self.expand(&entry)? {
                    Some(paths) => out.extend(paths),
                    None => dropped.push(entry),
                }
            }
            Ok(out)
        };
        let expanded = Permissions {
            read: list(permissions.read)?,
            write: list(permissions.write)?,
            run: list(permissions.run)?,
            list: list(permissions.list)?,
            ..permissions
        };
        Ok((expanded, dropped))
    }
}

/// The paths of a user-defined variable: the union of its sources'.
///
/// `None` when no source is defined, i.e. every imported env var is unset and
/// has no default.
fn evaluate(def: &VariableDef, env: &impl Fn(&str) -> Option<OsString>) -> Option<Vec<PathBuf>> {
    let mut paths: Option<Vec<PathBuf>> = None;
    for source in def.sources() {
        let found = match source {
            Source::Path(path) => Some(vec![PathBuf::from(path)]),
            Source::Env(import) => match env(&import.env).filter(|v| !v.is_empty()) {
                Some(value) => Some(
                    std::env::split_paths(&value).filter(|p| !p.as_os_str().is_empty()).collect(),
                ),
                None => import.default.as_deref().and_then(|default| evaluate(default, env)),
            },
        };
        if let Some(found) = found {
            paths.get_or_insert_with(Vec::new).extend(found);
        }
    }
    paths
}

/// The toolchain root holding `rustc` as `bin/rustc`, found through `RUSTC`.
///
/// A bare `RUSTC` is looked up on `PATH`. Under rustup's proxy (`~/.cargo/bin`)
/// this finds the proxy rather than a toolchain; Cargo normally passes the
/// toolchain's own `rustc`.
fn sysroot(rustc: Option<OsString>, path: Option<OsString>) -> Option<PathBuf> {
    let rustc = PathBuf::from(rustc?);
    let rustc = if rustc.components().count() > 1 {
        rustc
    } else {
        std::env::split_paths(&path?).map(|dir| dir.join(&rustc)).find(|p| p.is_file())?
    };
    rustc.parent()?.parent().map(Path::to_path_buf)
}

/// Expand a trailing `prefix*` in the last component; other paths pass through.
///
/// A glob under a missing directory matches nothing; a directory that exists
/// but can't be listed is an error, since silently granting less would hide
/// the cause.
pub fn glob(entry: &str) -> Result<Vec<PathBuf>, PathError> {
    let path = PathBuf::from(entry);
    if !path.is_absolute() {
        return Err(PathError::Relative(entry.to_string()));
    }
    let Some(prefix) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix('*'))
    else {
        return Ok(vec![path]);
    };
    if prefix.contains('*') {
        return Err(PathError::Glob(entry.to_string()));
    }
    let parent = path.parent().unwrap_or(&path);
    let entries = match std::fs::read_dir(parent) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(PathError::List(entry.to_string(), err.to_string())),
    };
    let mut matches: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with(prefix)))
        .map(|e| e.path())
        .collect();
    matches.sort();
    Ok(matches)
}

/// A policy path entry that cannot be expanded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// The entry uses `$NAME`; policy paths don't read the environment.
    EnvInPath(String),
    /// The variable name isn't `[a-z][a-z0-9_]*`.
    InvalidName(String),
    /// No built-in or `[variables]` entry has that name.
    Unknown(String),
    /// The entry is neither absolute nor a variable.
    Relative(String),
    /// The glob is not a single trailing `*`.
    Glob(String),
    /// The glob's parent dir exists but can't be listed.
    List(String, String),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::EnvInPath(entry) => write!(
                f,
                "`{entry}`: paths can't read env vars; use a variable such as `:manifest_dir`, \
                 or define one in `[variables]` with `{{ env = \"NAME\" }}`"
            ),
            PathError::InvalidName(entry) => {
                write!(f, "`{entry}`: variable names are `[a-z][a-z0-9_]*`")
            }
            PathError::Unknown(entry) => write!(
                f,
                "`{entry}`: unknown variable; built-ins are :{}, others need `[variables]`",
                BUILTIN.join(", :")
            ),
            PathError::Relative(entry) => {
                write!(f, "`{entry}`: path must be absolute or start with a `:variable`")
            }
            PathError::Glob(entry) => write!(f, "`{entry}`: only a trailing `*` is supported"),
            PathError::List(entry, err) => write!(f, "`{entry}`: can't list its parent dir: {err}"),
        }
    }
}

impl std::error::Error for PathError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EnvImport;

    fn env(name: &str) -> Option<OsString> {
        match name {
            "CARGO_MANIFEST_DIR" => Some("/src/dep".into()),
            "RUSTC" => Some("/tc/nightly/bin/rustc".into()),
            "OPENSSL_DIR" => Some("/opt/ssl".into()),
            "PKG_CONFIG_PATH" => Some("/a/pc:/b/pc".into()),
            _ => None,
        }
    }

    fn vars(defs: &[(&str, VariableDef)]) -> Variables {
        let defs = defs.iter().map(|(n, d)| (n.to_string(), d.clone())).collect();
        Variables::new(&defs, env)
    }

    fn import(name: &str, default: Option<VariableDef>) -> Source {
        Source::Env(EnvImport { env: name.to_string(), default: default.map(Box::new) })
    }

    fn path(path: &str) -> Source {
        Source::Path(path.to_string())
    }

    #[test]
    fn expands_builtins() {
        let v = vars(&[]);
        assert_eq!(v.expand(":manifest_dir/build").unwrap(), Some(vec!["/src/dep/build".into()]));
        assert_eq!(v.expand(":sysroot").unwrap(), Some(vec!["/tc/nightly".into()]));
        // OUT_DIR isn't set, so `:out_dir` is undefined.
        assert_eq!(v.expand(":out_dir/x").unwrap(), None);
    }

    #[test]
    fn passes_other_entries_through() {
        let v = vars(&[]);
        assert_eq!(v.expand("/usr/include").unwrap(), Some(vec!["/usr/include".into()]));
        assert_eq!(v.expand("cc").unwrap(), Some(vec!["cc".into()]));
    }

    #[test]
    fn arrays_union_their_sources() {
        let def = VariableDef::Many(vec![
            import("PKG_CONFIG_PATH", None),
            import("UNSET", None),
            path("/usr/lib64/pkgconfig"),
        ]);
        let v = vars(&[("pc", def)]);
        let expected = ["/a/pc", "/b/pc", "/usr/lib64/pkgconfig"].map(String::from).to_vec();
        assert_eq!(v.expand(":pc").unwrap(), Some(expected));
    }

    #[test]
    fn an_unset_import_uses_its_default() {
        let set = VariableDef::One(import("OPENSSL_DIR", Some(VariableDef::One(path("/usr")))));
        let unset = VariableDef::One(import("UNSET", Some(VariableDef::One(path("/usr")))));
        let none = VariableDef::One(import("UNSET", None));
        let v = vars(&[("set", set), ("unset", unset), ("none", none)]);
        assert_eq!(v.expand(":set/include").unwrap(), Some(vec!["/opt/ssl/include".into()]));
        assert_eq!(v.expand(":unset/include").unwrap(), Some(vec!["/usr/include".into()]));
        assert_eq!(v.expand(":none/include").unwrap(), None);
    }

    #[test]
    fn expansion_applies_the_subpath_to_every_path() {
        let def = VariableDef::Many(vec![path("/usr/lib64"), path("/usr/lib/x86_64-linux-gnu")]);
        let v = vars(&[("lib_dir", def)]);
        let expected = ["/usr/lib64/libz.so.1", "/usr/lib/x86_64-linux-gnu/libz.so.1"];
        assert_eq!(
            v.expand(":lib_dir/libz.so.1").unwrap(),
            Some(expected.map(String::from).to_vec())
        );
    }

    #[test]
    fn rejects_bad_entries() {
        let v = vars(&[]);
        assert_eq!(v.expand("$HOME/x"), Err(PathError::EnvInPath("$HOME/x".into())));
        assert_eq!(v.expand(":Bad"), Err(PathError::InvalidName(":Bad".into())));
        assert_eq!(v.expand(":nope/x"), Err(PathError::Unknown(":nope/x".into())));
        assert_eq!(glob("usr/lib"), Err(PathError::Relative("usr/lib".into())));
        assert_eq!(glob("/a/*b*"), Err(PathError::Glob("/a/*b*".into())));
    }

    #[test]
    fn apply_drops_entries_with_undefined_variables() {
        let permissions = Permissions {
            read: vec![":manifest_dir".into(), ":out_dir/gen".into()],
            run: vec!["cc".into()],
            ..Permissions::default()
        };
        let (expanded, dropped) = vars(&[]).apply(permissions).unwrap();
        assert_eq!(expanded.read, ["/src/dep"]);
        assert_eq!(expanded.run, ["cc"]);
        assert_eq!(dropped, [":out_dir/gen"]);
    }

    #[test]
    fn sysroot_is_found_on_path_for_a_bare_rustc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("bin")).unwrap();
        std::fs::write(dir.path().join("bin/rustc"), "").unwrap();
        let found = sysroot(Some("rustc".into()), Some(dir.path().join("bin").into()));
        assert_eq!(found, Some(dir.path().to_path_buf()));
    }

    #[test]
    fn expands_a_trailing_glob() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["lib", "lib64", "other"] {
            std::fs::create_dir(dir.path().join(name)).unwrap();
        }
        let entry = format!("{}/lib*", dir.path().display());
        assert_eq!(glob(&entry).unwrap(), [dir.path().join("lib"), dir.path().join("lib64")]);
    }

    #[test]
    fn a_glob_under_a_missing_dir_matches_nothing() {
        assert_eq!(glob("/nonexistent/crabshell/lib*").unwrap(), Vec::<PathBuf>::new());
    }

    #[test]
    fn validates_names_and_imports() {
        assert!(is_valid_name("openssl_dir2"));
        assert!(!is_valid_name("openssl-dir"));
        assert!(!is_valid_name("_x"));
        assert!(is_crate_chosen("DEP_OPENSSL_INCLUDE"));
        assert!(is_crate_chosen("CARGO_PKG_DESCRIPTION"));
        assert!(!is_crate_chosen("OPENSSL_DIR"));
        assert!(!is_crate_chosen("CARGO_HOME"));
    }
}
