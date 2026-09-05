use policy_contracts::{
    CommitSha, Risk, SensorCondition, SensorDefinition, SensorEvidence, SensorId, SensorProfile,
    SensorStatus, Sha256Digest, TestAdequacyEvidence, TestAdequacyPolicy, TrustClass, UtcTimestamp,
};
use policy_core::{
    evaluate_sensor_evidence_for_change, resolve_required_sensors, SensorEvaluationStatus,
};

fn id(value: &str) -> SensorId {
    SensorId::parse(value).unwrap()
}

fn condition(id: &str, glob: &str) -> SensorCondition {
    SensorCondition {
        id: id.into(),
        risks: vec![],
        path_globs: vec![glob.into()],
    }
}

fn profile() -> SensorProfile {
    SensorProfile {
        schema_version: 1,
        sensors: vec![SensorDefinition {
            id: id("compile"),
            required: true,
            required_when: vec![],
            not_applicable_when: vec![condition("docs-only", "docs/**")],
            accepted_exit_codes: vec![0],
        }],
        adequacy: TestAdequacyPolicy {
            acceptance_criterion_ids: vec![],
            changed_behavior_ids: vec![],
            negative_case_ids: vec![],
        },
    }
}

fn evidence(status: SensorStatus, exit_code: Option<i32>) -> SensorEvidence {
    SensorEvidence {
        sensor_id: id("compile"),
        status,
        head_sha: CommitSha::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap(),
        command_digest: digest('b'),
        result_digest: digest('c'),
        observed_at: UtcTimestamp::parse("2026-09-04T00:00:00Z").unwrap(),
        trust: TrustClass::CiObserved,
        resolved_command: vec!["cargo".into(), "test".into()],
        source_digest: Some(digest('d')),
        configuration_digest: Some(digest('e')),
        parser_version: Some("1".into()),
        exit_code,
        discovered: None,
        executed: None,
        passed: None,
        not_applicable_condition_id: None,
        not_applicable_reason: None,
    }
}

fn digest(character: char) -> Sha256Digest {
    Sha256Digest::parse(character.to_string().repeat(64)).unwrap()
}

fn empty_adequacy() -> TestAdequacyEvidence {
    TestAdequacyEvidence {
        acceptance_criteria: vec![],
        changed_behaviors: vec![],
        negative_case_ids: vec![],
        executed_test_ids: vec![],
        discovered: 0,
        executed: 0,
        passed: 0,
    }
}

#[test]
fn passed_status_with_unaccepted_exit_code_is_blocked() {
    let result = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[evidence(SensorStatus::Passed, Some(1))],
        None,
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-SENSOR-EXIT-CODE"));
}

#[test]
fn not_applicable_must_match_an_explicit_na_condition_for_this_change() {
    let mut item = evidence(SensorStatus::NotApplicable, None);
    item.not_applicable_condition_id = Some("docs-only".into());
    item.not_applicable_reason = Some("documentation-only change".into());

    let mismatch = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[item.clone()],
        None,
    )
    .unwrap();
    assert_eq!(mismatch.status, SensorEvaluationStatus::Inconclusive);

    let matched = evaluate_sensor_evidence_for_change(
        &profile(),
        &["docs/readme.md"],
        Risk::Low,
        &[item],
        None,
    )
    .unwrap();
    assert_eq!(matched.status, SensorEvaluationStatus::Passed);
}

#[test]
fn malformed_glob_and_duplicate_sensor_ids_fail_closed() {
    let mut malformed = profile();
    malformed.sensors[0].not_applicable_when = vec![condition("bad", "[")];
    assert!(resolve_required_sensors(&malformed, &["src/lib.rs"], Risk::Low).is_err());

    let mut duplicate = profile();
    duplicate.sensors.push(duplicate.sensors[0].clone());
    assert!(resolve_required_sensors(&duplicate, &["src/lib.rs"], Risk::Low).is_err());
}

#[test]
fn duplicate_executed_test_ids_fail_adequacy() {
    let mut adequacy = empty_adequacy();
    adequacy.discovered = 2;
    adequacy.executed = 2;
    adequacy.passed = 2;
    adequacy.executed_test_ids = vec!["same".into(), "same".into()];
    let result = evaluate_sensor_evidence_for_change(
        &profile(),
        &["src/lib.rs"],
        Risk::Low,
        &[evidence(SensorStatus::Passed, Some(0))],
        Some(&adequacy),
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-ADEQ-DUPLICATE-TEST"));
}
