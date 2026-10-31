# Open questions

- **Declared inputs are the cache key.** The contents of declared `read` paths and
  declared `env` vars are the build script's inputs. Enabling `network` disables
  the cache.
- **Glob patterns.** Deferred, and rejected for now. Because the categories are
  named tables rather than a glob, patterns like `*-sys` can be added later
  without competing with a catch-all for precedence.
- **Vendored dependencies are a known gap.** `cargo vendor` puts third-party
  source inside your project, so the `CARGO_HOME` test classifies it as
  `[local]`. The environment alone cannot distinguish a vendored directory from
  a path dependency; resolving this needs the manifest or `cargo metadata`.
- **Policy changes don't rerun build scripts.** The policy isn't part of
  Cargo's fingerprint, so after a policy edit, results cached under the old
  policy remain until a clean build.
- **A project lint.** E.g. no absolute machine paths in a checked-in
  `sandbox.toml`; those belong in the user's config.
- **Diagnostics need a cargo driver.** crabshell reports problems as
  `cargo::warning` lines (no `CRABSHELL_CONFIG`, a user config narrowing a
  project's policy, network granted to `[external]`), but Cargo redirects
  build-script output and only shows warnings for local packages. A
  `cargo crabshell` driver could collect and show them, and also own setting
  `CRABSHELL_CONFIG` and `host.runner`.
- **Value provenance from Cargo.** crabshell rejects crate-chosen env vars
  (`DEP_*`, `CARGO_PKG_*`) in variable definitions by name only. The
  `host.runner` contract could tell the runner which values Cargo injected,
  which would make the check exact. An upstream ask.
- **Per-platform variable values.** The runtime libraries in implicit access
  depend on the *host* (where build scripts run); paths such as a cross
  sysroot's headers depend on the *target*. Variables will need per-host and
  per-target values, perhaps mirroring Cargo's `[host]` and
  `[target.<triple>]` tables. Today only `x86_64-unknown-linux-gnu` has
  defaults for them.
- **What should the default policy grant?** Real build scripts read more than
  `OUT_DIR`: `serde_core` needs `CARGO_PKG_VERSION_PATCH` and `RUSTC`, for
  instance. The default will likely include the variables Cargo sets for build
  scripts, as an explicit list rather than a `CARGO_*` prefix (which would also
  match secrets like `CARGO_REGISTRY_TOKEN`), and read access to the crate's
  own directory. Whether those paths are exposed as relative paths is open.
- **Should `OUT_DIR` be readable by default?** It's write-only today; a
  policy grants read with `:out_dir`. In the
  [experiments](../build-scripts/experiments.md), 5 of 23 build scripts
  needed it: compile probes read back rustc's output (proc-macro2,
  thiserror, rustix), and cc-rs opens its files read-write (libz-sys,
  openssl-sys). rustix didn't fail without it, but silently dropped cfgs.
  Unlike `TMPDIR`, `OUT_DIR` keeps files from earlier runs, so reading it
  makes them inputs. Alternatives to a default are adding it to the `rustc`
  and `cc` grants, or keeping it per crate. `TMPDIR` is write-only too, and
  only gcc needed to read it; the `cc` grant includes it.
- **Default variables for tool discovery.** Tools already define how they're
  found, and grants should follow the same conventions, or `CC=clang` would
  silently fall outside the `cc` grant. Candidates for built-in variables:
  `:cc` from `CC` (then `CC_<target>`, `TARGET_CC`, `HOST_CC`, else `cc`, as
  cc-rs does), `:pkg_config` from `PKG_CONFIG`, and `:pkgconfig_paths` from
  `PKG_CONFIG_PATH`/`PKG_CONFIG_LIBDIR`. Built-in grants would then use them,
  e.g. `run = [":cc", "as", "ld"]`. Names like `CC_<target>` depend on the
  target, so this ties into per-platform values.
- **Opt-in default grants.** Some named grants are needed by most crates of a
  kind, e.g. `rustc` for compile probes. A one-line opt-in could add such
  grants to every package's default, instead of each policy listing them.
- **Generating machine facts.** crabshell doesn't run host tools during
  builds, but a separate, user-run command (e.g. `crabshell probe`) could run
  the tools' own discovery queries once (`gcc -print-prog-name`,
  `pkg-config --variable pc_path pkg-config`, `ld.so --help`, `readelf` on
  the tools) and write the results to the user config as variables and grant
  extensions. The output stays reviewable and fixed until regenerated, so
  builds stay deterministic. It can go stale after system upgrades (gcc moving
  `cc1` to a new version dir); granting directories rather than versioned
  files limits that. The fixtures' `hosts/*.toml` are hand-written examples of
  such output.
- **Host-conditional configuration.** The policy format can't say "on this kind
  of host, use these grants": the test harness installs a per-distro user
  config (`crabshell/fixtures/hosts/<ID>.toml`) from outside. Cargo-style
  `cfg()` tables can't tell distros apart (Amazon Linux and Ubuntu are both
  `x86_64-unknown-linux-gnu`). Options:
  - built-in grants list every distro's entries, and missing programs are
    skipped like missing paths are (today a `run` entry not on `PATH` is an
    error);
  - conditions on host facts, e.g. `/etc/os-release`'s `ID`, or whether a path
    exists.
