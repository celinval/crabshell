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
| `rustc-probe` | libc, getrandom, nix, parking_lot_core, proc-macro2, quote, rustix, serde, serde_core, thiserror, anyhow, async-io, camino, constant_time_eq, cookie, crc32fast, crossbeam-utils, doc-comment, encoding_rs, erased-serde, eyre, fs-err, heapless, httparse, io-lifetimes, matrixmultiply, memoffset, paste, portable-atomic, portable-atomic-util, proc-macro-error, proc-macro-error-attr, proc-macro-hack, ref-cast, rustversion, slotmap, syn 1, typeid, zmij, allocative, atomic-traits, bigdecimal, bnum, branches, circular-queue, const_fn, crc32c, decorum, erasable, error-chain, field-offset, fixed, flatbuffers, float16, hex_lit, im-rc, libm, maybe-uninit, multer, no-panic, num-traits, proc-macro2-diagnostics, rstest_macros, s2n-quic-platform, slice-dst, slog, stabby-abi, stabby-macros, substring, wasm-bindgen, wasm-bindgen-shared |
| `system-lib` | openssl-sys, libsqlite3-sys, libz-sys, lzma-sys, zstd-sys, alsa-sys (all via pkg-config), glib-sys, gobject-sys (via system-deps), libdbus-sys, magic-sys, libsystemd-sys, pcsc-sys, x11, libc, libseccomp, libseccomp-sys, pcap, libpulse-sys, libpulse-simple-sys, openmp-sys, x11-dl, tiny-xlib, yeslogic-fontconfig-sys, clang-sys, inotify-sys, khronos-egl, odbc-sys, mongocrypt-sys, coremidi-sys, libxdo-sys, midir, msvc_spectre_libs, winapi, winapi-x86_64-pc-windows-gnu, windows_x86_64_msvc, wit-bindgen-rt, system-deps, target-lexicon, typeid, num-traits, proc-macro2, quote |
| `declared` | crossbeam-utils, patched to declare `[package.metadata.crabshell]` |

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
| `cfg-only` | 38 | 0–6 | – | 0 |
| `rustc-probe` | 87 | 0–20 | `rustc`, `cc` | 0 |
| `system-lib` | 42 | 0–52 | `rustc`, `pkg-config`, `pkg-config-path`, `cc` | 0 |
| `codegen` | 102 | 0–20 | `rustc`, `cc`, `git` | 0 |
| `cc-toolchain` | 53 | 0–20 | `rustc`, `cc`, `cxx`, `clang`, `cmake`, `autotools`, and `git`, `pkg-config`, which no policy grants | 0 |
| `vcs-metadata` | 99 | 0–36 | `rustc`, `git`, `git-repo`, `sh`, `grep`, `awk` | 0 |
| `lang-runtime` | 50 | 0–20 | `rustc`, `python`, `java` | 0 |
| `protoc` | 90 | 0–20 | `rustc`, `cc`, `protoc` | 2 |
| `bindgen` | 41 | 0–20 | `rustc`, `cc`, `bindgen`, `pkg-config` | 2 |
| `network-fetch` | 67 | 0–20 | `rustc`, `cc`, `cxx`, `pkg-config`, `bindgen`, `autotools` | 6 |

- **Few requests name a path or a program.** The floor and named grants
  absorb most host knowledge, so a vendor can write requests without knowing
  the user's system. The exceptions are tools no grant covers or that only a
  download path runs (drm-fourcc's `rustfmt`, pq-sys's `pg_config`,
  protosol's bundled protoc, hdfs-native's `cp`, and `curl`, `zig` and `z3`
  in network-fetch), and consumer variables naming a local copy
  (`:duckdb_lib_dir`, `:libdrm_include_path`).
- **Env names are the whole remaining cost,** and about half are variables
  Cargo itself defines (`CARGO_CFG_*`, `CARGO_FEATURE_*`, `TARGET`, `HOST`).
  Granting those by default would roughly halve the largest requests (`libc`,
  `openssl-sys`); see the open question on the default policy.
- **Many build scripts read their own crate,** so they request the files
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
- **A C-only configure still probes the C++ compiler.** libffi is pure C, yet
  libffi-sys's configure runs libtool's C++ checks, looking for `g++` before
  `c++`. Requesting `cc` and the coreutils of `autotools` denied exec of
  `/usr/bin/g++`: every C++ test failed, and the generated `libtool` and
  `config.status` lost the whole C++ tag (`compiler_CXX=''`, `LD_CXX=''`, no
  PIC flags) while all four `cargo:` directives still matched. The
  `autotools` grant now runs `c++`, which on Fedora-like distributions is the
  same file as `g++`, so Landlock's exec rule, which attaches to the file,
  covers both; where a distro ships two binaries, the probe would degrade
  again. What then remains in `config.log` is configure's platform banner,
  which runs `/bin/arch` and records `unknown` without it.
- **Crates pick clang over the default compiler, and cc-rs says so.** minicov
  compiles the LLVM profiling runtime with `compiler("clang")` and
  `-nostdlibinc`, and aegis runs `clang --version` and uses clang when that
  succeeds. A one-line `clang` grant is enough on top of `cc`, whose `/lib64`
  and `/usr/lib64` reads already cover clang's resource headers. Denying it
  fails minicov, which names clang and has no fallback. aegis falls back to
  the default compiler, but not silently: cc-rs declares the knobs of the
  compiler family it picked, so the baseline prints
  `rerun-if-env-changed=CLANG_NO_DEFAULT_CONFIG` and `CCC_OVERRIDE_OPTIONS`
  where the fallback prints `GCC_EXEC_PREFIX` and the `CC`, `HOST_CC`,
  `CC_<target>` lookups. The objects' `.comment` section confirms which
  compiler ran.
- **A script that reads its whole environment can only be requested by
  pattern.** sqlite3-src walks `env::vars()` and turns every `SQLITE_*` name
  into a define for the amalgamation, so the library's feature set depends on
  which of them the policy passes; the request names `SQLITE_*`, which is as
  precise as it can be. libffi-sys also reads `cp --help` before copying the
  vendored sources with `cp -R`, and unwraps the output: without `cp` it
  panics instead of degrading.
- **Requests name grants the policy will never give.** brotli-sys runs `git
  submodule update --init` when its vendored tree holds no `.git`, ignoring
  the result; in `CARGO_HOME` there's no repository, so denying exec changes
  no directive, and the request still names the `git` grant another fixture
  defines. bzip2-sys, libdeflate-sys and sqlite3-src likewise request
  `pkg-config` for the feature sets that look for a system library; the
  policies grant it to none of them, which is what keeps the vendored path,
  and the fixture, independent of whether the host has `bzip2.pc`.
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
- **zerocopy already used `[package.metadata.build-rs]`,** the key
  crabshell first read requests from. It keeps its own version table
  there (`no-zerocopy-core-error-1-81-0 = "1.81.0"`), which crabshell
  rejected as a malformed request, failing zerocopy's build script, and
  every crate pulling it in (rand, ahash), without requests. crabshell
  now reads `[package.metadata.crabshell]` and ignores other tools'
  tables; zerocopy runs `$RUSTC --version` and reads its own
  `Cargo.toml`.
