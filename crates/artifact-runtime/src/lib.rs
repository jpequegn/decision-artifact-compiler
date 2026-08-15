//! Native bounded execution and append-only receipt storage.

use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use artifact_core::{
    Authority, CheckReceipt, CompiledArtifact, CompiledTask, InputBinding, ResultArtifact,
    ResultEvidence, ResultStatus,
};
use async_trait::async_trait;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkerRequest {
    pub run_id: String,
    pub task_id: String,
    pub objective: String,
    pub inputs: BTreeMap<String, Value>,
    pub evidence: BTreeMap<String, Value>,
    pub authority: Authority,
    pub output_schema: Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorkerResult {
    pub output: Value,
    pub worker: String,
}

#[derive(Clone, Debug, Error)]
#[error("worker failed: {message}")]
pub struct WorkerError {
    pub message: String,
}

#[async_trait]
pub trait Worker: Send + Sync {
    async fn execute(&self, request: WorkerRequest) -> Result<WorkerResult, WorkerError>;
}

#[derive(Clone, Debug, Default)]
pub struct FakeWorker {
    pub delay_ms: u64,
    pub fail_tasks: BTreeSet<String>,
}

#[async_trait]
impl Worker for FakeWorker {
    async fn execute(&self, request: WorkerRequest) -> Result<WorkerResult, WorkerError> {
        if self.delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        }
        if self.fail_tasks.contains(&request.task_id) {
            return Err(WorkerError {
                message: format!("configured failure for {}", request.task_id),
            });
        }
        Ok(WorkerResult {
            output: json!({
                "task_id": request.task_id,
                "inputs": request.inputs,
                "evidence": request.evidence.keys().collect::<Vec<_>>(),
                "completed": true,
            }),
            worker: "deterministic-fake".to_owned(),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct DispatchOptions {
    pub approved_gates: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskState {
    Completed,
    Failed,
    Blocked,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RunSummary {
    pub run_id: String,
    pub artifact_digest: String,
    pub states: BTreeMap<String, TaskState>,
    pub outputs: BTreeMap<String, Value>,
    pub results: Vec<ResultArtifact>,
}

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("ledger error: {0}")]
    Ledger(#[from] rusqlite::Error),
    #[error("receipt encoding error: {0}")]
    Encoding(#[from] serde_json::Error),
    #[error("receipt chain is invalid at sequence {0}")]
    Tampered(i64),
    #[error("run `{0}` was not found")]
    RunNotFound(String),
}

pub struct Ledger {
    connection: Connection,
}

impl Ledger {
    /// Open or create a receipt ledger.
    ///
    /// # Errors
    /// Returns `SQLite` errors while opening or migrating the ledger.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, RuntimeError> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS receipts (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                payload TEXT NOT NULL,
                previous_hash TEXT NOT NULL,
                entry_hash TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS receipts_run ON receipts(run_id, seq);",
        )?;
        Ok(Self { connection })
    }

    /// Append one hash-linked receipt.
    ///
    /// # Errors
    /// Returns storage or JSON encoding errors.
    pub fn append<T: Serialize>(
        &self,
        run_id: &str,
        kind: &str,
        payload: &T,
    ) -> Result<(), RuntimeError> {
        let payload = serde_json::to_string(payload)?;
        let previous_hash: String = self.connection.query_row(
            "SELECT COALESCE((SELECT entry_hash FROM receipts ORDER BY seq DESC LIMIT 1), '')",
            [],
            |row| row.get(0),
        )?;
        let entry_hash = receipt_hash(&previous_hash, run_id, kind, &payload);
        self.connection.execute(
            "INSERT INTO receipts(run_id, kind, payload, previous_hash, entry_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, kind, payload, previous_hash, entry_hash],
        )?;
        Ok(())
    }

    /// Verify the ledger hash chain and reconstruct a run from receipts only.
    ///
    /// # Errors
    /// Returns an error for tampering, malformed receipts, missing runs, or `SQLite` failures.
    pub fn replay(&self, run_id: &str) -> Result<RunSummary, RuntimeError> {
        self.verify_chain()?;
        let mut statement = self
            .connection
            .prepare("SELECT kind, payload FROM receipts WHERE run_id = ?1 ORDER BY seq")?;
        let rows = statement.query_map([run_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut summary = None;
        for row in rows {
            let (kind, payload) = row?;
            if kind == "run_completed" {
                summary = Some(serde_json::from_str(&payload)?);
            }
        }
        summary.ok_or_else(|| RuntimeError::RunNotFound(run_id.to_owned()))
    }

    /// Return all receipts for inspection after verifying the chain.
    ///
    /// # Errors
    /// Returns an error for tampering or `SQLite` failures.
    pub fn inspect(&self, run_id: &str) -> Result<Vec<Value>, RuntimeError> {
        self.verify_chain()?;
        let mut statement = self.connection.prepare(
            "SELECT seq, kind, payload, previous_hash, entry_hash FROM receipts WHERE run_id = ?1 ORDER BY seq",
        )?;
        let rows = statement.query_map([run_id], |row| {
            let payload: String = row.get(2)?;
            Ok(json!({
                "seq": row.get::<_, i64>(0)?,
                "kind": row.get::<_, String>(1)?,
                "payload": serde_json::from_str::<Value>(&payload).unwrap_or(Value::String(payload)),
                "previous_hash": row.get::<_, String>(3)?,
                "entry_hash": row.get::<_, String>(4)?,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    /// Return the latest receipt sequence in the ledger.
    ///
    /// # Errors
    /// Returns `SQLite` query failures.
    pub fn last_sequence(&self) -> Result<u64, RuntimeError> {
        self.connection
            .query_row("SELECT COALESCE(MAX(seq), 0) FROM receipts", [], |row| {
                row.get(0)
            })
            .map_err(Into::into)
    }

    fn verify_chain(&self) -> Result<(), RuntimeError> {
        let mut statement = self.connection.prepare(
            "SELECT seq, run_id, kind, payload, previous_hash, entry_hash FROM receipts ORDER BY seq",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        let mut expected_previous = String::new();
        for row in rows {
            let (seq, run_id, kind, payload, previous, hash) = row?;
            let expected = receipt_hash(&expected_previous, &run_id, &kind, &payload);
            if previous != expected_previous || hash != expected {
                return Err(RuntimeError::Tampered(seq));
            }
            expected_previous = hash;
        }
        Ok(())
    }
}

/// Dispatch a compiled artifact through bounded workers and persist receipts.
///
/// # Errors
/// Returns ledger failures. Worker failures are captured as terminal task states.
pub async fn dispatch(
    artifact: &CompiledArtifact,
    worker: Arc<dyn Worker>,
    ledger: &Ledger,
    options: &DispatchOptions,
) -> Result<RunSummary, RuntimeError> {
    let run_id = new_run_id();
    ledger.append(
        &run_id,
        "run_started",
        &json!({
            "artifact_digest": artifact.artifact_digest,
            "policy_digest": artifact.policy_digest,
            "owner": artifact.owner,
        }),
    )?;
    let mut states = BTreeMap::new();
    let mut outputs = BTreeMap::new();
    let mut results = Vec::new();
    let tasks: BTreeMap<_, _> = artifact
        .tasks
        .iter()
        .map(|task| (task.id.clone(), task))
        .collect();

    while states.len() < tasks.len() {
        let mut ready: Vec<_> = tasks
            .values()
            .filter(|task| !states.contains_key(&task.id))
            .filter(|task| {
                task.dependencies
                    .iter()
                    .all(|id| states.get(id) == Some(&TaskState::Completed))
            })
            .filter(|task| {
                task.gates
                    .iter()
                    .all(|gate| options.approved_gates.contains(&gate.id))
            })
            .copied()
            .collect();
        ready.sort_by(|left, right| left.id.cmp(&right.id));
        ready.truncate(artifact.budgets.concurrency_limit);
        if ready.is_empty() {
            block_remaining(artifact, &tasks, &mut states, &mut results, ledger, &run_id)?;
            break;
        }

        let futures = ready.into_iter().map(|task| {
            let request = worker_request(&run_id, artifact, task, &outputs);
            let worker = Arc::clone(&worker);
            async move {
                (
                    task.id.clone(),
                    request.clone(),
                    worker.execute(request).await,
                )
            }
        });
        for (task_id, request, result) in futures::future::join_all(futures).await {
            ledger.append(&run_id, "task_dispatched", &request)?;
            let dispatch_receipt_seq = ledger.last_sequence()?;
            match result {
                Ok(result) => {
                    outputs.insert(task_id.clone(), result.output.clone());
                    states.insert(task_id.clone(), TaskState::Completed);
                    ledger.append(
                        &run_id,
                        "task_completed",
                        &json!({"task_id": task_id, "result": result}),
                    )?;
                    results.push(result_artifact(
                        artifact,
                        tasks[&task_id],
                        &run_id,
                        ResultStatus::Completed,
                        result.output,
                        dispatch_receipt_seq,
                    ));
                }
                Err(error) => {
                    states.insert(task_id.clone(), TaskState::Failed);
                    ledger.append(
                        &run_id,
                        "task_failed",
                        &json!({"task_id": task_id, "error": error.to_string()}),
                    )?;
                    results.push(result_artifact(
                        artifact,
                        tasks[&task_id],
                        &run_id,
                        ResultStatus::Failed,
                        Value::Null,
                        dispatch_receipt_seq,
                    ));
                }
            }
        }
    }
    let summary = run_summary(&run_id, artifact, states, outputs, results);
    ledger.append(&run_id, "run_completed", &summary)?;
    Ok(summary)
}

fn run_summary(
    run_id: &str,
    artifact: &CompiledArtifact,
    states: BTreeMap<String, TaskState>,
    outputs: BTreeMap<String, Value>,
    results: Vec<ResultArtifact>,
) -> RunSummary {
    RunSummary {
        run_id: run_id.to_owned(),
        artifact_digest: artifact.artifact_digest.clone(),
        states,
        outputs,
        results,
    }
}

fn block_remaining(
    artifact: &CompiledArtifact,
    tasks: &BTreeMap<String, &CompiledTask>,
    states: &mut BTreeMap<String, TaskState>,
    results: &mut Vec<ResultArtifact>,
    ledger: &Ledger,
    run_id: &str,
) -> Result<(), RuntimeError> {
    let blocked: Vec<_> = tasks
        .keys()
        .filter(|task_id| !states.contains_key(*task_id))
        .cloned()
        .collect();
    for task_id in blocked {
        states.insert(task_id.clone(), TaskState::Blocked);
        ledger.append(run_id, "task_blocked", &json!({"task_id": task_id}))?;
        results.push(result_artifact(
            artifact,
            tasks[&task_id],
            run_id,
            ResultStatus::Abandoned,
            Value::Null,
            0,
        ));
    }
    Ok(())
}

fn result_artifact(
    artifact: &CompiledArtifact,
    task: &CompiledTask,
    run_id: &str,
    status: ResultStatus,
    output: Value,
    dispatch_receipt_seq: u64,
) -> ResultArtifact {
    let passed = status == ResultStatus::Completed;
    ResultArtifact {
        version: artifact.format_version.clone(),
        run_id: run_id.to_owned(),
        artifact_digest: artifact.artifact_digest.clone(),
        task_id: task.id.clone(),
        status,
        output,
        evidence: task
            .evidence
            .iter()
            .filter_map(|id| {
                artifact
                    .evidence
                    .iter()
                    .find(|item| item.id == *id)
                    .map(|item| ResultEvidence {
                        id: id.clone(),
                        digest: item.evidence_digest.clone(),
                    })
            })
            .collect(),
        checks: task
            .acceptance
            .iter()
            .map(|acceptance| CheckReceipt {
                check: acceptance.value.clone(),
                passed,
                detail: if passed {
                    "deterministic worker acceptance".to_owned()
                } else {
                    "task did not complete".to_owned()
                },
            })
            .collect(),
        logs: vec![format!("ledger://{run_id}/{}", task.id)],
        diffs: Vec::new(),
        citations: task
            .evidence
            .iter()
            .map(|id| format!("evidence://{id}"))
            .collect(),
        tests: task
            .acceptance
            .iter()
            .map(|acceptance| acceptance.value.clone())
            .collect(),
        dispatch_receipt_seq,
    }
}

fn worker_request(
    run_id: &str,
    artifact: &CompiledArtifact,
    task: &CompiledTask,
    outputs: &BTreeMap<String, Value>,
) -> WorkerRequest {
    let evidence_by_id: BTreeMap<_, _> = artifact
        .evidence
        .iter()
        .map(|item| (&item.id, item))
        .collect();
    let evidence = task
        .evidence
        .iter()
        .filter_map(|id| {
            evidence_by_id.get(id).map(|item| {
                (
                    id.clone(),
                    json!({
                        "uri": item.uri,
                        "digest": item.declared_digest,
                        "evidence_digest": item.evidence_digest
                    }),
                )
            })
        })
        .collect();
    let inputs = task
        .inputs
        .iter()
        .filter_map(|(name, binding)| match binding {
            InputBinding::Literal { value } => Some((name.clone(), value.clone())),
            InputBinding::Evidence { evidence } => evidence_by_id.get(evidence).map(|item| {
                (
                    name.clone(),
                    json!({"uri": item.uri, "digest": item.declared_digest}),
                )
            }),
            InputBinding::Task { task, path: _ } => {
                outputs.get(task).map(|value| (name.clone(), value.clone()))
            }
        })
        .collect();
    WorkerRequest {
        run_id: run_id.to_owned(),
        task_id: task.id.clone(),
        objective: task.objective.value.clone(),
        inputs,
        evidence,
        authority: task.authority.clone(),
        output_schema: task.output_schema.clone(),
    }
}

fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |value| value.as_nanos());
    let sequence = RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("run-{nanos:x}-{sequence:x}")
}

fn receipt_hash(previous: &str, run_id: &str, kind: &str, payload: &str) -> String {
    let mut hasher = Sha256::new();
    for value in [previous, run_id, kind, payload] {
        hasher.update(value.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{:x}", hasher.finalize())
}
