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

404 crates, 384 of them among the report's 1190 with build scripts. 249
request env, 178 reads, 16 writes, 9 programs directly and
24 network. Grants requested:
`rustc` (96), `cc` (55), `pkg-config` (31), `protoc` (22), `bindgen` (19), `git` (13), `cxx` (8), `pkg-config-path` (5), `autotools` (4), `cmake` (4), `clang` (2), `python` (1), `java` (1), `awk` (1), `grep` (1), `sh` (1), `git-repo` (1).

| Crate | Categories | Fixtures | Env | Read | Write | Run | Network | Grants |
|---|---|---|---:|---|---|---|---|---|
| aegis | cfg-only, cc-toolchain | cc-toolchain | 10 | `:manifest_dir/src/c`, `:out_dir` | – | – | – | `cc`, `clang` |
| ahash | rustc-probe | bindgen, lang-runtime, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| alloca | cc-toolchain | cc-toolchain | – | `:manifest_dir/alloca.c`, `:out_dir` | – | – | – | `cc` |
| allocative | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| alsa-sys | rustc-probe, system-lib, bindgen, cc-toolchain | system-lib | 6 | `:out_dir` | – | – | – | `cc`, `pkg-config` |
| amq-protocol | codegen | codegen | 3 | – | – | – | – | – |
| anyhow | rustc-probe | bindgen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 3 | `:manifest_dir/src/nightly.rs`, `:out_dir` | – | – | – | `rustc` |
| async-io | rustc-probe | lang-runtime, rustc-probe | 1 | `:out_dir` | – | – | – | `rustc` |
| atomic-polyfill | cfg-only | cfg-only | 1 | – | – | – | – | – |
| atomic-traits | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| aws-types | rustc-probe, vcs-metadata | vcs-metadata | – | – | – | – | – | `rustc` |
| basic-cookies | codegen | codegen | 2 | `:manifest_dir` | – | – | – | – |
| bigdecimal | cfg-only, codegen, rustc-probe | rustc-probe | 7 | `:out_dir` | – | – | – | `rustc` |
| bindgen | cfg-only | bindgen, network-fetch | 1 | – | – | – | – | – |
| blake2b-rs | cfg-only, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/BLAKE2`, `:out_dir` | – | – | – | `cc` |
| blake3 | cfg-only, cc-toolchain | cc-toolchain, protoc | 13 | `:manifest_dir/c`, `:out_dir` | – | – | – | `cc` |
| blas-src | cfg-only | cfg-only | 6 | – | – | – | – | – |
| blst | cfg-only, cc-toolchain | cc-toolchain | 10 | `:manifest_dir/blst`, `:out_dir` | – | – | – | `cc` |
| bnum | rustc-probe | rustc-probe | – | – | – | – | – | – |
| borsh | cfg-only | protoc | 2 | – | – | – | – | – |
| branches | rustc-probe | rustc-probe | 1 | – | – | – | – | `rustc` |
| brotli-sys | vcs-metadata, cc-toolchain | cc-toolchain | – | `:manifest_dir/brotli`, `:out_dir` | – | – | – | `cc`, `git` |
| bugreport | cfg-only, vcs-metadata | vcs-metadata | 9 | – | – | – | – | – |
| bzip2-sys | system-lib, cc-toolchain | cc-toolchain, network-fetch | 4 | `:manifest_dir/bzip2-1.0.8`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| camino | rustc-probe | codegen, lang-runtime, rustc-probe | – | – | – | – | – | `rustc` |
| cdivsufsort | cc-toolchain | cc-toolchain | 2 | `:manifest_dir/c-sources`, `:out_dir` | – | – | – | `cc` |
| cedar-policy | protoc | protoc | – | `:manifest_dir/protobuf_schema` | – | – | – | `protoc` |
| cedar-policy-core | codegen | protoc | 2 | `:manifest_dir/src/parser`, `:manifest_dir/src/validator/cedar_schema` | – | – | – | – |
| cee-scape | cfg-only, cc-toolchain | cc-toolchain | 2 | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| cgmath | codegen | codegen | – | – | – | – | – | – |
| chroma-types | protoc | protoc | – | `:manifest_dir/idl` | – | – | – | `protoc` |
| chrono-tz | codegen | codegen, protoc | 1 | `:manifest_dir/tz` | – | – | – | – |
| circular-queue | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| clang-sys | system-lib | bindgen, network-fetch, system-lib | – | – | – | – | – | – |
| clarabel | rustc-probe, vcs-metadata, lang-runtime | lang-runtime | 6 | – | – | – | – | `rustc` |
| clear_on_drop | cc-toolchain | cc-toolchain | – | `:manifest_dir/src/hide.c`, `:out_dir` | – | – | – | `cc` |
| codepage-437 | codegen | codegen | – | `:manifest_dir/dialect-specs` | – | – | – | – |
| const_fn | cfg-only, rustc-probe | rustc-probe | 3 | – | – | – | – | `rustc` |
| constant_time_eq | cfg-only, rustc-probe | network-fetch, rustc-probe | 1 | – | – | – | – | – |
| containerd-client | protoc | protoc | – | `:manifest_dir/vendor`, `:out_dir` | – | – | – | `protoc` |
| containerd-shim-protos | protoc | protoc | – | `:manifest_dir/vendor`, `:out_dir` | – | – | – | – |
| cookie | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| coremidi-sys | system-lib | system-lib | 1 | – | – | – | – | – |
| cortex-m-semihosting | cfg-only | cfg-only | 1 | – | – | – | – | – |
| cpp_demangle | codegen | codegen | 1 | `:manifest_dir/in`, `:manifest_dir/tests` | – | – | – | – |
| cranelift-isle | codegen | codegen | – | `:manifest_dir/isle_examples` | – | – | – | – |
| crc32c | codegen, rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| crc32fast | cfg-only, rustc-probe | lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| cross | codegen, vcs-metadata | vcs-metadata | 1 | `:manifest_dir/docker` | – | – | – | `git` |
| crossbeam-deque | cfg-only | cfg-only, codegen, network-fetch, protoc, vcs-metadata | 2 | – | – | – | – | – |
| crossbeam-epoch | cfg-only | cfg-only, codegen, network-fetch, protoc, vcs-metadata | 2 | – | – | – | – | – |
| crossbeam-utils | cfg-only | cfg-only, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 2 | – | – | – | – | – |
| crunchy | codegen | cfg-only, codegen, protoc | – | – | – | – | – | – |
| cryptoki-sys | rustc-probe, bindgen | bindgen | 1 | `:manifest_dir/vendor` | – | – | – | `bindgen`, `rustc` |
| current_platform | cfg-only | cfg-only | 2 | – | – | – | – | – |
| cxx | rustc-probe, cc-toolchain | cc-toolchain | 3 | `:manifest_dir/include`, `:manifest_dir/src/cxx.cc`, `:out_dir` | – | – | – | `cc`, `cxx`, `rustc` |
| dashu-int | cfg-only | cfg-only | – | – | – | – | – | – |
| decancer | cfg-only | cfg-only | – | – | – | – | – | – |
| decorum | cfg-only, rustc-probe | rustc-probe | – | – | – | – | – | – |
| defmt | cfg-only, codegen | codegen | 1 | `:manifest_dir/defmt.x.in` | – | – | – | – |
| defmt-macros | cfg-only | codegen | – | – | – | – | – | – |
| denokv_proto | protoc | protoc | 1 | `:manifest_dir/schema`, `:out_dir` | `:manifest_dir/protobuf` | – | – | `protoc` |
| doc-comment | rustc-probe | rustc-probe | – | – | – | – | – | – |
| drm-fourcc | bindgen | bindgen | – | `:out_dir` | `:manifest_dir/src/as_enum.rs`, `:manifest_dir/src/consts.rs` | `rustfmt` | – | `bindgen` |
| drm-sys | system-lib, bindgen | bindgen | 1 | `:libdrm_include_path` | – | – | – | `bindgen`, `pkg-config` |
| embassy-time-driver | cfg-only | cfg-only | – | – | – | – | – | – |
| encoding_c | – | rustc-probe | 1 | – | – | – | – | – |
| encoding_c_mem | – | rustc-probe | 1 | – | – | – | – | – |
| encoding_rs | rustc-probe | network-fetch, rustc-probe, vcs-metadata | – | – | – | – | – | – |
| erasable | rustc-probe | rustc-probe | 2 | `:out_dir` | – | – | – | `rustc` |
| erased-serde | rustc-probe | codegen, protoc, rustc-probe | – | – | – | – | – | `rustc` |
| errno-dragonfly | cc-toolchain | cc-toolchain | – | `:manifest_dir/src/errno.c`, `:out_dir` | – | – | – | `cc` |
| error-chain | rustc-probe | codegen, rustc-probe | 1 | – | – | – | – | `rustc` |
| esaxx-rs | cc-toolchain | cc-toolchain | – | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc`, `cxx` |
| etcd-client | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| exacl | rustc-probe, system-lib, bindgen | bindgen | – | `:manifest_dir/bindgen/wrapper.h` | – | – | – | `bindgen`, `rustc` |
| eyre | cfg-only, rustc-probe | rustc-probe, vcs-metadata | 2 | `:out_dir` | – | – | – | `rustc` |
| faster-hex | rustc-probe | vcs-metadata | – | `:out_dir` | – | – | – | `rustc` |
| fastnum | cfg-only, codegen | codegen | 5 | – | – | – | – | – |
| field-offset | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| file-lock | cc-toolchain | cc-toolchain | – | `:manifest_dir/src/file_lock.c`, `:out_dir` | – | – | – | `cc` |
| findshlibs | cfg-only | protoc | 1 | – | – | – | – | – |
| fixed | cfg-only, rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| flatbuffers | cfg-only, rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| float16 | cfg-only, rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| font-awesome-as-a-crate | cfg-only, codegen | codegen | – | `:manifest_dir/fontawesome-free-6.2.0-desktop/svgs` | – | – | – | – |
| foundationdb-sys | rustc-probe, system-lib, bindgen | bindgen | 1 | `:manifest_dir/include/730` | – | – | – | `bindgen`, `rustc` |
| fs-err | rustc-probe | lang-runtime, rustc-probe | 2 | `:out_dir` | – | – | – | `rustc` |
| fst | codegen | codegen | – | – | – | – | – | – |
| fts-sys | rustc-probe, bindgen | bindgen | 5 | `:manifest_dir/src/fts-sys.h` | – | – | – | `bindgen`, `rustc` |
| gbm-sys | bindgen | bindgen | – | `:manifest_dir/include` | – | – | – | `bindgen` |
| generic-array | rustc-probe | network-fetch, protoc, rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| geozero | protoc | protoc | – | `:manifest_dir/src/mvt` | `:manifest_dir/src/mvt` | – | – | `protoc` |
| getrandom | cfg-only, rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 2 | – | – | – | – | `rustc` |
| gherkin | codegen | codegen | – | `:manifest_dir/src/languages.json` | – | – | – | – |
| gl | codegen | codegen | – | – | – | – | – | – |
| glean-core | codegen, system-lib, lang-runtime | lang-runtime | 1 | `:manifest_dir/Cargo.toml`, `:manifest_dir/src/glean.udl` | – | – | – | – |
| glib-sys | cfg-only, system-lib | system-lib | 18 | `:manifest_path` | – | – | – | `pkg-config`, `pkg-config-path` |
| gobject-sys | cfg-only, system-lib | system-lib | 14 | `:manifest_path` | – | – | – | `pkg-config`, `pkg-config-path` |
| goosefs-sdk | protoc | protoc | 1 | `:manifest_dir/proto` | `:manifest_dir/src/generated` | – | – | `protoc` |
| grok | codegen | codegen | – | `:manifest_dir/patterns` | – | – | – | – |
| guppy-workspace-hack | cfg-only | codegen | – | – | – | – | – | – |
| hdfs-native | protoc | protoc | – | `:manifest_dir/minidfs`, `:manifest_dir/src/proto` | – | `cp` | – | `protoc` |
| heapless | cfg-only, rustc-probe | codegen, rustc-probe, vcs-metadata | 1 | `:out_dir` | – | – | – | `rustc` |
| hex_lit | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| hotpath | rustc-probe, vcs-metadata | vcs-metadata | 2 | – | – | – | – | `rustc` |
| hotpath-meta | rustc-probe, vcs-metadata | vcs-metadata | 3 | – | – | – | – | `rustc` |
| htmlize | codegen | codegen | – | `:manifest_dir/entities.json` | – | – | – | – |
| httparse | cfg-only, rustc-probe | network-fetch, protoc, rustc-probe | 6 | – | – | – | – | `rustc` |
| human_name | codegen | codegen | 1 | `:manifest_dir/build` | – | – | – | – |
| hyphenation | codegen | codegen | 1 | `:manifest_dir/dictionaries`, `:manifest_dir/patterns` | – | – | – | – |
| i-slint-compiler | codegen, vcs-metadata | vcs-metadata | 1 | `:manifest_dir/widgets` | – | – | – | – |
| iceoryx2-pal-configuration | cfg-only | bindgen, cfg-only | 1 | – | – | – | – | – |
| iceoryx2-pal-posix | cfg-only, bindgen | bindgen | 2 | – | – | – | – | `bindgen` |
| icu_calendar_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_casemap_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_collator_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_datetime_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_decimal_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_experimental_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_list_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_locale_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_locale_fallback_data | cfg-only | codegen | 1 | – | – | – | – | – |
| icu_locid_transform_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_normalizer_data | cfg-only | codegen, lang-runtime, network-fetch, protoc, vcs-metadata | 1 | – | – | – | – | – |
| icu_plurals_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_properties_data | cfg-only | cfg-only, codegen, lang-runtime, network-fetch, protoc, vcs-metadata | 1 | – | – | – | – | – |
| icu_segmenter_data | cfg-only | codegen | 1 | – | – | – | – | – |
| icu_time_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| icu_timezone_data | cfg-only | cfg-only | 1 | – | – | – | – | – |
| im-rc | rustc-probe | rustc-probe | 2 | – | – | – | – | `rustc` |
| indexmap | rustc-probe | protoc, vcs-metadata | 1 | `:out_dir` | – | – | – | `rustc` |
| inotify-sys | system-lib | system-lib | 1 | – | – | – | – | `pkg-config` |
| input-sys | bindgen | bindgen | 3 | `:manifest_dir/include`, `:manifest_dir/src/bindings` | – | – | – | `bindgen` |
| intel-mkl-src | system-lib, network-fetch | network-fetch | 2 | – | – | – | yes | `pkg-config` |
| io-lifetimes | cfg-only, rustc-probe | rustc-probe | 2 | `:out_dir` | – | – | – | `rustc` |
| iref | codegen | codegen | – | `:manifest_dir/src/uri` | – | – | – | – |
| irg-kvariants | – | codegen | 1 | `:manifest_dir/dictionaries/source/kVariants.tsv` | – | – | – | – |
| iso_currency | codegen | codegen | – | `:manifest_dir/isodata.tsv` | – | – | – | – |
| ittapi-sys | cc-toolchain | cc-toolchain | – | `:manifest_dir/c-library`, `:out_dir` | – | – | – | `cc` |
| jetscii | cfg-only, codegen | codegen | 2 | – | – | – | – | – |
| jiter | lang-runtime | lang-runtime | 2 | – | – | – | – | – |
| jni | cfg-only, lang-runtime | lang-runtime | 2 | – | – | – | – | – |
| jni-macros | rustc-probe | lang-runtime | – | – | – | – | – | `rustc` |
| jrsonnet-evaluator | – | codegen | – | – | – | – | – | – |
| khronos-egl | system-lib | system-lib | 3 | – | – | – | – | `pkg-config` |
| khronos_api | codegen | codegen | – | `:manifest_dir/api_webgl/extensions` | – | – | – | – |
| lalrpop-util | cfg-only | cfg-only | – | – | – | – | – | – |
| libbpf-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 4 | – | – | – | – | – |
| libc | cfg-only, rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | 20 | – | – | – | – | `rustc` |
| libdbus-sys | system-lib, cc-toolchain | system-lib | 3 | `:manifest_dir/vendor`, `:out_dir` | – | – | – | `cc`, `pkg-config`, `pkg-config-path` |
| libdd-ddsketch | protoc | protoc | – | `:manifest_dir/src` | `:manifest_dir/src/pb` | – | – | `protoc` |
| libdd-trace-protobuf | protoc | protoc | 1 | `:manifest_dir/src` | `:manifest_dir/src` | – | – | `protoc` |
| libdeflate-sys | system-lib, cc-toolchain | cc-toolchain | – | `:manifest_dir/libdeflate`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| libduckdb-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 9 | `:duckdb_lib_dir`, `:manifest_dir/src/bindgen_bundled_version.rs` | – | – | yes | `pkg-config` |
| libffi-sys | system-lib, cc-toolchain | cc-toolchain | 6 | `:manifest_dir/libffi`, `:out_dir` | – | – | – | `autotools`, `cc` |
| liblzma-sys | cfg-only, system-lib, cc-toolchain | network-fetch | 3 | `:manifest_dir/config.h`, `:manifest_dir/xz`, `:out_dir` | – | – | – | `cc` |
| libm | cfg-only | lang-runtime, protoc, rustc-probe, vcs-metadata | 6 | – | – | – | – | – |
| libmimalloc-sys | cc-toolchain | cc-toolchain | 16 | `:manifest_dir/c_src/mimalloc`, `:out_dir` | – | – | – | `cc` |
| libnghttp2-sys | cc-toolchain | cc-toolchain | 3 | `:manifest_dir/nghttp2`, `:out_dir` | – | – | – | `cc` |
| libproc | bindgen | bindgen | – | – | – | – | – | `bindgen` |
| libpulse-simple-sys | cfg-only, system-lib | system-lib | 4 | – | – | – | – | `pkg-config` |
| libpulse-sys | cfg-only, system-lib | system-lib | 4 | – | – | – | – | `pkg-config` |
| libsamplerate-sys | cc-toolchain | cc-toolchain | 2 | `:manifest_dir/libsamplerate`, `:out_dir` | – | – | – | `cc`, `cmake` |
| libseccomp | cfg-only, system-lib | system-lib | 6 | – | – | – | – | `pkg-config` |
| libseccomp-sys | system-lib | system-lib | 2 | – | – | – | – | – |
| libsodium-sys-stable | system-lib, cc-toolchain, network-fetch | network-fetch | 4 | – | – | `zig` | yes | `autotools`, `cc`, `pkg-config` |
| libsqlite3-sys | system-lib, bindgen, cc-toolchain | system-lib | 17 | `:manifest_dir/bindgen-bindings`, `:manifest_dir/sqlite3` | – | – | – | `pkg-config` |
| libsystemd-sys | system-lib | system-lib | 9 | – | – | – | – | `pkg-config`, `pkg-config-path` |
| libxdo-sys | system-lib | system-lib | – | – | – | – | – | – |
| libz-ng-sys | cc-toolchain | cc-toolchain | 3 | `:manifest_dir/src/zlib-ng`, `:out_dir` | – | – | – | `cc`, `cmake` |
| libz-sys | system-lib, cc-toolchain | system-lib | 7 | `:manifest_dir/src/smoke.c`, `:manifest_dir/src/zlib`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| lindera-cc-cedict | codegen, network-fetch | network-fetch | 4 | `:manifest_dir/context_id_freq.txt`, `:manifest_dir/metadata.json`, `:out_dir` | – | – | yes | – |
| lindera-ipadic | network-fetch | network-fetch | 4 | `:manifest_dir/context_id_freq.txt`, `:manifest_dir/metadata.json`, `:out_dir` | – | – | yes | – |
| lindera-ipadic-neologd | network-fetch | network-fetch | 4 | `:manifest_dir/context_id_freq.txt`, `:manifest_dir/metadata.json`, `:out_dir` | – | – | yes | – |
| lindera-ko-dic | network-fetch | network-fetch | 4 | `:manifest_dir/context_id_freq.txt`, `:manifest_dir/metadata.json`, `:out_dir` | – | – | yes | – |
| lindera-unidic | network-fetch | network-fetch | 4 | `:manifest_dir/context_id_freq.txt`, `:manifest_dir/metadata.json`, `:out_dir` | – | – | yes | – |
| link-cplusplus | system-lib, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/src/dummy.cc`, `:out_dir` | – | – | – | `cc`, `cxx` |
| linkme | codegen | codegen | 1 | – | – | – | – | – |
| linkme-impl | rustc-probe | codegen | – | – | – | – | – | `rustc` |
| liteparse-pdfium-sys | cfg-only, system-lib, bindgen, network-fetch | network-fetch | 7 | `:manifest_dir/bindings.rs` | – | – | yes | – |
| logos-codegen | rustc-probe | protoc | – | – | – | – | – | `rustc` |
| lrlex | vcs-metadata | vcs-metadata | 2 | – | – | – | – | – |
| lrpar | cfg-only, vcs-metadata | vcs-metadata | 2 | – | – | – | – | – |
| lz4-sys | cfg-only, cc-toolchain | cc-toolchain | 2 | `:manifest_dir/liblz4`, `:out_dir` | – | – | – | `cc` |
| lzma-sys | system-lib, cc-toolchain | system-lib | 6 | `:manifest_dir/xz-*` | – | – | – | `cc`, `pkg-config` |
| mach-sys | vcs-metadata | vcs-metadata | – | – | – | – | – | – |
| magic-sys | cfg-only, system-lib | system-lib | 14 | – | – | – | – | `pkg-config`, `pkg-config-path` |
| malachite-nz | lang-runtime | lang-runtime | 5 | – | – | – | – | – |
| matrixmultiply | cfg-only, rustc-probe | lang-runtime, protoc, rustc-probe | 4 | `:out_dir` | – | – | – | `rustc` |
| maybe-uninit | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| memoffset | rustc-probe | bindgen, protoc, rustc-probe, vcs-metadata | 1 | `:out_dir` | – | – | – | `rustc` |
| metrics-exporter-prometheus | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| midir | system-lib | system-lib | – | – | – | – | – | – |
| mime_guess | codegen | codegen, network-fetch | – | – | – | – | – | – |
| mini-moka | – | rustc-probe | – | – | – | – | – | – |
| minicov | cc-toolchain | cc-toolchain | – | `:manifest_dir/c`, `:out_dir` | – | – | – | `cc`, `clang` |
| miniserde | codegen | codegen | 2 | `:manifest_dir/src/place.rs` | – | – | – | – |
| miniz-sys | cc-toolchain | cc-toolchain | 1 | `:manifest_dir/miniz.c`, `:manifest_dir/miniz.h`, `:out_dir` | – | – | – | `cc` |
| mongocrypt-sys | system-lib | system-lib | 1 | – | – | – | – | – |
| mp3lame-sys | cfg-only, cc-toolchain | cc-toolchain | 4 | `:manifest_dir/lame-3.100`, `:out_dir` | – | – | – | `autotools`, `cc`, `cxx` |
| msvc_spectre_libs | system-lib | system-lib, vcs-metadata | 8 | – | – | – | – | – |
| multer | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| mundy | lang-runtime | lang-runtime | 15 | `:out_dir` | – | – | – | `java` |
| names | codegen, vcs-metadata | vcs-metadata | 2 | `:manifest_dir/data` | – | – | – | – |
| nano-gemm-f64 | codegen | codegen | – | – | – | – | – | – |
| napi | cfg-only, lang-runtime | lang-runtime | 3 | – | – | – | – | – |
| napi-sys | – | lang-runtime | 1 | – | – | – | – | – |
| neon-build | lang-runtime | lang-runtime | – | – | – | – | – | – |
| netstat2 | rustc-probe, bindgen | bindgen | 1 | – | – | – | – | `bindgen`, `rustc` |
| nextest-workspace-hack | cfg-only | cfg-only | – | – | – | – | – | – |
| nix | cfg-only | bindgen, network-fetch, rustc-probe, vcs-metadata | 1 | – | – | – | – | – |
| no-panic | cfg-only, rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| ntriple | codegen | codegen | – | `:manifest_dir/grammar/ntriple.rustpeg` | – | – | – | – |
| num-bigint-dig | – | rustc-probe | – | – | – | – | – | – |
| num-complex | rustc-probe | protoc | 1 | `:out_dir` | – | – | – | `rustc` |
| num-traits | rustc-probe | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | 2 | `:out_dir` | – | – | – | `rustc` |
| numpy | lang-runtime | lang-runtime | 2 | – | – | – | – | – |
| objc2 | cfg-only | cfg-only | 2 | – | – | – | – | – |
| object | rustc-probe | cc-toolchain, codegen, protoc | – | – | – | – | – | `rustc` |
| oboe-sys | cfg-only, system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 7 | `:manifest_dir/oboe`, `:manifest_dir/oboe-ext`, `:out_dir` | – | – | yes | `bindgen`, `cc`, `cxx` |
| odbc-sys | system-lib | system-lib | 2 | – | – | – | – | – |
| oid-registry | codegen | codegen | – | `:manifest_dir/assets/oid_db.txt` | – | – | – | – |
| openmp-sys | system-lib, cc-toolchain | system-lib | 5 | `:out_dir` | – | – | – | `cc` |
| openssl-sys | cfg-only, system-lib, bindgen, cc-toolchain | system-lib | 22 | `:manifest_dir/build/expando.c`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| ort-sys | system-lib, network-fetch | network-fetch | 9 | – | – | – | yes | – |
| oxc_sourcemap | lang-runtime | lang-runtime | 4 | – | – | – | – | – |
| packed_struct | codegen | codegen | – | – | – | – | – | – |
| parking_lot_core | cfg-only | codegen, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | – |
| paste | cfg-only, rustc-probe | bindgen, rustc-probe | – | – | – | – | – | `rustc` |
| pbjson-types | protoc | protoc | – | `:manifest_dir/descriptors.bin` | – | – | – | – |
| pcap | cfg-only, system-lib | system-lib | 6 | – | – | – | – | `pkg-config` |
| pci-ids | codegen, network-fetch | network-fetch | – | `:manifest_dir/pciids/pci.ids` | `:manifest_dir/pciids/pci.ids` | `curl` | yes | – |
| pcsc-sys | system-lib | system-lib | 6 | – | – | – | – | `pkg-config` |
| perfetto_protos | protoc | protoc | – | `:manifest_dir/protos`, `:out_dir` | – | – | – | `protoc` |
| pest_stack_guard | – | codegen | 1 | – | – | – | – | – |
| phonenumber | – | codegen | – | `:manifest_dir/assets/PhoneNumberMetadata.xml` | – | – | – | – |
| pinyin | codegen | codegen | – | – | – | – | – | – |
| pkcs11-bindings | rustc-probe, bindgen | bindgen | – | `:manifest_dir/pkcs11.h`, `:manifest_dir/pkcs11f.h`, `:manifest_dir/pkcs11t.h`, `:manifest_dir/wrapper.h` | – | – | – | `bindgen`, `rustc` |
| ply-rs | – | codegen | 2 | `:manifest_dir/README.md` | – | – | – | – |
| portable-atomic | cfg-only, rustc-probe | rustc-probe, vcs-metadata | 11 | `:out_dir` | – | – | – | `rustc` |
| portable-atomic-util | cfg-only, rustc-probe | rustc-probe | 5 | – | – | – | – | `rustc` |
| postgresql_embedded | network-fetch | network-fetch | 4 | – | – | – | yes | – |
| pprof | protoc | protoc | – | `:manifest_dir/proto` | `:manifest_dir/proto` | – | – | `protoc` |
| pprof2 | protoc | protoc | – | `:manifest_dir/proto` | `:manifest_dir/proto` | – | – | `protoc` |
| pq-sys | rustc-probe, system-lib, bindgen | bindgen | 9 | `:manifest_dir/src`, `:manifest_dir/wrapper.h` | – | `pg_config` | – | `bindgen`, `pkg-config`, `rustc` |
| precis-core | codegen, network-fetch | codegen | 1 | `:manifest_dir/resources/ucd` | – | – | yes | – |
| precis-profiles | codegen, network-fetch | codegen | 1 | `:manifest_dir/resources/ucd` | – | – | yes | – |
| prettier-please | cfg-only, vcs-metadata | vcs-metadata | – | – | – | – | – | – |
| prettyplease | cfg-only, vcs-metadata | bindgen, network-fetch, protoc, vcs-metadata | 4 | – | – | – | – | – |
| proc-macro-error | rustc-probe | codegen, rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| proc-macro-error-attr | rustc-probe | codegen, rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| proc-macro-hack | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| proc-macro-nested | codegen | codegen | – | `:out_dir` | – | – | – | – |
| proc-macro2 | cfg-only, rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | 3 | `:manifest_dir/src/probe`, `:out_dir` | – | – | – | `rustc` |
| proc-macro2-diagnostics | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| proc-maps | rustc-probe, bindgen | bindgen | – | `:manifest_dir/src/freebsd_maps/wrapper.h` | `:manifest_dir/src/freebsd_maps/bindings.rs` | – | – | `bindgen`, `rustc` |
| procfs | – | rustc-probe | 1 | – | – | – | – | – |
| prometheus | protoc | protoc | – | `:manifest_dir/proto` | `:manifest_dir/proto` | – | – | – |
| prometheus-client | protoc | protoc | – | `:manifest_dir/src/encoding/proto` | – | – | – | – |
| prost-build | – | protoc | 2 | – | – | – | – | – |
| prost-validate-types | protoc | protoc | – | `:manifest_dir/proto/validate`, `:out_dir` | – | – | – | `protoc` |
| prost-wkt-types | protoc | protoc | – | `:manifest_dir/proto`, `:out_dir` | – | – | – | – |
| protobuf | codegen, vcs-metadata | protoc, vcs-metadata | 2 | – | – | – | – | – |
| protobuf-src | cc-toolchain | cc-toolchain | 1 | `:manifest_dir/protobuf`, `:out_dir` | – | – | – | `cc`, `cmake`, `cxx` |
| protoc | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| protosol | protoc | protoc | 3 | `:manifest_dir/.gitmodules`, `:manifest_dir/proto`, `:manifest_dir/src/generated` | `:manifest_dir/src/generated` | `:manifest_dir/opt/bin/protoc` | – | `protoc` |
| psm | cfg-only, cc-toolchain | cc-toolchain, codegen, protoc | 6 | `:manifest_dir/src/arch`, `:out_dir` | – | – | – | `cc` |
| pulldown-cmark | codegen | codegen, protoc, vcs-metadata | – | – | – | – | – | – |
| pulsar | protoc | protoc | – | `:manifest_dir/PulsarApi.proto` | – | – | – | `protoc` |
| pyo3 | rustc-probe, lang-runtime | lang-runtime | 8 | – | – | – | – | `rustc` |
| pyo3-ffi | rustc-probe, lang-runtime | lang-runtime | 8 | – | – | – | – | `python`, `rustc` |
| quote | rustc-probe | bindgen, cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, system-lib, vcs-metadata | – | – | – | – | – | `rustc` |
| radium | – | codegen, lang-runtime | 2 | – | – | – | – | – |
| rayon-core | cfg-only | cfg-only, codegen, network-fetch, protoc, vcs-metadata | – | – | – | – | – | – |
| rb-sys-env | cfg-only | cfg-only | 1 | – | – | – | – | – |
| readme-rustdocifier | – | codegen | 2 | `:manifest_dir/README.md` | – | – | – | – |
| reed-solomon-simd | – | codegen | 2 | `:manifest_dir/README.md` | – | – | – | – |
| ref-cast | codegen, rustc-probe | protoc, rustc-probe | 1 | – | – | – | – | `rustc` |
| regorus | vcs-metadata | vcs-metadata | 1 | `:manifest_dir/scripts` | `:manifest_dir/.git/hooks` | – | – | `git` |
| ring | cc-toolchain | cc-toolchain, network-fetch, protoc | 13 | `:manifest_dir/crypto`, `:manifest_dir/include`, `:manifest_dir/pregenerated`, `:manifest_dir/third_party`, `:out_dir` | – | – | – | `cc` |
| rio_api | – | cfg-only | – | – | – | – | – | – |
| riptoken | – | codegen | 1 | – | – | – | – | – |
| rquickjs-sys | cfg-only, bindgen, cc-toolchain, network-fetch | network-fetch | 6 | `:manifest_dir/quickjs`, `:manifest_dir/quickjs.bind.h`, `:out_dir` | – | `curl` | yes | `cc` |
| rstest_macros | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| rust_decimal | – | codegen | – | `:manifest_dir/README.md` | – | – | – | – |
| rustc_version_runtime | codegen, rustc-probe, vcs-metadata | vcs-metadata | – | – | – | – | – | `rustc` |
| rustix | cfg-only, rustc-probe | bindgen, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 14 | `:manifest_dir/src/backend/linux_raw/arch`, `:out_dir` | – | – | – | `rustc` |
| rustler | codegen, lang-runtime | lang-runtime | 3 | – | – | – | – | – |
| rustler_sys | codegen, lang-runtime | lang-runtime | 3 | – | – | – | – | – |
| rustls | rustc-probe | network-fetch, protoc | – | – | – | – | – | – |
| rustversion | codegen, rustc-probe | codegen, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| s2n-quic-platform | cfg-only, rustc-probe, cc-toolchain | rustc-probe | 5 | `:manifest_dir/features`, `:out_dir` | – | – | – | `cc`, `rustc` |
| scratch | cfg-only | cfg-only | – | `:out_dir` | – | – | – | – |
| scuffle-workspace-hack | cfg-only | cfg-only | – | – | – | – | – | – |
| secp256k1-sys | cfg-only, system-lib, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/depend/secp256k1`, `:out_dir` | – | – | – | `cc` |
| selectors | codegen | codegen | – | – | – | – | – | – |
| selinux-sys | rustc-probe, system-lib, bindgen, cc-toolchain | bindgen | 8 | `:manifest_dir/src/selinux-sys.h`, `:out_dir` | – | – | – | `bindgen`, `cc`, `rustc` |
| sentencepiece-model | protoc | protoc | – | `:manifest_dir/src/sentencepiece.proto`, `:manifest_dir/src/sentencepiece_model.proto` | – | – | – | – |
| sentry-contexts | codegen, rustc-probe, vcs-metadata | vcs-metadata | 2 | – | – | – | – | `rustc` |
| serde | cfg-only, codegen, rustc-probe | codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| serde_core | cfg-only, codegen, rustc-probe | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 2 | – | – | – | – | `rustc` |
| serde_json | cfg-only | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 2 | – | – | – | – | – |
| sha1collisiondetection | – | rustc-probe | – | – | – | – | – | – |
| sha2-asm | cc-toolchain | cc-toolchain | 2 | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| sherpa-onnx-sys | cfg-only, codegen, system-lib, network-fetch | network-fetch | 11 | `:sherpa_onnx_lib_dir` | – | – | yes | – |
| signal-hook | cc-toolchain | cc-toolchain, vcs-metadata | 1 | – | – | – | – | – |
| slice-dst | rustc-probe | rustc-probe | 1 | `:out_dir` | – | – | – | `rustc` |
| slog | cfg-only, rustc-probe | rustc-probe | 1 | – | – | – | – | – |
| slotmap | rustc-probe | rustc-probe | – | – | – | – | – | `rustc` |
| smallbox | – | cfg-only | – | – | – | – | – | – |
| smg-mesh | protoc | protoc | – | `:manifest_dir/src/proto` | – | – | – | `protoc` |
| smoltcp | cfg-only, codegen | codegen | 2 | – | – | – | – | – |
| snap | codegen | codegen | – | – | – | – | – | – |
| snappy_src | bindgen, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/snappy`, `:out_dir` | – | – | – | `cc`, `cxx` |
| snmalloc-sys | vcs-metadata, system-lib, cc-toolchain | cc-toolchain | 8 | `:manifest_dir/upstream`, `:out_dir` | – | – | – | `cc`, `cmake`, `cxx` |
| socketcan | – | rustc-probe | 1 | – | – | – | – | – |
| solana-ed25519 | cfg-only, rustc-probe | vcs-metadata | 2 | – | – | – | – | `rustc` |
| solana-version | vcs-metadata | vcs-metadata | – | – | – | – | – | `git` |
| soroban-env-common | vcs-metadata | vcs-metadata | 1 | `:manifest_dir/.cargo_vcs_info.json` | – | – | – | `git` |
| sqlite-vec | cc-toolchain | cc-toolchain | – | `:manifest_dir/sqlite-vec.c`, `:manifest_dir/sqlite-vec.h`, `:manifest_dir/sqlite3.h`, `:manifest_dir/sqlite3ext.h`, `:out_dir` | – | – | – | `cc` |
| sqlite3-parser | codegen, cc-toolchain | codegen | 1 | `:manifest_dir/src/parser/parse.y`, `:manifest_dir/third_party`, `:out_dir` | – | – | – | `cc` |
| sqlite3-src | cfg-only, system-lib, cc-toolchain | cc-toolchain | 7 | `:manifest_dir/source`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
| ss58-registry | codegen | codegen | – | – | – | – | – | – |
| stabby-abi | cfg-only, codegen, rustc-probe | rustc-probe | 4 | – | – | – | – | `rustc` |
| stabby-macros | codegen, rustc-probe, vcs-metadata | rustc-probe, vcs-metadata | 5 | – | – | – | – | `rustc` |
| stacker | cc-toolchain | codegen, protoc | 1 | – | – | – | – | – |
| stellar-strkey | vcs-metadata | vcs-metadata | 1 | `:manifest_dir/.cargo_vcs_info.json` | – | – | – | `git` |
| stellar-xdr | cfg-only, vcs-metadata | vcs-metadata | 1 | `:manifest_dir/.cargo_vcs_info.json` | – | – | – | `git` |
| stop-words | codegen | codegen | 1 | `:manifest_dir/src/iso/stopwords-iso.json` | – | – | – | – |
| substring | rustc-probe | rustc-probe | 1 | `:out_dir` | – | – | – | `rustc` |
| swift-rs | lang-runtime | lang-runtime | 1 | – | – | – | – | – |
| symbolic-demangle | cc-toolchain | protoc | – | – | – | – | – | – |
| syn | rustc-probe | codegen, lang-runtime, protoc, rustc-probe, vcs-metadata | – | – | – | – | – | `rustc` |
| sys-info | system-lib, cc-toolchain | cc-toolchain | 1 | `:manifest_dir/c`, `:out_dir` | – | – | – | `cc` |
| syscalls | cfg-only | cfg-only | 1 | – | – | – | – | – |
| system-deps | system-lib, network-fetch | network-fetch, system-lib | 3 | – | – | – | yes | – |
| target-lexicon | cfg-only, codegen, rustc-probe | codegen, lang-runtime, network-fetch, system-lib | 1 | – | – | – | – | `rustc` |
| target-spec | cfg-only, codegen | codegen | 2 | – | – | – | – | – |
| target-triple | – | network-fetch | 2 | – | – | – | – | – |
| terminfo | codegen | codegen | – | – | – | – | – | – |
| test-fuzz-internal | – | rustc-probe | – | – | – | – | – | – |
| thiserror | cfg-only, codegen, rustc-probe | bindgen, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 4 | `:manifest_dir/build/probe.rs`, `:out_dir` | – | – | – | `rustc` |
| tikv-jemalloc-sys | cc-toolchain | cc-toolchain | 7 | `:manifest_dir/configure`, `:manifest_dir/jemalloc`, `:out_dir` | – | – | – | `autotools`, `cc` |
| tiny-keccak | cfg-only | cfg-only, codegen | – | – | – | – | – | – |
| tiny-xlib | codegen, system-lib | system-lib | 7 | – | – | – | – | `pkg-config` |
| tls-parser | codegen | codegen | 1 | `:manifest_dir/scripts/tls-ciphersuites.txt` | – | – | – | – |
| tokio-cron-scheduler | codegen, protoc | protoc | 1 | `:manifest_dir/proto`, `:out_dir` | `:manifest_dir/src/job` | – | – | `protoc` |
| tracel-llvm-bundler | network-fetch | network-fetch | 1 | – | – | – | yes | – |
| tracing-perfetto-sdk-schema | codegen, protoc | protoc | 1 | `:manifest_dir/protos` | – | – | – | `protoc` |
| tree-sitter | cfg-only, bindgen, cc-toolchain | cc-toolchain | 5 | `:manifest_dir/include`, `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| tree-sitter-language | cfg-only | cc-toolchain, cfg-only | 2 | – | – | – | – | – |
| tree-sitter-rust | cc-toolchain | cc-toolchain | – | `:manifest_dir/src`, `:out_dir` | – | – | – | `cc` |
| ttrpc | protoc | protoc | – | `:manifest_dir/src/ttrpc.proto` | – | – | – | – |
| tun | network-fetch | network-fetch | – | – | – | – | yes | – |
| tun-rs | cfg-only, rustc-probe, bindgen | bindgen | 2 | `:manifest_dir/src/platform/windows/tun/wintun_functions.h` | – | – | – | `bindgen`, `rustc` |
| tun-tap | cc-toolchain | cc-toolchain | – | `:manifest_dir/src/tuntap.c`, `:out_dir` | – | – | – | `cc` |
| typeid | cfg-only, rustc-probe | codegen, protoc, rustc-probe, system-lib | – | – | – | – | – | `rustc` |
| typetag | codegen | codegen, protoc | 1 | – | – | – | – | – |
| typst-utils | vcs-metadata | vcs-metadata | – | – | – | – | – | `git` |
| uncased | rustc-probe | codegen | – | – | – | – | – | `rustc` |
| unicode-case-mapping | codegen | codegen | – | – | – | – | – | – |
| unicode-general-category | codegen | codegen | – | – | – | – | – | – |
| unicode_names2 | codegen | codegen | – | – | – | – | – | – |
| utoipa-swagger-ui | codegen, network-fetch | network-fetch | 4 | `:out_dir` | – | – | yes | – |
| v4l2-sys-mit | bindgen | bindgen | – | `:manifest_dir/wrapper.h` | – | – | – | `bindgen` |
| valuable | cfg-only | cfg-only | 1 | – | – | – | – | – |
| vcs-metadata | vcs-metadata | vcs-metadata | – | – | – | – | – | `git` |
| vergen | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| vergen-gitcl | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| vergen-gix | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| vergen-lib | cfg-only, rustc-probe | vcs-metadata | – | – | – | – | – | – |
| verus_prettyplease | cfg-only, vcs-metadata | vcs-metadata | 1 | – | – | – | – | – |
| vsprintf | cc-toolchain | cc-toolchain | – | `:manifest_dir/src/lib.c`, `:out_dir` | – | – | – | `cc` |
| vstd | – | vcs-metadata | 1 | – | – | – | – | – |
| wasm-bindgen | cfg-only, rustc-probe | rustc-probe | 3 | – | – | – | – | – |
| wasm-bindgen-shared | vcs-metadata | rustc-probe, vcs-metadata | 1 | `:manifest_dir/src/lib.rs`, `:manifest_dir/src/tys.rs` | – | – | – | `git` |
| wasm-encoder | rustc-probe | vcs-metadata | – | – | – | – | – | `rustc` |
| wasmparser | rustc-probe | vcs-metadata | – | – | – | – | – | `rustc` |
| web_atoms | codegen | codegen | 1 | `:manifest_dir/local_names.txt` | – | – | – | – |
| winapi | system-lib | system-lib | 3 | – | – | – | – | – |
| winapi-x86_64-pc-windows-gnu | system-lib | system-lib | 3 | – | – | – | – | – |
| wincode | codegen | vcs-metadata | – | – | – | – | – | – |
| windows_x86_64_msvc | system-lib | system-lib | 1 | – | – | – | – | – |
| wit-bindgen-rt | cfg-only, system-lib | system-lib, vcs-metadata | 2 | `:manifest_dir/src/libwit_bindgen_cabi.a`, `:manifest_dir/src/libwit_bindgen_cabi_realloc.a` | – | – | – | – |
| wit-bindgen-rust | cfg-only | vcs-metadata | – | – | – | – | – | – |
| wit-bindgen-rust-macro | cfg-only | vcs-metadata | – | – | – | – | – | – |
| with-built | rustc-probe | vcs-metadata | 36 | `:manifest_dir/../Cargo.lock`, `:out_dir` | – | – | – | `rustc` |
| with-shadow | vcs-metadata | vcs-metadata | 14 | `:out_dir` | – | – | – | `awk`, `git`, `grep` |
| with-vergen | rustc-probe, vcs-metadata | vcs-metadata | 7 | – | – | – | – | `git`, `rustc`, `sh` |
| with-vergen-gix | rustc-probe, vcs-metadata | vcs-metadata | 6 | – | – | – | – | `git-repo`, `rustc` |
| woothee | codegen, vcs-metadata, network-fetch | codegen | – | `:manifest_dir/templates` | `:manifest_dir/src`, `:manifest_dir/tests` | – | yes | `git` |
| x11 | system-lib | system-lib | 52 | – | – | – | – | `pkg-config` |
| x11-dl | codegen, system-lib | system-lib | 33 | – | – | – | – | `pkg-config` |
| xgboost_lib-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 9 | `:manifest_dir/lib`, `:manifest_dir/xgboost` | `:cargo_home/xgboost-prebuilt` | `codesign`, `install_name_tool` | yes | `bindgen` |
| yara-x-proto | protoc | protoc | – | `:manifest_dir/src` | – | – | – | – |
| yellowstone-grpc-proto | protoc | protoc | – | `:manifest_dir/proto` | – | – | – | `protoc` |
| yeslogic-fontconfig-sys | system-lib | system-lib | 4 | – | – | – | – | `pkg-config` |
| z3-sys | system-lib, bindgen, cc-toolchain, network-fetch | network-fetch | 8 | – | – | `z3` | yes | `pkg-config` |
| zed_extension_api | vcs-metadata | vcs-metadata | 1 | – | – | – | – | – |
| zerocopy | cfg-only, rustc-probe | bindgen, codegen, lang-runtime, protoc, rustc-probe, vcs-metadata | – | `:manifest_dir/Cargo.toml` | – | – | – | `rustc` |
| zmij | cfg-only, rustc-probe | cc-toolchain, codegen, lang-runtime, network-fetch, protoc, rustc-probe, vcs-metadata | 1 | – | – | – | – | `rustc` |
| zstd-safe | cfg-only | network-fetch | 2 | – | – | – | – | – |
| zstd-sys | system-lib, bindgen, cc-toolchain | cc-toolchain, network-fetch, system-lib | 11 | `:manifest_dir/zstd`, `:out_dir` | – | – | – | `cc`, `pkg-config` |
