# Category experiments

We ran real crates from three [categories](categories.md) under crabshell's
Landlock backend, to see what each category actually needs.

## Setup

Each fixture under `e2e-tests/` is a standalone workspace whose only crate
depends on crates of one category. Its `Cargo.lock` is committed. Every fixture
has several `sandbox.*.toml` policy variants:

| Fixture | Crates with build scripts |
|---|---|
| `cfg-only` | crossbeam-utils, crossbeam-epoch, crossbeam-deque, rayon-core, tiny-keccak, crunchy, icu_properties_data |
| `rustc-probe` | libc, getrandom, nix, parking_lot_core, proc-macro2, quote, rustix, serde, serde_core, thiserror |
| `system-lib` | openssl-sys, libsqlite3-sys, libz-sys, lzma-sys, zstd-sys (all via pkg-config), libc |
| `declared` | crossbeam-utils, patched to declare `[package.metadata.build-rs]` |

`cargo test -- --ignored` (`crabshell/tests/workspaces.rs`) first builds each
fixture without the sandbox, as a baseline. It then builds the fixture once per
variant and compares every build script's `cargo:` directives with the
baseline. **A build that succeeds isn't enough:** many build scripts degrade
silently when access is denied.

Environment: Amazon Linux 2023, kernel 6.12, nightly Cargo with
`-Zhost-config -Ztarget-applies-to-host`. No static glibc is installed.

## Results

| Fixture | Variant | Result |
|---|---|---|
| all | floor only | fails: the dynamic loader isn't readable |
| `cfg-only` | per-package grants | same as baseline |
| `cfg-only` | `[external]` bundle | same as baseline |
| `rustc-probe` | per-package grants | same as baseline |
| `rustc-probe` | `[external]` bundle | same as baseline |
| `rustc-probe` | bundle under `[limits] read = ["/lib*"]` | fails (rustix); 5 other scripts silently emit different cfgs |
| `system-lib` | per-package grants | same as baseline |
| `system-lib` | `[external]` bundle | same as baseline |
| `system-lib` | bundle without `ld` | **succeeds, but libz-sys links its vendored zlib** |
| `declared` | broad grant, crate declares less | same as baseline; the script sees only what it declared |
| `declared` | grant narrower than declaration | succeeds with a warning; the declaration can't widen |

The bundles that matched the baseline, as `sandbox.toml` fragments:

```toml
# cfg-only: the loader, plus env vars Cargo itself defines.
read = ["/lib*"]
env = [
    "TARGET", "HOST", "PROFILE", "OPT_LEVEL", "DEBUG", "NUM_JOBS",
    "CARGO_CFG_*", "CARGO_FEATURE_*", "CARGO_PKG_*", "CARGO_MANIFEST_*", "DEP_*",
]

# rustc-probe adds:
read  = ["$RUSTUP_HOME/toolchains"]
env   = ["RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTC_LINKER", "CARGO_ENCODED_RUSTFLAGS"]

# system-lib adds:
read  = ["/usr/lib64/pkgconfig", "/usr/share/pkgconfig", "/usr/include", "/usr/libexec/gcc"]
run   = ["pkg-config", "sh", "rpm", "x86_64-amazon-linux-gnu-pkg-config", "pkgconf", "cc", "as", "ld"]
env   = ["PKG_CONFIG*", "HOST_PKG_CONFIG*", "CC*", "CFLAGS*", "HOST_CC", "HOST_CFLAGS",
         "CRATE_CC_NO_DEFAULTS", "SOURCE_DATE_EPOCH", "PATH"]
```

## Findings

- **Denial often degrades silently instead of failing.** Some examples:
  - With `$RUSTC` unreadable, proc-macro2, quote, serde, serde_core and
    thiserror assume an old compiler and emit different cfgs.
  - Without `ld`, libz-sys can't link its zlib check and builds the vendored
    copy instead.
  - crossbeam-utils only warns without `TARGET`.
  - Optional knobs (`CARGO_CFG_SANITIZE`, `ICU4X_DATA_DIR`) are read with
    `unwrap_or_default()`.

  Policy authors need a way to see what a script *tried* to access, e.g. an
  audit mode, or Landlock's audit logging on newer kernels.
- **Two bundles cover the first two categories without crate-specific
  knowledge.** The `cfg-only` and `rustc-probe` bundles above worked
  unchanged for every crate in those fixtures. Crate-specific user knobs
  (`RUSTC_BOOTSTRAP`, `RUST_LIBC_UNSTABLE_*`, `OPENSSL_DIR`) can stay out of
  bundles: denying them selects the default, which is what an unconfigured
  build does anyway.
- **The crate-root-only floor can't run a dynamic build script.** The kernel
  loads the interpreter (`/lib64/ld-linux-*.so`) and libc from inside the
  sandbox. Either the floor includes the loader, every policy grants
  `/lib*`, or build scripts link statically (`[host] rustflags =
  ["-C", "target-feature=+crt-static"]`, which needs static glibc).
- **`/dev/null` must be writable.** Compile probes silence rustc with
  `Stdio::null()`, which opens it read-write. It's now part of the floor.
- **`run` hides transitive executables.** On Amazon Linux `pkg-config` is a
  shell script that runs `rpm --eval` and then a second script that execs
  `pkgconf`. `cc` execs `cc1`, `as`, `collect2` and `ld`, and finds `cc1`
  through `PATH`. A `run` grant is only useful together with the files and
  env that program needs, which is what a bundle should package. The chain is
  distro-specific.
- **Compilers need a temp dir, but `/tmp` is a shared channel.** gcc writes
  temporary files; granting `/tmp` would let build scripts leave files for
  each other. The floor now gives each run a private `TMPDIR` instead, and gcc
  honors it.
- **`rerun-if-env-changed` lists what a script reads.** It's an exact,
  machine-readable record of the env vars a script consults, including names
  built at runtime such as `PKG_CONFIG_PATH_x86_64-unknown-linux-gnu`. Tooling
  could suggest `env` grants from a baseline run.
- **Prefix patterns are needed for env.** `CARGO_CFG_*` alone covers about 25
  variables. Clamping and tightening intersect patterns by prefix.
- **Unsandboxed, scripts leak the whole environment into build output.**
  cc-rs prints each compiler invocation along with its full environment. In the
  baseline that put every session variable, tokens included, into
  `target/**/run/stdout`.
- **Cargo doesn't rerun build scripts when the policy changes.** The policy
  isn't part of Cargo's fingerprint, so after a policy edit, cached results
  from the old policy remain.
- **Toolchain roots need roles.** `$RUSTUP_HOME/toolchains` works only under
  rustup; a `$sysroot` role derived from `$RUSTC` would be portable.
