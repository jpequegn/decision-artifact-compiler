# Evaluation Methodology

`artifact-eval` runs deterministic release checks and writes JSON, CSV, and Markdown reports.

## Measures

- **Compile correctness:** expected accept/reject classification over 20 valid and 20 invalid artifact
  mutations.
- **Blocker precision:** expected invalid cases that fail closed without false acceptance.
- **Injection boundary:** adversarial evidence text cannot create network or secret authority.
- **Authority boundary:** an undeclared task domain is rejected.
- **Conflict detection:** parallel completed outputs with distinct digests require resolution.
- **Replay parity:** hash-verified replay equals the original terminal summary without worker calls.
- **Context duplication:** full artifact bytes repeated once per task versus task objectives plus
  declared evidence descriptions.
- **Reviewability:** compiled tasks retaining nonzero source locations.

## Interpretation

The context measure is a transparent byte-count proxy, not a model token or quality benchmark. It
isolates duplication removed by contract-driven context construction. It does not claim equivalent
model performance, latency, or cost across providers.

Regenerate reports with `cargo run -p artifact-eval -- --output-dir reports`. CI runs the same command
with `--check` and fails when checked-in reports are stale.
