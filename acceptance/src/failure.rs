use crate::execution::Scenario;
use policy_contracts::{ExecutionStatus, SensorStatus};
use project_verification::{reports::TestOutcome, run::NormalizedSensor};
use serde::Serialize;

fn failed_names(sensor: &NormalizedSensor) -> crate::Result<Vec<String>> {
    let report = sensor
        .configuration
        .sensor
        .report
        .as_ref()
        .ok_or("no report")?;
    let original = sensor
        .result
        .original_report
        .as_ref()
        .ok_or("no bound report")?;
    let bytes = std::fs::read(sensor.configuration.component_root.join(&report.path))?;
    if project_verification::bytes_digest(&bytes) != original.digest {
        return Err("report changed after normalization".into());
    }
    names_from_report(report.format, &bytes)
}

fn names_from_report(
    format: project_verification::reports::ReportFormat,
    bytes: &[u8],
) -> crate::Result<Vec<String>> {
    use project_verification::reports::ReportFormat;
    match format {
        ReportFormat::VitestJson => {
            let value: serde_json::Value = serde_json::from_slice(bytes)?;
            Ok(value["testResults"]
                .as_array()
                .ok_or("missing suites")?
                .iter()
                .flat_map(|suite| suite["assertionResults"].as_array().into_iter().flatten())
                .filter(|test| test["status"] == "failed")
                .filter_map(|test| test["fullName"].as_str().map(str::to_owned))
                .collect())
        }
        ReportFormat::Trx => {
            let document = roxmltree::Document::parse(std::str::from_utf8(bytes)?)?;
            Ok(document
                .descendants()
                .filter(|node| {
                    node.has_tag_name("UnitTestResult")
                        && node.attribute("outcome") == Some("Failed")
                })
                .filter_map(|node| node.attribute("testName").map(str::to_owned))
                .collect())
        }
        ReportFormat::PlaywrightJson => {
            let value: serde_json::Value = serde_json::from_slice(bytes)?;
            let mut pending = vec![&value];
            let mut names = Vec::new();
            while let Some(suite) = pending.pop() {
                pending.extend(suite["suites"].as_array().into_iter().flatten());
                for spec in suite["specs"].as_array().into_iter().flatten() {
                    for test in spec["tests"].as_array().into_iter().flatten() {
                        if test["results"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .any(|result| result["status"] == "failed")
                        {
                            names.push(
                                spec["title"]
                                    .as_str()
                                    .ok_or("missing spec title")?
                                    .to_owned(),
                            );
                        }
                    }
                }
            }
            Ok(names)
        }
    }
}

#[derive(Debug, Serialize)]
pub struct FailureObservation {
    pub sensor_status: SensorStatus,
    pub process_status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub version_matches: bool,
    pub discovered: Option<u64>,
    pub executed: Option<u64>,
    pub failed_tests: Vec<String>,
    pub failed_test_names: Vec<String>,
    pub failed_test_messages: Vec<String>,
    pub diagnostics: String,
}

impl FailureObservation {
    pub fn from_sensor(sensor: &NormalizedSensor) -> Self {
        let process = &sensor.collection.record.execution;
        let version = &sensor.collection.record.version_probe;
        let parsed = sensor.result.parsed.as_ref();
        Self {
            sensor_status: sensor.evidence.status,
            process_status: process.status,
            exit_code: process.exit_code,
            version_matches: version.status == ExecutionStatus::Passed
                && version.exit_code == Some(0)
                && String::from_utf8_lossy(&version.stdout).trim()
                    == sensor.configuration.sensor.version_stdout.trim(),
            discovered: parsed.map(|r| r.discovered),
            executed: parsed.map(|r| r.executed),
            failed_tests: parsed
                .map(|r| {
                    r.tests
                        .iter()
                        .filter(|t| t.outcome == TestOutcome::Failed)
                        .map(|t| t.id.clone())
                        .collect()
                })
                .unwrap_or_default(),
            failed_test_names: failed_names(sensor).unwrap_or_default(),
            failed_test_messages: parsed
                .map(|r| {
                    r.tests
                        .iter()
                        .filter(|t| t.outcome == TestOutcome::Failed)
                        .flat_map(|t| t.messages.clone())
                        .collect()
                })
                .unwrap_or_default(),
            diagnostics: format!(
                "{}\n{}",
                String::from_utf8_lossy(&process.stdout),
                String::from_utf8_lossy(&process.stderr)
            ),
        }
    }

    #[cfg(test)]
    fn proves(&self, scenario: Scenario) -> bool {
        self.proves_sensor(scenario, scenario.sensor().unwrap_or(""))
    }

    pub fn proves_sensor(&self, scenario: Scenario, sensor: &str) -> bool {
        if !scenario.rejecting_sensors().contains(&sensor) {
            return false;
        }
        if !self.version_matches || self.sensor_status != SensorStatus::Failed {
            return false;
        }
        if scenario == Scenario::EmptyReport {
            return self.process_status == ExecutionStatus::Passed
                && self.exit_code == Some(0)
                && self.discovered == Some(0)
                && self.executed == Some(0)
                && self.failed_tests.is_empty();
        }
        if self.process_status != ExecutionStatus::Failed
            || self.exit_code.is_none_or(|code| code <= 0)
        {
            return false;
        }
        if scenario == Scenario::TypeError {
            return self.diagnostics.lines().any(|line| {
                line.contains("src/type-error.ts(1,") && line.contains("error TS2322:")
            });
        }
        let api_test = "PolicyKernel.Example.Tests.Integration.GreetingEndpointTests.Request_with_acceptance_token_returns_the_greeting_contract";
        let (expected, markers): (&str, &[&str]) = match (scenario, sensor) {
            (Scenario::ComponentFailure, _) => (
                "renders a valid greeting payload supplied through component props",
                &["Greeting payload contains an unexpected message"],
            ),
            (Scenario::ApiAuthorization, _) => (
                api_test,
                &[
                    "Assert.Equal() Failure",
                    "Expected: OK",
                    "Actual: Unauthorized",
                ],
            ),
            (Scenario::ContractMismatch, "shared:contract") => (
                "the live API response satisfies the frontend greeting decoder",
                &["Greeting payload is missing message"],
            ),
            (Scenario::ContractMismatch, "backend:integration") => (
                api_test,
                &[
                    "Assert.Equal() Failure",
                    "Expected: \"Hello policy\"",
                    "Actual: null",
                ],
            ),
            (Scenario::ContractMismatch, "shared:e2e") => (
                "renders the greeting returned by the live API",
                &[
                    "getByRole('heading', { name: 'Hello policy' })",
                    "<element(s) not found>",
                ],
            ),
            _ => return false,
        };
        self.discovered.is_some_and(|count| count > 0)
            && self.executed.is_some_and(|count| count > 0)
            && self.failed_tests.len() == 1
            && self.failed_test_names.len() == 1
            && self
                .failed_test_names
                .iter()
                .any(|name| name.ends_with(expected))
            && self.failed_test_messages.iter().any(|message| {
                let normalized = message.split_whitespace().collect::<Vec<_>>().join(" ");
                markers.iter().all(|marker| normalized.contains(marker))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_hashed_ids_retain_failed_names_from_the_bound_report() {
        // A real report identity is hashed, so matching its ID against a title cannot work.
        let bytes = br#"{"testResults":[{"assertionResults":[{"status":"failed","fullName":"target assertion"},{"status":"passed","fullName":"unrelated assertion"}]}]}"#;
        assert_eq!(
            names_from_report(
                project_verification::reports::ReportFormat::VitestJson,
                bytes
            )
            .unwrap(),
            vec!["target assertion"]
        );
        let trx = br#"<TestRun xmlns="http://microsoft.com/schemas/VisualStudio/TeamTest/2010"><Results><UnitTestResult outcome="Failed" testName="target API test"/><UnitTestResult outcome="Passed" testName="other"/></Results></TestRun>"#;
        assert_eq!(
            names_from_report(project_verification::reports::ReportFormat::Trx, trx).unwrap(),
            vec!["target API test"]
        );
        let playwright = br#"{"suites":[{"suites":[{"specs":[{"title":"target browser test","tests":[{"results":[{"status":"failed"}]}]},{"title":"passed test","tests":[{"results":[{"status":"passed"}]}]}]}]}]}"#;
        assert_eq!(
            names_from_report(
                project_verification::reports::ReportFormat::PlaywrightJson,
                playwright
            )
            .unwrap(),
            vec!["target browser test"]
        );
    }
    fn failure() -> FailureObservation {
        FailureObservation {
            sensor_status: SensorStatus::Failed,
            process_status: ExecutionStatus::Failed,
            exit_code: Some(1),
            version_matches: true,
            discovered: Some(1),
            executed: Some(1),
            failed_tests: vec!["frontend:unit:hashed-id".into()],
            failed_test_names: vec![
                "renders a valid greeting payload supplied through component props".into(),
            ],
            diagnostics: String::new(),
            failed_test_messages: vec!["Greeting payload contains an unexpected message".into()],
        }
    }
    #[test]
    fn recognizes_assertion_failure_but_not_timeout_or_unparsable_report() {
        let mut observed = failure();
        assert!(observed.proves(Scenario::ComponentFailure));
        observed.sensor_status = SensorStatus::TimedOut;
        assert!(!observed.proves(Scenario::ComponentFailure));
        observed.sensor_status = SensorStatus::Unparsable;
        assert!(!observed.proves(Scenario::ComponentFailure));
        observed.sensor_status = SensorStatus::Failed;
        observed.failed_test_names = vec!["some unrelated failed test".into()];
        assert!(!observed.proves(Scenario::ComponentFailure));
    }

    #[test]
    fn targeted_test_with_unrelated_error_is_not_proof() {
        let mut observed = failure();
        assert!(observed.proves_sensor(Scenario::ComponentFailure, "frontend:unit"));
        observed.failed_test_messages = vec!["Cannot load test dependency".into()];
        assert!(!observed.proves_sensor(Scenario::ComponentFailure, "frontend:unit"));
    }

    #[test]
    fn contract_correlated_sensors_require_specific_native_failures() {
        for (id, name, message) in [
            ("shared:contract", "the live API response satisfies the frontend greeting decoder", "Greeting payload is missing message"),
            ("backend:integration", "PolicyKernel.Example.Tests.Integration.GreetingEndpointTests.Request_with_acceptance_token_returns_the_greeting_contract", "Assert.Equal() Failure: Strings differ\nExpected: \"Hello policy\"\nActual:   null"),
            ("shared:e2e", "renders the greeting returned by the live API", "getByRole('heading', { name: 'Hello policy' }) Expected: visible Received: <element(s) not found>"),
        ] {
            let mut observed = failure();
            observed.failed_test_names = vec![name.into()];
            observed.failed_test_messages = vec![message.into()];
            assert!(observed.proves_sensor(Scenario::ContractMismatch, id));
            observed.failed_test_messages = vec!["Connection refused".into()];
            assert!(!observed.proves_sensor(Scenario::ContractMismatch, id));
        }
    }
    #[test]
    fn empty_report_requires_zero_counts_from_a_successful_producer() {
        let mut observed = failure();
        assert!(!observed.proves(Scenario::EmptyReport));
        observed.discovered = Some(0);
        observed.executed = Some(0);
        observed.failed_tests.clear();
        observed.process_status = ExecutionStatus::Passed;
        observed.exit_code = Some(0);
        assert!(observed.proves(Scenario::EmptyReport));
        observed.discovered = None;
        assert!(!observed.proves(Scenario::EmptyReport));
    }

    #[test]
    fn type_error_requires_the_injected_file_and_diagnostic() {
        let mut observed = failure();
        observed.diagnostics = "src/type-error.ts(1,14): error TS2322: Type 'string' is not assignable to type 'number'.".into();
        assert!(observed.proves(Scenario::TypeError));
        observed.diagnostics = "src/other.ts(1,14): error TS2322: unrelated".into();
        assert!(!observed.proves(Scenario::TypeError));
        observed.diagnostics = "src/type-error.ts: file unavailable".into();
        assert!(!observed.proves(Scenario::TypeError));
    }

    #[test]
    fn api_and_contract_require_the_target_test_and_matching_runtime() {
        for (scenario, name) in [
            (Scenario::ApiAuthorization, "backend:integration:PolicyKernel.Example.Tests.Integration.GreetingEndpointTests.Request_with_acceptance_token_returns_the_greeting_contract"),
            (Scenario::ContractMismatch, "shared:contract:fixture:the live API response satisfies the frontend greeting decoder"),
        ] {
            let mut observed = failure();
            observed.failed_test_names = vec![name.into()];
            observed.failed_test_messages = vec![match scenario {
                Scenario::ApiAuthorization => "Assert.Equal() Failure: Values differ Expected: OK Actual: Unauthorized",
                _ => "Greeting payload is missing message",
            }.into()];
            assert!(observed.proves(scenario));
            observed.version_matches = false;
            assert!(!observed.proves(scenario));
            observed.version_matches = true;
            observed.process_status = ExecutionStatus::TimedOut;
            assert!(!observed.proves(scenario));
        }
        assert!(!failure().proves(Scenario::Baseline));
    }
}
