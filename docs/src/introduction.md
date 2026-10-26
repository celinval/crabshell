# Introduction

Design notes for sandboxing Cargo build scripts (and, later, proc-macros).

crabshell runs each build script with only the files, programs, env vars, and
network it has been allowed. Two parties shape that policy: the crate author
declares what the script needs, and the consumer sets the ceiling. A crate can
get less than the consumer allows, never more.

> ⚠️ **Prototype.** The permission format and confinement mechanism will
> change.

The book has two parts:

- **Design** describes the approach and the `sandbox.toml` policy contract.
- **Research** surveys what real build scripts do and compares crabshell with
  related work.

Build the book with `mdbook build docs`. The output goes to `docs/book`.
