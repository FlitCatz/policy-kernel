use policy_contracts::*;
use policy_core::*;
use serde_json::{json, Value};

fn raw(path: &str) -> Value {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    serde_json::from_slice(&std::fs::read(root.join(path)).unwrap()).unwrap()
}
fn bundle() -> EvidenceBundle {
    serde_json::from_value(raw("tests/fixtures/evidence/valid.json")).unwrap()
}
fn post() -> PostMergeEvaluationInput {
    serde_json::from_value(raw("tests/fixtures/post-merge/merge.json")).unwrap()
}
fn pre() -> PreMergeEvaluationInput {
    serde_json::from_value(raw("tests/fixtures/pre-merge/ready.json")).unwrap()
}
fn ts(s: &str) -> UtcTimestamp {
    UtcTimestamp::parse(s).unwrap()
}

#[test]
fn canonical_json_accepts_both_signed_and_unsigned_integer_domains() {
    assert!(canonical_sha256(&json!([-1, i64::MIN, u64::MAX])).is_ok());
    assert!(canonical_sha256(&json!([1.5])).is_err());
}

#[test]
fn canonical_json_bytes_sort_nested_keys_and_match_the_digest_preimage() {
    let value = json!({"b": {"d": 1, "c": [true, null]}, "a": "x"});
    let bytes = canonical_json(&value).unwrap();
    assert_eq!(bytes, br#"{"a":"x","b":{"c":[true,null],"d":1}}"#);
    let digest = <sha2::Sha256 as sha2::Digest>::digest(&bytes);
    assert_eq!(
        canonical_sha256(&value).unwrap().as_str(),
        hex::encode(digest)
    );
    assert!(canonical_json(&json!({"a": [0.5]})).is_err());
}

#[test]
fn provenance_freshness_preserves_nanosecond_boundaries() {
    let now = ts("2026-09-04T09:00:00Z");
    for (observed, stale) in [
        ("2026-09-04T09:00:00Z", false),
        ("2026-09-04T08:00:00Z", false),
        ("2026-09-04T09:00:00.000000001Z", true),
        ("2026-09-04T07:59:59.999999999Z", true),
    ] {
        let mut value = bundle();
        value.manifest.created_at = ts(observed);
        let result = validate_provenance(EvaluationMode::Authoritative, &value, &now, 3600);
        assert_eq!(
            result.reason_codes().contains(&"PK-PROV-STALE"),
            stale,
            "{observed}"
        );
    }
}

#[test]
fn review_freshness_preserves_nanosecond_boundaries() {
    for (observed, stale) in [
        ("2026-09-04T09:00:00Z", false),
        ("2026-09-04T08:00:00Z", false),
        ("2026-09-04T09:00:00.000000001Z", true),
        ("2026-09-04T07:59:59.999999999Z", true),
    ] {
        let mut value: ReviewEvaluationInput =
            serde_json::from_value(raw("tests/fixtures/reviews/approved.json")).unwrap();
        value.reviews[0].observed_at = ts(observed);
        let result = evaluate_reviews(value);
        assert_eq!(result.has_reason("PK-PROV-STALE"), stale, "{observed}");
        assert!(!result.has_reason("NO-SUCH-REASON"));
    }
}

#[test]
fn every_individual_post_merge_policy_error_has_a_diagnostic() {
    for (field, replacement) in [
        ("required_check_name", json!(" ")),
        ("max_evidence_age_seconds", json!(0)),
        ("authorized_actors", json!([""])),
        ("protected_target_branches", json!([""])),
        ("authorized_actors", json!(["owner", "owner"])),
        ("protected_target_branches", json!(["main", "main"])),
        ("allowed_merge_methods", json!(["merge", "merge"])),
    ] {
        let mut value = raw("tests/fixtures/post-merge/merge.json");
        value["policy"][field] = replacement;
        let input: PostMergeEvaluationInput = serde_json::from_value(value).unwrap();
        let result = evaluate_post_merge(&input);
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r.code == "PK-POST-POLICY-INVALID"),
            "{field}: {:?}",
            result.reasons
        );
    }
}

