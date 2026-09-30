use policy_contracts::{
    CommitSha, Risk, SensorCondition, SensorDefinition, SensorEvidence, SensorId, SensorProfile,
    SensorStatus, Sha256Digest, TestAdequacyEvidence, TestAdequacyPolicy, TestMapping, TrustClass,
    UtcTimestamp,
};
use policy_core::{evaluate_sensor_evidence, resolve_required_sensors, SensorEvaluationStatus};

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
                not_applicable_when: vec![],
                accepted_exit_codes: vec![0],
                result_requirement: None,
                result_binding: None,
            },
            SensorDefinition {
                id: id("api-e2e"),
                required: false,
                required_when: vec![SensorCondition {
                    id: "high-risk-api".into(),
                    risks: vec![Risk::High, Risk::Critical],
                    path_globs: vec!["src/api/**".into(), "src/api.rs".into()],
                }],
                not_applicable_when: vec![],
                accepted_exit_codes: vec![0],
                result_requirement: None,
                result_binding: None,
            },
            SensorDefinition {
                id: id("docs"),
                required: false,
                required_when: vec![SensorCondition {
                    id: "docs-only".into(),
                    risks: vec![],
                    path_globs: vec!["docs/**".into()],
                }],
                not_applicable_when: vec![],
                accepted_exit_codes: vec![0],
                result_requirement: None,
                result_binding: None,
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
    Sha256Digest::parse("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap()
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
        resolved_command: vec!["sensor".into()],
        source_digest: Some(digest()),
        configuration_digest: Some(digest()),
        parser_version: Some("1".into()),
        exit_code: Some(0),
        discovered: None,
        executed: None,
        passed: None,
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

    for status in [
        SensorStatus::TimedOut,
        SensorStatus::Skipped,
        SensorStatus::Unparsable,
    ] {
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
fn context_free_not_applicable_cannot_authorize_a_required_sensor() {
    let mut evidence = sensor("api-e2e", SensorStatus::NotApplicable);
    evidence.not_applicable_condition_id = Some("high-risk-api".into());
    evidence.not_applicable_reason = Some("API condition was evaluated explicitly".into());
    let valid =
        evaluate_sensor_evidence(&profile(), &[id("api-e2e")], &[evidence], Some(&adequate()));
    assert_eq!(valid.status, SensorEvaluationStatus::Inconclusive);
    assert!(valid.has_reason("PK-SENSOR-NOT-APPLICABLE-INVALID"));

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

#[test]
fn profile_fixture_satisfies_the_strict_contract() {
    let parsed: SensorProfile =
        serde_json::from_str(include_str!("../../../tests/fixtures/sensors/profile.json"))
            .expect("profile fixture must satisfy the strict sensor contract");
    assert_eq!(parsed, profile());
}

#[test]
fn fullstack_empty_component_suite_cannot_hide_behind_aggregate_adequacy() {
    let mut combined = profile();
    combined.sensors.truncate(2);
    combined.sensors[0].id = id("frontend:unit");
    combined.sensors[1].id = id("backend:unit");
    combined.sensors[1].required = true;
    let mut frontend = sensor("frontend:unit", SensorStatus::Passed);
    frontend.discovered = Some(1);
    frontend.executed = Some(1);
    frontend.passed = Some(1);
    let mut backend = sensor("backend:unit", SensorStatus::Passed);
    backend.discovered = Some(0);
    backend.executed = Some(0);
    backend.passed = Some(0);

    let result = policy_core::evaluate_sensor_evidence_for_change(
        &combined,
        &["frontend/app.tsx", "backend/Api.cs"],
        Risk::Low,
        &[frontend, backend],
        Some(&adequate()),
    )
    .unwrap();
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-ADEQ-NO-TESTS"));
}

#[test]
fn fullstack_requires_each_component_and_shared_checks() {
    let mut combined = profile();
    combined.sensors.clear();
    let names = [
        "frontend:unit",
        "backend:unit",
        "shared:contract",
        "shared:e2e",
    ];
    for name in names {
        let mut definition = profile().sensors.remove(0);
        definition.id = id(name);
        combined.sensors.push(definition);
    }
    let evidence: Vec<_> = names
        .iter()
        .map(|name| sensor(name, SensorStatus::Passed))
        .collect();
    let required = resolve_required_sensors(&combined, &["frontend/app.tsx"], Risk::Low).unwrap();
    assert_eq!(required, names.map(id));
    assert_eq!(
        evaluate_sensor_evidence(&combined, &required, &evidence, Some(&adequate())).status,
        SensorEvaluationStatus::Passed
    );
    for missing in 0..names.len() {
        let mut incomplete = evidence.clone();
        incomplete.remove(missing);
        let result = evaluate_sensor_evidence(&combined, &required, &incomplete, Some(&adequate()));
        assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
        assert!(result.has_reason("PK-SENSOR-MISSING"));
    }
}

#[test]
fn skipped_tests_inside_successful_process_do_not_pass() {
    let mut evidence = sensor("compile", SensorStatus::Passed);
    evidence.discovered = Some(2);
    evidence.executed = Some(1);
    evidence.passed = Some(1);
    let result =
        evaluate_sensor_evidence(&profile(), &[id("compile")], &[evidence], Some(&adequate()));
    assert_eq!(result.status, SensorEvaluationStatus::Blocked);
    assert!(result.has_reason("PK-SENSOR-RESULT-COUNT-MISMATCH"));
}

#[test]
fn duplicate_composed_sensor_ids_cannot_bypass_context_free_validation() {
    let mut combined = profile();
    combined.sensors.push(combined.sensors[0].clone());
    let result = evaluate_sensor_evidence(
        &combined,
        &[id("compile")],
        &[sensor("compile", SensorStatus::Passed)],
        Some(&adequate()),
    );
    assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
    assert!(result.has_reason("PK-SENSOR-PROFILE-INVALID"));
}

#[test]
fn v2_test_sensor_requires_complete_nonzero_counts_despite_aggregate_success() {
    let mut profile = profile();
    profile.schema_version = 2;
    for definition in &mut profile.sensors {
        definition.result_requirement = Some(policy_contracts::SensorResultRequirement::TestCounts);
    }
    // Exercise every incomplete combination, not just entirely absent counts.
    for mask in 0..7 {
        let mut item = sensor("compile", SensorStatus::Passed);
        item.discovered = (mask & 1 != 0).then_some(1);
        item.executed = (mask & 2 != 0).then_some(1);
        item.passed = (mask & 4 != 0).then_some(1);
        let result =
            evaluate_sensor_evidence(&profile, &[id("compile")], &[item], Some(&adequate()));
        assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
        assert!(result.has_reason("PK-SENSOR-RESULT-INCOMPLETE"));
    }
    for (counts, expected, reason) in [
        (
            (0, 0, 0),
            SensorEvaluationStatus::Blocked,
            Some("PK-ADEQ-NO-TESTS"),
        ),
        (
            (2, 1, 1),
            SensorEvaluationStatus::Blocked,
            Some("PK-SENSOR-RESULT-COUNT-MISMATCH"),
        ),
        ((1, 1, 1), SensorEvaluationStatus::Passed, None),
    ] {
        let mut item = sensor("compile", SensorStatus::Passed);
        item.discovered = Some(counts.0);
        item.executed = Some(counts.1);
        item.passed = Some(counts.2);
        let result =
            evaluate_sensor_evidence(&profile, &[id("compile")], &[item], Some(&adequate()));
        assert_eq!(result.status, expected);
        if let Some(reason) = reason {
            assert!(result.has_reason(reason));
        }
    }
    profile.sensors[0].result_requirement =
        Some(policy_contracts::SensorResultRequirement::Process);
    assert_eq!(
        evaluate_sensor_evidence(
            &profile,
            &[id("compile")],
            &[sensor("compile", SensorStatus::Passed)],
            Some(&adequate())
        )
        .status,
        SensorEvaluationStatus::Passed
    );
}

#[test]
fn directly_constructed_invalid_profile_versions_fail_both_evaluation_paths() {
    for (version, requirement) in [
        (99, None),
        (2, None),
        (1, Some(policy_contracts::SensorResultRequirement::Process)),
    ] {
        let mut profile = profile();
        profile.schema_version = version;
        profile.sensors[0].result_requirement = requirement;
        assert!(resolve_required_sensors(&profile, &["src/api.rs"], Risk::High).is_err());
        let result = evaluate_sensor_evidence(
            &profile,
            &[id("compile")],
            &[sensor("compile", SensorStatus::Passed)],
            Some(&adequate()),
        );
        assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
        assert!(result.has_reason("PK-SENSOR-PROFILE-INVALID"));
    }
}

#[test]
fn policy_bound_result_provenance_cannot_be_substituted() {
    let mut profile = profile();
    profile.schema_version = 2;
    for definition in &mut profile.sensors {
        definition.result_requirement = Some(policy_contracts::SensorResultRequirement::Process);
    }
    profile.sensors[0].result_binding = Some(policy_contracts::SensorResultBinding {
        configuration_digest: digest(),
        parser_version: "1".into(),
        resolved_command: vec!["sensor".into()],
    });
    let evaluate =
        |item| evaluate_sensor_evidence(&profile, &[id("compile")], &[item], Some(&adequate()));
    assert_eq!(
        evaluate(sensor("compile", SensorStatus::Passed)).status,
        SensorEvaluationStatus::Passed
    );
    for field in 0..3 {
        let mut item = sensor("compile", SensorStatus::Passed);
        match field {
            0 => item.configuration_digest = Some(Sha256Digest::parse("c".repeat(64)).unwrap()),
            1 => item.parser_version = Some("2".into()),
            _ => item.resolved_command.push("--changed".into()),
        }
        let result = evaluate(item);
        assert_eq!(result.status, SensorEvaluationStatus::Inconclusive);
        assert!(result.has_reason("PK-SENSOR-RESULT-BINDING"));
    }
    for field in 0..5 {
        let mut invalid = profile.clone();
        let binding = invalid.sensors[0].result_binding.as_mut().unwrap();
        match field {
            0 => binding.parser_version = " ".into(),
            1 => binding.parser_version = "parser\0".into(),
            2 => binding.resolved_command.clear(),
            3 => binding.resolved_command[0] = " ".into(),
            _ => binding.resolved_command.push("arg\0".into()),
        }
        assert!(resolve_required_sensors(&invalid, &[], Risk::Low).is_err());
        let result = evaluate_sensor_evidence(
            &invalid,
            &[id("compile")],
            &[sensor("compile", SensorStatus::Passed)],
            Some(&adequate()),
        );
        assert!(result.has_reason("PK-SENSOR-PROFILE-INVALID"));
    }
}

fn evaluate_compile(
    profile: &SensorProfile,
    item: SensorEvidence,
    adequacy: Option<&TestAdequacyEvidence>,
) -> policy_core::SensorEvaluation {
    evaluate_sensor_evidence(profile, &[id("compile")], &[item], adequacy)
}

#[test]
fn every_typed_result_provenance_field_is_required_independently() {
    let passing = evaluate_compile(
        &profile(),
        sensor("compile", SensorStatus::Passed),
        Some(&adequate()),
    );
    assert_eq!(passing.status, SensorEvaluationStatus::Passed);

    let damages: [fn(&mut SensorEvidence); 6] = [
        |item| item.resolved_command.clear(),
        |item| item.source_digest = None,
        |item| item.configuration_digest = None,
        |item| item.parser_version = None,
        |item| item.parser_version = Some(" ".into()),
        |item| item.exit_code = None,
    ];
    for (index, damage) in damages.into_iter().enumerate() {
        let mut item = sensor("compile", SensorStatus::Passed);
        damage(&mut item);
        let actual = evaluate_compile(&profile(), item, Some(&adequate()));
        assert_eq!(
            actual.status,
            SensorEvaluationStatus::Inconclusive,
            "{index}"
        );
        assert!(actual.has_reason("PK-SENSOR-RESULT-INCOMPLETE"), "{index}");
    }
}

#[test]
fn each_adequacy_obligation_alone_requires_adequacy_evidence() {
    let obligation = |acceptance: &[&str], behavior: &[&str], negative: &[&str]| {
        let mut profile = profile();
        profile.adequacy = TestAdequacyPolicy {
            acceptance_criterion_ids: acceptance.iter().map(|s| (*s).into()).collect(),
            changed_behavior_ids: behavior.iter().map(|s| (*s).into()).collect(),
            negative_case_ids: negative.iter().map(|s| (*s).into()).collect(),
        };
        evaluate_compile(&profile, sensor("compile", SensorStatus::Passed), None)
    };
    for actual in [
        obligation(&["AC-1"], &[], &[]),
        obligation(&[], &["BEHAVIOR-1"], &[]),
        obligation(&[], &[], &["NEG-1"]),
    ] {
        assert_eq!(actual.status, SensorEvaluationStatus::Inconclusive);
        assert!(actual.has_reason("PK-ADEQ-MISSING"));
    }
    assert_eq!(
        obligation(&[], &[], &[]).status,
        SensorEvaluationStatus::Passed
    );
}

#[test]
fn a_mapping_without_tests_does_not_satisfy_its_requirement() {
    let mut empty = adequate();
    empty.acceptance_criteria[0].test_ids.clear();
    let actual = evaluate_compile(
        &profile(),
        sensor("compile", SensorStatus::Passed),
        Some(&empty),
    );
    assert_eq!(actual.status, SensorEvaluationStatus::Blocked);
    assert!(actual.has_reason("PK-ADEQ-CRITERION-UNMAPPED"));

    let mut other = adequate();
    other.acceptance_criteria[0].requirement_id = "AC-OTHER".into();
    let actual = evaluate_compile(
        &profile(),
        sensor("compile", SensorStatus::Passed),
        Some(&other),
    );
    assert!(actual.has_reason("PK-ADEQ-CRITERION-UNMAPPED"));
}

#[test]
fn blank_and_duplicate_condition_ids_are_each_rejected() {
    let mut blank = profile();
    blank.sensors[1].required_when[0].id = "  ".into();
    assert!(resolve_required_sensors(&blank, &["src/api.rs"], Risk::High).is_err());

    let mut duplicate = profile();
    let condition = duplicate.sensors[1].required_when[0].clone();
    duplicate.sensors[1].not_applicable_when.push(condition);
    assert!(resolve_required_sensors(&duplicate, &["src/api.rs"], Risk::High).is_err());
}
