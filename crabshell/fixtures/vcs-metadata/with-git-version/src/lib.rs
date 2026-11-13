//! Records the commit from a proc-macro instead of a build script.
//!
//! crabshell only wraps build scripts, so no policy applies here: the macro
//! runs `git` in the crate's dir while rustc expands it. The assertion below
//! fails if it ever doesn't, which would mean something confined rustc.

use git_version::git_version;

/// The commit this crate was built from.
pub const GIT_HASH: &str = git_version!(args = ["--always", "--abbrev=40"], fallback = "unknown");

const _: () = assert!(
    GIT_HASH.len() == 40,
    "the proc-macro's `git` ran outside any sandbox"
);
