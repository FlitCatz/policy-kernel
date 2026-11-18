use policy_contracts::{Sha256Digest, TrustClass, UtcTimestamp};
use policy_native_contracts::*;
use policy_native_verifier::*;
use serde_json::{json, Value};

fn fixture() -> (ExpectedScopeV1, ImportedObservationsV1) {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../policy-native-contracts/tests/corpus/v1.json"
    ))
    .unwrap();
    let mut expected: ExpectedScopeV1 =
        serde_json::from_value(corpus["valid"]["expected"].clone()).unwrap();
    let run: NativeRunV1 = serde_json::from_value(corpus["valid"]["run"].clone()).unwrap();
    let mut bindings = expected.bindings.observations.as_slice().to_vec();
    bindings[0].command_digest =
        canonical_digest(DigestRole::Command, &run.observations[0].command).unwrap();
    expected.bindings.observations = RequiredSet::new(bindings).unwrap();
    let imported = ImportedObservationsV1 {
        schema_version: VersionV1::default(),
        expected_scope_digest: expected.digest().unwrap(),
        expected_run_id: run.subject.run_id.clone(),
        expected_scope_id: run.subject.scope_id.clone(),
        adapter_version: RecordId::parse("fixture/1").unwrap(),
        trust: LocalTrust::LocalUntrusted,
        review: ReviewGate::Unmet,
        unsuccessful_reports: vec![],
        run,
    };
    (expected, imported)
}

#[test]
fn satisfied_scope_assembles_local_evidence_without_reviews() {
    let (expected, imported) = fixture();
    let evaluation = evaluate_scope(&expected, &imported).unwrap();
    assert_eq!(evaluation.result.status, ScopeStatus::Satisfied);
    assert_eq!(evaluation.result.trust, LocalTrust::LocalUntrusted);
    assert_eq!(evaluation.result.review, ReviewGate::Unmet);
    assert!(evaluation.core_reasons.is_empty());
    let evidence = evaluation.evidence.unwrap();
    assert!(evidence.reviews.is_empty());
    assert!(evidence
        .executions
        .iter()
        .all(|e| e.trust == TrustClass::LocalUntrusted
            && e.provider.as_str() == "local-recorder"
            && e.model.as_str() == "not-a-model"));
    assert!(evidence
        .sensors
        .iter()
        .all(|s| s.trust == TrustClass::LocalUntrusted));
    assert!(policy_core::validate_bundle(&evidence).is_valid());
    assert_eq!(
        evaluation.result.digests.evidence,
        Some(canonical_digest(DigestRole::Evidence, &evidence).unwrap())
    );
}

#[test]
fn missing_and_unexecuted_mappings_never_pass() {
    let (expected, mut imported) = fixture();
    imported.run.mappings.remove(0);
    let evaluation = evaluate_scope(&expected, &imported).unwrap();
    assert_eq!(evaluation.result.status, ScopeStatus::NonPass);
    assert!(evaluation
        .core_reasons
        .iter()
        .any(|r| r.code == "PK-ADEQ-CRITERION-UNMAPPED"));
    assert_eq!(
        evaluation.result.requirements[0].status,
        ScopeStatus::NonPass
    );
    assert!(evaluation
        .result
        .reasons
        .iter()
        .any(|r| r.code == ReasonCode::UnexecutedMapping));
    let (_, mut imported) = fixture();
    imported.run.test_observations[0].tests[0].outcome = TestOutcome::NotExecuted;
    imported.run.test_observations[0].tests[0].attempts = 0;
    imported.run.test_observations[0].counts.executed = 0;
    imported.run.test_observations[0].counts.passed = 0;
    assert!(evaluate_scope(&expected, &imported).is_err());
}

#[test]
fn empty_policy_fake_trust_review_and_unknown_versions_are_rejected() {
    let (expected, imported) = fixture();
    for field in ["observations", "requirements"] {
        let mut v = serde_json::to_value(&expected).unwrap();
        v["requirements"][field] = json!([]);
        assert!(serde_json::from_value::<ExpectedScopeV1>(v).is_err());
    }
    for (field, value) in [
        ("trust", json!("ci_observed")),
        ("review", json!("met")),
        ("schema_version", json!(2)),
        ("trusted", json!(true)),
    ] {
        let mut v = serde_json::to_value(&imported).unwrap();
        v[field] = value;
        assert!(serde_json::from_value::<ImportedObservationsV1>(v).is_err());
    }
}

