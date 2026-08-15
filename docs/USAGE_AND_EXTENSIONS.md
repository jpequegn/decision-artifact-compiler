# Usage Patterns and Extensions

## Typical Usage

1. Capture a decision in Markdown with explicit non-goals, evidence, budgets, and authority.
2. Run `validate` during authoring and code review.
3. Attach the canonical compile report and plan export to the approval record.
4. Dispatch through a worker adapter only after the immutable digest is approved.
5. Inspect or replay receipts when work fails, stalls, or is audited.
6. Reconcile immutable results into a proposed patch and review that patch as a new artifact version.

This fits repository changes, data investigations, controlled research, release preparation, and
other multi-step work where authority and evidence matter more than conversational convenience.

## Adapter Extension

Implement the async `artifact_runtime::Worker` trait. The adapter receives `WorkerRequest`, which is
already reduced to declared context and authority. A production adapter should enforce those fields
at its process, container, API proxy, or capability-token boundary and return structured
`WorkerResult` output.

Do not pass the original artifact or undeclared conversation history to the adapter. Doing so would
reintroduce ambient authority and invalidate the context-isolation guarantee.

## Innovative Extensions

- **Signed decision supply chain:** sign artifact, evidence, policy, and receipt-tail digests with
  separate author, approver, and executor keys.
- **Selective reruns:** use task/context/policy digests to rerun only nodes invalidated by changed
  evidence or authority.
- **Policy-as-code bridge:** compile authority into OPA, Cedar, macOS sandbox profiles, containers,
  or egress proxy rules.
- **Organizational decision graph:** connect superseded artifacts and reconciliation proposals into a
  queryable lineage graph.
- **Human/agent separation of duties:** require different identities for authorship, approval,
  execution, evidence verification, and reconciliation.
- **Portable review bundles:** package Markdown, compiled IR, WASM reviewer, evidence manifests, and
  anchored receipts into an offline audit archive.
- **Provider comparison:** run identical typed contexts through multiple worker adapters and reconcile
  conflicting outputs without granting either provider more authority.
