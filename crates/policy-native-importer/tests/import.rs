#![cfg(unix)]
use policy_contracts::{ExecutionId, UtcTimestamp};
use policy_native_contracts::*;
use policy_native_importer::*;
use policy_report_adapters::{parse_report, ReportFormat};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};

struct Fixture {
    temp: tempfile::TempDir,
    input: PathBuf,
    expected: ExpectedScopeV1,
    identity: ExpectedIdentity,
    run: NativeRunV1,
}
impl Fixture {
    fn new(format: ReportFormat) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let input = temp.path().canonicalize().unwrap().join("input");
        fs::create_dir(&input).unwrap();
        let corpus: Value = serde_json::from_str(include_str!(
            "../../policy-native-contracts/tests/corpus/v1.json"
        ))
        .unwrap();
        let mut expected: ExpectedScopeV1 =
            serde_json::from_value(corpus["valid"]["expected"].clone()).unwrap();
        let mut run: NativeRunV1 = serde_json::from_value(corpus["valid"]["run"].clone()).unwrap();
        let (id, bytes, parts): (&str, &[u8], [&str; 2]) = match format {
            ReportFormat::VitestJson => (
                "vitest_json",
                include_bytes!(
                    "../../policy-report-adapters/tests/fixtures/project-reports/vitest.json"
                ),
                [
                    "/repo/test/login.test.ts",
                    "login accepts valid credentials",
                ],
            ),
            ReportFormat::JestJson => (
                "jest_json",
                include_bytes!(
                    "../../policy-report-adapters/tests/fixtures/project-reports/jest.json"
                ),
                [
                    "/repo/test/login.test.ts",
                    "login accepts valid credentials",
                ],
            ),
            ReportFormat::PlaywrightJson => (
                "playwright_json",
                include_bytes!(
                    "../../policy-report-adapters/tests/fixtures/project-reports/playwright.json"
                ),
                ["login-spec", "chromium"],
            ),
            ReportFormat::Trx => (
                "trx",
                include_bytes!(
                    "../../policy-report-adapters/tests/fixtures/project-reports/vstest.trx"
                ),
                ["login-test", "Login.AcceptsCredentials"],
            ),
        };
        let parsed = parse_report(format, "unit", bytes).unwrap();
        let parser = ParserBindingV1 {
            id: RecordId::parse(id).unwrap(),
            version: RecordId::parse(format.parser_version()).unwrap(),
        };
        let mut bindings = expected.bindings.observations.as_slice().to_vec();
        bindings[0].parser = Some(parser.clone());
        bindings[0].command_digest =
            canonical_digest(DigestRole::Command, &run.observations[0].command).unwrap();
        expected.bindings.observations = RequiredSet::new(bindings).unwrap();
        expected.bindings.evaluated_at = UtcTimestamp::parse("2025-09-01T00:01:00Z").unwrap();
        run.observations[0].started_at = parsed.started_at.clone();
        run.observations[0].completed_at = UtcTimestamp::parse("2025-09-01T00:00:10Z").unwrap();
        run.observations[0].observed_at = UtcTimestamp::parse("2025-09-01T00:00:11Z").unwrap();
        run.artifacts[0].digest = raw_digest(bytes);
        let test = &mut run.test_observations[0];
        test.parser = parser;
        test.raw_report.digest = raw_digest(bytes);
        test.tests[0].id = RecordId::parse(parsed.tests[0].id.clone()).unwrap();
        test.tests[0].raw_identity =
            RecordId::parse(serde_json::to_string(&parts).unwrap()).unwrap();
        if format == ReportFormat::JestJson {
            test.tests = parsed
                .tests
                .iter()
                .zip(["renders the selected greeting", "rejects an empty greeting"])
                .map(|(parsed, title)| TestV1 {
                    id: RecordId::parse(parsed.id.clone()).unwrap(),
                    raw_identity: RecordId::parse(
                        serde_json::to_string(&["tests/Greeting.test.cjs", title]).unwrap(),
                    )
                    .unwrap(),
                    outcome: TestOutcome::Passed,
                    attempts: 1,
                })
                .collect();
            test.counts.discovered = 2;
            test.counts.executed = 2;
            test.counts.passed = 2;
        }
        for mapping in &mut run.mappings {
            mapping.tests = RequiredSet::new(vec![TestRefV1 {
                suite_id: test.observation_id.clone(),
                test_id: test.tests[0].id.clone(),
            }])
            .unwrap();
        }
        fs::write(input.join("raw.json"), bytes).unwrap();
        let identity = ExpectedIdentity {
            run_id: run.subject.run_id.clone(),
            scope_id: run.subject.scope_id.clone(),
        };
        Self {
            temp,
            input,
            expected,
            identity,
            run,
        }
    }
    fn output(&self) -> PathBuf {
        self.temp.path().canonicalize().unwrap().join("output")
    }
    fn import(&self) -> ImportRecord {
        let result = import_run(
            &self.input,
            &self.output(),
            &self.expected,
            &self.identity,
            &self.run,
        )
        .unwrap();
        assert!(self.output().join("complete").exists());
        assert!(self.output().join("import.json").exists());
        assert!(!self.output().join("complete.pending").exists());
        result
    }
    fn replace_raw(&mut self, value: Value) {
        let bytes = serde_json::to_vec(&value).unwrap();
        fs::write(self.input.join("raw.json"), &bytes).unwrap();
        self.run.artifacts[0].digest = raw_digest(&bytes);
        self.run.test_observations[0].raw_report.digest = raw_digest(&bytes);
    }
    fn raw(&self) -> Value {
        serde_json::from_slice(&fs::read(self.input.join("raw.json")).unwrap()).unwrap()
    }
}

