//! Resolving the effective permission set the runner enforces.
//!
//! Combines the `sandbox.toml` tables, a package's declaration, and the
//! `[limits]` caps. Pure and filesystem-free, so it is covered by unit tests
//! rather than end-to-end builds. Types are placeholders until layer 3 is
//! implemented.

/// The fully resolved permissions a build script runs with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Permissions {
    /// Read-only paths.
    pub read: Vec<String>,
    /// Executables the script may run.
    pub run: Vec<String>,
    /// Environment variables the script may read.
    pub env: Vec<String>,
    /// Whether network access is granted.
    pub network: bool,
}

#[cfg(test)]
mod tests {}
