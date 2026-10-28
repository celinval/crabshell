# Paths and variables

> **Status:** decided, not implemented yet. The prototype still expands
> `$NAME` from the environment; see [Prototype today](#prototype-today).

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
| `:cargo_home` | `CARGO_HOME` |
| `:sysroot` | the toolchain root, derived from `RUSTC`'s location |

The mapping is a fixed list, not every `CARGO_*` variable, so new Cargo
variables don't silently become paths.

Built-in [grants](grants.md) may also come with variables of their own, such as
`:pkgconfig_paths`.

## User-defined variables

User and project config define variables in `[variables]`. A variable is
either a single path, or a **path list** that expands to several entries:

```toml
[variables]
openssl_dir     = [{ env = "OPENSSL_DIR" }, "/usr"]          # first defined wins
pkgconfig_paths = [{ env = "PKG_CONFIG_LIBDIR" }, "/usr/lib64/pkgconfig", "/usr/share/pkgconfig"]
```

- A definition may import an environment variable with `{ env = "NAME" }`.
  An env value is split on `:` where the variable is a path list. Policy paths
  themselves never expand the environment, so every env dependency is one
  `env = "..."` key.
- An unset env var leaves that alternative undefined. If no alternative is
  defined, the variable is undefined, and grants that use it are dropped (and
  reported under `CRABSHELL_LOG`). For optional knobs that's the right outcome:
  unset means "use the default".
- **Only user-supplied values may be imported.** Some of crabshell's
  environment is chosen by crates:
  - `DEP_<links>_*` comes from a dependency's build script;
  - `CARGO_PKG_*` and `CARGO_MANIFEST_LINKS` come from the crate's own
    manifest.

  Definitions that import those are rejected when the config loads. The check
  is by name; see [open questions](open-questions.md) for the upstream fix.
- Crates can't define variables, only use them in declarations.

crabshell doesn't run host tools to discover paths. Built-in defaults are
static data; when they're wrong for a machine, the user sets the variable.

## Globs

A trailing `*` in the last component matches every sibling with that prefix
(`/lib*`). A glob under a missing directory matches nothing; a directory that
exists but can't be listed is an error. `env` entries also accept a trailing
`*` (`CARGO_CFG_*`).

## Comparing paths

Clamping and narrowing compare paths by **containment**: a declared
`:manifest_dir/build` is inside a granted `:manifest_dir`. That's what lets a
crate narrow its read access to a few files. (The prototype still compares
paths as strings, so today `/usr` doesn't cover `/usr/include`.)

## Prototype today

- An absolute path, or `$NAME/sub/path` where `NAME` is one of
  `CARGO_MANIFEST_DIR`, `OUT_DIR`, `CARGO_HOME`, `HOME` or `RUSTUP_HOME`, read
  from the build script's environment.
- Globs and `env` patterns as above. A pattern covers every entry with its
  prefix when clamping and narrowing.
