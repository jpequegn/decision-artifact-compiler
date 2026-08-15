# Decision Artifact Compiler

Compile approved, versioned Markdown decisions into typed agent task graphs with explicit context,
authority, budgets, gates, acceptance evidence, and reviewable reconciliation.

This repository implements [project-ideas #235](https://github.com/jpequegn/project-ideas/issues/235).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p artifact-cli -- --help
```

## Status

The workspace is being implemented through repository issues. The initial scaffold defines the
quality gates and command surface; domain behavior follows in later issues.

## License

MIT
