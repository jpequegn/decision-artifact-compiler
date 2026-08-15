use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Authority, DecisionArtifact, GateKind, RiskClass, SourceSpan};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub code: String,
    pub message: String,
    pub task_id: Option<String>,
    pub source: SourceSpan,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PolicyReport {
    pub allowed: bool,
    pub decisions: Vec<PolicyDecision>,
}

#[derive(Debug, Error)]
#[error("artifact policy denied execution")]
pub struct PolicyError {
    pub report: PolicyReport,
}

/// Enforce least-privilege and budget invariants over a parsed artifact.
///
/// # Errors
/// Returns source-linked denial decisions when any task broadens authority,
/// budgets are unusable, or required approval gates are absent.
pub fn enforce_policy(artifact: &DecisionArtifact) -> Result<PolicyReport, PolicyError> {
    let mut decisions = budget_decisions(artifact);
    for task in &artifact.tasks {
        compare_authority(
            &artifact.authority,
            &task.authority,
            &task.id,
            &task.span,
            &mut decisions,
        );
        let needs_approval = artifact.risk_class == RiskClass::Consequential
            || !task.authority.side_effects.is_empty();
        if needs_approval
            && !task
                .gates
                .iter()
                .any(|gate| gate.kind == GateKind::Approval && gate.approver.is_some())
        {
            decisions.push(denial(
                "approval_gate_required",
                "consequential or side-effecting tasks require a named approval gate",
                Some(&task.id),
                &task.span,
                task.authority.side_effects.clone(),
            ));
        }
    }
    if decisions.is_empty() {
        decisions.push(PolicyDecision {
            allowed: true,
            code: "policy_allowed".to_owned(),
            message: "all declared task capabilities are within artifact authority".to_owned(),
            task_id: None,
            source: artifact.span.clone(),
            evidence: Vec::new(),
        });
    }
    let report = PolicyReport {
        allowed: decisions.iter().all(|decision| decision.allowed),
        decisions,
    };
    if report.allowed {
        Ok(report)
    } else {
        Err(PolicyError { report })
    }
}

fn budget_decisions(artifact: &DecisionArtifact) -> Vec<PolicyDecision> {
    let mut decisions = Vec::new();
    for (name, value) in [
        ("time_ms", artifact.budgets.time_ms),
        ("token_limit", artifact.budgets.token_limit),
        ("cost_micros", artifact.budgets.cost_micros),
        (
            "concurrency_limit",
            u64::try_from(artifact.budgets.concurrency_limit).unwrap_or(0),
        ),
    ] {
        if value == 0 {
            decisions.push(denial(
                "budget_must_be_positive",
                &format!("{name} must be greater than zero"),
                None,
                &artifact.span,
                vec![name.to_owned()],
            ));
        }
    }
    decisions
}

fn compare_authority(
    parent: &Authority,
    child: &Authority,
    task_id: &str,
    source: &SourceSpan,
    decisions: &mut Vec<PolicyDecision>,
) {
    for (scope, allowed, requested) in [
        ("read_path", &parent.read_paths, &child.read_paths),
        ("write_path", &parent.write_paths, &child.write_paths),
        ("command", &parent.commands, &child.commands),
        (
            "network_domain",
            &parent.network_domains,
            &child.network_domains,
        ),
        ("secret", &parent.secrets, &child.secrets),
        ("side_effect", &parent.side_effects, &child.side_effects),
    ] {
        for value in requested {
            if !allowed.iter().any(|grant| scope_contains(grant, value)) {
                decisions.push(denial(
                    "authority_scope_exceeded",
                    &format!("task requests undeclared {scope} capability `{value}`"),
                    Some(task_id),
                    source,
                    vec![format!("{scope}:{value}")],
                ));
            }
        }
    }
}

fn scope_contains(grant: &str, requested: &str) -> bool {
    if grant == requested {
        return true;
    }
    if let Some(prefix) = grant.strip_suffix("/**") {
        return requested == prefix || requested.starts_with(&format!("{prefix}/"));
    }
    if let Some(suffix) = grant.strip_prefix("*.") {
        return requested.ends_with(&format!(".{suffix}"));
    }
    false
}

fn denial(
    code: &str,
    message: &str,
    task_id: Option<&str>,
    source: &SourceSpan,
    evidence: Vec<String>,
) -> PolicyDecision {
    PolicyDecision {
        allowed: false,
        code: code.to_owned(),
        message: message.to_owned(),
        task_id: task_id.map(str::to_owned),
        source: source.clone(),
        evidence,
    }
}
