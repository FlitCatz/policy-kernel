use policy_contracts::{ReviewStatus, TrustClass};
use policy_core::{evaluate_reviews, ReviewEvaluationInput};
use serde_json::{json, Value};
mod support;

fn owner_input() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/reviews/approved.json"
    ))
    .unwrap();
    value["policy"]["schema_version"] = json!(4);
    value["policy"]["required_reviews"]["minimum_reviewers"] = json!(2);
    value["policy"]["owner_with_independent_agent_review"] = json!({
        "owner_actor": "repository-owner", "maximum_age_seconds": 604800
    });
    value["modifications"][0]["actor"] = json!("repository-owner");
    value["reviews"][0]["identity"] = json!({
        "actor": "independent-agent", "kind": "automated",
        "pull_request_author": "repository-owner"
    });
    value["reviews"][0]["owner_approval"] = json!({
        "actor": "repository-owner", "kind": "comment",
        "repository": value["repository"], "head_sha": value["head_sha"],
        "record_digest": "a".repeat(64),
        "observed_at": "2026-09-04T08:59:00Z", "trust": "ci_observed"
    });
    value
}

fn evaluate(value: Value) -> policy_core::ReviewEvaluation {
    let parsed = serde_json::from_value::<ReviewEvaluationInput>(value);
    assert!(
        parsed.is_ok(),
        "new v4 review evidence must parse: {parsed:?}"
    );
    evaluate_reviews(parsed.unwrap())
}

#[test]
fn owner_comment_or_approval_and_one_independent_agent_satisfy_opt_in() {
    for kind in ["comment", "approval"] {
        let mut value = owner_input();
        value["reviews"][0]["owner_approval"]["kind"] = json!(kind);
        let result = evaluate(value);
        assert_eq!(result.status, ReviewStatus::Approved, "{result:?}");
        assert!(result.reasons.is_empty());
    }
}

#[test]
fn missing_owner_and_agent_evidence_have_distinct_reasons() {
    let mut missing = owner_input();
    missing["reviews"][0]
        .as_object_mut()
        .unwrap()
        .remove("owner_approval");
    let result = evaluate(missing);
    assert_eq!(result.status, ReviewStatus::Inconclusive);
    assert!(result.has_reason("PK-REVIEW-OWNER-MISSING"));

    let mut missing = owner_input();
    missing["reviews"] = json!([]);
    let result = evaluate(missing);
    assert_eq!(result.status, ReviewStatus::Inconclusive);
    assert!(result.has_reason("PK-REVIEW-AGENT-MISSING"));
}

#[test]
fn owner_rule_fails_closed_for_each_evidence_boundary() {
    // The task's contract is the oracle: none of these inputs proves both
    // owner authorization and an independent automated review of this head.
    for (pointer, replacement, reason) in [
        (
            "/reviews/0/owner_approval/actor",
            json!("another-owner"),
            "PK-REVIEW-OWNER-IDENTITY",
        ),
        (
            "/reviews/0/owner_approval/repository",
            json!("other/repository"),
            "PK-REVIEW-OWNER-BINDING",
        ),
        (
            "/reviews/0/owner_approval/head_sha",
            json!("f".repeat(40)),
            "PK-REVIEW-OWNER-BINDING",
        ),
        (
            "/reviews/0/head_sha",
            json!("f".repeat(40)),
            "PK-REVIEW-AGENT-BINDING",
        ),
        (
            "/reviews/0/owner_approval/observed_at",
            json!("2026-08-28T08:59:59Z"),
            "PK-REVIEW-OWNER-STALE",
        ),
        (
            "/reviews/0/observed_at",
            json!("2026-08-28T08:59:59Z"),
            "PK-REVIEW-AGENT-STALE",
        ),
        (
            "/reviews/0/owner_approval/observed_at",
            json!("2026-09-04T09:00:00.001Z"),
            "PK-REVIEW-OWNER-STALE",
        ),
        (
            "/reviews/0/observed_at",
            json!("2026-09-04T09:00:00.001Z"),
            "PK-REVIEW-AGENT-STALE",
        ),
        (
            "/reviews/0/identity/actor",
            json!("repository-owner"),
            "PK-REVIEW-AGENT-SELF-AUTHOR",
        ),
        (
            "/reviews/0/identity/actor",
            json!(" \t"),
            "PK-REVIEW-AGENT-IDENTITY-MISSING",
        ),
        (
            "/reviews/0/identity/actor",
            json!("agent\0suffix"),
            "PK-REVIEW-AGENT-IDENTITY-MISSING",
        ),
        (
            "/reviews/0/identity/kind",
            json!("human"),
            "PK-REVIEW-AGENT-NOT-AUTOMATED",
        ),
        (
            "/reviews/0/provider",
            json!("producer-a"),
            "PK-REVIEW-AGENT-NOT-INDEPENDENT",
        ),
        (
            "/reviews/0/execution_id",
            json!("modify-a"),
            "PK-REVIEW-AGENT-NOT-INDEPENDENT",
        ),
        (
            "/reviews/0/identity",
            Value::Null,
            "PK-REVIEW-AUTHOR-MISSING",
        ),
        (
            "/modifications/0/actor",
            Value::Null,
            "PK-REVIEW-MODIFIER-IDENTITY-MISSING",
        ),
    ] {
        let mut value = owner_input();
        *value.pointer_mut(pointer).unwrap() = replacement;
        let result = evaluate(value);
        assert_eq!(
            result.status,
            ReviewStatus::Inconclusive,
            "{pointer}: {result:?}"
        );
        assert!(result.has_reason(reason), "{pointer}: {result:?}");
    }
}

