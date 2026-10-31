//! Locating and parsing policy files into typed data.
//!
//! Resolving the effective permission set is [`crate::policy`]'s job. Two
//! files can apply: the project's, named by `CRABSHELL_CONFIG`, and the user's
//! own, at [`user_config_path`]. crabshell never searches near a package for
//! a `sandbox.toml`: a dependency could ship one and grant itself anything.
//!
//! A package falls into one of two categories, `[local]` or `[external]`, with
//! `[package.<name>]` overriding both. Glob keys under `[package]` are rejected
//! rather than ignored, so a table that would silently grant nothing cannot be
//! mistaken for an applied policy. `[grant.<name>]` tables define or override
//! [named grants](crate::grants).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::Deserialize;

pub const FILE_NAME: &str = "sandbox.toml";

/// A parsed `sandbox.toml`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
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
    #[schemars(extend("propertyNames" = { "pattern": "^[^*]+$" }))]
    pub package: BTreeMap<String, Fields>,
    /// Named grants, overriding built-ins of the same name per field.
    #[serde(default)]
    #[schemars(extend("propertyNames" = { "pattern": "^[a-z][a-z0-9_-]*$" }))]
    pub grant: BTreeMap<String, Fields>,
    /// User-defined path variables, used in policy paths as `:name`.
    #[serde(default)]
    #[schemars(extend("propertyNames" = { "pattern": "^[a-z][a-z0-9_]*$" }))]
    pub variables: BTreeMap<String, VariableDef>,
}

/// A `[variables]` entry: one source, or an array whose sources all apply.
///
/// A variable's value is a list of paths; an array is the union of its
/// sources' paths.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum VariableDef {
    One(Source),
    Many(Vec<Source>),
}

/// Where some of a variable's paths come from.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum Source {
    /// An absolute path.
    Path(String),
    /// The paths in an environment variable, or a default when it's unset.
    Env(EnvImport),
}

/// `{ env = "NAME", default = ... }`: import an environment variable.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvImport {
    /// The environment variable to read, as a list of paths like `PATH`.
    pub env: String,
    /// The value when `env` is unset or empty, in the same forms as a definition.
    pub default: Option<Box<VariableDef>>,
}

/// The fields a policy table may set.
///
/// Backs `[limits]`, the category tables, and each `[package.<name>]` entry;
/// unset fields fall through during resolution.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fields {
    /// Read-only paths.
    pub read: Option<ListField>,
    /// Write-only paths, beyond `OUT_DIR` and `TMPDIR`.
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(untagged, deny_unknown_fields)]
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

/// Where the user's own config lives, which caps every project's policy.
///
/// `$XDG_CONFIG_HOME/crabshell/config.toml`, or `~/.config/crabshell/config.toml`
/// when `XDG_CONFIG_HOME` is unset or not absolute, as the XDG spec says.
pub fn user_config_path(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let xdg = env("XDG_CONFIG_HOME").map(PathBuf::from).filter(|dir| dir.is_absolute());
    let base = match xdg {
        Some(dir) => dir,
        None => PathBuf::from(env("HOME")?).join(".config"),
    };
    Some(base.join("crabshell").join("config.toml"))
}

impl SandboxConfig {
    /// Take `other`'s machine facts, `[variables]` and `[grant.*]`, over this
    /// config's own of the same name.
    pub fn take_facts(&mut self, other: &SandboxConfig) {
        self.variables.extend(other.variables.clone());
        self.grant.extend(other.grant.clone());
    }
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
    let error = |kind| ConfigError::Grant {
        path: path.to_path_buf(),
        kind,
    };
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
    for (name, def) in &config.variables {
        validate_variable(name, def).map_err(|kind| ConfigError::Variable {
            path: path.to_path_buf(),
            name: name.clone(),
            kind,
        })?;
    }
    let known = crate::grants::effective(config);
    let tables = [&config.local, &config.external]
        .into_iter()
        .chain(config.package.values());
    for name in tables
        .filter_map(|t| t.grants.as_ref())
        .flat_map(list_entries)
    {
        if !crate::grants::is_valid_name(name) {
            return Err(error(GrantError::InvalidName(name.clone())));
        }
        if !known.contains_key(name) {
            return Err(error(GrantError::Unknown(name.clone())));
        }
    }
    Ok(())
}

fn validate_variable(name: &str, def: &VariableDef) -> Result<(), VariableError> {
    if !crate::paths::is_valid_name(name) {
        return Err(VariableError::InvalidName);
    }
    if crate::paths::BUILTIN.contains(&name) {
        return Err(VariableError::Builtin);
    }
    validate_sources(def)
}

fn validate_sources(def: &VariableDef) -> Result<(), VariableError> {
    for source in def.sources() {
        match source {
            Source::Path(path) if !Path::new(path).is_absolute() => {
                return Err(VariableError::NotAbsolute(path.clone()));
            }
            Source::Path(_) => {}
            Source::Env(import) => {
                if crate::paths::is_crate_chosen(&import.env) {
                    return Err(VariableError::CrateChosen(import.env.clone()));
                }
                if let Some(default) = &import.default {
                    validate_sources(default)?;
                }
            }
        }
    }
    Ok(())
}

