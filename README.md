# CrabShell

**A sandbox for Cargo build scripts.** `crabshell` runs untrusted `build.rs`
code with only the access it actually needs — not the full privileges of
whoever ran `cargo build`.

> ⚠️ **Prototype.** Exploratory work; the API, permission format, and
> confinement mechanism will change. Not for production use.

## Why

Many crates ship a build script (`build.rs`) that Cargo compiles and runs during
`cargo build` — to probe for system libraries, generate code, compile vendored
C. But they run with **all of your privileges**. Any dependency in your graph,
including a transitive one pulled in by a routine `cargo update`, can silently
read your credentials (`~/.aws`, `~/.ssh`, env vars), tamper with files anywhere,
or reach the network — before you run a single test. Your build is only as
trustworthy as the least-trustworthy `build.rs` in the tree, with no way to
contain a bad one.

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

A bonus: once inputs are declared, a build script's result becomes safely
cacheable and remote-executable — something Cargo can't do reliably today.

## Usage sketch

A crate author declares needs in `Cargo.toml`:

```toml
[package.metadata.build-rs]
read = ["/usr/include", "/usr/lib"]   # extra read-only paths
run  = ["cc", "pkg-config"]           # executables it may run
env  = ["TARGET", "OPT_LEVEL"]        # env vars it may read
```

You set the project policy in `sandbox.toml`:

```toml
[limits]                              # a hard ceiling nothing can exceed
network = false

[workspace]                           # what your own crates get
read = ["$include_dir", "$lib_dir", "$CARGO_MANIFEST_DIR"]

[package."*"]                         # default for external dependencies
read = ["$include_dir", "$lib_dir"]
```

You add `crabshell` as a host runner in your Cargo `config.toml`.

```toml
[host]
runner = ["crabshell"]
```

By default a build script gets almost nothing: it can write to `OUT_DIR`, and
that's it. Everything else is opt-in.

Similar to Cargo, CrabShell will follow your directory hierarchy to find sandbox
configurations. This allows you to set different policies for different parts of
your workspace. CrabShell also allows system users to set default limits on resource usage.

---

# Prototype design

## Approach

Three layers, each independently useful, that compose into an enforced sandbox.

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

### 2. A default namespace-based runner (Linux)

A reference runner that confines a build script using unprivileged Linux
namespaces, no root required:

- **Filesystem**: a mount namespace with a read-only view of allowed system
  paths, a writable bind for `OUT_DIR` and the target dir, and nothing else.
  Package source is read-only by default; anything unlisted is absent, not just
  denied.
- **Network**: a network namespace with no interfaces (loopback only) unless
  network access is granted.
- **PID / IPC / user**: isolated PID and IPC namespaces; a user namespace to drop
  privileges without elevated rights.
- **Syscalls**: an optional seccomp-bpf filter to deny ungranted syscall classes
  (e.g. raw sockets, `ptrace`).

The runner translates the effective permission set into concrete namespace,
bind-mount, and seccomp config. Non-Linux platforms fall back to no confinement
initially, with the interface left open for backends like macOS sandbox
profiles later.

### 3. Declarative permissions

Two inputs feed the runner's policy.

**What the package requests** — in `Cargo.toml`:

```toml
[package.metadata.build-rs]
read = ["/usr/include", "/usr/lib"]   # extra read-only paths
run  = ["cc", "pkg-config"]           # executables it may exec
env  = ["TARGET", "OPT_LEVEL"]        # env vars it may read
```

Only `OUT_DIR` (write) and its path are implicit; everything else must be listed.

