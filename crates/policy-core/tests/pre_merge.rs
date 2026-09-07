use policy_contracts::*;
use policy_core::{canonical_sha256, evaluate_pre_merge, pre_merge_evidence_digest};
use serde_json::{json, Value};

fn raw(name: &str) -> Value {
    serde_json::from_str(match name {
        "ready" => include_str!("../../../tests/fixtures/pre-merge/ready.json"),
        "missing-agent-ready" => {
            include_str!("../../../tests/fixtures/pre-merge/missing-agent-ready.json")
        }
        "head-mismatch" => include_str!("../../../tests/fixtures/pre-merge/head-mismatch.json"),
        "advisory" => include_str!("../../../tests/fixtures/pre-merge/advisory.json"),
        _ => panic!("unknown fixture"),
    })
    .unwrap()
}

fn input(value: Value) -> PreMergeEvaluationInput {
    serde_json::from_value(value).unwrap()
}

// Only test setup: repair transport hashes after deliberately changing evidence.
// Expected decisions below are literal and do not depend on the sealing helper.
fn seal(value: &mut Value) {
    for path in [
        "executions",
        "findings",
        "knowledge",
        "modifications",
        "reviews",
        "sensors",
        "test_adequacy",
        "waivers",
    ] {
        let digest = canonical_sha256(&value["evidence"][path]).unwrap();
        let artifacts = value["evidence"]["manifest"]["child_digests"]
            .as_array_mut()
            .unwrap();
        let artifact = artifacts.iter_mut().find(|a| a["path"] == path).unwrap();
        artifact["digest"] = json!(digest);
    }
}

fn seal_policy(value: &mut Value) {
    value["policy"]["sensor_profile_digest"] =
        json!(canonical_sha256(&value["context"]["sensor_profile"]).unwrap());
    let mut policy = value["policy"].clone();
    policy.as_object_mut().unwrap().remove("policy_digest");
    value["policy"]["policy_digest"] = json!(canonical_sha256(&policy).unwrap());
    value["evidence"]["manifest"]["policy_digest"] = value["policy"]["policy_digest"].clone();
}

fn assert_result(value: Value, status: PreMergeStatus, code: &str) {
    let decision = evaluate_pre_merge(&input(value));
    assert_eq!(decision.status, status, "{:?}", decision.reasons);
    assert!(
        decision.reasons.iter().any(|r| r.code == code),
        "{code}: {:?}",
        decision.reasons
    );
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_err());
}

#[test]
fn authoritative_ready_requires_all_evidence_classes() {
    let decision = evaluate_pre_merge(&input(raw("ready")));
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{:?}",
        decision.reasons
    );
    assert!(!decision.advisory);
    assert!(decision.reasons.is_empty());
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_ok());
}

#[test]
fn missing_agent_ready_and_replayed_head_cannot_authorize() {
    assert_result(
        raw("missing-agent-ready"),
        PreMergeStatus::Inconclusive,
        "PK-PRE-AGENT-READY-MISSING",
    );
    assert_result(
        raw("head-mismatch"),
        PreMergeStatus::Inconclusive,
        "PK-PRE-BINDING-MISMATCH",
    );
}

#[test]
fn advisory_mode_never_emits_authoritative_decision() {
    let decision = evaluate_pre_merge(&input(raw("advisory")));
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{:?}",
        decision.reasons
    );
    assert!(decision.advisory);
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_err());
}

#[test]
fn every_binding_and_agent_authorization_is_checked() {
    for (pointer, replacement, code) in [
        (
            "/work_item/repository",
            json!("other/repo"),
            "PK-PRE-BINDING-MISMATCH",
        ),
        (
            "/work_item/work_item_id",
            json!("other-task"),
            "PK-PRE-BINDING-MISMATCH",
        ),
        (
            "/pull_request/base_sha",
            json!("f".repeat(40)),
            "PK-PRE-BINDING-MISMATCH",
        ),
        (
            "/context/workflow_run_id",
            json!("other-run"),
            "PK-PRE-WORKFLOW-MISMATCH",
        ),
        (
            "/context/agent_ready/repository",
            json!("other/repo"),
            "PK-PRE-AGENT-READY-BINDING",
        ),
        (
            "/context/agent_ready/work_item_id",
            json!("other-task"),
            "PK-PRE-AGENT-READY-BINDING",
        ),
        (
            "/context/agent_ready/pull_request_number",
            json!(7),
            "PK-PRE-AGENT-READY-BINDING",
        ),
        (
            "/context/agent_ready/head_sha",
            json!("f".repeat(40)),
            "PK-PRE-AGENT-READY-BINDING",
        ),
        (
            "/context/agent_ready/actor",
            json!("stranger"),
            "PK-PRE-AGENT-READY-UNAUTHORIZED",
        ),
        (
            "/context/agent_ready/label",
            json!("ready"),
            "PK-PRE-AGENT-READY-UNAUTHORIZED",
        ),
        (
            "/context/agent_ready/trust",
            json!("local_untrusted"),
            "PK-PROV-UNTRUSTED",
        ),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_result(value, PreMergeStatus::Inconclusive, code);
    }
}

