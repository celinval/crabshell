//! Resolving the effective permission set the runner enforces.
//!
//! Resolution is per field: the most specific table that sets a field wins, a
//! table with `inherit = true` appends to the next table up, named grants add
//! their entries, and `[limits]` clamps the result.
//!
//! Every entry expands to concrete values first, through an [`Expand`]:
//! `:variables` and glob patterns to paths, env patterns to the names actually
//! set. Clamping and narrowing then intersect concrete values only: a path
//! covers the paths under it, so a request can narrow a granted directory to a
//! few files, and env names compare by equality. The expander is the only
//! part that touches the filesystem or environment, so this module is covered
//! by unit tests.

use crate::config::{Fields, ListField, SandboxConfig};
use crate::declared::Declared;
use crate::grants;
use crate::paths::PathError;

/// Turns policy entries into the concrete values they stand for.
pub trait Expand {
    /// The paths a `read`, `write` or `run` entry names. A bare program name
    /// (`cc`) stays as it is.
    fn paths(&self, entry: &str) -> Result<Vec<String>, PathError>;
    /// The env var names an `env` entry names.
    fn names(&self, entry: &str) -> Vec<String>;
}

/// Which kind of values a list field holds.
#[derive(Clone, Copy)]
enum Kind {
    Paths,
    Names,
}

fn expand_all(x: &impl Expand, kind: Kind, entries: &[String]) -> Result<Vec<String>, PathError> {
    let mut out = Vec::new();
    for entry in entries {
        match kind {
            Kind::Paths => out.extend(x.paths(entry)?),
            Kind::Names => out.extend(x.names(entry)),
        }
    }
    Ok(dedup(out))
}

/// Environment variables every build script has implicit access to.
///
/// Cargo requires a build script to place its output in `OUT_DIR`, so a script
/// that cannot read it cannot run at all. Implicit access is not policy:
/// neither `[limits]` nor requests remove it.
pub const IMPLICIT_ENV: &[&str] = &["OUT_DIR"];

/// Paths every build script has implicit read and write access to.
///
/// Compile probes silence rustc with `Stdio::null()`, which opens `/dev/null`
/// for reading as stdin and for writing as stdout. It holds no data, so
/// granting it leaks nothing. The runner adds write-only access to `OUT_DIR`
/// and a private `TMPDIR`, and execute access to the build script itself.
pub const IMPLICIT_READ_WRITE: &[&str] = &["/dev/null"];

/// The default grant for `read`: the crate root.
///
/// It's what `read` resolves to when no table sets it, so tables can replace
/// it (an array) or extend it (`inherit = true`), `[limits]` clamps it, and a
/// request narrows it, e.g. to `:manifest_dir/build/probe.rs`.
pub const DEFAULT_READ: &[&str] = &[":manifest_dir"];

/// The default grant for `env`: the target and host triples, and what Cargo
/// derives from the target and the enabled features.
///
/// None of these hold user secrets or machine-specific paths, which would
/// also leak into the build script's cache key. Like any grant, tables
/// replace or extend it, and a request narrows it: a crate still requests the
/// names it reads.
pub const DEFAULT_ENV: &[&str] = &["TARGET", "HOST", "CARGO_CFG_*", "CARGO_FEATURE_*"];

/// The dynamic loader and the libraries every std build script links on `host`.
///
/// A build script is a host program built with std. On `x86_64-*-linux-gnu`
/// its ELF interpreter is the dynamic loader, and it needs `libc.so.6` and
/// `libgcc_s.so.1` (for unwinding). Each library has candidate locations
/// (Fedora-style `/lib64`, Debian-style `/lib/x86_64-linux-gnu`); candidates
/// that don't exist are skipped. Hosts not listed get nothing, which fails
/// closed.
///
/// They're implicit access. These stand in for built-in variables with
/// per-host defaults that config will be able to override; see
/// `docs/src/policy/paths.md`.
pub fn implicit_runtime(host: &str) -> Vec<&'static str> {
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

