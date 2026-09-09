use project_verification::reports::{ReportFormat, TestOutcome, parse_report};
use serde_json::{Value, json};

const VITEST: &[u8] = include_bytes!("../../../tests/fixtures/project-reports/vitest.json");
const PLAYWRIGHT: &[u8] = include_bytes!("../../../tests/fixtures/project-reports/playwright.json");
const TRX: &str = include_str!("../../../tests/fixtures/project-reports/vstest.trx");
fn parse_json(
    format: ReportFormat,
    value: &Value,
) -> Result<project_verification::reports::ParsedReport, project_verification::reports::ReportError>
{
    parse_report(format, "auth", &serde_json::to_vec(value).unwrap())
}

#[test]
fn pinned_minimal_reports_have_stable_namespaced_ids() {
    for (format, bytes) in [
        (ReportFormat::VitestJson, VITEST),
        (ReportFormat::PlaywrightJson, PLAYWRIGHT),
        (ReportFormat::Trx, TRX.as_bytes()),
    ] {
        let first = parse_report(format, "auth", bytes).unwrap();
        assert!(first.successful);
        assert_eq!((first.discovered, first.executed, first.passed), (1, 1, 1));
        assert_eq!(
            first.tests[0].id,
            parse_report(format, "auth", bytes).unwrap().tests[0].id
        );
        assert_ne!(
            first.tests[0].id,
            parse_report(format, "payments", bytes).unwrap().tests[0].id
        );
        assert!(parse_report(format, "auth", b"garbage").is_err());
    }
}
#[test]
fn vitest_requires_summary_timestamp_and_unique_tests() {
    let original: Value = serde_json::from_slice(VITEST).unwrap();
    for key in [
        "startTime",
        "numTotalTests",
        "numPendingTests",
        "numFailedTestSuites",
        "success",
    ] {
        let mut value = original.clone();
        value.as_object_mut().unwrap().remove(key);
        assert!(
            parse_json(ReportFormat::VitestJson, &value).is_err(),
            "{key}"
        );
    }
    let mut value = original.clone();
    value["numTotalTests"] = json!(2);
    assert!(parse_json(ReportFormat::VitestJson, &value).is_err());
    let mut value = original.clone();
    let test = value["testResults"][0]["assertionResults"][0].clone();
    value["testResults"][0]["assertionResults"]
        .as_array_mut()
        .unwrap()
        .push(test);
    value["numTotalTests"] = json!(2);
    value["numPassedTests"] = json!(2);
    assert!(parse_json(ReportFormat::VitestJson, &value).is_err());
}
#[test]
fn vitest_failure_skip_and_empty_never_succeed() {
    for status in ["failed", "skipped", "todo"] {
        let mut value: Value = serde_json::from_slice(VITEST).unwrap();
        value["testResults"][0]["assertionResults"][0]["status"] = json!(status);
        value["testResults"][0]["assertionResults"][0]["failureMessages"] =
            json!(["reason retained"]);
        value["numPassedTests"] = json!(0);
        value[match status {
            "failed" => "numFailedTests",
            "todo" => "numTodoTests",
            _ => "numPendingTests",
        }] = json!(1);
        let report = parse_json(ReportFormat::VitestJson, &value).unwrap();
        assert!(!report.successful);
        assert!(report.tests[0].messages.contains(&"reason retained".into()));
    }
    let mut value: Value = serde_json::from_slice(VITEST).unwrap();
    value["testResults"] = json!([]);
    value["numTotalTests"] = json!(0);
    value["numPassedTests"] = json!(0);
    assert!(
        !parse_json(ReportFormat::VitestJson, &value)
            .unwrap()
            .successful
    );
}
#[test]
fn playwright_rejects_missing_summary_mismatch_and_duplicates() {
    let original: Value = serde_json::from_slice(PLAYWRIGHT).unwrap();
    for key in ["startTime", "expected", "skipped", "unexpected", "flaky"] {
        let mut value = original.clone();
        value["stats"].as_object_mut().unwrap().remove(key);
        assert!(parse_json(ReportFormat::PlaywrightJson, &value).is_err());
    }
    let mut value = original.clone();
    value["stats"]["expected"] = json!(2);
    assert!(parse_json(ReportFormat::PlaywrightJson, &value).is_err());
    let mut value = original.clone();
    let test = value["suites"][0]["specs"][0]["tests"][0].clone();
    value["suites"][0]["specs"][0]["tests"]
        .as_array_mut()
        .unwrap()
        .push(test);
    value["stats"]["expected"] = json!(2);
    assert!(parse_json(ReportFormat::PlaywrightJson, &value).is_err());
}
#[test]
fn playwright_retries_expected_failures_global_errors_and_skips_are_not_success() {
    let original: Value = serde_json::from_slice(PLAYWRIGHT).unwrap();
    let mut value = original.clone();
    value["errors"] = json!([{"message":"worker crashed"}]);
    let report = parse_json(ReportFormat::PlaywrightJson, &value).unwrap();
    assert!(!report.successful);
    assert!(
        report
            .messages
            .iter()
            .any(|message| message.contains("worker crashed"))
    );
    let mut value = original.clone();
    value["suites"][0]["specs"][0]["tests"][0]["results"][0]["retry"] = json!(1);
    assert!(
        !parse_json(ReportFormat::PlaywrightJson, &value)
            .unwrap()
            .successful
    );
    for (status, actual, expected) in [
        ("expected", "failed", "failed"),
        ("flaky", "passed", "passed"),
        ("unexpected", "failed", "passed"),
        ("skipped", "skipped", "skipped"),
    ] {
        let mut value = original.clone();
        let test = &mut value["suites"][0]["specs"][0]["tests"][0];
        test["status"] = json!(status);
        test["expectedStatus"] = json!(expected);
        test["results"][0]["status"] = json!(actual);
        test["annotations"] = json!([{"type":"skip","description":"reason retained"}]);
        value["stats"]["expected"] = json!(0);
        value["stats"][status] = json!(1);
        let report = parse_json(ReportFormat::PlaywrightJson, &value).unwrap();
        assert!(!report.successful);
        assert!(report.tests[0].messages.contains(&"reason retained".into()));
    }
    let mut value = original.clone();
    value["suites"] = json!([]);
    value["stats"]["expected"] = json!(0);
    assert!(
        !parse_json(ReportFormat::PlaywrightJson, &value)
            .unwrap()
            .successful
    );
}
#[test]
fn trx_rejects_dtd_namespace_missing_summary_counts_and_duplicate_results() {
    for text in [TRX.replace("total=\"1\"","total=\"2\""),TRX.replace(" start=\"2025-09-01T00:00:00Z\"",""),TRX.replace("ResultSummary","MissingSummary"),TRX.replace("http://microsoft.com/schemas/VisualStudio/TeamTest/2010","urn:other"),TRX.replace("</Results>","<UnitTestResult testId=\"login-test\" testName=\"Login.AcceptsCredentials\" outcome=\"Passed\"/></Results>"),TRX.replace("<?xml version=\"1.0\" encoding=\"utf-8\"?>","<!DOCTYPE TestRun [<!ENTITY x SYSTEM 'file:///etc/passwd'>]>")] {
        assert!(parse_report(ReportFormat::Trx,"auth",text.as_bytes()).is_err());
    }
}
#[test]
fn trx_failures_skips_and_empty_are_not_success() {
    for (outcome, executed, failed) in [("Failed", 1, 1), ("NotExecuted", 0, 0)] {
        let text = TRX.replace("outcome=\"Passed\"",&format!("outcome=\"{outcome}\""))
            .replace("executed=\"1\"",&format!("executed=\"{executed}\""))
            .replace("passed=\"1\"","passed=\"0\"").replace("failed=\"0\"",&format!("failed=\"{failed}\""))
            .replace("outcome=\"Failed\"/>","outcome=\"Failed\"><Output><ErrorInfo><Message>reason retained</Message></ErrorInfo></Output></UnitTestResult>");
        let report = parse_report(ReportFormat::Trx, "auth", text.as_bytes()).unwrap();
        assert!(!report.successful);
        if report.tests[0].outcome == TestOutcome::Failed {
            assert!(report.tests[0].messages.contains(&"reason retained".into()));
        }
    }
    let text = TRX.replace("<UnitTestResult testId=\"login-test\" testName=\"Login.AcceptsCredentials\" outcome=\"Passed\"/>","").replace("=\"1\"","=\"0\"");
    assert!(
        !parse_report(ReportFormat::Trx, "auth", text.as_bytes())
            .unwrap()
            .successful
    );
}