#[test]
fn closed_draft_and_unauthorized_work_are_blocked() {
    for (pointer, replacement, code) in [
        ("/pull_request/state", json!("closed"), "PK-PRE-PR-NOT-OPEN"),
        ("/pull_request/state", json!("merged"), "PK-PRE-PR-NOT-OPEN"),
        ("/pull_request/is_draft", json!(true), "PK-PRE-PR-DRAFT"),
        (
            "/work_item/authorized",
            json!(false),
            "PK-PRE-WORK-UNAUTHORIZED",
        ),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_result(value, PreMergeStatus::Blocked, code);
    }
}

#[test]
fn missing_context_and_legacy_input_fail_closed() {
    let mut value = raw("ready");
    value.as_object_mut().unwrap().remove("context");
    assert_result(
        value,
        PreMergeStatus::Inconclusive,
        "PK-PRE-CONTEXT-MISSING",
    );
    let legacy: PreMergeEvaluationInput = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/valid-pre-merge.json"
    ))
    .unwrap();
    assert_ne!(
        evaluate_pre_merge(&legacy).status,
        PreMergeStatus::ReadyToMerge
    );
}

#[test]
fn missing_and_non_success_evidence_cannot_pass() {
    for (pointer, replacement, status, code) in [
        (
            "/evidence/modifications",
            json!([]),
            PreMergeStatus::Inconclusive,
            "PK-PRE-MODIFICATION-MISSING",
        ),
        (
            "/evidence/executions",
            json!([]),
            PreMergeStatus::Inconclusive,
            "PK-EXEC-MISSING",
        ),
        (
            "/evidence/reviews",
            json!([]),
            PreMergeStatus::Inconclusive,
            "PK-REVIEW-MISSING",
        ),
        (
            "/evidence/sensors",
            json!([]),
            PreMergeStatus::Inconclusive,
            "PK-SENSOR-MISSING",
        ),
        (
            "/evidence/test_adequacy",
            Value::Null,
            PreMergeStatus::Inconclusive,
            "PK-ADEQ-MISSING",
        ),
        (
            "/evidence/executions/0/status",
            json!("failed"),
            PreMergeStatus::Blocked,
            "PK-EXEC-FAILED",
        ),
        (
            "/evidence/executions/0/status",
            json!("timed_out"),
            PreMergeStatus::Inconclusive,
            "PK-EXEC-TIMEOUT",
        ),
        (
            "/evidence/executions/0/status",
            json!("skipped"),
            PreMergeStatus::Inconclusive,
            "PK-EXEC-SKIPPED",
        ),
        (
            "/evidence/executions/0/status",
            json!("unparsable"),
            PreMergeStatus::Inconclusive,
            "PK-EXEC-UNPARSABLE",
        ),
        (
            "/evidence/sensors/0/status",
            json!("failed"),
            PreMergeStatus::Blocked,
            "PK-SENSOR-FAILED",
        ),
        (
            "/evidence/sensors/0/status",
            json!("timed_out"),
            PreMergeStatus::Inconclusive,
            "PK-SENSOR-TIMEOUT",
        ),
        (
            "/evidence/sensors/0/status",
            json!("not_applicable"),
            PreMergeStatus::Inconclusive,
            "PK-SENSOR-NOT-APPLICABLE-INVALID",
        ),
        (
            "/evidence/test_adequacy/acceptance_criteria",
            json!([]),
            PreMergeStatus::Blocked,
            "PK-ADEQ-CRITERION-UNMAPPED",
        ),
        (
            "/evidence/test_adequacy/discovered",
            json!(0),
            PreMergeStatus::Blocked,
            "PK-ADEQ-NO-TESTS",
        ),
        (
            "/evidence/sensors/0/source_digest",
            json!("f".repeat(64)),
            PreMergeStatus::Inconclusive,
            "PK-PRE-SOURCE-MISMATCH",
        ),
        (
            "/evidence/executions/0/trust",
            json!("local_untrusted"),
            PreMergeStatus::Inconclusive,
            "PK-PROV-UNTRUSTED",
        ),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = replacement;
        seal(&mut value);
        assert_result(value, status, code);
    }
}

