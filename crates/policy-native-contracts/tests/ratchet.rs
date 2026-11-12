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
    // SHA-256 of the documented canonical envelopes, independently computed:
    // {"role":"ratchet_rule","schema":"policy_native_v1","value":{"approved_baseline":[],"head_baseline":[],"schema_version":1}}
    // {"role":"ratchet_observation","schema":"policy_native_v1","value":{"entries":[],"schema_version":1}}
    let rule: RatchetRuleV1 = serde_json::from_value(
        json!({"schema_version":1,"approved_baseline":[],"head_baseline":[]}),
    )
    .unwrap();
    let observation: RatchetObservationV1 =
        serde_json::from_value(json!({"schema_version":1,"entries":[]})).unwrap();
    assert_eq!(
        serde_json::to_value(rule.digest().unwrap()).unwrap(),
        json!({"kind":"canonical_sha256","schema":"policy_native_v1","role":"ratchet_rule",
            "sha256":"d3a829928a08de4ab958d07471ff9b324ebccab9e293c674b1f1748fb42e025c"})
    );
    assert_eq!(
        serde_json::to_value(observation.digest().unwrap()).unwrap(),
        json!({"kind":"canonical_sha256","schema":"policy_native_v1","role":"ratchet_observation",
            "sha256":"e94681eafcefef224d8b55bc12c80ca3a8a86dd2e417c55f5ae195f6c9f0200c"})
    );
    for role in ["ratchet_rule", "ratchet_observation", "ratchet_result"] {
        assert!(serde_json::from_value::<DigestRole>(json!(role)).is_err());
    }
}
