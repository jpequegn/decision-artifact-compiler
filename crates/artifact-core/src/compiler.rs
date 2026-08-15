use std::{collections::BTreeMap, fmt::Write};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{
    AcceptanceCheck, ApprovalStatus, Authority, Budget, DecisionArtifact, Gate, InputBinding,
    Reconciliation, RiskClass, SourceSpan,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    LiteralHuman,
    ModelProposed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Provenanced<T> {
    pub value: T,
    pub provenance: Provenance,
    pub source: SourceSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CompiledEvidence {
    pub id: String,
    pub uri: String,
    pub declared_digest: String,
    pub evidence_digest: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CompiledTask {
    pub id: String,
    pub objective: Provenanced<String>,
    pub dependencies: Vec<String>,
    pub evidence: Vec<String>,
    pub inputs: BTreeMap<String, InputBinding>,
    pub output_schema: Value,
    pub acceptance: Vec<AcceptanceCheck>,
    pub authority: Authority,
    pub gates: Vec<Gate>,
    pub task_digest: String,
    pub context_digest: String,
    pub policy_digest: String,
    pub source: SourceSpan,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CompiledArtifact {
    pub format_version: String,
    pub artifact_id: String,
    pub artifact_digest: String,
    pub owner: String,
    pub status: ApprovalStatus,
    pub risk_class: RiskClass,
    pub objective: Provenanced<String>,
    pub non_goals: Provenanced<String>,
    pub authority: Authority,
    pub budgets: Budget,
    pub policy_digest: String,
    pub evidence: Vec<CompiledEvidence>,
    pub reconciliation: Reconciliation,
    pub tasks: Vec<CompiledTask>,
    pub source: SourceSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PlanLimits {
    pub timeout_ms: u64,
    pub token_limit: u64,
    pub cost_micros: u64,
    pub retry_limit: u32,
    pub concurrency_limit: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlanNode {
    pub id: String,
    pub objective: String,
    pub dependencies: Vec<String>,
    pub inputs: BTreeMap<String, InputBinding>,
    pub output_schema: Value,
    pub tool: String,
    pub authority: Authority,
    pub timeout_ms: u64,
    pub retry_limit: u32,
    pub verifier: Vec<AcceptanceCheck>,
    pub gates: Vec<Gate>,
    pub source: SourceSpan,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PlanRunnerExport {
    pub version: String,
    pub id: String,
    pub artifact_digest: String,
    pub authority: Authority,
    pub limits: PlanLimits,
    pub nodes: Vec<PlanNode>,
}

/// Compile a validated artifact into normalized, digest-addressed IR.
///
/// # Errors
/// Returns an error when a declaration cannot be encoded for canonical hashing.
pub fn compile_artifact(
    artifact: &DecisionArtifact,
) -> Result<CompiledArtifact, serde_json::Error> {
    let authority = normalized_authority(&artifact.authority);
    let policy_digest = digest(&serde_json::json!({
        "authority": authority,
        "budgets": artifact.budgets,
        "risk_class": artifact.risk_class,
        "status": artifact.status,
    }))?;

    let evidence = compile_evidence(artifact)?;
    let evidence_digests: BTreeMap<_, _> = evidence
        .iter()
        .map(|item| (item.id.clone(), item.evidence_digest.clone()))
        .collect();
    let tasks = compile_tasks(artifact, &evidence_digests)?;

    let mut compiled = CompiledArtifact {
        format_version: artifact.version.clone(),
        artifact_id: artifact.id.clone(),
        artifact_digest: String::new(),
        owner: artifact.owner.clone(),
        status: artifact.status.clone(),
        risk_class: artifact.risk_class.clone(),
        objective: Provenanced {
            value: artifact.objective.clone(),
            provenance: Provenance::LiteralHuman,
            source: artifact.span.clone(),
        },
        non_goals: Provenanced {
            value: artifact.non_goals.clone(),
            provenance: Provenance::LiteralHuman,
            source: artifact.span.clone(),
        },
        authority,
        budgets: artifact.budgets.clone(),
        policy_digest,
        evidence,
        reconciliation: artifact.reconciliation.clone(),
        tasks,
        source: artifact.span.clone(),
    };
    compiled.artifact_digest = digest(&serde_json::to_value(&compiled)?)?;
    Ok(compiled)
}

fn compile_evidence(
    artifact: &DecisionArtifact,
) -> Result<Vec<CompiledEvidence>, serde_json::Error> {
    let mut evidence: Vec<_> = artifact
        .evidence
        .iter()
        .map(|item| {
            let evidence_digest = digest(&serde_json::json!({
                "id": item.id,
                "uri": item.uri,
                "declared_digest": item.digest,
                "description": item.description,
            }))?;
            Ok(CompiledEvidence {
                id: item.id.clone(),
                uri: item.uri.clone(),
                declared_digest: item.digest.clone(),
                evidence_digest,
                description: item.description.clone(),
            })
        })
        .collect::<Result<_, serde_json::Error>>()?;
    evidence.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(evidence)
}

fn compile_tasks(
    artifact: &DecisionArtifact,
    evidence_digests: &BTreeMap<String, String>,
) -> Result<Vec<CompiledTask>, serde_json::Error> {
    let mut tasks: Vec<_> = artifact
        .tasks
        .iter()
        .map(|task| {
            let authority = normalized_authority(&task.authority);
            let mut dependencies = task.dependencies.clone();
            dependencies.sort();
            let mut referenced_evidence = task.evidence.clone();
            referenced_evidence.sort();
            let context_digest = digest(&serde_json::json!({
                "evidence": referenced_evidence.iter().filter_map(|id| evidence_digests.get(id).map(|digest| (id, digest))).collect::<BTreeMap<_, _>>(),
                "inputs": task.inputs,
            }))?;
            let task_policy_digest = digest(&serde_json::json!({
                "authority": authority,
                "gates": task.gates,
                "acceptance": task.acceptance,
            }))?;
            let task_digest = digest(&serde_json::json!({
                "id": task.id,
                "objective": task.objective,
                "dependencies": dependencies,
                "context_digest": context_digest,
                "policy_digest": task_policy_digest,
                "output_schema": task.output_schema,
            }))?;
            Ok(CompiledTask {
                id: task.id.clone(),
                objective: Provenanced {
                    value: task.objective.clone(),
                    provenance: Provenance::LiteralHuman,
                    source: task.span.clone(),
                },
                dependencies,
                evidence: referenced_evidence,
                inputs: task.inputs.clone(),
                output_schema: task.output_schema.clone(),
                acceptance: task.acceptance.clone(),
                authority,
                gates: task.gates.clone(),
                task_digest,
                context_digest,
                policy_digest: task_policy_digest,
                source: task.span.clone(),
            })
        })
        .collect::<Result<_, serde_json::Error>>()?;
    tasks.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(tasks)
}

#[must_use]
pub fn export_plan(compiled: &CompiledArtifact) -> PlanRunnerExport {
    PlanRunnerExport {
        version: compiled.format_version.clone(),
        id: compiled.artifact_id.clone(),
        artifact_digest: compiled.artifact_digest.clone(),
        authority: compiled.authority.clone(),
        limits: PlanLimits {
            timeout_ms: compiled.budgets.time_ms,
            token_limit: compiled.budgets.token_limit,
            cost_micros: compiled.budgets.cost_micros,
            retry_limit: compiled.budgets.retry_limit,
            concurrency_limit: compiled.budgets.concurrency_limit,
        },
        nodes: compiled
            .tasks
            .iter()
            .map(|task| PlanNode {
                id: task.id.clone(),
                objective: task.objective.value.clone(),
                dependencies: task.dependencies.clone(),
                inputs: task.inputs.clone(),
                output_schema: task.output_schema.clone(),
                tool: "artifact-worker".to_owned(),
                authority: task.authority.clone(),
                timeout_ms: compiled.budgets.time_ms,
                retry_limit: compiled.budgets.retry_limit,
                verifier: task.acceptance.clone(),
                gates: task.gates.clone(),
                source: task.source.clone(),
            })
            .collect(),
    }
}

#[must_use]
pub fn compile_report(compiled: &CompiledArtifact) -> String {
    let mut report = format!(
        "# Compile report: {}\n\n- Artifact digest: `{}`\n- Policy digest: `{}`\n- Owner: `{}`\n- Tasks: {}\n- Evidence: {}\n\n## Task graph\n",
        compiled.artifact_id,
        compiled.artifact_digest,
        compiled.policy_digest,
        compiled.owner,
        compiled.tasks.len(),
        compiled.evidence.len()
    );
    for task in &compiled.tasks {
        let dependencies = if task.dependencies.is_empty() {
            "none".to_owned()
        } else {
            task.dependencies.join(", ")
        };
        write!(
            report,
            "\n### {}\n\n- Depends on: {}\n- Task digest: `{}`\n- Context digest: `{}`\n- Policy digest: `{}`\n- Source: lines {}-{}\n",
            task.id,
            dependencies,
            task.task_digest,
            task.context_digest,
            task.policy_digest,
            task.source.start_line,
            task.source.end_line
        )
        .expect("writing to a string cannot fail");
    }
    report
}

fn normalized_authority(authority: &Authority) -> Authority {
    let mut normalized = authority.clone();
    normalized.read_paths.sort();
    normalized.write_paths.sort();
    normalized.commands.sort();
    normalized.network_domains.sort();
    normalized.secrets.sort();
    normalized.side_effects.sort();
    normalized
}

fn digest(value: &Value) -> Result<String, serde_json::Error> {
    let canonical = canonicalize(value);
    let encoded = serde_json::to_vec(&canonical)?;
    Ok(format!("sha256:{:x}", Sha256::digest(encoded)))
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        Value::Object(values) => Value::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize(value)))
                .collect(),
        ),
        value => value.clone(),
    }
}
