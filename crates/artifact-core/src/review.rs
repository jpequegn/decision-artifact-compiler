use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    CompiledArtifact, compile_artifact, enforce_policy, parse_artifact, validate_artifact,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ReviewDiagnostic {
    pub code: String,
    pub message: String,
    pub start_line: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReviewSnapshot {
    pub valid: bool,
    pub diagnostics: Vec<ReviewDiagnostic>,
    pub compiled: Option<CompiledArtifact>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SemanticChange {
    pub kind: String,
    pub path: String,
    pub detail: String,
    pub authority_broadening: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactDiff {
    pub base_digest: String,
    pub current_digest: String,
    pub changes: Vec<SemanticChange>,
}

#[must_use]
pub fn review_source(source: &str) -> ReviewSnapshot {
    let artifact = match parse_artifact(source) {
        Ok(artifact) => artifact,
        Err(error) => {
            return invalid("parse_error", &error.to_string(), None);
        }
    };
    if let Err(error) = validate_artifact(&artifact) {
        let diagnostics = match error {
            crate::ValidationError::Invalid { diagnostics } => diagnostics
                .into_iter()
                .map(|item| ReviewDiagnostic {
                    code: item.code,
                    message: item.message,
                    start_line: Some(item.span.start_line),
                })
                .collect(),
            error @ crate::ValidationError::Schema(_) => vec![ReviewDiagnostic {
                code: "validation_error".to_owned(),
                message: error.to_string(),
                start_line: None,
            }],
        };
        return ReviewSnapshot {
            valid: false,
            diagnostics,
            compiled: None,
        };
    }
    if let Err(error) = enforce_policy(&artifact) {
        return ReviewSnapshot {
            valid: false,
            diagnostics: error
                .report
                .decisions
                .into_iter()
                .map(|item| ReviewDiagnostic {
                    code: item.code,
                    message: item.message,
                    start_line: Some(item.source.start_line),
                })
                .collect(),
            compiled: None,
        };
    }
    match compile_artifact(&artifact) {
        Ok(compiled) => ReviewSnapshot {
            valid: true,
            diagnostics: Vec::new(),
            compiled: Some(compiled),
        },
        Err(error) => invalid("compile_error", &error.to_string(), None),
    }
}

#[must_use]
pub fn semantic_diff(base: &CompiledArtifact, current: &CompiledArtifact) -> ArtifactDiff {
    let mut changes = Vec::new();
    if base.objective.value != current.objective.value {
        changes.push(change(
            "modified",
            "objective",
            "objective text changed",
            false,
        ));
    }
    compare_authority(
        "authority",
        &base.authority,
        &current.authority,
        &mut changes,
    );
    if base.budgets != current.budgets {
        changes.push(change(
            "modified",
            "budgets",
            "execution budgets changed",
            false,
        ));
    }
    compare_evidence(base, current, &mut changes);
    let base_tasks: BTreeMap<_, _> = base.tasks.iter().map(|task| (&task.id, task)).collect();
    let current_tasks: BTreeMap<_, _> = current.tasks.iter().map(|task| (&task.id, task)).collect();
    for (id, task) in &current_tasks {
        if let Some(previous) = base_tasks.get(id) {
            compare_authority(
                &format!("tasks.{id}.authority"),
                &previous.authority,
                &task.authority,
                &mut changes,
            );
            if previous.task_digest != task.task_digest {
                changes.push(change(
                    "modified",
                    &format!("tasks.{id}"),
                    "task declaration changed",
                    false,
                ));
            }
        } else {
            changes.push(change(
                "added",
                &format!("tasks.{id}"),
                "task added",
                !authority_values(&task.authority).is_empty(),
            ));
        }
    }
    for id in base_tasks
        .keys()
        .filter(|id| !current_tasks.contains_key(*id))
    {
        changes.push(change(
            "removed",
            &format!("tasks.{id}"),
            "task removed",
            false,
        ));
    }
    changes.sort_by(|left, right| (&left.path, &left.kind).cmp(&(&right.path, &right.kind)));
    ArtifactDiff {
        base_digest: base.artifact_digest.clone(),
        current_digest: current.artifact_digest.clone(),
        changes,
    }
}

fn compare_evidence(
    base: &CompiledArtifact,
    current: &CompiledArtifact,
    changes: &mut Vec<SemanticChange>,
) {
    let previous: BTreeMap<_, _> = base
        .evidence
        .iter()
        .map(|item| (&item.id, &item.evidence_digest))
        .collect();
    for evidence in &current.evidence {
        match previous.get(&evidence.id) {
            None => changes.push(change(
                "added",
                &format!("evidence.{}", evidence.id),
                "evidence added",
                false,
            )),
            Some(digest) if **digest != evidence.evidence_digest => changes.push(change(
                "modified",
                &format!("evidence.{}", evidence.id),
                "evidence digest changed",
                false,
            )),
            _ => {}
        }
    }
}

fn compare_authority(
    path: &str,
    base: &crate::Authority,
    current: &crate::Authority,
    changes: &mut Vec<SemanticChange>,
) {
    let before: BTreeSet<_> = authority_values(base).into_iter().collect();
    let after: BTreeSet<_> = authority_values(current).into_iter().collect();
    for capability in after.difference(&before) {
        changes.push(change(
            "added",
            path,
            &format!("capability added: {capability}"),
            true,
        ));
    }
    for capability in before.difference(&after) {
        changes.push(change(
            "removed",
            path,
            &format!("capability removed: {capability}"),
            false,
        ));
    }
}

fn authority_values(authority: &crate::Authority) -> Vec<String> {
    [
        ("read", &authority.read_paths),
        ("write", &authority.write_paths),
        ("command", &authority.commands),
        ("network", &authority.network_domains),
        ("secret", &authority.secrets),
        ("side_effect", &authority.side_effects),
    ]
    .into_iter()
    .flat_map(|(kind, values)| values.iter().map(move |value| format!("{kind}:{value}")))
    .collect()
}

fn invalid(code: &str, message: &str, start_line: Option<usize>) -> ReviewSnapshot {
    ReviewSnapshot {
        valid: false,
        diagnostics: vec![ReviewDiagnostic {
            code: code.to_owned(),
            message: message.to_owned(),
            start_line,
        }],
        compiled: None,
    }
}

fn change(kind: &str, path: &str, detail: &str, authority_broadening: bool) -> SemanticChange {
    SemanticChange {
        kind: kind.to_owned(),
        path: path.to_owned(),
        detail: detail.to_owned(),
        authority_broadening,
    }
}
