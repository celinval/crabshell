//! Resolving the effective permission set the runner enforces.
//!
//! Resolution is per field: the most specific table that sets a field wins, a
//! table with `inherit = true` appends to the next table up, and `[limits]`
//! clamps the result. Pure and filesystem-free, so it is covered by unit tests
//! rather than end-to-end builds.
//!
//! Only `env` is enforced by the runner so far. Clamping compares list entries
//! exactly, which is right for `env` names but not for `read` paths, where
//! prefix containment is needed.

use crate::config::{Fields, ListField, SandboxConfig};

/// Environment variables the structural floor always grants.
///
/// Cargo requires a build script to place its output in `OUT_DIR`, so a script
/// that cannot read it cannot run at all. Floor entries bypass `[limits]`.
pub const FLOOR_ENV: &[&str] = &["OUT_DIR"];

/// The package whose build script is being run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageContext {
    /// Package name, matched against `[package.<name>]`.
    pub name: String,
    /// Whether the package's source is on the filesystem.
    ///
    /// Selects `[local]` for workspace members and path dependencies, and
    /// `[external]` for registry and git dependencies.
    pub is_local: bool,
}

/// The fully resolved permissions a build script runs with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Permissions {
    /// Read-only paths.
    pub read: Vec<String>,
    /// Executables the script may run.
    pub run: Vec<String>,
    /// Environment variables the script may read.
    pub env: Vec<String>,
    /// Whether network access is granted.
    pub network: bool,
}

/// Resolve the effective permissions for `pkg` under `config`.
pub fn resolve(config: &SandboxConfig, pkg: &PackageContext) -> Permissions {
    let chain = chain(config, pkg);
    let mut env = resolve_list(&chain, &config.limits, |f| &f.env);
    env.extend(FLOOR_ENV.iter().map(|name| name.to_string()));

    Permissions {
        read: resolve_list(&chain, &config.limits, |f| &f.read),
        run: resolve_list(&chain, &config.limits, |f| &f.run),
        env: dedup(env),
        network: resolve_network(&chain, &config.limits),
    }
}

/// The tables applying to `pkg`, most specific first.
fn chain<'a>(config: &'a SandboxConfig, pkg: &PackageContext) -> Vec<&'a Fields> {
    let category = if pkg.is_local { &config.local } else { &config.external };
    config.package.get(&pkg.name).into_iter().chain([category]).collect()
}

/// Resolve one list field down the chain, then clamp it by `limits`.
fn resolve_list(
    chain: &[&Fields],
    limits: &Fields,
    get: impl Fn(&Fields) -> &Option<ListField> + Copy,
) -> Vec<String> {
    let mut value = lookup_list(chain, get);
    if let Some(cap) = get(limits) {
        let cap = entries(cap);
        value.retain(|entry| cap.contains(entry));
    }
    value
}

/// Take the first table that sets the field, expanding `inherit` upwards.
fn lookup_list(
    chain: &[&Fields],
    get: impl Fn(&Fields) -> &Option<ListField> + Copy,
) -> Vec<String> {
    let Some((head, rest)) = chain.split_first() else {
        // The structural floor sets no lists.
        return Vec::new();
    };
    match get(head) {
        None => lookup_list(rest, get),
        Some(ListField::Override(value)) => dedup(value.clone()),
        Some(ListField::Inherit { inherit, value }) => {
            let mut out = if *inherit { lookup_list(rest, get) } else { Vec::new() };
            out.extend(value.iter().cloned());
            dedup(out)
        }
    }
}

/// Network is denied by the floor, granted by the chain, and clamped by `limits`.
fn resolve_network(chain: &[&Fields], limits: &Fields) -> bool {
    let granted = chain.iter().find_map(|fields| fields.network).unwrap_or(false);
    granted && limits.network.unwrap_or(true)
}

/// The entries a list field carries, ignoring whether it inherits.
fn entries(field: &ListField) -> &[String] {
    match field {
        ListField::Override(value) => value,
        ListField::Inherit { value, .. } => value,
    }
}

