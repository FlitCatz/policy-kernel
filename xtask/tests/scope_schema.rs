use policy_contracts::{PolicySnapshot, PreMergeEvaluationInput};
use serde_json::{json, Value};

fn legacy_policy(version: u32) -> Value {
    let fixture: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/pre-merge/ready.json")).unwrap();
    let mut policy = fixture["policy"].clone();
    policy["schema_version"] = json!(version);
    if version < 3 {
        policy.as_object_mut().unwrap().remove("required_reviews");
    }
    policy
}

#[test]
fn policy_schema_and_decoder_gate_scope_rules_to_v5() {
    let schema: Value =
        serde_json::from_str(include_str!("../../schemas/policy.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let assert_contract = |value: &Value, accepted| {
        assert_eq!(validator.is_valid(value), accepted, "schema: {value}");
        assert_eq!(
            serde_json::from_value::<PolicySnapshot>(value.clone()).is_ok(),
            accepted,
            "decoder: {value}"
        );
    };

    let rules = json!({
        "default_required_sensor_ids": ["compile"],
        "rules": [
            {"when":"label", "label":"run-ci", "required_sensor_ids":["tests"]},
            {"when":"paths", "path_globs":["src/**"], "required_sensor_ids":["compile"]}
        ]
    });
    for version in 1..=4 {
        let mut policy = legacy_policy(version);
        assert_contract(&policy, true);
        policy["scope_rules"] = rules.clone();
        assert_contract(&policy, false);
        policy["scope_rules"] = Value::Null;
        assert_contract(&policy, false);
    }

    let mut v5 = legacy_policy(5);
    assert_contract(&v5, false);
    v5["scope_rules"] = rules;
    assert_contract(&v5, true);
    let decoded: PolicySnapshot = serde_json::from_value(v5.clone()).unwrap();
    assert_eq!(serde_json::to_value(&decoded).unwrap(), v5);
    let mut direct = decoded;
    direct.schema_version = 4;
    assert!(direct.validate_version().is_err());
    direct.schema_version = 5;
    direct.scope_rules = None;
    assert!(direct.validate_version().is_err());

    for (pointer, invalid) in [
        ("/scope_rules", Value::Null),
        ("/scope_rules/default_required_sensor_ids", json!([])),
        (
            "/scope_rules/default_required_sensor_ids",
            json!(["compile", "compile"]),
        ),
        ("/scope_rules/rules/0/label", json!("  ")),
        ("/scope_rules/rules/0/label", json!("run-ci\0other")),
        ("/scope_rules/rules/0/label", json!("prüfung")),
        ("/scope_rules/rules/0/label", json!("run-ci\u{0080}")),
        ("/scope_rules/rules/0/required_sensor_ids", json!([])),
        ("/scope_rules/rules/1/when", json!("event")),
        ("/scope_rules/rules/1/path_globs", json!([])),
        (
            "/scope_rules/rules/1",
            json!({"when":"paths", "required_sensor_ids":["compile"]}),
        ),
        (
            "/scope_rules/rules/1",
            json!({
                "when":"paths", "path_globs":["src/**"], "label":"run-ci", "required_sensor_ids":["compile"]
            }),
        ),
    ] {
        let mut policy = v5.clone();
        *policy.pointer_mut(pointer).unwrap() = invalid;
        assert_contract(&policy, false);
    }
}

#[test]
fn pre_merge_input_schema_and_decoder_agree_on_versioned_risk() {
    let schema = serde_json::to_value(schemars::schema_for!(PreMergeEvaluationInput)).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let ready: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/pre-merge/ready.json")).unwrap();

    let assert_contract = |input: &Value, accepted| {
        let version = &input["policy"]["schema_version"];
        let context = input.get("context");
        assert_eq!(
            validator.is_valid(input),
            accepted,
            "schema v{version}: {context:?}"
        );
        assert_eq!(
            serde_json::from_value::<PreMergeEvaluationInput>(input.clone()).is_ok(),
            accepted,
            "decoder v{version}: {context:?}"
        );
    };

    for version in 1..=5 {
        let mut input = ready.clone();
        input["policy"] = legacy_policy(version);
        if version == 5 {
            input["policy"]["scope_rules"] = json!({
                "default_required_sensor_ids": ["compile"], "rules": []
            });
            input["context"]["labels"] = json!([]);
        }
        assert_contract(&input, true);
        input["context"]["risk"] = json!("unknown");
        assert_contract(&input, false);
        input["context"]["risk"] = Value::Null;
        assert_contract(&input, version == 5);
        input["context"].as_object_mut().unwrap().remove("risk");
        assert_contract(&input, version == 5);
        input["context"] = Value::Null;
        assert_contract(&input, true);
        input.as_object_mut().unwrap().remove("context");
        assert_contract(&input, true);
    }
}
