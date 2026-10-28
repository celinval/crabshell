# Named grants

> **Status:** implemented in the prototype. The built-ins live in
> `crabshell/src/grants.toml` and still use the prototype's `$ROOT` path syntax.

A named grant is a reusable set of permissions spanning every field. Policies
assign it to packages by name:

```toml
[grant.pkg-config]
run  = ["pkg-config"]
read = [":pkgconfig_paths"]
env  = ["PKG_CONFIG*", "HOST_PKG_CONFIG*"]

[external]
grants = ["rustc", "pkg-config"]
```

- **Grant names** match `[a-z][a-z0-9_-]*`, so they can name tools like
  `pkg-config`. Unlike variables, they never appear inside a path.
- **Built-in grants** ship with crabshell (`rustc`, `cc` and `pkg-config` to
  start). They're static: they reference [variables](paths.md) and never run
  anything to discover paths.
- **User or project config overrides a built-in by name.** A `[grant.<name>]`
  table follows the per-field rules of every other table: an array replaces
  that field of the built-in; `{ inherit = true, value = [...] }` extends it.
- **A crate requests grants by name, never defines them.** In
  `[package.metadata.build-rs]`, `requests = ["pkg-config"]` narrows like any
  other declaration. The different key marks the direction of trust: the
  consumer *grants*, the crate *requests*. A crate that could define a grant's
  contents could grant itself anything.
- **Grants expand to fields before clamping and narrowing.** `[limits]` and
  declarations see only the expanded `read`, `write`, `run`, `env` and
  `network`.

A grant is a *permission* a build script may use, not a dependency: build
scripts commonly fall back to something else when a tool is missing. That's
why a crate *requests* grants rather than *requiring* them.
