# Paths

Policy paths are **symbolic roots plus root-relative subpaths**, not concrete
machine paths. A symbolic root (e.g. `$include_dir`, `$lib_dir`, `$sysroot`,
`$OUT_DIR`, `$CARGO_MANIFEST_DIR`, `$TARGET_DIR`) names a *role*; the runner
resolves it per machine at execution time. Which roots exist and how
toolchain-derived ones resolve still needs refinement.

The prototype supports a subset:

- An absolute path, or `$NAME/sub/path` where `NAME` is one of
  `CARGO_MANIFEST_DIR`, `OUT_DIR`, `CARGO_HOME`, `HOME` or `RUSTUP_HOME`,
  read from the build script's environment. Other roots are an error.
- A trailing `*` in the last component, matching every sibling with that
  prefix (`/lib*`).
- `env` entries also accept a trailing `*` (`CARGO_CFG_*`). When clamping and
  tightening, a pattern covers every entry with its prefix. Paths are
  compared as strings, so `/usr` doesn't yet cover `/usr/include`.
