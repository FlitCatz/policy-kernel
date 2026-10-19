use policy_contracts::Sha256Digest;
use policy_core::{canonical_sha256, CanonicalError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RawHashKind {
    RawByteSha256,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalHashKind {
    CanonicalSha256,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DigestSchema {
    PolicyNativeV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DigestRole {
    ExpectedScope,
    NativeRun,
    LocalScopeResult,
    SourceSet,
    ScopeDefinition,
    Command,
    Configuration,
    Policy,
    Evidence,
}

/// An externally computed SHA-256 of exact file bytes, never typed JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RawByteDigest {
    pub kind: RawHashKind,
    pub sha256: Sha256Digest,
}

/// SHA-256 of the canonical {schema, role, value} envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CanonicalDigest {
    pub kind: CanonicalHashKind,
    pub schema: DigestSchema,
    pub role: DigestRole,
    pub sha256: Sha256Digest,
}

pub fn canonical_digest<T: Serialize>(
    role: DigestRole,
    value: &T,
) -> Result<CanonicalDigest, CanonicalError> {
    #[derive(Serialize)]
    struct Preimage<'a, T> {
        schema: DigestSchema,
        role: DigestRole,
        value: &'a T,
    }
    let schema = DigestSchema::PolicyNativeV1;
    Ok(CanonicalDigest {
        kind: CanonicalHashKind::CanonicalSha256,
        schema,
        role,
        sha256: canonical_sha256(&Preimage {
            schema,
            role,
            value,
        })?,
    })
}
