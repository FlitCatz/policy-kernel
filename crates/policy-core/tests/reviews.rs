use policy_contracts::{EvaluationMode, FindingSeverity, ReviewStatus, UtcTimestamp};
use policy_core::{evaluate_reviews, ReviewEvaluationInput};

fn load(name: &str) -> ReviewEvaluationInput {
    let raw = match name {
        "approved.json" => include_str!("../../../tests/fixtures/reviews/approved.json"),
        "self-review.json" => include_str!("../../../tests/fixtures/reviews/self-review.json"),
        "critical.json" => include_str!("../../../tests/fixtures/reviews/critical.json"),
        "medium-confirmed.json" => {
            include_str!("../../../tests/fixtures/reviews/medium-confirmed.json")
        }
        "medium-conflict.json" => {
            include_str!("../../../tests/fixtures/reviews/medium-conflict.json")
        }
        "waiver-expired.json" => {
            include_str!("../../../tests/fixtures/reviews/waiver-expired.json")
        }
        _ => panic!("unknown fixture {name}"),
    };
    serde_json::from_str(raw).expect("fixture must satisfy the strict review input contract")
}

#[test]
fn review_policy_decision_table_is_fail_closed() {
    for (name, expected, reason) in [
        ("critical.json", ReviewStatus::Blocked, "PK-REVIEW-HIGH"),
        (
            "medium-confirmed.json",
            ReviewStatus::Blocked,
            "PK-REVIEW-MEDIUM-CONFIRMED",
        ),
        (
            "medium-conflict.json",
            ReviewStatus::Inconclusive,
            "PK-REVIEW-MEDIUM-CONFLICT",
        ),
        (
            "self-review.json",
            ReviewStatus::Inconclusive,
            "PK-SELF-PROVIDER",
        ),
    ] {
        let actual = evaluate_reviews(load(name));
        assert_eq!(actual.status, expected, "fixture {name}");
        assert!(actual.has_reason(reason), "fixture {name}: {reason}");
    }
}

#[test]
fn independent_approval_without_findings_is_approved() {
    let actual = evaluate_reviews(load("approved.json"));
    assert_eq!(actual.status, ReviewStatus::Approved);
    assert!(actual.reasons.is_empty());
}

#[test]
fn original_producer_cannot_review_after_an_independent_one_line_repair() {
    let mut input = load("self-review.json");
    input
        .modifications
        .push(policy_contracts::ModificationEvidence {
            provider: policy_contracts::ProviderId::parse("repair-provider").unwrap(),
            model: policy_contracts::ModelId::parse("repair-model").unwrap(),
            execution_id: policy_contracts::ExecutionId::parse("repair-execution").unwrap(),
            head_sha: input.head_sha.clone(),
            observed_at: UtcTimestamp::parse("2026-09-04T08:57:00Z").unwrap(),
            trust: policy_contracts::TrustClass::TrustedRunnerAttested,
        });

    let actual = evaluate_reviews(input);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-SELF-PROVIDER"));
}

#[test]
fn missing_duplicate_and_invalid_review_provenance_are_inconclusive() {
    let mut missing = load("approved.json");
    missing.reviews.clear();
    assert!(evaluate_reviews(missing).has_reason("PK-REVIEW-MISSING"));

    let mut duplicate_provider = load("approved.json");
    let mut duplicate = duplicate_provider.reviews[0].clone();
    duplicate.execution_id = policy_contracts::ExecutionId::parse("review-b-2").unwrap();
    duplicate_provider.reviews.push(duplicate);
    assert!(evaluate_reviews(duplicate_provider).has_reason("PK-PROV-DUPLICATE-PROVIDER"));

    let mut duplicate_execution = load("approved.json");
    let mut duplicate = duplicate_execution.reviews[0].clone();
    duplicate.provider = policy_contracts::ProviderId::parse("reviewer-c").unwrap();
    duplicate_execution.reviews.push(duplicate);
    assert!(evaluate_reviews(duplicate_execution).has_reason("PK-PROV-DUPLICATE-EXECUTION"));

    let mut wrong_head = load("approved.json");
    wrong_head.reviews[0].head_sha =
        policy_contracts::CommitSha::parse("ffffffffffffffffffffffffffffffffffffffff").unwrap();
    assert!(evaluate_reviews(wrong_head).has_reason("PK-PROV-BINDING-MISMATCH"));
}

