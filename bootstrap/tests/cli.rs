#![cfg(unix)]
use assert_cmd::Command;

#[test]
fn plan_output_round_trips_to_apply_and_conflict_exit_code_is_distinct() {
    let target = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let root = target.path().canonicalize().unwrap();
    let output = Command::new(assert_cmd::cargo::cargo_bin!("policy-bootstrap"))
        .args([
            "plan",
            "--profile",
            "dotnet",
            "--release",
            "1.0.0",
            "--target",
        ])
        .arg(&root)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 0);
    let saved = outside.path().join("plan.json");
    std::fs::write(&saved, output).unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("policy-bootstrap"))
        .args(["apply", "--plan"])
        .arg(&saved)
        .assert()
        .success();
    assert!(root.join(".policy/bootstrap-manifest.json").is_file());
    std::fs::write(root.join("global.json"), "local edit").unwrap();
    Command::new(assert_cmd::cargo::cargo_bin!("policy-bootstrap"))
        .args([
            "inspect",
            "--profile",
            "dotnet",
            "--release",
            "1.0.0",
            "--target",
        ])
        .arg(&root)
        .assert()
        .code(2);
    Command::new(assert_cmd::cargo::cargo_bin!("policy-bootstrap"))
        .args(["apply", "--plan"])
        .arg(saved)
        .assert()
        .code(64);
}