- **Some scripts read their whole environment.** mach-sys hashes it, with
  the time, into a random nonce; built and shadow-rs collect it but only
  look up names. Requests cover only the names used.
- **Build-dependency libraries read env on the crate's behalf.** pyo3 and
  pyo3-ffi both call pyo3-build-config, so each requests what that library
  reads (`TARGET`, `PYO3_*`), and pyo3 reads the interpreter config pyo3-ffi
  passes on as `DEP_PYTHON_PYO3_CONFIG`. An empty request narrows the default
  grant away too: target-lexicon needs `TARGET` requested explicitly.
- **Interpreter config travels as link metadata.** Only pyo3-ffi runs
  Python; the build scripts of numpy, jiter and malachite-nz read the result
  as `DEP_PYO3_PYTHON_PYO3_CONFIG` (or `DEP_PYTHON_PYO3_CONFIG`) and panic
  without it. Most binding crates need no runtime at all: rustler and
  rustler_sys generate NIF glue from vendored headers, napi and jni only
  emit cfgs, uniffi parses a UDL file. So missing Erlang, Swift, Go, OCaml
  or PHP didn't block them.
- **A crate's own features reach its build script only as environment.**
  pyo3-ffi picks how Python is linked from `CARGO_FEATURE_EXTENSION_MODULE`,
  falling back to `PYO3_BUILD_EXTENSION_MODULE`. A request listing only the
  crate's `PYO3_*` knobs narrows `CARGO_FEATURE_*` away, and the script then
  links `libpython` and adds an rpath to it into a build that asked for an
  extension module: the policy changed what the crate links. Every feature a
  script branches on has to be requested by name.
- **One `links` key per lock caps the fixture's versions.** pyo3-ffi claims
  `links = "python"`, so a workspace holds one pyo3 generation: malachite-nz
  takes 0.29 and clarabel's `python` feature 0.25, so the fixture moved to
  0.29 (where pyo3-macros-backend has no build script) and clarabel's Python
  path stays out of reach.
- **Build metadata can re-enter Cargo.** clarabel's `buildinfo` feature runs
  vergen's cargo builder, which spawns `cargo metadata`, and its sysinfo
  builder reads the host's name, user, memory and CPU. Running Cargo has no
  grant (it would read the whole registry), and vergen propagates the error
  instead of degrading, so the fixture leaves the feature off and the request
  records the knobs.
- **Android paths can't be requested.** mundy compiles its Java support
  classes with `javac` and dexes them through `java -jar d8.jar`, taken from
  a JDK and an Android SDK named by `JAVA_HOME`, `ANDROID_HOME` and friends,
  or from whatever `java` on `PATH` resolves to. No grant covers either, and
  no request can name their paths, so mundy's asks for a `java` grant that no
  fixture defines; on Linux its script reads `CARGO_CFG_TARGET_OS` and stops.
- **A pyo3 user still has cfgs of its own to lose.** malachite-nz reads
  `CARGO_CFG_TARGET_FEATURE` with `unwrap_or_default` to decide whether to
  compile a second copy of its FFT kernels for AVX2; a request that leaves it
  out drops them silently.
- **The category's remaining crates need runtimes this host hasn't got.**
  Erlang, OCaml, PHP, Go and Swift are missing, so ocaml-sys,
  ocaml-boxroot-sys, ext-php-rs and envtest stay out; the pctx runtimes build
  a V8 snapshot, which needs V8 downloaded and run; tauri needs GTK and
  WebKit through pkg-config. rb-sys and magnus only need an installed Ruby,
  but this host's sits in a non-standard prefix whose subdirectories symlink
  into separate package trees, all outside the jail's roots. blazediff's
  `napi` feature would bring napi 2 next to the fixture's napi 3 and pull its
  vendored C codecs in with it.
- **Fallbacks hide denied variables on cross builds.** rustler and
  rustler_sys fall back to the host's family when `CARGO_CFG_TARGET_FAMILY`
  is missing: the same output natively, wrong output cross-compiling to
  Windows.
- **`rerun-if-env-changed` misses or overstates what's read.** napi-build
  declares knobs it never reads (`NAPI_TYPE_DEF_TMP_FOLDER`,
  `DEBUG_GENERATED_CODE`), as uniffi does `UNIFFI_TESTS_DISABLE_EXTENSIONS`,
  while uniffi reads `UNIFFI_FORCE_CONTRACT_VERSION` undeclared. The
  baseline's list is a hint, not a request.
- **Target-specific needs stay unverified.** napi-build's Windows, WASI and
  Android setups read `LIBNODE_PATH`, `EMNAPI_LINK_DIR` or `WASI_SDK_PATH`
  and run more tools; the requests list only what the Linux path reads.
- **ring checks its own identity.** It asserts `CARGO_MANIFEST_LINKS` against
  `CARGO_PKG_NAME` and `CARGO_PKG_VERSION_*`, so it fails outright without
  them.
