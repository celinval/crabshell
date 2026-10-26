# Proposed categories

Ordered by privilege.

| Category | Grants beyond the floor | Main labels |
|---|---|---|
| `cfg-only` | Cargo-defined env vars (`CARGO_CFG_*`, `CARGO_FEATURE_*`, `CARGO_PKG_*`, `TARGET`, `HOST`, `PROFILE`, `DEP_*`, `DOCS_RS`); read own package source | cfg/alias/check-cfg, docs.rs, feature validation |
| `codegen` | same as `cfg-only`; output stays in `OUT_DIR` | generated sources, parser generators, linker scripts |
| `rustc-probe` | exec `$RUSTC` and wrappers; read sysroot; scratch in `OUT_DIR` | rustc version, compile probes (`autocfg`) |
| `vcs-metadata` | exec `git`; read the workspace `.git` | build metadata |
| `protoc` | exec `protoc`; read its include dir | protobuf/gRPC |
| `system-lib` | exec `pkg-config`; read pkg-config and system lib dirs | system library linking |
| `bindgen` | load libclang; read header roots | bindgen FFI |
| `cc-toolchain` | exec C/C++/asm toolchain, `cmake`, `make`; read toolchain roots; jobserver | native C/C++/asm builds |
| `lang-runtime` | exec a named interpreter; read its install | pyo3, rb-sys, napi |
| `network-fetch` | network; a cache dir outside `OUT_DIR` | prebuilt downloads |

Coverage if labels are mapped to tiers (each crate takes the tier of its
highest-privilege label; the 54 unlabelled scripts are excluded):

| Highest tier needed | Crates | Cumulative |
|---|---:|---:|
| 0: `cfg-only` / `codegen` | 364 | 32% |
| 1: + `rustc-probe` | 178 | 48% |
| 2: + host tools (`vcs-metadata`, `protoc`, `system-lib`, `bindgen`, `lang-runtime`) | 291 | 73% |
| 3: + `cc-toolchain` | 269 | 97% |
| 4: + `network-fetch` | 34 | 100% |

The tier assignment is coarse. It counts what a crate *might* need under
some feature set, not what one build does.

