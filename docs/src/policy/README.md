# Policy specification

> The consumer-facing `sandbox.toml` format. Expected to evolve independently
> of the core design.

- [Structure](structure.md): the tables and what each one matches.
- [Resolution](resolution.md): how a package's effective permissions are
  computed, and how list fields merge.
- [Paths and variables](paths.md): `:variables`, where their values come from,
  and how paths compare.
- [Named grants](grants.md): reusable permission sets, built in or defined in
  config.
- [Open questions](open-questions.md): known gaps and undecided defaults.

JSON Schemas for `sandbox.toml` and for the `[package.metadata.build-rs]`
table are in `crabshell/schema/`, generated from the types crabshell parses
with. Editors that use [Taplo](https://taplo.tamasfe.dev/) validate a file
that starts with a schema comment:

```toml
#:schema ./crabshell/schema/sandbox.schema.json
```
