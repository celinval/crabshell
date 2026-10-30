# CrabShell

**Confined, cacheable Cargo build scripts.** `crabshell` runs each build script
with only the access it actually needs — not the full privileges of whoever ran
`cargo build` — and, because that access is declared, knows every input the
script could have used.

> ⚠️ **Prototype:** Exploratory work; the API, permission format, and
> confinement mechanism will change. Not for production use.

> 🤖 **AI Generated**: The code in this repo is heavily AI generated (read: vibe coding).
> The goal at this point is to refine and assess the protocol, not to create a
> usable solution.

## Why

Many crates ship a build script (`build.rs`) that Cargo compiles and runs during
`cargo build` — to probe for system libraries, generate code, compile vendored
C. But they run with **all of your privileges**. Any dependency in your graph,
including a transitive one pulled in by a routine `cargo update`, can silently
read your credentials (`~/.aws`, `~/.ssh`, env vars), tamper with files anywhere,
or reach the network — before you run a single test. Your build is only as
trustworthy as the least-trustworthy `build.rs` in the tree, with no way to
contain a bad one.

Build scripts are also opaque to Cargo. It can't tell what a script depends
on: `rerun-if-*` directives are self-reported and error prone. So Cargo cache
soundness rely on build script correctness. It also cannot safely reuse a result
built on another machine or run the script remotely.

## What it does

crabshell runs each build script in a sandbox that grants only the files,
programs, env vars, and network it's been explicitly allowed. Everything else
isn't there.

Two parties shape the policy:

- **Crate authors** declare what their build script needs in `Cargo.toml` —
  which paths it reads, tools it runs, env vars it uses. Reviewable documentation
  of intent.
- **You, the consumer**, set the ceiling in a `sandbox.toml` at your project
  root — what any build script in your graph is *allowed*, regardless of what it
  asks for.

The consumer controls the final policy: a crate can get *less* than you allow, never more.
A compromised dependency can't grant itself new powers by editing its own
manifest.

The same policy is the script's input set. A script can only observe what it
was granted, so the granted paths, programs and env vars form a sound cache
key, and describe what a remote executor must provide.

### What it doesn't cover

crabshell limits the damage a build script can do while it runs. It is not a
guarantee against untrusted code:

- **Library code** compiled from a dependency runs unconfined in your tests
  and binaries.
- **Proc macros** run inside rustc and cannot be wrapped by the runner.
- **Build outputs** are trusted by Cargo. A confined script can still write
  code into `OUT_DIR` that gets `include!`d, or emit linker directives (see
  [Future possibilities](docs/src/future.md)).

## Usage sketch

A crate author declares needs in `Cargo.toml`:

```toml
[package.metadata.build-rs]
requests = ["pkg-config", "cc"]       # named grants it may use
read = [":manifest_dir"]              # its own files
env  = ["OPENSSL_DIR", "OPENSSL_STATIC"]   # knobs it reads
```

You set the project policy in `sandbox.toml`:

```toml
[limits]                              # a hard ceiling nothing can exceed
network = false

[external]                            # registry and git dependencies
grants = ["rustc", "pkg-config", "cc"]
env = { inherit = true, value = ["OPENSSL_*"] }
```

Paths use crabshell variables such as `:manifest_dir`, never env vars.

You add `crabshell` as a host runner in your Cargo `config.toml`, and point it
at the policy:

```toml
[host]
runner = ["crabshell"]

[env]
CRABSHELL_CONFIG = { value = "sandbox.toml", relative = true }
```

`relative = true` resolves the path against the directory holding
`config.toml`, so every build script in the graph — including dependencies
unpacked under `CARGO_HOME` — reads the same policy. Without it, crabshell
falls back to walking up from the package being built, which only finds a
config for your own crates.

By default a build script gets almost nothing: it can read its own crate and
write to `OUT_DIR`, and that's it. Everything else is opt-in.

## Design

Design notes, the `sandbox.toml` policy specification, and a survey of what
build scripts do are in the [CrabShell book](docs/src/SUMMARY.md). Build it
with [mdBook](https://rust-lang.github.io/mdBook/):

```sh
mdbook build docs   # output in docs/book
```
