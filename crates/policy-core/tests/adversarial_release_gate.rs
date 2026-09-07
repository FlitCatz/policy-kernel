mod support;

use policy_contracts::{PostMergeEvaluationInput, PostMergeStatus, PreMergeStatus};
use policy_core::{evaluate_post_merge, evaluate_pre_merge, pre_merge_evidence_digest};
use serde_json::{json, Value};
use support::{fixture_value, pre_merge_input, ready_value, seal_evidence};

fn assert_pre_merge_reason(value: Value, status: PreMergeStatus, reason: &str) {
    let decision = evaluate_pre_merge(&pre_merge_input(value));
    assert_eq!(decision.status, status, "{reason}: {:?}", decision.reasons);
    assert!(
        decision.reasons.iter().any(|item| item.code == reason),
        "missing {reason}: {:?}",
        decision.reasons
    );
}

fn post_merge(relative: &str) -> PostMergeEvaluationInput {
    serde_json::from_value(fixture_value(relative))
        .expect("fixture must satisfy PostMergeEvaluationInput")
}

fn assert_post_merge_reason(value: Value, status: PostMergeStatus, reason: &str) {
    let input: PostMergeEvaluationInput =
        serde_json::from_value(value).expect("fixture must satisfy PostMergeEvaluationInput");
    let decision = evaluate_post_merge(&input);
    assert_eq!(decision.status, status, "{reason}: {:?}", decision.reasons);
    assert!(
        decision.reasons.iter().any(|item| item.code == reason),
        "missing {reason}: {:?}",
        decision.reasons
    );
}

#[test]
fn altered_evidence_and_replayed_agent_ready_fail_closed() {
    let mut digest_tampering = ready_value();
    digest_tampering["evidence"]["manifest"]["child_digests"][0]["digest"] = json!("f".repeat(64));
    assert_pre_merge_reason(
        digest_tampering,
        PreMergeStatus::Inconclusive,
        "PK-PROV-DIGEST-MISMATCH",
    );

    let mut replay = ready_value();
    replay["context"]["agent_ready"]["head_sha"] = json!("f".repeat(40));
    assert_pre_merge_reason(
        replay,
        PreMergeStatus::Inconclusive,
        "PK-PRE-AGENT-READY-BINDING",
    );
}

#[test]
fn missing_or_non_independent_review_evidence_cannot_authorize() {
    let mut missing = ready_value();
    missing["evidence"]["reviews"] = json!([]);
    seal_evidence(&mut missing);
    assert_pre_merge_reason(missing, PreMergeStatus::Inconclusive, "PK-REVIEW-MISSING");

    let mut overlap = ready_value();
    overlap["evidence"]["reviews"][0]["provider"] = json!("producer-a");
    seal_evidence(&mut overlap);
    assert_pre_merge_reason(overlap, PreMergeStatus::Inconclusive, "PK-SELF-PROVIDER");

    let mut duplicate_execution = ready_value();
    duplicate_execution["evidence"]["reviews"][0]["execution_id"] = json!("modify-a");
    seal_evidence(&mut duplicate_execution);
    assert_pre_merge_reason(
        duplicate_execution,
        PreMergeStatus::Inconclusive,
        "PK-PROV-DUPLICATE-EXECUTION",
    );
}

