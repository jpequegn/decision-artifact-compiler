# Security Model

## Trust Boundaries

- Human-authored typed declarations are the only source of authority.
- Prose, evidence descriptions, worker output, and model-proposed text are untrusted data.
- Task authority must be a subset of artifact authority across file, command, network, secret, and
  side-effect scopes.
- Consequential or side-effecting tasks require a named approval gate.
- Workers receive only declared inputs, evidence, output schema, and task authority.
- Reconciliation emits a proposal tied to an immutable digest; it cannot mutate approval state.

## Fail-Closed Rules

Execution is denied for unknown format versions, malformed declarations, unresolved references,
cycles, zero execution budgets, broadened capabilities, and missing required gates. Stable reason
codes and source spans make each denial reviewable.

Prompt injection in evidence cannot add typed capability fields. The adversarial corpus verifies
that text requesting network, secret, and deployment access remains inert.

## Scope Semantics

- Exact grants match exact requests.
- `path/**` grants that path and descendants.
- `*.example.com` grants subdomains, not the apex or unrelated suffixes.
- Commands, secrets, and side effects otherwise require exact declarations.

## Ledger Integrity

Receipts are SHA-256 hash-linked. This detects accidental or unsophisticated tampering but is not a
signature scheme: an attacker who can rewrite the complete database can recompute the chain. For a
production deployment, periodically anchor the tail hash in an independently controlled signed log.

## Limitations

- The included worker is deterministic and does not sandbox arbitrary third-party adapters.
- File globs are policy strings; the runtime does not currently mediate operating-system syscalls.
- SHA-256 digests provide identity, not author signatures or timestamp attestation.
- SQLite is single-host storage. Multi-host execution needs a transactional shared ledger.
- Review approval is local UI state. Production approval should use authenticated, signed identities.