/// Paths every build script on `host` can read implicitly.
///
/// The [`implicit_runtime`] libraries plus [`IMPLICIT_READ_WRITE`].
pub fn implicit_read(host: &str) -> Vec<&'static str> {
    let mut paths = implicit_runtime(host);
    paths.extend(IMPLICIT_READ_WRITE);
    paths
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
    /// Write-only paths, beyond `OUT_DIR` and `TMPDIR`.
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
pub fn resolve(
    config: &SandboxConfig,
    pkg: &PackageContext,
    x: &impl Expand,
) -> Result<Permissions, PathError> {
    let chain = chain(config, pkg);
    let grants = grants::effective(config);
    let names = lookup_list(&chain, |f| &f.grants, &[]);
    let field = |get: fn(&Fields) -> &Option<ListField>, kind, default: &[&str]| {
        let mut value = lookup_list(&chain, get, default);
        value.extend(grants::expand(&grants, &names, get));
        let value = expand_all(x, kind, &value)?;
        Ok(match get(&config.limits) {
            Some(cap) => intersect(&value, &expand_all(x, kind, entries(cap))?),
            None => value,
        })
    };
    let mut env = field(|f| &f.env, Kind::Names, DEFAULT_ENV)?;
    env.extend(IMPLICIT_ENV.iter().map(|name| name.to_string()));
    let mut read = field(|f| &f.read, Kind::Paths, DEFAULT_READ)?;
    read.extend(implicit_read(&pkg.host).into_iter().map(str::to_string));
    let mut write = field(|f| &f.write, Kind::Paths, &[])?;
    write.extend(IMPLICIT_READ_WRITE.iter().map(|path| path.to_string()));
    let network = chain.iter().find_map(|fields| fields.network);
    let network = network.unwrap_or_else(|| grants::expand_network(&grants, &names));

    Ok(Permissions {
        read: dedup(read),
        write: dedup(write),
        run: field(|f| &f.run, Kind::Paths, &[])?,
        env: dedup(env),
        network: network && config.limits.network.unwrap_or(true),
        list: Vec::new(),
    })
}

/// Narrow `permissions` to what the package `declared` it needs.
///
/// The declaration's `requests` expand through `config`'s grants first. Each
/// list then becomes its intersection with the declaration, plus the
/// implicit access. Network survives only if both grant it.
pub fn tighten(
    permissions: Permissions,
    declared: &Declared,
    pkg: &PackageContext,
    config: &SandboxConfig,
    x: &impl Expand,
) -> Result<Permissions, PathError> {
    let grants = grants::effective(config);
    let wanted = |own: &[String], get: fn(&Fields) -> &Option<ListField>, kind| {
        let mut value = own.to_vec();
        value.extend(grants::expand(&grants, &declared.requests, get));
        expand_all(x, kind, &value)
    };
    let keep = |granted: Vec<String>, wanted: Vec<String>, floor: &[&str]| -> Vec<String> {
        let mut out = intersect(&granted, &wanted);
        out.extend(granted.into_iter().filter(|entry| floor.contains(&entry.as_str())));
        dedup(out)
    };
    let network = declared.network || grants::expand_network(&grants, &declared.requests);
    Ok(Permissions {
        read: keep(
            permissions.read,
            wanted(&declared.read, |f| &f.read, Kind::Paths)?,
            &implicit_read(&pkg.host),
        ),
        write: keep(
            permissions.write,
            wanted(&declared.write, |f| &f.write, Kind::Paths)?,
            IMPLICIT_READ_WRITE,
        ),
        run: keep(permissions.run, wanted(&declared.run, |f| &f.run, Kind::Paths)?, &[]),
        env: keep(permissions.env, wanted(&declared.env, |f| &f.env, Kind::Names)?, IMPLICIT_ENV),
        network: permissions.network && network,
        list: permissions.list,
    })
}

/// The tables applying to `pkg`, most specific first.
fn chain<'a>(config: &'a SandboxConfig, pkg: &PackageContext) -> Vec<&'a Fields> {
    let category = if pkg.is_local { &config.local } else { &config.external };
    config.package.get(&pkg.name).into_iter().chain([category]).collect()
}

/// The entries of either concrete list that the other list covers.
fn intersect(a: &[String], b: &[String]) -> Vec<String> {
    let covered = |entry: &&String, by: &[String]| by.iter().any(|outer| covers(outer, entry));
    let mut out: Vec<String> = a.iter().filter(|e| covered(e, b)).cloned().collect();
    out.extend(b.iter().filter(|e| covered(e, a)).cloned());
    dedup(out)
}

/// Whether concrete `outer` covers `entry`: equal, or a path under `outer` at
/// a `/` boundary. Env names contain no `/`, so for them it's plain equality.
fn covers(outer: &str, entry: &str) -> bool {
    match entry.strip_prefix(outer) {
        Some(rest) => rest.is_empty() || rest.starts_with('/') || outer.ends_with('/'),
        None => false,
    }
}