- **autocfg needs `OUT_DIR` read back.** Like proc-macro2's probes, without it
  num-traits succeeds but silently drops `has_total_cmp`.
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
- **A consumer's own `.pc` directory needs a grant of its own.** Most of the
  category's remaining crates probe libraries installed on neither CI nor the
  test host, so the fixture ships stand-in `.pc` files under `pkgconfig/`,
  with no host paths in them, and points the crates
  at them with `PKG_CONFIG_PATH`, as a cross-compilation sysroot or a vcpkg
  prefix would. That directory can't ride in `[external] read`: requests
  narrow it away, and no crate can name a prefix the consumer chose. It has
  to be part of a named grant the crate requests. Extending the built-in
  `pkg-config` grant doesn't work either, since a machine-facts user config
  (`fixtures/hosts/*.toml`, or `crabshell-probe`'s output) replaces a
  project's grant of the same name instead of layering over it. The fixture
  defines `pkg-config-path`, reading `:pkgconfig_dir` imported from
  `PKG_CONFIG_PATH` — which a built-in `:pkgconfig_paths` would make
  unnecessary.
- **system-deps probes from the crate's own manifest.** glib-sys's and
  gobject-sys's build scripts name no library at all: system-deps reads
  `[package.metadata.system-deps]` from `$CARGO_MANIFEST_DIR/Cargo.toml`, so
  they need `CARGO_MANIFEST_DIR` even though the file is inside the crate
  root. It declares 10 `SYSTEM_DEPS_<KEY>_*` knobs per library on top of 13
  `PKG_CONFIG_*` ones, and emits `cargo:rustc-cfg=system_deps_have_<key>` per
  library found, so a denied probe does show up in the directives.
- **Per-library knobs aren't always valid identifiers.** The pkg-config crate
  uppercases the library name and maps only `-` to `_`, so glib-2.0's knobs
  are `GLIB_2.0_STATIC` and friends, dot included. Patterns match them, but no
  shell can set them.
- **Most probes fail the build rather than degrade.** alsa-sys (ALSA is
  installed here), glib-sys, gobject-sys, libdbus-sys, magic-sys,
  libsystemd-sys and pcsc-sys panic or exit 1 when their probe fails, unlike
  the fallbacks above: denying the `pkg-config` grant, `PKG_CONFIG_PATH` or
  the directory it names stops the build instead of quietly changing a cfg.
  magic-sys's second chance, vcpkg, bails out before reading its own knobs on
  a non-MSVC target, so the fallback is unobservable here.
- **The package name to probe can itself be consumer config.**
  libsystemd-sys probes `libsystemd` unless `SYSTEMD_PKG_NAME` names another
  implementation; the fixture points it at elogind, as a musl system would.
  The pkg-config crate then derives its knobs from that value
  (`LIBELOGIND_STATIC`), so the request can't enumerate them without knowing
  the consumer's choice.
- **Knobs suffix the target triple as often as they prefix it.**
  libsystemd-sys reads its knobs through the build-env crate
  (`SYSTEMD_PKG_NAME_<target>`, then `HOST_SYSTEMD_PKG_NAME`, then the plain
  name) and msvc_spectre_libs appends the triple to `MSVC_SPECTRE_LIB_DIR`.
  Prefix patterns cover those, unlike openssl-sys's.
- **Feature-gated probes make a request the union of every target.** x11
  probes one library per enabled feature, 17 in all, so its request lists 51
  `<LIB>_*` names although the fixture enables no feature and the script
  probes nothing: the fixture's largest request belongs to the crate that does
  the least here.
- **`framework` link kinds can't cross targets.** accelerate-src prints
  `cargo:rustc-link-lib=framework=Accelerate` whatever the target, and rustc
  rejects a framework outside Apple, so it fails the build before any policy
  applies; it's left out. coremidi-sys gates the same directive on `TARGET`,
  so it's in.
- **A script that compiles a probe with bare `rustc` can't even be
  baselined.** libudev-sys unwraps `Command::new("rustc")`'s result, which
  under rustup is the proxy in `~/.cargo/bin`, outside the jail: the baseline
  panics too, so unlike doc-comment's silent loss the crate can't join the
  fixture at all. Its library is missing anyway: this host installs `udev.pc`
  but not `libudev.pc`.
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
  syn). Resolving the 184 crates of the category no fixture covers yet, for
  this host (one is yanked), leaves 15 whose build scripts all stay within
  the default grant: 12 ICU4X data crates, dashu-int, scuffle-workspace-hack
  and z3, which needs `pkg-config`. 137 of the others pull proc-macro2, 115
  libc, 85 serde_core, 72 thiserror and 64 semver. Beyond the crates above,
  the probe sits in anyhow (wai-bindgen-gen-rust-wasm), rustversion
  (gxhash), num-traits through autocfg (embassy-hal-internal,
  range-set-blaze) and libm (wasmtime-internal-core).
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
  key), nextest-workspace-hack and scuffle-workspace-hack run `fn main() {}`
  and request nothing; dashu-int's only declares a cfg its own test workflow
  sets through `RUSTFLAGS`.
- **One knob can span a whole crate family.** ICU4X publishes a data crate
  per component, each with the same ten-line build script reading
  `ICU4X_DATA_DIR`; the fixture now builds 13 of them, across the 1.x and 2.x
  generations. Requests are per package, so `requests.toml` repeats the
  entry 13 times, while the consumer grants the name once for all external
  crates. Nothing in the format factors a request over a family, the way a
  named grant factors a policy.

### More generators

The `codegen` fixture grew to 102 build scripts: table generators (snap,
selectors, terminfo, cgmath, gl, fst, ss58-registry, packed_struct),
generators reading their own data (oid-registry, gherkin, iso_currency, grok,
tls-parser, khronos_api, human_name, htmlize, hyphenation, precis-core,
precis-profiles, codepage-437, font-awesome-as-a-crate, cranelift-isle,
stop-words), generators reading their own Rust sources (iref, miniserde),
parser generators (basic-cookies and lalrpop, ntriple and rust-peg,
sqlite3-parser and a lemon it compiles from C), a template engine
(amq-protocol and handlebars), a linker script (defmt) and env-configured
constants (smoltcp, fastnum). Every cell behaves as expected, and beyond the
directives, every generated file in `OUT_DIR` matched the baseline byte for
byte, bar one that no two builds agree on (below).

- **Most generators open their data relative to the working dir,** the
  crate root, so they work under the default grant's crate-root read. The
  ones that join `CARGO_MANIFEST_DIR` instead (chrono-tz, web_atoms,
  tls-parser, human_name, hyphenation, miniserde, cpp_demangle,
  precis-core, precis-profiles) need the variable, and fail outright without
  it, since they all unwrap it.
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
- **Data compiled into the script escapes the policy.** pinyin reads its
  table with `include_str!` through `env!("CARGO_MANIFEST_DIR")`,
  ss58-registry its registry JSON, stop-words its `nltk` word lists (but not
  the ISO one, which it opens at build time), amq-protocol its handlebars
  template, and amq-protocol-codegen the AMQP spec from the build
  dependency's own source dir. Those reads happen when the script, or the
  library it calls, is compiled, so no request covers them and no denial
  reaches them. packed_struct reads its size features with `cfg!` rather
  than `CARGO_FEATURE_*`, for the same reason.
- **A generator can read its own module tree.** iref inlines `src/uri/mod.rs`
  into one file, following every `mod` declaration it finds, so the request
  has to name the directory rather than the files; miniserde copies
  `src/place.rs` with a module renamed after the patch version, and
  codepage-437 splices each dialect's `overlaps.rs` into its output.
- **`is_file` probes are invisible to the policy.** Landlock gates opening,
  not `stat`, so amq-protocol finds its README beside the manifest, or at the
  workspace root above it, with no read grant at all, and cpp_demangle sees
  that the `in/` and `tests/` dirs it generates tests from are missing — as
  they are in any published package, which is how a widely used crate ships a
  build script that does nothing. amq-protocol passes the README's path to
  rustc with `rustc-env`, and the crate `include_str!`s it outside the
  sandbox.
- **A script can run what it just compiled, and a read grant is what allows
  it.** sqlite3-parser builds lemon and mkkeywordhash from vendored C with
  cc-rs, then runs both from `OUT_DIR` to generate its parser and keyword
  hash. A `run` entry can't name them: crabshell resolves `run` before the
  script starts, when the files don't exist yet. The `read` grant for
  `OUT_DIR` carries Landlock's execute right, so it works, and nothing in the
  request records it.
