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
- **User vs project config.** Parked. A Cargo-style optional include is
  probably enough for now; a project lint (e.g. no absolute machine paths in a
  checked-in `sandbox.toml`) can come later.
- **Combining path lists.** A variable's alternatives mean "first defined
  wins", which fits `PKG_CONFIG_LIBDIR` (it replaces the default search path).
  `PKG_CONFIG_PATH` instead *extends* it, so definitions also need a way to
  concatenate lists.
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
