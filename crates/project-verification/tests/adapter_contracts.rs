use project_verification::{adapter_contracts::AdapterContract, parse_profile};
use serde_json::{json, Value};

fn project() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    value["schema_version"] = json!(4);
    value["selection"]["schema_version"] = json!(3);
    value["selection"]["runtime_files"] = json!([]);
    for tool in value["selection"]["tools"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        let normalization = tool["version"]["normalization"].clone();
        tool["version"] = json!({"normalization":normalization,"acceptance":{"kind":"record"}});
    }
    value["selection"]["packages"][0]
        .as_object_mut()
        .unwrap()
        .remove("requirement");
    value["selection"]["packages"][0]["acceptance"] = json!({"kind":"record"});
    value["selection"]["contracts"] = json!({"node":["process/v1"],"npm":["process/v1","npm-ci-audit/v1"],"jest":["jest-json/v1"]});
    value
}

#[test]
fn project_accepts_synthetic_producer_without_weakening_legacy_range() {
    let profile = parse_profile(&project().to_string()).unwrap();
    let tool = &profile.selection.unwrap().tools["jest"];
    assert_eq!(tool.observe(b"99.0.0\n").unwrap(), "99.0.0");
    let legacy = parse_profile(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    assert!(legacy.selection.unwrap().tools["jest"]
        .observe(b"99.0.0\n")
        .is_err());
}

#[test]
fn project_rejects_missing_unknown_or_mismatched_contracts() {
    let original = project();
    for contracts in [json!([]), json!(["future-json/v9"]), json!(["process/v1"])] {
        let mut value = original.clone();
        value["selection"]["contracts"]["jest"] = contracts;
        assert!(parse_profile(&value.to_string()).is_err());
    }
    let mut value = original.clone();
    value["selection"]["contracts"]["npm"] = json!(["process/v1"]);
    assert!(parse_profile(&value.to_string()).is_err());
    let mut value = original.clone();
    value["selection"]["tools"]["jest"]["producer_contract"] = json!("playwright163");
    assert!(parse_profile(&value.to_string()).is_err());
}

#[test]
fn adapter_ids_are_stable_wire_values() {
    assert_eq!(
        serde_json::to_string(&AdapterContract::NpmCiAuditV1).unwrap(),
        "\"npm-ci-audit/v1\""
    );
    assert!(serde_json::from_str::<AdapterContract>("\"future-json/v9\"").is_err());
}