/// Take the first table that sets the field, expanding `inherit` upwards.
///
/// Below the tables, the field resolves to its `default` grant.
fn lookup_list(
    chain: &[&Fields],
    get: impl Fn(&Fields) -> &Option<ListField> + Copy,
    default: &[&str],
) -> Vec<String> {
    let Some((head, rest)) = chain.split_first() else {
        return default.iter().map(|entry| entry.to_string()).collect();
    };
    match get(head) {
        None => lookup_list(rest, get, default),
        Some(ListField::Override(value)) => dedup(value.clone()),
        Some(ListField::Inherit { inherit, value }) => {
            let mut out = if *inherit { lookup_list(rest, get, default) } else { Vec::new() };
            out.extend(value.iter().cloned());
            dedup(out)
        }
    }
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

    /// Keeps paths as written, and matches env patterns against [`NAMES`].
    struct Fixed;

    /// The env vars set in these tests.
    const NAMES: &[&str] = &[
        "TARGET",
        "HOST",
        "CARGO_CFG_TARGET_OS",
        "CARGO_CFG_SANITIZE",
        "CARGO_FEATURE_STD",
        "CARGO_PKG_NAME",
        "PKG_CONFIG_PATH",
        "CC",
        "ZLIB_STATIC",
        "HOME",
    ];

    impl Expand for Fixed {
        fn paths(&self, entry: &str) -> Result<Vec<String>, PathError> {
            Ok(vec![entry.to_string()])
        }

        fn names(&self, entry: &str) -> Vec<String> {
            match glob::Pattern::new(entry) {
                Ok(pattern) if entry.contains('*') => {
                    NAMES.iter().filter(|n| pattern.matches(n)).map(|n| n.to_string()).collect()
                }
                _ => vec![entry.to_string()],
            }
        }
    }

    fn resolve(config: &SandboxConfig, pkg: &PackageContext) -> Permissions {
        super::resolve(config, pkg, &Fixed).unwrap()
    }

    fn tighten(
        permissions: Permissions,
        declared: &Declared,
        pkg: &PackageContext,
        config: &SandboxConfig,
    ) -> Permissions {
        super::tighten(permissions, declared, pkg, config, &Fixed).unwrap()
    }

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

    /// Every read path implicit access grants.
    fn implicit() -> Vec<String> {
        implicit_read(HOST).into_iter().map(str::to_string).collect()
    }

    /// The default read grant followed by implicit access.
    fn default_read() -> Vec<String> {
        DEFAULT_READ.iter().map(|path| path.to_string()).chain(implicit()).collect()
    }

    /// The resolved env minus implicit access.
    fn granted_env(config: &SandboxConfig, pkg: &PackageContext) -> Vec<String> {
        let mut env = resolve(config, pkg).env;
        env.retain(|name| !IMPLICIT_ENV.contains(&name.as_str()));
        env
    }

    #[test]
    fn no_policy_grants_the_defaults_and_implicit_access() {
        let resolved = resolve(&SandboxConfig::default(), &external("any"));
        // The default's patterns expand to the names actually set.
        let expected = ["TARGET", "HOST", "CARGO_CFG_TARGET_OS", "CARGO_CFG_SANITIZE"];
        let expected: Vec<&str> =
            expected.into_iter().chain(["CARGO_FEATURE_STD", "OUT_DIR"]).collect();
        assert_eq!(resolved.env, expected);
        assert_eq!(resolved.read, default_read());
        assert_eq!(resolved.write, ["/dev/null"]);
        assert!(resolved.run.is_empty());
        assert!(!resolved.network);
    }

    #[test]
    fn implicit_access_survives_limits_but_the_default_grant_doesnt() {
        let config = config("[limits]\nenv = []\nread = []\nwrite = []\n");
        assert_eq!(resolve(&config, &external("dep")).env, ["OUT_DIR"]);
        assert_eq!(resolve(&config, &external("dep")).read, implicit());
        assert_eq!(resolve(&config, &external("dep")).write, ["/dev/null"]);
    }

    #[test]
    fn requests_narrow_the_default_env_grant() {
        let declared = Declared { env: vec!["CARGO_CFG_TARGET_OS".into()], ..Declared::default() };
        let none = SandboxConfig::default();
        let tightened =
            tighten(resolve(&none, &external("dep")), &declared, &external("dep"), &none);
        assert_eq!(tightened.env, ["CARGO_CFG_TARGET_OS", "OUT_DIR"]);
    }

    #[test]
    fn tables_replace_or_extend_the_default_grant() {
        let replaced = config("[external]\nread = [\"/usr/include\"]\n");
        let mut expected = vec!["/usr/include".to_string()];
        expected.extend(implicit());
        assert_eq!(resolve(&replaced, &external("dep")).read, expected);

        let extended =
            config("[external]\nread = { inherit = true, value = [\"/usr/include\"] }\n");
        let mut expected = vec![DEFAULT_READ[0].to_string(), "/usr/include".to_string()];
        expected.extend(implicit());
        assert_eq!(resolve(&extended, &external("dep")).read, expected);
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
            requests: vec![],
        };

        let tightened =
            tighten(resolve(&config, &external("dep")), &declared, &external("dep"), &config);
        let mut expected = vec!["/usr/include".to_string()];
        expected.extend(implicit());
        assert_eq!(tightened.read, expected);
        assert!(tightened.run.is_empty());
        assert_eq!(tightened.env, ["TARGET", "OUT_DIR"]);
        assert!(!tightened.network);
    }

    #[test]
    fn patterns_expand_before_they_intersect() {
        let capped = config(
            r#"
            [limits]
            env = ["CARGO_*"]

            [external]
            env = ["CARGO_CFG_*", "CARGO_PKG_NAME", "HOME"]
            "#,
        );
        assert_eq!(
            granted_env(&capped, &external("dep")),
            ["CARGO_CFG_TARGET_OS", "CARGO_CFG_SANITIZE", "CARGO_PKG_NAME"]
        );

        let broad = config("[external]\nenv = [\"*\"]\n");
        let declared = Declared { env: vec!["TARGET".into()], ..Declared::default() };
        assert_eq!(
            tighten(resolve(&broad, &external("dep")), &declared, &external("dep"), &broad).env,
            ["TARGET", "OUT_DIR"]
        );
    }

    #[test]
    fn a_declaration_narrows_the_default_grant_but_not_implicit_access() {
        let tightened = tighten(
            resolve(&SandboxConfig::default(), &external("dep")),
            &Declared::default(),
            &external("dep"),
            &SandboxConfig::default(),
        );
        assert_eq!(tightened.read, implicit());
        assert_eq!(tightened.write, ["/dev/null"]);
        assert_eq!(tightened.env, ["OUT_DIR"]);
    }

    #[test]
    fn grants_expand_into_fields() {
        let config = config(
            r#"
            [external]
            grants = ["pkg-config"]
            env = ["TARGET"]

            [grant.pkg-config]
            run = ["pkgconf"]
            env = ["PKG_CONFIG_PATH"]
            "#,
        );
        let resolved = resolve(&config, &external("dep"));
        assert_eq!(resolved.run, ["pkgconf"]);
        assert_eq!(granted_env(&config, &external("dep")), ["TARGET", "PKG_CONFIG_PATH"]);
    }

    #[test]
    fn limits_clamp_granted_entries() {
        let config = config(
            r#"
            [limits]
            run = []

            [external]
            grants = ["cc"]
            "#,
        );
        assert!(resolve(&config, &external("dep")).run.is_empty());
    }

    #[test]
    fn requests_narrow_like_their_fields() {
        let config = config(
            r#"
            [external]
            grants = ["cc", "pkg-config"]
            env = ["*"]
            "#,
        );
        let declared = Declared {
            env: vec!["ZLIB_STATIC".into()],
            requests: vec!["pkg-config".into()],
            ..Declared::default()
        };
        let tightened =
            tighten(resolve(&config, &external("dep")), &declared, &external("dep"), &config);
        // Only pkg-config's tools survive; cc's are dropped.
        assert_eq!(tightened.run, ["pkg-config", "pkgconf"]);
        assert!(tightened.env.contains(&"ZLIB_STATIC".to_string()));
        assert!(tightened.env.contains(&"PKG_CONFIG_PATH".to_string()));
        assert!(!tightened.env.contains(&"CC".to_string()));
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

    #[test]
    fn paths_cover_the_paths_under_them() {
        assert!(covers(":manifest_dir", ":manifest_dir/build/probe.rs"));
        assert!(covers("/usr", "/usr/include"));
        assert!(covers("/", "/etc"));
        assert!(!covers("/usr", "/usrlocal"));
        assert!(!covers("/usr/include", "/usr"));
        assert!(covers("TARGET", "TARGET"));
        assert!(!covers("TARGET", "TARGET_CC"));
    }

    #[test]
    fn a_request_narrows_the_crate_root_to_files() {
        let declared = Declared {
            read: vec![":manifest_dir/build/probe.rs".into(), "/etc".into()],
            ..Declared::default()
        };
        let none = SandboxConfig::default();
        let tightened =
            tighten(resolve(&none, &external("dep")), &declared, &external("dep"), &none);
        let mut expected = vec![":manifest_dir/build/probe.rs".to_string()];
        expected.extend(implicit());
        assert_eq!(tightened.read, expected);
    }
}
