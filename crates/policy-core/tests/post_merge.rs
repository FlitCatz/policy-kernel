use policy_contracts::{PostMergeEvaluationInput, PostMergeStatus, PreMergeEvaluationInput};
use policy_core::{
    canonical_sha256, evaluate_post_merge, evaluate_pre_merge, pre_merge_policy_digest,
};
use serde_json::{json, Value};

fn fixture(name: &str) -> &'static str {
    match name {
        "merge.json" => include_str!("../../../tests/fixtures/post-merge/merge.json"),
        "squash.json" => include_str!("../../../tests/fixtures/post-merge/squash.json"),
        "wrong-head.json" => {
            include_str!("../../../tests/fixtures/post-merge/wrong-head.json")
        }
        "missing-check.json" => {
            include_str!("../../../tests/fixtures/post-merge/missing-check.json")
        }
        "late-check.json" => {
            include_str!("../../../tests/fixtures/post-merge/late-check.json")
        }
        "advisory.json" => include_str!("../../../tests/fixtures/post-merge/advisory.json"),
        _ => panic!("unknown fixture {name}"),
    }
}

fn raw(name: &str) -> Value {
    serde_json::from_str(fixture(name)).unwrap()
}

fn load(name: &str) -> PostMergeEvaluationInput {
    serde_json::from_str(fixture(name)).unwrap()
}

fn assert_reason(value: Value, status: PostMergeStatus, reason: &str) {
    let input: PostMergeEvaluationInput = serde_json::from_value(value).unwrap();
    let decision = evaluate_post_merge(&input);
    assert_eq!(decision.status, status, "{:?}", decision.reasons);
    assert!(
        decision.reasons.iter().any(|item| item.code == reason),
        "missing {reason}: {:?}",
        decision.reasons
    );
}

#[test]
fn merge_and_squash_ship_with_a_distinct_merge_commit_sha() {
    for name in ["merge.json", "squash.json"] {
        let input = load(name);
        assert_ne!(
            input.merge.merge_commit_sha,
            input.pre_merge.as_decision().binding.head_sha
        );
        let decision = evaluate_post_merge(&input);
        assert_eq!(decision.status, PostMergeStatus::Shipped, "{name}");
        assert_eq!(
            decision.merge_commit_sha.as_ref(),
            Some(&input.merge.merge_commit_sha)
        );
        assert!(decision.reasons.is_empty());
    }
}

#[test]
fn required_fixtures_fail_for_the_specified_authorization_break() {
    for (name, reason) in [
        ("wrong-head.json", "PK-POST-HEAD-MISMATCH"),
        ("missing-check.json", "PK-POST-CHECK-MISSING"),
        ("late-check.json", "PK-POST-CHECK-AFTER-MERGE"),
    ] {
        assert_reason(raw(name), PostMergeStatus::Inconclusive, reason);
    }
}

#[test]
fn advisory_decision_is_rejected_before_post_merge_evaluation() {
    assert!(serde_json::from_str::<PostMergeEvaluationInput>(fixture("advisory.json")).is_err());
}

#[test]
fn every_required_check_binding_must_match_exactly() {
    for (pointer, replacement, reason) in [
        (
            "/check/repository",
            json!("other/repository"),
            "PK-POST-REPOSITORY-MISMATCH",
        ),
        (
            "/check/pull_request_number",
            json!(8),
            "PK-POST-PR-MISMATCH",
        ),
        (
            "/check/head_sha",
            json!("b".repeat(40)),
            "PK-POST-HEAD-MISMATCH",
        ),
        (
            "/check/policy_digest",
            json!("b".repeat(64)),
            "PK-POST-POLICY-MISMATCH",
        ),
        (
            "/check/decision_digest",
            json!("b".repeat(64)),
            "PK-POST-DECISION-MISMATCH",
        ),
        (
            "/check/check_name",
            json!("another-check"),
            "PK-POST-CHECK-NAME-MISMATCH",
        ),
    ] {
        let mut value = raw("merge.json");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_reason(value, PostMergeStatus::Inconclusive, reason);
    }
}

