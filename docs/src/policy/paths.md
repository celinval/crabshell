# Paths and variables

A policy path is an absolute path, or a **variable** plus a subpath:
`:manifest_dir/build/grammar.lalrpop`. The leading `:` keeps variables apart
from environment variables; policy paths never read the environment
directly.

Variable names match `[a-z][a-z0-9_]*`, and a name ends at `/` or the end of
the entry. Being lowercase also sets them apart from env vars
(`:openssl_dir` vs `OPENSSL_DIR`). Any other name is a config error.

## Built-in variables

crabshell derives these from what Cargo tells the build script:

| Variable | Value |
|---|---|
| `:manifest_dir` | the crate root (`CARGO_MANIFEST_DIR`) |
| `:manifest_path` | the crate's `Cargo.toml` (`CARGO_MANIFEST_PATH`) |
| `:out_dir` | `OUT_DIR` |
| `:tmp_dir` | the private `TMPDIR` crabshell creates |
| `:cargo_home` | `CARGO_HOME` |
| `:sysroot` | the toolchain root, derived from `RUSTC`'s location |

The mapping is a fixed list, not every `CARGO_*` variable, so new Cargo
variables don't silently become paths.

Built-ins are shortcuts, not reserved names: they save a policy from spelling
out a path the environment already holds, and nothing enforces them. A config
may redefine one, and a declaration's own names shadow one (see below).
Implicit access doesn't go through them — the runner reads `OUT_DIR` and
builds `TMPDIR` directly — so redefining `:out_dir` can't change what a build
script may write.

Built-in [grants](grants.md) may also come with variables of their own, such as
`:pkgconfig_paths`.

## User-defined variables

User and project config define variables in `[variables]`. A variable's
value is a **list of paths**, and `:name/rest` expands to `path/rest` for each
of them:

```toml
[variables]
lib_dir     = ["/usr/lib64", "/usr/lib/x86_64-linux-gnu"]              # union
pc_path     = [{ env = "PKG_CONFIG_PATH" }, "/usr/lib64/pkgconfig"]    # extends
openssl_dir = { env = "OPENSSL_DIR", default = "/usr" }                # fallback
```

- A definition is a source or an array of sources, whose paths are unioned.
  Paths that don't exist on a machine are harmless (Landlock skips them), so
  one definition can cover several distros' layouts.
- A source is an absolute path, or `{ env = "NAME" }`, which imports an
  environment variable. Policy paths themselves never expand the environment,
  so every env dependency is one `env = "..."` key.
- An imported value is read as a path list, like `PATH`: a single path is a
  one-element list. A path that contains `:` can't be imported.
- `default` applies when the env var is unset or empty, and takes the same
  forms as a definition.
- A variable none of whose sources is defined is undefined, and grants that
  use it are dropped (and reported under `CRABSHELL_LOG`). For optional knobs
  that's the right outcome: unset means "use the default".
- **Only user-supplied values may be imported.** Some of crabshell's
  environment is chosen by crates:
  - `DEP_<links>_*` comes from a dependency's build script;
  - `CARGO_PKG_*` and `CARGO_MANIFEST_LINKS` come from the crate's own
    manifest.

  Definitions that import those are rejected when the config loads. The check
  is by name; see [open questions](open-questions.md) for the upstream fix.

crabshell doesn't run host tools to discover paths. Built-in defaults are
static data; when they're wrong for a machine, the user sets the variable.

## Variables in a declaration

A crate can't define `[variables]`, and never sees the consumer's: a
declaration resolves against the built-ins plus names of its own. That keeps a
published declaration from depending on what a consumer happens to call
things, so an unknown variable in a declaration is always the crate author's
own mistake.

Its own names come from the environment variables it requests, lowercased:

```toml
[package.metadata.crabshell]
env  = ["OPENSSL_DIR"]
read = [":openssl_dir/include", ":openssl_dir/lib"]
```

- This is how a crate declares a path it can't know the value of. Without it a
  declaration could only name a fixed guess, and a consumer who set the knob
  and granted the real directory would lose it to narrowing.
- A name of its own **shadows a built-in** of the same name, like any inner
  scope. `SYSROOT` is a real C-toolchain knob, and a crate requesting it means
  that sysroot, not the one crabshell derives from `RUSTC`. `CRABSHELL_LOG`
  reports a shadowed built-in.
- The value can't widen anything: a declaration only narrows, so whatever the
  variable holds, the result stays inside what the consumer granted.
- An unset variable is undefined, so entries using it drop — right for a knob
  whose absence means "use the default".

## Globs

Path entries may be glob patterns, in the syntax of the
[`glob`](https://docs.rs/glob) crate: `*` and `?` within a component, `[...]`
character classes, and `**` as a whole component for any depth:

```toml
read = [":manifest_dir/xz-*", ":manifest_dir/**/probe.rs"]
```

- Patterns expand when the build script starts, so files created afterwards
  aren't covered.
- A matched directory grants its whole subtree; a trailing `dir/**` is the same
  as `dir`.
- A pattern under a missing directory matches nothing; a directory that exists
  but can't be listed is an error, since silently granting less would hide
  the cause.
- `env` entries use the same syntax, matched against the variables actually
  set: `CARGO_CFG_*`.

## Comparing paths

Every entry expands to concrete values before anything is compared:
variables and globs to paths, env patterns to names. Clamping and narrowing
then compare **concrete values**: a path covers the paths under it, at a `/`
boundary, and env names compare by equality. So a request for
`:manifest_dir/build/probe.rs` is inside the granted `:manifest_dir`, and an
absolute `[limits]` cap applies to grants written with variables.