#[test]
fn pre_merge_policy_rejects_blank_actors_and_duplicate_sensor_ids() {
    for (field, replacement) in [
        ("required_check_name", json!(" ")),
        ("max_evidence_age_seconds", json!(0)),
        ("authorized_actors", json!([""])),
        ("authorized_waiver_actors", json!([" "])),
        ("required_sensor_ids", json!(["same", "same"])),
    ] {
        let mut value = raw("tests/fixtures/pre-merge/ready.json");
        value["policy"][field] = replacement;
        let input: PreMergeEvaluationInput = serde_json::from_value(value).unwrap();
        let result = evaluate_pre_merge(&input);
        assert!(
            result
                .reasons
                .iter()
                .any(|r| r.code == "PK-PRE-POLICY-INVALID"),
            "{field}"
        );
    }
}

#[test]
fn check_diagnostics_identify_each_broken_link_even_when_another_link_matches() {
    for (field, replacement, code) in [
        (
            "repository",
            json!("other/repo"),
            "PK-POST-REPOSITORY-MISMATCH",
        ),
        ("pull_request_number", json!(999), "PK-POST-PR-MISMATCH"),
        ("head_sha", json!("f".repeat(40)), "PK-POST-HEAD-MISMATCH"),
    ] {
        // The check still matches the authorization, but not the merge observation.
        let mut value = raw("tests/fixtures/post-merge/merge.json");
        value["merge"][field] = replacement;
        let result = evaluate_post_merge(&serde_json::from_value(value).unwrap());
        assert_eq!(
            result.reasons.iter().filter(|r| r.code == code).count(),
            2,
            "{field}"
        );
    }
    for pointer in ["/policy/policy_digest", "/pre_merge/binding/policy_digest"] {
        let mut value = raw("tests/fixtures/post-merge/merge.json");
        *value.pointer_mut(pointer).unwrap() = json!("f".repeat(64));
        let result = evaluate_post_merge(&serde_json::from_value(value).unwrap());
        assert_eq!(
            result
                .reasons
                .iter()
                .filter(|r| r.code == "PK-POST-POLICY-MISMATCH")
                .count(),
            2,
            "{pointer}"
        );
    }
}

#[test]
fn simultaneous_check_start_completion_and_pre_merge_evaluation_are_allowed() {
    let mut input = post();
    let completed = input.check.as_ref().unwrap().completed_at.clone();
    input.check.as_mut().unwrap().started_at = completed.clone();
    let mut pre = input.pre_merge.into_decision();
    pre.binding.evaluated_at = completed;
    input.pre_merge = pre.try_into().unwrap();
    assert_eq!(evaluate_post_merge(&input).status, PostMergeStatus::Shipped);
}

#[test]
fn a_blocked_merge_cannot_be_downgraded_by_a_later_unknown_observation() {
    let mut input = post();
    input.merge.protected = false;
    input.merge.actor_type = None;
    input.merge.via_merge_queue = None;
    assert_eq!(evaluate_post_merge(&input).status, PostMergeStatus::Blocked);
}

#[test]
fn merge_at_evaluation_time_is_fresh_and_not_a_future_merge() {
    let mut input = post();
    input.evaluation_time = input.merge.merged_at.clone();
    assert_eq!(evaluate_post_merge(&input).status, PostMergeStatus::Shipped);
}

#[test]
fn zero_duration_execution_does_not_invent_a_time_error() {
    let mut input = pre();
    input.evidence.executions[0].started_at = input.evidence.executions[0].completed_at.clone();
    let result = evaluate_pre_merge(&input);
    assert!(!result
        .reasons
        .iter()
        .any(|r| r.code == "PK-EXEC-TIME-INVALID"));
}

#[test]
fn policy_sensor_missing_from_profile_is_explicitly_unknown() {
    let mut input = pre();
    input
        .policy
        .required_sensor_ids
        .push(SensorId::parse("absent-sensor").unwrap());
    let result = evaluate_pre_merge(&input);
    assert!(result.reasons.iter().any(|r| r.code == "PK-SENSOR-UNKNOWN"));
}