#[test]
fn freshness_uses_the_stricter_window_with_exact_fractional_boundaries() {
    for (global, owner, timestamp, approved) in [
        (604800, 604800, "2026-08-28T09:00:00Z", true),
        (604800, 604800, "2026-08-28T08:59:59.999Z", false),
        (604800, 60, "2026-09-04T08:59:00Z", true),
        (604800, 60, "2026-09-04T08:58:59.999Z", false),
        (60, 604800, "2026-09-04T08:59:00Z", true),
        (60, 604800, "2026-09-04T08:58:59.999Z", false),
        (604800, u64::MAX, "2026-08-28T09:00:00Z", true),
    ] {
        for field in ["observed_at", "owner_approval"] {
            let mut value = owner_input();
            value["policy"]["max_evidence_age_seconds"] = json!(global);
            value["policy"]["owner_with_independent_agent_review"]["maximum_age_seconds"] =
                json!(owner);
            value["modifications"][0]["observed_at"] = json!("2026-09-04T09:00:00Z");
            value["reviews"][0]["observed_at"] = json!("2026-09-04T09:00:00Z");
            value["reviews"][0]["owner_approval"]["observed_at"] = json!("2026-09-04T09:00:00Z");
            if field == "observed_at" {
                value["reviews"][0][field] = json!(timestamp);
            } else {
                value["reviews"][0][field]["observed_at"] = json!(timestamp);
            }
            let result = evaluate(value);
            assert_eq!(
                result.status == ReviewStatus::Approved,
                approved,
                "{global}/{owner}/{field}/{timestamp}: {result:?}"
            );
        }
    }
}

#[test]
fn other_authors_keep_quorum_and_conflicting_author_records_fail_closed() {
    let mut value = owner_input();
    value["reviews"][0]["identity"]["pull_request_author"] = json!("another-author");
    assert!(evaluate(value.clone()).has_reason("PK-REVIEW-QUORUM"));
    let mut second = value["reviews"][0].clone();
    second["provider"] = json!("second-agent-provider");
    second["execution_id"] = json!("second-agent-execution");
    value["reviews"].as_array_mut().unwrap().push(second);
    assert_eq!(evaluate(value.clone()).status, ReviewStatus::Approved);
    value["reviews"][1]["identity"]["pull_request_author"] = json!("repository-owner");
    assert!(evaluate(value).has_reason("PK-REVIEW-AUTHOR-MISSING"));
}

#[test]
fn opt_in_preserves_provider_findings_conflict_and_approval_requirements() {
    let mut value = owner_input();
    value["policy"]["required_reviews"]["allowed_providers"] = json!(["another-provider"]);
    assert!(evaluate(value).has_reason("PK-REVIEW-PROVIDER-NOT-ALLOWED"));

    let mut value = owner_input();
    value["reviews"][0]["status"] = json!("blocked");
    value["findings"] = json!([{
        "id": "security-finding", "severity": "high", "summary": "must be addressed",
        "reviewer_execution_id": "review-b"
    }]);
    assert_eq!(evaluate(value).status, ReviewStatus::Blocked);
    let mut value = owner_input();
    value["reviews"][0]["status"] = json!("inconclusive");
    assert!(evaluate(value).has_reason("PK-REVIEW-AGENT-MISSING"));

    let mut value = owner_input();
    let mut second = value["reviews"][0].clone();
    second["provider"] = json!("second-agent-provider");
    second["execution_id"] = json!("second-agent-execution");
    second.as_object_mut().unwrap().remove("owner_approval");
    second["status"] = json!("blocked");
    value["reviews"].as_array_mut().unwrap().push(second);
    assert!(evaluate(value.clone()).has_reason("PK-REVIEW-CONFLICT"));
    value["reviews"][1]["owner_approval"] = value["reviews"][0]["owner_approval"].clone();
    assert!(evaluate(value).has_reason("PK-REVIEW-OWNER-AMBIGUOUS"));
}