**What the consumer grants** — a top-level `sandbox.toml` sets, per package, what
any build script in the graph is actually allowed. The consumer is the upper
bound: a package's requests are *restrictive*, not permissive, and the effective
set is the intersection of the two. The `sandbox.toml` schema is in
[Appendix A](#appendix-a-sandboxtoml-policy-specification), kept separate because
the policy surface can evolve without touching the runner mechanism.

### Flow

```
cargo build
  └─ compiles build.rs → build-script-build
  └─ for each build script (via host.runner):
       runner  ← Cargo invokes runner with the build-script exe + context
       policy  ← limits(pkg) ∩ (sandbox.toml grant for pkg, else catch-all)
       runner sets up namespaces / mounts / seccomp per policy
       runner execs build script inside the sandbox
       Cargo reads cargo:: directives from stdout as usual
```

## `sandbox.toml` policy specification

> The consumer-facing policy format. Expected to evolve independently of the core
> design.

### Structure

`sandbox.toml` is resolved **per field**, in the spirit of Cargo's `[profile]`
inheritance: a table sets only the fields it cares about; unset fields fall
through to a less-specific table, ultimately to a built-in baseline. Three kinds
of table:

```toml
# Consumer caps. Constrains ONLY the fields it names; unnamed fields are uncapped.
[limits]
network = false

# Policy for workspace members. Inherits unset fields from the built-in baseline.
[workspace]
read = ["$include_dir", "$lib_dir", "$CARGO_MANIFEST_DIR"]

# External dependencies with no named entry ("*" excludes workspace members).
[package."*"]
read = ["$include_dir", "$lib_dir"]

# One named package (workspace member or external). Inherits from its category
# table, then the baseline.
[package.openssl-sys]
read = { inherit = true, value = ["$include_dir/openssl"] }   # append to inherited read paths
run  = ["cc", "pkg-config"]                                   # override run
```

There is no user-authored `baseline` table. The bottom of the chain is the
built-in **structural floor** — `OUT_DIR` write access and the `OUT_DIR` path
value, nothing else. A build script gets nothing beyond the floor unless a table
or its own declaration grants it.

### Resolution

Each field resolves independently. For a package `pkg` and field `f`, take the
value from the most specific table that sets `f`, then clamp by `[limits]`:

```
value(pkg, f) = first table that sets f, in order:
    [package.pkg].f            # named entry — matches any package, workspace or external
    category(pkg).f            # [workspace].f if pkg is a workspace member,
                               # [package."*"].f if pkg is an external dependency
    <structural floor>.f       # OUT_DIR write + OUT_DIR path only

then, if [limits] sets f:  value = value ∩ [limits].f   # clamp
```

- **Per-field merge, not per-table replace.** A table overrides only the fields
  it names; every other field is inherited.
- **A named entry inherits from its category.** `[package.foo]` matches `foo`
  regardless of workspace membership; unset fields fall through to `[workspace]`
  or `[package."*"]`, then the baseline.
- **`limits` clamps per field, with a warning — it does not fail the build.**
  When a resolved value exceeds a `limits` field, the runner reduces it to the
  cap and warns. This keeps a build working under a strong global ceiling even
  when a table requests more.
- **A package's declaration can only tighten, never widen.** The effective set is
  intersected with what the package declares, so a package can voluntarily reduce
  its access but never exceed the consumer's policy. This is the invariant that
  stops a compromised dependency update from escalating its own privileges.

### List fields

`read`, `run`, and `env` each accept an array or a table, following Cargo's
array-or-table idiom:

- an **array** — `read = ["$include_dir/openssl"]` — **overrides** the inherited
  value.
- a **table** — `read = { inherit = true, value = [...] }` — **appends**:
  `inherit = true` unions `value` on top of less-specific tables. `inherit =
  false` is equivalent to the array form.

`inherit` only adds entries; removing one is the job of `[limits]` or an override
array.

### Paths

Policy paths are **symbolic roots plus root-relative subpaths**, not concrete
machine paths. A symbolic root (e.g. `$include_dir`, `$lib_dir`, `$sysroot`,
`$OUT_DIR`, `$CARGO_MANIFEST_DIR`, `$TARGET_DIR`) names a *role*; the runner
resolves it per machine at execution time. Which roots exist and how
toolchain-derived ones resolve still needs refinement.

### Notes

- **Declared inputs are the cache key.** The contents of declared `read` paths and
  declared `env` vars are the build script's inputs. Enabling `network` disables
  the cache.
- **Glob patterns.** Only the `*` catch-all is specified; it matches external
  dependencies only. Broader patterns (`openssl-*`, `*-sys`) are deferred.

## Capability bundles

The primitive fields (`read`, `run`, `env`, `network`) are precise but low-level:
most `*-sys` crates hand-list the same C-toolchain access. A planned higher layer
lets a build script declare *named capabilities* instead — reusable bundles that
expand to primitives — e.g. `requires = ["cc_toolchain", "system-utils"]`, where
`cc_toolchain` means "run the C compiler and linker and read the toolchain's
include/lib roots," resolved per target. This keeps the definition in one audited
place rather than copy-pasted across thousands of crates.

Capabilities are syntax sugar: a `requires` entry expands to primitives *before*
clamping and merging. The format and standard set are TBD.