#[test]
fn unconfirmed_medium_is_inconclusive_and_low_or_info_is_recorded() {
    let mut input = load("medium-confirmed.json");
    input.reviews.pop();
    input.findings.pop();
    let medium = evaluate_reviews(input);
    assert_eq!(medium.status, ReviewStatus::Inconclusive);
    assert!(medium.has_reason("PK-REVIEW-MEDIUM-UNCONFIRMED"));

    for severity in [FindingSeverity::Low, FindingSeverity::Info] {
        let mut input = load("critical.json");
        input.findings[0].severity = severity;
        input.reviews[0].status = ReviewStatus::Approved;
        let actual = evaluate_reviews(input);
        assert_eq!(actual.status, ReviewStatus::Approved);
    }
}

#[test]
fn waiver_must_match_every_binding_and_injected_time() {
    let expired = evaluate_reviews(load("waiver-expired.json"));
    assert_eq!(expired.status, ReviewStatus::Blocked);
    assert!(expired.has_reason("PK-WAIVER-EXPIRED"));

    let mut valid = load("waiver-expired.json");
    valid.waivers[0].expires_at = UtcTimestamp::parse("2026-09-04T10:00:00Z").unwrap();
    let accepted = evaluate_reviews(valid.clone());
    assert_eq!(accepted.status, ReviewStatus::Approved);
    assert!(accepted.has_reason("PK-WAIVER-APPLIED"));

    valid.waivers[0].evidence_digest = policy_contracts::Sha256Digest::parse(
        "9999999999999999999999999999999999999999999999999999999999999999",
    )
    .unwrap();
    let mismatched = evaluate_reviews(valid);
    assert_eq!(mismatched.status, ReviewStatus::Blocked);
    assert!(mismatched.has_reason("PK-WAIVER-INVALID"));
}

#[test]
fn every_waiver_field_is_validated_and_orphan_waivers_fail_closed() {
    let mut valid = load("waiver-expired.json");
    valid.waivers[0].expires_at = UtcTimestamp::parse("2026-09-04T10:00:00Z").unwrap();

    let mut invalid_inputs = Vec::new();

    let mut input = valid.clone();
    input.waivers[0].actor = "unauthorized-actor".into();
    invalid_inputs.push(input);

    let mut input = valid.clone();
    input.waivers[0].finding_id = "F-OTHER".into();
    invalid_inputs.push(input);

    let mut input = valid.clone();
    input.waivers[0].repository =
        policy_contracts::RepositoryId::parse("other/repository").unwrap();
    invalid_inputs.push(input);

    let mut input = valid.clone();
    input.waivers[0].head_sha =
        policy_contracts::CommitSha::parse("ffffffffffffffffffffffffffffffffffffffff").unwrap();
    invalid_inputs.push(input);

    let mut input = valid.clone();
    input.waivers[0].reason = "  ".into();
    invalid_inputs.push(input);

    let mut input = valid.clone();
    input.waivers[0].issued_at = UtcTimestamp::parse("2026-09-04T09:01:00Z").unwrap();
    invalid_inputs.push(input);

    for input in invalid_inputs {
        let actual = evaluate_reviews(input);
        assert_eq!(actual.status, ReviewStatus::Blocked);
        assert!(actual.has_reason("PK-WAIVER-INVALID"));
    }

    let mut orphan = load("approved.json");
    orphan.waivers = valid.waivers;
    let actual = evaluate_reviews(orphan);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-WAIVER-INVALID"));
}

