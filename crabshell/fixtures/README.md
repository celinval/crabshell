# Workspace fixtures

Real crates built through crabshell, to see whether the policy contract
works: how much crates would need to request, and how much consumers must
grant without their help.

Each folder is a standalone workspace with one crate that depends on crates
of one [category](../../docs/src/build-scripts/categories.md). `Cargo.lock` pins
every version; `rust-toolchain.toml` pins the nightly.

| Fixture | Build scripts that… |
|---|---|
| `cfg-only` | read Cargo's environment and emit cfgs |
| `rustc-probe` | also run `$RUSTC` to detect versions or features |
| `system-lib` | locate system libraries through pkg-config, and use the C toolchain |

## Scenarios

Every fixture builds in each cell of **requests × grants**:

| | default | tight | loose |
|---|---|---|---|
| **no requests** | fails | same as baseline | same as baseline |
| **precise requests** | fails | same as baseline | same as baseline |

- **Requests** stand in for the crates' own `[package.metadata.build-rs]`,
  since we can't edit them. `requests.toml` lists, for every build script,
  exactly what its code uses, from the vendor's point of view: no host paths,
  tools through [named grants](../../docs/src/policy/grants.md). They're
  injected with `CRABSHELL_REQUESTS`, which only the `test-requests` feature
  honors.
- **Grants** are the consumer's policy: the default grant alone (no config),
  `sandbox.tight.toml` (the union of all requests), or `sandbox.loose.toml`
  (the category's grants and any env var).

Precise × loose is the best case: everyone declares, and requests narrow a
broad policy to what's needed. No requests × tight is the worst case: the
consumer works out every crate's needs.

A scenario that works must also print the same `cargo:` directives as a
baseline build without crabshell, because a build script denied something
often degrades silently instead of failing. In `system-lib`, libz-sys prints
its whole environment, so the baseline and *no requests × loose* leak the fake
token the tests plant; every other scenario must not.

## Running

```sh
cargo test --features test-requests
```

Every build runs inside the `jail` helper from `probe`, with a fresh
`CARGO_HOME` and no network, so untrusted crates can't reach the host. See
`crabshell/fixtures.rs`.

To build a fixture by hand, build crabshell first (`.cargo/config.toml` runs
`../../../target/debug/crabshell`), then copy a scenario file to `sandbox.toml`
(ignored by git):

```sh
cargo build -p crabshell --features test-requests
cd crabshell/fixtures/cfg-only
cp sandbox.tight.toml sandbox.toml
cargo -Zhost-config -Ztarget-applies-to-host build
# with precise requests:
CRABSHELL_REQUESTS=$PWD/requests.toml cargo -Zhost-config -Ztarget-applies-to-host build
```

The `system-lib` policies extend the built-in `pkg-config` grant for Amazon
Linux, where `pkg-config` is a wrapper that runs `rpm`; other distros may need
different overrides.
