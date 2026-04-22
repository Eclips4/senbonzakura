Don't write unnecessary comments. Only comment non-obvious design decisions.

Use `uv` for Python, not `pip`.

Use `cargo test --workspace` to verify changes.

The Lean proof in `proof/` must build with `cd proof && lake build` with zero sorry, zero warnings.

Rebuild the LSP after any core changes: `cargo build --release -p senbonzakura-lsp`.
