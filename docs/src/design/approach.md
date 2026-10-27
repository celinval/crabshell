# Approach

Three layers, each independently useful, that compose into an enforced sandbox.

## 1. Cargo's `host.runner` hook (unstable)

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

## 2. A sandbox backend

The runner enforces the resolved permissions through a sandbox backend, for
example Landlock, namespaces (e.g. bubblewrap), or seccomp. The policy format
describes what a build script may do, not how it is enforced, so backends can
be swapped or combined without changing it. Platforms without a backend fall
back to no confinement.

The prototype's Linux backend uses Landlock. crabshell restricts its own
process and then execs the build script, which inherits the restriction:

- `read` grants read and execute, `write` grants full access, and `run`
  grants execute on one resolved program.
- Unless network is granted, crabshell also enters an empty network namespace
  (through an unprivileged user namespace). Landlock alone only covers TCP.
- Denied paths are visible but return `EACCES`; they aren't hidden as they
  would be with mount namespaces.

## 3. Declarative permissions

Two inputs feed the runner's policy.

**What the package requests** — in `Cargo.toml`:

```toml
[package.metadata.build-rs]
read = ["/usr/include", "/usr/lib"]   # extra read-only paths
write = ["$HOME/.cache/foo"]          # extra writable paths
run  = ["cc", "pkg-config"]           # executables it may exec
env  = ["TARGET", "OPT_LEVEL"]        # env vars it may read
```

Only the [structural floor](../policy/structure.md) is implicit: the crate's own
directory (read), `OUT_DIR`, `/dev/null` and a private `TMPDIR` (write);
everything else must be listed. Once a crate declares anything, unlisted fields
request nothing.

**What the consumer grants** — a top-level `sandbox.toml` sets, per package, what
any build script in the graph is actually allowed. The consumer is the upper
bound: a package's requests are *restrictive*, not permissive, and the effective
set is the intersection of the two. The `sandbox.toml` schema is in the
[policy specification](../policy/README.md), kept separate because the policy
surface can evolve without touching the runner mechanism.

## Flow

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
