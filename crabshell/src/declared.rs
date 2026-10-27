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
    parse(&text).map_err(|source| DeclaredError { path, source })
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

/// A malformed `[package.metadata.build-rs]` table, or manifest.
#[derive(Debug)]
pub struct DeclaredError {
    path: PathBuf,
    source: toml::de::Error,
}

impl fmt::Display for DeclaredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failed to parse {}: {}", self.path.display(), self.source)
    }
}

impl std::error::Error for DeclaredError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
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