#[test]
fn agent_cannot_be_any_modifier_even_with_different_provider() {
    let mut value = owner_input();
    value["modifications"][0]["actor"] = json!("independent-agent");
    let result = evaluate(value);
    assert_eq!(result.status, ReviewStatus::Inconclusive);
    assert!(result.has_reason("PK-REVIEW-AGENT-SELF-MODIFIER"));
}

#[test]
fn owner_authorization_is_separate_and_modifier_identities_are_trusted() {
    let mut value = owner_input();
    value["reviews"][0]["owner_approval"]["record_digest"] =
        value["reviews"][0]["report_digest"].clone();
    assert!(evaluate(value).has_reason("PK-REVIEW-OWNER-RECORD-NOT-SEPARATE"));
    let mut value = owner_input();
    value["mode"] = json!("advisory");
    value["modifications"][0]["trust"] = json!("local_untrusted");
    assert!(evaluate(value).has_reason("PK-REVIEW-MODIFIER-UNTRUSTED"));
}

#[test]
fn local_runner_and_untrusted_evidence_cannot_supply_owner_or_agent_reviews() {
    for mode in ["authoritative", "advisory"] {
        for trust in [TrustClass::LocalRunnerAttested, TrustClass::LocalUntrusted] {
            for (pointer, reason) in [
                ("/reviews/0/trust", "PK-REVIEW-AGENT-UNTRUSTED"),
                (
                    "/reviews/0/owner_approval/trust",
                    "PK-REVIEW-OWNER-UNTRUSTED",
                ),
            ] {
                let mut value = owner_input();
                value["mode"] = json!(mode);
                *value.pointer_mut(pointer).unwrap() = json!(trust);
                let result = evaluate(value);
                assert_eq!(result.status, ReviewStatus::Inconclusive);
                assert!(result.has_reason(reason), "{result:?}");
            }
        }
    }
}

#[test]
fn default_quorum_and_v4_without_opt_in_are_unchanged() {
    let legacy: ReviewEvaluationInput = serde_json::from_str(include_str!(
        "../../../tests/fixtures/reviews/approved.json"
    ))
    .unwrap();
    let expected = evaluate_reviews(legacy.clone());
    let mut v4 = legacy.clone();
    v4.policy.schema_version = 4;
    assert_eq!(evaluate_reviews(v4.clone()), expected);
    v4.policy
        .required_reviews
        .as_mut()
        .unwrap()
        .minimum_reviewers = 2;
    assert!(evaluate_reviews(v4).has_reason("PK-REVIEW-QUORUM"));

    let mut absent = owner_input();
    absent["policy"]
        .as_object_mut()
        .unwrap()
        .remove("owner_with_independent_agent_review");
    assert!(evaluate(absent).has_reason("PK-REVIEW-QUORUM"));
}