- **Writing a file a tool opens read-write needs the dir readable.** With
  `OUT_DIR` write-only, `ld` can't create rlemon there and cc-rs's
  compiler-family probe can't create its temp file; both report `Permission
  denied` and sqlite3-parser fails. Like `/dev/null`, a dir written by
  programs the script runs has to be granted read as well.
- **Not every generator's output is reproducible.**
  font-awesome-as-a-crate writes its icon lookups in `HashMap` order, so its
  generated file differs between any two builds, the baseline's included.
  Directives still match, but `OUT_DIR` can't be compared byte for byte.
- **Network paths are feature-gated more often than patched out.**
  precis-core and precis-profiles download the UCD files only with their
  `networking` feature, and otherwise read the copy bundled under
  `resources/ucd`; woothee's `generate` feature clones its upstream
  repository into a temp dir and rewrites its own `src/dataset.rs` from the
  YAML found there, while the default build script does nothing. Each
  request records the network, woothee's `git` grant and its writes into its
  own sources; a request naming a grant the policy doesn't define just
  narrows away.
- **A consumer knob can redirect generated code anywhere.** amq-protocol
  writes its protocol module to `AMQ_PROTOCOL_CODEGEN_DIR` when that's set,
  which no variable names and no policy here can grant. Denied, it falls back
  to `OUT_DIR`, which is what an unconfigured build does.
- **Perfect-hash builders need nothing but their input.** htmlize runs
  phf_codegen over `entities.json` and human_name over four JSON files under
  `build/`; the hashes are computed in the script. hyphenation instead copies
  its bundled `dictionaries/` into `OUT_DIR` file by file, and rebuilds them
  from `patterns/` on a normalization feature, so its request names both.
- **A pure-Rust generator can still pull a C build.** amq-protocol's
  handlebars brings pest, pest_stack_guard, stacker and psm, whose build
  script assembles its stack-switching routines: the `cc` grant this fixture
  now needs is owed as much to a template engine as to the one crate that
  compiles a tool on purpose.
- **A tool vendored in a crate can only be granted by path.**
  protoc-bin-vendored ships protoc inside its crate, and perfetto_protos
  runs it through a path compiled into that dependency. A bare `run =
  ["protoc"]` is looked up on `PATH`, and no variable names a dependency's
  source dir, so the fixture's `protoc` grant globs
  `:cargo_home/registry/src/*/protoc-bin-vendored-linux-x86_64-*/bin/protoc`
  and the include dir next to it. Only the consumer can write that: it
  changes with `cargo vendor`, git sources and the host arch. The crate
  just requests `protoc`. According to the runner code, a bare name missing
  from `PATH` fails every build script it's granted to, even scripts that
  never run it, while a missing path is skipped.
- **`PROTOC` is consumer config that changes other scripts' behavior.**
  prost-build runs the protoc `PROTOC` names (cedar-policy, etcd-client,
  containerd-client, prost-validate-types, tracing-perfetto-sdk-schema), so
  the fixture sets it in `.cargo/config.toml`. When `PROTOC` is set,
  tink-proto regenerates its code into its own `src/`, which fails on
  read-only sources even in the baseline, so it's left out. prost-build 0.8's
  build script records `PROTOC` for later. If `PROTOC` is denied, it silently
  falls back to the protoc bundled in its own crate.
- **Checked-in code is gated on the crate's own files.** pprof regenerates
  only when its proto's hash changes, geozero only when its generated file
  is missing, and pbjson-types reads checked-in descriptors. Each request
  has to name those exact files. If geozero's file is denied, the crate
  tries to regenerate into its own sources.
- **Generators read their output back.** containerd-client,
  containerd-shim-protos, prost-wkt-types, prost-validate-types and
  perfetto_protos reread what they wrote to `OUT_DIR`. prost-build reads
  protoc's descriptors back from a temp file, so the `protoc` grant reads
  `:tmp_dir`, as `cc` does. Pure-Rust parsers (protox, protobuf-codegen's
  `pure()`) need nothing beyond the crate's own `.proto` files.
- **Shipping checked-in code is the category's norm.** Of the 12 crates the
  fixture gained last, 8 generate nothing in a default build: pprof2
  regenerates only when its proto's hash changes (like pprof), while
  prometheus, libdd-trace-protobuf, denokv_proto, tokio-cron-scheduler,
  protosol, goosefs-sdk and hdfs-native each need a feature, and goosefs-sdk
  the knob below as well. All of them generate into their own `src/` or
  `proto/`, so each request names a write no consumer grants — and the
  fixture can carry them precisely because the default path needs neither the
  write nor protoc.
- **Denying a regeneration knob is indistinguishable from not setting it.**
  goosefs-sdk declares `rerun-if-env-changed=GOOSEFS_SDK_REGEN_PROTO` and
  checks it for `1`; statsig-rust and bigtable_rs use `STATSIG_BUILD_PROTO`
  and `BUILD_BIGTABLE_RS_GOOGLE_PROTO` the same way. Unlike the silent
  degradation elsewhere, the denied path is the one the vendor ships, so it's
  the safe outcome; the knob is the consumer's own and a policy that grants
  it would also have to grant the write.
- **protobuf-codegen's default parser is the pure one.** prometheus's `gen`
  feature and yara-x-proto generate without protoc at all; a `protoc` request
  only appears where a crate calls `.protoc()` or goes through prost-build or
  tonic-prost-build.
- **A build script can point `PROTOC` at its own protoc.** hdfs-native,
  libdd-trace-protobuf, yellowstone-grpc-proto and pulsar (with
  `protobuf-src`) call `env::set_var("PROTOC", ...)` before prost-build runs,
  overriding whatever the consumer set. crabshell only filters the
  environment the script starts with, so the grant has to cover whichever
  binary the script then picks: protoc-bin-vendored's, inside `CARGO_HOME`,
  or the one protobuf-src builds from vendored C++ sources, which would need
  `cc` and `autotools` as well.
- **A request can name a program inside its own crate.** protosol looks for
  protoc at `PROTOC_EXECUTABLE`, `PROTOSOL_PROTOC` or `opt/bin/protoc` in its
  crate, which a bare name on `PATH` can't express, so its request names that
  path in `run`; hdfs-native requests `cp` for the tree it copies into
  `OUT_DIR`. Neither is granted here, so both narrow away: those are the two
  programs in the table above.
- **A `links` key can publish a schema directory.** protosol lists its proto
  dir and emits `cargo:PROTO_DIR=<that dir>`, so dependents find the schemas
  as `DEP_PROTOSOL_PROTO_DIR` (temporalio-protos publishes a descriptor set
  the same way). A dependent would have to request both that `DEP_*` name and
  a read of a path in `CARGO_HOME` that no variable names.
- **Codegen can depend on the build script's own profile.** chroma-types
  compiles one schema more when its build script was compiled with
  `debug_assertions`, which no grant or request affects; the crate notes that
  `#[cfg(test)]` wouldn't work there either.
- **Most of the category's remaining crates are too big to fixture.** Every
  scenario rebuilds the whole graph, so crates shipping megabytes of
  generated Rust (google-cloud-googleapis, gcloud-googleapis, the lance
  family, delta_kernel, temporalio-protos, statsig-rust) or pulling
  arrow/datafusion, cranelift (yara-x) or aws-lc-rs (bigtable_rs) stayed out.
  sp1-prover-types can't be compared at all: it emits a build timestamp, so
  no two builds print the same directives.
- **bindgen's needs land on its consumers.** With bindgen's default
  `runtime` feature, clang-sys's build script does nothing; each crate
  running bindgen loads libclang into its own build script, so libclang, its
  libraries, clang's resource dir and the system headers are read by the
  consumer, through a `bindgen` grant the fixture defines.
