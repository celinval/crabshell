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
| `rustc-probe` | libc, getrandom, nix, parking_lot_core, proc-macro2, quote, rustix, serde, serde_core, thiserror, anyhow, async-io, camino, constant_time_eq, cookie, crc32fast, crossbeam-utils, doc-comment, encoding_rs, erased-serde, eyre, fs-err, heapless, httparse, io-lifetimes, matrixmultiply, memoffset, paste, portable-atomic, portable-atomic-util, proc-macro-error, proc-macro-error-attr, proc-macro-hack, ref-cast, rustversion, slotmap, syn 1, typeid, zmij |
| `system-lib` | openssl-sys, libsqlite3-sys, libz-sys, lzma-sys, zstd-sys (all via pkg-config), libc, libseccomp, libseccomp-sys, pcap, libpulse-sys, libpulse-simple-sys, openmp-sys, x11-dl, tiny-xlib, yeslogic-fontconfig-sys, clang-sys, inotify-sys, odbc-sys, mongocrypt-sys, winapi, num-traits, proc-macro2, quote |
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

Builds see only `PATH`, `CARGO_HOME`, `RUSTUP_HOME`, `HOME`, `TMPDIR`, `RUSTC`,
`RUSTDOC`, `GIT_CEILING_DIRECTORIES` and a fake token. The harness asserts that the `system-lib` baseline leaks the
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
| `cfg-only` | 22 | 0–6 | – | 0 |
| `rustc-probe` | 39 | 0–20 | `rustc` | 0 |
| `system-lib` | 23 | 0–33 | `rustc`, `pkg-config`, `cc` | 0 |
| `codegen` | 46 | 0–20 | `rustc` | 0 |
| `cc-toolchain` | 33 | 0–20 | `rustc`, `cc`, `cxx`, `cmake`, `autotools` | 0 |
| `vcs-metadata` | 43 | 0–36 | `rustc`, `git`, `sh`, `grep`, `awk` | 0 |
| `lang-runtime` | 7 | 1–20 | `rustc`, `python` | 0 |

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
- **Generators find their data through `CARGO_MANIFEST_DIR`.** chrono-tz and
  web_atoms read `$CARGO_MANIFEST_DIR/...` rather than paths relative to the
  working dir, so they fail under the default grant: it reads the crate root
  but doesn't pass the variable naming it.
- **Vendored builds need more tools than linking a system library.** cc-rs
  archives with `ar`, which the built-in `cc` grant now includes. CMake
  needs a grant of its own (`cmake`, `make`, `sh`, `ranlib`, its modules
  under `/usr/share/cmake*`), which the fixture defines since crabshell has no
  built-in one; the cmake crate also probes `c++` for C projects and reports
  a failure as a `cargo:warning`.
- **C++ needs `c++` beyond the `cc` grant.** cc-rs runs `c++` for
  `.cpp(true)` builds (cxx, link-cplusplus, snappy_src, esaxx-rs), and the
  autotools crate looks up the C++ compiler even for C projects. The `cc`
  grant already passes `CXX*`; the fixture adds a one-line `cxx` grant.
- **Configure scripts degrade silently where cargo can't see it.** jemalloc,
  Cyrus SASL and LAME build through `sh`, about 50 coreutils and `make`
  (the fixture's `autotools` grant). Without `file` and its magic database,
  `dd`, `getconf` and `objdump`, configure still succeeds and every `cargo:`
  directive matches, but libtool is configured differently (`LD` without
  `-m elf_x86_64`, a 32 KiB command-line limit, `shlibpath_overrides_runpath`
  flipped). Only comparing `config.status` against the baseline showed it;
  directive comparison can't. A missing `run` entry also fails the whole
  script (`not found on PATH`), so a long tool list is less portable than a
  long read list, whose missing paths are skipped.
- **Target-prefixed knobs recur.** tikv-jemalloc-sys reads
  `<TARGET>_LDFLAGS`, `<TARGET>_JEMALLOC_SYS_*` and so on before the plain
  names; prefix patterns can't express them, so requests list only the plain
  names.
- **The repository is named relative to the crate.** A build script running
  `git` reads the git dir of the repository holding the crate, usually above
  it at the workspace root. For a workspace member the consumer knows the
  layout and grants `:manifest_dir/..`; a built-in `git` grant couldn't,
  since the depth varies and worktrees keep it elsewhere. Without git the
  script still succeeds, embedding `unknown`. git itself fails if
  `/etc/gitconfig` exists but is unreadable, so a `git` grant needs it.
- **Dirty checks read the whole work tree.** `git status` rereads files whose
  index entries can't prove them unchanged (e.g. written in the same second
  as the index). With only the git dir readable, vergen-gitcl reports
  `VERGEN_GIT_DIRTY=true` and shadow-rs lists every file outside its crate as
  dirty, so the `git` grant reads the workspace, not just `.git`.
