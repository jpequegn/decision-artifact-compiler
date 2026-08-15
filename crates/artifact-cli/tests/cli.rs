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
