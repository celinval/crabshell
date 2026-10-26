# Comparison with Cackle

[Cackle](https://github.com/cackle-rs/cackle) (`cargo acl`) is the closest
prior art for Cargo. This chapter compares the two *contracts*: what a policy
can express, who writes it, and what it guarantees. It describes Cackle's
documented configuration (`CONFIG.md`) as of 2026-10-26.

## Different questions

The two tools answer different questions:

- **Cackle:** which crates may *use* which APIs. It checks the compiled code of
  every crate (library, build script, test) against named API sets and fails
  the build on unapproved use. Separately, it sandboxes code that runs during
  the build and tests.
- **crabshell:** what a build script may *touch* while it runs. The contract
  is only about runtime access, and only for build scripts.

## Contract side by side

| | Cackle | crabshell |
|---|---|---|
| Policy author | Consumer only (`cackle.toml`). Crates can export API *definitions* (`cackle/export.toml`), but not requests. | Consumer sets the ceiling (`sandbox.toml`). The crate author declares requests in `Cargo.toml`, which can only tighten. |
| Subjects | Library code, build scripts, tests, rustc (so proc macros) | Build scripts |
| Permission unit | Named API: a set of Rust paths (`fs`, `net`, `process`, custom `[api.NAME]`), plus `allow_unsafe` | Resource: `read` paths, `run` executables, `env` vars, `network` |
| When it's checked | Statically at build time, plus a runtime sandbox | Runtime only |
| Reads | Not restricted by path. The filesystem is read-only, with home and temp directories hidden. | Allowlist of paths |
| Writes | Temp dir and `OUT_DIR`. Tests can add source-relative dirs (`bind_writable`, `make_writable`). | `OUT_DIR` only |
| Exec | No runtime allowlist. The `process` API gates static use. | `run` allowlist |
| Env | Cleared, with `pass_env` per package | `env` allowlist on top of the floor |
| Network | Denied. `allow_network` per build script, or globally for rustc. | `network` field, capped by `[limits]` |
| Package selection | `[pkg.NAME]`, by name | `[package.NAME]` by name, plus `[local]` / `[external]` categories with per-field inheritance |
| Global ceiling | None. Grants are per package. | `[limits]` clamps every resolved field |
| Paths | Concrete, relative to the package source | Symbolic roots, resolved per machine |
| Mechanism | Named in the config (`[sandbox] kind = "Bubblewrap"`) | Not part of the contract. Any backend can enforce it. |
| Build-script output | Not covered | Planned: [output verification](../future.md) |

## What Cackle covers that crabshell doesn't

- **Library code.** Static API checks catch a dependency that opens files or
  sockets in your binary or tests. crabshell explicitly leaves library code
  out of scope.
- **Proc macros.** Running rustc in a sandbox confines proc macros. The
  `host.runner` hook can't reach them.
- **`unsafe`.** Cackle gates which crates may use `unsafe` code.

These are complementary to crabshell rather than in conflict with it. A
project could use Cackle's static checks alongside crabshell's runtime policy.

## What crabshell's contract adds

- **Two-party policy.** The author's declaration documents intent and is
  reviewable. Intersecting it with the consumer's grant means an updated
  dependency can't widen its own access.
- **Read and exec allowlists.** Cackle hides a few directories. crabshell
  grants only what is listed.
- **Policy for packages you haven't named.** `[local]`, `[external]` and
  `[limits]` set rules for new dependencies before they arrive. In Cackle,
  each package is configured individually.
- **Portable paths.** Symbolic roots let one policy work across machines and
  toolchains.
- **Declared inputs.** Because a script's reads and env vars are listed, they
  can serve as a cache key.

## Why the prototype doesn't build on Cackle

- **The contract is what we're exploring.** Cackle's configuration has no
  read or exec allowlist, no author declarations, no categories and no global
  ceiling. Adding them would mean replacing its sandbox configuration model,
  which amounts to a fork.
- **Integration point.** Cackle runs the build under its own `cargo acl`
  command. crabshell targets Cargo's `host.runner` hook, so the result is
  something Cargo itself could adopt, and so the prototype can test that hook.
- **Backend independence.** Cackle's config names bubblewrap as the sandbox.
  crabshell keeps the backend out of the contract, so it can also use Landlock
  where namespaces aren't available, such as unprivileged CI containers.
- **Scope.** Static API analysis is a large, separate component, and it
  doesn't answer the question crabshell asks about what a build script may
  touch at runtime.