- **Metadata libraries run more than git.** vergen-gitcl runs every git
  command through `$SHELL -c`, falling back to `sh`; without `sh` it warns
  "no suitable 'git' command found" and sets no `VERGEN_GIT_*`. shadow-rs
  pipes `git status` through `grep` and `awk`, and runs `rustc` and `cargo`
  (even `cargo tree`) by name from `PATH`, not `$RUSTC`/`$CARGO`. Under
  rustup that's the proxy in `~/.cargo/bin`, which the `rustc` grant doesn't
  cover, so those versions depend on the host; the fixture denies those
  constants. built runs `$RUSTDOC -V`, and Cargo passes a bare `rustdoc`
  unless configured: same problem, so the harness now sets `RUSTDOC` like
  `RUSTC`. The `sh`, `grep` and `awk` grants need distro-specific libraries
  (bash's libtinfo, gawk's six), kept in the fixture's grants.
- **Reproducible timestamps need `SOURCE_DATE_EPOCH` granted.** vergen (in
  the members, and in registry crates lrlex and lrpar), built and shadow-rs
  use it instead of the current time. The fixture sets it through Cargo's
  `[env]`; a policy that doesn't pass it silently embeds the build time.
- **Generated files hide degradation.** built and shadow-rs write
  `OUT_DIR/*.rs` and print no directives, so the members also export a
  digest of the file; that's how the dirty-check denial above showed up.
- **Registry crates' git finds no repository,** or the wrong one. Crates
  like wasm-bindgen-shared, typst-utils and crate-git-revision run git in
  their own source dir in `CARGO_HOME`, which has no `.git`; git then walks
  up into whatever repository encloses `CARGO_HOME` (a dotfiles repo, or
  here the crabshell checkout). Denial makes no difference, so the policies
  don't grant them `git`, which would read the whole registry.
  crate-git-revision reads `.cargo_vcs_info.json`, which Cargo publishes,
  and requests it. typst-utils reads its knobs with `option_env!`, when the
  script compiles, so no policy affects them.
- **zerocopy already uses `[package.metadata.build-rs]`.** It keeps its
  version cfgs there, and crabshell fails its build on the unknown keys.
  `names` (through rand) was left out of the fixture for this; crabshell
  needs another table name, or to ignore tables it can't parse.
- **Some scripts read their whole environment.** mach-sys hashes it, with
  the time, into a random nonce; built and shadow-rs collect it but only
  look up names. Requests cover only the names used.
- **Build-dependency libraries read env on the crate's behalf.** pyo3,
  pyo3-ffi and pyo3-macros-backend each call pyo3-build-config, so each
  requests what that library reads (`TARGET`, `PYO3_*`), and pyo3 reads
  the interpreter config pyo3-ffi passes on as `DEP_PYTHON_PYO3_CONFIG`.
  An empty request narrows the default grant away too: target-lexicon needs
  `TARGET` requested explicitly.
- **ring checks its own identity.** It asserts `CARGO_MANIFEST_LINKS` against
  `CARGO_PKG_NAME` and `CARGO_PKG_VERSION_*`, so it fails outright without
  them.
- **autocfg needs `OUT_DIR` read back.** Like proc-macro2's probes, without it
  num-traits succeeds but silently drops `has_total_cmp`.
- **`[package.metadata.build-rs]` is already taken.** zerocopy (and so
  ahash) keeps its own version table there, which crabshell rejects as a
  malformed declaration, failing the build. The key needs a name no crate
  uses, or crabshell must tolerate tables it can't parse; both are left out
  of `rustc-probe` until then.
