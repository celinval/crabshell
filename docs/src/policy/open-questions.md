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
