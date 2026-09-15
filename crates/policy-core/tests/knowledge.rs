mod support;

use policy_contracts::*;
use policy_core::{canonical_sha256, evaluate_pre_merge, pre_merge_policy_digest};
use serde_json::{json, Value};
use support::{pre_merge_input, ready_value, seal_evidence};

fn requirement() -> Value {
    json!({"kind": "code_graph_index", "source": "ci/codegraph", "required_when": []})
}

fn seal_policy(value: &mut Value) {
    let mut policy = value["policy"].clone();
    policy.as_object_mut().unwrap().remove("policy_digest");
    value["policy"]["policy_digest"] = json!(canonical_sha256(&policy).unwrap());
    value["evidence"]["manifest"]["policy_digest"] = value["policy"]["policy_digest"].clone();
}

fn required() -> Value {
    let mut value = ready_value();
    value["policy"]["schema_version"] = json!(2);
    value["policy"]["required_knowledge"] = json!([requirement()]);
    value["evidence"]["knowledge"] = json!([{
        "kind": "code_graph_index", "source": "ci/codegraph", "version": "1.7.0",
        "configuration_digest": "1".repeat(64), "content_digest": "2".repeat(64),
        "source_set_digest": value["evidence"]["manifest"]["source_set_digest"],
        "head_sha": value["evidence"]["manifest"]["head_sha"], "trust": "ci_observed"
    }]);
    seal_policy(&mut value);
    seal_evidence(&mut value);
    value
}

fn assert_ready(value: Value) -> PreMergeDecision {
    let decision = evaluate_pre_merge(&pre_merge_input(value));
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{:?}",
        decision.reasons
    );
    assert!(decision.reasons.is_empty());
    decision
}

fn assert_reason(mut value: Value, reason: &str) {
    seal_evidence(&mut value);
    let decision = evaluate_pre_merge(&pre_merge_input(value));
    assert_eq!(
        decision.status,
        PreMergeStatus::Inconclusive,
        "{:?}",
        decision.reasons
    );
    assert!(
        decision.reasons.iter().any(|item| item.code == reason),
        "{reason}: {:?}",
        decision.reasons
    );
}

#[test]
fn knowledge_is_optional_for_legacy_and_undeclared_v2_policies() {
    let mut legacy = ready_value();
    legacy["evidence"]["knowledge"] = json!([]);
    seal_evidence(&mut legacy);
    let parsed = pre_merge_input(legacy.clone());
    assert_eq!(
        pre_merge_policy_digest(&parsed.policy).unwrap(),
        parsed.policy.policy_digest
    );
    assert_ready(legacy.clone());
    legacy["policy"]["schema_version"] = json!(2);
    seal_policy(&mut legacy);
    assert_ready(legacy);
}

#[test]
fn required_knowledge_accepts_ci_and_attested_bindings() {
    for mode in ["authoritative", "advisory"] {
        for trust in ["ci_observed", "trusted_runner_attested"] {
            let mut value = required();
            value["mode"] = json!(mode);
            value["evidence"]["knowledge"][0]["trust"] = json!(trust);
            seal_evidence(&mut value);
            assert_ready(value);
        }
    }
}

