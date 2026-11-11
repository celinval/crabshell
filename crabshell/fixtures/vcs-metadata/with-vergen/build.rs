//! Emits vergen's build, Cargo, git and rustc metadata as `rustc-env`.

use vergen_gitcl::{Build, Cargo, Emitter, Gitcl, Rustc};

fn main() {
    Emitter::default()
        .add_instructions(&Build::all_build())
        .unwrap()
        .add_instructions(&Cargo::all_cargo())
        .unwrap()
        .add_instructions(&Gitcl::all_git())
        .unwrap()
        .add_instructions(&Rustc::all_rustc())
        .unwrap()
        .emit()
        .unwrap();
}
