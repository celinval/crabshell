# Structure

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

```text
error: `[package."*"]` is not supported
  = help: use `[external]` for dependencies, or `[local]` for workspace
          members and path dependencies
```

Rejecting rather than ignoring is deliberate. A silently-dropped
`[package."*"] network = false` reads like a restriction while granting
whatever the category table says.

There is no user-authored `baseline` table. The bottom of the chain is the
built-in **floor**, in two parts:

- **Structural**, which a build script can't run without, so nothing removes
  it:
  - write access to `OUT_DIR` and `/dev/null`,
  - a private, empty `TMPDIR`,
  - the `OUT_DIR` and `TMPDIR` values.
- **Default**: read access to the crate's own directory (`:manifest_dir`). A
  crate's declaration may narrow it to the files its build script reads
  (decided, not implemented yet: the prototype keeps the whole directory).

A build script gets nothing beyond the floor unless a table or its own
declaration grants it.

`TMPDIR` is `crabshell-tmp` in the build script's unit directory (next to
`OUT_DIR`), emptied at the start of every run. A shared `/tmp` would let build
scripts leave files for each other, and crabshell can't remove a temp dir
after the script exits because it replaces itself with the script.