#[test]
fn tampering_policy_profile_and_transport_digests_is_rejected() {
    for (pointer, replacement, code) in [
        (
            "/policy/max_evidence_age_seconds",
            json!(7200),
            "PK-PRE-POLICY-DIGEST",
        ),
        (
            "/context/sensor_profile/adequacy/acceptance_criterion_ids",
            json!([]),
            "PK-PRE-PROFILE-DIGEST",
        ),
        (
            "/evidence/manifest/child_digests/0/digest",
            json!("f".repeat(64)),
            "PK-PROV-DIGEST-MISMATCH",
        ),
        (
            "/evidence/manifest/policy_digest",
            json!("f".repeat(64)),
            "PK-PRE-BINDING-MISMATCH",
        ),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert_result(value, PreMergeStatus::Inconclusive, code);
    }
}

#[test]
fn runtime_schema_checks_cover_directly_constructed_inputs() {
    let ready = input(raw("ready"));
    for index in 0..9 {
        let mut value = ready.clone();
        *match index {
            0 => &mut value.schema_version,
            1 => &mut value.policy.schema_version,
            2 => &mut value.work_item.schema_version,
            3 => &mut value.pull_request.schema_version,
            4 => &mut value.evidence.schema_version,
            5 => &mut value.evidence.manifest.schema_version,
            6 => &mut value.context.as_mut().unwrap().schema_version,
            7 => {
                &mut value
                    .context
                    .as_mut()
                    .unwrap()
                    .agent_ready
                    .as_mut()
                    .unwrap()
                    .schema_version
            }
            _ => {
                &mut value
                    .context
                    .as_mut()
                    .unwrap()
                    .sensor_profile
                    .schema_version
            }
        } = 99;
        let decision = evaluate_pre_merge(&value);
        assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        assert!(decision
            .reasons
            .iter()
            .any(|r| r.code == "PK-PRE-SCHEMA-UNSUPPORTED"));
    }
}

#[test]
fn timestamps_use_injected_time_without_fractional_future_bypass() {
    for pointer in [
        "/work_item/observed_at",
        "/pull_request/observed_at",
        "/context/agent_ready/observed_at",
        "/evidence/manifest/created_at",
        "/evidence/executions/0/completed_at",
        "/evidence/sensors/0/observed_at",
    ] {
        for time in ["2026-09-07T09:00:00.001Z", "2026-09-07T07:59:59Z"] {
            let mut value = raw("ready");
            *value.pointer_mut(pointer).unwrap() = json!(time);
            seal(&mut value);
            assert_result(value, PreMergeStatus::Inconclusive, "PK-PROV-STALE");
        }
    }
    let mut value = raw("ready");
    value["evidence"]["executions"][0]["started_at"] = json!("2026-09-07T08:59:00Z");
    seal(&mut value);
    assert_result(value, PreMergeStatus::Inconclusive, "PK-EXEC-TIME-INVALID");
}

#[test]
fn conditional_requirements_and_policy_required_union_cannot_be_skipped() {
    let mut value = raw("ready");
    value["policy"]["required_sensor_ids"] = json!(["compile"]);
    value["context"]["sensor_profile"]["sensors"][1]["required"] = json!(false);
    value["context"]["sensor_profile"]["sensors"][1]["required_when"] =
        json!([{"id":"code-change", "risks":["high"], "path_globs":["src/**"]}]);
    value["evidence"]["sensors"].as_array_mut().unwrap().pop();
    seal_policy(&mut value);
    seal(&mut value);
    assert_result(value, PreMergeStatus::Inconclusive, "PK-SENSOR-MISSING");

    let mut value = raw("ready");
    value["context"]["sensor_profile"]["sensors"][1]["required"] = json!(false);
    value["evidence"]["sensors"].as_array_mut().unwrap().pop();
    seal_policy(&mut value);
    seal(&mut value);
    assert_result(value, PreMergeStatus::Inconclusive, "PK-SENSOR-MISSING");
}

