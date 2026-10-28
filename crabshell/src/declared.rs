//! Reading what a package declares its build script needs.
//!
//! A package declares its needs in `Cargo.toml`:
//!
//! ```toml
//! [package.metadata.build-rs]
//! read = ["/usr/include"]
//! write = ["/dev/null"]
//! run  = ["pkg-config"]
//! env  = ["TARGET"]
//! requests = ["cc"]
//! ```
//!
//! A declaration only tightens the consumer's policy (see
//! [`crate::policy::tighten`]). Once a package declares anything, unlisted
//! fields request nothing, and `network` defaults to `false`.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// A package's `[package.metadata.build-rs]` table.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declared {
    /// Read-only paths.
    #[serde(default)]
    pub read: Vec<String>,
    /// Writable paths, beyond `OUT_DIR`.
    #[serde(default)]
    pub write: Vec<String>,
    /// Executables the script may run.
    #[serde(default)]
    pub run: Vec<String>,
    /// Environment variables the script may read.
    #[serde(default)]
    pub env: Vec<String>,
    /// Whether the script needs network access.
    #[serde(default)]
    pub network: bool,
    /// Named grants the script may use, which expand to the fields above.
    #[serde(default)]
    pub requests: Vec<String>,
}

/// Read the declaration from the manifest in `manifest_dir`.
///
/// Returns `None` when the manifest has no `[package.metadata.build-rs]`
/// table, or when there is no manifest.
pub fn load(manifest_dir: &Path) -> Result<Option<Declared>, DeclaredError> {
    let path = manifest_dir.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    parse(&text).map_err(|source| DeclaredError::Parse { path, source })
}

/// Read `package`'s declaration from a test file of requests.
///
/// The file maps package names to declarations (`[package.<name>]` tables in
/// the `[package.metadata.build-rs]` format), standing in for what crates we
/// can't edit would declare. Returns `None` for packages the file doesn't list.
#[cfg(feature = "test-requests")]
pub fn injected(path: &Path, package: &str) -> Result<Option<Declared>, DeclaredError> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Requests {
        #[serde(default)]
        package: std::collections::BTreeMap<String, Declared>,
    }
    let text = std::fs::read_to_string(path)
        .map_err(|source| DeclaredError::Read { path: path.to_path_buf(), source })?;
    let requests: Requests = toml::from_str(&text)
        .map_err(|source| DeclaredError::Parse { path: path.to_path_buf(), source })?;
    Ok(requests.package.get(package).cloned())
}

fn parse(text: &str) -> Result<Option<Declared>, toml::de::Error> {
    #[derive(Deserialize)]
    struct Manifest {
        package: Option<Package>,
    }
    #[derive(Deserialize)]
    struct Package {
        metadata: Option<Metadata>,
    }
    #[derive(Deserialize)]
    struct Metadata {
        #[serde(rename = "build-rs")]
        build_rs: Option<Declared>,
    }

    let manifest: Manifest = toml::from_str(text)?;
    Ok(manifest.package.and_then(|p| p.metadata).and_then(|m| m.build_rs))
}

/// A declaration that can't be read or parsed.
#[derive(Debug)]
pub enum DeclaredError {
    /// A requests file could not be read.
    Read { path: PathBuf, source: std::io::Error },
    /// A manifest or requests file was malformed.
    Parse { path: PathBuf, source: toml::de::Error },
}

impl fmt::Display for DeclaredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeclaredError::Read { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            DeclaredError::Parse { path, source } => {
                write!(f, "failed to parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for DeclaredError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DeclaredError::Read { source, .. } => Some(source),
            DeclaredError::Parse { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_declaration() {
        let declared = parse(
            r#"
            [package]
            name = "dep"

            [package.metadata.build-rs]
            run = ["pkg-config"]
            env = ["TARGET"]
            "#,
        )
        .unwrap()
        .unwrap();

        assert_eq!(declared.run, ["pkg-config"]);
        assert_eq!(declared.env, ["TARGET"]);
        assert!(declared.read.is_empty());
        assert!(!declared.network);
    }

    #[test]
    fn a_manifest_without_a_declaration_has_none() {
        assert_eq!(parse("[package]\nname = \"dep\"\n").unwrap(), None);
        assert_eq!(parse("[package.metadata.docs]\nall = true\n").unwrap(), None);
        assert_eq!(parse("[workspace]\n").unwrap(), None);
    }

    #[test]
    fn rejects_unknown_fields() {
        assert!(parse("[package.metadata.build-rs]\nnet = true\n").is_err());
    }
}
