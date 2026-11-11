# Crate coverage

Every build script the [fixtures](experiments.md) run, with what its precise
request (`requests.toml`) asks for. A crate in several fixtures shows the union
of its requests.

- **Categories** are every category the crate's build spans: those of all
  its labels in the [build script report](labels.md#source), and those its
  request implies (a `cc` grant for `cc-toolchain`, network for
  `network-fetch`, and so on). A build often spans several, e.g. a `-sys`
  crate that probes pkg-config and falls back to compiling its bundled C.
- **Env** counts the variables requested. Any request narrows the default
  grant's env (`TARGET`, `HOST`, `CARGO_CFG_*`, `CARGO_FEATURE_*`), so the
  count includes those the script reads.
- **Read**, **Write** and **Run** list requested paths and programs. Most
  scripts read nothing beyond their own crate root, which the default grant
  covers; `:out_dir` means reading back what the script wrote.
- **Grants** are the named grants requested.

226 crates, 212 of them among the report's 1,190 with build scripts. 145
request env, 92 reads, 4 writes, 4 programs directly and
12 network. Grants requested:
`rustc` (57), `cc` (31), `pkg-config` (17), `protoc` (11), `bindgen` (7), `cxx` (7), `git` (6), `cmake` (4), `autotools` (4), `python` (1), `awk` (1), `grep` (1), `sh` (1).

| Crate | Categories | Fixtures | Env | Read | Write | Run | Network | Grants |
|---|---|---|---:|---|---|---|---|---|
| ahash | rustc-probe | lang-runtime, rustc-probe | 1 | – | – | – | – | `rustc` |
| anyhow | rustc-probe | lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 3 | `:manifest_dir/src/nightly.rs`, `:out_dir` | – | – | – | `rustc` |
| async-io | rustc-probe | rustc-probe | 1 | `:out_dir` | – | – | – | `rustc` |
| atomic-polyfill | cfg-only | cfg-only | 1 | – | – | – | – | – |
| basic-cookies | codegen | codegen | 2 | `:manifest_dir` | – | – | – | – |
| bindgen | cfg-only | bindgen | 1 | – | – | – | – | – |
| blake3 | cfg-only, cc-toolchain | cc-toolchain | 13 | `:manifest_dir/c`, `:out_dir` | – | – | – | `cc` |
| blas-src | cfg-only | cfg-only | 6 | – | – | – | – | – |
| blst | cfg-only, cc-toolchain | cc-toolchain | 10 | `:manifest_dir/blst`, `:out_dir` | – | – | – | `cc` |
| borsh | cfg-only | protoc | 2 | – | – | – | – | – |
| bugreport | cfg-only, vcs-metadata | vcs-metadata | 9 | – | – | – | – | – |
| camino | rustc-probe | lang-runtime, rustc-probe | – | – | – | – | – | `rustc` |
| cedar-policy | protoc | protoc | – | `:manifest_dir/protobuf_schema` | – | – | – | `protoc` |
| cedar-policy-core | codegen | protoc | 2 | `:manifest_dir/src/parser`, `:manifest_dir/src/validator/cedar_schema` | – | – | – | – |
| cgmath | codegen | codegen | – | – | – | – | – | – |
| chrono-tz | codegen | codegen | 1 | `:manifest_dir/tz` | – | – | – | – |
| clang-sys | system-lib | bindgen, system-lib | – | – | – | – | – | – |
| constant_time_eq | cfg-only, rustc-probe | network-fetch, rustc-probe | 1 | – | – | – | – | – |
| containerd-client | protoc | protoc | – | `:manifest_dir/vendor`, `:out_dir` | – | – | – | `protoc` |
| containerd-shim-protos | protoc | protoc | – | `:manifest_dir/vendor`, `:out_dir` | – | – | – | – |
| cookie | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| cortex-m-semihosting | cfg-only | cfg-only | 1 | – | – | – | – | – |
| crc32fast | cfg-only, rustc-probe | lang-runtime, network-fetch, rustc-probe | – | – | – | – | – | `rustc` |
| crossbeam-deque | cfg-only | cfg-only, protoc, vcs-metadata | 2 | – | – | – | – | – |
| crossbeam-epoch | cfg-only | cfg-only, protoc, vcs-metadata | 2 | – | – | – | – | – |
| crossbeam-utils | cfg-only | cfg-only, lang-runtime, protoc, rustc-probe, vcs-metadata | 2 | – | – | – | – | – |
| crunchy | codegen | cfg-only, codegen | – | – | – | – | – | – |
| cryptoki-sys | rustc-probe, bindgen | bindgen | 1 | `:manifest_dir/vendor` | – | – | – | `bindgen`, `rustc` |
| current_platform | cfg-only | cfg-only | 2 | – | – | – | – | – |
| cxx | rustc-probe, cc-toolchain | cc-toolchain | 3 | `:manifest_dir/include`, `:manifest_dir/src/cxx.cc`, `:out_dir` | – | – | – | `cc`, `cxx`, `rustc` |
| decancer | cfg-only | cfg-only | – | – | – | – | – | – |
| defmt | cfg-only, codegen | codegen | 1 | `:manifest_dir/defmt.x.in` | – | – | – | – |
| defmt-macros | cfg-only | codegen | – | – | – | – | – | – |
| doc-comment | rustc-probe | rustc-probe | – | – | – | – | – | – |
| embassy-time-driver | cfg-only | cfg-only | – | – | – | – | – | – |
| encoding_rs | rustc-probe | rustc-probe | – | – | – | – | – | – |
| erased-serde | rustc-probe | codegen, protoc, rustc-probe | – | – | – | – | – | `rustc` |
| esaxx-rs | cc-toolchain | cc-toolchain | – | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc`, `cxx` |
| etcd-client | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| exacl | rustc-probe, system-lib, bindgen | bindgen | – | `:manifest_dir/bindgen/wrapper.h` | – | – | – | `bindgen`, `rustc` |
| eyre | cfg-only, rustc-probe | rustc-probe | 2 | `:out_dir` | – | – | – | `rustc` |
| fastnum | cfg-only, codegen | codegen | 5 | – | – | – | – | – |
| findshlibs | cfg-only | protoc | 1 | – | – | – | – | – |
| foundationdb-sys | rustc-probe, system-lib, bindgen | bindgen | 1 | `:manifest_dir/include/730` | – | – | – | `bindgen`, `rustc` |
| fs-err | rustc-probe | lang-runtime, rustc-probe | 2 | `:out_dir` | – | – | – | `rustc` |
| fts-sys | rustc-probe, bindgen | bindgen | 5 | `:manifest_dir/src/fts-sys.h` | – | – | – | `bindgen`, `rustc` |
| generic-array | rustc-probe | network-fetch, protoc | – | – | – | – | – | `rustc` |
| geozero | protoc | protoc | – | `:manifest_dir/src/mvt` | `:manifest_dir/src/mvt` | – | – | `protoc` |
| getrandom | cfg-only, rustc-probe | cc-toolchain, lang-runtime, network-fetch, protoc, rustc-probe | 2 | – | – | – | – | `rustc` |
| gherkin | codegen | codegen | – | `:manifest_dir/src/languages.json` | – | – | – | – |
| gl | codegen | codegen | – | – | – | – | – | – |
| glean-core | codegen, system-lib, lang-runtime | lang-runtime | 1 | `:manifest_dir/Cargo.toml`, `:manifest_dir/src/glean.udl` | – | – | – | – |
| grok | codegen | codegen | – | `:manifest_dir/patterns` | – | – | – | – |
| guppy-workspace-hack | cfg-only | codegen | – | – | – | – | – | – |
| heapless | cfg-only, rustc-probe | codegen, rustc-probe, vcs-metadata | 1 | `:out_dir` | – | – | – | `rustc` |
| hotpath-meta | rustc-probe, vcs-metadata | vcs-metadata | 3 | – | – | – | – | `rustc` |
| httparse | cfg-only, rustc-probe | network-fetch, protoc, rustc-probe | 6 | – | – | – | – | `rustc` |
| iceoryx2-pal-configuration | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_locale_fallback_data | cfg-only | codegen | 1 | – | – | – | – | – |
| icu_normalizer_data | cfg-only | lang-runtime, network-fetch, vcs-metadata | 1 | – | – | – | – | – |
| icu_properties_data | cfg-only | cfg-only, lang-runtime, network-fetch, vcs-metadata | 1 | – | – | – | – | – |
| icu_segmenter_data | cfg-only | codegen | 1 | – | – | – | – | – |
| indexmap | rustc-probe | protoc, vcs-metadata | 1 | `:out_dir` | – | – | – | `rustc` |
| inotify-sys | system-lib | system-lib | 1 | – | – | – | – | `pkg-config` |
| intel-mkl-src | system-lib, network-fetch | network-fetch | 2 | – | – | – | yes | `pkg-config` |
| io-lifetimes | cfg-only, rustc-probe | rustc-probe | 2 | `:out_dir` | – | – | – | `rustc` |
| iso_currency | codegen | codegen | – | `:manifest_dir/isodata.tsv` | – | – | – | – |
| jetscii | cfg-only, codegen | codegen | 2 | – | – | – | – | – |
| jiter | lang-runtime | lang-runtime | 2 | – | – | – | – | – |
| jni | cfg-only, lang-runtime | lang-runtime | 2 | – | – | – | – | – |
| jni-macros | rustc-probe | lang-runtime | – | – | – | – | – | `rustc` |
| khronos_api | codegen | codegen | – | `:manifest_dir/api_webgl/extensions` | – | – | – | – |
| lalrpop-util | cfg-only | cfg-only | – | – | – | – | – | – |
| libbpf-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 4 | – | – | – | – | – |
| libc | cfg-only, rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | 20 | – | – | – | – | `rustc` |
| libdd-ddsketch | protoc | protoc | – | `:manifest_dir/src` | `:manifest_dir/src/pb` | – | – | `protoc` |
| libduckdb-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 9 | `:duckdb_lib_dir`, `:manifest_dir/src/bindgen_bundled_version.rs` | – | – | yes | `pkg-config` |
| liblzma-sys | cfg-only, system-lib, cc-toolchain | network-fetch | 3 | `:manifest_dir/config.h`, `:manifest_dir/xz`, `:out_dir` | – | – | – | `cc` |
| libm | cfg-only | vcs-metadata | 6 | – | – | – | – | – |
| libmimalloc-sys | cc-toolchain | cc-toolchain | 16 | `:manifest_dir/c_src/mimalloc`, `:out_dir` | – | – | – | `cc` |
| libnghttp2-sys | cc-toolchain | cc-toolchain | 3 | `:manifest_dir/nghttp2`, `:out_dir` | – | – | – | `cc` |
| libpulse-simple-sys | cfg-only, system-lib | system-lib | 4 | – | – | – | – | `pkg-config` |
| libpulse-sys | cfg-only, system-lib | system-lib | 4 | – | – | – | – | `pkg-config` |
| libsamplerate-sys | cc-toolchain | cc-toolchain | 2 | `:manifest_dir/libsamplerate`, `:out_dir` | – | – | – | `cc`, `cmake` |
| libseccomp | cfg-only, system-lib | system-lib | 6 | – | – | – | – | `pkg-config` |
| libseccomp-sys | system-lib | system-lib | 2 | – | – | – | – | – |
| libsodium-sys-stable | system-lib, cc-toolchain, network-fetch | network-fetch | 4 | – | – | `zig` | yes | `autotools`, `cc`, `pkg-config` |
| libsqlite3-sys | system-lib, bindgen, cc-toolchain | system-lib | 17 | `:manifest_dir/bindgen-bindings`, `:manifest_dir/sqlite3` | – | – | – | `pkg-config` |
| libz-ng-sys | cc-toolchain | cc-toolchain | 3 | `:manifest_dir/src/zlib-ng`, `:out_dir` | – | – | – | `cc`, `cmake` |
| libz-sys | system-lib, cc-toolchain | system-lib | 7 | `:manifest_dir/src/smoke.c`, `:manifest_dir/src/zlib`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| link-cplusplus | system-lib, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/src/dummy.cc`, `:out_dir` | – | – | – | `cc`, `cxx` |
| linkme | codegen | codegen | 1 | – | – | – | – | – |
| linkme-impl | rustc-probe | codegen | – | – | – | – | – | `rustc` |
| liteparse-pdfium-sys | cfg-only, system-lib, bindgen, network-fetch | network-fetch | 5 | `:manifest_dir/bindings.rs` | – | – | yes | – |
| logos-codegen | rustc-probe | protoc | – | – | – | – | – | `rustc` |
| lrlex | vcs-metadata | vcs-metadata | 2 | – | – | – | – | – |
| lrpar | cfg-only, vcs-metadata | vcs-metadata | 2 | – | – | – | – | – |
| lz4-sys | cfg-only, cc-toolchain | cc-toolchain | 2 | `:manifest_dir/liblz4`, `:out_dir` | – | – | – | `cc` |
| lzma-sys | system-lib, cc-toolchain | system-lib | 6 | `:manifest_dir/xz-*` | – | – | – | `cc`, `pkg-config` |
| mach-sys | vcs-metadata | vcs-metadata | – | – | – | – | – | – |
| matrixmultiply | cfg-only, rustc-probe | lang-runtime, rustc-probe | 4 | `:out_dir` | – | – | – | `rustc` |
| memoffset | rustc-probe | protoc, rustc-probe | 1 | `:out_dir` | – | – | – | `rustc` |
| metrics-exporter-prometheus | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| mime_guess | codegen | codegen, network-fetch | – | – | – | – | – | – |
| mongocrypt-sys | system-lib | system-lib | 1 | – | – | – | – | – |
| mp3lame-sys | cfg-only, cc-toolchain | cc-toolchain | 4 | `:manifest_dir/lame-3.100`, `:out_dir` | – | – | – | `autotools`, `cc`, `cxx` |
| names | codegen, vcs-metadata | vcs-metadata | 2 | `:manifest_dir/data` | – | – | – | – |
| napi | cfg-only, lang-runtime | lang-runtime | 3 | – | – | – | – | – |
| napi-sys | – | lang-runtime | 1 | – | – | – | – | – |
| neon-build | lang-runtime | lang-runtime | – | – | – | – | – | – |
| nextest-workspace-hack | cfg-only | cfg-only | – | – | – | – | – | – |
| nix | cfg-only | network-fetch, rustc-probe | 1 | – | – | – | – | – |
| num-traits | rustc-probe | codegen, lang-runtime, network-fetch, protoc, system-lib, vcs-metadata | 2 | `:out_dir` | – | – | – | `rustc` |
| numpy | lang-runtime | lang-runtime | 2 | – | – | – | – | – |
| objc2 | cfg-only | cfg-only | 2 | – | – | – | – | – |
| object | rustc-probe | cc-toolchain, protoc | – | – | – | – | – | `rustc` |
| odbc-sys | system-lib | system-lib | 2 | – | – | – | – | – |
| oid-registry | codegen | codegen | – | `:manifest_dir/assets/oid_db.txt` | – | – | – | – |
| openmp-sys | system-lib, cc-toolchain | system-lib | 5 | `:out_dir` | – | – | – | `cc` |
| openssl-sys | cfg-only, system-lib, bindgen, cc-toolchain | system-lib | 22 | `:manifest_dir/build/expando.c`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| ort-sys | system-lib, network-fetch | network-fetch | 9 | – | – | – | yes | – |
| oxc_sourcemap | lang-runtime | lang-runtime | 4 | – | – | – | – | – |
| parking_lot_core | cfg-only | codegen, protoc, rustc-probe | 1 | – | – | – | – | – |
| paste | cfg-only, rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| pbjson-types | protoc | protoc | – | `:manifest_dir/descriptors.bin` | – | – | – | – |
| pcap | cfg-only, system-lib | system-lib | 6 | – | – | – | – | `pkg-config` |
| pci-ids | codegen, network-fetch | network-fetch | – | `:manifest_dir/pciids/pci.ids` | `:manifest_dir/pciids/pci.ids` | `curl` | yes | – |
| perfetto_protos | protoc | protoc | – | `:manifest_dir/protos`, `:out_dir` | – | – | – | `protoc` |
| pkcs11-bindings | rustc-probe, bindgen | bindgen | – | `:manifest_dir/pkcs11.h`, `:manifest_dir/pkcs11f.h`, `:manifest_dir/pkcs11t.h`, `:manifest_dir/wrapper.h` | – | – | – | `bindgen`, `rustc` |
| portable-atomic | cfg-only, rustc-probe | rustc-probe, vcs-metadata | 11 | `:out_dir` | – | – | – | `rustc` |
| portable-atomic-util | cfg-only, rustc-probe | rustc-probe | 5 | – | – | – | – | `rustc` |
| pprof | protoc | protoc | – | `:manifest_dir/proto` | `:manifest_dir/proto` | – | – | `protoc` |
| prettyplease | cfg-only, vcs-metadata | bindgen, protoc, vcs-metadata | 4 | – | – | – | – | – |
| proc-macro-error | rustc-probe | rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| proc-macro-error-attr | rustc-probe | rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| proc-macro-hack | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| proc-macro2 | cfg-only, rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | 3 | `:manifest_dir/src/probe`, `:out_dir` | – | – | – | `rustc` |
| prometheus-client | protoc | protoc | – | `:manifest_dir/src/encoding/proto` | – | – | – | – |
| prost-build | – | protoc | 2 | – | – | – | – | – |
| prost-validate-types | protoc | protoc | – | `:manifest_dir/proto/validate`, `:out_dir` | – | – | – | `protoc` |
| prost-wkt-types | protoc | protoc | – | `:manifest_dir/proto`, `:out_dir` | – | – | – | – |
| protobuf | codegen, vcs-metadata | protoc, vcs-metadata | 2 | – | – | – | – | – |
| protobuf-src | cc-toolchain | cc-toolchain | 1 | `:manifest_dir/protobuf`, `:out_dir` | – | – | – | `cc`, `cmake`, `cxx` |
| protoc | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| psm | cfg-only, cc-toolchain | cc-toolchain, protoc | 6 | `:manifest_dir/src/arch`, `:out_dir` | – | – | – | `cc` |
| pulldown-cmark | codegen | protoc | – | – | – | – | – | – |
| pyo3 | rustc-probe, lang-runtime | lang-runtime | 4 | – | – | – | – | `rustc` |
| pyo3-ffi | rustc-probe, lang-runtime | lang-runtime | 5 | – | – | – | – | `python`, `rustc` |
| pyo3-macros-backend | – | lang-runtime | 1 | – | – | – | – | – |
| quote | rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | – | – | – | – | – | `rustc` |
| radium | – | lang-runtime | 2 | – | – | – | – | – |
| rayon-core | cfg-only | cfg-only, vcs-metadata | – | – | – | – | – | – |
| rb-sys-env | cfg-only | cfg-only | 1 | – | – | – | – | – |
| ref-cast | codegen, rustc-probe | protoc, rustc-probe | 1 | – | – | – | – | `rustc` |
| ring | cc-toolchain | cc-toolchain, network-fetch | 13 | `:manifest_dir/crypto`, `:manifest_dir/include`, `:manifest_dir/pregenerated`, `:manifest_dir/third_party`, `:out_dir` | – | – | – | `cc` |
| rquickjs-sys | cfg-only, bindgen, cc-toolchain, network-fetch | network-fetch | 6 | `:manifest_dir/quickjs`, `:manifest_dir/quickjs.bind.h`, `:out_dir` | – | `curl` | yes | `cc` |
| rustc_version_runtime | codegen, rustc-probe, vcs-metadata | vcs-metadata | – | – | – | – | – | `rustc` |
| rustix | cfg-only, rustc-probe | bindgen, lang-runtime, network-fetch, protoc, rustc-probe | 14 | `:manifest_dir/src/backend/linux_raw/arch`, `:out_dir` | – | – | – | `rustc` |
| rustler | codegen, lang-runtime | lang-runtime | 3 | – | – | – | – | – |
| rustler_sys | codegen, lang-runtime | lang-runtime | 3 | – | – | – | – | – |
| rustls | rustc-probe | network-fetch | – | – | – | – | – | – |
| rustversion | codegen, rustc-probe | codegen, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| sasl2-sys | cfg-only, system-lib, cc-toolchain | cc-toolchain | 3 | `:manifest_dir/sasl2`, `:manifest_dir/version.c`, `:out_dir` | – | – | – | `autotools`, `cc` |
| scratch | cfg-only | cfg-only | – | `:out_dir` | – | – | – | – |
| secp256k1-sys | cfg-only, system-lib, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/depend/secp256k1`, `:out_dir` | – | – | – | `cc` |
| selectors | codegen | codegen | – | – | – | – | – | – |
| selinux-sys | rustc-probe, system-lib, bindgen, cc-toolchain | bindgen | 8 | `:manifest_dir/src/selinux-sys.h`, `:out_dir` | – | – | – | `bindgen`, `cc`, `rustc` |
| sentencepiece-model | protoc | protoc | – | `:manifest_dir/src/sentencepiece.proto`, `:manifest_dir/src/sentencepiece_model.proto` | – | – | – | – |
| serde | cfg-only, codegen, rustc-probe | codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| serde_core | cfg-only, codegen, rustc-probe | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 2 | – | – | – | – | `rustc` |
| serde_json | cfg-only | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, vcs-metadata | 2 | – | – | – | – | – |
| sha2-asm | cc-toolchain | cc-toolchain | 2 | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| signal-hook | cc-toolchain | cc-toolchain | 1 | – | – | – | – | – |
| slotmap | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| smoltcp | cfg-only, codegen | codegen | 2 | – | – | – | – | – |
| snap | codegen | codegen | – | – | – | – | – | – |
| snappy_src | bindgen, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/snappy`, `:out_dir` | – | – | – | `cc`, `cxx` |
| snmalloc-sys | vcs-metadata, system-lib, cc-toolchain | cc-toolchain | 8 | `:manifest_dir/upstream`, `:out_dir` | – | – | – | `cc`, `cmake`, `cxx` |
| stacker | cc-toolchain | protoc | 1 | – | – | – | – | – |
| stellar-strkey | vcs-metadata | vcs-metadata | 1 | `:manifest_dir/.cargo_vcs_info.json` | – | – | – | `git` |
| swift-rs | lang-runtime | lang-runtime | 1 | – | – | – | – | – |
| symbolic-demangle | cc-toolchain | protoc | – | – | – | – | – | – |
| syn | rustc-probe | codegen, protoc, rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| syscalls | cfg-only | cfg-only | 1 | – | – | – | – | – |
| system-deps | system-lib, network-fetch | network-fetch | – | – | – | – | yes | – |
| target-lexicon | cfg-only, codegen, rustc-probe | codegen, lang-runtime, network-fetch | 1 | – | – | – | – | `rustc` |
| target-spec | cfg-only, codegen | codegen | 2 | – | – | – | – | – |
| terminfo | codegen | codegen | – | – | – | – | – | – |
| thiserror | cfg-only, codegen, rustc-probe | codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 4 | `:manifest_dir/build/probe.rs`, `:out_dir` | – | – | – | `rustc` |
| tikv-jemalloc-sys | cc-toolchain | cc-toolchain | 7 | `:manifest_dir/configure`, `:manifest_dir/jemalloc`, `:out_dir` | – | – | – | `autotools`, `cc` |
| tiny-keccak | cfg-only | cfg-only, codegen | – | – | – | – | – | – |
| tiny-xlib | codegen, system-lib | system-lib | 7 | – | – | – | – | `pkg-config` |
| tls-parser | codegen | codegen | 1 | `:manifest_dir/scripts/tls-ciphersuites.txt` | – | – | – | – |
| tracel-llvm-bundler | network-fetch | network-fetch | 1 | – | – | – | yes | – |
| tracing-perfetto-sdk-schema | codegen, protoc | protoc | 1 | `:manifest_dir/protos` | – | – | – | `protoc` |
| tree-sitter | cfg-only, bindgen, cc-toolchain | cc-toolchain | 5 | `:manifest_dir/include`, `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| tree-sitter-language | cfg-only | cc-toolchain, cfg-only | 2 | – | – | – | – | – |
| tree-sitter-rust | cc-toolchain | cc-toolchain | – | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| ttrpc | protoc | protoc | – | `:manifest_dir/src/ttrpc.proto` | – | – | – | – |
| tun | network-fetch | network-fetch | – | – | – | – | yes | – |
| typeid | cfg-only, rustc-probe | codegen, protoc, rustc-probe | – | – | – | – | – | `rustc` |
| typetag | codegen | codegen, protoc | 1 | – | – | – | – | – |
| typst-utils | vcs-metadata | vcs-metadata | – | – | – | – | – | `git` |
| uncased | rustc-probe | codegen | – | – | – | – | – | `rustc` |
| unicode_names2 | codegen | codegen | – | – | – | – | – | – |
| utoipa-swagger-ui | codegen, network-fetch | network-fetch | 4 | `:out_dir` | – | – | yes | – |
| v4l2-sys-mit | bindgen | bindgen | – | `:manifest_dir/wrapper.h` | – | – | – | `bindgen` |
| valuable | cfg-only | cfg-only | 1 | – | – | – | – | – |
| vcs-metadata | vcs-metadata | vcs-metadata | – | – | – | – | – | `git` |
| vergen | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| vergen-gitcl | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| vergen-lib | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| wasm-bindgen-shared | vcs-metadata | vcs-metadata | 1 | `:manifest_dir/src/lib.rs`, `:manifest_dir/src/tys.rs` | – | – | – | `git` |
| web_atoms | codegen | codegen | 1 | `:manifest_dir/local_names.txt` | – | – | – | – |
| winapi | system-lib | system-lib | 3 | – | – | – | – | – |
| wincode | codegen | vcs-metadata | – | – | – | – | – | – |
| with-built | rustc-probe | vcs-metadata | 36 | `:manifest_dir/../Cargo.lock`, `:out_dir` | – | – | – | `rustc` |
| with-shadow | vcs-metadata | vcs-metadata | 14 | `:out_dir` | – | – | – | `awk`, `git`, `grep` |
| with-vergen | rustc-probe, vcs-metadata | vcs-metadata | 7 | – | – | – | – | `git`, `rustc`, `sh` |
| x11-dl | codegen, system-lib | system-lib | 33 | – | – | – | – | `pkg-config` |
| yara-x-proto | protoc | protoc | – | `:manifest_dir/src` | – | – | – | – |
| yeslogic-fontconfig-sys | system-lib | system-lib | 4 | – | – | – | – | `pkg-config` |
| z3-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 8 | – | – | `z3` | yes | `pkg-config` |
| zerocopy | cfg-only, rustc-probe | codegen, lang-runtime, protoc, rustc-probe, vcs-metadata | – | `:manifest_dir/Cargo.toml` | – | – | – | `rustc` |
| zmij | cfg-only, rustc-probe | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| zstd-sys | system-lib, bindgen, cc-toolchain | cc-toolchain, system-lib | 11 | `:manifest_dir/zstd`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