- **Finding libclang needs `LIBCLANG_PATH`.** Unset, clang-sys lists
  `/usr/lib*/*/*` and similar globs, which no grant short of reading all of
  `/usr` allows, and bindgen fails with "Unable to find libclang". With
  `LIBCLANG_PATH` naming libclang's directory, clang-sys only checks for
  `libclang.so` there, and the grant reads `:libclang_dir/libclang.so` and
  the resource dir `:libclang_dir/clang` through a variable imported from
  it. The tests set it from `clang -print-resource-dir`.
- **Most bindgen denials fail loudly.** Without libclang's libraries
  (libLLVM), the resource dir (`stdint.h`) or the system headers, bindgen
  panics. A `clang` that is found but can't run panics in clang-sys. Two
  degrade silently, with no effect on these crates: without `PATH`, clang
  isn't found and bindgen goes on without its default include paths; without
  the `rustc` grant, bindgen 0.72 and later can't run `$RUSTC --version` and
  target the newest Rust they know. Both could change the generated
  bindings, which the harness doesn't compare; comparing `OUT_DIR`'s `.rs`
  files by hand showed no difference in any cell, for every file the fixture
  generates. gbm-sys and drm-sys confirm the `rustc` part: on bindgen 0.69
  and 0.70 they never run it, and their bindings match the baseline without
  the grant.
- **rustfmt never runs.** bindgen formats its output with `rustfmt` from
  `PATH`, which is rustup's proxy in `~/.cargo/bin`, outside every jail; it
  reports the failure on stderr and writes unformatted bindings, in the
  baseline too. Granting it would mean granting rustup.
- **A `run` entry for a missing tool fails the build.** crabshell resolves
  each `run` name on `PATH` before starting the script, so a grant can't
  offer optional tools: `llvm-config`, which clang-sys tries first, isn't
  installed here and isn't granted, and the `bindgen` grant's `clang` makes
  hosts with libclang but no clang fail.
- **Headers bindgen can't find without help.** pq-sys (libpq-fe.h is in
  `/usr/include/postgresql` on Debian) and alsa-sys (`asoundlib.h`, under
  `alsa/`) need `BINDGEN_EXTRA_CLANG_ARGS` on some distros, input-sys's
  bundled headers need libudev's, and io-uring's generated bindings don't
  compile against this host's older kernel headers; none of them generates
  bindings here. pq-sys and input-sys joined the fixture with that path off,
  taking their pre-generated bindings instead.
- **Bundled headers cost nothing but a read of the crate.** gbm-sys ships
  gbm.h and parses it through a relative `-Iinclude`, so it needs no system
  library at build time at all, only `:manifest_dir/include` and the
  `bindgen` grant for libclang and clang's own headers.
- **An include dir the consumer chooses needs a consumer variable.** drm-sys
  parses the kernel's drm.h and drm_mode.h from `LIBDRM_INCLUDE_PATH`, or
  else from pkg-config's `libdrm`, which not every distro installs. Its
  request has to name a variable (`:libdrm_include_path`) the consumer
  defines under exactly that name, the same gap as libduckdb-sys's
  `DUCKDB_LIB_DIR`. Here it's belt and braces: those headers sit under
  `/usr/include`, which the `bindgen` grant reads anyway, and the fixture
  passes libdrm's own copy of them through `BINDGEN_EXTRA_CLANG_ARGS`, a
  knob that reaches the script through the `bindgen` grant's env.
- **Pre-generated bindings move the cost to the crate's own files and
  features.** pq-sys copies the file its target's pointer width selects, and
  input-sys checks the one its features select, so both request their own
  `src/` and the names behind that choice rather than libclang.
  input-sys's version comes from `CARGO_FEATURE_LIBINPUT_*`: with that left
  out of its request, narrowing drops it, and the script silently falls back
  to the oldest libinput it knows, exporting `LIBINPUT_VERSION_STR=1_9`
  instead of `1_21` and compiling different bindings. `CARGO_FEATURE_*` is in
  the default grant, so only a request can lose it.
- **A bindgen path that never runs here still costs a request.** libproc,
  netstat2, proc-maps, tun-rs and iceoryx2-pal-posix only bind on another
  target, and drm-fourcc only under a feature; their build scripts do
  nothing in this fixture, yet each is listed with what that path would use:
  writes into its own sources (proc-maps, drm-fourcc), a header parsed as
  C++ from a hardcoded Command Line Tools include dir no request can name
  without a host path (libproc, netstat2), `clang -E` and then `rustfmt`
  over the generated sources (drm-fourcc), and `DOCS_RS` (tun-rs). None of
  it is verifiable from here.
- **Fetching crates build offline through env vars.** In `network-fetch`,
  the consumer points libduckdb-sys, liteparse-pdfium-sys, ort-sys,
  libsodium-sys-stable, libbpf-sys and z3-sys at a local copy through
  `.cargo/config.toml`'s `[env]`, and the tight policy grants those names.
  Denied, each script takes its download path instead and fails, being
  offline (liteparse-pdfium-sys and ort-sys panic without `HOME` for their
  cache; libsodium-sys-stable builds its bundled source and fails without
  `sh` and `make`), or silently degrades: libbpf-sys and z3-sys drop the
  search path, and z3-sys reports a different version. Others skip the
  download on a feature (utoipa-swagger-ui's `vendored`,
  tracel-llvm-bundler's `xtask`), or fall back on failure: pci-ids to its
  bundled copy, intel-mkl-src to linking nothing at all.
- **A local copy the script reads needs a consumer variable.**
  libduckdb-sys copies the library from `DUCKDB_LIB_DIR` into `OUT_DIR`;
  denied, it only warns. The consumer defines `duckdb_lib_dir = { env =
  "DUCKDB_LIB_DIR" }`, but the crate's request must name that variable
  too, or it narrows the read away. There's no convention for such names,
  so a vendor can't write the request without knowing the consumer's
  config.
- **Some scripts write outside `OUT_DIR`.** pci-ids runs `curl` to rewrite
  `pciids/pci.ids` inside its own sources, in the registry. With a real
  local copy, liteparse-pdfium-sys copies `libpdfium.so` into
  `target/<profile>/deps`, and sherpa-onnx-sys's `shared` mode into
  `target/<profile>`; no variable names those dirs. sherpa-onnx-sys is left
  out: its `static` mode needs the archives to exist when rustc builds the
  rlib.
- **An optional tool can't be requested.** z3-sys runs `z3 --version` when
  present, but crabshell rejects a `run` entry that isn't on `PATH`, so a
  request for it would break builds on hosts without Z3.

### More version probes

The `rustc-probe` fixture grew to 87 build scripts: crates running `$RUSTC`
themselves (const_fn, branches, allocative, hex_lit, maybe-uninit, no-panic,
stabby-macros), through rustc_version (atomic-traits, field-offset,
flatbuffers, float16, crc32c, rstest_macros, stabby-abi), version_check
(circular-queue, error-chain, im-rc, multer, proc-macro2-diagnostics) or
autocfg (bigdecimal, erasable, slice-dst, substring), compile probes that link
(s2n-quic-platform), and more rustversion users (slog, decorum,
wasm-bindgen). Every cell behaves as expected.

- **A compile probe that links needs the whole C toolchain.**
  s2n-quic-platform compiles each file in its own `features` dir with
  `--crate-type bin --out-dir $OUT_DIR`, so rustc also links, and each probe
  resolves a syscall through the linker. Without the `cc` grant rustc reports
  "linker `cc` not found" and the crate silently loses seven cfgs
  (`s2n_quic_platform_cmsg`, `socket_msg`, `socket_mmsg`, `gso`, `gro`,
  `pktinfo`, `tos`). The fixture's policies grant `cc` for it alone; probes
  that stop at `--emit=metadata` (fixed) or at `--version` need only `rustc`.
- **A probe can need the features of the crate it probes for.**
  s2n-quic-platform scans `env::vars()` for `CARGO_FEATURE_*` and passes each
  on as `--cfg feature="..."`, which its probes' `extern` blocks are gated on.
  Enumerating the environment means the request has to name the pattern, not
  the variables; without it the probes fail to compile, and the loss is as
  silent as the missing linker.
- **A degraded generated file fails at the consumer, not the script.**
  stabby-macros parses `release:`, `commit-hash:` and `LLVM version:` out of
  `$RUSTC -vV` and writes them, with `OPT_LEVEL`, `DEBUG`, `NUM_JOBS`,
  `TARGET` and `HOST`, into `OUT_DIR/env_vars.rs`. Denying those names only
  omits constants, so the build script succeeds and the crate fails to compile
  with five unresolved names.
- **The `rustc` grant's variable matters more than its program.** fixed 1.31
  no longer compiles the probes it has the code for, but still `expect`s
  `RUSTC`, so it panics without the grant although it never runs rustc. The
  crates that unwrap rustc_version (atomic-traits, field-offset, flatbuffers,
  float16, crc32c, rstest_macros, stabby-abi) and those that assert on
  `--version` output (allocative, hex_lit) also fail outright, while
  version_check's users (error-chain, im-rc, multer, circular-queue,
  proc-macro2-diagnostics) get `None` and emit nothing.
