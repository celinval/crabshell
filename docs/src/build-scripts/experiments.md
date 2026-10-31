# Category experiments

We ran real crates from three [categories](categories.md) under crabshell's
Landlock backend, to see what each category actually needs.

## Setup

Each fixture under `crabshell/fixtures/` is a standalone workspace whose only crate
depends on crates of one category. Its `Cargo.lock` is committed. Every fixture
has several `sandbox.*.toml` policy variants:

| Fixture | Crates with build scripts |
|---|---|
| `cfg-only` | crossbeam-utils, crossbeam-epoch, crossbeam-deque, rayon-core, tiny-keccak, crunchy, icu_properties_data |
| `rustc-probe` | libc, getrandom, nix, parking_lot_core, proc-macro2, quote, rustix, serde, serde_core, thiserror |
| `system-lib` | openssl-sys, libsqlite3-sys, libz-sys, lzma-sys, zstd-sys (all via pkg-config), libc |
| `declared` | crossbeam-utils, patched to declare `[package.metadata.build-rs]` |

`cargo test --features test-requests` (`crabshell/fixtures.rs`) first builds each
fixture without the sandbox, as a baseline. It then builds the fixture once per
variant and compares every build script's `cargo:` directives with the
baseline. **A build that succeeds isn't enough:** many build scripts degrade
silently when access is denied.

The crates under test may be untrusted, so every build runs inside an outer
**jail** (`probe`'s `jail` helper, reusing crabshell's confinement) around the
whole `cargo build`. That includes proc-macros, which crabshell never sees. The
jail has no network, reads only the toolchain, the fixture and the fetched
crates, and writes only the build's target dir and a scratch `TMPDIR`. The
baseline is the jail without crabshell; each variant adds crabshell inside it.
For trusted fixtures, an unjailed reference build must match the baseline,
showing that the jail itself doesn't change behavior.

Each test run gets a fresh scratch dir holding:

- `CARGO_HOME`, filled by `cargo fetch` before the jail starts (fetching runs
  no crate code), so it holds no credentials;
- an empty `HOME`;
- one target dir per build.

Builds see only `PATH`, `CARGO_HOME`, `RUSTUP_HOME`, `HOME`, `TMPDIR`, `RUSTC`
and a fake token. The harness asserts that the `system-lib` baseline leaks the
fake token (see below), and that no sandboxed build does.

Environment: Amazon Linux 2023, kernel 6.12, `nightly-2026-10-13` (pinned in
`crabshell/fixtures/rust-toolchain.toml`) with `-Zhost-config -Ztarget-applies-to-host`.
No static glibc is installed.

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

## Round 2: requests × grants

The fixtures were reworked into a matrix (see `crabshell/fixtures/README.md`): crates
with no requests or with precise vendor-view requests (`requests.toml`), under
the default grant, the union of all requests, or a loose policy. With the
loader and libc as implicit access and named grants for `rustc`, `cc` and
`pkg-config`, all three fixtures behave as expected in every cell: the default
grant alone fails, and every other cell matches the baseline. Under the loose policy, requests
narrow `env = ["*"]` to exactly the names requested.

What the requests cost, per build script:

| Fixture | Scripts | Env names per request | Grants requested | Paths or programs |
|---|---:|---|---|---:|
| `cfg-only` | 7 | 0–2 | – | 0 |
| `rustc-probe` | 10 | 0–20 | `rustc` | 0 |
| `system-lib` | 6 | 6–22 | `rustc`, `pkg-config`, `cc` | 0 |

- **No request names a path or a program.** The floor and named grants
  absorb all host knowledge, so a vendor can write requests without knowing
  the user's system.
- **Env names are the whole remaining cost,** and about half are variables
  Cargo itself defines (`CARGO_CFG_*`, `CARGO_FEATURE_*`, `TARGET`, `HOST`).
  Granting those by default would roughly halve the largest requests (`libc`,
  `openssl-sys`); see the open question on the default policy.
- **8 of 23 build scripts read their own crate,** so they request the files
  they use, e.g. thiserror's `build/probe.rs` or libz-sys's `src/smoke.c`;
  requests otherwise narrow the crate root away. Two of them, proc-macro2 and
  thiserror, compile probe sources from their crate: without that request
  they still succeed, but silently lose cfgs.
- **Some knobs can't be expressed portably.** openssl-sys reads each knob
  also prefixed with the target triple (`X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR`).
  Patterns only match prefixes, so the request has to name the triple.
- **The only machine-specific config is the consumer's.** On Amazon Linux the
  built-in `pkg-config` grant needs extending (`sh`, `rpm` and rpm's
  libraries); no request changes.

## Methodology TODOs

- **A control variant for crabshell's own effect.** A permissive policy
  (`read = ["/"]`, `env = ["*"]`), run inside the jail and compared against the
  baseline, would show that crabshell's mechanism alone (env re-export,
  private `TMPDIR`, nested Landlock) doesn't change behavior. Today that's
  assumed. Worth adding once the policy contract settles.
