use policy_native_contracts::*;
use serde_json::json;

#[test]
fn ratchet_schemas_match_committed_assets_and_reject_invalid_wire_shapes() {
    let key = json!({"tool":"analyzer", "rule":"R1", "path":"src/lib.rs",
        "symbol":"compute", "context_hash":"a".repeat(64)});
    for (schema, committed, valid, invalid) in [
        (
            schemars::schema_for!(RatchetRuleV1),
            include_str!("../schemas/ratchet-rule-v1.schema.json"),
            json!({"schema_version":1,"approved_baseline":[],"head_baseline":[]}),
            json!({"schema_version":2,"approved_baseline":[],"head_baseline":[]}),
        ),
        (
            schemars::schema_for!(RatchetObservationV1),
            include_str!("../schemas/ratchet-observation-v1.schema.json"),
            json!({"schema_version":1,"entries":[{"key":key,"count":1}]}),
            json!({"schema_version":1,"entries":[{"key":key,"count":-1}]}),
        ),
        (
            schemars::schema_for!(RatchetResultV1),
            include_str!("../schemas/ratchet-result-v1.schema.json"),
            json!({"schema_version":1,"status":"pass","violations":[],"rule_digest":null,"observation_digest":null}),
            json!({"schema_version":1,"status":"success","violations":[],"rule_digest":null,"observation_digest":null}),
        ),
    ] {
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&schema).unwrap()),
            committed
        );
        let schema = serde_json::to_value(schema).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&valid));
        assert!(!validator.is_valid(&invalid));
        let mut unknown = valid.clone();
        unknown["unknown"] = json!(true);
        assert!(!validator.is_valid(&unknown));
    }
}

#[test]
fn ratchet_digest_domains_pin_canonical_preimages_without_changing_native_vectors() {
    let value = json!({"z":1,"a":2});
    // SHA-256 of the documented canonical envelope, independently computed.
    let digest = canonical_digest(DigestRole::RatchetRule, &value).unwrap();
    assert_eq!(
        digest.sha256.as_str(),
        "0392dd8bd4e2568c80042ee09ffa443438e966229253da8a422b2775d00cea7e"
    );
    assert_ne!(
        digest,
        canonical_digest(DigestRole::RatchetObservation, &value).unwrap()
    );
    assert_ne!(
        digest,
        canonical_digest(DigestRole::RatchetResult, &value).unwrap()
    );
}
