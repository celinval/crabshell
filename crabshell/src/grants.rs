//! Named grants: reusable permission sets that policies grant by name.
//!
//! crabshell ships built-in grants (`grants.toml`); a `[grant.<name>]` table in
//! `sandbox.toml` overrides a built-in per field, or defines a new grant.
//! Consumers assign grants with `grants = [...]`, and crates request them with
//! `requests = [...]` in `[package.metadata.build-rs]`; both expand to plain
//! fields before clamping and narrowing.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::config::{Fields, ListField, SandboxConfig};

/// The grants crabshell ships with.
const BUILTIN: &str = include_str!("grants.toml");

/// Named grants by name, with overrides applied.
pub type Grants = BTreeMap<String, Fields>;

/// The built-in grants.
pub fn builtin() -> Grants {
    #[derive(Deserialize)]
    struct File {
        grant: Grants,
    }
    let file: File = toml::from_str(BUILTIN).expect("built-in grants.toml is valid");
    file.grant
}

/// The built-in grants with `config`'s `[grant.<name>]` tables applied.
pub fn effective(config: &SandboxConfig) -> Grants {
    let mut grants = builtin();
    for (name, overrides) in &config.grant {
        let base = grants.remove(name).unwrap_or_default();
        grants.insert(name.clone(), merge(base, overrides));
    }
    grants
}

/// Whether `name` is a valid grant name: `[a-z][a-z0-9_-]*`.
pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// The entries of field `get` across the grants named in `names`.
///
/// Unknown names contribute nothing; config validation rejects them in
/// consumer policy, while a crate's request for a grant crabshell doesn't know
/// only narrows.
pub fn expand(
    grants: &Grants,
    names: &[String],
    get: impl Fn(&Fields) -> &Option<ListField>,
) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| grants.get(name))
        .filter_map(|grant| get(grant).as_ref())
        .flat_map(|field| entries(field).iter().cloned())
        .collect()
}

/// Whether any grant named in `names` grants network access.
pub fn expand_network(grants: &Grants, names: &[String]) -> bool {
    names.iter().filter_map(|name| grants.get(name)).any(|grant| grant.network == Some(true))
}

/// Apply `overrides` to `base` field by field.
fn merge(base: Fields, overrides: &Fields) -> Fields {
    let list = |base: Option<ListField>, over: &Option<ListField>| match over {
        None => base,
        Some(ListField::Inherit { inherit: true, value }) => {
            let mut merged: Vec<String> = base.as_ref().map(entries).unwrap_or_default().to_vec();
            merged.extend(value.iter().cloned());
            Some(ListField::Override(merged))
        }
        Some(field) => Some(ListField::Override(entries(field).to_vec())),
    };
    Fields {
        read: list(base.read, &overrides.read),
        write: list(base.write, &overrides.write),
        run: list(base.run, &overrides.run),
        env: list(base.env, &overrides.env),
        network: overrides.network.or(base.network),
        grants: None,
    }
}

fn entries(field: &ListField) -> &[String] {
    match field {
        ListField::Override(value) => value,
        ListField::Inherit { value, .. } => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(toml: &str) -> SandboxConfig {
        toml::from_str(toml).unwrap()
    }

    #[test]
    fn builtins_parse_and_have_valid_names() {
        let grants = builtin();
        assert!(["cc", "pkg-config", "rustc"].iter().all(|name| grants.contains_key(*name)));
        assert!(grants.keys().all(|name| is_valid_name(name)));
        assert!(grants.values().all(|grant| grant.grants.is_none()));
    }

    #[test]
    fn an_array_replaces_a_builtin_field() {
        let grants = effective(&config("[grant.pkg-config]\nrun = [\"pkgconf\"]\n"));
        let run = expand(&grants, &["pkg-config".into()], |f| &f.run);
        assert_eq!(run, ["pkgconf"]);
        // Fields the override doesn't name keep the built-in value.
        assert!(!expand(&grants, &["pkg-config".into()], |f| &f.env).is_empty());
    }

    #[test]
    fn inherit_extends_a_builtin_field() {
        let toml = "[grant.pkg-config]\nrun = { inherit = true, value = [\"rpm\"] }\n";
        let run = expand(&effective(&config(toml)), &["pkg-config".into()], |f| &f.run);
        assert_eq!(run, ["pkg-config", "pkgconf", "rpm"]);
    }

    #[test]
    fn config_can_define_new_grants() {
        let grants = effective(&config("[grant.protoc]\nrun = [\"protoc\"]\nnetwork = false\n"));
        assert_eq!(expand(&grants, &["protoc".into()], |f| &f.run), ["protoc"]);
        assert!(!expand_network(&grants, &["protoc".into()]));
    }

    #[test]
    fn unknown_names_expand_to_nothing() {
        assert!(expand(&builtin(), &["nope".into()], |f| &f.read).is_empty());
    }

    #[test]
    fn validates_names() {
        assert!(is_valid_name("pkg-config"));
        assert!(is_valid_name("cc2_x"));
        assert!(!is_valid_name("Pkg"));
        assert!(!is_valid_name("-cc"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("a.b"));
    }
}