- **Probes inherit env the script never reads.** eyre's compile probe uses a
  nightly feature, which on stable works only because rustc inherits
  `RUSTC_BOOTSTRAP`. Without it in eyre's request, it silently drops
  `generic_member_access`; without `:out_dir` read, it also drops
  `track_caller`, since rustc can't read the probe the script wrote there.
  anyhow instead fails outright without `:out_dir`: it can't list its probe
  dir to remove it.
- **Many denials are invisible on the test host.** autocfg's version checks
  (async-io, fs-err, memoffset, matrixmultiply) only parse `rustc --version`,
  so without `:out_dir` they just warn that `std` can't be probed. heapless's
  probe and io-lifetimes's only matter on ARM or WASI targets, zmij's
  `OPT_LEVEL` only at `s`/`z`, httparse's `CARGO_CFG_TARGET_FEATURE` only with
  `sse4.2` or `avx2` enabled. The directive comparison can't catch these.
- **Version checks can run at compile time.** constant_time_eq and
  encoding_rs use rustversion's `cfg!` macro, evaluated when the build
  script is compiled, so the script needs no `rustc` grant; rustversion's own
  build script does.
- **Running `rustc` from `PATH` can't be requested portably.** doc-comment
  ignores `$RUSTC`; under rustup, `rustc` on `PATH` is a proxy needing its
  own env, so the request leaves it denied. Its detection fails silently,
  which only matters before rustc 1.30.
- **Some knobs can't be expressed portably.** openssl-sys reads each knob
  also prefixed with the target triple (`X86_64_UNKNOWN_LINUX_GNU_OPENSSL_DIR`).
  Patterns only match prefixes, so the request has to name the triple.
- **System-library fallbacks hide a denied probe.** Without pkg-config,
  libpulse-sys and libpulse-simple-sys link the library by soname, pcap
  `dlopen`s `libpcap.so` and, failing that, assumes the newest libpcap
  (every `libpcap_*` cfg), and libseccomp drops its `libseccomp_v2_6` cfg.
  None fails. This host has none of their `-dev` packages, so the baseline
  takes the same fallbacks and the fixture can't show the difference.
- **Some probes only reach generated source.** x11-dl and tiny-xlib (with
  `dlopen`) write each X11 library's pkg-config libdir into `OUT_DIR`, or
  `None`. Denied, they emit the same directives, so comparing directives
  can't catch it; the harness would have to compare `OUT_DIR` too.
- **`dlopen` in a build script needs the library dir readable.** pcap loads
  `libpcap.so` by name, which no request can turn into a portable path. Only
  the `cc` grant's library dirs happen to cover it.
- **cc-rs writes into `OUT_DIR` even to ask for search dirs.** openmp-sys
  only runs `cc -print-search-dirs`, but cc-rs's compiler-family detection
  opens a file in `OUT_DIR` read-write. Denied, it warns and guesses the
  family, which openmp-sys uses to tell clang and MSVC apart.
- **Some knobs are read when the script is compiled.** odbc-sys reads
  `ODBC_SYS_STATIC_PATH` and `DYLD_LIBRARY_PATH` with `option_env!`, so rustc
  bakes them into the build script, out of crabshell's reach and missing from
  `rerun-if-env-changed`.
- **Per-library knobs multiply.** x11-dl asks pkg-config about 16
  libraries, so it requests 32 `<LIB>_STATIC`/`<LIB>_DYNAMIC` names; the tight
  policy needs 16 prefix patterns for them. Most of the category's other
  crates only emit link directives on Linux and request 0–3 names.
- **The only machine-specific config is the consumer's.** On Amazon Linux the
  built-in `pkg-config` grant needs extending (`sh`, `rpm` and rpm's
  libraries); no request changes.
- **A crate's own script being cfg-only doesn't make its build cfg-only.**
  `cfg-only` keeps working under the default grant alone, but the category's
  most downloaded crates had to stay out of it. libm's script fails without
  `OPT_LEVEL` and `CARGO_MANIFEST_DIR`, which it unwraps. serde_json's own
  script reads only `CARGO_CFG_*`, but it depends on serde_core (`$RUSTC`,
  `CARGO_PKG_VERSION_PATCH`) and zmij (`$RUSTC`). A version probe somewhere
  below rules out most of the rest: libc (assert_cmd, quinn-udp, rlimit,
  kvm-ioctls, wee_alloc), heapless (embassy-time-queue-utils),
  portable-atomic (atomic-time), bare-metal (cortex-m, panic-probe,
  nrf-pac), slotmap (ic-cdk-executor) and proc-macro2 (anything using
  syn).