impl VariableDef {
    /// The sources this definition unions.
    pub fn sources(&self) -> &[Source] {
        match self {
            VariableDef::One(source) => std::slice::from_ref(source),
            VariableDef::Many(sources) => sources,
        }
    }
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
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The file was not valid TOML or violated the schema.
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
    /// A `[package.<name>]` key used an unsupported glob pattern.
    GlobKey { path: PathBuf, key: String },
    /// A named grant is invalid or used where it can't apply.
    Grant { path: PathBuf, kind: GrantError },
    /// A `[variables]` entry is invalid.
    Variable {
        path: PathBuf,
        name: String,
        kind: VariableError,
    },
}

/// What is wrong with a `[variables]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariableError {
    /// The name isn't `[a-z][a-z0-9_]*`.
    InvalidName,
    /// The name is a built-in variable.
    Builtin,
    /// A path source isn't absolute.
    NotAbsolute(String),
    /// It imports an env var whose value a crate chooses (`DEP_*`, `CARGO_PKG_*`).
    CrateChosen(String),
}

impl fmt::Display for VariableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VariableError::InvalidName => write!(f, "variable names are `[a-z][a-z0-9_]*`"),
            VariableError::Builtin => write!(f, "built-in variables can't be redefined"),
            VariableError::NotAbsolute(path) => write!(f, "`{path}` must be an absolute path"),
            VariableError::CrateChosen(env) => write!(
                f,
                "`{env}` is chosen by a crate, not the user, so it can't define a variable"
            ),
        }
    }
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
                write!(
                    f,
                    "`{name}` is not a valid grant name; use `[a-z][a-z0-9_-]*`"
                )
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
            ConfigError::Variable { path, name, kind } => {
                write!(f, "{}: `[variables] {name}`: {kind}", path.display())
            }
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
            ConfigError::GlobKey { .. }
            | ConfigError::Grant { .. }
            | ConfigError::Variable { .. } => None,
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
            read = [":include_dir", ":lib_dir", ":manifest_dir"]

            [external]
            read = [":include_dir", ":lib_dir"]

            [package.openssl-sys]
            read = { inherit = true, value = [":include_dir/openssl"] }
            run = ["cc", "pkg-config"]
        "#;

        let config: SandboxConfig = toml::from_str(text).unwrap();

        assert_eq!(config.limits.network, Some(false));
        assert_eq!(
            config.local.read,
            Some(ListField::Override(vec![
                ":include_dir".into(),
                ":lib_dir".into(),
                ":manifest_dir".into(),
            ]))
        );
        assert_eq!(
            config.external.read,
            Some(ListField::Override(vec![
                ":include_dir".into(),
                ":lib_dir".into()
            ]))
        );
        assert_eq!(
            config.package["openssl-sys"].read,
            Some(ListField::Inherit {
                inherit: true,
                value: vec![":include_dir/openssl".into()],
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
    fn user_config_follows_xdg() {
        let env = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| pairs.iter().find(|(k, _)| *k == name).map(|(_, v)| OsString::from(v))
        };
        let xdg = user_config_path(env(&[("XDG_CONFIG_HOME", "/x"), ("HOME", "/h")]));
        assert_eq!(xdg, Some(PathBuf::from("/x/crabshell/config.toml")));
        let relative = user_config_path(env(&[("XDG_CONFIG_HOME", "x"), ("HOME", "/h")]));
        assert_eq!(relative, Some(PathBuf::from("/h/.config/crabshell/config.toml")));
        assert_eq!(user_config_path(env(&[])), None);
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

    #[test]
    fn parses_variables() {
        let (_dir, loaded) = load_text(
            r#"
            [variables]
            openssl_dir = { env = "OPENSSL_DIR", default = "/usr" }
            pc = [{ env = "PKG_CONFIG_PATH" }, "/usr/lib64/pkgconfig"]
            fixed = "/opt/x"
            "#,
        );
        let config = loaded.unwrap();
        assert_eq!(
            config.variables["openssl_dir"],
            VariableDef::One(Source::Env(EnvImport {
                env: "OPENSSL_DIR".into(),
                default: Some(Box::new(VariableDef::One(Source::Path("/usr".into())))),
            }))
        );
        assert_eq!(config.variables["pc"].sources().len(), 2);
        assert_eq!(config.variables["fixed"], VariableDef::One(Source::Path("/opt/x".into())));
    }

    #[test]
    fn rejects_bad_variables() {
        assert!(load_err("[variables]\nOpenSSL = \"/usr\"\n").contains("names are"));
        assert!(load_err("[variables]\nsysroot = \"/usr\"\n").contains("can't be redefined"));
        assert!(load_err("[variables]\nx = \"usr\"\n").contains("must be an absolute path"));
        let nested = "[variables]\nx = { env = \"A\", default = { env = \"DEP_X\" } }\n";
        assert!(load_err(nested).contains("chosen by a crate"));
        let unknown = "[variables]\nx = { env = \"A\", list = true }\n";
        assert!(load_err(unknown).contains("failed to parse"));
        for env in ["DEP_OPENSSL_INCLUDE", "CARGO_PKG_HOMEPAGE"] {
            let text = format!("[variables]\nx = {{ env = \"{env}\" }}\n");
            assert!(load_err(&text).contains("chosen by a crate"));
        }
    }

    #[test]
    fn rejects_unknown_keys_in_list_tables() {
        assert!(load_err("[external]\nread = { inherit = true, valu = [\"/x\"] }\n").contains("failed to parse"));
    }
}
