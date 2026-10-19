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