#[test]
fn bundle_validity_reports_invalid_integrity_and_waiver_bindings() {
    let valid = bundle();
    assert!(validate_bundle(&valid).is_valid());
    let mut invalid = valid.clone();
    invalid.manifest.child_digests.clear();
    assert!(!validate_bundle(&invalid).is_valid());
    for (repository, head) in [
        (
            valid.manifest.repository.clone(),
            valid.manifest.head_sha.clone(),
        ),
        (
            RepositoryId::parse("other/repo").unwrap(),
            valid.manifest.head_sha.clone(),
        ),
        (
            valid.manifest.repository.clone(),
            CommitSha::parse("f".repeat(40)).unwrap(),
        ),
    ] {
        let matching = repository == valid.manifest.repository && head == valid.manifest.head_sha;
        let mut value = valid.clone();
        value.waivers.push(WaiverEvidence {
            actor: "human".into(),
            finding_id: "finding".into(),
            repository,
            head_sha: head,
            evidence_digest: value.manifest.policy_digest.clone(),
            reason: "accepted".into(),
            issued_at: ts("2026-09-04T08:30:00Z"),
            expires_at: ts("2026-09-04T10:00:00Z"),
        });
        let result = validate_bundle(&value);
        assert_eq!(
            result.reason_codes().contains(&"PK-PROV-BINDING-MISMATCH"),
            !matching
        );
    }
}

#[test]
fn trust_is_checked_for_each_evidence_kind_independently() {
    for untrusted in [false, true] {
        for kind in [
            "modifications",
            "executions",
            "reviews",
            "sensors",
            "knowledge",
        ] {
            let mut value = raw("tests/fixtures/pre-merge/ready.json")["evidence"].clone();
            if kind == "knowledge" {
                value["knowledge"] = json!([{
                    "kind": "code_graph_index", "source": "project-index", "version": "1",
                    "configuration_digest": "a".repeat(64), "content_digest": "b".repeat(64),
                    "source_set_digest": value["manifest"]["source_set_digest"],
                    "head_sha": value["manifest"]["head_sha"], "trust": "ci_observed"
                }]);
            }
            value[kind][0]["trust"] = json!(if untrusted {
                "local_untrusted"
            } else {
                "ci_observed"
            });
            let input: EvidenceBundle = serde_json::from_value(value).unwrap();
            let result = validate_provenance(
                EvaluationMode::Authoritative,
                &input,
                &ts("2026-09-04T09:00:00Z"),
                3600,
            );
            assert_eq!(
                result.reason_codes().contains(&"PK-PROV-UNTRUSTED"),
                untrusted,
                "{kind}"
            );
        }
    }
}

#[test]
fn waiver_issued_exactly_at_evaluation_is_valid_until_expiry() {
    let mut input: ReviewEvaluationInput =
        serde_json::from_value(raw("tests/fixtures/reviews/waiver-expired.json")).unwrap();
    input.waivers[0].issued_at = input.evaluation_time.clone();
    input.waivers[0].expires_at = ts("2026-09-04T09:00:01Z");
    let result = evaluate_reviews(input);
    assert_eq!(
        result.status,
        ReviewStatus::Inconclusive,
        "{:?}",
        result.reasons
    );
    assert!(result.has_reason("PK-WAIVER-APPLIED"));
    assert!(result.has_reason("PK-REVIEW-QUORUM"));
}

#[test]
fn waiver_lifetime_must_be_positive_even_when_it_is_already_expired() {
    let mut input: ReviewEvaluationInput =
        serde_json::from_value(raw("tests/fixtures/reviews/waiver-expired.json")).unwrap();
    input.waivers[0].issued_at = input.evaluation_time.clone();
    input.waivers[0].expires_at = input.evaluation_time.clone();
    let result = evaluate_reviews(input);
    assert!(result.has_reason("PK-WAIVER-INVALID"));
    assert!(!result.has_reason("PK-WAIVER-EXPIRED"));
}

