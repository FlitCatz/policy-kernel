use policy_contracts::{PolicySnapshot, TrustClass};
use serde_json::{json, Value};

fn policy(version: u32) -> Value {
    let ready: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/pre-merge/ready.json")).unwrap();
    let mut policy = ready["policy"].clone();
    policy["schema_version"] = json!(version);
    if version < 3 {
        policy.as_object_mut().unwrap().remove("required_reviews");
    }
    policy
}

fn local_policy() -> Value {
    json!({
        "allowed_runner_ids": ["owner-runner"],
        "approved_scope_template_digests": ["a".repeat(64)],
        "maximum_age_seconds": 86400,
        "ci_required_when": []
    })
}

fn check_policy(value: Value, expected: bool) {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/policy.schema.json")).unwrap();
    let validator = jsonschema::draft202012::options().build(&schema).unwrap();
    assert_eq!(validator.is_valid(&value), expected, "schema: {value}");
    assert_eq!(
        serde_json::from_value::<PolicySnapshot>(value.clone()).is_ok(),
        expected,
        "Rust: {value}"
    );
    if expected {
        let parsed: PolicySnapshot = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
}

#[test]
fn v4_local_runner_block_is_optional_and_legacy_versions_reject_its_presence() {
    for version in 1..=4 {
        let value = policy(version);
        check_policy(value.clone(), true);
        let parsed: PolicySnapshot = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        for block in [local_policy(), Value::Null] {
            let mut value = value.clone();
            value["local_runner_evidence"] = block.clone();
            check_policy(value, version == 4 && !block.is_null());
        }
    }
    for reviews in [None, Some(Value::Null)] {
        let mut value = policy(4);
        value.as_object_mut().unwrap().remove("required_reviews");
        if let Some(reviews) = reviews {
            value["required_reviews"] = reviews;
        }
        check_policy(value, false);
    }
}

#[test]
fn local_runner_policy_shapes_match_rust_and_json_schema() {
    let mut valid = policy(4);
    valid["local_runner_evidence"] = local_policy();
    check_policy(valid.clone(), true);
    for field in [
        "allowed_runner_ids",
        "approved_scope_template_digests",
        "maximum_age_seconds",
        "ci_required_when",
    ] {
        let mut missing = valid.clone();
        missing["local_runner_evidence"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        check_policy(missing, false);
        let mut null = valid.clone();
        null["local_runner_evidence"][field] = Value::Null;
        check_policy(null, false);
    }
    for (field, replacement) in [
        ("allowed_runner_ids", json!([])),
        ("allowed_runner_ids", json!(["runner", "runner"])),
        ("allowed_runner_ids", json!([""])),
        ("allowed_runner_ids", json!([" \t"])),
        ("allowed_runner_ids", json!(["\u{2003}"])),
        ("allowed_runner_ids", json!(["runner\0"])),
        ("allowed_runner_ids", json!([1])),
        ("approved_scope_template_digests", json!([])),
        ("approved_scope_template_digests", json!(["abc"])),
        ("approved_scope_template_digests", json!(["z".repeat(64)])),
        ("maximum_age_seconds", json!(0)),
        ("maximum_age_seconds", json!(-1)),
        ("maximum_age_seconds", json!(1.5)),
        ("maximum_age_seconds", json!("86400")),
        ("ci_required_when", json!({})),
        ("ci_required_when", json!([{}])),
        (
            "ci_required_when",
            json!([{"id":"security", "risks":["unknown"], "path_globs":[]}]),
        ),
    ] {
        let mut value = valid.clone();
        value["local_runner_evidence"][field] = replacement;
        check_policy(value, false);
    }
    let mut unknown = valid.clone();
    unknown["local_runner_evidence"]["unexpected"] = json!(true);
    check_policy(unknown, false);
    valid["local_runner_evidence"]["maximum_age_seconds"] = json!(u64::MAX);
    valid["local_runner_evidence"]["allowed_runner_ids"] = json!(["runner-a", "runner-b"]);
    valid["local_runner_evidence"]["ci_required_when"] = json!([{
        "id": "security", "risks": ["high"], "path_globs": [".github/**"]
    }]);
    check_policy(valid, true);
}

#[test]
fn local_runner_trust_round_trips_in_v1_evidence() {
    let wire = json!("local_runner_attested");
    let trust: TrustClass = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(trust).unwrap(), wire);
    let ready: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/pre-merge/ready.json")).unwrap();
    let mut evidence = ready["evidence"].clone();
    evidence["sensors"][0]["trust"] = wire;
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/evidence-bundle.schema.json")).unwrap();
    let validator = jsonschema::draft202012::options().build(&schema).unwrap();
    assert!(validator.is_valid(&evidence));
    let parsed: policy_contracts::EvidenceBundle =
        serde_json::from_value(evidence.clone()).unwrap();
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(serde_json::to_value(parsed).unwrap(), evidence);
}

#[test]
fn direct_construction_obeys_local_runner_and_review_boundaries() {
    let local: policy_contracts::LocalRunnerEvidencePolicy =
        serde_json::from_value(local_policy()).unwrap();
    assert!(local.validate().is_ok());
    for version in 1..=4 {
        let mut value: PolicySnapshot = serde_json::from_value(policy(version)).unwrap();
        value.local_runner_evidence = Some(local.clone());
        assert_eq!(value.validate_version().is_ok(), version == 4);
    }
    for invalid in [
        policy_contracts::LocalRunnerEvidencePolicy {
            allowed_runner_ids: vec![],
            ..local.clone()
        },
        policy_contracts::LocalRunnerEvidencePolicy {
            allowed_runner_ids: vec!["runner".into(), "runner".into()],
            ..local.clone()
        },
        policy_contracts::LocalRunnerEvidencePolicy {
            allowed_runner_ids: vec![" ".into()],
            ..local.clone()
        },
        policy_contracts::LocalRunnerEvidencePolicy {
            allowed_runner_ids: vec!["runner\0".into()],
            ..local.clone()
        },
        policy_contracts::LocalRunnerEvidencePolicy {
            approved_scope_template_digests: vec![],
            ..local.clone()
        },
        policy_contracts::LocalRunnerEvidencePolicy {
            maximum_age_seconds: 0,
            ..local.clone()
        },
    ] {
        assert!(invalid.validate().is_err());
        let mut value: PolicySnapshot = serde_json::from_value(policy(4)).unwrap();
        value.local_runner_evidence = Some(invalid);
        assert!(value.validate_version().is_err());
    }
    for (field, invalid) in [
        ("minimum_reviewers", json!(0)),
        ("require_provider_independence", json!(false)),
    ] {
        let mut value = policy(4);
        value["required_reviews"][field] = invalid;
        check_policy(value, false);
    }
    let mut value: PolicySnapshot = serde_json::from_value(policy(4)).unwrap();
    value.required_reviews.as_mut().unwrap().minimum_reviewers = 0;
    assert!(value.validate_version().is_err());
    value.required_reviews.as_mut().unwrap().minimum_reviewers = 1;
    value
        .required_reviews
        .as_mut()
        .unwrap()
        .require_provider_independence = false;
    assert!(value.validate_version().is_err());
}
