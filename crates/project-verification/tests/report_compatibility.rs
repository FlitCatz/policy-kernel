use project_verification::reports::{
    parse_report, ParsedReport, ReportFormat, TestOutcome, TestResult,
};

#[test]
fn legacy_module_reexports_identical_types_and_corpus_outputs() {
    for (format, name) in [
        (ReportFormat::VitestJson, "vitest.json"),
        (ReportFormat::PlaywrightJson, "playwright.json"),
        (ReportFormat::Trx, "vstest.trx"),
        (ReportFormat::JestJson, "jest.json"),
    ] {
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../policy-report-adapters/tests/fixtures/project-reports");
        let input = std::fs::read(corpus.join(name)).unwrap();
        let expected = std::fs::read(corpus.join(format!("{name}.expected.json"))).unwrap();
        let report: ParsedReport =
            policy_report_adapters::parse_report(format, "auth", &input).unwrap();
        let legacy: policy_report_adapters::ParsedReport =
            parse_report(format, "auth", &input).unwrap();
        assert_eq!(serde_json::to_vec(&report).unwrap(), expected);
        assert_eq!(serde_json::to_vec(&legacy).unwrap(), expected);
        let test: &TestResult = &report.tests[0];
        let outcome: policy_report_adapters::TestOutcome = test.outcome;
        assert_eq!(outcome, TestOutcome::Passed);
        let error: project_verification::reports::ReportError =
            policy_report_adapters::ReportError("compatibility".into());
        assert_eq!(error.to_string(), "invalid report: compatibility");
    }
}

#[test]
fn legacy_jest_profile_keeps_wire_contracts_and_schema_four_migration() {
    use project_verification::{
        adapter_contracts::AdapterContract,
        parse_profile,
        selection::{upgrade_frontend_profile, Capability},
    };
    let raw = include_str!("fixtures/legacy-jest-profile.json");
    let legacy = parse_profile(raw).unwrap();
    let selection = legacy.selection.as_ref().unwrap();
    assert_eq!(
        selection.tools["jest"].capabilities,
        std::collections::BTreeSet::from([Capability::JestJson])
    );
    let mut widened_jest = serde_json::to_value(&selection.tools["jest"]).unwrap();
    widened_jest["version"]["requirement"] = serde_json::json!(">=30.2.0, <200.0.0");
    let widened_jest: project_verification::selection::ToolSelection =
        serde_json::from_value(widened_jest).unwrap();
    assert_eq!(widened_jest.observe(b"30.5.2\n").unwrap(), "30.5.2");
    let error = widened_jest.observe(b"31.0.0\n").unwrap_err();
    assert!(error
        .to_string()
        .contains("unsupported producer capability JestJson"));
    assert_eq!(
        legacy.sensors[2].report.as_ref().unwrap().format,
        ReportFormat::JestJson
    );
    let upgraded = upgrade_frontend_profile(legacy).unwrap();
    assert_eq!(
        upgraded.selection.as_ref().unwrap().contracts["jest"],
        std::collections::BTreeSet::from([AdapterContract::JestJsonV1])
    );
    let encoded = serde_json::to_string(&upgraded).unwrap();
    assert!(encoded.contains("jest-json/v1"));
    assert!(encoded.contains("jest_json"));
    parse_profile(&encoded).unwrap();
    let mut schema_one: serde_json::Value = serde_json::from_str(raw).unwrap();
    schema_one["schema_version"] = serde_json::json!(1);
    schema_one["version"] = serde_json::json!("1.0.2");
    schema_one.as_object_mut().unwrap().remove("selection");
    for sensor in schema_one["sensors"].as_array_mut().unwrap() {
        sensor["version_stdout"] = serde_json::json!("legacy producer");
    }
    schema_one["sensors"][2]["report"]["format"] = serde_json::json!("vitest_json");
    let mut schema_one = parse_profile(&schema_one.to_string()).unwrap();
    schema_one.sensors[2].report.as_mut().unwrap().format = ReportFormat::JestJson;
    let error = schema_one.validate().unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Jest report contract requires profile schema2"),
        "{error}"
    );
    assert!(parse_profile(&serde_json::to_string(&schema_one).unwrap()).is_err());
}
