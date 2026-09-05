use crate::{
    deserialize_schema_version, CommitSha, ExecutionId, FindingSeverity, ModelId, ProviderId,
    RepositoryId, ReviewStatus, SensorId, Sha256Digest, UtcTimestamp, WorkItemId, WorkflowRunId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustClass {
    LocalUntrusted,
    CiObserved,
    TrustedRunnerAttested,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Passed,
    Failed,
    TimedOut,
    Skipped,
    Unparsable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SensorStatus {
    Passed,
    Failed,
    TimedOut,
    Skipped,
    NotApplicable,
    Unparsable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    LatClaim,
    CodeGraphIndex,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactDigest {
    pub path: String,
    pub digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceManifest {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub work_item_id: WorkItemId,
    pub base_sha: CommitSha,
    pub head_sha: CommitSha,
    pub policy_digest: Sha256Digest,
    pub source_set_digest: Sha256Digest,
    pub workflow_run_id: WorkflowRunId,
    pub created_at: UtcTimestamp,
    pub child_digests: Vec<ArtifactDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModificationEvidence {
    pub provider: ProviderId,
    pub model: ModelId,
    pub execution_id: ExecutionId,
    pub head_sha: CommitSha,
    pub observed_at: UtcTimestamp,
    pub trust: TrustClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvidence {
    pub execution_id: ExecutionId,
    pub provider: ProviderId,
    pub model: ModelId,
    pub status: ExecutionStatus,
    pub command_digest: Sha256Digest,
    pub input_digest: Sha256Digest,
    pub output_digest: Sha256Digest,
    pub started_at: UtcTimestamp,
    pub completed_at: UtcTimestamp,
    pub trust: TrustClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReviewEvidence {
    pub provider: ProviderId,
    pub model: ModelId,
    pub execution_id: ExecutionId,
    pub status: ReviewStatus,
    pub head_sha: CommitSha,
    pub report_digest: Sha256Digest,
    pub observed_at: UtcTimestamp,
    pub trust: TrustClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub id: String,
    pub severity: FindingSeverity,
    pub summary: String,
    pub reviewer_execution_id: ExecutionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WaiverEvidence {
    pub actor: String,
    pub finding_id: String,
    pub repository: RepositoryId,
    pub head_sha: CommitSha,
    pub evidence_digest: Sha256Digest,
    pub reason: String,
    pub issued_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensorEvidence {
    pub sensor_id: SensorId,
    pub status: SensorStatus,
    pub head_sha: CommitSha,
    pub command_digest: Sha256Digest,
    pub result_digest: Sha256Digest,
    pub observed_at: UtcTimestamp,
    pub trust: TrustClass,
    #[serde(default)]
    pub resolved_command: Vec<String>,
    #[serde(default)]
    pub source_digest: Option<Sha256Digest>,
    #[serde(default)]
    pub configuration_digest: Option<Sha256Digest>,
    #[serde(default)]
    pub parser_version: Option<String>,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub discovered: Option<u64>,
    #[serde(default)]
    pub executed: Option<u64>,
    #[serde(default)]
    pub passed: Option<u64>,
    #[serde(default)]
    pub not_applicable_condition_id: Option<String>,
    #[serde(default)]
    pub not_applicable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestMapping {
    pub requirement_id: String,
    pub test_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestAdequacyEvidence {
    pub acceptance_criteria: Vec<TestMapping>,
    pub changed_behaviors: Vec<TestMapping>,
    pub negative_case_ids: Vec<String>,
    #[serde(default)]
    pub executed_test_ids: Vec<String>,
    pub discovered: u64,
    pub executed: u64,
    pub passed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeEvidence {
    pub kind: KnowledgeKind,
    pub source: String,
    pub version: String,
    pub configuration_digest: Sha256Digest,
    pub content_digest: Sha256Digest,
    pub source_set_digest: Sha256Digest,
    pub head_sha: CommitSha,
    pub trust: TrustClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub manifest: EvidenceManifest,
    pub modifications: Vec<ModificationEvidence>,
    pub executions: Vec<ExecutionEvidence>,
    pub reviews: Vec<ReviewEvidence>,
    pub findings: Vec<Finding>,
    pub waivers: Vec<WaiverEvidence>,
    pub sensors: Vec<SensorEvidence>,
    pub test_adequacy: Option<TestAdequacyEvidence>,
    pub knowledge: Vec<KnowledgeEvidence>,
}
