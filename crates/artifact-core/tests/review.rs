use artifact_core::{review_source, semantic_diff};

const EXAMPLE: &str = include_str!("../../../examples/repository-change.md");

#[test]
fn review_validation_matches_native_compilation() {
    let snapshot = review_source(EXAMPLE);
    assert!(snapshot.valid);
    let compiled = snapshot.compiled.expect("compiled");
    let native = artifact_core::compile_artifact(
        &artifact_core::parse_artifact(EXAMPLE).expect("native parse"),
    )
    .expect("native compile");
    assert_eq!(compiled, native);
}

#[test]
fn semantic_diff_highlights_authority_broadening() {
    let base = review_source(EXAMPLE).compiled.expect("base");
    let changed = EXAMPLE.replace(
        "network_domains: []",
        "network_domains: [\"api.example.com\"]",
    );
    let current = review_source(&changed).compiled.expect("current");
    let diff = semantic_diff(&base, &current);
    assert!(diff.changes.iter().any(|change| {
        change.authority_broadening && change.detail.contains("network:api.example.com")
    }));
}

#[test]
fn invalid_review_has_stable_diagnostics() {
    let snapshot = review_source("not an artifact");
    assert!(!snapshot.valid);
    assert_eq!(snapshot.diagnostics[0].code, "parse_error");
}
