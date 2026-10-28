//! Discovering and parsing `sandbox.toml` into typed data.
//!
//! Resolving the effective permission set is [`crate::policy`]'s job. [`find`]
//! walks up from a directory and returns the closest `sandbox.toml`; for
//! simplicity only that one file is used.
//!
//! A package falls into one of two categories, `[local]` or `[external]`, with
//! `[package.<name>]` overriding both. Glob keys under `[package]` are rejected
//! rather than ignored, so a table that would silently grant nothing cannot be
//! mistaken for an applied policy. `[grant.<name>]` tables define or override
//! [named grants](crate::grants).

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
    /// Policy for workspace members and path dependencies.
    #[serde(default)]
    pub local: Fields,
    /// Policy for registry and git dependencies.
    #[serde(default)]
    pub external: Fields,
    /// Per-package policy, overriding the category tables.
    #[serde(default)]
    pub package: BTreeMap<String, Fields>,
    /// Named grants, overriding built-ins of the same name per field.
    #[serde(default)]
    pub grant: BTreeMap<String, Fields>,
}

/// The fields a policy table may set.
///
/// Backs `[limits]`, the category tables, and each `[package.<name>]` entry;
/// unset fields fall through during resolution.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    /// Read-only paths.
    pub read: Option<ListField>,
    /// Writable paths, beyond `OUT_DIR`.
    pub write: Option<ListField>,
    /// Executables the script may run.
    pub run: Option<ListField>,
    /// Environment variables the script may read.
    pub env: Option<ListField>,
    /// Whether network access is granted.
    pub network: Option<bool>,
    /// Named grants to apply, which expand to the fields above.
    pub grants: Option<ListField>,
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

/// Read, parse, and validate the `sandbox.toml` at `path`.
pub fn load(path: &Path) -> Result<SandboxConfig, ConfigError> {
    let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    let config: SandboxConfig = toml::from_str(&text).map_err(|source| ConfigError::Parse {
        path: path.to_path_buf(),
        source,
    })?;
    validate(&config, path)?;
    Ok(config)
}

/// Reject tables that would never apply, and grant names that don't resolve.
///
/// Glob patterns are not supported yet, and a `[package.<glob>]` table that is
/// quietly ignored reads like an applied restriction while granting nothing.
fn validate(config: &SandboxConfig, path: &Path) -> Result<(), ConfigError> {
    let error = |kind| ConfigError::Grant { path: path.to_path_buf(), kind };
    for key in config.package.keys() {
        if key.contains('*') {
            return Err(ConfigError::GlobKey {
                path: path.to_path_buf(),
                key: key.clone(),
            });
        }
    }
    for (name, grant) in &config.grant {
        if !crate::grants::is_valid_name(name) {
            return Err(error(GrantError::InvalidName(name.clone())));
        }
        if grant.grants.is_some() {
            return Err(error(GrantError::Nested(name.clone())));
        }
    }
    if config.limits.grants.is_some() {
        return Err(error(GrantError::InLimits));
    }
    let known = crate::grants::effective(config);
    let tables = [&config.local, &config.external].into_iter().chain(config.package.values());
    for name in tables.filter_map(|t| t.grants.as_ref()).flat_map(list_entries) {
        if !crate::grants::is_valid_name(name) {
            return Err(error(GrantError::InvalidName(name.clone())));
        }
        if !known.contains_key(name) {
            return Err(error(GrantError::Unknown(name.clone())));
        }
    }
    Ok(())
}

fn list_entries(field: &ListField) -> &[String] {
    match field {
        ListField::Override(value) => value,
        ListField::Inherit { value, .. } => value,
    }
}

/// An error discovering or parsing a `sandbox.toml`.
#[derive(Debug)]
pub enum ConfigError {
    /// The file could not be read.
    Read { path: PathBuf, source: std::io::Error },
    /// The file was not valid TOML or violated the schema.
    Parse { path: PathBuf, source: toml::de::Error },
    /// A `[package.<name>]` key used an unsupported glob pattern.
    GlobKey { path: PathBuf, key: String },
    /// A named grant is invalid or used where it can't apply.
    Grant { path: PathBuf, kind: GrantError },
}