#[test]
fn shared_corpus_imports_all_formats_and_preserves_exact_bytes() {
    for format in [
        ReportFormat::VitestJson,
        ReportFormat::JestJson,
        ReportFormat::PlaywrightJson,
        ReportFormat::Trx,
    ] {
        let f = Fixture::new(format);
        let result = f.import();
        assert!(result.issues.is_empty(), "{format:?}: {:?}", result.issues);
        assert_eq!(result.reports.len(), 1);
        assert_eq!(result.trust, LocalTrust::LocalUntrusted);
        assert_eq!(
            fs::read(f.output().join(&result.artifacts[0].stored_file)).unwrap(),
            fs::read(f.input.join("raw.json")).unwrap()
        );
    }
}

#[test]
fn wrong_subject_identity_stale_future_and_replayed_raw_are_rejected() {
    for case in 0..10 {
        let mut f = Fixture::new(ReportFormat::VitestJson);
        match case {
            0 => {
                f.expected.subject.head =
                    policy_contracts::CommitSha::parse("2".repeat(40)).unwrap()
            }
            1 => {
                f.expected.subject.source_set_digest.sha256 =
                    policy_contracts::Sha256Digest::parse("2".repeat(64)).unwrap()
            }
            2 => {
                f.expected.subject.repository =
                    policy_contracts::RepositoryId::parse("other/repo").unwrap()
            }
            3 => f.identity.run_id = ExecutionId::parse("fresh-run").unwrap(),
            4 => f.identity.scope_id = RecordId::parse("other-scope").unwrap(),
            5 => f.expected.bindings.maximum_age_seconds = 59,
            6 => {
                f.expected.bindings.evaluated_at =
                    UtcTimestamp::parse("2025-09-01T00:00:05Z").unwrap()
            }
            7 => {
                let mut v = f.raw();
                v["startTime"] = json!(1756684799000i64);
                f.replace_raw(v);
            }
            8 => {
                f.expected.bindings.maximum_age_seconds = 60;
                f.expected.bindings.evaluated_at =
                    UtcTimestamp::parse("2025-09-01T00:01:00.001Z").unwrap();
            }
            _ => f.run.observations[0].subject.run_id = ExecutionId::parse("other-child").unwrap(),
        }
        let result = f.import();
        assert!(!result.issues.is_empty(), "case {case}");
        assert_eq!(result.artifacts.len(), 1);
    }
}