fn ready_owner_value() -> Value {
    let mut value = support::ready_value();
    let owner = owner_input();
    value["policy"]["schema_version"] = json!(4);
    value["policy"]["owner_with_independent_agent_review"] =
        owner["policy"]["owner_with_independent_agent_review"].clone();
    value["evidence"]["reviews"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    value["evidence"]["reviews"][0]["identity"] = owner["reviews"][0]["identity"].clone();
    value["evidence"]["reviews"][0]["owner_approval"] =
        owner["reviews"][0]["owner_approval"].clone();
    value["evidence"]["reviews"][0]["owner_approval"]["repository"] =
        value["pull_request"]["repository"].clone();
    value["evidence"]["reviews"][0]["owner_approval"]["head_sha"] =
        value["pull_request"]["head_sha"].clone();
    value["evidence"]["reviews"][0]["owner_approval"]["observed_at"] =
        value["evaluation_time"].clone();
    for modification in value["evidence"]["modifications"].as_array_mut().unwrap() {
        modification["actor"] = json!("repository-owner");
    }
    seal_owner_value(&mut value);
    value
}

fn seal_owner_value(value: &mut Value) {
    let policy: policy_contracts::PolicySnapshot =
        serde_json::from_value(value["policy"].clone()).unwrap();
    let digest = policy_core::pre_merge_policy_digest(&policy).unwrap();
    value["policy"]["policy_digest"] = json!(digest);
    value["evidence"]["manifest"]["policy_digest"] = json!(digest);
    support::seal_evidence(value);
}

#[test]
fn full_pre_merge_uses_the_owner_rule_and_binds_all_new_evidence() {
    use policy_core::{evaluate_pre_merge, pre_merge_evidence_digest};
    let original = support::pre_merge_input(ready_owner_value());
    let decision = evaluate_pre_merge(&original);
    assert_eq!(
        decision.status,
        policy_contracts::PreMergeStatus::ReadyToMerge,
        "{decision:?}"
    );
    let old_digest = pre_merge_evidence_digest(&original.evidence).unwrap();
    for (pointer, replacement) in [
        (
            "/evidence/reviews/0/owner_approval/actor",
            json!("wrong-owner"),
        ),
        (
            "/evidence/reviews/0/identity/actor",
            json!("repository-owner"),
        ),
        (
            "/evidence/reviews/0/owner_approval/record_digest",
            json!("d".repeat(64)),
        ),
        ("/evidence/modifications/0/actor", json!("another-modifier")),
    ] {
        let mut value = ready_owner_value();
        *value.pointer_mut(pointer).unwrap() = replacement;
        seal_owner_value(&mut value);
        let changed = support::pre_merge_input(value);
        assert_ne!(
            pre_merge_evidence_digest(&changed.evidence).unwrap(),
            old_digest
        );
        assert_ne!(
            evaluate_pre_merge(&changed).decision_digest,
            decision.decision_digest
        );
    }
    let mut value = ready_owner_value();
    value["evidence"]["reviews"][0]["identity"]["actor"] = json!("repository-owner");
    seal_owner_value(&mut value);
    assert_eq!(
        evaluate_pre_merge(&support::pre_merge_input(value)).status,
        policy_contracts::PreMergeStatus::Inconclusive
    );

    let mut value = ready_owner_value();
    value["policy"]
        .as_object_mut()
        .unwrap()
        .remove("owner_with_independent_agent_review");
    seal_owner_value(&mut value);
    assert_eq!(
        evaluate_pre_merge(&support::pre_merge_input(value)).status,
        policy_contracts::PreMergeStatus::Inconclusive
    );
}

#[test]
fn v5_pre_merge_applies_owner_and_independent_agent_review() {
    use policy_contracts::PreMergeStatus;
    use policy_core::evaluate_pre_merge;

    let mut value = ready_owner_value();
    value["policy"]["schema_version"] = json!(5);
    value["policy"]["scope_rules"] = json!({
        "default_required_sensor_ids": ["compile"], "rules": []
    });
    value["context"].as_object_mut().unwrap().remove("risk");
    value["context"]["labels"] = json!([]);
    seal_owner_value(&mut value);
    let decision = evaluate_pre_merge(&support::pre_merge_input(value.clone()));
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{decision:?}"
    );

    value["evidence"]["reviews"][0]
        .as_object_mut()
        .unwrap()
        .remove("owner_approval");
    seal_owner_value(&mut value);
    let decision = evaluate_pre_merge(&support::pre_merge_input(value));
    assert_eq!(decision.status, PreMergeStatus::Inconclusive);
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "PK-REVIEW-OWNER-MISSING"));
}

#[test]
fn local_evidence_opt_in_cannot_upgrade_reviews_or_owner_authorization() {
    for pointer in [
        "/evidence/reviews/0/trust",
        "/evidence/reviews/0/owner_approval/trust",
    ] {
        let mut value = ready_owner_value();
        value["policy"]["local_runner_evidence"] = json!({
            "allowed_runner_ids": ["runner"], "approved_scope_template_digests": ["a".repeat(64)],
            "maximum_age_seconds": 600, "ci_required_when": []
        });
        *value.pointer_mut(pointer).unwrap() = json!("local_runner_attested");
        seal_owner_value(&mut value);
        let result = policy_core::evaluate_pre_merge(&support::pre_merge_input(value));
        assert_eq!(
            result.status,
            policy_contracts::PreMergeStatus::Inconclusive,
            "{result:?}"
        );
    }
}
