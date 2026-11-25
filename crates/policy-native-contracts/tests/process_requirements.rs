use policy_native_contracts::ExpectedScopeV1;
use serde_json::{json, Value};

fn process_scope() -> Value {
    let corpus: Value = serde_json::from_str(include_str!("corpus/v1.json")).unwrap();
    let mut scope = corpus["valid"]["expected"].clone();
    scope["requirements"]["observations"] = json!([{
        "id": "format", "accepted_exit_codes": [0], "tests": null
    }]);
    scope["bindings"]["observations"][0]["id"] = json!("format");
    scope["bindings"]["observations"][0]["parser"] = Value::Null;
    scope["requirements"]["requirements"] = json!([{
        "id": "formatting", "process_checks": ["format"]
    }]);
    scope
}

#[test]
fn process_requirement_round_trips_without_a_suite() {
    let scope = process_scope();
    let parsed: ExpectedScopeV1 = serde_json::from_value(scope.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), scope);

    let schema =
        serde_json::from_str(include_str!("../schemas/expected-scope-v1.schema.json")).unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&scope));
}

#[test]
fn process_requirement_rejects_missing_empty_duplicate_and_suite_references() {
    for checks in [
        json!(["unapproved"]),
        json!([]),
        json!(["format", "format"]),
        Value::Null,
    ] {
        let mut scope = process_scope();
        scope["requirements"]["requirements"][0]["process_checks"] = checks;
        assert!(serde_json::from_value::<ExpectedScopeV1>(scope).is_err());
    }
    let mut scope = process_scope();
    scope["requirements"]["requirements"][0]["minimum_executed_tests"] = json!(0);
    assert!(serde_json::from_value::<ExpectedScopeV1>(scope).is_err());

    let corpus: Value = serde_json::from_str(include_str!("corpus/v1.json")).unwrap();
    let mut scope = corpus["valid"]["expected"].clone();
    scope["requirements"]["requirements"][0] = json!({"id":"formatting","process_checks":["unit"]});
    assert!(serde_json::from_value::<ExpectedScopeV1>(scope).is_err());
}

#[test]
fn mixed_scope_keeps_both_requirement_shapes_and_requires_unique_ids() {
    let corpus: Value = serde_json::from_str(include_str!("corpus/v1.json")).unwrap();
    let mut scope = corpus["valid"]["expected"].clone();
    let process = process_scope();
    for (parent, field) in [
        ("requirements", "observations"),
        ("requirements", "requirements"),
        ("bindings", "observations"),
    ] {
        scope[parent][field]
            .as_array_mut()
            .unwrap()
            .push(process[parent][field][0].clone());
    }
    let parsed: ExpectedScopeV1 = serde_json::from_value(scope.clone()).unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), scope);
    let mut no_suite_requirement = scope.clone();
    no_suite_requirement["requirements"]["requirements"] =
        process["requirements"]["requirements"].clone();
    assert!(serde_json::from_value::<ExpectedScopeV1>(no_suite_requirement).is_err());

    scope["requirements"]["requirements"][3]["id"] = json!("AC-1");
    assert!(serde_json::from_value::<ExpectedScopeV1>(scope).is_err());
}
