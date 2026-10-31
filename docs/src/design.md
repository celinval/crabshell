# Design overview

## Guidelines

We follow these tenets to guide our design, in priority order:

1. *Security is non-negotiable.* No convenience or speedup may let a build
   script reach more than it was granted.
2. *Usability drives adoption.* Most build scripts should run with minimal
   annotations, and the policies people do write should survive updates and
   work across build environments.
3. *Precise inputs make builds efficient.* A script can only observe what it
   was granted, so its grants should be precise enough to serve as a sound
   cache key and a spec for remote execution.
4. *Confinement is free.* Sandboxing must not noticeably slow the build or
   need privileged setup.

## Approach

Three layers, each independently useful, that compose into an enforced sandbox
whose policy also names every input a build script can observe.

### 1. Cargo's `host.runner` hook (unstable)

We build on Cargo's accepted, unstable `host.runner` hook
([rust-lang/cargo#16591](https://github.com/rust-lang/cargo/issues/16591)),
which routes each compiled build script through a configured *runner* program
instead of executing it directly — same spirit as `rustc-wrapper`. The runner
gets the build-script executable plus the context Cargo already provides (env +
args), launches it, and Cargo consumes the script's `cargo::` directives from
stdout as usual. The contract with the rest of the build is unchanged.

Because the hook is unstable and meant for experimentation, we adopt it as-is
and propose small upstream refinements if needed, rather than forking. Cargo
stays policy-free — it decides *that* a script runs through a runner; the runner
decides *how* it's confined. (The hook covers build scripts only; proc-macros
can't be wrapped this way, since rustc loads them directly.)

### 2. A sandbox backend

The runner enforces the resolved permissions through a sandbox backend, for
example Landlock, namespaces (e.g. bubblewrap), or seccomp. The policy format
describes what a build script may do, not how it is enforced, so backends can
be swapped or combined without changing it. Platforms without a backend fall
back to no confinement.

The prototype's Linux backend uses Landlock. crabshell restricts its own
process and then execs the build script, which inherits the restriction:

- `read` grants read and execute, `write` grants write-only access, and
  `run` grants execute on one resolved program. A path that needs reading
  and writing is listed in both.
- Unless network is granted, crabshell also enters an empty network namespace
  (through an unprivileged user namespace). Landlock alone only covers TCP.
- Denied paths are visible but return `EACCES`; they aren't hidden as they
  would be with mount namespaces.

### 3. Declarative permissions

Two inputs feed the runner's policy.

**What the package requests** — in `Cargo.toml`:

```toml
[package.metadata.build-rs]
requests = ["pkg-config", "cc"]            # named grants it may use
read = [":manifest_dir"]                   # its own files
env  = ["OPENSSL_DIR", "OPENSSL_STATIC"]   # knobs it reads
```

Nothing needs listing for what a build script can't run without: its
[implicit access](policy/structure.md) (`OUT_DIR`, `/dev/null`, a private
`TMPDIR`, the dynamic loader and libc). When the policy says nothing, a
package is also granted read access to its own directory, the **default
grant**, which tables can replace and a request may narrow.
Everything else must be listed. Once a crate declares anything, unlisted
fields request nothing.

**What the consumer grants** — a top-level `sandbox.toml` sets, per package, what
any build script in the graph is actually allowed. The consumer is the upper
bound: a package's requests are *restrictive*, not permissive, and the effective
set is the intersection of the two. The `sandbox.toml` schema is in the
[policy specification](policy/README.md), kept separate because the policy
surface can evolve without touching the runner mechanism.

### Flow

```text
cargo build
  └─ compiles build.rs → build-script-build
  └─ for each build script (via host.runner):
       runner  ← Cargo invokes runner with the build-script exe + context
       policy  ← limits ∩ (named grant for pkg, else [local] / [external])
       runner applies the sandbox per policy
       runner execs build script inside the sandbox
       Cargo reads cargo:: directives from stdout as usual
```