#[test]
fn merge_must_match_the_authorized_pr_and_policy() {
    for (pointer, replacement, reason) in [
        (
            "/merge/repository",
            json!("other/repository"),
            "PK-POST-REPOSITORY-MISMATCH",
        ),
        (
            "/merge/pull_request_number",
            json!(8),
            "PK-POST-PR-MISMATCH",
        ),
        (
            "/merge/head_sha",
            json!("b".repeat(40)),
            "PK-POST-HEAD-MISMATCH",
        ),
        (
            "/pre_merge/binding/policy_digest",
            json!("b".repeat(64)),
            "PK-POST-POLICY-MISMATCH",
        ),
    ] {
        let mut value = raw("merge.json");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_reason(value, PostMergeStatus::Inconclusive, reason);
    }
}

#[test]
fn unsuccessful_or_invalid_check_observations_are_inconclusive() {
    for (pointer, replacement, reason) in [
        (
            "/check/conclusion",
            json!("failure"),
            "PK-POST-CHECK-NOT-SUCCESS",
        ),
        (
            "/check/started_at",
            json!("2026-09-07T09:05:00Z"),
            "PK-POST-CHECK-TIME-INVALID",
        ),
        (
            "/pre_merge/binding/evaluated_at",
            json!("2026-09-07T09:04:00.000000001Z"),
            "PK-POST-CHECK-TIME-INVALID",
        ),
    ] {
        let mut value = raw("merge.json");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_reason(value, PostMergeStatus::Inconclusive, reason);
    }
}

#[test]
fn protected_target_human_actor_declared_method_and_no_queue_are_required() {
    for (pointer, replacement, reason) in [
        (
            "/merge/protected",
            json!(false),
            "PK-POST-TARGET-UNPROTECTED",
        ),
        (
            "/merge/target_branch",
            json!("development"),
            "PK-POST-TARGET-UNPROTECTED",
        ),
        (
            "/merge/actor_type",
            json!("bot"),
            "PK-POST-MERGE-ACTOR-UNAUTHORIZED",
        ),
        (
            "/merge/merged_by",
            json!("stranger"),
            "PK-POST-MERGE-ACTOR-UNAUTHORIZED",
        ),
        (
            "/merge/method",
            json!("rebase"),
            "PK-POST-MERGE-METHOD-UNDECLARED",
        ),
        (
            "/merge/via_merge_queue",
            json!(true),
            "PK-POST-MERGE-QUEUE-UNSUPPORTED",
        ),
    ] {
        let mut value = raw("merge.json");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_reason(value, PostMergeStatus::Blocked, reason);
    }
}

#[test]
fn absent_new_authorization_facts_fail_closed() {
    for (pointer, reason) in [
        ("/merge/target_branch", "PK-POST-TARGET-UNPROTECTED"),
        ("/merge/actor_type", "PK-POST-MERGE-ACTOR-UNAUTHORIZED"),
        ("/merge/via_merge_queue", "PK-POST-MERGE-QUEUE-UNKNOWN"),
    ] {
        let mut value = raw("merge.json");
        value.pointer_mut(pointer).unwrap().take();
        value["merge"]
            .as_object_mut()
            .unwrap()
            .retain(|_, item| !item.is_null());
        assert_reason(value, PostMergeStatus::Inconclusive, reason);
    }

    for (field, reason) in [
        ("protected_target_branches", "PK-POST-TARGET-UNPROTECTED"),
        ("allowed_merge_methods", "PK-POST-MERGE-METHOD-UNDECLARED"),
    ] {
        let mut value = raw("merge.json");
        value["policy"].as_object_mut().unwrap().remove(field);
        // Keep the declared policy digest coherent so this isolates the missing fact.
        let mut policy = value["policy"].clone();
        policy.as_object_mut().unwrap().remove("policy_digest");
        let digest = canonical_sha256(&policy).unwrap();
        value["policy"]["policy_digest"] = json!(digest);
        value["pre_merge"]["binding"]["policy_digest"] = json!(digest);
        value["check"]["policy_digest"] = json!(digest);
        assert_reason(value, PostMergeStatus::Inconclusive, reason);
    }
}

