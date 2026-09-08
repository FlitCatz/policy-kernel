use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use std::{fs, path::PathBuf, process::Command};

const SCHEMAS: [&str; 5] = [
    "policy.schema.json",
    "evidence-bundle.schema.json",
    "sensor-profile.schema.json",
    "pre-merge-decision.schema.json",
    "post-merge-decision.schema.json",
];

fn workspace_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path)
}

fn schema_path(name: &str) -> PathBuf {
    workspace_path(&format!("schemas/{name}"))
}

fn read_schema(name: &str) -> Value {
    serde_json::from_slice(&fs::read(schema_path(name)).unwrap()).unwrap()
}

fn is_valid(schema: &Value, instance: &Value) -> bool {
    jsonschema::options()
        .should_validate_formats(true)
        .build(schema)
        .unwrap()
        .is_valid(instance)
}

fn ready_input() -> Value {
    serde_json::from_slice(
        &fs::read(workspace_path("tests/fixtures/pre-merge/ready.json")).unwrap(),
    )
    .unwrap()
}

fn advisory_decision() -> Value {
    serde_json::from_slice(
        &fs::read(workspace_path(
            "tests/fixtures/contracts/advisory-decision.json",
        ))
        .unwrap(),
    )
    .unwrap()
}

fn post_merge_decision() -> Value {
    json!({
        "schema_version": 1,
        "status": "shipped",
        "binding": {
            "repository": "FlitCatz/policy-kernel",
            "pull_request_number": 8,
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "evidence_digest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "evaluated_at": "2026-09-08T00:00:00Z"
        },
        "merge_commit_sha": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "reasons": [],
        "decision_digest": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    })
}

#[test]
fn schema_regeneration_is_deterministic_and_matches_checked_in_files() {
    let before: Vec<Option<Vec<u8>>> = SCHEMAS
        .iter()
        .map(|name| fs::read(schema_path(name)).ok())
        .collect();

    let first = Command::new(cargo_bin!("xtask"))
        .arg("schema")
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let generated: Vec<Option<Vec<u8>>> = SCHEMAS
        .iter()
        .map(|name| fs::read(schema_path(name)).ok())
        .collect();
    assert_eq!(
        before, generated,
        "schema generation changed checked-in files"
    );

    let second = Command::new(cargo_bin!("xtask"))
        .arg("schema")
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let regenerated: Vec<Option<Vec<u8>>> = SCHEMAS
        .iter()
        .map(|name| fs::read(schema_path(name)).ok())
        .collect();
    assert_eq!(generated, regenerated);
    assert!(generated.iter().all(Option::is_some));
}

#[test]
fn schemas_accept_matching_contract_fixtures_and_reject_unknown_fields_and_versions() {
    let ready = ready_input();
    let fixtures = [
        ("policy.schema.json", ready["policy"].clone()),
        ("evidence-bundle.schema.json", ready["evidence"].clone()),
        (
            "sensor-profile.schema.json",
            ready["context"]["sensor_profile"].clone(),
        ),
        ("pre-merge-decision.schema.json", advisory_decision()),
        ("post-merge-decision.schema.json", post_merge_decision()),
    ];

    for (name, valid) in fixtures {
        let schema = read_schema(name);
        assert!(
            is_valid(&schema, &valid),
            "{name} rejected its valid fixture"
        );

        let mut unknown = valid.clone();
        unknown["unexpected"] = json!(true);
        assert!(
            !is_valid(&schema, &unknown),
            "{name} accepted an unknown root field"
        );

        let mut future = valid;
        future["schema_version"] = json!(99);
        assert!(
            !is_valid(&schema, &future),
            "{name} accepted an unsupported schema version"
        );
    }
}

#[test]
fn decision_schema_matches_identity_and_timestamp_deserializers() {
    let schema = read_schema("pre-merge-decision.schema.json");
    let valid = advisory_decision();
    for (pointer, replacement) in [
        ("/binding/repository", json!("missing-slash")),
        ("/binding/pull_request_number", json!(0)),
        ("/binding/head_sha", json!("deadbeef")),
        ("/binding/policy_digest", json!("not-a-digest")),
        ("/binding/evaluated_at", json!("not-a-timestamp")),
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = replacement;
        assert!(
            !is_valid(&schema, &invalid),
            "schema accepted invalid value at {pointer}"
        );
    }
}

fn assert_standard_integer_bounds(value: &Value, seen: &mut (usize, usize)) {
    match value {
        Value::Object(object) => {
            if let Some(format) = object.get("format").and_then(Value::as_str) {
                match format {
                    "uint64" => {
                        seen.0 += 1;
                        assert!(object.get("minimum").and_then(Value::as_u64).is_some());
                        assert_eq!(object.get("maximum"), Some(&json!(u64::MAX)));
                    }
                    "int32" => {
                        seen.1 += 1;
                        assert_eq!(object.get("minimum"), Some(&json!(i32::MIN)));
                        assert_eq!(object.get("maximum"), Some(&json!(i32::MAX)));
                    }
                    _ => {}
                }
            }
            for child in object.values() {
                assert_standard_integer_bounds(child, seen);
            }
        }
        Value::Array(items) => {
            for child in items {
                assert_standard_integer_bounds(child, seen);
            }
        }
        _ => {}
    }
}

#[test]
fn every_emitted_rust_integer_format_has_standard_json_schema_bounds() {
    let mut seen = (0, 0);
    for name in SCHEMAS {
        assert_standard_integer_bounds(&read_schema(name), &mut seen);
    }
    assert!(seen.0 > 0, "expected emitted uint64 schemas");
    assert!(seen.1 > 0, "expected emitted int32 schemas");
}

#[test]
fn schemas_accept_integer_extrema_and_reject_adjacent_out_of_range_values() {
    let overflow: Value = serde_json::from_str("18446744073709551616").unwrap();

    let policy_schema = read_schema("policy.schema.json");
    let mut policy = ready_input()["policy"].clone();
    for valid in [json!(0), json!(u64::MAX)] {
        policy["max_evidence_age_seconds"] = valid;
        assert!(is_valid(&policy_schema, &policy));
    }
    for invalid in [json!(-1), overflow.clone()] {
        policy["max_evidence_age_seconds"] = invalid;
        assert!(!is_valid(&policy_schema, &policy));
    }

    let decision_schema = read_schema("pre-merge-decision.schema.json");
    let mut decision = advisory_decision();
    for valid in [json!(1), json!(u64::MAX)] {
        decision["binding"]["pull_request_number"] = valid;
        assert!(is_valid(&decision_schema, &decision));
    }
    for invalid in [json!(0), overflow] {
        decision["binding"]["pull_request_number"] = invalid;
        assert!(
            !is_valid(&decision_schema, &decision),
            "accepted out-of-range pull request number {}",
            decision["binding"]["pull_request_number"]
        );
    }

    let profile_schema = read_schema("sensor-profile.schema.json");
    let mut profile = ready_input()["context"]["sensor_profile"].clone();
    for valid in [json!(i32::MIN), json!(i32::MAX)] {
        profile["sensors"][0]["accepted_exit_codes"][0] = valid;
        assert!(is_valid(&profile_schema, &profile));
    }
    for invalid in [
        json!(i64::from(i32::MIN) - 1),
        json!(i64::from(i32::MAX) + 1),
    ] {
        profile["sensors"][0]["accepted_exit_codes"][0] = invalid;
        assert!(!is_valid(&profile_schema, &profile));
    }
}
