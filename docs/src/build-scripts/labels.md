# Observed labels

## Source

[Crates.io Build Script Report](https://ekse.github.io/crates_build_scripts/report.html)
(accessed 2026-10-24).

- Covers the top 10,000 crates by downloads; **1,190** ship a build script.
- Each build script was summarized and labelled by an LLM (DeepSeek Flash
  4.1). Labels are not hand-verified.
- 24 labels. A script can carry several: 667 have one label, 469 have two or
  more, and 54 have none.

## Labels mapped to categories

Column legend:

- **Exec**: programs the script spawns. **Read**: paths beyond its own
  package source. **Env**: notable env vars beyond Cargo's standard set.
- **Net**: needs network access.
- **User input**: whether a good policy depends on machine- or
  consumer-specific values a crate author can't know in advance.

| Label | Crates | Category | Exec | Read | Env | Net | User input |
|---|---:|---|---|---|---|---|---|
| `check_cfg_declaration` | 132 | `cfg-only` | — | — | — | no | no |
| `cfg_alias_platform_emission` | 96 | `cfg-only` | — | — | `CARGO_CFG_*`, `TARGET` | no | no |
| `docsrs_skip` | 57 | `cfg-only` | — | — | `DOCS_RS` | no | no |
| `rerun_directives_noop` | 31 | `cfg-only` | — | — | — | no | no |
| `feature_combination_validation` | 27 | `cfg-only` | — | — | `CARGO_FEATURE_*` | no | no |
| `target_triple_export` | 19 | `cfg-only` | — | — | `TARGET`, `HOST` | no | no |
| `sanitizer_profiler_detection` | 14 | `cfg-only` | — | — | `CARGO_CFG_SANITIZE` | no | no |
| `wasm_target_configuration` | 39 | `cfg-only` | — | — | `CARGO_CFG_*` | no | no |
| `simd_cpu_feature_detection` | 24 | `cfg-only` | (some also compile C/asm) | — | `CARGO_CFG_TARGET_FEATURE` | no | no |
| `dependency_version_cfg` | 41 | `cfg-only` | (some run `cpp`) | — | `DEP_*` | no | no |
| `environment_config_constants` | 37 | `cfg-only` + declared env | — | sometimes a user data dir | crate-specific (`ICU4X_DATA_DIR`) | no | **yes**: which vars, which dirs |
| `generated_rust_source` | 174 | `codegen` | — | own source only | `CARGO_PKG_*` | no | no |
| `parser_generator` | 16 | `codegen` | usually none (lalrpop runs in-process) | own grammars | — | no | no |
| `linker_script_link_args` | 25 | `codegen` | — | own source only | — | no | no; but emits `rustc-link-arg` |
| `rustc_version_detection` | 173 | `rustc-probe` | `$RUSTC` (+ wrappers) | sysroot | `RUSTC`, `RUSTC_WRAPPER` | no | no |
| `compiler_feature_detection` | 55 | `rustc-probe` | `$RUSTC` | sysroot, `OUT_DIR` scratch | `RUSTC`, `CARGO_ENCODED_RUSTFLAGS` | no | no |
| `build_metadata_embedding` | 62 | `vcs-metadata` | `git` (some) | workspace `.git` | `CARGO_PKG_*`, `PROFILE` | no | no; useful mainly for `[local]` |
| `protobuf_grpc_codegen` | 53 | `protoc` | `protoc` (prost), or none (protox) | own `.proto`, protoc includes | `PROTOC`, `PROTOC_INCLUDE` | no | sometimes: protoc location |
| `system_library_linking` | 240 | `system-lib` | `pkg-config`, vcpkg | pkg-config dirs, system lib dirs | `PKG_CONFIG_*`, `<LIB>_DIR`-style | no | **yes**: library locations vary per machine |
| `bindgen_ffi` | 99 | `bindgen` | loads libclang (dlopen, not exec) | system/vendored headers, libclang | `LIBCLANG_PATH`, `BINDGEN_EXTRA_CLANG_ARGS` | no | sometimes: header/libclang location |
| `language_binding_config` | 25 | `lang-runtime` | Python/Ruby/Node interpreter | interpreter install | `PYO3_PYTHON`, `PYO3_CONFIG_FILE` | no | **yes**: which interpreter |
| `native_c_cpp_compilation` | 280 | `cc-toolchain` | `cc`, `c++`, `ar`, `cmake`, `make`, `configure` | toolchain + sysroot headers/libs | `CC`, `CFLAGS`, `TARGET_*`, `CARGO_MAKEFLAGS` | no | sometimes: cross toolchains |
| `assembly_compilation` | 21 | `cc-toolchain` | `nasm`, `perl` | toolchain | `CC`, `NASM` | no | no |
| `prebuilt_binary_download` | 34 | `network-fetch` | `curl`/in-process HTTP, `tar` | cache dirs outside `OUT_DIR` | crate-specific (e.g. v8's `RUSTY_V8_ARCHIVE`) | **yes** | **yes**: URLs, checksums, or an offline artifact |
