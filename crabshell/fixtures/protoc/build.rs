//! Compiles `proto/` with the protoc that protoc-bin-vendored ships.
//!
//! The schema imports a well-known type, which protoc finds in the include
//! dir next to its binary.

fn main() {
    let protoc = protoc_bin_vendored::protoc_bin_path().unwrap();
    prost_build::Config::new()
        .protoc_executable(protoc)
        .compile_protos(&["proto/fixture.proto"], &["proto"])
        .unwrap();
    println!("cargo:rerun-if-changed=proto");
}
