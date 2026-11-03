//! Fixture crate; only its build script matters.

/// The commit this crate was built from.
pub const GIT_HASH: &str = env!("GIT_HASH");
