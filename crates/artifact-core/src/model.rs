use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct SourceSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalStatus {
    Draft,
    Approved,
    Superseded,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    Low,
    Medium,
    High,
    Consequential,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Authority {
    #[serde(default)]
    pub read_paths: Vec<String>,
    #[serde(default)]
    pub write_paths: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub network_domains: Vec<String>,
    #[serde(default)]
    pub secrets: Vec<String>,
    #[serde(default)]
    pub side_effects: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Budget {
    pub time_ms: u64,
    pub token_limit: u64,
    pub cost_micros: u64,
    pub retry_limit: u32,
    pub concurrency_limit: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Evidence {
    pub id: String,
    pub uri: String,
    pub digest: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputBinding {
    Literal { value: Value },
    Evidence { evidence: String },
    Task { task: String, path: Option<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceKind {
    Command,
    JsonSchema,
    Evidence,
    HumanReview,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct AcceptanceCheck {
    pub kind: AcceptanceKind,
    pub value: String,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GateKind {
    Approval,
    Evidence,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Gate {
    pub id: String,
    pub kind: GateKind,
    pub approver: Option<String>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct Task {
    pub id: String,
    pub objective: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub inputs: BTreeMap<String, InputBinding>,
    pub output_schema: Value,
    #[serde(default)]
    pub acceptance: Vec<AcceptanceCheck>,
    pub authority: Authority,
    #[serde(default)]
    pub gates: Vec<Gate>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconciliationMode {
    ProposedPatch,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Reconciliation {
    pub mode: ReconciliationMode,
    #[serde(default)]
    pub require_all_evidence: bool,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
pub struct DecisionArtifact {
    pub version: String,
    pub id: String,
    pub owner: String,
    pub status: ApprovalStatus,
    pub risk_class: RiskClass,
    pub objective: String,
    pub non_goals: String,
    pub authority: Authority,
    pub budgets: Budget,
    pub evidence: Vec<Evidence>,
    pub reconciliation: Reconciliation,
    pub tasks: Vec<Task>,
    pub span: SourceSpan,
}
