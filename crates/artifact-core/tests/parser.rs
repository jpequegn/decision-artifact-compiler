use artifact_core::{ValidationError, artifact_json_schema, parse_artifact, validate_artifact};
use serde::Deserialize;

const EXAMPLE: &str = include_str!("../../../examples/repository-change.md");

#[derive(Deserialize)]
struct Case {
    name: String,
    mutation: String,
    valid: bool,
}

#[test]
fn parses_valid_artifact_with_task_spans() {
    let artifact = parse_artifact(EXAMPLE).expect("parse example");
    validate_artifact(&artifact).expect("validate example");
    assert_eq!(artifact.tasks.len(), 3);
    assert_eq!(artifact.tasks[0].id, "inspect");
    assert!(artifact.tasks[0].span.start_line > 1);
    assert!(artifact.tasks[0].span.end_line > artifact.tasks[0].span.start_line);
}

#[test]
fn emits_stable_graph_and_reference_diagnostics() {
    let source = EXAMPLE.replace("dependencies: [implement]", "dependencies: [verify]");
    let artifact = parse_artifact(&source).expect("parse cycle");
    let ValidationError::Invalid { diagnostics } =
        validate_artifact(&artifact).expect_err("cycle must fail")
    else {
        panic!("unexpected error");
    };
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "dependency_cycle")
    );
    assert!(diagnostics.iter().all(|item| item.span.start_line > 0));
}

#[test]
fn schema_is_deterministic() {
    let first = artifact_json_schema().expect("schema");
    assert_eq!(first, artifact_json_schema().expect("schema again"));
    assert!(first.contains("concurrency_limit"));
    assert!(first.contains("network_domains"));
}

#[test]
fn forty_case_golden_corpus_matches_expectations() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../../../fixtures/golden/cases.json"))
            .expect("golden manifest");
    assert_eq!(cases.len(), 40);
    for case in cases {
        let source = mutate(EXAMPLE, &case.mutation);
        let actual =
            parse_artifact(&source).is_ok_and(|artifact| validate_artifact(&artifact).is_ok());
        assert_eq!(actual, case.valid, "golden case {}", case.name);
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
