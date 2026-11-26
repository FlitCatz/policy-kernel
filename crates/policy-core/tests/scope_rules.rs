mod support;

use policy_contracts::{PreMergeEvaluationInput, PreMergeStatus, Reason, SensorId};
use policy_core::{
    canonical_sha256, evaluate_pre_merge, pre_merge_policy_digest, resolve_required_scope,
};
use serde_json::{json, Value};
use support::{fixture_value, pre_merge_input, ready_value, seal_evidence};

fn scope_input(paths: Value, labels: Value) -> Value {
    let mut input = ready_value();
    input["policy"]["schema_version"] = json!(5);
    input["policy"]["required_sensor_ids"] = json!([]);
    input["policy"]["scope_rules"] = json!({
        "default_required_sensor_ids": ["compile"],
        "rules": [
            {"when": "paths", "path_globs": ["src/**"], "required_sensor_ids": ["compile"]},
            {"when": "label", "label": "run-ci", "required_sensor_ids": ["tests"]}
        ]
    });
    input["context"]["changed_paths"] = paths;
    input["context"]["labels"] = labels;
    input["context"].as_object_mut().unwrap().remove("risk");
    for sensor in input["context"]["sensor_profile"]["sensors"]
        .as_array_mut()
        .unwrap()
    {
        sensor["required"] = json!(false);
    }
    seal_policy(&mut input);
    input
}

fn seal_policy(input: &mut Value) {
    input["policy"]["sensor_profile_digest"] =
        json!(canonical_sha256(&input["context"]["sensor_profile"]).unwrap());
    let mut preimage = input["policy"].clone();
    preimage.as_object_mut().unwrap().remove("policy_digest");
    let digest = json!(canonical_sha256(&preimage).unwrap());
    input["policy"]["policy_digest"] = digest.clone();
    input["evidence"]["manifest"]["policy_digest"] = digest;
}

fn retain_sensor(input: &mut Value, id: &str) {
    input["evidence"]["sensors"]
        .as_array_mut()
        .unwrap()
        .retain(|sensor| sensor["sensor_id"] == id);
    seal_evidence(input);
}

#[test]
fn label_widens_a_path_matched_pre_merge_scope_without_caller_risk() {
    let mut input = scope_input(json!(["src/nested/lib.rs"]), json!([]));
    retain_sensor(&mut input, "compile");
    let unlabeled = pre_merge_input(input.clone());
    let decision = evaluate_pre_merge(&unlabeled);
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{:?}",
        decision.reasons
    );

    input["context"]["labels"] = json!(["run-ci"]);
    let labeled = pre_merge_input(input.clone());
    let decision = evaluate_pre_merge(&labeled);
    assert_eq!(decision.status, PreMergeStatus::Inconclusive);
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "PK-SENSOR-MISSING"));

    let mut complete = scope_input(json!(["src/nested/lib.rs"]), json!(["run-ci"]));
    assert_eq!(
        evaluate_pre_merge(&pre_merge_input(complete.clone())).status,
        PreMergeStatus::ReadyToMerge
    );
    retain_sensor(&mut complete, "tests");
    assert_eq!(
        evaluate_pre_merge(&pre_merge_input(complete)).status,
        PreMergeStatus::Inconclusive
    );
}

#[test]
fn mixed_case_label_requires_the_declared_checks() {
    let mut input = scope_input(json!(["src/lib.rs"]), json!([]));
    let mut compile_only = input.clone();
    retain_sensor(&mut compile_only, "compile");
    assert_scope_input_ready(&compile_only);

    input["context"]["labels"] = json!(["Run-CI"]);
    let parsed = pre_merge_input(input.clone());
    let complete = evaluate_pre_merge(&parsed);
    assert_eq!(complete.status, PreMergeStatus::ReadyToMerge);
    let serialized = serde_json::to_value(parsed).unwrap();
    assert_eq!(
        serialized["policy"]["scope_rules"]["rules"][1]["label"],
        "run-ci"
    );
    assert_eq!(serialized["context"]["labels"], json!(["Run-CI"]));

    compile_only["context"]["labels"] = json!(["Run-CI"]);
    let labeled = pre_merge_input(compile_only);
    let decision = evaluate_pre_merge(&labeled);
    assert_eq!(decision.status, PreMergeStatus::Inconclusive);
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason.code == "PK-SENSOR-MISSING"));
    let context = labeled.context.unwrap();
    assert_eq!(
        resolve_required_scope(
            &labeled.policy,
            &context.sensor_profile,
            &context.changed_paths,
            context.labels.as_deref().unwrap(),
        )
        .unwrap(),
        [
            SensorId::parse("compile").unwrap(),
            SensorId::parse("tests").unwrap()
        ]
    );
}

