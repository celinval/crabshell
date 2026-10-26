# Related work

- [Package manager sandboxing](https://nesbitt.io/2026/09/24/package-manager-sandboxing.html)
  (Andrew Nesbitt, 2026) surveys how package managers gate, confine, or remove
  install-time code, and argues the open problem is validating a confined
  phase's output.
- [Cackle](https://github.com/cackle-rs/cackle) checks per-crate permissions and
  sandboxes build scripts, and is the closest prior art for Cargo. See
  [Comparison with Cackle](cackle.md).
- [rust-lang/cargo#16591](https://github.com/rust-lang/cargo/issues/16591), the
  `host.runner` hook crabshell builds on.
