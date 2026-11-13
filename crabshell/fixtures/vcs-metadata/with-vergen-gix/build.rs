//! Emits vergen's build, Cargo, git and rustc metadata as `rustc-env`.
//!
//! Same as `with-vergen`, except that gix reads the repository in process:
//! the script runs no program, so it needs the git dir and the work tree
//! readable but no `git` on `PATH`.

use vergen_gix::{Build, Cargo, Emitter, Gix, Rustc};

fn main() {
    Emitter::default()
        .add_instructions(&Build::all_build())
        .unwrap()
        .add_instructions(&Cargo::all_cargo())
        .unwrap()
        .add_instructions(&Gix::all_git())
        .unwrap()
        .add_instructions(&Rustc::all_rustc())
        .unwrap()
        .emit()
        .unwrap();
}