#[test]
fn expired_waiver_sensor_timeout_and_empty_discovery_remain_non_success() {
    let mut expired = ready_value();
    expired["evidence"]["findings"] = json!([{
        "id": "F-EXPIRED",
        "severity": "high",
        "summary": "authorization defect",
        "reviewer_execution_id": "review-b"
    }]);
    seal_evidence(&mut expired);
    let evidence_digest = pre_merge_evidence_digest(&pre_merge_input(expired.clone()).evidence)
        .expect("typed evidence has a canonical digest");
    expired["evidence"]["waivers"] = json!([{
        "actor": "release-owner",
        "finding_id": "F-EXPIRED",
        "repository": "FlitCatz/policy-kernel",
        "head_sha": "c".repeat(40),
        "evidence_digest": evidence_digest,
        "reason": "accepted only before evaluation",
        "issued_at": "2026-09-07T08:30:00Z",
        "expires_at": "2026-09-07T09:00:00Z"
    }]);
    seal_evidence(&mut expired);
    assert_pre_merge_reason(expired, PreMergeStatus::Blocked, "PK-WAIVER-EXPIRED");

    let mut timeout = ready_value();
    timeout["evidence"]["sensors"][0]["status"] = json!("timed_out");
    seal_evidence(&mut timeout);
    assert_pre_merge_reason(timeout, PreMergeStatus::Inconclusive, "PK-SENSOR-TIMEOUT");

    let mut no_tests = ready_value();
    no_tests["evidence"]["test_adequacy"]["discovered"] = json!(0);
    no_tests["evidence"]["test_adequacy"]["executed"] = json!(0);
    no_tests["evidence"]["test_adequacy"]["passed"] = json!(0);
    seal_evidence(&mut no_tests);
    assert_pre_merge_reason(no_tests, PreMergeStatus::Blocked, "PK-ADEQ-NO-TESTS");
}

#[test]
fn stale_supplied_knowledge_and_local_authoritative_evidence_fail_closed() {
    let mut stale_codegraph = ready_value();
    stale_codegraph["evidence"]["knowledge"] = json!([{
        "kind": "code_graph_index",
        "source": "project-index",
        "version": "1.0.0",
        "configuration_digest": "1".repeat(64),
        "content_digest": "2".repeat(64),
        "source_set_digest": "e".repeat(64),
        "head_sha": "f".repeat(40),
        "trust": "trusted_runner_attested"
    }]);
    seal_evidence(&mut stale_codegraph);
    assert_pre_merge_reason(
        stale_codegraph,
        PreMergeStatus::Inconclusive,
        "PK-PROV-BINDING-MISMATCH",
    );

    let mut local = ready_value();
    local["evidence"]["executions"][0]["trust"] = json!("local_untrusted");
    seal_evidence(&mut local);
    assert_pre_merge_reason(local, PreMergeStatus::Inconclusive, "PK-PROV-UNTRUSTED");
}

#[test]
fn synthetic_or_misbinding_required_checks_cannot_authorize_shipped() {
    assert_post_merge_reason(
        fixture_value("tests/fixtures/post-merge/missing-check.json"),
        PostMergeStatus::Inconclusive,
        "PK-POST-CHECK-MISSING",
    );
    assert_post_merge_reason(
        fixture_value("tests/fixtures/post-merge/wrong-head.json"),
        PostMergeStatus::Inconclusive,
        "PK-POST-HEAD-MISMATCH",
    );
    assert_post_merge_reason(
        fixture_value("tests/fixtures/post-merge/late-check.json"),
        PostMergeStatus::Inconclusive,
        "PK-POST-CHECK-AFTER-MERGE",
    );
}

#[test]
fn unprotected_direct_merge_and_unsupported_queue_are_blocked() {
    let mut direct = fixture_value("tests/fixtures/post-merge/merge.json");
    direct["merge"]["protected"] = json!(false);
    assert_post_merge_reason(
        direct,
        PostMergeStatus::Blocked,
        "PK-POST-TARGET-UNPROTECTED",
    );

    let mut queued = fixture_value("tests/fixtures/post-merge/merge.json");
    queued["merge"]["via_merge_queue"] = json!(true);
    assert_post_merge_reason(
        queued,
        PostMergeStatus::Blocked,
        "PK-POST-MERGE-QUEUE-UNSUPPORTED",
    );
}

#[test]
fn merge_and_squash_commit_oids_may_differ_from_the_evaluated_pr_head() {
    for relative in [
        "tests/fixtures/post-merge/merge.json",
        "tests/fixtures/post-merge/squash.json",
    ] {
        let input = post_merge(relative);
        assert_ne!(
            input.merge.merge_commit_sha,
            input.pre_merge.as_decision().binding.head_sha,
            "{relative} must exercise distinct merge and PR-head OIDs"
        );
        let decision = evaluate_post_merge(&input);
        assert_eq!(decision.status, PostMergeStatus::Shipped, "{relative}");
        assert!(decision.reasons.is_empty(), "{relative}");
    }
}
