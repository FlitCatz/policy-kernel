use policy_contracts::PolicySnapshot;
use serde_json::{json, Value};

fn policy(version: u32) -> Value {
    let ready: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/pre-merge/ready.json")).unwrap();
    let mut policy = ready["policy"].clone();
    policy["schema_version"] = json!(version);
    if version < 3 {
        policy.as_object_mut().unwrap().remove("required_reviews");
    }
    policy
}

#[test]
fn owner_review_option_is_v4_only_nonnull_and_omitted_when_absent() {
    for version in 1..=4 {
        let legacy = policy(version);
        let parsed: PolicySnapshot = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), legacy);
        for block in [
            json!({"owner_actor":"repository-owner", "maximum_age_seconds":604800}),
            Value::Null,
        ] {
            let mut value = legacy.clone();
            value["owner_with_independent_agent_review"] = block.clone();
            let parsed = serde_json::from_value::<PolicySnapshot>(value);
            assert_eq!(parsed.is_ok(), version == 4 && !block.is_null());
        }
    }
}

#[test]
fn invalid_owner_policy_shapes_and_direct_construction_fail_closed() {
    let mut valid = policy(4);
    valid["owner_with_independent_agent_review"] =
        json!({"owner_actor":"repository-owner", "maximum_age_seconds":604800});
    for (key, replacement) in [
        ("owner_actor", json!("")),
        ("owner_actor", json!(" \t\n")),
        ("owner_actor", json!("owner\0suffix")),
        ("owner_actor", Value::Null),
        ("maximum_age_seconds", json!(0)),
        ("maximum_age_seconds", json!(-1)),
        ("maximum_age_seconds", Value::Null),
        ("unexpected", json!(true)),
    ] {
        let mut invalid = valid.clone();
        invalid["owner_with_independent_agent_review"][key] = replacement;
        assert!(serde_json::from_value::<PolicySnapshot>(invalid).is_err());
    }
    for key in ["owner_actor", "maximum_age_seconds"] {
        let mut invalid = valid.clone();
        invalid["owner_with_independent_agent_review"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert!(serde_json::from_value::<PolicySnapshot>(invalid).is_err());
    }
    let parsed: PolicySnapshot = serde_json::from_value(valid).unwrap();
    for version in [1, 2, 3, 5, 99] {
        let mut invalid = parsed.clone();
        invalid.schema_version = version;
        assert!(invalid.validate_version().is_err());
    }
    let mut invalid = parsed.clone();
    invalid
        .owner_with_independent_agent_review
        .as_mut()
        .unwrap()
        .maximum_age_seconds = 0;
    assert!(invalid.validate_version().is_err());
    let mut invalid = parsed;
    invalid
        .owner_with_independent_agent_review
        .as_mut()
        .unwrap()
        .owner_actor = "\0".into();
    assert!(invalid.validate_version().is_err());
}
