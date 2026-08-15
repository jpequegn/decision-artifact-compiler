use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::CompiledArtifact;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    Completed,
    Failed,
    Abandoned,
    Superseded,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ResultEvidence {
    pub id: String,
    pub digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CheckReceipt {
    pub check: String,
    pub passed: bool,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResultArtifact {
    pub version: String,
    pub run_id: String,
    pub artifact_digest: String,
    pub task_id: String,
    pub status: ResultStatus,
    pub output: Value,
    pub evidence: Vec<ResultEvidence>,
    pub checks: Vec<CheckReceipt>,
    pub logs: Vec<String>,
    pub diffs: Vec<String>,
    pub citations: Vec<String>,
    pub tests: Vec<String>,
    pub dispatch_receipt_seq: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconciliationDiagnostic {
    pub code: String,
    pub message: String,
    pub task_id: Option<String>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconciliationOutcome {
    pub task_id: String,
    pub status: ResultStatus,
    pub output_digest: Option<String>,
    pub run_id: Option<String>,
    pub dispatch_receipt_seq: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReconciliationProposal {
    pub base_artifact_digest: String,
    pub outcomes: Vec<ReconciliationOutcome>,
    pub markdown_patch: String,
}

#[derive(Debug, Error)]
#[error("results cannot be reconciled")]
pub struct ReconciliationError {
    pub diagnostics: Vec<ReconciliationDiagnostic>,
}

/// Validate immutable worker results and construct a review-only Markdown patch.
///
/// # Errors
/// Returns stable diagnostics for stale bases, conflicts, undeclared evidence,
/// missing acceptance receipts, or invalid task references.
pub fn reconcile_results(
    artifact: &CompiledArtifact,
    results: &[ResultArtifact],
) -> Result<ReconciliationProposal, ReconciliationError> {
    let mut diagnostics = Vec::new();
    let tasks: BTreeMap<_, _> = artifact
        .tasks
        .iter()
        .map(|task| (task.id.as_str(), task))
        .collect();
    let mut by_task: BTreeMap<&str, Vec<&ResultArtifact>> = BTreeMap::new();
    for result in results {
        if validate_result(artifact, &tasks, result, &mut diagnostics) {
            by_task.entry(&result.task_id).or_default().push(result);
        }
    }

    for (task_id, candidates) in &by_task {
        let digests: BTreeSet<_> = candidates
            .iter()
            .filter(|result| result.status == ResultStatus::Completed)
            .map(|result| value_digest(&result.output))
            .collect();
        if digests.len() > 1 {
            diagnostics.push(diagnostic(
                "conflicting_parallel_results",
                "parallel completed results have different output digests",
                Some(task_id),
                digests.into_iter().collect(),
            ));
        }
    }
    if !diagnostics.is_empty() {
        diagnostics.sort_by(|left, right| {
            (&left.task_id, &left.code, &left.evidence).cmp(&(
                &right.task_id,
                &right.code,
                &right.evidence,
            ))
        });
        return Err(ReconciliationError { diagnostics });
    }

    let outcomes = artifact
        .tasks
        .iter()
        .map(|task| outcome_for(task.id.as_str(), by_task.get(task.id.as_str())))
        .collect::<Vec<_>>();
    let markdown_patch = render_patch(artifact, &outcomes, results);
    Ok(ReconciliationProposal {
        base_artifact_digest: artifact.artifact_digest.clone(),
        outcomes,
        markdown_patch,
    })
}

fn validate_result(
    artifact: &CompiledArtifact,
    tasks: &BTreeMap<&str, &crate::CompiledTask>,
    result: &ResultArtifact,
    diagnostics: &mut Vec<ReconciliationDiagnostic>,
) -> bool {
    if result.artifact_digest != artifact.artifact_digest {
        diagnostics.push(diagnostic(
            "stale_artifact_digest",
            "result was produced from a different artifact digest",
            Some(&result.task_id),
            vec![result.artifact_digest.clone()],
        ));
    }
    let Some(task) = tasks.get(result.task_id.as_str()) else {
        diagnostics.push(diagnostic(
            "unknown_result_task",
            "result references an unknown task",
            Some(&result.task_id),
            Vec::new(),
        ));
        return false;
    };
    let declared: BTreeSet<_> = task.evidence.iter().map(String::as_str).collect();
    for evidence in &result.evidence {
        if !declared.contains(evidence.id.as_str()) {
            diagnostics.push(diagnostic(
                "undeclared_result_evidence",
                "result cites evidence outside the task context",
                Some(&result.task_id),
                vec![format!("{}@{}", evidence.id, evidence.digest)],
            ));
        }
    }
    if artifact.reconciliation.require_all_evidence {
        validate_required_evidence(&declared, result, diagnostics);
    }
    if result.status == ResultStatus::Completed {
        for acceptance in &task.acceptance {
            if !result
                .checks
                .iter()
                .any(|check| check.check == acceptance.value && check.passed)
            {
                diagnostics.push(diagnostic(
                    "acceptance_check_missing",
                    "completed result lacks a passing acceptance receipt",
                    Some(&result.task_id),
                    vec![acceptance.value.clone()],
                ));
            }
        }
    }
    true
}

fn validate_required_evidence(
    declared: &BTreeSet<&str>,
    result: &ResultArtifact,
    diagnostics: &mut Vec<ReconciliationDiagnostic>,
) {
    let supplied: BTreeSet<_> = result
        .evidence
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    for missing in declared.difference(&supplied) {
        diagnostics.push(diagnostic(
            "required_evidence_missing",
            "result omitted required task evidence",
            Some(&result.task_id),
            vec![(*missing).to_owned()],
        ));
    }
}

fn outcome_for(task_id: &str, results: Option<&Vec<&ResultArtifact>>) -> ReconciliationOutcome {
    let selected = results.and_then(|items| items.last().copied());
    selected.map_or_else(
        || ReconciliationOutcome {
            task_id: task_id.to_owned(),
            status: ResultStatus::Abandoned,
            output_digest: None,
            run_id: None,
            dispatch_receipt_seq: None,
        },
        |result| ReconciliationOutcome {
            task_id: task_id.to_owned(),
            status: result.status.clone(),
            output_digest: (result.status == ResultStatus::Completed)
                .then(|| value_digest(&result.output)),
            run_id: Some(result.run_id.clone()),
            dispatch_receipt_seq: Some(result.dispatch_receipt_seq),
        },
    )
}

fn render_patch(
    artifact: &CompiledArtifact,
    outcomes: &[ReconciliationOutcome],
    results: &[ResultArtifact],
) -> String {
    let mut patch = format!(
        "\n\n## Reconciliation proposal\n\nBase artifact: `{}`\n\n```yaml\n",
        artifact.artifact_digest
    );
    patch.push_str(&serde_yaml::to_string(&outcomes).expect("serializable outcomes"));
    patch.push_str("```\n\n### Immutable evidence and receipts\n");
    for result in results {
        write!(
            patch,
            "\n- `{}`: run `{}`, dispatch receipt `{}`, evidence {}\n",
            result.task_id,
            result.run_id,
            result.dispatch_receipt_seq,
            result
                .evidence
                .iter()
                .map(|item| format!("`{}@{}`", item.id, item.digest))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .expect("writing to a string cannot fail");
    }
    patch
}

fn value_digest(value: &Value) -> String {
    let encoded = serde_json::to_vec(value).expect("JSON values are serializable");
    format!("sha256:{:x}", Sha256::digest(encoded))
}

fn diagnostic(
    code: &str,
    message: &str,
    task_id: Option<&str>,
    evidence: Vec<String>,
) -> ReconciliationDiagnostic {
    ReconciliationDiagnostic {
        code: code.to_owned(),
        message: message.to_owned(),
        task_id: task_id.map(str::to_owned),
        evidence,
    }
}
