//! crabshell — a sandbox for Cargo build scripts.
//!
//! crabshell is invoked by Cargo as a `host.runner`: Cargo compiles a build
//! script, then hands the resulting executable (and its arguments) to us
//! instead of running it directly. Our job is to launch that executable inside
//! a sandbox and let its `cargo::` directives flow back to Cargo unchanged.
//!
//! This crate holds the testable pieces of that workflow — policy resolution
//! ([`policy`]) and build-script execution ([`runner`]). The binary is a thin
//! shell that parses arguments and drives the steps in order.

pub mod policy;
pub mod runner;