#[test]
fn subject_binding_time_required_set_and_exclusions_fail_closed() {
    for case in 0..10 {
        let (expected, mut imported) = fixture();
        match case {
            0 => {
                imported.run.subject.source.head =
                    policy_contracts::CommitSha::parse("2".repeat(40)).unwrap()
            }
            1 => {
                imported.run.observations[0].subject.source.head =
                    policy_contracts::CommitSha::parse("2".repeat(40)).unwrap()
            }
            2 => {
                imported.expected_scope_digest.sha256 = Sha256Digest::parse("c".repeat(64)).unwrap()
            }
            3 => imported.run.observations[0].command.cwd = "other".into(),
            4 => {
                imported.run.observations[0].configuration_digest.sha256 =
                    Sha256Digest::parse("c".repeat(64)).unwrap()
            }
            5 => {
                imported.run.test_observations[0].parser.version = RecordId::parse("other").unwrap()
            }
            6 => {
                imported.run.observations[0].started_at =
                    UtcTimestamp::parse("2020-01-01T00:00:00Z").unwrap()
            }
            7 => {
                imported.run.observations.clear();
                imported.run.test_observations.clear();
                imported.run.mappings.clear();
            }
            8 => imported
                .run
                .observations
                .push(imported.run.observations[0].clone()),
            _ => imported.expected_run_id = policy_contracts::ExecutionId::parse("other").unwrap(),
        }
        match evaluate_scope(&expected, &imported) {
            Err(_) => {}
            Ok(e) => {
                assert_eq!(e.result.status, ScopeStatus::Error, "case {case}");
                assert!(e.evidence.is_none());
            }
        }
    }
}

#[test]
fn native_bounds_policy_digest_and_incomplete_completion_are_enforced() {
    let (mut expected, mut imported) = fixture();
    let baseline = evaluate_scope(&expected, &imported).unwrap();
    let mut rules = expected.requirements.observations.as_slice().to_vec();
    rules[0].tests.as_mut().unwrap().minimum_passed = 2;
    expected.requirements.observations = RequiredSet::new(rules).unwrap();
    imported.expected_scope_digest = expected.digest().unwrap();
    let e = evaluate_scope(&expected, &imported).unwrap();
    assert_eq!(e.result.status, ScopeStatus::NonPass);
    assert_ne!(e.result.digests.policy, baseline.result.digests.policy);
    imported.run.observations[0].completion = CompletionV1::TimedOut {
        detail: RecordId::parse("timeout").unwrap(),
    };
    assert_eq!(
        evaluate_scope(&expected, &imported).unwrap().result.status,
        ScopeStatus::Error
    );
}

