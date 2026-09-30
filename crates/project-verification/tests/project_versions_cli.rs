use project_verification::{
    parse_profile,
    selection::{ci_inputs, upgrade_frontend_profile},
};
use serde_json::{json, Value};
use std::{fs, process::Command};

fn legacy() -> project_verification::ProjectProfile {
    parse_profile(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap()
}

#[test]
fn migration_preserves_requested_ranges_and_explicit_provisioning() {
    let candidate = upgrade_frontend_profile(legacy()).unwrap();
    let value = serde_json::to_value(&candidate).unwrap();
    assert_eq!(value["schema_version"], json!(4));
    assert_eq!(value["version"], json!("1.1.0"));
    assert_eq!(
        value["selection"]["tools"]["jest"]["version"]["acceptance"],
        json!({"kind":"numeric_range","requirement":">=30.2.0, <31.0.0"})
    );
    assert_eq!(value["selection"]["provisioning"]["npm"], json!("10.9.4"));
    assert_eq!(ci_inputs(&candidate).unwrap()["npm_version"], "10.9.4");
    let mut without = value;
    without["selection"]
        .as_object_mut()
        .unwrap()
        .remove("provisioning");
    let without = parse_profile(&without.to_string()).unwrap();
    assert!(!ci_inputs(&without).unwrap().contains_key("npm_version"));
}

#[test]
fn explicit_cli_migration_only_writes_stdout() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("profile.json");
    let original = include_str!("../../../examples/react-jest/project-profile.json");
    fs::write(&file, original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_project-verification"))
        .args([
            "migrate-profile",
            file.to_str().unwrap(),
            "--target-schema",
            "4",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], json!(4));
    assert_eq!(fs::read_to_string(file).unwrap(), original);
}