- **Denying an `OUT_DIR` listing is silent.** scratch empties its `OUT_DIR`
  with `remove_dir_all` and ignores errors. `OUT_DIR` is write-only, so
  opening it fails and stale contents stay. A fresh build looks the same, so
  only the request (`read = [":out_dir"]`) records the need.
- **Some knobs forward a host path to rustc.**
  iceoryx2-pal-configuration passes `IOX2_CUSTOM_PLATFORM_CONFIGURATION_PATH`
  on with `rustc-env`, and the crate `include!`s that file, so rustc reads it
  outside the sandbox. Denying the variable silently selects the built-in
  configuration.
- **Requests can't depend on the target.** tree-sitter-language reads
  `CARGO_MANIFEST_DIR` only for `wasm32-unknown-*`. The vendor still
  requests it, so a tight policy grants it on every target.
- **`DEP_*` isn't in the default grant.** rb-sys-env replays
  `DEP_RB_ENCODED_CARGO_ARGS`, which is unset here since nothing it depends
  on has `links = "rb"`. The `dependency_version_cfg` crates (openssl,
  native-tls, curl, hyper-openssl) read `DEP_OPENSSL_*` the same way, and
  would need it requested and granted; they pull `-sys` crates anyway.
- **Empty build scripts cost nothing.** embassy-time-driver (for its `links`
  key) and nextest-workspace-hack run `fn main() {}` and request nothing.

### More generators

The `codegen` fixture grew to 46 build scripts: table generators (snap,
selectors, terminfo, cgmath, gl), generators reading their own data
(oid-registry, gherkin, iso_currency, grok, tls-parser, khronos_api), a
lalrpop parser (basic-cookies), a linker script (defmt) and env-configured
constants (smoltcp, fastnum). Every cell behaves as expected, and beyond the
directives, every generated file in `OUT_DIR` matched the baseline byte for
byte.

- **A crate already uses `[package.metadata.build-rs]`.** zerocopy keeps its
  version cfgs there (`no-zerocopy-simd-x86-avx12-1-89-0 = "1.89.0"`) and
  its build script parses its own `Cargo.toml` for them. crabshell rejects
  the table's unknown fields, so zerocopy's build script fails in every
  scenario without requests. zerocopy, a dependency of rand's `std_rng`, is
  among the most downloaded crates; the fixture leaves it out (with
  unicode_names2, which pulled it in). The metadata key needs a name of its
  own, or crabshell must ignore tables it can't parse.
- **Most generators open their data relative to the working dir,** the
  crate root, so they work under the default grant's crate-root read. Only
  those using `CARGO_MANIFEST_DIR` (chrono-tz, web_atoms, tls-parser) need
  the variable.
- **lalrpop walks the whole crate root** for `.lalrpop` files, not just
  `src`, so a crate using it requests `:manifest_dir`. Narrowed to `src`, it
  fails loudly; so does grok's `glob`, when its patterns dir can't be listed.
- **Env-configured constants degrade silently.** smoltcp scans `env::vars()`
  for `SMOLTCP_*` and fastnum reads `RUST_FASTNUM_*`; denied, both fall back
  to their defaults. The difference lands only in generated files, not in
  directives, so a baseline comparison of directives can't catch it. The
  same holds for `CARGO_CFG_TARGET_FEATURE` (target-spec, jetscii) and
  lalrpop's `CARGO_FEATURE_*` scan, though the default grant covers those.
- **Some generators can't be compared to a baseline.** lrlex and lrpar
  (through promql-parser) run vergen, which emits the build time as
  `cargo:rustc-env`, so no two builds print the same directives. They're
  left out.
- **Compile probes can be blind to denial.** heapless compiles an ARM
  `clrex` probe from `OUT_DIR`; on x86 it fails whether or not rustc can
  read it.

## Methodology TODOs

- **A control variant for crabshell's own effect.** A permissive policy
  (`read = ["/"]`, `env = ["*"]`), run inside the jail and compared against the
  baseline, would show that crabshell's mechanism alone (env re-export,
  private `TMPDIR`, nested Landlock) doesn't change behavior. Today that's
  assumed. Worth adding once the policy contract settles.
