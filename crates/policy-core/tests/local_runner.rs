mod support;

use policy_contracts::{
    PostMergeEvaluationInput, PostMergeStatus, PreMergeDecision, PreMergeStatus,
};
use policy_core::{
    canonical_sha256, evaluate_post_merge, evaluate_pre_merge, pre_merge_policy_digest,
};
use serde_json::{json, Value};
use support::{pre_merge_input, ready_value, seal_evidence};

fn candidate(block: bool, ci_matches: bool) -> Value {
    let mut value = ready_value();
    value["policy"]["schema_version"] = json!(4);
    if block {
        value["policy"]["local_runner_evidence"] = json!({
            "allowed_runner_ids": ["owner-runner"],
            "approved_scope_template_digests": ["a".repeat(64)],
            "maximum_age_seconds": 300,
            "ci_required_when": [{"id": "ci-path", "risks": [],
                "path_globs": [if ci_matches {"src/**"} else {"tools/**"}]}]
        });
    }
    value["evidence"]["knowledge"] = json!([{
        "kind": "code_graph_index", "source": "ci/index", "version": "1",
        "configuration_digest": "a".repeat(64), "content_digest": "b".repeat(64),
        "source_set_digest": value["evidence"]["manifest"]["source_set_digest"],
        "head_sha": value["pull_request"]["head_sha"], "trust": "ci_observed"
    }]);
    value
}

fn decide(mut value: Value) -> PreMergeDecision {
    let policy = serde_json::from_value(value["policy"].clone()).unwrap();
    let digest = json!(pre_merge_policy_digest(&policy).unwrap());
    value["policy"]["policy_digest"] = digest.clone();
    value["evidence"]["manifest"]["policy_digest"] = digest;
    seal_evidence(&mut value);
    evaluate_pre_merge(&pre_merge_input(value))
}

fn has(decision: &PreMergeDecision, code: &str) -> bool {
    decision.reasons.iter().any(|r| r.code == code)
}

const KINDS: [&str; 6] = [
    "/evidence/sensors/0",
    "/evidence/executions/0",
    "/evidence/modifications/0",
    "/evidence/reviews/0",
    "/context/agent_ready",
    "/evidence/knowledge/0",
];

