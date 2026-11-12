//! Language-independent debt identities and comparison records.
use crate::digest::canonical_envelope_sha256;
use crate::{CanonicalHashKind, DigestSchema, RecordId, VersionV1};
use policy_contracts::Sha256Digest;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Identity is the entire tuple; allowances never transfer between contexts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetKeyV1 {
    pub tool: RecordId,
    pub rule: RecordId,
    pub path: String,
    pub symbol: RecordId,
    pub context_hash: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetEntryV1 {
    pub key: RatchetKeyV1,
    /// Positive counts only; absence represents zero.
    #[schemars(range(min = 1))]
    pub count: u64,
}

/// Caller supplies independently approved base allowances and proposed head
/// allowances. Decoding checks wire types; evaluation checks entry invariants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetRuleV1 {
    pub schema_version: VersionV1,
    pub approved_baseline: Vec<RatchetEntryV1>,
    pub head_baseline: Vec<RatchetEntryV1>,
}

/// Already aggregated findings. Extraction, suppression detection and context
/// hashing belong to the producer, outside this contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetObservationV1 {
    pub schema_version: VersionV1,
    pub entries: Vec<RatchetEntryV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RatchetStatus {
    Pass,
    Fail,
}

/// This declaration order defines the first component of violation sorting.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum RatchetInput {
    Rule,
    ApprovedBaseline,
    HeadBaseline,
    Observation,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RatchetReasonCode {
    AllowanceIncreased,
    ContextReplacement,
    DebtIncreased,
    DuplicateKey,
    InvalidCount,
    InvalidPath,
    MalformedInput,
    NewDebt,
}

/// Sorted by input, then the full key tuple, then reason code. A duplicate key
/// has one violation regardless of how many times it occurs.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetViolationV1 {
    pub input: RatchetInput,
    pub key: Option<RatchetKeyV1>,
    pub code: RatchetReasonCode,
}

/// Canonical envelope roles for ratchet inputs, separate from scope roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RatchetDigestRole {
    RatchetRule,
    RatchetObservation,
}

/// SHA-256 of the native canonical {schema, role, value} envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetDigest {
    pub kind: CanonicalHashKind,
    pub schema: DigestSchema,
    pub role: RatchetDigestRole,
    pub sha256: Sha256Digest,
}

impl RatchetDigest {
    fn new<T: Serialize>(
        role: RatchetDigestRole,
        value: &T,
    ) -> Result<Self, policy_core::CanonicalError> {
        Ok(Self {
            kind: CanonicalHashKind::CanonicalSha256,
            schema: DigestSchema::PolicyNativeV1,
            role,
            sha256: canonical_envelope_sha256(role, value)?,
        })
    }
}

/// A comparison result, never an approval, execution attestation or merge gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RatchetResultV1 {
    pub schema_version: VersionV1,
    pub status: RatchetStatus,
    pub violations: Vec<RatchetViolationV1>,
    /// Absent if either input is malformed or structurally invalid.
    pub rule_digest: Option<RatchetDigest>,
    pub observation_digest: Option<RatchetDigest>,
}

/// Collect every invalid entry and duplicate key without choosing a winner.
fn validate_ratchet_entries(
    input: RatchetInput,
    entries: &[RatchetEntryV1],
) -> Vec<RatchetViolationV1> {
    let mut seen = BTreeSet::new();
    let mut violations = BTreeSet::new();
    for entry in entries {
        if entry.count == 0 {
            violations.insert(RatchetViolationV1 {
                input,
                key: Some(entry.key.clone()),
                code: RatchetReasonCode::InvalidCount,
            });
        }
        if crate::validation::relative_path(&entry.key.path, false).is_err() {
            violations.insert(RatchetViolationV1 {
                input,
                key: Some(entry.key.clone()),
                code: RatchetReasonCode::InvalidPath,
            });
        }
        if !seen.insert(&entry.key) {
            violations.insert(RatchetViolationV1 {
                input,
                key: Some(entry.key.clone()),
                code: RatchetReasonCode::DuplicateKey,
            });
        }
    }
    violations.into_iter().collect()
}

impl RatchetRuleV1 {
    pub fn validate(&self) -> Vec<RatchetViolationV1> {
        let mut violations =
            validate_ratchet_entries(RatchetInput::ApprovedBaseline, &self.approved_baseline);
        violations.extend(validate_ratchet_entries(
            RatchetInput::HeadBaseline,
            &self.head_baseline,
        ));
        violations
    }

    pub fn digest(&self) -> Result<RatchetDigest, policy_core::CanonicalError> {
        RatchetDigest::new(RatchetDigestRole::RatchetRule, self)
    }
}

impl RatchetObservationV1 {
    pub fn validate(&self) -> Vec<RatchetViolationV1> {
        validate_ratchet_entries(RatchetInput::Observation, &self.entries)
    }

    pub fn digest(&self) -> Result<RatchetDigest, policy_core::CanonicalError> {
        RatchetDigest::new(RatchetDigestRole::RatchetObservation, self)
    }
}