#[test]
fn adding_an_approval_never_weakens_a_waived_blocked_report() {
    let mut waived = load("waiver-expired.json");
    waived.waivers[0].expires_at = UtcTimestamp::parse("2026-09-04T10:00:00Z").unwrap();
    assert_eq!(
        evaluate_reviews(waived.clone()).status,
        ReviewStatus::Approved
    );

    let mut approval = waived.reviews[0].clone();
    approval.provider = policy_contracts::ProviderId::parse("reviewer-c").unwrap();
    approval.execution_id = policy_contracts::ExecutionId::parse("review-c").unwrap();
    approval.status = ReviewStatus::Approved;
    let mut with_approval = waived.clone();
    with_approval.reviews.push(approval.clone());
    let actual = evaluate_reviews(with_approval.clone());
    assert_eq!(actual.status, ReviewStatus::Approved);
    assert!(!actual.has_reason("PK-REVIEW-CONFLICT"));

    // An unwaived, expired or partially waived blocked report still conflicts.
    let mut expired = with_approval.clone();
    expired.waivers[0].expires_at = expired.evaluation_time.clone();
    let actual = evaluate_reviews(expired);
    assert_eq!(actual.status, ReviewStatus::Blocked);
    assert!(actual.has_reason("PK-REVIEW-CONFLICT"));

    let mut partial = with_approval;
    partial.findings.push(policy_contracts::Finding {
        id: "F-STABLE-2".into(),
        severity: FindingSeverity::Medium,
        summary: "unwaived medium issue".into(),
        reviewer_execution_id: policy_contracts::ExecutionId::parse("review-b").unwrap(),
    });
    let actual = evaluate_reviews(partial);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-REVIEW-CONFLICT"));

    // Without any approval there is nothing to contradict.
    let lone_blocked = evaluate_reviews(load("critical.json"));
    assert_eq!(lone_blocked.status, ReviewStatus::Blocked);
    assert!(!lone_blocked.has_reason("PK-REVIEW-CONFLICT"));

    // Waived low-severity findings do not make a blocked report actionable.
    let mut low_only = waived;
    low_only.findings[0].severity = FindingSeverity::Low;
    low_only.reviews.push(approval);
    let actual = evaluate_reviews(low_only);
    assert!(actual.has_reason("PK-REVIEW-CONFLICT"));
    assert!(actual.has_reason("PK-REVIEW-BLOCKED-UNRESOLVED"));
}

#[test]
fn contradictory_review_reports_are_inconclusive_without_findings() {
    let mut input = load("approved.json");
    let mut conflicting = input.reviews[0].clone();
    conflicting.provider = policy_contracts::ProviderId::parse("reviewer-c").unwrap();
    conflicting.execution_id = policy_contracts::ExecutionId::parse("review-c").unwrap();
    conflicting.status = ReviewStatus::Blocked;
    input.reviews.push(conflicting);

    let actual = evaluate_reviews(input);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-REVIEW-CONFLICT"));
}

#[test]
fn blocked_report_requires_actionable_findings_or_valid_waivers() {
    let mut no_findings = load("approved.json");
    no_findings.reviews[0].status = ReviewStatus::Blocked;
    let actual = evaluate_reviews(no_findings);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-REVIEW-BLOCKED-UNRESOLVED"));

    let mut low_only = load("critical.json");
    low_only.findings[0].severity = FindingSeverity::Low;
    let actual = evaluate_reviews(low_only);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-REVIEW-BLOCKED-UNRESOLVED"));
}

