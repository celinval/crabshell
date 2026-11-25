# Threat model

> [!NOTE]
> This page describes the protection crabshell aims for,
> not what the prototype enforces today.
> Where the prototype falls short is tracked in
> [open questions](policy/open-questions.md).

This page defines whom crabshell trusts and what it protects.
Every grant, default and fallback in the policy should hold up against it.

## Attacker

The attacker controls a dependency:
its source, its build script and its declaration,
and can publish new versions of it.
Whatever its build script reads can reach them:
the script can always write `OUT_DIR` and print `cargo::rustc-env=` directives,
and the crate compiles either into the artifact.
They don't control Cargo, the toolchain,
the user's config or the project's policy.

## Trust

- **Trusted to enforce:**
  the kernel and the sandbox backend, the crabshell runner,
  and Cargo, whose environment names the paths a policy refers to
  and decides whether a package is local.
- **Trusted to grant:**
  the user's config and the project's policy
  (see [policy files](policy/structure.md#policy-files)).
  Neither removes [implicit access](policy/structure.md#tables).
- **Untrusted:**
  every build script, local or fetched,
  and its declaration, which [can only narrow](policy/resolution.md) a grant.
  `[local]` selects a table, not a trust level:
  vendored crates, path `[patch]` overrides and git submodules are local
  but third-party.

## Scope

crabshell confines each build script while it runs.
Everything else is out of scope:

- **The compile and link steps.**
  rustc reads whatever a crate's source names (`include_bytes!`, `env!`),
  and Cargo acts on a script's `cargo::` directives,
  including `rustc-link-arg`, which can run code at link time.
- **Proc-macros**, which run inside rustc, where `host.runner` can't wrap them.
- **The built artifact** at runtime.
- **Resource exhaustion** (CPU, memory, disk).
- **Hosts without a [sandbox backend](design.md#2-a-sandbox-backend)**,
  which run build scripts unconfined.
- **Learning which paths exist**:
  denied paths are visible
  (see [the Linux backend](design.md#2-a-sandbox-backend)).

## What is protected

Beyond its [implicit access](policy/structure.md#tables)
and its [effective policy grant](policy/resolution.md) (including any
[default grant](#defaults)), a build script must not:

- read host files, environment variables (e.g. `CARGO_REGISTRY_TOKEN`),
  or other packages' sources and outputs;
- modify host files, the source tree, or other packages' `OUT_DIR`;
- reach the network, or local services that do.

## Defaults

What every script gets without asking,
its implicit access and the default grant,
may expose only what the attacker already has:
the system libraries every program loads,
the package's own source,
and build facts that hold no secrets.
