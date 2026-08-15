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
