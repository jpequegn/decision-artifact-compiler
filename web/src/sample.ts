export const sampleArtifact = `---
version: v1
id: repository-change
owner: julien
status: approved
risk_class: low
authority:
  read_paths: ["src/**", "tests/**", "Cargo.toml"]
  write_paths: ["src/**", "tests/**"]
  commands: ["cargo test", "cargo clippy"]
  network_domains: []
  secrets: []
  side_effects: []
budgets:
  time_ms: 120000
  token_limit: 20000
  cost_micros: 500000
  retry_limit: 1
  concurrency_limit: 2
evidence:
  - id: request
    uri: "file://REQUEST.md"
    digest: "sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
    description: Approved repository change request
reconciliation:
  mode: proposed_patch
  require_all_evidence: true
---

## Objective

Implement and verify a scoped repository change.

## Non-goals

Do not publish, deploy, access secrets, or use the network.

## Tasks

\`\`\`task inspect
objective: Inspect the relevant source and tests
dependencies: []
evidence: [request]
inputs:
  request: {kind: evidence, evidence: request}
output_schema: {type: object, required: [findings]}
acceptance: [{kind: evidence, value: findings}]
authority:
  read_paths: ["src/**", "tests/**", "Cargo.toml"]
  write_paths: []
  commands: []
  network_domains: []
  secrets: []
  side_effects: []
gates: []
\`\`\`

\`\`\`task implement
objective: Implement the approved change
dependencies: [inspect]
evidence: [request]
inputs:
  findings: {kind: task, task: inspect, path: /findings}
output_schema: {type: object, required: [changed_files]}
acceptance: [{kind: evidence, value: changed_files}]
authority:
  read_paths: ["src/**", "tests/**", "Cargo.toml"]
  write_paths: ["src/**", "tests/**"]
  commands: []
  network_domains: []
  secrets: []
  side_effects: []
gates: []
\`\`\`

\`\`\`task verify
objective: Run the declared verification commands
dependencies: [implement]
evidence: [request]
inputs:
  changes: {kind: task, task: implement, path: /changed_files}
output_schema: {type: object, required: [tests_passed]}
acceptance: [{kind: command, value: cargo test}]
authority:
  read_paths: ["src/**", "tests/**", "Cargo.toml"]
  write_paths: []
  commands: ["cargo test", "cargo clippy"]
  network_domains: []
  secrets: []
  side_effects: []
gates: []
\`\`\`
`;
