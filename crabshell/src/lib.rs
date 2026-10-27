//! crabshell — a sandbox for Cargo build scripts.
//!
//! Cargo invokes crabshell as a `host.runner`, handing over a compiled build
//! script to run inside a sandbox while its `cargo::` directives flow back
//! unchanged. This crate holds the config parsing ([`config`], [`declared`]), policy
//! resolution ([`policy`], [`paths`]), and execution ([`runner`]); the binary drives them.

pub mod config;
pub mod declared;
pub mod paths;
pub mod policy;
pub mod runner;
