//! Generates `built.rs` in `OUT_DIR`, like an application embedding its build.
//!
//! `built` writes the file without printing directives, so this also exports
//! a digest of it: a denied variable or tool changes the digest instead of
//! going unnoticed.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

fn main() {
    built::write_built_file().expect("failed to write built.rs");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let text = std::fs::read_to_string(Path::new(&out_dir).join("built.rs")).unwrap();
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    println!("cargo:rustc-env=BUILT_DIGEST={:016x}", hasher.finish());
}
