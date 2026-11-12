use policy_native_contracts::*;
use serde_json::{json, Value};
#[path = "support/attestation.rs"]
mod attestation;
mod support;

#[test]
fn attestation_schemas_and_corpus_enforce_wire_and_relational_contracts() {
    let corpus: Value = serde_json::from_str(include_str!("corpus/attestation-v1.json")).unwrap();
    assert_eq!(corpus, attestation::corpus());
    for (kind, schema, committed) in [
        (
            "statement",
            schemars::schema_for!(LocalAttestationV1),
            include_str!("../schemas/local-attestation-v1.schema.json"),
        ),
        (
            "signed",
            schemars::schema_for!(SignedLocalAttestationV1),
            include_str!("../schemas/signed-local-attestation-v1.schema.json"),
        ),
    ] {
        assert_eq!(
            format!(
                "{}\n",
                serde_json::to_string_pretty(&serde_json::to_value(schema).unwrap()).unwrap()
            ),
            committed
        );
        let schema: Value = serde_json::from_str(committed).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let decode = |v| {
            if kind == "statement" {
                serde_json::from_value::<LocalAttestationV1>(v).is_ok()
            } else {
                serde_json::from_value::<SignedLocalAttestationV1>(v).is_ok()
            }
        };
        assert!(decode(corpus["valid"][kind].clone()));
        assert!(validator.is_valid(&corpus["valid"][kind]));
        for case in corpus["invalid"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["document"] == kind)
        {
            let mut value = corpus["valid"][kind].clone();
            let pointer = case["pointer"].as_str().unwrap();
            let (parent, field) = pointer.rsplit_once('/').unwrap();
            value
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(field.into(), case["value"].clone());
            assert!(!decode(value.clone()), "{kind}: {}", case["name"]);
            assert_eq!(
                validator.is_valid(&value),
                case["schema_valid"].as_bool().unwrap(),
                "{kind}: {}",
                case["name"]
            );
        }
    }
}

#[test]
fn template_digest_excludes_subject_and_clock_but_binds_all_approval_fields() {
    let expected: ExpectedScopeV1 = serde_json::from_value(support::expected()).unwrap();
    let digest = expected.template_digest().unwrap();
    let mut changed = expected.clone();
    changed.bindings.evaluated_at = serde_json::from_value(json!("2026-10-20T01:01:00Z")).unwrap();
    assert_eq!(digest, changed.template_digest().unwrap());
    assert_ne!(expected.digest().unwrap(), changed.digest().unwrap());
    changed = expected.clone();
    changed.subject.head = serde_json::from_value(json!("4".repeat(40))).unwrap();
    assert_eq!(digest, changed.template_digest().unwrap());
    assert_ne!(expected.digest().unwrap(), changed.digest().unwrap());
    let preimage = json!({"schema":"policy_native_v1","role":"scope_template","value":{
        "requirements":expected.requirements,"observations":expected.bindings.observations,
        "maximum_age_seconds":expected.bindings.maximum_age_seconds,"mode":expected.bindings.mode}});
    assert_eq!(
        digest.sha256,
        policy_core::canonical_sha256(&preimage).unwrap()
    );
    for (pointer, value) in [
        (
            "/requirements/approved_file_digest/sha256",
            json!("c".repeat(64)),
        ),
        (
            "/requirements/observations/0/tests/minimum_executed",
            json!(2),
        ),
        (
            "/requirements/requirements/0/minimum_executed_tests",
            json!(2),
        ),
        (
            "/requirements/exemptions",
            json!([{"id":"exempt-1","scope":"outside","reason":"approved exception"}]),
        ),
        (
            "/bindings/observations/0/command_digest/sha256",
            json!("c".repeat(64)),
        ),
        (
            "/bindings/observations/0/configuration_digest/sha256",
            json!("c".repeat(64)),
        ),
        ("/bindings/observations/0/parser/version", json!("2")),
        ("/bindings/maximum_age_seconds", json!(3601)),
    ] {
        let mut document = support::expected();
        *document.pointer_mut(pointer).unwrap() = value;
        let changed: ExpectedScopeV1 = serde_json::from_value(document).unwrap();
        assert_ne!(digest, changed.template_digest().unwrap(), "{pointer}");
    }
}

#[test]
fn canonical_vectors_pin_new_and_existing_document_digests() {
    let vectors: Value =
        serde_json::from_str(include_str!("corpus/canonical-attestation-v1.json")).unwrap();
    assert_eq!(vectors, attestation::canonical_vectors());
    let statement: LocalAttestationV1 =
        serde_json::from_value(attestation::corpus()["valid"]["statement"].clone()).unwrap();
    let preimage =
        json!({"schema":"policy_native_v1","role":"local_attestation","value":statement});
    assert_eq!(
        statement.digest().unwrap().sha256,
        policy_core::canonical_sha256(&preimage).unwrap()
    );
    for role in [
        LocalDigestRole::ScopeTemplate,
        LocalDigestRole::ImportedObservations,
        LocalDigestRole::LocalAttestation,
    ] {
        let digest = canonical_local_digest(role, &vectors["value"]).unwrap();
        assert_ne!(
            digest.sha256,
            canonical_digest(DigestRole::Command, &vectors["value"])
                .unwrap()
                .sha256
        );
        assert!(
            serde_json::from_value::<CanonicalDigest>(serde_json::to_value(digest).unwrap())
                .is_err()
        );
    }
    assert!(canonical_local_digest(LocalDigestRole::ImportedObservations, &json!(1.5)).is_err());
}

#[test]
fn direct_call_validation_and_canonical_signing_bytes_preserve_the_boundary() {
    let mut signed: SignedLocalAttestationV1 =
        serde_json::from_value(attestation::corpus()["valid"]["signed"].clone()).unwrap();
    signed.validate().unwrap();
    let original = signed.statement.digest().unwrap();
    let bytes = policy_core::canonical_json(&signed.statement).unwrap();
    let roundtrip: LocalAttestationV1 = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(original, roundtrip.digest().unwrap());
    signed.signature.runner_id = serde_json::from_value(json!("other")).unwrap();
    assert!(signed.validate().is_err());
    assert_eq!(original, signed.statement.digest().unwrap());
    signed.statement.scope_template_digest.role = LocalDigestRole::ImportedObservations;
    assert!(signed.statement.validate().is_err());
}