#[test]
fn critical_blocking_is_monotonic_under_conflict_and_invalid_provenance() {
    let mut conflict = load("critical.json");
    let mut second_review = conflict.reviews[0].clone();
    second_review.provider = policy_contracts::ProviderId::parse("reviewer-c").unwrap();
    second_review.execution_id = policy_contracts::ExecutionId::parse("review-c").unwrap();
    conflict.reviews.push(second_review);
    let mut lower = conflict.findings[0].clone();
    lower.severity = FindingSeverity::Low;
    lower.reviewer_execution_id = policy_contracts::ExecutionId::parse("review-c").unwrap();
    conflict.findings.push(lower);
    let actual = evaluate_reviews(conflict);
    assert_eq!(actual.status, ReviewStatus::Blocked);
    assert!(actual.has_reason("PK-REVIEW-HIGH"));
    assert!(actual.has_reason("PK-REVIEW-MEDIUM-CONFLICT"));

    let mut invalid_reference = load("critical.json");
    invalid_reference.findings[0].reviewer_execution_id =
        policy_contracts::ExecutionId::parse("unknown-review").unwrap();
    let actual = evaluate_reviews(invalid_reference);
    assert_eq!(actual.status, ReviewStatus::Blocked);
    assert!(actual.has_reason("PK-REVIEW-HIGH"));
    assert!(actual.has_reason("PK-PROV-REVIEW-FINDING"));
}

#[test]
fn local_reviews_are_allowed_only_in_advisory_mode() {
    let mut advisory = load("approved.json");
    advisory.mode = EvaluationMode::Advisory;
    advisory.modifications[0].trust = policy_contracts::TrustClass::LocalUntrusted;
    advisory.reviews[0].trust = policy_contracts::TrustClass::LocalUntrusted;
    assert_eq!(evaluate_reviews(advisory).status, ReviewStatus::Approved);

    let mut authoritative = load("approved.json");
    authoritative.reviews[0].trust = policy_contracts::TrustClass::LocalUntrusted;
    let actual = evaluate_reviews(authoritative);
    assert_eq!(actual.status, ReviewStatus::Inconclusive);
    assert!(actual.has_reason("PK-PROV-UNTRUSTED"));
}

#[test]
fn waiver_actor_and_issuance_window_are_fail_closed() {
    let mut valid = load("waiver-expired.json");
    valid.waivers[0].expires_at = UtcTimestamp::parse("2026-09-04T10:00:00Z").unwrap();

    let mut blank_actor = valid.clone();
    blank_actor.policy.authorized_waiver_actors = vec!["  ".into()];
    blank_actor.waivers[0].actor = "  ".into();
    let actual = evaluate_reviews(blank_actor);
    assert_eq!(actual.status, ReviewStatus::Blocked);
    assert!(actual.has_reason("PK-WAIVER-INVALID"));

    let mut stale = valid;
    stale.waivers[0].issued_at = UtcTimestamp::parse("2026-09-04T07:59:59Z").unwrap();
    let actual = evaluate_reviews(stale);
    assert_eq!(actual.status, ReviewStatus::Blocked);
    assert!(actual.has_reason("PK-WAIVER-INVALID"));
}

#[test]
fn review_freshness_and_expiry_boundaries_use_injected_time() {
    for timestamp in ["2026-09-04T07:59:59Z", "2026-09-04T09:00:00.001Z"] {
        let mut input = load("approved.json");
        input.reviews[0].observed_at = UtcTimestamp::parse(timestamp).unwrap();
        assert_eq!(evaluate_reviews(input).status, ReviewStatus::Inconclusive);
    }
    let mut input = load("waiver-expired.json");
    input.waivers[0].expires_at = input.evaluation_time.clone();
    assert_eq!(evaluate_reviews(input).status, ReviewStatus::Blocked);
}

#[test]
fn review_order_does_not_change_decision() {
    let input = load("medium-confirmed.json");
    let expected = evaluate_reviews(input.clone());
    let mut reversed = input;
    reversed.reviews.reverse();
    reversed.findings.reverse();
    assert_eq!(evaluate_reviews(reversed), expected);
}

#[test]
fn ambiguous_execution_cannot_confirm_medium_in_any_order() {
    let mut input = load("medium-confirmed.json");
    let mut duplicate = input.reviews[0].clone();
    duplicate.provider = input.reviews[1].provider.clone();
    input.reviews.push(duplicate);
    let first = evaluate_reviews(input.clone());
    input.reviews.reverse();
    let second = evaluate_reviews(input);
    assert_eq!(first, second);
    assert_eq!(first.status, ReviewStatus::Inconclusive);
}
