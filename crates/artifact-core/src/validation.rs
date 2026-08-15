use std::collections::{BTreeMap, BTreeSet};

use petgraph::{algo::is_cyclic_directed, graph::DiGraph};
use schemars::schema_for;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{DecisionArtifact, InputBinding, SourceSpan};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub subject: String,
    pub span: SourceSpan,
}

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("artifact failed validation")]
    Invalid { diagnostics: Vec<Diagnostic> },
    #[error("schema serialization failed: {0}")]
    Schema(#[from] serde_json::Error),
}

/// Validate graph, version, budget, evidence, and reference invariants.
///
/// # Errors
/// Returns all deterministic diagnostics when any invariant fails.
#[allow(clippy::too_many_lines)]
pub fn validate_artifact(artifact: &DecisionArtifact) -> Result<(), ValidationError> {
    let mut diagnostics = Vec::new();
    if artifact.version != crate::ARTIFACT_FORMAT_VERSION {
        diagnostics.push(diagnostic(
            "unsupported_version",
            format!("unsupported version '{}'", artifact.version),
            &artifact.id,
            &artifact.span,
        ));
    }
    if artifact.budgets.concurrency_limit == 0 {
        diagnostics.push(diagnostic(
            "invalid_budget",
            "concurrency_limit must be positive".to_owned(),
            &artifact.id,
            &artifact.span,
        ));
    }
    let evidence: BTreeSet<_> = artifact
        .evidence
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    if evidence.len() != artifact.evidence.len() {
        diagnostics.push(diagnostic(
            "duplicate_evidence",
            "evidence IDs must be unique".to_owned(),
            &artifact.id,
            &artifact.span,
        ));
    }
    let mut task_indexes = BTreeMap::new();
    let mut graph = DiGraph::<&str, ()>::new();
    for task in &artifact.tasks {
        if task_indexes.contains_key(task.id.as_str()) {
            diagnostics.push(diagnostic(
                "duplicate_task",
                format!("task '{}' is duplicated", task.id),
                &task.id,
                &task.span,
            ));
        } else {
            task_indexes.insert(task.id.as_str(), graph.add_node(task.id.as_str()));
        }
        for evidence_id in &task.evidence {
            if !evidence.contains(evidence_id.as_str()) {
                diagnostics.push(diagnostic(
                    "unresolved_evidence",
                    format!("evidence '{evidence_id}' is not declared"),
                    &task.id,
                    &task.span,
                ));
            }
        }
        for input in task.inputs.values() {
            if let InputBinding::Evidence { evidence: id } = input
                && !evidence.contains(id.as_str())
            {
                diagnostics.push(diagnostic(
                    "unresolved_evidence",
                    format!("evidence '{id}' is not declared"),
                    &task.id,
                    &task.span,
                ));
            }
        }
    }
    for task in &artifact.tasks {
        for dependency in &task.dependencies {
            match (
                task_indexes.get(dependency.as_str()),
                task_indexes.get(task.id.as_str()),
            ) {
                (Some(from), Some(to)) => {
                    graph.add_edge(*from, *to, ());
                }
                (None, _) => diagnostics.push(diagnostic(
                    "unresolved_dependency",
                    format!("dependency '{dependency}' is not declared"),
                    &task.id,
                    &task.span,
                )),
                _ => {}
            }
        }
        for input in task.inputs.values() {
            if let InputBinding::Task { task: source, .. } = input
                && !task.dependencies.contains(source)
            {
                diagnostics.push(diagnostic(
                    "invalid_task_reference",
                    format!("task input '{source}' must be a dependency"),
                    &task.id,
                    &task.span,
                ));
            }
        }
    }
    if is_cyclic_directed(&graph) {
        diagnostics.push(diagnostic(
            "dependency_cycle",
            "task dependency graph contains a cycle".to_owned(),
            &artifact.id,
            &artifact.span,
        ));
    }
    if diagnostics.is_empty() {
        Ok(())
    } else {
        Err(ValidationError::Invalid { diagnostics })
    }
}

/// Return the canonical JSON Schema for the parsed artifact contract.
///
/// # Errors
/// Returns a serialization error if the generated schema cannot be encoded.
pub fn artifact_json_schema() -> Result<String, ValidationError> {
    Ok(serde_json::to_string_pretty(&schema_for!(
        DecisionArtifact
    ))?)
}

fn diagnostic(code: &str, message: String, subject: &str, span: &SourceSpan) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        message,
        subject: subject.to_owned(),
        span: span.clone(),
    }
}