#[test]
fn missing_hash_malformed_skip_retry_duplicate_and_summary_fail_closed() {
    for case in 0..9 {
        let mut f = Fixture::new(if case == 4 {
            ReportFormat::PlaywrightJson
        } else {
            ReportFormat::VitestJson
        });
        match case {
            0 => fs::remove_file(f.input.join("raw.json")).unwrap(),
            1 => fs::write(f.input.join("raw.json"), b"changed").unwrap(),
            2 => {
                fs::write(f.input.join("raw.json"), b"{").unwrap();
                f.run.artifacts[0].digest = raw_digest(b"{");
                f.run.test_observations[0].raw_report.digest = raw_digest(b"{");
            }
            3 => {
                let mut v = f.raw();
                v["testResults"][0]["assertionResults"][0]["status"] = json!("skipped");
                v["numPassedTests"] = json!(0);
                v["numPendingTests"] = json!(1);
                f.replace_raw(v);
            }
            4 => {
                let mut v = f.raw();
                let attempt = v["suites"][0]["specs"][0]["tests"][0]["results"][0].clone();
                v["suites"][0]["specs"][0]["tests"][0]["results"]
                    .as_array_mut()
                    .unwrap()
                    .push(attempt);
                f.replace_raw(v);
            }
            5 => f
                .run
                .test_observations
                .push(f.run.test_observations[0].clone()),
            6 => {
                let mut v = f.raw();
                v["numTotalTests"] = json!(9);
                f.replace_raw(v);
            }
            7 => {
                let mut v = f.raw();
                v["testResults"][0]["assertionResults"][0]["retryCount"] = json!(1);
                f.replace_raw(v);
            }
            _ => {
                let mut v = f.raw();
                let suite = v["testResults"][0].clone();
                v["testResults"].as_array_mut().unwrap().push(suite);
                f.replace_raw(v);
            }
        }
        let result = f.import();
        assert!(!result.issues.is_empty(), "case {case}");
        if case != 0 {
            assert_eq!(result.artifacts.len(), 1);
        }
    }
}

#[test]
fn forged_tests_identities_counts_attempts_and_parser_bindings_are_rejected() {
    for case in 0..6 {
        let mut f = Fixture::new(ReportFormat::VitestJson);
        match case {
            0 => {
                f.run.test_observations[0].tests[0].raw_identity =
                    RecordId::parse("forged").unwrap()
            }
            1 => {
                f.run.test_observations[0].tests[0].outcome = TestOutcome::Failed;
                f.run.test_observations[0].counts.passed = 0;
                f.run.test_observations[0].counts.failed = 1;
            }
            2 => {
                f.run.test_observations[0].tests[0].attempts = 2;
                f.run.test_observations[0].counts.retries = 1;
            }
            3 => f.run.test_observations[0].parser.version = RecordId::parse("unknown").unwrap(),
            4 => {
                let mut bindings = f.expected.bindings.observations.as_slice().to_vec();
                bindings[0].parser.as_mut().unwrap().id = RecordId::parse("unknown").unwrap();
                f.run.test_observations[0].parser = bindings[0].parser.clone().unwrap();
                f.expected.bindings.observations = RequiredSet::new(bindings).unwrap();
            }
            _ => {
                f.run.test_observations.clear();
                f.run.mappings.clear();
            }
        }
        assert!(!f.import().issues.is_empty(), "case {case}");
    }
}

#[test]
fn path_attacks_extra_entries_and_size_limits_preserve_safe_artifacts() {
    for case in 0..9 {
        let mut f = Fixture::new(ReportFormat::VitestJson);
        match case {
            0 => {
                fs::remove_file(f.input.join("raw.json")).unwrap();
                std::os::unix::fs::symlink("../secret", f.input.join("raw.json")).unwrap();
                fs::write(f.temp.path().join("secret"), "secret").unwrap();
            }
            1 => fs::write(f.input.join("extra"), "retained extra").unwrap(),
            2 => fs::create_dir(f.input.join("extra-dir")).unwrap(),
            3 => f.run.artifacts[0].path = "../outside".into(),
            4 => f.run.artifacts[0].path = "/absolute".into(),
            5 => {
                fs::File::create(f.input.join("huge"))
                    .unwrap()
                    .set_len(16 * 1024 * 1024 + 1)
                    .unwrap();
            }
            6 => {
                std::os::unix::fs::symlink(f.temp.path(), f.input.join("dir")).unwrap();
                f.run.artifacts[0].path = "dir/raw.json".into();
            }
            7 => f.run.artifacts[0].path = "a\\b".into(),
            _ => {
                std::os::unix::net::UnixListener::bind(f.input.join("socket")).unwrap();
            }
        }
        let result = f.import();
        assert!(!result.issues.is_empty(), "case {case}");
        if case == 1 {
            assert_eq!(result.artifacts.len(), 2);
        }
        if case == 0 {
            assert!(result.artifacts.is_empty());
        }
    }
}