- **A silent fallback to the MSRV.** const_fn warns "unable to determine
  rustc version", emits no cfg at all, and the crate then assumes its MSRV
  (1.31): the version it writes to `OUT_DIR` is `include!`d only behind the
  cfg the script would have emitted, so nothing fails. maybe-uninit and
  branches do the same without even a warning (branches falls back to
  `branches_stable`).
- **The profile is a cfg too.** error-chain turns `PROFILE` into
  `cfg(build="debug")`, which isn't in the default grant: denying it drops the
  cfg silently. branches reads `OPT_LEVEL` the same way, but only emits
  `branches_cold_weights` at `2` or `3`, so the fixture's debug build can't
  show the difference.
- **autocfg's `OUT_DIR` read keeps costing cfgs.** Without it bigdecimal
  silently drops `has_unsigned_abs`, as num-traits drops `has_total_cmp`.
  Version-only users (substring) notice nothing on this host.
- **More probes run when the build script compiles.** slog and wasm-bindgen
  use rustversion's `cfg!` (wasm-bindgen renames the crate to
  `rustversion_compat`), decorum its attribute form
  (`#[rustversion::nightly] fn main`), so the most downloaded crate of the
  category needs no `rustc` grant: its request is three `CARGO_CFG_TARGET_*`
  names.
- **Running `rustc` from `PATH` recurs.** bnum does what doc-comment does,
  and its nightly check is equally invisible on a stable host: both the
  baseline and every sandboxed build see no `nightly` in the output.
- **A crate can insist on its own name.** im-rc matches `CARGO_PKG_NAME`
  against `im`/`im-rc` and panics on anything else, so it fails without the
  variable, as ring does.
- **`RUSTC_WORKSPACE_WRAPPER` is deliberately not used.** const_fn, branches
  and rustc_version run `RUSTC_WRAPPER` but skip the workspace wrapper, since
  clippy's driver answers `-vV` differently; the built-in `rustc` grant passes
  both.

### More build metadata
`vcs-metadata` grew to 99 build scripts. Two members now record the commit
without running `git` themselves: `with-vergen-gix`, where vergen-gix reads
the repository in process with gix, and `with-git-version`, where a
proc-macro runs `git` while rustc expands it. The registry crates added embed
the compiler version (aws-types, sentry-contexts, stabby-macros, hotpath), the
commit (cross, solana-version, regorus, and stellar-xdr and soroban-env-common
through crate-git-revision), their own version (prettier-please,
verus_prettyplease, zed_extension_api), or a listing of their own files
(i-slint-compiler, cross). Every cell behaves as expected.
- **Reading the repository in process costs the same reads, minus the
  program.** gix walks up from the crate looking for a `.git`, so the request
  has to name the repository exactly as the `git` grant does; the fixture's
  `git-repo` grant is that grant without `run`. Requesting `git` instead
  would ask for a program the script never spawns. Granted nothing, gix
  reports "Could not find a git repository" and vergen sets no
  `VERGEN_GIT_*`; granted only the git dir, its dirty check fails with "IO
  error while writing blob or reading file metadata" and vergen again drops
  every `VERGEN_GIT_*`, not just the dirty flag as the git CLI does. Both
  fail the crate's `env!`, so a library reading the repository itself
  degrades louder, and all-or-nothing, where `git` degrades quietly.
- **A proc-macro is outside the policy.** `with-git-version` embeds the
  commit through git-version's `git_version!`, which runs `git` in the crate's
  dir while rustc expands it. crabshell only wraps build scripts, so every
  cell, including the bare default grant, gets the real commit; the member
  asserts at compile time that it did. The directive comparison can't see it
  either, since a proc-macro prints no directives. Any crate can embed the
  commit this way.
- **`.cargo_vcs_info.json` makes a published crate deterministic.**
  crate-git-revision (stellar-xdr, soroban-env-common, stellar-strkey) reads
  the commit Cargo records at publish time and only falls back to git, so
  denial changes nothing for a registry build. Its 0.0.9 dirty check runs
  `git status --porcelain --untracked-files=no`, which in a checkout would
  need the work tree like every other dirty check.
- **Scripts that print nothing can only be checked by their output files.**
  cross writes `commit-info.txt` and `docker-images.rs` into `OUT_DIR` and
  prints no directives at all; aws-types, sentry-contexts and stabby-macros
  generate a source file with the compiler version and print nothing either.
  Comparing every script's `OUT_DIR` against the baseline is the only check
  that covers them: in all four working cells, every file matched byte for
  byte except mach-sys's nonce and the `.rmeta` of eyre's compile probe,
  which rustc writes into the per-cell `OUT_DIR` and which differs between
  any two builds.
