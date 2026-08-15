# Release Checklist

- [x] Rust formatting, clippy warning denial, unit, integration, property, and documentation tests pass.
- [x] The 40-case golden corpus classifies correctly.
- [x] Adversarial injection, authority, conflict, tamper, gate, cancellation, and isolation tests pass.
- [x] End-to-end compile, dispatch, replay, result export, and reconcile workflow passes.
- [x] `artifact-wasm` compiles for `wasm32-unknown-unknown` and matches native review output.
- [x] WASM regenerates on Linux, keeps a stable browser interface, and builds without runtime network dependencies.
- [x] TypeScript model tests, npm audit, and production build pass.
- [x] Desktop and mobile browser smoke checks show no overlap and exercise approval and diagnostics.
- [x] JSON, CSV, and Markdown evaluation reports are current.
- [x] Architecture, formats, security, usage, limitations, troubleshooting, and extensions are documented.
- [x] MIT license and repository metadata are present.
