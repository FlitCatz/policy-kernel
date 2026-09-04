use policy_contracts::{
    EvaluationMode, EvidenceBundle, ModificationEvidence, ReviewEvidence, UtcTimestamp,
};
use policy_core::{
    canonical_sha256, validate_bundle, validate_provenance, verifier_is_independent,
};
use serde_json::json;

fn fixture(name: &str) -> EvidenceBundle {
    let raw = match name {
        "valid.json" => include_str!("../../../tests/fixtures/evidence/valid.json"),
        "tampered.json" => include_str!("../../../tests/fixtures/evidence/tampered.json"),
        "local-authoritative.json" => {
            include_str!("../../../tests/fixtures/evidence/local-authoritative.json")
        }
        "stale.json" => include_str!("../../../tests/fixtures/evidence/stale.json"),
        _ => panic!("unknown fixture {name}"),
    };
    serde_json::from_str(raw).expect("fixture must satisfy the strict contract")
}

fn fixed_evaluation_time() -> UtcTimestamp {
    UtcTimestamp::parse("2026-09-04T09:00:00Z").unwrap()
}

#[test]
fn canonical_digest_is_independent_of_object_key_order() {
    let left = json!({"z": [3, 2, 1], "a": {"y": true, "x": null}});
    let right = json!({"a": {"x": null, "y": true}, "z": [3, 2, 1]});

    assert_eq!(
        canonical_sha256(&left).unwrap(),
        canonical_sha256(&right).unwrap()
    );
}

#[test]
fn canonical_digest_matches_hand_checked_utf8_sha256() {
    let value = json!({"b": 2, "a": 1});
    assert_eq!(
        canonical_sha256(&value).unwrap().as_str(),
        "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
    );
}

#[test]
fn canonical_digest_rejects_floating_point_numbers() {
    assert!(canonical_sha256(&json!({"unsafe": 1.5})).is_err());
}

#[test]
fn valid_bundle_passes_integrity_validation() {
    let mut bundle = fixture("valid.json");
    bundle.manifest.child_digests.clear();
    assert_eq!(
        validate_bundle(&bundle).reason_codes(),
        ["PK-PROV-DIGEST-MISMATCH"]
    );
}

#[test]
fn matching_child_digest_passes_integrity_validation() {
    assert!(validate_bundle(&fixture("valid.json")).is_valid());
}

#[test]
fn tampered_child_digest_is_inconclusive() {
    let result = validate_bundle(&fixture("tampered.json"));
    assert_eq!(result.reason_codes(), ["PK-PROV-DIGEST-MISMATCH"]);
}

#[test]
fn local_evidence_cannot_satisfy_authoritative_mode() {
    let result = validate_provenance(
        EvaluationMode::Authoritative,
        &fixture("local-authoritative.json"),
        &fixed_evaluation_time(),
        3_600,
    );
    assert_eq!(result.reason_codes(), ["PK-PROV-UNTRUSTED"]);
}

#[test]
fn stale_evidence_fails_closed() {
    let result = validate_provenance(
        EvaluationMode::Authoritative,
        &fixture("stale.json"),
        &fixed_evaluation_time(),
        3_600,
    );
    assert_eq!(result.reason_codes(), ["PK-PROV-STALE"]);
}

#[test]
fn evidence_head_and_knowledge_source_set_must_match_manifest() {
    let mut bundle = fixture("valid.json");
    bundle.modifications[0].head_sha =
        policy_contracts::CommitSha::parse("ffffffffffffffffffffffffffffffffffffffff").unwrap();

    assert!(validate_bundle(&bundle)
        .reason_codes()
        .contains(&"PK-PROV-BINDING-MISMATCH"));
}

