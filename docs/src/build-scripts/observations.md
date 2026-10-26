# Observations

- **Two small categories cover half the scripts.** `cfg-only` plus
  `rustc-probe` cover 48% of labelled build scripts. The floor plus Cargo's
  own env vars plus `$RUSTC` would let them run unmodified, which supports
  widening the default (see
  [Open questions](../policy/open-questions.md)).
- **Labels mix across tiers.** `openssl-sys`, `zstd-sys`, `libsqlite3-sys`
  and `aws-lc-sys` combine `system-lib`, `cc-toolchain` and `bindgen`, choosing
  between them by feature or env var. A crate needs to request several
  categories, and which one is used depends on the consumer's configuration.
- **Policy depends on user input** for `system-lib` (where the library is),
  `lang-runtime` (which interpreter), `environment_config_constants` (which
  crate-specific vars), and `network-fetch` (what may be downloaded). A
  category alone can't resolve these. The consumer must bind them, e.g. by
  setting a symbolic root or allowing a named env var.
- **Outputs matter as well as inputs.** `linker_script_link_args` and
  `language_binding_config` emit `rustc-link-arg` / rpath directives, and
  `build_metadata_embedding` emits `rustc-env`. A category could also limit
  which `cargo::` directives a script may emit (e.g. `cfg-only` emits only
  `rustc-cfg`, `rustc-check-cfg`, `rerun-if-*` and warnings).
- **Some scripts write outside `OUT_DIR`.** `prometheus` (`gen` feature)
  writes generated code into its source tree. `tauri-plugin-fs` generates
  permission files. `rmcp` sets `core.hooksPath` in the workspace's git
  config. `scratch` deletes and recreates `OUT_DIR`. A read-only source tree
  will break the first two (acceptable), and blocks the third (intended).
- **The jobserver is a hidden dependency** of `cc-toolchain`: `cc` and `make`
  read inherited file descriptors named in `CARGO_MAKEFLAGS`.
- **Network downloads often have an offline escape hatch** (an env var pointing
  at a local artifact). Granting that env var plus a read path may be
  preferable to granting `network`.
