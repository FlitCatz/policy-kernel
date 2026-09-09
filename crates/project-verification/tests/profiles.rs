use policy_contracts::Risk;
use project_verification::{parse_profile, profile::Impact};
use serde_json::{json, Value};

const FULL: &str = include_str!("../../../profiles/fullstack/profile.json");

#[test]
fn shipped_profile_instances_match_schema_and_have_unique_fullstack_suites() {
    for raw in [
        include_str!("../../../profiles/react-typescript/profile.json"),
        include_str!("../../../profiles/dotnet/profile.json"),
        FULL,
    ] {
        let profile = parse_profile(raw).unwrap();
        let all = profile.selected(&Impact::Unknown).unwrap();
        assert_eq!(all.len(), profile.sensors.len());
        assert!(all
            .iter()
            .all(|s| s.definition.result_requirement.is_some()));
    }
    let profile = parse_profile(FULL).unwrap();
    for id in [
        "frontend:unit",
        "backend:unit",
        "backend:integration",
        "shared:contract",
        "shared:e2e",
    ] {
        assert!(profile.selected(&Impact::Unknown).unwrap().iter().any(|s| s
            .definition
            .id
            .as_str()
            == id));
    }
}

#[test]
fn invalid_composition_and_report_contracts_fail_before_execution() {
    for pointer in ["/components/0/root", "/sensors/0/inputs/0"] {
        let mut value: Value = serde_json::from_str(FULL).unwrap();
        *value.pointer_mut(pointer).unwrap() = json!("../outside");
        assert!(parse_profile(&value.to_string()).is_err());
    }
    let mut missing: Value = serde_json::from_str(FULL).unwrap();
    missing["components"].as_array_mut().unwrap().remove(1);
    assert!(parse_profile(&missing.to_string()).is_err());
    let mut duplicate: Value = serde_json::from_str(FULL).unwrap();
    let first = duplicate["sensors"][0].clone();
    duplicate["sensors"].as_array_mut().unwrap().push(first);
    assert!(parse_profile(&duplicate.to_string()).is_err());
    let mut downgraded: Value = serde_json::from_str(FULL).unwrap();
    let unit = downgraded["sensors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["definition"]["id"] == "frontend:unit")
        .unwrap();
    unit["definition"]["result_requirement"] = json!("process");
    assert!(parse_profile(&downgraded.to_string()).is_err());
    assert!(serde_json::from_value::<Impact>(json!({"kind":"unknown","paths":[]})).is_err());
}

#[test]
fn unknown_impact_is_not_an_empty_known_change() {
    let mut p = parse_profile(FULL).unwrap();
    let sensor = p
        .sensors
        .iter_mut()
        .find(|s| s.definition.id.as_str() == "frontend:lint")
        .unwrap();
    sensor.definition.required = false;
    sensor.definition.required_when = serde_json::from_value(json!([{
        "id":"source","risks":[],"path_globs":["frontend/src/**"]}]))
    .unwrap();
    let known = Impact::Known {
        paths: vec![],
        risk: Risk::Low,
    };
    assert!(!p
        .selected(&known)
        .unwrap()
        .iter()
        .any(|s| s.definition.id.as_str() == "frontend:lint"));
    assert!(p
        .selected(&Impact::Unknown)
        .unwrap()
        .iter()
        .any(|s| s.definition.id.as_str() == "frontend:lint"));
    assert!(p
        .selected(&Impact::Known {
            paths: vec!["../outside".into()],
            risk: Risk::High
        })
        .is_err());
}
