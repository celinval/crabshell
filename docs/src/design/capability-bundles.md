# Capability bundles

The primitive fields (`read`, `run`, `env`, `network`) are precise but low-level:
most `*-sys` crates hand-list the same C-toolchain access. A planned higher layer
lets a build script declare *named capabilities* instead — reusable bundles that
expand to primitives — e.g. `requires = ["cc_toolchain", "system-utils"]`, where
`cc_toolchain` means "run the C compiler and linker and read the toolchain's
include/lib roots," resolved per target. This keeps the definition in one audited
place rather than copy-pasted across thousands of crates.

Capabilities are syntax sugar: a `requires` entry expands to primitives *before*
clamping and merging. The format and standard set are TBD.
[Proposed categories](../build-scripts/categories.md) sketches a candidate set
from observed build scripts.
