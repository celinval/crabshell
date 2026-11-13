//! Fixture crate; only its build script matters.

/// The commit this crate was built from.
pub const GIT_SHA: &str = env!("VERGEN_GIT_SHA");
