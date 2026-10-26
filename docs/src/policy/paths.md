# Paths

Policy paths are **symbolic roots plus root-relative subpaths**, not concrete
machine paths. A symbolic root (e.g. `$include_dir`, `$lib_dir`, `$sysroot`,
`$OUT_DIR`, `$CARGO_MANIFEST_DIR`, `$TARGET_DIR`) names a *role*; the runner
resolves it per machine at execution time. Which roots exist and how
toolchain-derived ones resolve still needs refinement.
