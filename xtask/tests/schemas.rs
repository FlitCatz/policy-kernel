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
        wrong_version["schema_version"] = json!(99);
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
    let larger_than_u64: Value = serde_json::from_str("18446744073709551616").unwrap();
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
                value["sensors"][0]["id"] = json!("\u{0085}")
            }),
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
                value["binding"]["pull_request_number"] = larger_than_u64
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

    let sensor_schema: Value =
        serde_json::from_slice(&fs::read(schema_dir.join("sensor-profile.schema.json")).unwrap())
            .unwrap();
    let sensor_validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&sensor_schema)
        .unwrap();
    let rust_non_whitespace = mutate(&valid[2], |value| {
        value["sensors"][0]["id"] = json!("\u{feff}")
    });
    assert!(
        sensor_validator.is_valid(&rust_non_whitespace),
        "sensor-profile rejected a string ID accepted by Rust trim validation"
    );

    let pre_schema: Value = serde_json::from_slice(
        &fs::read(schema_dir.join("pre-merge-decision.schema.json")).unwrap(),
    )
    .unwrap();
    let pre_validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&pre_schema)
        .unwrap();
    let maximum_u64 = mutate(&valid[3], |value| {
        value["binding"]["pull_request_number"] = json!(u64::MAX)
    });
    assert!(
        pre_validator.is_valid(&maximum_u64),
        "pre-merge-decision rejected the maximum valid u64 pull request number"
    );
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

#[test]
fn sensor_profile_schema_matches_result_binding_deserialization() {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/sensor-profile.schema.json")).unwrap();
    let validator = jsonschema::draft202012::options().build(&schema).unwrap();
    let cases = [
        ("", json!(["cargo"]), false),
        (" \t\r\n", json!(["cargo"]), false),
        (
            "\u{0085}\u{00a0}\u{1680}\u{2007}\u{2028}\u{202f}\u{205f}\u{3000}",
            json!(["cargo"]),
            false,
        ),
        ("parser\0version", json!(["cargo"]), false),
        ("v1", json!([]), false),
        ("v1", json!([""]), false),
        ("v1", json!([" \t\n"]), false),
        ("v1", json!(["\u{0085}\u{3000}"]), false),
        ("v1", json!(["car\0go"]), false),
        ("v1", json!(["cargo", "test\0"]), false),
        ("v1", json!(["cargo", "", " \t", "test\n"]), true),
        (" v1\n", json!([" cargo\n"]), true),
        ("\u{feff}", json!(["\u{feff}"]), true),
        ("\u{200b}", json!(["\u{200b}"]), true),
    ];
    for (parser, command, expected) in cases {
        let profile = json!({
            "schema_version": 2,
            "sensors": [{
                "id": "unit", "required": true, "required_when": [],
                "result_requirement": "process",
                "result_binding": {
                    "configuration_digest": "b".repeat(64),
                    "parser_version": parser, "resolved_command": command
                }
            }],
            "adequacy": {
                "acceptance_criterion_ids": [], "changed_behavior_ids": [],
                "negative_case_ids": []
            }
        });
        assert_eq!(
            serde_json::from_value::<policy_contracts::SensorProfile>(profile.clone()).is_ok(),
            expected,
            "Rust: parser={parser:?}, command={command}"
        );
        assert_eq!(
            validator.is_valid(&profile),
            expected,
            "schema: parser={parser:?}, command={command}"
        );
    }
}

#[test]
fn sensor_profile_schema_matches_versioned_result_requirements() {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/sensor-profile.schema.json")).unwrap();
    let validator = jsonschema::draft202012::options().build(&schema).unwrap();
    let legacy: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/sensors/profile.json")).unwrap();
    assert!(validator.is_valid(&legacy));
    let mut v2 = legacy.clone();
    v2["schema_version"] = json!(2);
    assert!(!validator.is_valid(&v2));
    for definition in v2["sensors"].as_array_mut().unwrap() {
        definition["result_requirement"] = json!("test_counts");
    }
    assert!(validator.is_valid(&v2));
    for requirement in [json!("process"), json!("test_counts")] {
        v2["sensors"][0]["result_requirement"] = requirement.clone();
        assert!(validator.is_valid(&v2));
        let mut wrong_v1 = legacy.clone();
        wrong_v1["sensors"][0]["result_requirement"] = requirement;
        assert!(!validator.is_valid(&wrong_v1));
    }
    for invalid in [Value::Null, json!("unknown")] {
        v2["sensors"][0]["result_requirement"] = invalid;
        assert!(!validator.is_valid(&v2));
    }
}
