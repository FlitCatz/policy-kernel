use policy_contracts::{
    CommitSha, Risk, SensorCondition, SensorDefinition, SensorEvidence, SensorId, SensorProfile,
    SensorStatus, Sha256Digest, TestAdequacyEvidence, TestAdequacyPolicy, TestMapping, TrustClass,
    UtcTimestamp,
};
use policy_core::{
    evaluate_sensor_evidence, resolve_required_sensors, SensorEvaluationStatus,
};

fn id(value: &str) -> SensorId {
    SensorId::parse(value).unwrap()
}

fn profile() -> SensorProfile {
    SensorProfile {
        schema_version: 1,
        sensors: vec![
            SensorDefinition {
                id: id("compile"),
                required: true,
                required_when: vec![],
            },
            SensorDefinition {
                id: id("api-e2e"),
                required: false,
                required_when: vec![SensorCondition {
                    id: "high-risk-api".into(),
                    risks: vec![Risk::High, Risk::Critical],
                    path_globs: vec!["src/api/**".into(), "src/api.rs".into()],
                }],
            },
            SensorDefinition {
                id: id("docs"),
                required: false,
                required_when: vec![SensorCondition {
                    id: "docs-only".into(),
                    risks: vec![],
                    path_globs: vec!["docs/**".into()],
                }],
            },
        ],
        adequacy: TestAdequacyPolicy {
            acceptance_criterion_ids: vec!["AC-1".into()],
            changed_behavior_ids: vec!["BEHAVIOR-1".into()],
            negative_case_ids: vec!["NEG-1".into()],
        },
    }
}

fn sha() -> CommitSha {
    CommitSha::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap()
}

fn digest() -> Sha256Digest {
    Sha256Digest::parse(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    )
    .unwrap()
}

fn sensor(sensor_id: &str, status: SensorStatus) -> SensorEvidence {
    SensorEvidence {
        sensor_id: id(sensor_id),
        status,
        head_sha: sha(),
        command_digest: digest(),
        result_digest: digest(),
        observed_at: UtcTimestamp::parse("2026-09-04T00:00:00Z").unwrap(),
        trust: TrustClass::CiObserved,
        not_applicable_condition_id: None,
        not_applicable_reason: None,
    }
}

fn adequate() -> TestAdequacyEvidence {
    TestAdequacyEvidence {
        acceptance_criteria: vec![TestMapping {
            requirement_id: "AC-1".into(),
            test_ids: vec!["test-api".into()],
        }],
        changed_behaviors: vec![TestMapping {
            requirement_id: "BEHAVIOR-1".into(),
            test_ids: vec!["test-api".into()],
        }],
        negative_case_ids: vec!["NEG-1".into()],
        executed_test_ids: vec!["test-api".into()],
        discovered: 1,
        executed: 1,
        passed: 1,
    }
}

#[test]
fn required_when_requires_all_declared_dimensions() {
    let ids = resolve_required_sensors(&profile(), &["src/api.rs"], Risk::High).unwrap();
    assert_eq!(ids, [id("compile"), id("api-e2e")]);

    let path_only = resolve_required_sensors(&profile(), &["src/api.rs"], Risk::Low).unwrap();
    assert_eq!(path_only, [id("compile")]);
}

#[test]
fn path_normalization_is_deterministic_and_parent_segments_fail_closed() {
    let windows = resolve_required_sensors(&profile(), &["src\\api.rs"], Risk::High).unwrap();
    assert_eq!(windows, [id("compile"), id("api-e2e")]);

    assert!(resolve_required_sensors(&profile(), &["src/../api.rs"], Risk::High).is_err());
}

#[test]
fn missing_timeout_and_unparsable_required_sensors_are_inconclusive() {
    let missing = evaluate_sensor_evidence(
        &profile(),
        &[id("compile"), id("api-e2e")],
        &[sensor("compile", SensorStatus::Passed)],
        Some(&adequate()),
    );
    assert_eq!(missing.status, SensorEvaluationStatus::Inconclusive);
    assert!(missing.has_reason("PK-SENSOR-MISSING"));

    for status in [SensorStatus::TimedOut, SensorStatus::Skipped, SensorStatus::Unparsable] {
        let result = evaluate_sensor_evidence(
            &profile(),
            &[id("compile")],
            &[sensor("compile", status)],
            Some(&adequate()),
        );
        assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
    }
}

#[test]
fn an_executed_required_sensor_failure_is_blocked() {
    let result = evaluate_sensor_evidence(
        &profile(),
        &[id("compile")],
        &[sensor("compile", SensorStatus::Failed)],
        Some(&adequate()),
    );
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-SENSOR-FAILED"));
}

#[test]
fn not_applicable_needs_a_matching_condition_and_reason() {
    let mut evidence = sensor("api-e2e", SensorStatus::NotApplicable);
    evidence.not_applicable_condition_id = Some("high-risk-api".into());
    evidence.not_applicable_reason = Some("API condition was evaluated explicitly".into());
    let valid = evaluate_sensor_evidence(
        &profile(),
        &[id("api-e2e")],
        &[evidence],
        Some(&adequate()),
    );
    assert_eq!(valid.status, SensorEvaluationStatus::Passed);

    let invalid = evaluate_sensor_evidence(
        &profile(),
        &[id("api-e2e")],
        &[sensor("api-e2e", SensorStatus::NotApplicable)],
        Some(&adequate()),
    );
    assert_eq!(invalid.status, SensorEvaluationStatus::Inconclusive);
    assert!(invalid.has_reason("PK-SENSOR-NOT-APPLICABLE-INVALID"));
}

#[test]
fn zero_exit_with_zero_discovered_tests_fails_adequacy() {
    let mut no_tests = adequate();
    no_tests.acceptance_criteria.clear();
    no_tests.changed_behaviors.clear();
    no_tests.negative_case_ids.clear();
    no_tests.executed_test_ids.clear();
    no_tests.discovered = 0;
    no_tests.executed = 0;
    no_tests.passed = 0;

    let result = evaluate_sensor_evidence(
        &profile(),
        &[id("compile")],
        &[sensor("compile", SensorStatus::Passed)],
        Some(&no_tests),
    );
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-ADEQ-NO-TESTS"));
}

#[test]
fn adequacy_requires_executed_mappings_negative_cases_and_consistent_counts() {
    let mut unmapped = adequate();
    unmapped.executed_test_ids.clear();
    let result = evaluate_sensor_evidence(
        &profile(),
        &[id("compile")],
        &[sensor("compile", SensorStatus::Passed)],
        Some(&unmapped),
    );
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-ADEQ-UNEXECUTED-MAPPING"));

    let mut inconsistent = adequate();
    inconsistent.passed = 2;
    let result = evaluate_sensor_evidence(
        &profile(),
        &[id("compile")],
        &[sensor("compile", SensorStatus::Passed)],
        Some(&inconsistent),
    );
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-ADEQ-COUNT-MISMATCH"));
}

#[test]
fn missing_adequacy_evidence_is_inconclusive() {
    let result = evaluate_sensor_evidence(
        &profile(),
        &[id("compile")],
        &[sensor("compile", SensorStatus::Passed)],
        None,
    );
    assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
    assert!(result.has_reason("PK-ADEQ-MISSING"));
}
