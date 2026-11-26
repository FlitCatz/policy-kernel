use std::{fs, process::Command};

#[test]
fn invalid_repository_is_unknown_and_writes_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let evidence = directory.path().join("result.json");
    let result = Command::new(env!("CARGO_BIN_EXE_policy-cli"))
        .args([
            "gate",
            "--repo",
            "invalid",
            "--pr",
            "42",
            "--policy",
            "gate.json",
            "--output",
        ])
        .arg(&evidence)
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(3), "{result:?}");
    assert_eq!(
        String::from_utf8(result.stdout).unwrap(),
        "unknown: expected repository in owner/name form\n"
    );
    let record: serde_json::Value = serde_json::from_slice(&fs::read(evidence).unwrap()).unwrap();
    assert_eq!(record["verdict"], "unknown");
    assert_eq!(record["policy_path"], "gate.json");
}
