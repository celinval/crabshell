//! Resolving the effective permission set for a build script.
//!
//! This is the pure core of crabshell: given the consumer's `sandbox.toml`
//! tables, a package's own declaration, and the `[limits]` caps, it computes
//! the single effective permission set the runner enforces. Being pure and
//! filesystem-free, it is exercised by table-driven unit tests (below) rather
//! than end-to-end builds.
//!
//! The resolution rules — per-field inheritance, `[limits]` clamping, and
//! intersection with the package declaration — are specified in the README's
//! policy specification. The types here are placeholders to be filled in as we
//! implement layer 3.

/// The effective, fully resolved permissions a build script runs with.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Permissions {
    /// Read-only paths (resolved symbolic roots).
    pub read: Vec<String>,
    /// Executables the script may run.
    pub run: Vec<String>,
    /// Environment variables the script may read.
    pub env: Vec<String>,
    /// Whether network access is granted.
    pub network: bool,
}

#[cfg(test)]
mod tests {
    // Resolution unit tests land here as layer 3 is implemented.
}