#[test]
fn chronology_and_freshness_use_exact_boundaries() {
    let mut equal = raw("merge.json");
    equal["check"]["completed_at"] = equal["merge"]["merged_at"].clone();
    assert_reason(
        equal,
        PostMergeStatus::Inconclusive,
        "PK-POST-CHECK-AFTER-MERGE",
    );

    for (pointer, timestamp, reason) in [
        (
            "/check/started_at",
            "2026-09-07T08:04:00Z",
            "PK-POST-CHECK-STALE",
        ),
        (
            "/merge/merged_at",
            "2026-09-07T08:59:59.999999999Z",
            "PK-POST-MERGE-STALE",
        ),
        (
            "/merge/merged_at",
            "2026-09-07T10:00:00.000000001Z",
            "PK-POST-MERGE-TIME-INVALID",
        ),
    ] {
        let mut value = raw("merge.json");
        *value.pointer_mut(pointer).unwrap() = json!(timestamp);
        assert_reason(value, PostMergeStatus::Inconclusive, reason);
    }

    let mut boundary = raw("merge.json");
    boundary["check"]["started_at"] = json!("2026-09-07T08:05:00Z");
    boundary["evaluation_time"] = json!("2026-09-07T10:05:00Z");
    assert_eq!(
        evaluate_post_merge(&serde_json::from_value(boundary).unwrap()).status,
        PostMergeStatus::Shipped
    );
}

#[test]
fn schema_versions_are_checked_at_runtime_and_wire_boundaries_are_strict() {
    for pointer in [
        "/schema_version",
        "/policy/schema_version",
        "/pre_merge/schema_version",
        "/check/schema_version",
        "/merge/schema_version",
    ] {
        let mut input = load("merge.json");
        match pointer {
            "/schema_version" => input.schema_version = 99,
            "/policy/schema_version" => input.policy.schema_version = 99,
            "/pre_merge/schema_version" => {
                let mut decision = input.pre_merge.clone().into_decision();
                decision.schema_version = 99;
                input.pre_merge = decision.try_into().unwrap();
            }
            "/check/schema_version" => input.check.as_mut().unwrap().schema_version = 99,
            _ => input.merge.schema_version = 99,
        }
        assert_eq!(
            evaluate_post_merge(&input).status,
            PostMergeStatus::Inconclusive
        );
    }
    for pointer in ["/merge", "/check"] {
        let mut value = raw("merge.json");
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(serde_json::from_value::<PostMergeEvaluationInput>(value).is_err());
    }
}

#[test]
fn decision_digest_binds_every_input_and_the_result_deterministically() {
    let original = evaluate_post_merge(&load("merge.json"));
    for (pointer, replacement) in [
        ("/evaluation_time", json!("2026-09-07T09:59:59Z")),
        ("/merge/merge_commit_sha", json!("d".repeat(40))),
        ("/check/workflow_run_id", json!("another-run")),
    ] {
        let mut value = raw("merge.json");
        *value.pointer_mut(pointer).unwrap() = replacement;
        let decision = evaluate_post_merge(&serde_json::from_value(value).unwrap());
        assert_eq!(decision.status, original.status);
        assert_ne!(decision.decision_digest, original.decision_digest);
    }
    for _ in 0..100 {
        assert_eq!(evaluate_post_merge(&load("merge.json")), original);
    }
}

#[test]
fn actual_pre_merge_output_can_authorize_post_merge() {
    let mut value: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/pre-merge/ready.json")).unwrap();
    value["policy"]["protected_target_branches"] = json!(["main"]);
    value["policy"]["allowed_merge_methods"] = json!(["merge", "squash"]);
    let policy: policy_contracts::PolicySnapshot =
        serde_json::from_value(value["policy"].clone()).unwrap();
    let policy_digest = pre_merge_policy_digest(&policy).unwrap();
    value["policy"]["policy_digest"] = json!(policy_digest);
    value["evidence"]["manifest"]["policy_digest"] = json!(policy_digest);
    let input: PreMergeEvaluationInput = serde_json::from_value(value).unwrap();
    let pre_merge = evaluate_pre_merge(&input);
    assert_eq!(
        pre_merge.status,
        policy_contracts::PreMergeStatus::ReadyToMerge
    );

    let mut post = raw("merge.json");
    post["policy"] = serde_json::to_value(&input.policy).unwrap();
    post["pre_merge"] = serde_json::to_value(&pre_merge).unwrap();
    post["check"]["pull_request_number"] = json!(6);
    post["check"]["policy_digest"] = json!(pre_merge.binding.policy_digest);
    post["check"]["decision_digest"] = json!(pre_merge.decision_digest);
    post["merge"]["pull_request_number"] = json!(6);
    let decision = evaluate_post_merge(&serde_json::from_value(post).unwrap());
    assert_eq!(
        decision.status,
        PostMergeStatus::Shipped,
        "{:?}",
        decision.reasons
    );
}