- **A listing of its own crate is a read like any other.** cross lists its
  `docker/` dir for the image names and i-slint-compiler walks `widgets/` for
  the styles it embeds, both through `CARGO_MANIFEST_DIR`, so both fail under
  the default grant. Listing is also all they need: the generated code reaches
  the files with `include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), ...))`,
  which rustc reads, outside the sandbox.
- **A write into `.git` can be requested but never granted.** regorus copies
  its own `scripts/pre-commit` and `pre-push` into `.git/hooks` whenever it
  finds a real `.git` next to the crate — a build script installing hooks in
  the consumer's repository. From the registry there is no `.git`, so the copy
  never happens; the request records the write, and no policy here grants one
  outside `OUT_DIR`.
- **Half the metadata crates need no grant at all.** prettier-please pastes
  its version in with `concat!("cargo:VERSION=", env!(...))`, read when the
  script compiles, so its request is empty; verus_prettyplease reads the same
  value at run time and requests one name. zed_extension_api turns
  `CARGO_PKG_VERSION` into bytes, vstd defaults `RUST_MIN_STACK`, and hotpath
  recovers a custom profile's name from `OUT_DIR`'s own path, which the floor
  always provides.
- **Non-determinism has to be kept out of the fixture.** vergen's and
  built's `si`/sysinfo features report the host's name, CPU and memory, and
  mach-sys hashes the environment and the time into a nonce; the members take
  neither feature and nothing prints the nonce. `SOURCE_DATE_EPOCH` fixes
  every build timestamp, and gix's commit dates come from the fixture's
  commit, which `Fixture::init_repo` pins.
- **A registry crate that runs git watches files it will never see.**
  solana-version prints `rerun-if-changed` for `HEAD`, `refs`, `packed-refs`
  and `reftable` under both `--git-dir` and `--git-common-dir`, so in a
  checkout its directives name paths outside the crate. In `CARGO_HOME` git
  finds no repository, so, as for wasm-bindgen-shared and typst-utils,
  denying it makes no difference.
### More downloads
The `network-fetch` fixture grew to 67 build scripts: a local copy named only
for rustc (sherpa-onnx-sys), an offline path that still generates bindings
(xgboost_lib-sys), features that switch the download off (oboe-sys,
postgresql_embedded) and five MeCab dictionary crates (lindera-ipadic,
-ipadic-neologd, -ko-dic, -unidic, -cc-cedict) that do nothing without theirs.
Every cell behaves as expected. Most of the category's remaining crates can't
join; the bullets below say why.
- **A script's own shared libraries load before any policy applies.**
  sherpa-onnx-sys's build script links `libbz2.so.1`, because its build
  dependency bzip2-sys found the system copy through pkg-config, and
  tracel-llvm-bundler's links `libm.so.6`. The loader opens both before the
  script starts, so it dies with `error while loading shared libraries` and
  no `cargo:warning`. No request can name them: the crate doesn't know the
  host's paths, and the library is chosen by a build dependency. `libm.so.6`
  joined implicit access, next to libc and libgcc_s; for libbz2 the fixture
  sets `BZIP2_NO_PKG_CONFIG`, so bzip2-sys compiles its vendored copy. Both
  scripts load fine *without* requests, since the `cc` grant's library dirs
  cover them, so precise requests are what exposes this.
- **A checksum pins the artifact, so no stand-in works.** libduckdb-sys
  accepts an empty `libduckdb.so` and sherpa-onnx-sys empty archives, but
  risc0-circuit-recursion SHA-256-checks `recursion_zkr.zip` before copying
  it, and v8 checks its archive. Their switches for a local copy
  (`RECURSION_SRC_PATH`, `RUSTY_V8_ARCHIVE`) only move the download, so
  neither builds offline. risc0 dropped the download in 4.1 instead: the
  archive now ships in the crate, and the build script that fetched it is
  gone.
- **Shared download dirs sit above `OUT_DIR`, which nothing names.** v8 takes
  a lock file in `target/<profile>/build` and extracts into
  `target/<profile>/gn_out`, wasmer caches wee8 four levels above `OUT_DIR`,
  and sherpa-onnx-sys's `shared` mode copies the libraries into
  `target/<profile>`. `:out_dir` is the deepest path a request or a policy can
  name, so those paths are denied in every cell, even with a warm cache. v8,
  v8-goose and wasmer are left out for that reason.
- **`DOCS_RS` is the only offline switch for some crates, and it is global.**
  hwlocality-sys, lbug, oboe-sys, sherpa-onnx-sys and v8 skip their native
  setup when it's set. Cargo's `[env]` applies to the whole workspace, so a
  fixture can only set it for every crate at once, which would turn off the
  local-copy paths this fixture exists to test. A crate whose only switch is
  `DOCS_RS` needs a fixture of its own.
- **A download cache under `CARGO_HOME` is nameable; one under `HOME` isn't.**
  xgboost_lib-sys keeps its verified copy in
  `$CARGO_HOME/xgboost-prebuilt/<tag>`, so its request writes
  `:cargo_home/xgboost-prebuilt`. liteparse-pdfium-sys, ort-sys,
  intel-mkl-src, lbug and postgresql_embedded use dirs under `HOME`, which no
  variable names, so their requests can only record `HOME`.
- **Naming a library for rustc needs no grant at all.** sherpa-onnx-sys only
  checks that `SHERPA_ONNX_LIB_DIR` is a directory and prints it as
  `rustc-link-search`; rustc then bundles the `static` archives into the rlib,
  outside the sandbox. So the archives must exist when rustc runs, while the
  build script never reads them. Only crates that copy the artifact
  themselves, like libduckdb-sys, need the directory readable.
- **A crate whose feature is off is still a crate with a build script.**
  oboe-sys (`test`), postgresql_embedded (without `bundled`) and the five
  lindera dictionaries print no directives at all: their scripts read a few
  knobs and return. They cost one request each and nothing can degrade, which
  is what a fetching crate looks like once the consumer opts out of the fetch.
  lindera-dictionary declares `LINDERA_CTX_FREQ_FILE` without reading it, one
  more `rerun-if-env-changed` that overstates.
- **The same host-specific grant is needed twice now.** xgboost_lib-sys
  generates bindings even on its offline path, so this fixture needs the
  `bindgen` grant the `bindgen` fixture defines, copied verbatim: a policy can
  only `include` files inside its own fixture, since the jail reads nothing
  else. Two fixtures needing the same distro-specific list of libraries and
  headers argues for a built-in `bindgen` grant.
- **A denied switch fails loudly, which the matrix can't show.**
  `network_fetch_denied_switch` builds the fixture under the tight policy
  minus liteparse-pdfium-sys's `PDFIUM_*`: the script takes its download path
  and fails, and the build fails with it. Every working cell of the matrix
  grants the switch, and the default column fails earlier on a rustc probe,
  so observing this needed a scenario of its own. Its first run showed that
  round 1's request missed `CARGO_MANIFEST_DIR`, which the fallback path
  unwraps: a request is only checked on the paths the matrix runs, so knobs
  read after a switch go unverified. The silent half (libbpf-sys and z3-sys
  dropping a search path) still doesn't show up in directives.
