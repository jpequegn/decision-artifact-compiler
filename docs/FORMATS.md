# Formats

## Markdown Artifact v1

An artifact starts with YAML front matter, followed by required `Objective` and `Non-goals`
sections and one or more fenced `task` blocks. See the complete executable
[repository-change.md](../examples/repository-change.md) example. Generate the machine-readable
schema with `decision-artifact schema`.

Each artifact declares identity, owner, approval/risk status, aggregate authority, bounded execution
budgets, immutable evidence, and reconciliation policy. Each task declares an objective,
dependencies, evidence, typed inputs, output schema, acceptance checks, task authority, and gates.

## Canonical IR

`compile --format ir` emits normalized declarations with source spans and separate digests:

- `artifact_digest` identifies the complete compiled artifact.
- `task_digest` identifies a normalized task declaration.
- `context_digest` covers declared task inputs and evidence.
- `policy_digest` covers authority, gates, acceptance, budgets, status, and risk as applicable.
- `evidence_digest` covers immutable evidence metadata and its declared content digest.

Equivalent declaration ordering produces identical digests. Arrays whose order is semantic, such as
acceptance checks, remain ordered.

## Plan Export

`compile --format plan` emits `version`, `id`, artifact authority, execution limits, and DAG `nodes`.
Every node contains dependencies, inputs, output schema, worker tool name, task authority, timeout,
retry limit, verifiers, gates, and source span. This is the compatibility boundary for external plan
runners.

## Result Envelope

A result records format/run/artifact/task IDs, terminal status, structured output, immutable evidence
digests, acceptance receipts, logs, diffs, citations, tests, and the dispatch receipt sequence. A
result from another artifact digest is stale and cannot reconcile.

## Receipt Ledger

SQLite receipts contain an auto-incrementing sequence, run ID, event kind, JSON payload, previous
hash, and entry hash. The chain covers all runs globally; modifying or removing an earlier row causes
inspection and replay to fail at the affected sequence.
