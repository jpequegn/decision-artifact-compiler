# Troubleshooting

## Validation Fails

Run `decision-artifact validate <file>` and inspect the JSON diagnostics on stderr. Each structural or
policy diagnostic includes a stable code and source location. Common causes are unresolved evidence,
task inputs that reference non-dependencies, cycles, task authority outside the artifact scope, and
missing approval gates.

## Dispatch Produces Blocked Tasks

A task is blocked when a dependency failed or a declared gate ID was not approved. Inspect the run
with `decision-artifact inspect <run-id> --ledger <db>` and find `task_failed` or `task_blocked`
receipts preceding the terminal summary.

## Replay Reports Tampering

Replay verifies the global receipt chain, including other runs. Restore the unmodified database or
start a new ledger. Do not delete an individual run from a shared ledger because that breaks every
subsequent receipt hash.

## Reconciliation Rejects Results

Confirm result `artifact_digest` values equal the current compile digest, evidence IDs are declared by
the task, all required evidence is present, and completed results contain passing receipts for every
acceptance value. Parallel completed outputs with different digests require an explicit human choice.

## Web Build Cannot Find WASM

Run `rustup target add wasm32-unknown-unknown`, then `cd web && npm run wasm`. The generated package
belongs in `web/src/wasm-pkg`. Run `npm ci` before `npm run build` on a clean checkout.

## Port 5173 Is Busy

Choose another local port: `npm run dev -- --port 5174`.
