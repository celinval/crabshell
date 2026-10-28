# Named grants

The primitive fields (`read`, `run`, `env`, `network`) are precise but
low-level: most `*-sys` crates need the same C-toolchain access. In the
[category experiments](../build-scripts/experiments.md), per-package grants
collapsed into a few sets with no crate-specific knowledge: `cfg-only`,
`rustc-probe`, and pkg-config plus the C toolchain. Naming those sets keeps
their definition in one audited place, instead of copy-pasted across
thousands of crates.

The [specification](../policy/grants.md) follows from three decisions:

- **Permissions, not requirements.** Many build scripts work, or fall back,
  without a given tool, so a crate *requests* grants it may use; it never
  requires them.
- **Static built-ins, overridable.** crabshell ships definitions for common
  sets, written in terms of [variables](../policy/paths.md). It doesn't run
  host tools to discover paths; when a default doesn't fit a machine (on
  Amazon Linux, `pkg-config` is a wrapper that runs `rpm`), user or project
  config overrides the grant or the variable.
- **Only consumers define.** Crates request grants by name. Defining their
  contents is reserved for user and project config, so a crate can't grant
  itself anything.
