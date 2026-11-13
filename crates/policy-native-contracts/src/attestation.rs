//! Signed local-runner wire statements. No signing, authentication or promotion.
use crate::digest::canonical_envelope_sha256;
use crate::types::document;
use crate::validation::{require, role, subject};
use crate::*;
use policy_contracts::{CommitSha, ExecutionId, Sha256Digest, UtcTimestamp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Separate roles preserve every existing scope and ratchet schema byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalDigestRole {
    ScopeTemplate,
    ImportedObservations,
    LocalAttestation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalDigest {
    pub kind: CanonicalHashKind,
    pub schema: DigestSchema,
    pub role: LocalDigestRole,
    pub sha256: Sha256Digest,
}

/// Hash the same canonical {schema, role, value} envelope as existing documents.
/// Callers supply validated typed observations; this does not validate the value.
pub fn canonical_local_digest<T: Serialize>(
    role: LocalDigestRole,
    value: &T,
) -> Result<LocalDigest, policy_core::CanonicalError> {
    Ok(LocalDigest {
        kind: CanonicalHashKind::CanonicalSha256,
        schema: DigestSchema::PolicyNativeV1,
        role,
        sha256: canonical_envelope_sha256(role, value)?,
    })
}

impl ExpectedScopeV1 {
    /// Approval identity excluding the source subject and evaluation clock.
    pub fn template_digest(&self) -> Result<LocalDigest, policy_core::CanonicalError> {
        #[derive(Serialize)]
        struct Template<'a> {
            requirements: &'a RequirementsV1,
            observations: &'a RequiredSet<ObservationBindingV1>,
            maximum_age_seconds: u64,
            mode: LocalMode,
        }
        canonical_local_digest(
            LocalDigestRole::ScopeTemplate,
            &Template {
                requirements: &self.requirements,
                observations: &self.bindings.observations,
                maximum_age_seconds: self.bindings.maximum_age_seconds,
                mode: self.bindings.mode,
            },
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum LocalAttestationDomain {
    #[serde(rename = "policy-kernel/local-runner-attestation/v1")]
    LocalRunnerV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalRunnerV1 {
    pub runner_id: RecordId,
    pub os: RecordId,
    pub arch: RecordId,
    pub attestor_version: RecordId,
}

document!(LocalAttestationV1 {
    schema_version: VersionV1,
    domain: LocalAttestationDomain,
    subject: SubjectV1,
    head_tree: CommitSha,
    base: CommitSha,
    run_id: ExecutionId,
    scope_id: RecordId,
    expected_scope_digest: CanonicalDigest,
    scope_template_digest: LocalDigest,
    native_run_digest: CanonicalDigest,
    observations_digest: LocalDigest,
    result_digest: CanonicalDigest,
    runner: LocalRunnerV1,
    started_at: UtcTimestamp,
    completed_at: UtcTimestamp,
    attested_at: UtcTimestamp,
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LocalSignatureFormat {
    Sshsig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalSignatureV1 {
    pub format: LocalSignatureFormat,
    pub runner_id: RecordId,
    /// Opaque nonblank signature. Cryptographic verification belongs to consumers.
    pub value: RecordId,
}

document!(SignedLocalAttestationV1 {
    statement: LocalAttestationV1,
    signature: LocalSignatureV1,
});

impl LocalAttestationV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        subject(&self.subject)?;
        role(&self.expected_scope_digest, DigestRole::ExpectedScope)?;
        role(&self.native_run_digest, DigestRole::NativeRun)?;
        role(&self.result_digest, DigestRole::LocalScopeResult)?;
        require(
            self.scope_template_digest.role == LocalDigestRole::ScopeTemplate
                && self.observations_digest.role == LocalDigestRole::ImportedObservations,
            "local canonical digest role mismatch",
        )?;
        require(
            self.started_at <= self.completed_at && self.completed_at <= self.attested_at,
            "attestation time order mismatch",
        )
    }

    /// Document identity; signatures cover canonical JSON of the statement itself.
    pub fn digest(&self) -> Result<LocalDigest, policy_core::CanonicalError> {
        canonical_local_digest(LocalDigestRole::LocalAttestation, self)
    }
}

impl SignedLocalAttestationV1 {
    pub fn validate(&self) -> Result<(), ContractError> {
        self.statement.validate()?;
        require(
            self.statement.runner.runner_id == self.signature.runner_id,
            "signature runner differs from statement runner",
        )
    }
}