// Oracle: local-evidence design section 6.3. This exhaustive product checks every
// state, including the three no-authoritative-readiness invariants, without sampling.
#[test]
fn exhaustive_trust_kind_mode_and_policy_matrix() {
    for trust in [
        "local_untrusted",
        "local_runner_attested",
        "ci_observed",
        "trusted_runner_attested",
    ] {
        for (kind, pointer) in KINDS.iter().enumerate() {
            for advisory in [false, true] {
                for block in [false, true] {
                    for ci in [false, true] {
                        let mut value = candidate(block, ci);
                        value["mode"] = json!(if advisory {
                            "advisory"
                        } else {
                            "authoritative"
                        });
                        value.pointer_mut(pointer).unwrap()["trust"] = json!(trust);
                        let decision = decide(value);
                        let (status, reason) = if advisory {
                            (PreMergeStatus::ReadyToMerge, None)
                        } else {
                            match (trust, kind, block, ci) {
                                ("local_untrusted", _, _, _) => {
                                    (PreMergeStatus::Inconclusive, Some("PK-PROV-UNTRUSTED"))
                                }
                                ("local_runner_attested", 2..=5, _, _) => (
                                    PreMergeStatus::Inconclusive,
                                    Some("PK-PROV-LOCAL-CLASS-INVALID"),
                                ),
                                ("local_runner_attested", _, false, _) => (
                                    PreMergeStatus::Inconclusive,
                                    Some("PK-PROV-LOCAL-NOT-ACCEPTED"),
                                ),
                                ("local_runner_attested", _, true, true) => {
                                    (PreMergeStatus::Inconclusive, Some("PK-PROV-CI-REQUIRED"))
                                }
                                ("local_runner_attested", _, true, false) => {
                                    (PreMergeStatus::ReadyToMerge, Some("PK-PROV-LOCAL-ATTESTED"))
                                }
                                _ => (PreMergeStatus::ReadyToMerge, None),
                            }
                        };
                        assert_eq!(
                            decision.status, status,
                            "{trust} {pointer} advisory={advisory} block={block} ci={ci}: {:?}",
                            decision.reasons
                        );
                        if let Some(reason) = reason {
                            assert!(
                                has(&decision, reason),
                                "{trust} {pointer}: {:?}",
                                decision.reasons
                            );
                        }
                        assert_eq!(
                            has(&decision, "PK-PROV-LOCAL-ATTESTED"),
                            reason == Some("PK-PROV-LOCAL-ATTESTED")
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn local_execution_age_uses_start_and_exact_subsecond_boundaries() {
    for (start, expected) in [
        ("2026-09-07T08:55:00Z", PreMergeStatus::ReadyToMerge),
        (
            "2026-09-07T08:55:00.000000001Z",
            PreMergeStatus::ReadyToMerge,
        ),
        (
            "2026-09-07T08:54:59.999999999Z",
            PreMergeStatus::Inconclusive,
        ),
        ("2026-09-07T09:00:00Z", PreMergeStatus::ReadyToMerge),
        (
            "2026-09-07T09:00:00.000000001Z",
            PreMergeStatus::Inconclusive,
        ),
    ] {
        let mut value = candidate(true, false);
        value["evidence"]["executions"][0]["trust"] = json!("local_runner_attested");
        value["evidence"]["executions"][0]["started_at"] = json!(start);
        value["evidence"]["executions"][0]["completed_at"] = json!("2026-09-07T09:00:00Z");
        let decision = decide(value);
        assert_eq!(decision.status, expected, "{start}: {:?}", decision.reasons);
        assert_eq!(
            has(&decision, "PK-PROV-STALE"),
            expected == PreMergeStatus::Inconclusive
        );
    }
}

#[test]
fn local_freshness_can_outlive_global_freshness_without_relaxing_ci_or_advisory() {
    for (sensor, execution) in [(true, false), (false, true), (true, true)] {
        for (trust, advisory, expected) in [
            ("local_runner_attested", false, PreMergeStatus::ReadyToMerge),
            ("ci_observed", false, PreMergeStatus::Inconclusive),
            (
                "trusted_runner_attested",
                false,
                PreMergeStatus::Inconclusive,
            ),
            ("local_runner_attested", true, PreMergeStatus::Inconclusive),
        ] {
            let mut value = candidate(true, false);
            value["policy"]["max_evidence_age_seconds"] = json!(300);
            value["policy"]["local_runner_evidence"]["maximum_age_seconds"] = json!(600);
            if advisory {
                value["mode"] = json!("advisory");
            }
            if sensor {
                value["evidence"]["sensors"][0]["trust"] = json!(trust);
                value["evidence"]["sensors"][0]["observed_at"] = json!("2026-09-07T08:51:00Z");
            }
            if execution {
                value["evidence"]["executions"][0]["trust"] = json!(trust);
                value["evidence"]["executions"][0]["started_at"] = json!("2026-09-07T08:50:00Z");
                value["evidence"]["executions"][0]["completed_at"] = json!("2026-09-07T08:51:00Z");
            }
            let decision = decide(value);
            assert_eq!(
                decision.status, expected,
                "{trust} sensor={sensor} execution={execution} advisory={advisory}: {:?}",
                decision.reasons
            );
            assert_eq!(
                has(&decision, "PK-PROV-STALE"),
                expected == PreMergeStatus::Inconclusive
            );
        }
    }
}

#[test]
fn local_execution_completion_cannot_be_in_the_future() {
    let mut value = candidate(true, false);
    value["evidence"]["executions"][0]["trust"] = json!("local_runner_attested");
    value["evidence"]["executions"][0]["completed_at"] = json!("2026-09-07T09:00:00.000000001Z");
    let decision = decide(value);
    assert_eq!(decision.status, PreMergeStatus::Inconclusive);
    assert!(has(&decision, "PK-PROV-STALE"));
}

#[test]
fn local_sensor_age_and_future_observations_fail_closed() {
    for (time, expected) in [
        ("2026-09-07T08:55:00Z", PreMergeStatus::ReadyToMerge),
        (
            "2026-09-07T08:54:59.999999999Z",
            PreMergeStatus::Inconclusive,
        ),
        ("2026-09-07T09:00:00Z", PreMergeStatus::ReadyToMerge),
        (
            "2026-09-07T09:00:00.000000001Z",
            PreMergeStatus::Inconclusive,
        ),
    ] {
        let mut value = candidate(true, false);
        value["evidence"]["sensors"][0]["trust"] = json!("local_runner_attested");
        value["evidence"]["sensors"][0]["observed_at"] = json!(time);
        let decision = decide(value);
        assert_eq!(decision.status, expected, "{time}: {:?}", decision.reasons);
    }
}

#[test]
fn local_sensor_window_is_measured_from_execution_start_even_with_ci_execution() {
    for start in [
        "2026-09-07T08:54:59.999999999Z",
        "2026-09-07T09:00:00.000000001Z",
    ] {
        let mut value = candidate(true, false);
        value["evidence"]["sensors"][0]["trust"] = json!("local_runner_attested");
        value["evidence"]["executions"][0]["started_at"] = json!(start);
        let decision = decide(value);
        assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        assert!(has(&decision, "PK-PROV-STALE"));
        assert!(!has(&decision, "PK-PROV-LOCAL-ATTESTED"));
    }
    let mut value = candidate(true, false);
    value["evidence"]["sensors"][0]["trust"] = json!("local_runner_attested");
    value["evidence"]["sensors"][0]["command_digest"] = json!("f".repeat(64));
    let decision = decide(value);
    assert_eq!(decision.status, PreMergeStatus::Inconclusive);
    assert!(has(&decision, "PK-PROV-BINDING-MISSING"));
}

#[test]
fn ci_conditions_combine_risk_and_any_path_then_union_conditions() {
    for (paths, risk, requires_ci) in [
        (vec!["src/lib.rs", "tools/check.rs"], "high", true),
        (vec!["src/lib.rs", "tools/check.rs"], "low", false),
        (vec!["src/lib.rs"], "high", false),
        (vec!["tools\\check.rs"], "critical", true),
        (vec!["./tools/check.rs"], "high", true),
        (vec!["Cargo.lock"], "low", true),
        (vec![], "critical", false),
    ] {
        let mut value = candidate(true, false);
        value["evidence"]["executions"][0]["trust"] = json!("local_runner_attested");
        value["context"]["changed_paths"] = json!(paths);
        value["context"]["risk"] = json!(risk);
        value["policy"]["local_runner_evidence"]["ci_required_when"] = json!([
            {"id": "tools", "risks": ["high", "critical"], "path_globs": ["tools/**"]},
            {"id": "dependencies", "risks": [], "path_globs": ["Cargo.lock"]}
        ]);
        let decision = decide(value);
        assert_eq!(
            has(&decision, "PK-PROV-CI-REQUIRED"),
            requires_ci,
            "{paths:?} {risk}: {:?}",
            decision.reasons
        );
        assert_eq!(
            decision.status,
            if requires_ci {
                PreMergeStatus::Inconclusive
            } else {
                PreMergeStatus::ReadyToMerge
            }
        );
    }
}

#[test]
fn invalid_ci_conditions_are_rejected_even_when_other_conditions_do_not_match() {
    for conditions in [
        json!([{"id": " ", "risks": [], "path_globs": []}]),
        json!([{"id": "same", "risks": [], "path_globs": ["other/**"]}, {"id": "same", "risks": [], "path_globs": []}]),
        json!([{"id": "invalid", "risks": ["low"], "path_globs": ["[bad"]}]),
        json!([{"id": "invalid", "risks": [], "path_globs": [""]}]),
    ] {
        let mut value = candidate(true, false);
        value["policy"]["local_runner_evidence"]["ci_required_when"] = conditions;
        value["evidence"]["sensors"][0]["trust"] = json!("local_runner_attested");
        let decision = decide(value);
        assert_eq!(
            decision.status,
            PreMergeStatus::Inconclusive,
            "{:?}",
            decision.reasons
        );
        assert!(has(&decision, "PK-PROV-CI-REQUIRED"));
        assert!(!has(&decision, "PK-PROV-LOCAL-ATTESTED"));
    }
}

#[test]
fn empty_condition_dimensions_and_invalid_context_fail_closed_as_specified() {
    for (conditions, expected) in [
        (json!([]), PreMergeStatus::ReadyToMerge),
        (
            json!([{"id": "all", "risks": [], "path_globs": []}]),
            PreMergeStatus::Inconclusive,
        ),
        (
            json!([{"id": "risk", "risks": ["high"], "path_globs": []}]),
            PreMergeStatus::Inconclusive,
        ),
        (
            json!([{"id": "risk", "risks": ["low"], "path_globs": []}]),
            PreMergeStatus::ReadyToMerge,
        ),
    ] {
        let mut value = candidate(true, false);
        value["policy"]["local_runner_evidence"]["ci_required_when"] = conditions;
        value["evidence"]["executions"][0]["trust"] = json!("local_runner_attested");
        let decision = decide(value);
        assert_eq!(decision.status, expected, "{:?}", decision.reasons);
    }
    for context in [Value::Null, json!({"changed_paths": ["../src/lib.rs"]})] {
        let mut value = candidate(true, false);
        value["evidence"]["executions"][0]["trust"] = json!("local_runner_attested");
        if context.is_null() {
            value.as_object_mut().unwrap().remove("context");
        } else {
            value["context"]["changed_paths"] = context["changed_paths"].clone();
        }
        let decision = decide(value);
        assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        assert!(has(&decision, "PK-PROV-CI-REQUIRED"));
        assert!(!has(&decision, "PK-PROV-LOCAL-ATTESTED"));
    }
}

#[test]
fn standalone_review_evaluation_rejects_local_class_for_both_review_kinds() {
    for pointer in ["/evidence/reviews/0", "/evidence/modifications/0"] {
        let mut value = candidate(true, false);
        value.pointer_mut(pointer).unwrap()["trust"] = json!("local_runner_attested");
        let input = pre_merge_input(value);
        let result = policy_core::evaluate_reviews(policy_core::ReviewEvaluationInput {
            mode: input.mode,
            policy: input.policy,
            repository: input.pull_request.repository,
            head_sha: input.pull_request.head_sha,
            evidence_digest: policy_core::pre_merge_evidence_digest(&input.evidence).unwrap(),
            evaluation_time: input.evaluation_time,
            modifications: input.evidence.modifications,
            reviews: input.evidence.reviews,
            findings: input.evidence.findings,
            waivers: input.evidence.waivers,
        });
        assert_eq!(result.status, policy_contracts::ReviewStatus::Inconclusive);
        assert!(result.has_reason("PK-PROV-LOCAL-CLASS-INVALID"));
    }
}

#[test]
fn mixed_ci_and_local_evidence_preserves_the_local_marker_and_digest_binding() {
    let value = candidate(true, false);
    let ci = decide(value.clone());
    let mut local = value;
    local["evidence"]["sensors"][0]["trust"] = json!("local_runner_attested");
    local["evidence"]["executions"][0]["trust"] = json!("local_runner_attested");
    let decision = decide(local);
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{:?}",
        decision.reasons
    );
    assert_eq!(decision.reasons.len(), 1);
    assert!(has(&decision, "PK-PROV-LOCAL-ATTESTED"));
    assert_ne!(ci.decision_digest, decision.decision_digest);
    let mut input = pre_merge_input(candidate(true, false));
    input.evidence.sensors[0].trust = policy_contracts::TrustClass::LocalRunnerAttested;
    input.evidence.executions[0].trust = policy_contracts::TrustClass::LocalRunnerAttested;
    input.policy.policy_digest = pre_merge_policy_digest(&input.policy).unwrap();
    input.evidence.manifest.policy_digest = input.policy.policy_digest.clone();
    // Reconstruct the documented digest preimage independently, then remove only
    // the informational reason. Evidence changes alone cannot prove reason binding.
    let mut input_json = serde_json::to_value(&input).unwrap();
    input_json["evidence"]["manifest"]
        .as_object_mut()
        .unwrap()
        .remove("child_digests");
    for name in [
        "modifications",
        "executions",
        "reviews",
        "findings",
        "waivers",
        "sensors",
        "knowledge",
    ] {
        input_json["evidence"][name]
            .as_array_mut()
            .unwrap()
            .sort_by_key(|r| serde_json::to_vec(r).unwrap());
    }
    let mut result_json = serde_json::to_value(&decision).unwrap();
    result_json
        .as_object_mut()
        .unwrap()
        .remove("decision_digest");
    let mut preimage = json!({"domain": "policy-kernel/pre-merge-decision/v1", "input": input_json, "decision": result_json});
    assert_eq!(
        canonical_sha256(&preimage).unwrap(),
        decision.decision_digest
    );
    preimage["decision"]["reasons"] = json!([]);
    assert_ne!(
        canonical_sha256(&preimage).unwrap(),
        decision.decision_digest
    );
}

#[test]
fn local_ready_decision_can_never_ship_even_with_a_successful_bound_check() {
    let mut value = candidate(true, false);
    value["policy"]["protected_target_branches"] = json!(["main"]);
    value["policy"]["allowed_merge_methods"] = json!(["merge", "squash"]);
    let mut post = support::fixture_value("tests/fixtures/post-merge/merge.json");
    let ci = decide(value.clone());
    for local in [false, true] {
        if local {
            value["evidence"]["sensors"][0]["trust"] = json!("local_runner_attested");
        }
        let pre = if local {
            decide(value.clone())
        } else {
            ci.clone()
        };
        assert_eq!(pre.status, PreMergeStatus::ReadyToMerge);
        post["policy"] = value["policy"].clone();
        post["policy"]["policy_digest"] = json!(pre.binding.policy_digest);
        post["pre_merge"] = json!(pre);
        post["check"]["pull_request_number"] = json!(6);
        post["merge"]["pull_request_number"] = json!(6);
        post["check"]["policy_digest"] = json!(pre.binding.policy_digest);
        post["check"]["decision_digest"] = json!(pre.decision_digest);
        let input: PostMergeEvaluationInput = serde_json::from_value(post.clone()).unwrap();
        let decision = evaluate_post_merge(&input);
        assert_eq!(
            decision.status,
            if local {
                PostMergeStatus::Inconclusive
            } else {
                PostMergeStatus::Shipped
            },
            "{:?}",
            decision.reasons
        );
        assert_eq!(
            decision
                .reasons
                .iter()
                .any(|r| r.code == "PK-PROV-LOCAL-ATTESTED"),
            local
        );
    }
}