#[test]
fn waiver_issuance_must_be_fresh_to_the_nanosecond() {
    for (issued, valid) in [
        ("2026-09-04T08:00:00Z", true),
        ("2026-09-04T07:59:59.999999999Z", false),
        ("2026-09-04T09:00:00.000000001Z", false),
    ] {
        let mut input: ReviewEvaluationInput =
            serde_json::from_value(raw("tests/fixtures/reviews/waiver-expired.json")).unwrap();
        input.waivers[0].issued_at = ts(issued);
        input.waivers[0].expires_at = ts("2026-09-04T10:00:00Z");
        let result = evaluate_reviews(input);
        assert_eq!(result.has_reason("PK-WAIVER-INVALID"), !valid, "{issued}");
    }
}

#[test]
fn knowledge_head_and_source_set_must_each_match_the_manifest() {
    for (head_matches, source_matches) in [(true, true), (false, true), (true, false)] {
        let mut input = bundle();
        input.knowledge.push(KnowledgeEvidence {
            kind: KnowledgeKind::CodeGraphIndex,
            source: "supplied-index".into(),
            version: "1".into(),
            configuration_digest: input.manifest.policy_digest.clone(),
            content_digest: input.manifest.policy_digest.clone(),
            source_set_digest: if source_matches {
                input.manifest.source_set_digest.clone()
            } else {
                Sha256Digest::parse("f".repeat(64)).unwrap()
            },
            head_sha: if head_matches {
                input.manifest.head_sha.clone()
            } else {
                CommitSha::parse("f".repeat(40)).unwrap()
            },
            trust: TrustClass::CiObserved,
        });
        let digest = canonical_sha256(&input.knowledge).unwrap();
        input
            .manifest
            .child_digests
            .iter_mut()
            .find(|child| child.path == "knowledge")
            .unwrap()
            .digest = digest;
        let result = validate_bundle(&input);
        assert_eq!(result.is_valid(), head_matches && source_matches);
        assert_eq!(
            result.reason_codes().contains(&"PK-PROV-BINDING-MISMATCH"),
            !(head_matches && source_matches)
        );
    }
}

#[test]
fn execution_and_adequacy_heads_bind_to_the_manifest() {
    let valid = pre().evidence;
    let codes = |bundle: &EvidenceBundle, mode| {
        validate_provenance(mode, bundle, &ts("2026-09-07T09:00:00Z"), 3_600)
            .reason_codes()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert!(validate_bundle(&valid).is_valid());
    assert!(codes(&valid, EvaluationMode::Authoritative).is_empty());

    let other = CommitSha::parse("d".repeat(40)).unwrap();
    let mut execution = valid.clone();
    execution.executions[0].head_sha = Some(other.clone());
    let mut adequacy = valid.clone();
    adequacy.test_adequacy.as_mut().unwrap().head_sha = Some(other);
    for bundle in [&execution, &adequacy] {
        assert!(validate_bundle(bundle)
            .reason_codes()
            .contains(&"PK-PROV-BINDING-MISMATCH"));
    }

    let mut unbound_execution = valid.clone();
    unbound_execution.executions[0].head_sha = None;
    let mut unbound_adequacy = valid.clone();
    unbound_adequacy.test_adequacy.as_mut().unwrap().head_sha = None;
    for bundle in [&unbound_execution, &unbound_adequacy] {
        // Legacy evidence stays structurally valid and advisory-compatible.
        assert!(validate_bundle(bundle)
            .reason_codes()
            .iter()
            .all(|code| *code == "PK-PROV-DIGEST-MISMATCH"));
        assert!(
            !codes(bundle, EvaluationMode::Advisory).contains(&"PK-PROV-BINDING-MISSING".into())
        );
        assert!(codes(bundle, EvaluationMode::Authoritative)
            .contains(&"PK-PROV-BINDING-MISSING".into()));
    }
}
