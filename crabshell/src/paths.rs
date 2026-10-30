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
//! Entries may also be glob patterns, in the syntax of the [`glob`] crate:
//! `*` and `?` within a component, `[...]` character classes, and `**` as a
//! whole component for any depth. [`Expander`] expands variables, then globs,
//! before resolution compares anything; env entries match the same syntax
//! against the variables actually set.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use std::cell::RefCell;

use crate::config::{Source, VariableDef};
use crate::policy::Expand;

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

/// Expands entries against the variables, filesystem and environment.
pub struct Expander {
    variables: Variables,
    names: Vec<String>,
    dropped: RefCell<Vec<String>>,
}

impl Expander {
    /// An expander using `variables`, matching env patterns against `names`.
    pub fn new(variables: Variables, names: Vec<String>) -> Self {
        Self { variables, names, dropped: RefCell::new(Vec::new()) }
    }

    /// The entries dropped so far because their variable is undefined.
    pub fn dropped(&self) -> Vec<String> {
        self.dropped.borrow().clone()
    }
}

impl Expand for Expander {
    fn paths(&self, entry: &str) -> Result<Vec<String>, PathError> {
        let Some(expanded) = self.variables.expand(entry)? else {
            self.dropped.borrow_mut().push(entry.to_string());
            return Ok(Vec::new());
        };
        let mut out = Vec::new();
        for path in expanded {
            if is_pattern(&path) {
                out.extend(glob(&path)?.iter().map(|p| p.to_string_lossy().into_owned()));
            } else if path.contains('/') && !Path::new(&path).is_absolute() {
                return Err(PathError::Relative(entry.to_string()));
            } else {
                out.push(path);
            }
        }
        Ok(out)
    }

    fn names(&self, entry: &str) -> Vec<String> {
        if !is_pattern(entry) {
            return vec![entry.to_string()];
        }
        match glob::Pattern::new(entry) {
            Ok(pattern) => self.names.iter().filter(|n| pattern.matches(n)).cloned().collect(),
            Err(_) => Vec::new(),
        }
    }
}

fn is_pattern(entry: &str) -> bool {
    entry.contains(['*', '?', '['])
}

/// The paths matching an absolute glob `pattern`.
///
/// A pattern under a missing directory matches nothing; a directory that
/// exists but can't be listed is an error, since silently granting less would
/// hide the cause.
pub fn glob(pattern: &str) -> Result<Vec<PathBuf>, PathError> {
    if !Path::new(pattern).is_absolute() {
        return Err(PathError::Relative(pattern.to_string()));
    }
    let paths =
        glob::glob(pattern).map_err(|err| PathError::Glob(format!("{pattern}: {}", err.msg)))?;
    let mut matches = Vec::new();
    for path in paths {
        match path {
            Ok(path) => matches.push(path),
            Err(err) if err.error().kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(PathError::List(pattern.to_string(), err.error().to_string())),
        }
    }
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
    /// The glob pattern is malformed.
    Glob(String),
    /// A directory the glob walks exists but can't be listed.
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
            PathError::Glob(detail) => write!(f, "invalid glob pattern `{detail}`"),
            PathError::List(entry, err) => write!(f, "`{entry}`: can't list a directory: {err}"),
        }
    }
}

impl std::error::Error for PathError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EnvImport;
    use crate::policy::Expand;

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
    }

    fn expander(v: Variables) -> Expander {
        Expander::new(
            v,
            ["TARGET", "CARGO_CFG_TARGET_OS", "CARGO_CFG_UNIX", "HOME"].map(String::from).to_vec(),
        )
    }

    #[test]
    fn the_expander_drops_entries_with_undefined_variables() {
        let x = expander(vars(&[]));
        assert_eq!(x.paths(":manifest_dir").unwrap(), ["/src/dep"]);
        assert_eq!(x.paths(":out_dir/gen").unwrap(), Vec::<String>::new());
        assert_eq!(x.paths("cc").unwrap(), ["cc"]);
        assert_eq!(x.dropped(), [":out_dir/gen"]);
    }

    #[test]
    fn the_expander_globs_any_component() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for sub in ["xz-5.2/src", "xz-5.4/src", "other/src", "deep/a/b"] {
            std::fs::create_dir_all(root.join(sub)).unwrap();
        }
        std::fs::write(root.join("deep/a/b/probe.rs"), "").unwrap();
        std::fs::write(root.join("deep/probe.rs"), "").unwrap();
        let x = expander(vars(&[]));
        let root = root.display();

        let mid = x.paths(&format!("{root}/xz-*/src")).unwrap();
        assert_eq!(mid, [format!("{root}/xz-5.2/src"), format!("{root}/xz-5.4/src")]);
        let deep = x.paths(&format!("{root}/deep/**/probe.rs")).unwrap();
        assert_eq!(deep, [format!("{root}/deep/a/b/probe.rs"), format!("{root}/deep/probe.rs")]);
        assert!(matches!(x.paths(&format!("{root}/xz**")), Err(PathError::Glob(_))));
    }

    #[test]
    fn the_expander_matches_env_patterns_against_set_names() {
        let x = expander(vars(&[]));
        assert_eq!(x.names("CARGO_CFG_*"), ["CARGO_CFG_TARGET_OS", "CARGO_CFG_UNIX"]);
        assert_eq!(x.names("OPENSSL_*"), Vec::<String>::new());
        // A plain name stays, set or not.
        assert_eq!(x.names("OPENSSL_DIR"), ["OPENSSL_DIR"]);
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
