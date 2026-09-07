use crate::{canonical_sha256, CanonicalError};
use policy_contracts::{
    EvidenceBundle, PolicySnapshot, PreMergeDecision, PreMergeEvaluationInput, Sha256Digest,
};
use serde_json::{json, Value};

/// Hash the complete policy snapshot except its self-referential digest field.
pub fn pre_merge_policy_digest(policy: &PolicySnapshot) -> Result<Sha256Digest, CanonicalError> {
    let mut value = serde_json::to_value(policy)?;
    value
        .as_object_mut()
        .expect("policy is an object")
        .remove("policy_digest");
    canonical_sha256(&value)
}

/// Semantic identity of the evidence being waived, not a transport bundle hash.
/// Raw child hashes must still pass `validate_bundle`. Waivers are excluded to
/// avoid a self-reference cycle; the decision digest separately binds them.
pub fn pre_merge_evidence_digest(bundle: &EvidenceBundle) -> Result<Sha256Digest, CanonicalError> {
    let mut value = evidence_projection(bundle)?;
    value
        .as_object_mut()
        .expect("bundle is an object")
        .remove("waivers");
    canonical_sha256(&json!({"domain": "policy-kernel/pre-merge-evidence/v1", "evidence": value}))
}

pub(crate) fn decision_digest(
    input: &PreMergeEvaluationInput,
    decision: &PreMergeDecision,
) -> Result<Sha256Digest, CanonicalError> {
    let mut input_value = serde_json::to_value(input)?;
    input_value["evidence"] = evidence_projection(&input.evidence)?;
    let mut decision_value = serde_json::to_value(decision)?;
    decision_value
        .as_object_mut()
        .expect("decision is an object")
        .remove("decision_digest");
    canonical_sha256(&json!({
        "domain": "policy-kernel/pre-merge-decision/v1",
        "input": input_value,
        "decision": decision_value,
    }))
}

fn evidence_projection(bundle: &EvidenceBundle) -> Result<Value, CanonicalError> {
    let mut value = serde_json::to_value(bundle)?;
    // Only the semantic projection omits the transport envelope. The original
    // bundle is immutable and its exact, order-sensitive child hashes are checked.
    value["manifest"]
        .as_object_mut()
        .expect("manifest is an object")
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
        let records = value[name]
            .as_array_mut()
            .expect("evidence records are arrays");
        let mut keyed = records
            .drain(..)
            .map(|record| serde_json::to_vec(&record).map(|bytes| (bytes, record)))
            .collect::<Result<Vec<_>, _>>()?;
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        records.extend(keyed.into_iter().map(|(_, record)| record));
    }
    Ok(value)
}