#[test]
fn declared_not_applicable_condition_uses_actual_change_context() {
    let mut value = raw("ready");
    value["context"]["sensor_profile"]["sensors"][0]["not_applicable_when"] =
        json!([{"id":"docs", "risks":["low"], "path_globs":["docs/**"]}]);
    value["context"]["risk"] = json!("low");
    value["context"]["changed_paths"] = json!(["docs/readme.md"]);
    value["evidence"]["sensors"][0]["status"] = json!("not_applicable");
    value["evidence"]["sensors"][0]["not_applicable_condition_id"] = json!("docs");
    value["evidence"]["sensors"][0]["not_applicable_reason"] = json!("documentation only");
    seal_policy(&mut value);
    seal(&mut value);
    assert_eq!(
        evaluate_pre_merge(&input(value.clone())).status,
        PreMergeStatus::ReadyToMerge
    );
    value["context"]["changed_paths"] = json!(["src/lib.rs"]);
    assert_result(
        value,
        PreMergeStatus::Inconclusive,
        "PK-SENSOR-NOT-APPLICABLE-INVALID",
    );
}

#[test]
fn blocking_review_retains_other_gate_reasons_and_waiver_cannot_skip_sensors() {
    let mut value = raw("ready");
    value["evidence"]["findings"] = json!([{"id":"F-1", "severity":"high", "summary":"vulnerability", "reviewer_execution_id":"review-b"}]);
    value["context"]["agent_ready"] = Value::Null;
    seal(&mut value);
    let decision = evaluate_pre_merge(&input(value));
    assert_eq!(decision.status, PreMergeStatus::Blocked);
    for code in ["PK-REVIEW-HIGH", "PK-PRE-AGENT-READY-MISSING"] {
        assert!(decision.reasons.iter().any(|r| r.code == code));
    }

    let mut value = raw("ready");
    value["evidence"]["findings"] = json!([{"id":"F-1", "severity":"high", "summary":"vulnerability", "reviewer_execution_id":"review-b"}]);
    seal(&mut value);
    let target = pre_merge_evidence_digest(&input(value.clone()).evidence).unwrap();
    value["evidence"]["waivers"] = json!([{"actor":"release-owner", "finding_id":"F-1", "repository":"FlitCatz/policy-kernel", "head_sha":"c".repeat(40), "evidence_digest":target, "reason":"accepted for this head", "issued_at":"2026-09-07T08:59:00Z", "expires_at":"2026-09-07T10:00:00Z"}]);
    seal(&mut value);
    let decision = evaluate_pre_merge(&input(value.clone()));
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{:?}",
        decision.reasons
    );
    value["evidence"]["waivers"][0]["reason"] = json!("different justification");
    seal(&mut value);
    assert_ne!(
        decision.decision_digest,
        evaluate_pre_merge(&input(value.clone())).decision_digest
    );
    value["evidence"]["sensors"].as_array_mut().unwrap().pop();
    seal(&mut value);
    let target = pre_merge_evidence_digest(&input(value.clone()).evidence).unwrap();
    value["evidence"]["waivers"][0]["evidence_digest"] = json!(target);
    seal(&mut value);
    assert_result(value, PreMergeStatus::Inconclusive, "PK-SENSOR-MISSING");
}

