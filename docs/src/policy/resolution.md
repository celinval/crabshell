# Resolution

Each field resolves independently. For a package `pkg` and field `f`, take the
value from the most specific table that sets `f`, then clamp by `[limits]`:

```text
value(pkg, f) = first table that sets f, in order:
    [package.pkg].f            # named entry — matches any package, local or external
    category(pkg).f            # [local].f if pkg's source is on the filesystem,
                               # [external].f if Cargo fetched it
    <default grant>.f          # crate-root read for `read`, else nothing

then, if [limits] sets f:  value = value ∩ [limits].f   # clamp
```

- **Per-field merge, not per-table replace.** A table overrides only the fields
  it names; every other field is inherited.
- **A named entry inherits from its category.** `[package.foo]` matches `foo`
  regardless of where its source came from; unset fields fall through to
  `[local]` or `[external]`, then the default grant.
- **`limits` clamps per field, with a warning — it does not fail the build.**
  When a resolved value exceeds a `limits` field, the runner reduces it to the
  cap and warns. This keeps a build working under a strong global ceiling even
  when a table requests more.
- **A package's declaration can only tighten, never widen.** The effective set is
  intersected with what the package declares, so a package can voluntarily reduce
  its access but never exceed the consumer's policy. This is the invariant that
  stops a compromised dependency update from escalating its own privileges.
  - Paths intersect by containment, so a declaration can select subpaths of a
    grant, including the default grant's crate root, but never reach outside
    one. That holds whatever variables the declaration uses.
  - Implicit access isn't part of resolution, so no declaration affects it.
- **Named grants expand first.** A consumer's `grants = [...]` and a crate's
  `requests = [...]` expand to their fields before clamping and narrowing; see
  [Named grants](grants.md).

## List fields

`read`, `run`, and `env` each accept an array or a table, following Cargo's
array-or-table idiom:

- an **array** — `read = ["$include_dir/openssl"]` — **overrides** the inherited
  value.
- a **table** — `read = { inherit = true, value = [...] }` — **appends**:
  `inherit = true` unions `value` on top of less-specific tables. `inherit =
  false` is equivalent to the array form.

`inherit` only adds entries; removing one is the job of `[limits]` or an override
array.
