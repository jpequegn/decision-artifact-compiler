use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn help_describes_the_command_surface() {
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("compile"))
        .stdout(predicate::str::contains("schema"));
}

#[test]
fn prints_supported_format_version() {
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .arg("format-version")
        .assert()
        .success()
        .stdout("v1\n");
}

#[test]
fn validates_and_compiles_the_example() {
    let example = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/repository-change.md"
    );
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args(["validate", example])
        .assert()
        .success()
        .stdout(predicate::str::contains("3 tasks"));
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args(["compile", example])
        .assert()
        .success()
        .stdout(predicate::str::contains("artifact_digest"));
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args(["compile", example, "--format", "plan"])
        .assert()
        .success()
        .stdout(predicate::str::contains("artifact-worker"));
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args(["compile", example, "--format", "report"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Compile report"));
}

#[test]
fn prints_schema() {
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .arg("schema")
        .assert()
        .success()
        .stdout(predicate::str::contains("concurrency_limit"));
}

#[test]
fn runs_the_documented_end_to_end_workflow() {
    let example = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/repository-change.md"
    );
    let directory = tempfile::tempdir().expect("tempdir");
    let ledger = directory.path().join("runs.db");
    let results = directory.path().join("results.json");
    let patch = directory.path().join("proposal.md");
    let output = Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args([
            "dispatch",
            example,
            "--ledger",
            ledger.to_str().expect("ledger path"),
            "--results",
            results.to_str().expect("results path"),
        ])
        .output()
        .expect("dispatch output");
    assert!(output.status.success());
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout).expect("summary");
    let run_id = summary["run_id"].as_str().expect("run id");

    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args([
            "replay",
            run_id,
            "--ledger",
            ledger.to_str().expect("ledger path"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("completed"));
    Command::cargo_bin("decision-artifact")
        .expect("binary")
        .args([
            "reconcile",
            example,
            results.to_str().expect("results path"),
            "--output",
            patch.to_str().expect("patch path"),
        ])
        .assert()
        .success();
    assert!(
        std::fs::read_to_string(patch)
            .expect("proposal")
            .contains("Reconciliation proposal")
    );
}
