use policy_contracts::Risk;
use project_verification::{parse_profile, profile::Impact};
use serde_json::{json, Value};

const FULL: &str = include_str!("../../../profiles/fullstack/profile.json");

#[test]
fn consumer_profiles_reject_empty_adequacy_without_changing_legacy_parsing() {
    let legacy = parse_profile(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    for mut profile in [
        parse_profile(include_str!(
            "../../../examples/tjerp-compatible/project-profile.json"
        ))
        .unwrap(),
        project_verification::selection::upgrade_frontend_profile(legacy).unwrap(),
    ] {
        profile.adequacy.acceptance_criterion_ids.clear();
        profile.adequacy.changed_behavior_ids.clear();
        profile.adequacy.negative_case_ids.clear();
        assert!(profile.validate().is_err());
        assert!(parse_profile(&serde_json::to_string(&profile).unwrap()).is_err());
    }
    for raw in [
        FULL,
        include_str!("../../../bootstrap/templates/1.1.0/profiles/dotnet/profile.json"),
    ] {
        let profile = parse_profile(raw).unwrap();
        let evaluated =
            policy_core::evaluate_sensor_evidence(&profile.sensor_profile(), &[], &[], None);
        assert_ne!(
            evaluated.status,
            policy_core::SensorEvaluationStatus::Passed
        );
        assert!(evaluated.has_reason("PK-ADEQ-POLICY-EMPTY"));
    }
}

#[test]
fn consumer_negative_ids_must_be_required_behaviors_and_map_to_report_tests() {
    use policy_contracts::{TestAdequacyEvidence, TestMapping};
    use project_verification::reports::{parse_report, ReportFormat};
    let mut profile = parse_profile(include_str!(
        "../../../examples/tjerp-compatible/project-profile.json"
    ))
    .unwrap();
    profile.adequacy.negative_case_ids = vec!["frontend:reject-empty".into()];
    assert!(profile.validate().is_err());
    profile.adequacy.changed_behavior_ids = profile.adequacy.negative_case_ids.clone();
    profile.validate().unwrap();
    // Normalize a reporter fixture; use the adapter's IDs, never invented hashes.
    let report = parse_report(
        ReportFormat::JestJson,
        "frontend:unit",
        include_bytes!("../../../tests/fixtures/project-reports/jest.json"),
    )
    .unwrap();
    let ids: Vec<_> = report.tests.iter().map(|t| t.id.clone()).collect();
    let mapping = |id: &str| TestMapping {
        requirement_id: id.into(),
        test_ids: ids.clone(),
    };
    let evidence = TestAdequacyEvidence {
        acceptance_criteria: vec![mapping("frontend:greeting")],
        changed_behaviors: vec![mapping("frontend:reject-empty")],
        negative_case_ids: vec!["frontend:reject-empty".into()],
        executed_test_ids: ids.clone(),
        discovered: report.discovered,
        executed: report.executed,
        passed: report.passed,
        head_sha: None,
    };
    let result =
        policy_core::evaluate_sensor_evidence(&profile.sensor_profile(), &[], &[], Some(&evidence));
    assert_eq!(
        result.status,
        policy_core::SensorEvaluationStatus::Passed,
        "{:?}",
        result.reasons
    );
}

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
