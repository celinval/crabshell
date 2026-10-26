# Future possibilities

- **Output verification.** Cargo acts on a build script's `cargo::` directives
  with full privileges. Since stdout passes through the runner, it could check
  them against policy. The main target is `rustc-link-arg`, which can run code
  at link time outside the sandbox.
- **Version-scoped grants.** `[package.foo]` also covers future versions of
  `foo`. Grants could optionally name a version or SEMVER constraint, with a
  lint warning on broad grants that do not.
