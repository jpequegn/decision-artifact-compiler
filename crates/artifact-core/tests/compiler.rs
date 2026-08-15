use artifact_core::{compile_artifact, compile_report, export_plan, parse_artifact};

const EXAMPLE: &str = include_str!("../../../examples/repository-change.md");

#[test]
fn canonical_digests_ignore_declaration_order() {
    let original = parse_artifact(EXAMPLE).expect("parse");
    let mut reordered = original.clone();
    reordered.tasks.reverse();
    reordered.evidence.reverse();
    reordered.authority.commands.reverse();
    let first = compile_artifact(&original).expect("compile");
    let second = compile_artifact(&reordered).expect("compile reordered");
    assert_eq!(first.artifact_digest, second.artifact_digest);
    assert_eq!(first.tasks, second.tasks);
}

#[test]
fn policy_and_evidence_changes_are_compartmentalized() {
    let original = parse_artifact(EXAMPLE).expect("parse");
    let first = compile_artifact(&original).expect("compile");

    let mut authority_change = original.clone();
    authority_change
        .authority
        .commands
        .push("cargo test".to_owned());
    let authority = compile_artifact(&authority_change).expect("authority compile");
    assert_ne!(first.policy_digest, authority.policy_digest);
    assert_ne!(first.artifact_digest, authority.artifact_digest);

    let mut evidence_change = original;
    evidence_change.evidence[0].digest = "sha256:changed".to_owned();
    let evidence = compile_artifact(&evidence_change).expect("evidence compile");
    assert_ne!(
        first.evidence[0].evidence_digest,
        evidence.evidence[0].evidence_digest
    );
    assert_ne!(
        first.tasks[0].context_digest,
        evidence.tasks[0].context_digest
    );
}

#[test]
fn compiled_ir_round_trips_and_exports_a_runner_plan() {
    let compiled = compile_artifact(&parse_artifact(EXAMPLE).expect("parse")).expect("compile");
    let encoded = serde_json::to_string(&compiled).expect("serialize");
    let decoded: artifact_core::CompiledArtifact = serde_json::from_str(&encoded).expect("decode");
    assert_eq!(compiled, decoded);

    let plan = export_plan(&compiled);
    assert_eq!(plan.nodes.len(), 3);
    assert_eq!(plan.nodes[0].tool, "artifact-worker");
    assert!(plan.nodes.iter().all(|node| node.source.start_line > 0));
    assert!(compile_report(&compiled).contains("## Task graph"));
}
