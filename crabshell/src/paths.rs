//! Turning policy path entries into concrete paths.
//!
//! An entry is an absolute path, optionally starting with a `$NAME` root that
//! resolves from the build script's environment (e.g. `$CARGO_MANIFEST_DIR/src`).
//! The last component may end in `*` to match every sibling with that prefix
//! (e.g. `/lib*`). Role-based roots such as `$include_dir` are not supported yet.

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;

/// Variables a `$NAME` root may refer to.
pub const ROOTS: &[&str] = &["CARGO_MANIFEST_DIR", "OUT_DIR", "CARGO_HOME", "HOME", "RUSTUP_HOME"];

/// Expand `entry` into the paths it names.
///
/// `var` looks up a root's value. A glob that matches nothing expands to no
/// paths.
pub fn expand(
    entry: &str,
    var: impl Fn(&str) -> Option<OsString>,
) -> Result<Vec<PathBuf>, PathError> {
    let path = match entry.strip_prefix('$') {
        Some(rest) => {
            let (name, tail) = rest.split_once('/').unwrap_or((rest, ""));
            if !ROOTS.contains(&name) {
                return Err(PathError::UnknownRoot(entry.to_string()));
            }
            let root = var(name).ok_or_else(|| PathError::UnsetRoot(entry.to_string()))?;
            let mut path = PathBuf::from(root);
            if !tail.is_empty() {
                path.push(tail);
            }
            path
        }
        None => PathBuf::from(entry),
    };
    if !path.is_absolute() {
        return Err(PathError::Relative(entry.to_string()));
    }
    glob(path)
}

/// Expand a trailing `prefix*` in the last component; other paths pass through.
fn glob(path: PathBuf) -> Result<Vec<PathBuf>, PathError> {
    let Some(prefix) = path.file_name().and_then(|n| n.to_str()).and_then(|n| n.strip_suffix('*'))
    else {
        return Ok(vec![path]);
    };
    if prefix.contains('*') {
        return Err(PathError::Glob(path.display().to_string()));
    }
    let parent = path.parent().unwrap_or(&path);
    let Ok(entries) = std::fs::read_dir(parent) else {
        return Ok(Vec::new());
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
    /// `$NAME` is not one of [`ROOTS`].
    UnknownRoot(String),
    /// `$NAME` is a known root, but the variable is unset.
    UnsetRoot(String),
    /// The entry is neither absolute nor rooted.
    Relative(String),
    /// The glob is not a single trailing `*`.
    Glob(String),
}

impl fmt::Display for PathError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PathError::UnknownRoot(entry) => {
                write!(f, "`{entry}`: unknown root; expected one of ${}", ROOTS.join(", $"))
            }
            PathError::UnsetRoot(entry) => write!(f, "`{entry}`: root variable is not set"),
            PathError::Relative(entry) => {
                write!(f, "`{entry}`: path must be absolute or `$ROOT/...`")
            }
            PathError::Glob(entry) => write!(f, "`{entry}`: only a trailing `*` is supported"),
        }
    }
}

impl std::error::Error for PathError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(name: &str) -> Option<OsString> {
        (name == "HOME").then(|| "/home/me".into())
    }

    #[test]
    fn expands_a_root() {
        assert_eq!(expand("$HOME/.cargo", vars).unwrap(), [PathBuf::from("/home/me/.cargo")]);
        assert_eq!(expand("$HOME", vars).unwrap(), [PathBuf::from("/home/me")]);
    }

    #[test]
    fn passes_absolute_paths_through() {
        assert_eq!(expand("/usr/include", vars).unwrap(), [PathBuf::from("/usr/include")]);
    }

    #[test]
    fn rejects_bad_entries() {
        assert_eq!(
            expand("$include_dir", vars),
            Err(PathError::UnknownRoot("$include_dir".into()))
        );
        assert_eq!(expand("$OUT_DIR", vars), Err(PathError::UnsetRoot("$OUT_DIR".into())));
        assert_eq!(expand("usr/lib", vars), Err(PathError::Relative("usr/lib".into())));
        assert_eq!(expand("/a/*b*", vars), Err(PathError::Glob("/a/*b*".into())));
    }

    #[test]
    fn expands_a_trailing_glob() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["lib", "lib64", "other"] {
            std::fs::create_dir(dir.path().join(name)).unwrap();
        }
        let entry = format!("{}/lib*", dir.path().display());
        assert_eq!(
            expand(&entry, vars).unwrap(),
            [dir.path().join("lib"), dir.path().join("lib64")]
        );
    }
}
