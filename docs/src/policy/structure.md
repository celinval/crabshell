# Structure

`sandbox.toml` is resolved **per field**, in the spirit of Cargo's `[profile]`
inheritance: a table sets only the fields it cares about; unset fields fall
through to a less-specific table, ultimately to the built-in default grant. Three kinds
of table:

```toml
# Consumer caps. Constrains ONLY the fields it names; unnamed fields are uncapped.
[limits]
network = false

# Packages whose source is on your filesystem: workspace members and path
# dependencies. Inherits unset fields from the default grant.
[local]
read = [":include_dir", ":lib_dir", ":manifest_dir"]

# Packages Cargo fetched: registry and git dependencies.
[external]
read = [":include_dir", ":lib_dir"]

# One named package, local or external. Inherits from its category table, then
# the default grant.
[package.openssl-sys]
read = { inherit = true, value = [":include_dir/openssl"] }   # append to inherited read paths
run  = ["cc", "pkg-config"]                                   # override run

# User-defined variables used above; see Paths and variables.
[variables]
include_dir = "/usr/include"
lib_dir = "/usr/lib64"
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

```text
error: `[package."*"]` is not supported
  = help: use `[external]` for dependencies, or `[local]` for workspace
          members and path dependencies
```

Rejecting rather than ignoring is deliberate. A silently-dropped
`[package."*"] network = false` reads like a restriction while granting
whatever the category table says.

Beneath the tables, two built-ins apply to every build script:

- **Implicit access** is what a build script can't run without. It isn't
  policy: no table grants it, and neither `[limits]` nor a request removes it.
  - write access to `OUT_DIR` and `/dev/null`;
  - read access to the dynamic loader and the libraries every std build script
    links: on `x86_64-unknown-linux-gnu`, `ld-linux-x86-64.so.2`, `libc.so.6`
    and `libgcc_s.so.1`. A build script is always a host program built with
    std, even for a `no_std` crate. Their locations are per-host defaults for
    now, which will become overridable variables. Statically linked hosts
    (musl) need none;
  - a private, empty `TMPDIR`;
  - the `OUT_DIR` and `TMPDIR` values.
- **The default grant** is the bottom of the grant chain: what a field
  resolves to when no table sets it:
  - `read`: the crate's own directory (`:manifest_dir`). Cargo runs a build
    script inside it, so relative paths reach it without the
    `CARGO_MANIFEST_DIR` variable.
  - `env`: `TARGET`, `HOST`, `CARGO_CFG_*` and `CARGO_FEATURE_*`, facts
    about the build with no secrets or machine-specific paths, which would
    also leak into the cache key. Variables holding absolute paths
    (`CARGO_MANIFEST_DIR`), per-profile settings (`OPT_LEVEL`) or package
    metadata (`CARGO_PKG_*`) are requested explicitly. `DEP_<links>_*` too: its keys are
    open-ended and its values come from other crates' build scripts, so a
    crate requests the ones for the `links` dependencies it consumes.
  - other fields: nothing.

  Like any grant, an array replaces it, `inherit = true` extends it,
  `[limits]` clamps it, and a request narrows it. Until paths compare by
  containment, a crate that reads its own files requests the whole crate root.

A build script gets nothing beyond these unless a table or its own
declaration grants it.

`TMPDIR` is `crabshell-tmp` in the build script's unit directory (next to
`OUT_DIR`), emptied at the start of every run. A shared `/tmp` would let build
scripts leave files for each other, and crabshell can't remove a temp dir
after the script exits because it replaces itself with the script.