/// What is wrong with a named grant in `sandbox.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantError {
    /// The name isn't `[a-z][a-z0-9_-]*`.
    InvalidName(String),
    /// A policy table grants a name that is neither built in nor defined.
    Unknown(String),
    /// A `[grant.<name>]` table itself sets `grants`.
    Nested(String),
    /// `[limits]` sets `grants`; limits cap fields, they don't grant.
    InLimits,
}

impl fmt::Display for GrantError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GrantError::InvalidName(name) => {
                write!(f, "`{name}` is not a valid grant name; use `[a-z][a-z0-9_-]*`")
            }
            GrantError::Unknown(name) => {
                write!(f, "unknown grant `{name}`; define it with `[grant.{name}]`")
            }
            GrantError::Nested(name) => write!(f, "`[grant.{name}]` can't set `grants`"),
            GrantError::InLimits => write!(f, "`[limits]` can't set `grants`"),
        }
    }
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
            ConfigError::Grant { path, kind } => write!(f, "{}: {kind}", path.display()),
            ConfigError::GlobKey { path, key } => write!(
                f,
                "{}: `[package.\"{key}\"]` is not supported; \
                 use `[external]` for dependencies, or `[local]` for workspace \
                 members and path dependencies",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Read { source, .. } => Some(source),
            ConfigError::Parse { source, .. } => Some(source),
            ConfigError::GlobKey { .. } | ConfigError::Grant { .. } => None,
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

            [local]
            read = ["$include_dir", "$lib_dir", "$CARGO_MANIFEST_DIR"]

            [external]
            read = ["$include_dir", "$lib_dir"]

            [package.openssl-sys]
            read = { inherit = true, value = ["$include_dir/openssl"] }
            run = ["cc", "pkg-config"]
        "#;

        let config: SandboxConfig = toml::from_str(text).unwrap();

        assert_eq!(config.limits.network, Some(false));
        assert_eq!(
            config.local.read,
            Some(ListField::Override(vec![
                "$include_dir".into(),
                "$lib_dir".into(),
                "$CARGO_MANIFEST_DIR".into(),
            ]))
        );
        assert_eq!(
            config.external.read,
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

    /// Write `text` to a `sandbox.toml` in a fresh directory and load it.
    fn load_text(text: &str) -> (tempfile::TempDir, Result<SandboxConfig, ConfigError>) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        std::fs::write(&path, text).unwrap();
        let loaded = load(&path);
        (dir, loaded)
    }

    #[test]
    fn catch_all_package_key_is_rejected() {
        let (_dir, loaded) = load_text("[package.\"*\"]\nnetwork = false\n");
        let err = loaded.unwrap_err();

        assert!(matches!(&err, ConfigError::GlobKey { key, .. } if key == "*"));
        let message = err.to_string();
        assert!(message.contains("[external]"), "{message}");
        assert!(message.contains("[local]"), "{message}");
    }

    #[test]
    fn partial_glob_package_key_is_rejected() {
        let (_dir, loaded) = load_text("[package.\"*-sys\"]\nnetwork = false\n");
        assert!(matches!(loaded.unwrap_err(), ConfigError::GlobKey { key, .. } if key == "*-sys"));
    }

    #[test]
    fn plain_package_key_is_accepted() {
        let (_dir, loaded) = load_text("[package.openssl-sys]\nnetwork = true\n");
        assert_eq!(loaded.unwrap().package["openssl-sys"].network, Some(true));
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

    fn load_err(text: &str) -> String {
        let (_dir, loaded) = load_text(text);
        loaded.unwrap_err().to_string()
    }

    #[test]
    fn accepts_builtin_and_defined_grants() {
        let (_dir, loaded) = load_text(
            "[external]\ngrants = [\"cc\", \"protoc\"]\n\n[grant.protoc]\nrun = [\"protoc\"]\n",
        );
        assert!(loaded.is_ok(), "{loaded:?}");
    }

    #[test]
    fn rejects_bad_grants() {
        assert!(load_err("[external]\ngrants = [\"nope\"]\n").contains("unknown grant `nope`"));
        assert!(load_err("[grant.Bad]\nrun = []\n").contains("not a valid grant name"));
        assert!(load_err("[grant.x]\ngrants = [\"cc\"]\n").contains("can't set `grants`"));
        assert!(load_err("[limits]\ngrants = [\"cc\"]\n").contains("`[limits]` can't set"));
    }
}
