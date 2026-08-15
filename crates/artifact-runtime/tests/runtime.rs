use std::{
    collections::BTreeSet,
    sync::Arc,
    time::{Duration, Instant},
};

use artifact_core::{compile_artifact, parse_artifact};
use artifact_runtime::{DispatchOptions, FakeWorker, Ledger, RuntimeError, TaskState, dispatch};

const EXAMPLE: &str = include_str!("../../../examples/repository-change.md");

fn compiled() -> artifact_core::CompiledArtifact {
    compile_artifact(&parse_artifact(EXAMPLE).expect("parse")).expect("compile")
}

#[tokio::test]
async fn dispatches_dependencies_and_replays_without_workers() {
    let directory = tempfile::tempdir().expect("tempdir");
    let ledger = Ledger::open(directory.path().join("runs.db")).expect("ledger");
    let summary = dispatch(
        &compiled(),
        Arc::new(FakeWorker::default()),
        &ledger,
        &DispatchOptions::default(),
    )
    .await
    .expect("dispatch");
    assert!(
        summary
            .states
            .values()
            .all(|state| state == &TaskState::Completed)
    );
    assert_eq!(ledger.replay(&summary.run_id).expect("replay"), summary);
    assert!(ledger.inspect(&summary.run_id).expect("inspect").len() >= 8);
    let inspect_output = &summary.outputs["inspect"];
    assert_eq!(inspect_output["evidence"], serde_json::json!(["request"]));
    assert_eq!(
        inspect_output["inputs"]
            .as_object()
            .map(serde_json::Map::len),
        Some(1)
    );
}

#[tokio::test]
async fn failed_tasks_cancel_their_dependency_chain() {
    let directory = tempfile::tempdir().expect("tempdir");
    let ledger = Ledger::open(directory.path().join("runs.db")).expect("ledger");
    let summary = dispatch(
        &compiled(),
        Arc::new(FakeWorker {
            delay_ms: 0,
            fail_tasks: BTreeSet::from(["inspect".to_owned()]),
        }),
        &ledger,
        &DispatchOptions::default(),
    )
    .await
    .expect("dispatch");
    assert_eq!(summary.states["inspect"], TaskState::Failed);
    assert_eq!(summary.states["implement"], TaskState::Blocked);
    assert_eq!(summary.states["verify"], TaskState::Blocked);
}

#[tokio::test]
async fn independent_tasks_run_under_the_concurrency_cap() {
    let mut artifact = compiled();
    artifact.tasks[1].dependencies.clear();
    artifact.tasks[2].dependencies.clear();
    artifact.budgets.concurrency_limit = 3;
    let directory = tempfile::tempdir().expect("tempdir");
    let ledger = Ledger::open(directory.path().join("runs.db")).expect("ledger");
    let start = Instant::now();
    dispatch(
        &artifact,
        Arc::new(FakeWorker {
            delay_ms: 80,
            fail_tasks: BTreeSet::new(),
        }),
        &ledger,
        &DispatchOptions::default(),
    )
    .await
    .expect("dispatch");
    assert!(start.elapsed() < Duration::from_millis(200));
}

#[tokio::test]
async fn unapproved_gates_block_tasks_and_dependents() {
    let mut artifact = compiled();
    artifact
        .tasks
        .iter_mut()
        .find(|task| task.id == "inspect")
        .expect("inspect task")
        .gates
        .push(artifact_core::Gate {
            id: "human".to_owned(),
            kind: artifact_core::GateKind::Approval,
            approver: Some("reviewer".to_owned()),
        });
    let directory = tempfile::tempdir().expect("tempdir");
    let ledger = Ledger::open(directory.path().join("runs.db")).expect("ledger");
    let summary = dispatch(
        &artifact,
        Arc::new(FakeWorker::default()),
        &ledger,
        &DispatchOptions::default(),
    )
    .await
    .expect("dispatch");
    assert!(
        summary
            .states
            .values()
            .all(|state| state == &TaskState::Blocked)
    );
}

#[tokio::test]
async fn detects_ledger_tampering() {
    let directory = tempfile::tempdir().expect("tempdir");
    let path = directory.path().join("runs.db");
    let ledger = Ledger::open(&path).expect("ledger");
    let summary = dispatch(
        &compiled(),
        Arc::new(FakeWorker::default()),
        &ledger,
        &DispatchOptions::default(),
    )
    .await
    .expect("dispatch");
    drop(ledger);
    rusqlite::Connection::open(&path)
        .expect("connection")
        .execute("UPDATE receipts SET payload = '{}' WHERE seq = 1", [])
        .expect("tamper");
    let ledger = Ledger::open(path).expect("ledger");
    assert!(matches!(
        ledger.replay(&summary.run_id),
        Err(RuntimeError::Tampered(1))
    ));
}