#[test]
fn waiver_repository_and_head_must_match_manifest() {
    let mut bundle = fixture("valid.json");
    bundle.waivers.push(policy_contracts::WaiverEvidence {
        actor: "owner".to_owned(),
        finding_id: "finding-1".to_owned(),
        repository: policy_contracts::RepositoryId::parse("other/repository").unwrap(),
        head_sha: bundle.manifest.head_sha.clone(),
        evidence_digest: policy_contracts::Sha256Digest::parse(
            "9999999999999999999999999999999999999999999999999999999999999999",
        )
        .unwrap(),
        reason: "accepted risk".to_owned(),
        issued_at: UtcTimestamp::parse("2026-09-04T08:30:00Z").unwrap(),
        expires_at: UtcTimestamp::parse("2026-09-05T08:30:00Z").unwrap(),
    });
    bundle.manifest.child_digests.clear();

    let result = validate_bundle(&bundle);
    let codes = result.reason_codes();
    assert!(codes.contains(&"PK-PROV-BINDING-MISMATCH"));
}

#[test]
fn waiver_issuance_must_be_fresh() {
    let mut bundle = fixture("valid.json");
    bundle.modifications.clear();
    bundle.reviews.clear();
    bundle.waivers.push(policy_contracts::WaiverEvidence {
        actor: "owner".to_owned(),
        finding_id: "finding-1".to_owned(),
        repository: bundle.manifest.repository.clone(),
        head_sha: bundle.manifest.head_sha.clone(),
        evidence_digest: policy_contracts::Sha256Digest::parse(
            "9999999999999999999999999999999999999999999999999999999999999999",
        )
        .unwrap(),
        reason: "accepted risk".to_owned(),
        issued_at: UtcTimestamp::parse("2026-09-04T07:00:00Z").unwrap(),
        expires_at: UtcTimestamp::parse("2026-09-05T08:30:00Z").unwrap(),
    });
    bundle.manifest.child_digests.clear();

    let result = validate_provenance(
        EvaluationMode::Authoritative,
        &bundle,
        &fixed_evaluation_time(),
        3_600,
    );
    let codes = result.reason_codes();
    assert!(codes.contains(&"PK-PROV-STALE"));
}

#[test]
fn verifier_must_be_independent_of_every_modifier() {
    let bundle = fixture("valid.json");
    let verifier: &ReviewEvidence = &bundle.reviews[0];
    let modifiers: &[ModificationEvidence] = &bundle.modifications;

    assert!(!verifier_is_independent(verifier, modifiers));
}

#[test]
fn duplicate_reviewer_provider_and_execution_fail_closed() {
    let mut bundle = fixture("valid.json");
    bundle.modifications.clear();
    bundle.reviews.push(bundle.reviews[0].clone());

    let result = validate_provenance(
        EvaluationMode::Authoritative,
        &bundle,
        &fixed_evaluation_time(),
        3_600,
    );
    let codes = result.reason_codes();
    assert!(codes.contains(&"PK-PROV-DUPLICATE-EXECUTION"));
    assert!(codes.contains(&"PK-PROV-DUPLICATE-PROVIDER"));
}

#[test]
fn execution_ids_must_be_unique_across_all_evidence_kinds() {
    let mut bundle = fixture("local-authoritative.json");
    bundle.executions[0].trust = policy_contracts::TrustClass::TrustedRunnerAttested;
    bundle.modifications.push(ModificationEvidence {
        provider: policy_contracts::ProviderId::parse("producer").unwrap(),
        model: policy_contracts::ModelId::parse("model").unwrap(),
        execution_id: policy_contracts::ExecutionId::parse("local-run").unwrap(),
        head_sha: bundle.manifest.head_sha.clone(),
        observed_at: UtcTimestamp::parse("2026-09-04T08:55:00Z").unwrap(),
        trust: policy_contracts::TrustClass::TrustedRunnerAttested,
    });

    let result = validate_provenance(
        EvaluationMode::Authoritative,
        &bundle,
        &fixed_evaluation_time(),
        3_600,
    );
    assert!(result
        .reason_codes()
        .contains(&"PK-PROV-DUPLICATE-EXECUTION"));
}