#[test]
fn output_is_fresh_disjoint_and_never_overwritten() {
    let f = Fixture::new(ReportFormat::VitestJson);
    assert!(import_run(
        &f.input,
        &f.input.join("out"),
        &f.expected,
        &f.identity,
        &f.run
    )
    .is_err());
    assert!(!f.input.join("out").exists());
    f.import();
    let before = fs::read(f.output().join("import.json")).unwrap();
    assert!(import_run(&f.input, &f.output(), &f.expected, &f.identity, &f.run).is_err());
    assert_eq!(before, fs::read(f.output().join("import.json")).unwrap());
}

#[test]
fn failed_native_tests_are_retained_without_a_scope_decision() {
    let mut f = Fixture::new(ReportFormat::VitestJson);
    let mut v = f.raw();
    v["numPassedTests"] = json!(0);
    v["numFailedTests"] = json!(1);
    v["success"] = json!(false);
    v["testResults"][0]["assertionResults"][0]["status"] = json!("failed");
    f.replace_raw(v);
    f.run.test_observations[0].counts.passed = 0;
    f.run.test_observations[0].counts.failed = 1;
    f.run.test_observations[0].tests[0].outcome = TestOutcome::Failed;
    f.run.observations[0].completion = CompletionV1::Exited { code: 1 };
    let result = f.import();
    assert!(result.issues.is_empty(), "{:?}", result.issues);
    assert_eq!(result.reports[0].observation.counts.failed, 1);
    assert!(!result.reports[0].report.successful);
    let wire = serde_json::to_value(result).unwrap();
    assert!(wire.get("status").is_none());
    assert!(wire.get("evidence").is_none());
}

#[test]
fn nested_artifacts_and_symlinked_roots_have_no_implicit_path_exceptions() {
    let mut f = Fixture::new(ReportFormat::VitestJson);
    fs::create_dir(f.input.join("reports")).unwrap();
    fs::rename(f.input.join("raw.json"), f.input.join("reports/raw.json")).unwrap();
    f.run.artifacts[0].path = "reports/raw.json".into();
    assert!(f.import().issues.is_empty());

    let f = Fixture::new(ReportFormat::VitestJson);
    let alias = f.temp.path().canonicalize().unwrap().join("alias");
    std::os::unix::fs::symlink(&f.input, &alias).unwrap();
    let record = import_run(&alias, &f.output(), &f.expected, &f.identity, &f.run).unwrap();
    assert!(!record.issues.is_empty());
    assert!(record.artifacts.is_empty());
    assert!(f.output().join("complete").exists());
    assert!(import_run(
        &f.input,
        &alias.join("output"),
        &f.expected,
        &f.identity,
        &f.run
    )
    .is_err());
    assert!(!f.input.join("output").exists());
}

#[test]
fn failed_process_logs_survive_even_when_no_raw_report_was_produced() {
    let mut f = Fixture::new(ReportFormat::VitestJson);
    f.run.observations[0].completion = CompletionV1::Error {
        detail: RecordId::parse("native tool failed").unwrap(),
    };
    f.run.test_observations.clear();
    f.run.mappings.clear();
    fs::remove_file(f.input.join("raw.json")).unwrap();
    let log = b"native tool failed before producing report\n";
    fs::write(f.input.join("stderr.log"), log).unwrap();
    f.run.artifacts.push(ArtifactV1 {
        id: RecordId::parse("stderr").unwrap(),
        path: "stderr.log".into(),
        role: ArtifactRole::CommandLog,
        digest: raw_digest(log),
    });
    let record = f.import();
    assert!(!record.issues.is_empty());
    assert_eq!(record.artifacts.len(), 1);
    assert_eq!(
        fs::read(f.output().join(&record.artifacts[0].stored_file)).unwrap(),
        log
    );
    assert!(fs::read_to_string(f.output().join("run.json"))
        .unwrap()
        .contains("native tool failed"));
    assert!(record.reports.is_empty());
}