#[test]
fn required_scope_unions_path_and_label_rules_without_losing_the_baseline() {
    let mut value = scope_input(json!([]), json!([]));
    value["policy"]["required_sensor_ids"] = json!(["baseline"]);
    value["policy"]["scope_rules"]["rules"] = json!([
        {"when":"label", "label":"run-ci", "required_sensor_ids":["tests","baseline"]},
        {"when":"paths", "path_globs":["src/*"], "required_sensor_ids":["tests"]},
        {"when":"paths", "path_globs":["config/**","src/**"], "required_sensor_ids":["lint"]}
    ]);
    let profile = value["context"]["sensor_profile"]["sensors"]
        .as_array_mut()
        .unwrap();
    profile.push(json!({"id":"baseline","required":false,"required_when":[]}));
    profile.push(json!({"id":"lint","required":false,"required_when":[]}));
    let input = pre_merge_input(value);
    let profile = &input.context.unwrap().sensor_profile;

    for (paths, labels, expected) in [
        (&["notes.md"][..], &[][..], &["baseline", "compile"][..]),
        (
            &["notes.md"][..],
            &["unrelated", "prüfung"][..],
            &["baseline", "compile"][..],
        ),
        (
            &["notes.md"][..],
            &["run-ci"][..],
            &["baseline", "compile", "tests"][..],
        ),
        (
            &["src/nested/main.rs"][..],
            &[][..],
            &["baseline", "lint"][..],
        ),
        (
            &["src/main.rs", "config/nested/file.yml"][..],
            &[][..],
            &["baseline", "lint", "tests"][..],
        ),
        (
            &["src/main.rs", "notes.md"][..],
            &[][..],
            &["baseline", "compile", "lint", "tests"][..],
        ),
        (
            &["config/main.yml"][..],
            &["run-ci"][..],
            &["baseline", "lint", "tests"][..],
        ),
        (&[][..], &[][..], &["baseline", "compile"][..]),
    ] {
        let paths: Vec<_> = paths.iter().map(ToString::to_string).collect();
        let labels: Vec<_> = labels.iter().map(ToString::to_string).collect();
        let actual = resolve_required_scope(&input.policy, profile, &paths, &labels).unwrap();
        let expected: Vec<_> = expected
            .iter()
            .map(|id| SensorId::parse(*id).unwrap())
            .collect();
        assert_eq!(actual, expected, "{paths:?} with {labels:?}");
    }
}

#[test]
fn missing_or_unreadable_scope_observations_withhold_readiness() {
    let valid = scope_input(json!(["src/lib.rs"]), json!([]));
    assert_scope_input_ready(&valid);
    for (field, unreadable) in [
        ("labels", json!("run-ci")),
        ("labels", json!([null])),
        ("labels", json!(["", "run-ci"])),
        ("labels", Value::Null),
        ("changed_paths", Value::Null),
        ("changed_paths", json!([null])),
        ("changed_paths", json!(["src/file.rs", "src/../missing.rs"])),
        ("changed_paths", json!(["src/file.rs", "/outside.rs"])),
        ("changed_paths", json!(["src/file.rs", ""])),
    ] {
        let mut input = valid.clone();
        input["context"][field] = unreadable;
        assert_scope_input_rejected(input);
    }
    for field in ["changed_paths", "labels"] {
        let mut input = valid.clone();
        input["context"].as_object_mut().unwrap().remove(field);
        assert_scope_input_rejected(input);
    }
}

#[test]
fn invalid_inactive_rules_cannot_silently_shrink_the_scope() {
    let valid = scope_input(json!(["docs/readme.md"]), json!([]));
    assert_scope_input_ready(&valid);
    for broken_rule in [
        json!({"when":"paths","path_globs":["src/**","src/[a-z].rs"],"required_sensor_ids":["tests"]}),
        json!({"when":"label","label":"run-ci","required_sensor_ids":["undefined-check"]}),
    ] {
        let mut input = valid.clone();
        input["policy"]["scope_rules"]["rules"][0] = broken_rule;
        seal_policy(&mut input);
        assert_scope_input_rejected(input);
    }
}

fn assert_scope_input_ready(input: &Value) {
    let decision = evaluate_pre_merge(&pre_merge_input(input.clone()));
    assert_eq!(
        decision.status,
        PreMergeStatus::ReadyToMerge,
        "{decision:?}"
    );
}

fn assert_scope_input_rejected(input: Value) {
    if let Ok(parsed) = serde_json::from_value::<PreMergeEvaluationInput>(input) {
        let decision = evaluate_pre_merge(&parsed);
        assert_eq!(decision.status, PreMergeStatus::Inconclusive);
        assert!(decision
            .reasons
            .iter()
            .any(|reason| reason.code == "PK-PRE-SCOPE-INVALID"));
    }
}

