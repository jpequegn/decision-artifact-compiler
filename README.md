# Decision Artifact Compiler

Decision Artifact Compiler turns approved, versioned Markdown decisions into typed task DAGs with
explicit context, least-privilege authority, budgets, gates, evidence, receipts, and review-only
reconciliation. It provides a Rust CLI/runtime and a local TypeScript review workspace backed by the
same validation and semantic-diff core through WebAssembly.

This repository implements [project-ideas #235](https://github.com/jpequegn/project-ideas/issues/235).

## Capabilities

- Parse source-linked Markdown task declarations into a canonical typed IR.
- Fail closed on broken references, dependency cycles, broadened authority, invalid budgets, and
  missing consequential-action gates.
- Export canonical JSON, a human compile report, and a runner-neutral DAG plan.
- Dispatch deterministic or custom provider-neutral workers under dependency and concurrency limits.
- Persist hash-chained SQLite receipts and replay terminal state without worker calls.
- Validate immutable result envelopes and propose conflict-aware Markdown reconciliation patches.
- Review objectives, DAGs, authority, budgets, gates, evidence, diagnostics, diffs, and immutable
  digests in a responsive local browser workspace.

## Prerequisites

- Current stable Rust with `rustfmt`, `clippy`, and `wasm32-unknown-unknown`.
- Node.js 22 or newer and npm.
- `jq` only for extracting the run ID in the shell walkthrough below.

## Quick Start

```bash
git clone https://github.com/jpequegn/decision-artifact-compiler.git
cd decision-artifact-compiler
cargo run -p artifact-cli -- validate examples/repository-change.md
cargo run -p artifact-cli -- compile examples/repository-change.md --format report
```

Run, replay, and reconcile the deterministic example:

```bash
cargo run -q -p artifact-cli -- dispatch examples/repository-change.md \
  --ledger /tmp/artifact-runs.db --results /tmp/results.json > /tmp/run.json

RUN_ID="$(jq -r .run_id /tmp/run.json)"
cargo run -q -p artifact-cli -- replay "$RUN_ID" --ledger /tmp/artifact-runs.db
cargo run -q -p artifact-cli -- reconcile examples/repository-change.md \
  /tmp/results.json --output /tmp/reconciliation.md
```

The deterministic worker is a safe demonstration adapter. It emits structured results and
acceptance receipts but does not execute repository commands or call an AI provider.

## Review Workspace

```bash
cd web
npm ci
npm run dev
```

Open [http://127.0.0.1:5173](http://127.0.0.1:5173). The checked-in WASM asset makes validation and
diffing local and network-independent. Rebuild it after changing `artifact-core` or `artifact-wasm`:

```bash
cd web
npm run wasm
npm run build
```

## Evaluation

```bash
cargo run -p artifact-eval -- --output-dir reports
cargo run -p artifact-eval -- --output-dir reports --check
```

The checked-in [evaluation report](reports/evaluation.md) covers the 40-case golden corpus,
adversarial policy boundaries, conflict detection, replay parity, source-linked reviewability, and
context duplication against a chat-style baseline.

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Artifact and export formats](docs/FORMATS.md)
- [Security model](docs/SECURITY.md)
- [Evaluation methodology](docs/EVALUATION.md)
- [Usage patterns and extensions](docs/USAGE_AND_EXTENSIONS.md)
- [Troubleshooting](docs/TROUBLESHOOTING.md)
- [Project status](PROJECT_STATUS.md) and [release checklist](RELEASE_CHECKLIST.md)

## Development

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p artifact-wasm --target wasm32-unknown-unknown
cd web && npm ci && npm audit --audit-level=moderate && npm run test && npm run build
```

## License

MIT