#[test]
fn knowledge_identities_are_kind_source_pairs_and_diagnostics_distinguish_absence() {
    for (kind, source) in [
        ("code_graph_index", "ci/other-index"),
        ("lat_claim", "ci/codegraph"),
    ] {
        let mut value = required();
        value["policy"]["required_knowledge"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":kind,"source":source,"required_when":[]}));
        let mut observation = value["evidence"]["knowledge"][0].clone();
        observation["kind"] = json!(kind);
        observation["source"] = json!(source);
        value["evidence"]["knowledge"]
            .as_array_mut()
            .unwrap()
            .push(observation);
        seal_policy(&mut value);
        seal_evidence(&mut value);
        assert_ready(value);
    }
    let mut value = required();
    value["evidence"]["knowledge"][0]["kind"] = json!("lat_claim");
    seal_evidence(&mut value);
    let decision = evaluate_pre_merge(&pre_merge_input(value));
    assert!(decision
        .reasons
        .iter()
        .any(|r| r.code == "PK-KNOWLEDGE-KIND-MISMATCH"));
    assert!(!decision
        .reasons
        .iter()
        .any(|r| r.code == "PK-KNOWLEDGE-SOURCE-MISMATCH"));
    let mut value = required();
    value["evidence"]["knowledge"][0]["source"] = json!("ci/other-index");
    seal_evidence(&mut value);
    let decision = evaluate_pre_merge(&pre_merge_input(value));
    assert!(decision
        .reasons
        .iter()
        .any(|r| r.code == "PK-KNOWLEDGE-SOURCE-MISMATCH"));
    assert!(!decision
        .reasons
        .iter()
        .any(|r| r.code == "PK-KNOWLEDGE-KIND-MISMATCH"));
}

#[test]
fn required_knowledge_failures_have_distinct_reasons() {
    let mut missing = required();
    missing["evidence"]["knowledge"] = json!([]);
    assert_reason(missing, "PK-KNOWLEDGE-MISSING");
    for (field, replacement, reason) in [
        ("kind", json!("lat_claim"), "PK-KNOWLEDGE-KIND-MISMATCH"),
        (
            "source",
            json!("local/codegraph"),
            "PK-KNOWLEDGE-SOURCE-MISMATCH",
        ),
        (
            "head_sha",
            json!("9".repeat(40)),
            "PK-KNOWLEDGE-HEAD-MISMATCH",
        ),
        (
            "source_set_digest",
            json!("9".repeat(64)),
            "PK-KNOWLEDGE-SOURCE-SET-MISMATCH",
        ),
        ("trust", json!("local_untrusted"), "PK-KNOWLEDGE-UNTRUSTED"),
        ("version", json!(" \t"), "PK-KNOWLEDGE-METADATA-INVALID"),
        (
            "version",
            json!("version\0"),
            "PK-KNOWLEDGE-METADATA-INVALID",
        ),
    ] {
        for mode in ["authoritative", "advisory"] {
            let mut value = required();
            value["mode"] = json!(mode);
            value["evidence"]["knowledge"][0][field] = replacement.clone();
            assert_reason(value, reason);
        }
    }
}

#[test]
fn knowledge_conditions_use_current_paths_and_risk() {
    for (paths, risk, applies) in [
        (json!(["src/main.rs"]), "high", true),
        (json!(["./src\\main.rs"]), "high", true),
        (json!(["docs/guide.md"]), "high", false),
        (json!(["src/main.rs"]), "low", false),
    ] {
        let mut value = required();
        value["policy"]["required_knowledge"][0]["required_when"] =
            json!([{"risks": ["high"], "path_globs": ["src/**"]}]);
        value["context"]["changed_paths"] = paths;
        value["context"]["risk"] = json!(risk);
        value["evidence"]["knowledge"] = json!([]);
        seal_policy(&mut value);
        seal_evidence(&mut value);
        if applies {
            assert_reason(value, "PK-KNOWLEDGE-MISSING");
        } else {
            assert_ready(value);
        }
    }
}

#[test]
fn invalid_conditions_and_ambiguous_evidence_fail_closed() {
    let mut value = required();
    value["policy"]["required_knowledge"][0]["required_when"] =
        json!([{"risks": [], "path_globs": ["["]}]);
    seal_policy(&mut value);
    assert_reason(value, "PK-KNOWLEDGE-POLICY-INVALID");
    let mut value = required();
    value["context"]["changed_paths"] = json!(["src/../other.rs"]);
    assert_reason(value, "PK-KNOWLEDGE-CONTEXT-INVALID");
    let mut value = required();
    let duplicate = value["evidence"]["knowledge"][0].clone();
    value["evidence"]["knowledge"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert_reason(value, "PK-KNOWLEDGE-DUPLICATE");
    let mut value = required();
    value["policy"]["required_knowledge"]
        .as_array_mut()
        .unwrap()
        .push(requirement());
    seal_policy(&mut value);
    assert_reason(value, "PK-KNOWLEDGE-POLICY-INVALID");
}

#[test]
fn knowledge_requirements_bind_policy_and_decision_and_reject_prior_evidence() {
    let ready = assert_ready(required());
    for (field, replacement) in [
        ("source", json!("other/source")),
        ("kind", json!("lat_claim")),
        (
            "required_when",
            json!([{"risks": ["critical"], "path_globs": []}]),
        ),
    ] {
        let mut value = required();
        let old_policy = value["evidence"]["manifest"]["policy_digest"].clone();
        value["policy"]["required_knowledge"][0][field] = replacement;
        seal_policy(&mut value);
        assert_ne!(value["policy"]["policy_digest"], old_policy);
        value["evidence"]["manifest"]["policy_digest"] = old_policy;
        let decision = evaluate_pre_merge(&pre_merge_input(value.clone()));
        assert_ne!(decision.decision_digest, ready.decision_digest);
        assert_reason(value, "PK-PRE-BINDING-MISMATCH");
    }
}

#[test]
fn knowledge_does_not_replace_reviews_or_tests() {
    let mut value = required();
    value["evidence"]["reviews"] = json!([]);
    seal_evidence(&mut value);
    assert_ne!(
        evaluate_pre_merge(&pre_merge_input(value)).status,
        PreMergeStatus::ReadyToMerge
    );
    let mut value = required();
    value["evidence"]["test_adequacy"] = Value::Null;
    assert_reason(value, "PK-ADEQ-MISSING");
}

#[test]
fn alternate_conditions_and_multiple_requirements_are_all_enforced() {
    let mut value = required();
    value["policy"]["required_knowledge"][0]["required_when"] = json!([
        {"risks": ["critical"], "path_globs": ["unrelated/**"]},
        {"risks": [], "path_globs": []}
    ]);
    value["evidence"]["knowledge"] = json!([]);
    seal_policy(&mut value);
    assert_reason(value, "PK-KNOWLEDGE-MISSING");
    let mut value = required();
    value["policy"]["required_knowledge"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "kind": "lat_claim", "source": "ci/lat", "required_when": []
        }));
    let mut lat = value["evidence"]["knowledge"][0].clone();
    lat["kind"] = json!("lat_claim");
    lat["source"] = json!("ci/lat");
    value["evidence"]["knowledge"]
        .as_array_mut()
        .unwrap()
        .push(lat);
    seal_policy(&mut value);
    seal_evidence(&mut value);
    assert_ready(value.clone());
    value["evidence"]["knowledge"].as_array_mut().unwrap().pop();
    assert_reason(value, "PK-KNOWLEDGE-KIND-MISMATCH");
}

