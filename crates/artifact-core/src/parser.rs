use schemars::JsonSchema;
use serde::Deserialize;
use thiserror::Error;

use crate::{
    ApprovalStatus, Authority, Budget, DecisionArtifact, Evidence, Reconciliation, RiskClass,
    SourceSpan, Task,
};

#[derive(Debug, Error)]
pub enum ArtifactError {
    #[error("front matter must begin and end with '---'")]
    MissingFrontMatter,
    #[error("invalid front matter: {0}")]
    FrontMatter(serde_yaml::Error),
    #[error("invalid task '{id}': {source}")]
    Task {
        id: String,
        source: serde_yaml::Error,
    },
    #[error("task block '{0}' is not closed")]
    UnclosedTask(String),
    #[error("required Markdown section '{0}' is missing or empty")]
    MissingSection(String),
    #[error("task fence must declare a non-empty ID")]
    MissingTaskId,
}

#[derive(Deserialize, JsonSchema)]
struct FrontMatter {
    version: String,
    id: String,
    owner: String,
    status: ApprovalStatus,
    risk_class: RiskClass,
    authority: Authority,
    budgets: Budget,
    evidence: Vec<Evidence>,
    reconciliation: Reconciliation,
}

#[derive(Deserialize)]
struct TaskBody {
    objective: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    inputs: std::collections::BTreeMap<String, crate::InputBinding>,
    output_schema: serde_json::Value,
    #[serde(default)]
    acceptance: Vec<crate::AcceptanceCheck>,
    authority: Authority,
    #[serde(default)]
    gates: Vec<crate::Gate>,
}

/// Parse front matter, required Markdown sections, and fenced task declarations.
///
/// # Errors
/// Returns a source-oriented parse error when required structure or YAML is invalid.
pub fn parse_artifact(source: &str) -> Result<DecisionArtifact, ArtifactError> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.first().copied() != Some("---") {
        return Err(ArtifactError::MissingFrontMatter);
    }
    let end_front = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(index, line)| (*line == "---").then_some(index))
        .ok_or(ArtifactError::MissingFrontMatter)?;
    let front: FrontMatter = serde_yaml::from_str(&lines[1..end_front].join("\n"))
        .map_err(ArtifactError::FrontMatter)?;
    let objective = section(&lines, "## Objective")?;
    let non_goals = section(&lines, "## Non-goals")?;
    let tasks = task_blocks(&lines, end_front + 1)?;
    Ok(DecisionArtifact {
        version: front.version,
        id: front.id,
        owner: front.owner,
        status: front.status,
        risk_class: front.risk_class,
        objective,
        non_goals,
        authority: front.authority,
        budgets: front.budgets,
        evidence: front.evidence,
        reconciliation: front.reconciliation,
        tasks,
        span: SourceSpan {
            start_line: 1,
            start_column: 1,
            end_line: lines.len().max(1),
            end_column: lines.last().map_or(1, |line| line.len() + 1),
        },
    })
}

fn section(lines: &[&str], heading: &str) -> Result<String, ArtifactError> {
    let start = lines
        .iter()
        .position(|line| line.trim() == heading)
        .ok_or_else(|| ArtifactError::MissingSection(heading.to_owned()))?;
    let content = lines[start + 1..]
        .iter()
        .take_while(|line| !line.starts_with("## "))
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned();
    if content.is_empty() {
        return Err(ArtifactError::MissingSection(heading.to_owned()));
    }
    Ok(content)
}

fn task_blocks(lines: &[&str], start_at: usize) -> Result<Vec<Task>, ArtifactError> {
    let mut tasks = Vec::new();
    let mut index = start_at;
    while index < lines.len() {
        let Some(id) = lines[index].strip_prefix("```task") else {
            index += 1;
            continue;
        };
        let id = id.trim();
        if id.is_empty() {
            return Err(ArtifactError::MissingTaskId);
        }
        let start = index;
        let end = lines[index + 1..]
            .iter()
            .position(|line| line.trim() == "```")
            .map(|relative| index + 1 + relative)
            .ok_or_else(|| ArtifactError::UnclosedTask(id.to_owned()))?;
        let body: TaskBody =
            serde_yaml::from_str(&lines[index + 1..end].join("\n")).map_err(|source| {
                ArtifactError::Task {
                    id: id.to_owned(),
                    source,
                }
            })?;
        tasks.push(Task {
            id: id.to_owned(),
            objective: body.objective,
            dependencies: body.dependencies,
            evidence: body.evidence,
            inputs: body.inputs,
            output_schema: body.output_schema,
            acceptance: body.acceptance,
            authority: body.authority,
            gates: body.gates,
            span: SourceSpan {
                start_line: start + 1,
                start_column: 1,
                end_line: end + 1,
                end_column: lines[end].len() + 1,
            },
        });
        index = end + 1;
    }
    if tasks.is_empty() {
        return Err(ArtifactError::MissingSection("task blocks".to_owned()));
    }
    Ok(tasks)
}
