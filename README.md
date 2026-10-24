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

### What it doesn't cover

crabshell limits the damage a build script can do while it runs. It is not a
guarantee against untrusted code:

- **Library code** compiled from a dependency runs unconfined in your tests
  and binaries.
- **Proc macros** run inside rustc and cannot be wrapped by the runner.
- **Build outputs** are trusted by Cargo. A confined script can still write
  code into `OUT_DIR` that gets `include!`d, or emit linker directives (see
  [Future possibilities](#future-possibilities)).

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

[local]                               # workspace members and path dependencies
read = ["$include_dir", "$lib_dir", "$CARGO_MANIFEST_DIR"]

[external]                            # registry and git dependencies
read = ["$include_dir", "$lib_dir"]
```

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

By default a build script gets almost nothing: it can write to `OUT_DIR`, and
that's it. Everything else is opt-in.

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

### 2. A sandbox backend

The runner enforces the resolved permissions through a sandbox backend, for
example Landlock, namespaces (e.g. bubblewrap), or seccomp. The policy format
describes what a build script may do, not how it is enforced, so backends can
be swapped or combined without changing it. Platforms without a backend fall
back to no confinement.

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
set is the intersection of the two. The `sandbox.toml` schema is in the
[policy specification](#sandboxtoml-policy-specification), kept separate because
the policy surface can evolve without touching the runner mechanism.

### Flow

```
cargo build
  └─ compiles build.rs → build-script-build
  └─ for each build script (via host.runner):
       runner  ← Cargo invokes runner with the build-script exe + context
       policy  ← limits ∩ (named grant for pkg, else [local] / [external])
       runner applies the sandbox per policy
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

# Packages whose source is on your filesystem: workspace members and path
# dependencies. Inherits unset fields from the built-in baseline.
[local]
read = ["$include_dir", "$lib_dir", "$CARGO_MANIFEST_DIR"]

# Packages Cargo fetched: registry and git dependencies.
[external]
read = ["$include_dir", "$lib_dir"]

# One named package, local or external. Inherits from its category table, then
# the baseline.
[package.openssl-sys]
read = { inherit = true, value = ["$include_dir/openssl"] }   # append to inherited read paths
run  = ["cc", "pkg-config"]                                   # override run
```

A package is local when its manifest directory is outside `CARGO_HOME`, which
crabshell reads from the build script's environment. This mirrors the
distinction Cargo itself uses for `--cap-lints` and rebuild detection: the
question is whether the source is yours, not whether it is in your member
list.

Note this is deliberately *not* Cargo's `[profile.*.package."*"]` semantics,
where `*` means "every package except workspace members" — and so matches a
path dependency outside the workspace. Using that split here would make such a
package both `[local]` and part of the catch-all. To keep the categories
disjoint, crabshell names them explicitly and **rejects** glob keys under
`[package]`:

```
error: `[package."*"]` is not supported
  = help: use `[external]` for dependencies, or `[local]` for workspace
          members and path dependencies
```

Rejecting rather than ignoring is deliberate. A silently-dropped
`[package."*"] network = false` reads like a restriction while granting
whatever the category table says.

There is no user-authored `baseline` table. The bottom of the chain is the
built-in **structural floor** — `OUT_DIR` write access and the `OUT_DIR` path
value, nothing else. A build script gets nothing beyond the floor unless a table
or its own declaration grants it.

### Resolution

Each field resolves independently. For a package `pkg` and field `f`, take the
value from the most specific table that sets `f`, then clamp by `[limits]`:

```
value(pkg, f) = first table that sets f, in order:
    [package.pkg].f            # named entry — matches any package, local or external
    category(pkg).f            # [local].f if pkg's source is on the filesystem,
                               # [external].f if Cargo fetched it
    <structural floor>.f       # OUT_DIR write + OUT_DIR path only

then, if [limits] sets f:  value = value ∩ [limits].f   # clamp
```

- **Per-field merge, not per-table replace.** A table overrides only the fields
  it names; every other field is inherited.
- **A named entry inherits from its category.** `[package.foo]` matches `foo`
  regardless of where its source came from; unset fields fall through to
  `[local]` or `[external]`, then the baseline.
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
- **Glob patterns.** Deferred, and rejected for now. Because the categories are
  named tables rather than a glob, patterns like `*-sys` can be added later
  without competing with a catch-all for precedence.
- **Vendored dependencies are a known gap.** `cargo vendor` puts third-party
  source inside your project, so the `CARGO_HOME` test classifies it as
  `[local]`. The environment alone cannot distinguish a vendored directory from
  a path dependency; resolving this needs the manifest or `cargo metadata`.
- **What should the default policy grant?** Real build scripts read more than
  `OUT_DIR`: `serde_core` needs `CARGO_PKG_VERSION_PATCH` and `RUSTC`, for
  instance. The default will likely include the variables Cargo sets for build
  scripts, as an explicit list rather than a `CARGO_*` prefix (which would also
  match secrets like `CARGO_REGISTRY_TOKEN`), and read access to the crate's
  own directory. Whether those paths are exposed as relative paths is open.

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

## Future possibilities

- **Output verification.** Cargo acts on a build script's `cargo::` directives
  with full privileges. Since stdout passes through the runner, it could check
  them against policy. The main target is `rustc-link-arg`, which can run code
  at link time outside the sandbox.
- **Version-scoped grants.** `[package.foo]` also covers future versions of
  `foo`. Grants could optionally name a version or SEMVER constraint, with a
  lint warning on broad grants that do not.

## Related work

- [Package manager sandboxing](https://nesbitt.io/2026/09/24/package-manager-sandboxing.html)
  (Andrew Nesbitt, 2026) surveys how package managers gate, confine, or remove
  install-time code, and argues the open problem is validating a confined
  phase's output.
- [Cackle](https://github.com/cackle-rs/cackle) checks per-crate permissions and
  sandboxes build scripts, and is the closest prior art for Cargo.
- [rust-lang/cargo#16591](https://github.com/rust-lang/cargo/issues/16591), the
  `host.runner` hook crabshell builds on.