#[test]
fn knowledge_content_configuration_and_version_bind_the_decision() {
    let original = assert_ready(required());
    for (field, replacement) in [
        ("content_digest", json!("7".repeat(64))),
        ("configuration_digest", json!("7".repeat(64))),
        ("version", json!("1.7.1")),
    ] {
        let mut value = required();
        value["evidence"]["knowledge"][0][field] = replacement;
        seal_evidence(&mut value);
        let changed = assert_ready(value);
        assert_ne!(
            changed.binding.evidence_digest,
            original.binding.evidence_digest
        );
        assert_ne!(changed.decision_digest, original.decision_digest);
    }
}

#[test]
fn directly_constructed_policies_cannot_bypass_version_or_source_rules() {
    for source in ["", " \t", "ci\0codegraph"] {
        let mut input = pre_merge_input(required());
        input.policy.required_knowledge[0].source = source.into();
        input.policy.policy_digest = pre_merge_policy_digest(&input.policy).unwrap();
        input.evidence.manifest.policy_digest = input.policy.policy_digest.clone();
        let decision = evaluate_pre_merge(&input);
        assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        assert!(decision
            .reasons
            .iter()
            .any(|item| item.code == "PK-KNOWLEDGE-POLICY-INVALID"));
    }
    let mut input = pre_merge_input(required());
    input.policy.schema_version = 1;
    input.policy.policy_digest = pre_merge_policy_digest(&input.policy).unwrap();
    input.evidence.manifest.policy_digest = input.policy.policy_digest.clone();
    assert!(evaluate_pre_merge(&input)
        .reasons
        .iter()
        .any(|item| item.code == "PK-PRE-SCHEMA-UNSUPPORTED"));
    let mut value = required();
    value["context"] = Value::Null;
    assert_reason(value, "PK-KNOWLEDGE-CONTEXT-INVALID");
    for glob in ["", " \t", "src\0/**"] {
        let mut value = required();
        value["policy"]["required_knowledge"][0]["required_when"] =
            json!([{"risks": [], "path_globs": [glob]}]);
        seal_policy(&mut value);
        assert_reason(value, "PK-KNOWLEDGE-POLICY-INVALID");
    }
}
