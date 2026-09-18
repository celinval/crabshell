//! Discovering and parsing `sandbox.toml` into typed data.
//!
//! Resolving the effective permission set is [`crate::policy`]'s job. [`find`]
//! walks up from a directory and returns the closest `sandbox.toml`; for
//! simplicity only that one file is used.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const FILE_NAME: &str = "sandbox.toml";

/// A parsed `sandbox.toml`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxConfig {
    /// Consumer caps; constrains only the fields it names.
    #[serde(default)]
    pub limits: Fields,
    /// Policy for workspace members.
    #[serde(default)]
    pub workspace: Fields,
    /// Per-package policy; `"*"` is the external-dependency catch-all.
    #[serde(default)]
    pub package: BTreeMap<String, Fields>,
}

/// The fields a policy table may set.
///
/// Backs `[limits]`, `[workspace]`, and each `[package.*]` entry; unset fields
/// fall through during resolution.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    /// Read-only paths.
    pub read: Option<ListField>,
    /// Executables the script may run.
    pub run: Option<ListField>,
    /// Environment variables the script may read.
    pub env: Option<ListField>,
    /// Whether network access is granted.
    pub network: Option<bool>,
}

/// A list field, following Cargo's array-or-table idiom.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum ListField {
    /// An array overriding the inherited value.
    Override(Vec<String>),
    /// A table that, with `inherit = true`, appends `value` to the inherited value.
    Inherit {
        #[serde(default)]
        inherit: bool,
        #[serde(default)]
        value: Vec<String>,
    },
}

/// Return the closest `sandbox.toml` at or above `start`.
pub fn find(start: &Path) -> Option<PathBuf> {
    start.ancestors().map(|dir| dir.join(FILE_NAME)).find(|candidate| candidate.is_file())
}

/// Read and parse the `sandbox.toml` at `path`.
pub fn load(path: &Path) -> Result<SandboxConfig, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    toml::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })
}

/// An error discovering or parsing a `sandbox.toml`.
#[derive(Debug)]
pub enum ConfigError {
    /// The file could not be read.
    Read { path: PathBuf, source: std::io::Error },
    /// The file was not valid TOML or violated the schema.
    Parse { path: PathBuf, source: toml::de::Error },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Read { path, source } => {
                write!(f, "failed to read {}: {source}", path.display())
            }
            ConfigError::Parse { path, source } => {
                write!(f, "failed to parse {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_example() {
        let text = r#"
            [limits]
            network = false

            [workspace]
            read = ["$include_dir", "$lib_dir", "$CARGO_MANIFEST_DIR"]

            [package."*"]
            read = ["$include_dir", "$lib_dir"]

            [package.openssl-sys]
            read = { inherit = true, value = ["$include_dir/openssl"] }
            run = ["cc", "pkg-config"]
        "#;

        let config: SandboxConfig = toml::from_str(text).unwrap();

        assert_eq!(config.limits.network, Some(false));
        assert_eq!(
            config.workspace.read,
            Some(ListField::Override(vec![
                "$include_dir".into(),
                "$lib_dir".into(),
                "$CARGO_MANIFEST_DIR".into(),
            ]))
        );
        assert_eq!(
            config.package["*"].read,
            Some(ListField::Override(vec!["$include_dir".into(), "$lib_dir".into()]))
        );
        assert_eq!(
            config.package["openssl-sys"].read,
            Some(ListField::Inherit {
                inherit: true,
                value: vec!["$include_dir/openssl".into()],
            })
        );
        assert_eq!(
            config.package["openssl-sys"].run,
            Some(ListField::Override(vec!["cc".into(), "pkg-config".into()]))
        );
    }

    #[test]
    fn empty_config_is_all_defaults() {
        let config: SandboxConfig = toml::from_str("").unwrap();
        assert_eq!(config, SandboxConfig::default());
    }

    #[test]
    fn unknown_field_is_rejected() {
        assert!(toml::from_str::<SandboxConfig>("[limits]\nbogus = 1\n").is_err());
    }

    #[test]
    fn find_walks_up_to_first_config() {
        let root = tempfile::tempdir().unwrap();
        let nested = root.path().join("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.path().join(FILE_NAME), "").unwrap();

        assert_eq!(find(&nested), Some(root.path().join(FILE_NAME)));
    }

    #[test]
    fn find_prefers_the_closest_config() {
        let root = tempfile::tempdir().unwrap();
        let mid = root.path().join("a");
        let nested = mid.join("b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.path().join(FILE_NAME), "").unwrap();
        std::fs::write(mid.join(FILE_NAME), "").unwrap();

        assert_eq!(find(&nested), Some(mid.join(FILE_NAME)));
    }

    #[test]
    fn find_returns_none_when_absent() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(find(root.path()), None);
    }
}
