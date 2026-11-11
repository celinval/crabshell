//! Generates `shadow.rs` in `OUT_DIR`, like an application embedding its build.
//!
//! shadow-rs runs `rustc` and `cargo` from `PATH` rather than `$RUSTC` and
//! `$CARGO`, so their versions depend on the host's `PATH`; those constants
//! are denied, along with `cargo tree` and `cargo metadata`. It writes the
//! file without printing directives, so this also exports a digest of it: a
//! denied variable or tool changes the digest instead of going unnoticed.

use std::collections::BTreeSet;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::Path;

fn main() {
    let deny = BTreeSet::from([
        shadow_rs::CARGO_METADATA,
        shadow_rs::CARGO_TREE,
        shadow_rs::CARGO_VERSION,
        shadow_rs::RUST_VERSION,
    ]);
    shadow_rs::ShadowBuilder::builder()
        .deny_const(deny)
        .build()
        .expect("failed to write shadow.rs");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let text = std::fs::read_to_string(Path::new(&out_dir).join("shadow.rs")).unwrap();
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    println!("cargo:rustc-env=SHADOW_DIGEST={:016x}", hasher.finish());
}
