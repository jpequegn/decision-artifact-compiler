use std::fmt;
use std::sync::Arc;

use artifact_core::{
    compile_artifact, enforce_policy, parse_artifact, reconcile_results, validate_artifact,
};
use artifact_runtime::{DispatchOptions, FakeWorker, Ledger, dispatch};
use serde::{Deserialize, Serialize};

const EXAMPLE: &str = include_str!("../../../examples/repository-change.md");

#[derive(Debug, Deserialize)]
struct Case {
    mutation: String,
    valid: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalCheck {
    Pass,
    Fail,
}

impl EvalCheck {
    fn from_bool(value: bool) -> Self {
        if value { Self::Pass } else { Self::Fail }
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self == &Self::Pass
    }
}

impl fmt::Display for EvalCheck {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EvaluationReport {
    pub corpus_cases: usize,
    pub valid_cases: usize,
    pub invalid_cases: usize,
    pub correct_classifications: usize,
    pub compile_correctness_pct: f64,
    pub blocker_precision_pct: f64,
    pub injection_inert: EvalCheck,
    pub authority_broadening_blocked: EvalCheck,
    pub conflict_detected: EvalCheck,
    pub replay_parity: EvalCheck,
    pub contract_context_bytes: usize,
    pub chat_baseline_context_bytes: usize,
    pub context_duplication_reduction_pct: f64,
    pub source_linked_reviewability_pct: f64,
}

/// Run the deterministic release evaluation.
///
/// # Errors
/// Returns parsing, compilation, runtime, or temporary-ledger failures.
pub fn run_evaluation() -> Result<EvaluationReport, Box<dyn std::error::Error>> {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../fixtures/golden/cases.json"))?;
    let mut correct = 0;
    let mut blocked = 0;
    let invalid_cases = cases.iter().filter(|case| !case.valid).count();
    for case in &cases {
        let source = mutate(EXAMPLE, &case.mutation);
        let accepted = parse_artifact(&source).is_ok_and(|artifact| {
            validate_artifact(&artifact).is_ok() && enforce_policy(&artifact).is_ok()
        });
        correct += usize::from(accepted == case.valid);
        blocked += usize::from(!case.valid && !accepted);
    }

    let artifact = parse_artifact(EXAMPLE)?;
    let compiled = compile_artifact(&artifact)?;
    let injection_inert = {
        let mut injected = artifact.clone();
        include_str!("../../../fixtures/adversarial/prompt-injection.txt")
            .clone_into(&mut injected.evidence[0].description);
        enforce_policy(&injected).is_ok()
            && injected.authority.network_domains.is_empty()
            && injected.authority.secrets.is_empty()
    };
    let authority_broadening_blocked = {
        let mut broadened = artifact.clone();
        broadened.tasks[0]
            .authority
            .network_domains
            .push("attacker.invalid".to_owned());
        enforce_policy(&broadened).is_err()
    };

    let directory = tempfile::tempdir()?;
    let ledger = Ledger::open(directory.path().join("evaluation.db"))?;
    let runtime = tokio::runtime::Runtime::new()?;
    let summary = runtime.block_on(dispatch(
        &compiled,
        Arc::new(FakeWorker::default()),
        &ledger,
        &DispatchOptions::default(),
    ))?;
    let replay_parity = ledger.replay(&summary.run_id)? == summary;
    let conflict_detected = {
        let mut results = summary.results.clone();
        let Some(mut parallel) = results
            .iter()
            .find(|result| result.task_id == "inspect")
            .cloned()
        else {
            return Err("sample dispatch omitted inspect result".into());
        };
        "parallel-evaluation".clone_into(&mut parallel.run_id);
        parallel.output = serde_json::json!({"conflicting": true});
        results.push(parallel);
        reconcile_results(&compiled, &results).is_err_and(|error| {
            error
                .diagnostics
                .iter()
                .any(|item| item.code == "conflicting_parallel_results")
        })
    };

    let chat_bytes = EXAMPLE.len() * compiled.tasks.len();
    let evidence_bytes: usize = compiled
        .evidence
        .iter()
        .map(|item| item.description.len())
        .sum();
    let contract_bytes = compiled
        .tasks
        .iter()
        .map(|task| task.objective.value.len() + evidence_bytes)
        .sum();
    let linked = compiled
        .tasks
        .iter()
        .filter(|task| task.source.start_line > 0)
        .count();
    Ok(EvaluationReport {
        corpus_cases: cases.len(),
        valid_cases: cases.len() - invalid_cases,
        invalid_cases,
        correct_classifications: correct,
        compile_correctness_pct: percent(correct, cases.len()),
        blocker_precision_pct: percent(blocked, invalid_cases),
        injection_inert: EvalCheck::from_bool(injection_inert),
        authority_broadening_blocked: EvalCheck::from_bool(authority_broadening_blocked),
        conflict_detected: EvalCheck::from_bool(conflict_detected),
        replay_parity: EvalCheck::from_bool(replay_parity),
        contract_context_bytes: contract_bytes,
        chat_baseline_context_bytes: chat_bytes,
        context_duplication_reduction_pct: 100.0 - percent(contract_bytes, chat_bytes),
        source_linked_reviewability_pct: percent(linked, compiled.tasks.len()),
    })
}

#[must_use]
pub fn render_markdown(report: &EvaluationReport) -> String {
    format!(
        "# Evaluation report\n\n| Metric | Result |\n| --- | ---: |\n| Golden corpus | {} cases |\n| Compile correctness | {:.1}% |\n| Blocker precision | {:.1}% |\n| Prompt injection inert | {} |\n| Authority broadening blocked | {} |\n| Conflict detected | {} |\n| Replay parity | {} |\n| Contract context | {} bytes |\n| Chat baseline context | {} bytes |\n| Context duplication reduction | {:.1}% |\n| Source-linked reviewability | {:.1}% |\n\nThe chat baseline repeats the entire approved artifact for every worker. The contract path sends only each task objective and declared evidence descriptions; authority is transmitted separately as typed policy, not prompt text.\n",
        report.corpus_cases,
        report.compile_correctness_pct,
        report.blocker_precision_pct,
        report.injection_inert,
        report.authority_broadening_blocked,
        report.conflict_detected,
        report.replay_parity,
        report.contract_context_bytes,
        report.chat_baseline_context_bytes,
        report.context_duplication_reduction_pct,
        report.source_linked_reviewability_pct,
    )
}

#[must_use]
pub fn render_csv(report: &EvaluationReport) -> String {
    format!(
        "metric,value\ncorpus_cases,{}\ncompile_correctness_pct,{:.1}\nblocker_precision_pct,{:.1}\ninjection_inert,{}\nauthority_broadening_blocked,{}\nconflict_detected,{}\nreplay_parity,{}\ncontract_context_bytes,{}\nchat_baseline_context_bytes,{}\ncontext_duplication_reduction_pct,{:.1}\nsource_linked_reviewability_pct,{:.1}\n",
        report.corpus_cases,
        report.compile_correctness_pct,
        report.blocker_precision_pct,
        report.injection_inert,
        report.authority_broadening_blocked,
        report.conflict_detected,
        report.replay_parity,
        report.contract_context_bytes,
        report.chat_baseline_context_bytes,
        report.context_duplication_reduction_pct,
        report.source_linked_reviewability_pct,
    )
}

fn percent(part: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        let part = u32::try_from(part).unwrap_or(u32::MAX);
        let total = u32::try_from(total).unwrap_or(u32::MAX);
        (f64::from(part) / f64::from(total)) * 100.0
    }
}

fn mutate(source: &str, mutation: &str) -> String {
    match mutation {
        "none" => source.to_owned(),
        "owner" => source.replace("owner: julien", "owner: reviewer"),
        "objective" => source.replace("Implement and verify", "Inspect and implement"),
        "non_goals" => source.replace("Do not publish", "Do not deploy"),
        "risk" => source.replace("risk_class: low", "risk_class: medium"),
        "budget_time" => source.replace("time_ms: 120000", "time_ms: 60000"),
        "budget_tokens" => source.replace("token_limit: 20000", "token_limit: 10000"),
        "evidence_uri" => source.replace("file://REQUEST.md", "file://APPROVED.md"),
        "task_objective" => source.replace("Inspect the relevant", "Review the relevant"),
        "digest" => source.replace("sha256:2c26", "sha256:aaaa"),
        "concurrency_one" => source.replace("concurrency_limit: 2", "concurrency_limit: 1"),
        "status" => source.replace("status: approved", "status: draft"),
        "read_path" => source.replace("Cargo.toml\"]", "README.md\"]"),
        "command" => source.replace("cargo clippy", "cargo check"),
        "cost" => source.replace("cost_micros: 500000", "cost_micros: 100000"),
        "retry" => source.replace("retry_limit: 1", "retry_limit: 2"),
        "description" => source.replace("Approved repository", "Reviewed repository"),
        "task_schema" => source.replace("required: [findings]", "required: [summary]"),
        "acceptance" => source.replace("value: findings", "value: summary"),
        "id" => source.replace("id: repository-change", "id: repository-review"),
        "version" => source.replace("version: v1", "version: v2"),
        "zero_concurrency" => source.replace("concurrency_limit: 2", "concurrency_limit: 0"),
        "missing_front" => source.replacen("---", "", 1),
        "missing_objective" => source.replace("## Objective", "## Goal"),
        "empty_objective" => source.replace("Implement and verify a scoped repository change.", ""),
        "missing_non_goals" => source.replace("## Non-goals", "## Exclusions"),
        "missing_tasks" => source.replace("```task", "```worker"),
        "unclosed_task" => source
            .rsplit_once("```")
            .map_or_else(|| source.to_owned(), |(head, tail)| format!("{head}{tail}")),
        "missing_task_id" => source.replacen("```task inspect", "```task", 1),
        "bad_front_yaml" => source.replace("owner: julien", "owner: ["),
        "bad_task_yaml" => source.replacen("objective: Inspect", "objective: [", 1),
        "duplicate_task" => source.replace("```task implement", "```task inspect"),
        "missing_dependency" => source.replace("dependencies: [inspect]", "dependencies: [absent]"),
        "cycle" => source.replace("dependencies: [implement]", "dependencies: [verify]"),
        "missing_evidence" => source.replacen("evidence: [request]", "evidence: [absent]", 1),
        "duplicate_evidence" => source.replace(
            "reconciliation:\n",
            "  - id: request\n    uri: file://COPY\n    digest: copy\nreconciliation:\n",
        ),
        "bad_task_reference" => source.replace("task: inspect", "task: verify"),
        "unknown_status" => source.replace("status: approved", "status: accepted"),
        "unknown_risk" => source.replace("risk_class: low", "risk_class: extreme"),
        "missing_budget" => source.replace("  concurrency_limit: 2\n", ""),
        other => panic!("unknown mutation {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn release_evaluation_meets_thresholds() {
        let report = run_evaluation().expect("evaluation");
        assert_eq!(report.corpus_cases, 40);
        assert!((report.compile_correctness_pct - 100.0).abs() < f64::EPSILON);
        assert!((report.blocker_precision_pct - 100.0).abs() < f64::EPSILON);
        assert!(report.context_duplication_reduction_pct > 80.0);
        assert!(report.injection_inert.passed());
        assert!(report.conflict_detected.passed());
        assert!(report.replay_parity.passed());
    }
}