#[test]
fn skip_allowance_enforces_names_and_count_independently() {
    let (mut expected, mut imported) = fixture();
    let report = &mut imported.run.test_observations[0];
    let skipped_identity = RecordId::parse(r#"["test-id","Suite.SkippedCase"]"#).unwrap();
    report.tests.push(TestV1 {
        id: RecordId::parse("unit:normalized-skipped-id").unwrap(),
        raw_identity: skipped_identity.clone(),
        outcome: TestOutcome::Skipped,
        attempts: 0,
    });
    report.counts.discovered += 1;
    report.counts.skipped = 1;
    let suite_id = report.observation_id.clone();
    let artifact_id = report.raw_report.id.clone();
    let named = Some(vec![skipped_identity.clone()]);
    let other = Some(vec![RecordId::parse("Suite.OtherCase").unwrap()]);
    let mut rules = expected.requirements.observations.as_slice().to_vec();
    for (allowed, maximum, pass, unexpected) in [
        (named.clone(), 1, true, false),
        (other, 1, false, true),
        (Some(vec![]), 1, false, true),
        (named, 0, false, false),
        (None, 1, true, false),
        (None, 0, false, false),
    ] {
        let rule = rules[0].tests.as_mut().unwrap();
        rule.minimum_discovered = 2;
        rule.maximum_skipped = maximum;
        rule.allowed_skipped_tests = allowed;
        expected.requirements.observations = RequiredSet::new(rules.clone()).unwrap();
        imported.expected_scope_digest = expected.digest().unwrap();
        let evaluated = evaluate_scope(&expected, &imported).unwrap();
        assert_eq!(
            evaluated.result.status,
            if pass {
                ScopeStatus::Satisfied
            } else {
                ScopeStatus::NonPass
            }
        );
        let issue = evaluated
            .result
            .reasons
            .iter()
            .find(|reason| reason.code == ReasonCode::UnexpectedSkippedTest);
        assert_eq!(issue.is_some(), unexpected);
        if let Some(issue) = issue {
            assert_eq!(issue.location.suite_id.as_ref(), Some(&suite_id));
            assert_eq!(issue.location.artifact_id.as_ref(), Some(&artifact_id));
            assert_eq!(
                issue.location.test_identity.as_ref(),
                Some(&skipped_identity)
            );
            assert_eq!(
                evaluated.result.requirements[0].status,
                ScopeStatus::NonPass
            );
        }
    }

    // A missing execution must still be an invalid report when it isn't an explicit skip.
    let report = &mut imported.run.test_observations[0];
    report.tests[1].outcome = TestOutcome::NotExecuted;
    report.counts.skipped = 0;
    let evaluated = evaluate_scope(&expected, &imported).unwrap();
    assert_eq!(evaluated.result.status, ScopeStatus::Error);
    assert_eq!(evaluated.result.reasons[0].code, ReasonCode::InvalidReport);
}

#[test]
fn exclusions_never_waive_required_mappings_or_allow_unapproved_scope() {
    let (mut expected, mut imported) = fixture();
    let exclusion = ExclusionV1 {
        id: RecordId::parse("excluded").unwrap(),
        scope: RecordId::parse("third-party bundle").unwrap(),
        reason: RecordId::parse("not individually audited").unwrap(),
    };
    imported.run.exclusions.push(exclusion.clone());
    assert_eq!(
        evaluate_scope(&expected, &imported).unwrap().result.status,
        ScopeStatus::Error
    );
    expected.requirements.exemptions.push(exclusion);
    imported.expected_scope_digest = expected.digest().unwrap();
    assert_eq!(
        evaluate_scope(&expected, &imported).unwrap().result.status,
        ScopeStatus::Satisfied
    );
    imported.run.mappings.clear();
    assert_eq!(
        evaluate_scope(&expected, &imported).unwrap().result.status,
        ScopeStatus::NonPass
    );
    imported.run.exclusions[0].reason = RecordId::parse("different explanation").unwrap();
    assert_eq!(
        evaluate_scope(&expected, &imported).unwrap().result.status,
        ScopeStatus::Error
    );
}

#[test]
fn mapping_minimum_and_allowed_suite_are_independent_of_passing_counts() {
    let (mut expected, mut imported) = fixture();
    let mut requirements = expected.requirements.requirements.as_slice().to_vec();
    requirements[0].minimum_executed_tests = 2;
    expected.requirements.requirements = RequiredSet::new(requirements).unwrap();
    imported.expected_scope_digest = expected.digest().unwrap();
    let e = evaluate_scope(&expected, &imported).unwrap();
    assert_eq!(e.result.status, ScopeStatus::NonPass);
    assert_eq!(e.result.requirements[0].status, ScopeStatus::NonPass);
    assert!(e.core_reasons.is_empty()); // Additional native policy is stricter.

    // A second valid suite cannot cover a requirement approved only for unit.
    let (mut expected, mut imported) = fixture();
    let other = policy_contracts::SensorId::parse("other").unwrap();
    let mut rules = expected.requirements.observations.as_slice().to_vec();
    let mut rule = rules[0].clone();
    rule.id = other.clone();
    rules.push(rule);
    expected.requirements.observations = RequiredSet::new(rules).unwrap();
    let mut bindings = expected.bindings.observations.as_slice().to_vec();
    let mut binding = bindings[0].clone();
    binding.id = other.clone();
    bindings.push(binding);
    expected.bindings.observations = RequiredSet::new(bindings).unwrap();
    imported.expected_scope_digest = expected.digest().unwrap();
    let mut observation = imported.run.observations[0].clone();
    observation.id = other.clone();
    observation.execution_id = policy_contracts::ExecutionId::parse("other-execution").unwrap();
    imported.run.observations.push(observation);
    let mut report = imported.run.test_observations[0].clone();
    report.observation_id = other.clone();
    imported.run.test_observations.push(report);
    imported.run.mappings[0].tests = RequiredSet::new(vec![TestRefV1 {
        suite_id: other,
        test_id: imported.run.test_observations[0].tests[0].id.clone(),
    }])
    .unwrap();
    assert_eq!(
        evaluate_scope(&expected, &imported).unwrap().result.status,
        ScopeStatus::NonPass
    );
}

#[test]
fn start_age_window_is_inclusive_and_exact_at_both_edges() {
    let evaluated = UtcTimestamp::parse("2026-11-18T01:02:03.5Z").unwrap();
    let within = |started: &str, maximum| {
        started_within_age(&UtcTimestamp::parse(started).unwrap(), &evaluated, maximum)
    };
    assert!(within("2026-11-18T01:02:03.5Z", 0));
    assert!(!within("2026-11-18T01:02:03.499999999Z", 0));
    assert!(within("2026-11-18T01:01:03.5Z", 60));
    assert!(!within("2026-11-18T01:01:03.499999999Z", 60));
    assert!(within("2026-11-18T01:01:03.500000001Z", 60));
    assert!(within("0000-01-01T00:00:00Z", u64::MAX));
    // Only whole seconds count on the future side: a start less than one second
    // after evaluation passes here and is bounded by `observed_at` instead.
    assert!(within("2026-11-18T01:02:04.499999999Z", 0));
    assert!(!within("2026-11-18T01:02:04.5Z", 0));
    assert!(!within("2026-11-18T01:02:04.5Z", u64::MAX));
}
