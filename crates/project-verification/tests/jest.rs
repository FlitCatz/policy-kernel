use project_verification::reports::{parse_report, ReportFormat};
use serde_json::{json, Value};
const REPORT: &str = include_str!("../../../tests/fixtures/project-reports/jest.json");

fn format() -> ReportFormat {
    serde_json::from_str("\"jest_json\"").unwrap()
}

#[test]
fn actual_jest_report_has_distinct_contract_and_real_tests() {
    let report = parse_report(format(), "frontend:unit", REPORT.as_bytes()).unwrap();
    assert!(report.successful);
    assert_eq!(
        (report.discovered, report.executed, report.passed),
        (2, 2, 2)
    );
    assert_eq!(format().parser_version(), "jest-json/v1");
}

#[test]
fn jest_rejects_missing_malformed_duplicate_empty_skip_retry_and_interrupted_reports() {
    for change in 0..10 {
        let mut value: Value = serde_json::from_str(REPORT).unwrap();
        match change {
            0 => {
                value.as_object_mut().unwrap().remove("startTime");
            }
            1 => value["testResults"][0]["assertionResults"][0]["invocations"] = json!(2),
            2 => value["wasInterrupted"] = json!(true),
            3 => value["numRuntimeErrorTestSuites"] = json!(1),
            4 => {
                value["testResults"][0]["assertionResults"][0]["retryReasons"] =
                    json!(["failed previously"])
            }
            5 => value["snapshot"]["unmatched"] = json!(1),
            6 => {
                let test = value["testResults"][0]["assertionResults"][0].clone();
                value["testResults"][0]["assertionResults"][1] = test;
            }
            7 => {
                value["testResults"] = json!([]);
                for k in [
                    "numTotalTests",
                    "numPassedTests",
                    "numTotalTestSuites",
                    "numPassedTestSuites",
                ] {
                    value[k] = json!(0);
                }
            }
            8 => {
                value["testResults"][0]["assertionResults"][0]["status"] = json!("pending");
                value["numPendingTests"] = json!(1);
                value["numPassedTests"] = json!(1);
            }
            _ => {
                value["testResults"][0]["assertionResults"][0]["status"] = json!("failed");
                value["numFailedTests"] = json!(1);
                value["numPassedTests"] = json!(1);
            }
        }
        assert!(
            !parse_report(format(), "frontend:unit", value.to_string().as_bytes())
                .is_ok_and(|r| r.successful),
            "case {change}"
        );
    }
    for report in [b"{}".as_slice(), b"", b"{invalid"] {
        assert!(parse_report(format(), "frontend:unit", report).is_err());
    }
}
