# General guidelines

The code in this repo is currently a prototype mostly developed by AI. It is meant to
guide design decisions and sandbox specification. During implementation, it is important to capture
decisions that may affect end-to-end flow, expressivity and enforceability of the specification in our
documentation `docs`

### Correctness over convenience

- Model the full error space. No shortcuts or simplified error handling.
- Use the type system to encode correctness constraints.
- Prefer compile-time guarantees over runtime checks where possible.

### Prototype-grade engineering

- Prefer examples and end-to-end tests at this point.
- Unit tests should be used, but mostly to ensure implementation / corner case
  handling is correct.
- Pay attention to what test facilities already exist and reuse them.
- Capturing trade-offs is really imporatnt.

### Documentation (docs/ and README.md)

- Keep documentation *concise* and up-to-date.
- Documentation in markdown files are written for human consumption! And human time matters.
- Provide relevant information without overwhelming the reader — don't repeat yourself.
- Use examples when it makes understanding easier.

### Code documentation
- Doc comments (`///`) must start with a brief one-line summary. This first line is the title — keep it short and descriptive.
- Use inline comments to explain "why," not just "what."
- Don't add narrative comments in function bodies. Only comment what is non-obvious or needs a deeper "why" explanation.
  - Consider pulling the code to a separate function if it becomes too complex.
- Module-level documentation (`//!`) should explain purpose and responsibilities.
- Always use periods at the end of code comments.
