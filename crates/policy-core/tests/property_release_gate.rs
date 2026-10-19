mod support;

use chrono::{Duration, SecondsFormat};
use policy_contracts::{
    FindingSeverity, PreMergeEvaluationInput, PreMergeStatus, ReviewStatus, Risk, SensorProfile,
    UtcTimestamp,
};
use policy_core::{
    evaluate_pre_merge, evaluate_reviews, resolve_required_sensors, ReviewEvaluationInput,
};
use proptest::prelude::*;
use serde_json::json;
use support::{fixture_value, pre_merge_input, ready_value, seal_evidence};

fn review_fixture(relative: &str) -> ReviewEvaluationInput {
    serde_json::from_value(fixture_value(relative))
        .expect("fixture must satisfy ReviewEvaluationInput")
}

proptest! {
    #[test]
    fn review_report_permutations_do_not_change_consensus(
        reverse_reviews in any::<bool>(),
        reverse_findings in any::<bool>(),
    ) {
        let mut input = review_fixture("tests/fixtures/reviews/medium-confirmed.json");
        let expected = evaluate_reviews(input.clone());
        if reverse_reviews {
            input.reviews.reverse();
        }
        if reverse_findings {
            input.findings.reverse();
        }
        prop_assert_eq!(evaluate_reviews(input), expected);
    }

    #[test]
    fn evidence_record_order_does_not_change_the_pre_merge_decision(
        reverse_modifications in any::<bool>(),
        reverse_executions in any::<bool>(),
        reverse_reviews in any::<bool>(),
        reverse_sensors in any::<bool>(),
        reverse_manifest in any::<bool>(),
    ) {
        let mut value = ready_value();
        let expected = evaluate_pre_merge(&pre_merge_input(value.clone()));
        for (field, reverse) in [
            ("modifications", reverse_modifications),
            ("executions", reverse_executions),
            ("reviews", reverse_reviews),
            ("sensors", reverse_sensors),
        ] {
            if reverse {
                value["evidence"][field].as_array_mut().unwrap().reverse();
            }
        }
        seal_evidence(&mut value);
        if reverse_manifest {
            value["evidence"]["manifest"]["child_digests"]
                .as_array_mut().unwrap().reverse();
        }
        let reordered = evaluate_pre_merge(&pre_merge_input(value));
        prop_assert_eq!(reordered.status, expected.status);
        prop_assert_eq!(reordered.reasons, expected.reasons);
    }

    #[test]
    fn malformed_digest_head_and_time_never_enter_the_typed_evaluator(tag in 0_u32..10_000) {
        for (pointer, malformed) in [
            ("/policy/policy_digest", format!("bad-digest-{tag}")),
            ("/pull_request/head_sha", format!("bad-head-{tag}")),
            ("/evaluation_time", format!("bad-time-{tag}")),
        ] {
            let mut value = ready_value();
            *value.pointer_mut(pointer).unwrap() = json!(malformed);
            prop_assert!(serde_json::from_value::<PreMergeEvaluationInput>(value).is_err(), "{pointer}");
        }
    }

    #[test]
    fn well_formed_digest_and_head_changes_are_detected(character in prop::sample::select(vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'd', 'e', 'f'])) {
        let mut digest = ready_value();
        digest["evidence"]["manifest"]["child_digests"][0]["digest"] =
            json!(character.to_string().repeat(64));
        let decision = evaluate_pre_merge(&pre_merge_input(digest));
        prop_assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        prop_assert!(decision.reasons.iter().any(|reason| reason.code == "PK-PROV-DIGEST-MISMATCH"));

        let mut head = ready_value();
        head["context"]["agent_ready"]["head_sha"] = json!(character.to_string().repeat(40));
        let decision = evaluate_pre_merge(&pre_merge_input(head));
        prop_assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        prop_assert!(decision.reasons.iter().any(|reason| reason.code == "PK-PRE-AGENT-READY-BINDING"));
    }

    #[test]
    fn adding_an_unwaived_high_or_critical_finding_is_monotonically_blocking(
        severity in prop_oneof![Just(FindingSeverity::High), Just(FindingSeverity::Critical)]
    ) {
        let mut input = review_fixture("tests/fixtures/reviews/critical.json");
        input.findings[0].severity = FindingSeverity::Low;
        input.reviews[0].status = ReviewStatus::Approved;
        prop_assert_eq!(evaluate_reviews(input.clone()).status, ReviewStatus::Approved);

        input.findings[0].severity = severity;
        let elevated = evaluate_reviews(input);
        prop_assert_eq!(elevated.status, ReviewStatus::Blocked);
        prop_assert!(elevated.reasons.iter().any(|reason| reason.code == "PK-REVIEW-HIGH"));
    }

    #[test]
    fn waiver_expiry_uses_the_injected_evaluation_time(offset_seconds in -300_i64..=300_i64) {
        let mut input = review_fixture("tests/fixtures/reviews/waiver-expired.json");
        let expiry = *input.evaluation_time.as_datetime() + Duration::seconds(offset_seconds);
        input.waivers[0].expires_at = UtcTimestamp::parse(
            &expiry.to_rfc3339_opts(SecondsFormat::Nanos, true)
        ).unwrap();
        let actual = evaluate_reviews(input);
        if offset_seconds > 0 {
            prop_assert_eq!(actual.status, ReviewStatus::Inconclusive);
            prop_assert!(actual.has_reason("PK-REVIEW-QUORUM"));
            prop_assert!(actual.has_reason("PK-WAIVER-APPLIED"));
        } else {
            prop_assert_eq!(actual.status, ReviewStatus::Blocked);
            prop_assert!(actual.reasons.iter().any(|reason| reason.code == "PK-WAIVER-EXPIRED"));
        }
    }

    #[test]
    fn waiver_is_invalidated_by_any_evaluated_head_change(character in prop::sample::select(vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'd', 'e', 'f'])) {
        let mut input = review_fixture("tests/fixtures/reviews/waiver-expired.json");
        input.waivers[0].expires_at = UtcTimestamp::parse("2026-09-04T10:00:00Z").unwrap();
        input.head_sha = policy_contracts::CommitSha::parse(character.to_string().repeat(40)).unwrap();
        let actual = evaluate_reviews(input);
        prop_assert_eq!(actual.status, ReviewStatus::Blocked);
        prop_assert!(actual.reasons.iter().any(|reason| reason.code == "PK-WAIVER-INVALID"));
    }

    #[test]
    fn sensor_selection_is_deterministic_across_path_order_and_separators(
        paths in prop::collection::vec(
            prop::sample::select(vec![
                "src/api.rs".to_owned(),
                "src/api/handler.rs".to_owned(),
                "docs/guide.md".to_owned(),
                "src/lib.rs".to_owned(),
            ]),
            0..12,
        ),
        risk in prop_oneof![Just(Risk::Low), Just(Risk::Medium), Just(Risk::High), Just(Risk::Critical)],
    ) {
        let profile: SensorProfile = serde_json::from_value(fixture_value("tests/fixtures/sensors/profile.json")).unwrap();
        let original_refs = paths.iter().map(String::as_str).collect::<Vec<_>>();
        let original = resolve_required_sensors(&profile, &original_refs, risk).unwrap();

        let mut reordered = paths;
        reordered.reverse();
        let windows = reordered.into_iter().map(|path| path.replace('/', "\\\\")).collect::<Vec<_>>();
        let windows_refs = windows.iter().map(String::as_str).collect::<Vec<_>>();
        let normalized = resolve_required_sensors(&profile, &windows_refs, risk).unwrap();
        prop_assert_eq!(normalized, original);
    }
}
