use assert_cmd::Command;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const SCHEMAS: [(&str, &str); 5] = [
    ("policy", "policy.schema.json"),
    ("evidence-bundle", "evidence-bundle.schema.json"),
    ("sensor-profile", "sensor-profile.schema.json"),
    ("pre-merge-decision", "pre-merge-decision.schema.json"),
    ("post-merge-decision", "post-merge-decision.schema.json"),
];

#[test]
fn checked_in_schemas_are_deterministic_and_enforce_contract_validation() {
    let schema_dir = workspace_root().join("schemas");
    let before: Vec<_> = SCHEMAS
        .iter()
        .map(|(_, file)| fs::read(schema_dir.join(file)).expect("checked-in schema should exist"))
        .collect();

    let nested_cwd = unique_temp_dir();
    fs::create_dir_all(&nested_cwd).unwrap();
    let output = Command::new(assert_cmd::cargo::cargo_bin!("xtask"))
        .arg("schema")
        .current_dir(&nested_cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let after: Vec<_> = SCHEMAS
        .iter()
        .map(|(_, file)| fs::read(schema_dir.join(file)).unwrap())
        .collect();
    assert_eq!(
        after, before,
        "schema regeneration changed checked-in files"
    );

    let ready: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/pre-merge/ready.json")).unwrap();
    let valid_bundle: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/evidence/valid.json")).unwrap();
    let pre_decision: Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/contracts/advisory-decision.json"
    ))
    .unwrap();
    let post_decision = json!({
        "schema_version": 1,
        "status": "inconclusive",
        "binding": {
            "repository": "FlitCatz/policy-kernel",
            "pull_request_number": 8,
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "evidence_digest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "evaluated_at": "2026-09-07T18:00:00+09:00"
        },
        "reasons": [],
        "decision_digest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    });
    let instances = [
        ready["policy"].clone(),
        valid_bundle,
        ready["context"]["sensor_profile"].clone(),
        pre_decision,
        post_decision,
    ];

    for (((name, _), schema_bytes), instance) in
        SCHEMAS.iter().zip(after.iter()).zip(instances.iter())
    {
        let schema: Value = serde_json::from_slice(schema_bytes).unwrap();
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .unwrap();
        assert!(
            validator.is_valid(instance),
            "{name} rejected its valid fixture"
        );

        let mut wrong_version = instance.clone();
        wrong_version["schema_version"] = json!(2);
        assert!(
            !validator.is_valid(&wrong_version),
            "{name} accepted an unsupported schema version"
        );

        let mut unknown_field = instance.clone();
        unknown_field
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), Value::Null);
        assert!(
            !validator.is_valid(&unknown_field),
            "{name} accepted an unknown field"
        );
    }

    assert_contract_constraints(&schema_dir, &instances);
    fs::remove_dir_all(nested_cwd).unwrap();
}

fn assert_contract_constraints(schema_dir: &Path, valid: &[Value; 5]) {
    let invalid_cases = [
        (
            0,
            mutate(&valid[0], |value| value["policy_digest"] = json!("short")),
        ),
        (
            0,
            mutate(&valid[0], |value| {
                value["required_sensor_ids"][0] = json!("   ")
            }),
        ),
        (
            1,
            mutate(&valid[1], |value| {
                value["manifest"]["repository"] = json!("owner/repo/extra")
            }),
        ),
        (
            1,
            mutate(&valid[1], |value| {
                value["manifest"]["base_sha"] = json!("CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC")
            }),
        ),
        (
            1,
            mutate(&valid[1], |value| {
                value["manifest"]["schema_version"] = json!(2)
            }),
        ),
        (
            2,
            mutate(&valid[2], |value| value["sensors"][0]["id"] = json!("")),
        ),
        (
            2,
            mutate(&valid[2], |value| {
                value["sensors"][0]
                    .as_object_mut()
                    .unwrap()
                    .insert("unexpected".into(), Value::Null);
            }),
        ),
        (
            3,
            mutate(&valid[3], |value| value["mode"] = json!("unsafe")),
        ),
        (
            3,
            mutate(&valid[3], |value| {
                value["binding"]["pull_request_number"] = json!(0)
            }),
        ),
        (
            3,
            mutate(&valid[3], |value| {
                value["binding"]["evaluated_at"] = json!("not-a-timestamp")
            }),
        ),
        (
            4,
            mutate(&valid[4], |value| {
                value["merge_commit_sha"] = json!("short")
            }),
        ),
    ];

    for (schema_index, invalid) in invalid_cases {
        let schema: Value =
            serde_json::from_slice(&fs::read(schema_dir.join(SCHEMAS[schema_index].1)).unwrap())
                .unwrap();
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .unwrap();
        assert!(
            !validator.is_valid(&invalid),
            "{} accepted data rejected by its typed contract",
            SCHEMAS[schema_index].0
        );
    }
}

fn mutate(value: &Value, update: impl FnOnce(&mut Value)) -> Value {
    let mut changed = value.clone();
    update(&mut changed);
    changed
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn unique_temp_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "policy-kernel-xtask-{}-{nanos}",
        std::process::id()
    ))
}
