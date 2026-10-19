use policy_contracts::*;

#[test]
fn knowledge_policy_v2_is_explicit_and_legacy_policies_round_trip() {
    let ready: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/pre-merge/legacy-ready.json"
    ))
    .unwrap();
    let legacy = ready["policy"].clone();
    let parsed: PolicySnapshot = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), legacy);
    let mut v2 = legacy.clone();
    v2["schema_version"] = serde_json::json!(2);
    v2["required_knowledge"] = serde_json::json!([{
        "kind": "code_graph_index", "source": "ci/codegraph", "required_when": []
    }]);
    assert!(serde_json::from_value::<PolicySnapshot>(v2.clone()).is_ok());
    for version in [0, 1, 3, 99] {
        v2["schema_version"] = serde_json::json!(version);
        assert!(serde_json::from_value::<PolicySnapshot>(v2.clone()).is_err());
    }
    for declaration in [serde_json::json!([]), serde_json::Value::Null] {
        let mut invalid = legacy.clone();
        invalid["required_knowledge"] = declaration;
        assert!(serde_json::from_value::<PolicySnapshot>(invalid).is_err());
    }
}

#[test]
fn unknown_schema_version_is_rejected() {
    let error = serde_json::from_str::<PreMergeEvaluationInput>(include_str!(
        "../../../tests/fixtures/contracts/unknown-version.json"
    ))
    .unwrap_err();
    assert!(error.to_string().contains("unsupported schema version"));
}

#[test]
fn short_and_non_hex_commit_shas_are_rejected() {
    assert!(CommitSha::parse("deadbeef").is_err());
    assert!(CommitSha::parse("z".repeat(40)).is_err());
    assert!(
        serde_json::from_str::<PreMergeEvaluationInput>(include_str!(
            "../../../tests/fixtures/contracts/invalid-sha.json"
        ))
        .is_err()
    );
}

#[test]
fn advisory_decision_cannot_be_post_merge_authorization() {
    let decision: PreMergeDecision = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/advisory-decision.json"
    ))
    .unwrap();
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_err());
}

#[test]
fn advisory_decision_cannot_deserialize_as_post_merge_authorization() {
    assert!(
        serde_json::from_str::<AuthoritativePreMergeDecision>(include_str!(
            "../../../tests/fixtures/contracts/advisory-decision.json"
        ))
        .is_err()
    );
}

#[test]
fn authoritative_ready_decision_can_be_post_merge_authorization() {
    let mut decision: PreMergeDecision = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/advisory-decision.json"
    ))
    .unwrap();
    decision.mode = EvaluationMode::Authoritative;
    decision.advisory = false;
    decision.status = PreMergeStatus::ReadyToMerge;
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_ok());
}

#[test]
fn unknown_wire_fields_are_rejected() {
    let json = include_str!("../../../tests/fixtures/contracts/advisory-decision.json")
        .replace("\"advisory\":true", "\"advisory\":true,\"unexpected\":1");
    assert!(serde_json::from_str::<PreMergeDecision>(&json).is_err());
}

#[test]
fn valid_pre_merge_fixture_round_trips_byte_for_byte() {
    let json = include_str!("../../../tests/fixtures/contracts/valid-pre-merge.json").trim();
    let input: PreMergeEvaluationInput = serde_json::from_str(json).unwrap();
    assert_eq!(serde_json::to_string(&input).unwrap(), json);
}

#[test]
fn pull_request_numbers_must_be_positive() {
    assert!(PullRequestNumber::new(0).is_err());
    assert_eq!(PullRequestNumber::new(42).unwrap().get(), 42);
}

#[test]
fn sensor_profile_versions_preserve_legacy_bytes_and_require_explicit_v2_results() {
    let legacy = r#"{"schema_version":1,"sensors":[{"id":"unit","required":true,"required_when":[],"not_applicable_when":[],"accepted_exit_codes":[0]}],"adequacy":{"acceptance_criterion_ids":[],"changed_behavior_ids":[],"negative_case_ids":[]}}"#;
    let profile: SensorProfile = serde_json::from_str(legacy).unwrap();
    assert_eq!(serde_json::to_string(&profile).unwrap(), legacy);
    let base: serde_json::Value = serde_json::from_str(legacy).unwrap();
    for requirement in [
        serde_json::json!("process"),
        serde_json::json!("test_counts"),
        serde_json::Value::Null,
    ] {
        let mut value = base.clone();
        value["sensors"][0]["result_requirement"] = requirement;
        assert!(serde_json::from_value::<SensorProfile>(value).is_err());
    }
    for version in [0, 2, 99] {
        let mut value = base.clone();
        value["schema_version"] = serde_json::json!(version);
        assert!(serde_json::from_value::<SensorProfile>(value).is_err());
    }
    for requirement in ["process", "test_counts"] {
        let mut value = base.clone();
        value["schema_version"] = serde_json::json!(2);
        value["sensors"][0]["result_requirement"] = serde_json::json!(requirement);
        let parsed: SensorProfile = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    }
    for requirement in [serde_json::Value::Null, serde_json::json!("unknown")] {
        let mut value = base.clone();
        value["schema_version"] = serde_json::json!(2);
        value["sensors"][0]["result_requirement"] = requirement;
        assert!(serde_json::from_value::<SensorProfile>(value).is_err());
    }
}

#[test]
fn result_bindings_require_v2_and_reject_explicit_null() {
    let legacy: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/sensors/profile.json")).unwrap();
    let binding = serde_json::json!({
        "configuration_digest": "b".repeat(64),
        "parser_version": "1",
        "resolved_command": ["sensor"]
    });
    for value in [binding.clone(), serde_json::Value::Null] {
        let mut profile = legacy.clone();
        profile["sensors"][0]["result_binding"] = value;
        assert!(serde_json::from_value::<SensorProfile>(profile).is_err());
    }
    let mut profile = legacy;
    profile["schema_version"] = serde_json::json!(2);
    for definition in profile["sensors"].as_array_mut().unwrap() {
        definition["result_requirement"] = serde_json::json!("process");
    }
    profile["sensors"][0]["result_binding"] = binding;
    assert!(serde_json::from_value::<SensorProfile>(profile.clone()).is_ok());
    profile["sensors"][0]["result_binding"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<SensorProfile>(profile).is_err());
}
