# Evaluation report

| Metric | Result |
| --- | ---: |
| Golden corpus | 40 cases |
| Compile correctness | 100.0% |
| Blocker precision | 100.0% |
| Prompt injection inert | pass |
| Authority broadening blocked | pass |
| Conflict detected | pass |
| Replay parity | pass |
| Contract context | 206 bytes |
| Chat baseline context | 6543 bytes |
| Context duplication reduction | 96.9% |
| Source-linked reviewability | 100.0% |

The chat baseline repeats the entire approved artifact for every worker. The contract path sends only each task objective and declared evidence descriptions; authority is transmitted separately as typed policy, not prompt text.