### The unlabelled scripts
The report left 54 build scripts without a label. Reading all of them put 36
in an existing category: mostly `cfg-only` (23) and `codegen` (11), plus
rmcp, which configures git's hooks path (`vcs-metadata`), and
dylint_linting, which checks the toolchain file (`rustc-probe`). The other
18 need a privilege no category grants: 15 write into their own sources or
the target dir, two install hooks in the consumer's repository, and varisat
probes for tools that may not exist. Of the 36, 18 joined the fixtures:
README and data generators in `codegen` (rust_decimal, readme-rustdocifier,
reed-solomon-simd, ply-rs, irg-kvariants, jrsonnet-evaluator, phonenumber,
riptoken), scripts that read nothing in `cfg-only` (rio_api, smallbox), and
scripts that only read Cargo's environment but whose build still needs the
`rustc` grant in `rustc-probe` (procfs, socketcan, encoding_c,
encoding_c_mem, num-bigint-dig, test-fuzz-internal, mini-moka,
sha1collisiondetection).
- **A README is a build-script input as often as a data table.** Seven of the
  54 rewrite the crate's README into `OUT_DIR` for rustdoc to include
  (rust_decimal, readme-rustdocifier, reed-solomon-simd, foyer,
  rusqlite_migration, and ply-rs and mini-moka through skeptic), and each
  finds it differently: relative to the working dir, through
  `CARGO_MANIFEST_DIR` (skeptic), through `CARGO_MANIFEST_DIR` with a
  fallback to the parent directory for a crate in a workspace (foyer), or
  through `CARGO_PKG_README`, which holds an absolute path
  (rusqlite_migration). Each has to request the file: an empty request
  narrows the crate root away, and readme-rustdocifier then fails with a bare
  `PermissionDenied`.
- **Package metadata is what these generators need granted.**
  readme-rustdocifier rewrites the README's `crate` links from
  `CARGO_PKG_NAME` and `CARGO_PKG_VERSION`, and so does its consumer
  reed-solomon-simd through it; both fail outright without them, as
  rusqlite_migration does without `CARGO_PKG_README`. `CARGO_PKG_*` is
  requested explicitly today; see the open question on the default policy.
- **Some scripts only validate the build and stop it.** procfs prints to
  stderr and exits 1 on an unsupported target OS, socketcan returns an
  error, smallbox panics if a fat pointer isn't laid out as it expects, and
  rustc_apfloat panics unless `CARGO_PKG_VERSION` ends in the `+llvm-<hash>`
  its own `src/lib.rs` documents. Failing loudly is the useful direction,
  but it ties the default grant to what they check: `CARGO_CFG_TARGET_OS` is
  in it, `CARGO_PKG_VERSION` isn't, so rustc_apfloat can't join the
  `cfg-only` fixture, whose point is that the default grant suffices.
- **What the script reads when it's compiled is invisible.**
  test-fuzz-internal, riptoken, sha1collisiondetection and safer-ffi gate
  their whole body on `#[cfg(feature = ...)]`; rustc_apfloat and kzg-rs
  `include_str!` their data; jrsonnet-evaluator links the Jsonnet standard
  library in as a build-dependency constant. Their requests are empty or
  near-empty even where the script's purpose is to read a file.
- **The shape no category covers is writing into the crate's own sources.**
  Fifteen of the 54 do it. Every `tauri-plugin-*` crate (fs, log, http, cli,
  oauth, autostart, global-shortcut, deep-link, devtools) runs
  `tauri_plugin::Builder`, which creates `permissions/autogenerated/`,
  writes a TOML file per command, a JSON schema and a reference page into
  it, and deletes them again when the plugin has no commands (devtools). On
  mobile targets it also writes into the *app's* project, through variables
  the Tauri CLI sets: deep-link rewrites `AndroidManifest.xml`, `Info.plist`
  and the entitlements under `TAURI_IOS_PROJECT_PATH`. It already knows
  about read-only sources: it skips the mobile part under `DOCS_RS`, whose
  crate sources are mounted read-only, but not the permissions dir, and it
  fails outright without `CARGO_MANIFEST_LINKS`, like ring. The four
  `dioxus-*` crates call `lazy_js_bundle`, which hashes its `src/ts/*.ts`
  against `src/js/hash.txt` and, when they differ, runs `bun build` to
  rewrite `src/js/*.js`. extism and llguidance run cbindgen into the crate
  root, llguidance also into `target/<profile>`. A category for this needs
  `write = [":manifest_dir/<subdir>"]`, and the fixtures can't host any of
  it: the jail mounts `CARGO_HOME` read-only, so the baseline fails first.
- **Two crates write into the consumer's repository.** cargo-husky walks up
  from `OUT_DIR` for a `.git` directory and writes `pre-push`, `pre-commit`
  and `post-merge` into its `hooks/`; rusty-hook runs `git rev-parse
  --show-toplevel` through `sh` and writes the hooks and a
  `.rusty-hook.toml` there. Both are normally dev-dependencies, but their
  scripts run like any other. Denial is silent in both: cargo-husky warns on
  stderr and succeeds, rusty-hook prints its error and exits 0, and neither
  prints a `cargo:` directive, so no baseline comparison can see it. The
  grant would be write access to an enclosing repository no built-in
  variable names, and `cargo-husky`'s own escape hatch
  (`CARGO_HUSKY_DONT_INSTALL_HOOKS`) is the only portable way to say no.
- **A hash of checked-in output decides whether a tool runs.**
  `lazy_js_bundle` only runs `bun` when `src/js/hash.txt` doesn't match the
  TypeScript sources, so in a registry build it runs nothing. The comparison
  fails open: the read error reads as "no hash", so denying it would run
  `bun`, which, not being installed, panics. The complete wish list
  therefore has to include a tool that normally never runs, and the request
  can't say "only if".
- **Optional tool probes recur.** varisat runs `drat-trim` and `rate` to
  decide whether to enable two test cfgs, reporting a missing one as a
  `cargo:warning` that quotes the OS error, so the denial would be visible
  in the comparison but as a different error string. Like z3-sys, the tool
  can't be requested where it isn't installed; unlike z3-sys, the crate
  offers `VARISAT_HAVE_DRAT_TRIM`/`VARISAT_HAVE_RATE` to skip the probe,
  which is what a request-aware crate should do.
- **A few scripts are only no-ops where they're consumed.** rmcp points
  git's `core.hooksPath` at its own `.githooks`, but only when both that
  directory and `.git` exist, which is never true in the registry;
  diesel_derives generates macro documentation only under its
  `diesel_docsrs` cfg; neptune generates CUDA and OpenCL sources only with
  those features; dylint_linting installs rustup components only on docs.rs.
  The request still has to cover the live path, so these crates would
  request tools no ordinary build runs.
## Methodology TODOs

- **A control variant for crabshell's own effect.** A permissive policy
  (`read = ["/"]`, `env = ["*"]`), run inside the jail and compared against the
  baseline, would show that crabshell's mechanism alone (env re-export,
  private `TMPDIR`, nested Landlock) doesn't change behavior. Today that's
  assumed. Worth adding once the policy contract settles.
