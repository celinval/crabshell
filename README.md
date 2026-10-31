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

## Quick start

You need Linux with Landlock enabled, and a nightly Cargo: the `host.runner`
hook is unstable.

### Building a project

1. Install crabshell:

   ```sh
   cargo install --path crabshell
   ```

2. In your project's `.cargo/config.toml`, run build scripts through it:

   ```toml
   target-applies-to-host = false

   [host]
   runner = ["crabshell"]

   [env]
   CRABSHELL_CONFIG = { value = "sandbox.toml", relative = true }
   ```

   `relative = true` resolves the path against the parent of `.cargo/`, so
   every build script in the graph, including dependencies unpacked under
   `CARGO_HOME`, reads the same policy. crabshell never looks for a
   `sandbox.toml` near a package, since a dependency could ship one.

3. Add a `sandbox.toml` next to `.cargo/`. An empty file grants only the
   defaults: a build script can read its own crate and write to `OUT_DIR`.
   This one also lets dependencies run `rustc` probes:

   ```toml
   [limits]                  # a hard ceiling nothing can exceed
   network = false

   [external]                # registry and git dependencies
   grants = ["rustc"]
   ```

4. Build:

   ```sh
   cargo +nightly -Zhost-config -Ztarget-applies-to-host build
   ```

When a build script fails or behaves differently, set `CRABSHELL_LOG=1` to
print the permissions each script ran with, then extend `sandbox.toml`.

Your own `~/.config/crabshell/config.toml` (or under `$XDG_CONFIG_HOME`), if
present, caps every project's policy: a project can't grant more than it
allows.

### Trying crate declarations

> **Don't publish `[package.metadata.build-rs]`.** The declaration format
> isn't settled or versioned.

To try the vendor side today, only do it locally. Another possibility is to write
the declarations a crate *would* ship in a `requests.toml`, one table per package,
and point crabshell at it. It reads them in place of each crate's own metadata,
so you can model crates you don't own:

```toml
# requests.toml: what each build script would declare.
[package.thiserror]
requests = ["rustc"]                                  # named grants it may use
read = [":manifest_dir/build/probe.rs", ":out_dir"]   # files it reads
env  = ["CARGO_PKG_VERSION_PATCH"]                    # env vars beyond the defaults
```

```sh
cargo install --path crabshell --features test-requests
CRABSHELL_REQUESTS=$PWD/requests.toml cargo +nightly -Zhost-config -Ztarget-applies-to-host build
```

- A declaration only narrows what the consumer grants; it never widens it.
  Once a package declares anything, fields it leaves out request nothing.
- Name tools through grants such as `rustc`, `cc` and `pkg-config` rather
  than host paths, so a declaration works on every machine.
- Paths use variables such as `:manifest_dir` and `:out_dir`, never env vars.
  `OUT_DIR` is write-only unless a package requests `:out_dir`.
- The `rerun-if-env-changed` lines a script prints list the env vars it
  reads.

The `crabshell/fixtures/` fixtures use this flow; their `requests.toml` files are
worked examples for real crates.

## Design

Design notes, the `sandbox.toml` policy specification, and a survey of what
build scripts do are in the [CrabShell book](docs/src/SUMMARY.md). Build it
with [mdBook](https://rust-lang.github.io/mdBook/):

```sh
mdbook build docs   # output in docs/book
```