#[test]
fn caller_risk_cannot_narrow_inherited_requirements() {
    let mut input = scope_input(json!(["src/lib.rs"]), json!([]));
    let mut compile_only = input.clone();
    retain_sensor(&mut compile_only, "compile");
    assert_scope_input_ready(&compile_only);

    input["context"]["sensor_profile"]["sensors"][1]["required_when"] = json!([
        {"id":"high", "risks":["high"], "path_globs":["src/**"]}
    ]);
    seal_policy(&mut input);
    let mut missing_tests = input.clone();
    retain_sensor(&mut missing_tests, "compile");
    let without_risk = evaluate_pre_merge(&pre_merge_input(missing_tests.clone()));
    missing_tests["context"]["risk"] = json!("low");
    let with_risk = evaluate_pre_merge(&pre_merge_input(missing_tests));
    assert_eq!(without_risk.status, PreMergeStatus::Inconclusive);
    assert_eq!(with_risk.status, without_risk.status);
    assert_eq!(with_risk.reasons, without_risk.reasons);
    assert_eq!(
        with_risk.reasons,
        [Reason {
            code: "PK-SENSOR-MISSING".into(),
            message: "required sensor evidence is missing".into(),
        }]
    );
    input["context"]["risk"] = json!("low");
    assert_scope_input_ready(&input);
}

#[test]
fn labels_leave_legacy_verdicts_unchanged() {
    for version in 1..=4 {
        let mut input = ready_value();
        input["policy"]["schema_version"] = json!(version);
        if version < 3 {
            input["policy"]
                .as_object_mut()
                .unwrap()
                .remove("required_reviews");
        }
        seal_policy(&mut input);
        let baseline = evaluate_pre_merge(&pre_merge_input(input.clone()));
        if version < 3 {
            assert!(baseline
                .reasons
                .iter()
                .any(|reason| reason.code == "PK-POLICY-V3-MIGRATION-REQUIRED"));
        } else {
            assert_eq!(
                baseline.status,
                PreMergeStatus::ReadyToMerge,
                "{baseline:?}"
            );
        }

        input["context"]["labels"] = json!(["Run-CI"]);
        let labeled = evaluate_pre_merge(&pre_merge_input(input));
        assert_eq!(labeled.status, baseline.status, "policy v{version}");
        assert_eq!(labeled.reasons, baseline.reasons, "policy v{version}");
        assert_eq!(labeled.binding, baseline.binding, "policy v{version}");
        // The decision digest still binds the exact supplied context.
        assert_ne!(labeled.decision_digest, baseline.decision_digest);
    }
}

#[test]
fn legacy_scope_documents_and_decisions_keep_their_bytes() {
    let v1 = fixture_value("tests/fixtures/pre-merge/legacy-ready.json");
    let v3 = ready_value();
    let mut v4 = ready_value();
    v4["policy"]["schema_version"] = json!(4);
    v4["policy"]["local_runner_evidence"] = json!({
        "allowed_runner_ids": ["owner-runner"],
        "approved_scope_template_digests": ["a".repeat(64)],
        "maximum_age_seconds": 300,
        "ci_required_when": [{"id":"ci-path", "risks":["high"], "path_globs":["src/**"]}]
    });
    seal_policy(&mut v4);
    // Captured from the unmodified evaluator at the default-branch base.
    for (value, status, policy_digest, evidence_digest, decision_digest) in [
        (
            v1,
            PreMergeStatus::Inconclusive,
            "63fd019c05bc9d002a803fbf5187ab59f2316e012c421c13d4220a780e090e46",
            "4683ca506ea94a31512cad6be570ea7891be4dedaf8a291a2144c79b7fb1158c",
            "9b679c8e83c93e7b7ba262d7a00facdfcba34b126f10ae9b56b350b263b04c50",
        ),
        (
            v3,
            PreMergeStatus::ReadyToMerge,
            "9d8ed57f0955bfd8916cf6a655a7cdaf32e794909e28e1fd0f6dc595c47f21c8",
            "cb66737163db53bd1b13a3486c785c17cff72d0eb6c395ef594424f3172e4a85",
            "f7f6795dc882e5eaaef504ae423492dc54f4ac92070c28757aaf94a0d93414c4",
        ),
        (
            v4,
            PreMergeStatus::ReadyToMerge,
            "9346c355027f2ae3b310be04b47618247174c6731aaec0eb5db5d6164d6b8c72",
            "95a3bf2fcb4ec26251ca84dd24bcb2ac79507077916058a13788be97304d92a5",
            "afb22d95c9c4da454146228dceef4a70bf93641ae3052c503372da49d10d37d0",
        ),
    ] {
        let input = pre_merge_input(value.clone());
        let decision = evaluate_pre_merge(&input);
        assert_eq!(serde_json::to_value(&input).unwrap(), value);
        assert_eq!(decision.status, status);
        assert_eq!(
            pre_merge_policy_digest(&input.policy).unwrap().as_str(),
            policy_digest
        );
        assert_eq!(decision.binding.policy_digest.as_str(), policy_digest);
        assert_eq!(decision.binding.evidence_digest.as_str(), evidence_digest);
        assert_eq!(decision.decision_digest.as_str(), decision_digest);
    }
}
