use artifact_core::{
    CheckReceipt, ResultArtifact, ResultEvidence, ResultStatus, compile_artifact, parse_artifact,
    reconcile_results,
};
use serde_json::json;

const EXAMPLE: &str = include_str!("../../../examples/repository-change.md");

fn compiled() -> artifact_core::CompiledArtifact {
    compile_artifact(&parse_artifact(EXAMPLE).expect("parse")).expect("compile")
}

fn result(
    artifact: &artifact_core::CompiledArtifact,
    task_id: &str,
    status: ResultStatus,
) -> ResultArtifact {
    let task = artifact
        .tasks
        .iter()
        .find(|task| task.id == task_id)
        .expect("task");
    ResultArtifact {
        version: "v1".to_owned(),
        run_id: "run-test".to_owned(),
        artifact_digest: artifact.artifact_digest.clone(),
        task_id: task_id.to_owned(),
        status,
        output: json!({"task": task_id, "value": 1}),
        evidence: task
            .evidence
            .iter()
            .map(|id| ResultEvidence {
                id: id.clone(),
                digest: "sha256:evidence".to_owned(),
            })
            .collect(),
        checks: task
            .acceptance
            .iter()
            .map(|check| CheckReceipt {
                check: check.value.clone(),
                passed: true,
                detail: "verified".to_owned(),
            })
            .collect(),
        logs: vec!["worker.log#sha256:log".to_owned()],
        diffs: vec!["change.patch#sha256:diff".to_owned()],
        citations: vec!["file://REQUEST.md#sha256:evidence".to_owned()],
        tests: vec!["cargo test#sha256:test".to_owned()],
        dispatch_receipt_seq: 7,
    }
}

#[test]
fn proposes_review_patch_without_mutating_the_artifact() {
    let artifact = compiled();
    let before = artifact.clone();
    let results: Vec<_> = artifact
        .tasks
        .iter()
        .map(|task| result(&artifact, &task.id, ResultStatus::Completed))
        .collect();
    let proposal = reconcile_results(&artifact, &results).expect("reconcile");
    assert_eq!(artifact, before);
    assert_eq!(proposal.outcomes.len(), 3);
    assert!(proposal.markdown_patch.contains(&artifact.artifact_digest));
    assert!(proposal.markdown_patch.contains("dispatch receipt `7`"));
}

#[test]
fn preserves_failed_and_abandoned_history() {
    let artifact = compiled();
    let results = [result(&artifact, "inspect", ResultStatus::Failed)];
    let proposal = reconcile_results(&artifact, &results).expect("reconcile history");
    assert_eq!(proposal.outcomes[0].status, ResultStatus::Abandoned);
    assert!(
        proposal
            .outcomes
            .iter()
            .any(|item| item.task_id == "inspect" && item.status == ResultStatus::Failed)
    );
    assert!(
        proposal
            .outcomes
            .iter()
            .any(|item| item.task_id == "verify" && item.status == ResultStatus::Abandoned)
    );
}

#[test]
fn rejects_stale_missing_and_undeclared_evidence() {
    let artifact = compiled();
    let mut invalid = result(&artifact, "inspect", ResultStatus::Completed);
    invalid.artifact_digest = "sha256:stale".to_owned();
    invalid.evidence.clear();
    invalid.evidence.push(ResultEvidence {
        id: "secret-context".to_owned(),
        digest: "sha256:bad".to_owned(),
    });
    let error = reconcile_results(&artifact, &[invalid]).expect_err("must reject");
    let codes: Vec<_> = error
        .diagnostics
        .iter()
        .map(|item| item.code.as_str())
        .collect();
    assert!(codes.contains(&"stale_artifact_digest"));
    assert!(codes.contains(&"undeclared_result_evidence"));
    assert!(codes.contains(&"required_evidence_missing"));
}

#[test]
fn conflicting_parallel_results_require_resolution() {
    let artifact = compiled();
    let first = result(&artifact, "inspect", ResultStatus::Completed);
    let mut second = first.clone();
    second.run_id = "run-parallel".to_owned();
    second.output = json!({"task": "inspect", "value": 2});
    let error = reconcile_results(&artifact, &[first, second]).expect_err("conflict");
    assert!(
        error
            .diagnostics
            .iter()
            .any(|item| item.code == "conflicting_parallel_results")
    );
}