fn dedup(mut value: Vec<String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    value.retain(|entry| seen.insert(entry.clone()));
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(toml: &str) -> SandboxConfig {
        toml::from_str(toml).unwrap()
    }

    fn external(name: &str) -> PackageContext {
        PackageContext { name: name.to_string(), is_local: false }
    }

    fn local(name: &str) -> PackageContext {
        PackageContext { name: name.to_string(), is_local: true }
    }

    /// The resolved env minus the structural floor.
    fn granted_env(config: &SandboxConfig, pkg: &PackageContext) -> Vec<String> {
        let mut env = resolve(config, pkg).env;
        env.retain(|name| !FLOOR_ENV.contains(&name.as_str()));
        env
    }

    #[test]
    fn floor_grants_only_out_dir() {
        let resolved = resolve(&SandboxConfig::default(), &external("any"));
        assert_eq!(resolved.env, ["OUT_DIR"]);
        assert!(resolved.read.is_empty());
        assert!(resolved.run.is_empty());
        assert!(!resolved.network);
    }

    #[test]
    fn floor_survives_limits() {
        let config = config("[limits]\nenv = []\n");
        assert_eq!(resolve(&config, &external("dep")).env, ["OUT_DIR"]);
    }

    #[test]
    fn category_is_selected_by_locality() {
        let config = config(
            r#"
            [local]
            env = ["LOCAL"]

            [external]
            env = ["EXTERNAL"]
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["EXTERNAL"]);
        assert_eq!(granted_env(&config, &local("mine")), ["LOCAL"]);
    }

    #[test]
    fn named_entry_overrides_category() {
        let config = config(
            r#"
            [external]
            env = ["BASE"]

            [package.dep]
            env = ["OWN"]
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["OWN"]);
        assert_eq!(granted_env(&config, &external("other")), ["BASE"]);
    }

    #[test]
    fn unset_field_falls_through_to_category() {
        let config = config(
            r#"
            [external]
            env = ["BASE"]
            run = ["cc"]

            [package.dep]
            env = ["OWN"]
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["OWN"]);
        assert_eq!(resolve(&config, &external("dep")).run, ["cc"]);
    }

    #[test]
    fn inherit_appends_to_category() {
        let config = config(
            r#"
            [external]
            env = ["BASE"]

            [package.dep]
            env = { inherit = true, value = ["EXTRA"] }
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["BASE", "EXTRA"]);
    }

    #[test]
    fn inherit_false_matches_the_array_form() {
        let config = config(
            r#"
            [external]
            env = ["BASE"]

            [package.dep]
            env = { inherit = false, value = ["ONLY"] }
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["ONLY"]);
    }

    #[test]
    fn inherit_dedups_overlapping_entries() {
        let config = config(
            r#"
            [external]
            env = ["SHARED"]

            [package.dep]
            env = { inherit = true, value = ["SHARED", "EXTRA"] }
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["SHARED", "EXTRA"]);
    }

    #[test]
    fn limits_clamp_a_resolved_list() {
        let config = config(
            r#"
            [limits]
            env = ["ALLOWED"]

            [external]
            env = ["ALLOWED", "DENIED"]
            "#,
        );

        assert_eq!(granted_env(&config, &external("dep")), ["ALLOWED"]);
    }

    #[test]
    fn limits_leave_unnamed_fields_uncapped() {
        let config = config(
            r#"
            [limits]
            env = []

            [external]
            env = ["DENIED"]
            run = ["cc"]
            "#,
        );

        assert!(granted_env(&config, &external("dep")).is_empty());
        assert_eq!(resolve(&config, &external("dep")).run, ["cc"]);
    }

    #[test]
    fn network_is_denied_unless_granted() {
        let granted = config("[external]\nnetwork = true\n");
        assert!(resolve(&granted, &external("dep")).network);
        assert!(!resolve(&SandboxConfig::default(), &external("dep")).network);
    }

    #[test]
    fn limits_clamp_network() {
        let config = config(
            r#"
            [limits]
            network = false

            [external]
            network = true
            "#,
        );

        assert!(!resolve(&config, &external("dep")).network);
    }
}