#[path = "../../project-verification/tests/common/mod.rs"]
mod project_fixture;

#[test]
fn importer_verifier_command_and_project_normalization_agree_on_failed_fixture() {
    for failed in [false, true] {
        let mut f = Fixture::new(ReportFormat::VitestJson);
        if failed {
            let mut v = f.raw();
            v["numPassedTests"] = json!(0);
            v["numFailedTests"] = json!(1);
            v["success"] = json!(false);
            v["testResults"][0]["assertionResults"][0]["status"] = json!("failed");
            f.replace_raw(v);
            f.run.test_observations[0].counts.passed = 0;
            f.run.test_observations[0].counts.failed = 1;
            f.run.test_observations[0].tests[0].outcome = TestOutcome::Failed;
            f.run.observations[0].completion = CompletionV1::Exited { code: 1 };
        }
        let record = f.import();
        assert!(record.issues.is_empty(), "{:?}", record.issues);
        let imported = record.verified_observations().unwrap();
        let evaluation = policy_native_verifier::evaluate_scope(&f.expected, imported).unwrap();
        let (result, exit) = policy_cli::evaluate_scope_json(
            &serde_json::to_vec(&f.expected).unwrap(),
            &fs::read(f.output().join("observations.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(exit, if failed { 2 } else { 0 });
        assert_eq!(result, evaluation.result);
        assert_eq!(result.trust, LocalTrust::LocalUntrusted);
        assert_eq!(result.review, ReviewGate::Unmet);

        // Feed exactly the same report bytes through the compatibility normalizer.
        let root = tempfile::tempdir().unwrap();
        let request = project_fixture::request(root.path());
        let config = project_verification::run::prepare(&request)
            .unwrap()
            .configurations
            .remove(0);
        let raw = fs::read(f.input.join("raw.json")).unwrap();
        fs::create_dir(root.path().join("reports")).unwrap();
        fs::write(root.path().join("reports/unit.json"), &raw).unwrap();
        let mut collection = project_fixture::collection(&request, &config, &raw);
        if failed {
            collection.record.execution.exit_code = Some(1);
            collection.record.execution.status = policy_contracts::ExecutionStatus::Failed;
            collection.evidence.status = policy_contracts::ExecutionStatus::Failed;
            project_fixture::reseal(&mut collection);
        }
        let normalized =
            project_verification::run::normalize_collection(&request, config, collection).unwrap();
        let native_evidence = evaluation.evidence.as_ref().unwrap();
        assert_eq!(
            normalized.evidence.status,
            native_evidence.sensors[0].status
        );
        let mut sensor = normalized.evidence;
        sensor.sensor_id = native_evidence.sensors[0].sensor_id.clone();
        let comparison = policy_core::evaluate_sensor_evidence(
            &evaluation.profile,
            &[sensor.sensor_id.clone()],
            &[sensor],
            native_evidence.test_adequacy.as_ref(),
        );
        assert_eq!(comparison.reasons, evaluation.core_reasons);
        if failed {
            assert!(comparison.has_reason("PK-SENSOR-FAILED"));
        }
    }
}

#[test]
fn rejected_imports_never_export_verified_observations() {
    let f = Fixture::new(ReportFormat::VitestJson);
    fs::write(f.input.join("raw.json"), b"tampered").unwrap();
    let record = f.import();
    assert!(record.verified_observations().is_none());
    assert!(!f.output().join("observations.json").exists());
}

#[test]
fn parser_level_failure_survives_even_when_all_tests_pass() {
    let mut f = Fixture::new(ReportFormat::VitestJson);
    let mut raw = f.raw();
    raw["testResults"][0]["message"] = json!("suite teardown failed");
    f.replace_raw(raw);
    let record = f.import();
    assert!(record.issues.is_empty(), "{:?}", record.issues);
    let observations = record.verified_observations().unwrap();
    assert_eq!(observations.unsuccessful_reports.len(), 1);
    let e = policy_native_verifier::evaluate_scope(&f.expected, observations).unwrap();
    assert_eq!(e.result.status, ScopeStatus::NonPass);
    assert!(e.core_reasons.iter().any(|r| r.code == "PK-SENSOR-FAILED"));
}
