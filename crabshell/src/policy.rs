//! Resolving the effective permission set the runner enforces.
//!
//! Resolution is per field: the most specific table that sets a field wins, a
//! table with `inherit = true` appends to the next table up, and `[limits]`
//! clamps the result. Pure and filesystem-free, so it is covered by unit tests
//! rather than end-to-end builds.
//!
//! Clamping and tightening intersect lists entry by entry, where an entry
//! ending in `*` covers every entry with that prefix. Paths are compared as
//! strings, so `/usr` doesn't yet cover `/usr/include`.

use crate::config::{Fields, ListField, SandboxConfig};
use crate::declared::Declared;

/// Environment variables the structural floor always grants.
///
/// Cargo requires a build script to place its output in `OUT_DIR`, so a script
/// that cannot read it cannot run at all. Floor entries bypass `[limits]`.
pub const FLOOR_ENV: &[&str] = &["OUT_DIR"];

/// Paths the floor always grants read access to: the crate root.
///
/// Write access to `OUT_DIR` and a private `TMPDIR` are also structural; the
/// runner grants them alongside execute access to the build script itself.
/// Floor entries bypass `[limits]`.
pub const FLOOR_READ: &[&str] = &["$CARGO_MANIFEST_DIR"];

/// Paths the structural floor always grants write access to.
///
/// Compile probes silence their output with `Stdio::null()`, which opens
/// `/dev/null` read-write. It holds no data, so granting it leaks nothing.
pub const FLOOR_WRITE: &[&str] = &["/dev/null"];

/// The dynamic loader and the libraries every std build script links on `host`.
///
/// A build script is a host program built with std. On `x86_64-*-linux-gnu`
/// its ELF interpreter is the dynamic loader, and it needs `libc.so.6` and
/// `libgcc_s.so.1` (for unwinding). Each library has candidate locations
/// (Fedora-style `/lib64`, Debian-style `/lib/x86_64-linux-gnu`); candidates
/// that don't exist are skipped. Hosts not listed get nothing, which fails
/// closed.
///
/// These stand in for built-in variables with per-host defaults that config
/// will be able to override; see `docs/src/policy/paths.md`.
pub fn floor_runtime(host: &str) -> Vec<&'static str> {
    let candidates: &[&[&str]] = match host {
        "x86_64-unknown-linux-gnu" => &[
            &["/lib64/ld-linux-x86-64.so.2"],
            &["/lib64/libc.so.6", "/lib/x86_64-linux-gnu/libc.so.6"],
            &["/lib64/libgcc_s.so.1", "/lib/x86_64-linux-gnu/libgcc_s.so.1"],
        ],
        _ => &[],
    };
    candidates.iter().flat_map(|lib| lib.iter().copied()).collect()
}

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
    /// The host triple the build script runs on (Cargo's `HOST`), which
    /// selects the floor's runtime libraries.
    pub host: String,
}

/// The fully resolved permissions a build script runs with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Permissions {
    /// Read-only paths.
    pub read: Vec<String>,
    /// Writable paths, beyond `OUT_DIR`.
    pub write: Vec<String>,
    /// Executables the script may run.
    pub run: Vec<String>,
    /// Environment variables the script may read.
    pub env: Vec<String>,
    /// Whether network access is granted.
    pub network: bool,
    /// Directories whose entries may be listed, but not read.
    ///
    /// Not settable from `sandbox.toml`; lets an outer sandbox allow the
    /// directory listing an inner crabshell needs to expand globs like `/lib*`.
    pub list: Vec<String>,
}

/// Resolve the effective permissions for `pkg` under `config`.
pub fn resolve(config: &SandboxConfig, pkg: &PackageContext) -> Permissions {
    let chain = chain(config, pkg);
    let mut env = resolve_list(&chain, &config.limits, |f| &f.env);
    env.extend(FLOOR_ENV.iter().map(|name| name.to_string()));
    let mut read = resolve_list(&chain, &config.limits, |f| &f.read);
    read.extend(floor_read(&pkg.host).map(str::to_string));
    let mut write = resolve_list(&chain, &config.limits, |f| &f.write);
    write.extend(FLOOR_WRITE.iter().map(|path| path.to_string()));

    Permissions {
        read: dedup(read),
        write: dedup(write),
        run: resolve_list(&chain, &config.limits, |f| &f.run),
        env: dedup(env),
        network: resolve_network(&chain, &config.limits),
        list: Vec::new(),
    }
}

/// Narrow `permissions` to what the package `declared` it needs.
///
/// Each list becomes its intersection with the declaration, plus the
/// structural floor. Network survives only if both grant it.
pub fn tighten(permissions: Permissions, declared: &Declared, pkg: &PackageContext) -> Permissions {
    let keep = |granted: Vec<String>, wanted: &[String], floor: &[&str]| -> Vec<String> {
        let mut out = intersect(&granted, wanted);
        out.extend(granted.into_iter().filter(|entry| floor.contains(&entry.as_str())));
        dedup(out)
    };
    Permissions {
        read: keep(permissions.read, &declared.read, &floor_read(&pkg.host).collect::<Vec<_>>()),
        write: keep(permissions.write, &declared.write, FLOOR_WRITE),
        run: keep(permissions.run, &declared.run, &[]),
        env: keep(permissions.env, &declared.env, FLOOR_ENV),
        network: permissions.network && declared.network,
        list: permissions.list,
    }
}