#[test]
fn repeated_and_permuted_inputs_produce_identical_decision_bytes_and_digest() {
    let value = raw("ready");
    let expected = evaluate_pre_merge(&input(value.clone()));
    let bytes = serde_json::to_vec(&expected).unwrap();
    for _ in 0..100 {
        assert_eq!(
            serde_json::to_vec(&evaluate_pre_merge(&input(value.clone()))).unwrap(),
            bytes
        );
    }
    let mut permuted = value;
    for path in ["reviews", "sensors"] {
        permuted["evidence"][path].as_array_mut().unwrap().reverse();
    }
    seal(&mut permuted);
    permuted["evidence"]["manifest"]["child_digests"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_eq!(evaluate_pre_merge(&input(permuted)), expected);
}

#[test]
fn decision_digest_binds_external_observations_even_when_status_is_unchanged() {
    let original = evaluate_pre_merge(&input(raw("ready")));
    for (pointer, replacement) in [
        ("/work_item/observed_at", json!("2026-09-07T08:59:00Z")),
        ("/context/changed_paths", json!(["src/other.rs"])),
        ("/evaluation_time", json!("2026-09-07T09:00:01Z")),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = replacement;
        let decision = evaluate_pre_merge(&input(value));
        assert_eq!(decision.status, PreMergeStatus::ReadyToMerge);
        assert_ne!(decision.decision_digest, original.decision_digest);
    }
}

#[test]
fn freshness_accepts_exact_boundary_but_rejects_one_nanosecond_beyond() {
    for pointer in [
        "/work_item/observed_at",
        "/pull_request/observed_at",
        "/context/agent_ready/observed_at",
        "/evidence/manifest/created_at",
        "/evidence/executions/0/started_at",
        "/evidence/sensors/0/observed_at",
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = json!("2026-09-07T08:00:00Z");
        seal(&mut value);
        assert_eq!(
            evaluate_pre_merge(&input(value.clone())).status,
            PreMergeStatus::ReadyToMerge,
            "{pointer}"
        );
        *value.pointer_mut(pointer).unwrap() = json!("2026-09-07T07:59:59.999999999Z");
        seal(&mut value);
        assert_result(value, PreMergeStatus::Inconclusive, "PK-PROV-STALE");
    }
}

#[test]
fn digest_protocol_matches_independently_computed_sha256_vectors() {
    // Calculated independently with Node crypto over the documented canonical
    // domain envelopes, not with the production projection or hashing helpers.
    let decision = evaluate_pre_merge(&input(raw("ready")));
    assert_eq!(
        decision.binding.evidence_digest.as_str(),
        "04d6090f0015e390509a2d6e8f13a93e6ecdbc63ef866b2a836c5e4602a2e545"
    );
    assert_eq!(
        decision.decision_digest.as_str(),
        "e79c762fca2f2293678791aab60758615525b169464925f67028e5bd9054315d"
    );
}

#[test]
fn permutation_never_repairs_invalid_raw_integrity_or_changes_blocking_reasons() {
    let mut value = raw("ready");
    value["evidence"]["reviews"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_result(
        value,
        PreMergeStatus::Inconclusive,
        "PK-PROV-DIGEST-MISMATCH",
    );

    let mut value = raw("ready");
    value["evidence"]["findings"] = json!([
        {"id":"F-1", "severity":"high", "summary":"vulnerability", "reviewer_execution_id":"review-b"},
        {"id":"F-1", "severity":"low", "summary":"conflicting report", "reviewer_execution_id":"review-c"}
    ]);
    value["evidence"]["sensors"][0]["status"] = json!("timed_out");
    seal(&mut value);
    let expected = evaluate_pre_merge(&input(value.clone()));
    assert_eq!(expected.status, PreMergeStatus::Blocked);
    for code in [
        "PK-REVIEW-HIGH",
        "PK-REVIEW-MEDIUM-CONFLICT",
        "PK-SENSOR-TIMEOUT",
    ] {
        assert!(expected.reasons.iter().any(|r| r.code == code));
    }
    for field in ["findings", "reviews", "sensors"] {
        value["evidence"][field].as_array_mut().unwrap().reverse();
    }
    seal(&mut value);
    assert_eq!(evaluate_pre_merge(&input(value)), expected);
}

#[test]
fn aggregate_gate_preserves_contributor_and_execution_independence() {
    for (pointer, replacement, code) in [
        (
            "/evidence/reviews/0/provider",
            "producer-a",
            "PK-SELF-PROVIDER",
        ),
        (
            "/evidence/reviews/1/provider",
            "reviewer-b",
            "PK-PROV-DUPLICATE-PROVIDER",
        ),
        (
            "/evidence/executions/0/execution_id",
            "modify-a",
            "PK-PROV-DUPLICATE-EXECUTION",
        ),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = json!(replacement);
        seal(&mut value);
        assert_result(value, PreMergeStatus::Inconclusive, code);
    }
}

#[test]
fn legacy_absence_of_profile_binding_does_not_authorize_new_context() {
    let mut value = raw("ready");
    value["policy"]
        .as_object_mut()
        .unwrap()
        .remove("sensor_profile_digest");
    let mut policy = value["policy"].clone();
    policy.as_object_mut().unwrap().remove("policy_digest");
    value["policy"]["policy_digest"] = json!(canonical_sha256(&policy).unwrap());
    value["evidence"]["manifest"]["policy_digest"] = value["policy"]["policy_digest"].clone();
    assert_result(value, PreMergeStatus::Inconclusive, "PK-PRE-PROFILE-DIGEST");
}

#[test]
fn typed_context_and_agent_observation_reject_unsupported_or_unknown_wire_fields() {
    for (pointer, replacement) in [
        ("/context/schema_version", json!(99)),
        ("/context/agent_ready/schema_version", json!(99)),
    ] {
        let mut value = raw("ready");
        *value.pointer_mut(pointer).unwrap() = replacement;
        assert!(serde_json::from_value::<PreMergeEvaluationInput>(value).is_err());
    }
    for pointer in ["/context", "/context/agent_ready"] {
        let mut value = raw("ready");
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(serde_json::from_value::<PreMergeEvaluationInput>(value).is_err());
    }
    let value = input(raw("ready"));
    assert_eq!(
        serde_json::from_slice::<PreMergeEvaluationInput>(&serde_json::to_vec(&value).unwrap())
            .unwrap(),
        value
    );
}
