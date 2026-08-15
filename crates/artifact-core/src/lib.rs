//! Core contracts, parsing, and validation for approved decision artifacts.

mod compiler;
mod model;
mod parser;
mod policy;
mod validation;

pub use compiler::{
    CompiledArtifact, CompiledEvidence, CompiledTask, PlanLimits, PlanNode, PlanRunnerExport,
    Provenance, Provenanced, compile_artifact, compile_report, export_plan,
};
pub use model::{
    AcceptanceCheck, AcceptanceKind, ApprovalStatus, Authority, Budget, DecisionArtifact, Evidence,
    Gate, GateKind, InputBinding, Reconciliation, ReconciliationMode, RiskClass, SourceSpan, Task,
};
pub use parser::{ArtifactError, parse_artifact};
pub use policy::{PolicyDecision, PolicyError, PolicyReport, enforce_policy};
pub use validation::{Diagnostic, ValidationError, artifact_json_schema, validate_artifact};

/// Supported decision artifact format version.
pub const ARTIFACT_FORMAT_VERSION: &str = "v1";

#[must_use]
pub const fn artifact_format_version() -> &'static str {
    ARTIFACT_FORMAT_VERSION
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_format_version() {
        assert_eq!(artifact_format_version(), "v1");
    }
}