/// Every path the floor grants read access to on `host`.
fn floor_read(host: &str) -> impl Iterator<Item = &'static str> {
    FLOOR_READ.iter().copied().chain(floor_runtime(host))
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
    let value = lookup_list(chain, get);
    match get(limits) {
        Some(cap) => intersect(&value, entries(cap)),
        None => value,
    }
}

/// The entries of either list that the other list covers.
fn intersect(a: &[String], b: &[String]) -> Vec<String> {
    let covered = |entry: &&String, by: &[String]| by.iter().any(|pattern| covers(pattern, entry));
    let mut out: Vec<String> = a.iter().filter(|e| covered(e, b)).cloned().collect();
    out.extend(b.iter().filter(|e| covered(e, a)).cloned());
    dedup(out)
}

/// Whether `pattern` covers `entry`: equal, or a matching `prefix*`.
fn covers(pattern: &str, entry: &str) -> bool {
    match pattern.strip_suffix('*') {
        Some(prefix) => entry.starts_with(prefix),
        None => pattern == entry,
    }
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

    const HOST: &str = "x86_64-unknown-linux-gnu";

    fn external(name: &str) -> PackageContext {
        PackageContext { name: name.to_string(), is_local: false, host: HOST.to_string() }
    }

    fn local(name: &str) -> PackageContext {
        PackageContext { name: name.to_string(), is_local: true, host: HOST.to_string() }
    }

    /// Every read path the floor grants.
    fn floor() -> Vec<String> {
        floor_read(HOST).map(str::to_string).collect()
    }

    /// The resolved env minus the structural floor.
    fn granted_env(config: &SandboxConfig, pkg: &PackageContext) -> Vec<String> {
        let mut env = resolve(config, pkg).env;
        env.retain(|name| !FLOOR_ENV.contains(&name.as_str()));
        env
    }

    #[test]
    fn floor_grants_only_out_dir_the_crate_root_and_dev_null() {
        let resolved = resolve(&SandboxConfig::default(), &external("any"));
        assert_eq!(resolved.env, ["OUT_DIR"]);
        assert_eq!(resolved.read, floor());
        assert_eq!(resolved.write, ["/dev/null"]);
        assert!(resolved.run.is_empty());
        assert!(!resolved.network);
    }

    #[test]
    fn floor_survives_limits() {
        let config = config("[limits]\nenv = []\nread = []\nwrite = []\n");
        assert_eq!(resolve(&config, &external("dep")).env, ["OUT_DIR"]);
        assert_eq!(resolve(&config, &external("dep")).read, floor());
        assert_eq!(resolve(&config, &external("dep")).write, ["/dev/null"]);
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
    fn a_declaration_only_tightens() {
        let config = config(
            r#"
            [external]
            read = ["/usr/include", "/usr/lib"]
            run = ["cc"]
            env = ["TARGET", "CC"]
            network = true
            "#,
        );
        let declared = Declared {
            read: vec!["/usr/include".into(), "/etc".into()],
            write: vec![],
            run: vec![],
            env: vec!["TARGET".into(), "HOME".into()],
            network: false,
        };

        let tightened = tighten(resolve(&config, &external("dep")), &declared, &external("dep"));
        let mut expected = vec!["/usr/include".to_string()];
        expected.extend(floor());
        assert_eq!(tightened.read, expected);
        assert!(tightened.run.is_empty());
        assert_eq!(tightened.env, ["TARGET", "OUT_DIR"]);
        assert!(!tightened.network);
    }

    #[test]
    fn patterns_intersect_by_prefix() {
        let capped = config(
            r#"
            [limits]
            env = ["CARGO_*"]

            [external]
            env = ["CARGO_CFG_*", "CARGO_PKG_NAME", "HOME"]
            "#,
        );
        assert_eq!(granted_env(&capped, &external("dep")), ["CARGO_CFG_*", "CARGO_PKG_NAME"]);

        let broad = config("[external]\nenv = [\"*\"]\n");
        let declared = Declared { env: vec!["TARGET".into()], ..Declared::default() };
        assert_eq!(
            tighten(resolve(&broad, &external("dep")), &declared, &external("dep")).env,
            ["TARGET", "OUT_DIR"]
        );
    }

    #[test]
    fn a_declaration_keeps_the_floor() {
        let tightened = tighten(
            resolve(&SandboxConfig::default(), &external("dep")),
            &Declared::default(),
            &external("dep"),
        );
        assert_eq!(tightened, resolve(&SandboxConfig::default(), &external("dep")));
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
