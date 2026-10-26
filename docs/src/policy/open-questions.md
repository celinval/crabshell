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
- **What should the default policy grant?** Real build scripts read more than
  `OUT_DIR`: `serde_core` needs `CARGO_PKG_VERSION_PATCH` and `RUSTC`, for
  instance. The default will likely include the variables Cargo sets for build
  scripts, as an explicit list rather than a `CARGO_*` prefix (which would also
  match secrets like `CARGO_REGISTRY_TOKEN`), and read access to the crate's
  own directory. Whether those paths are exposed as relative paths is open.
