use crate::{deserialize_schema_version, CommitSha, EvidenceBundle, GitHubCheckObservation, GitHubMergeObservation, GitHubPullRequestObservation, PolicySnapshot, PullRequestNumber, RepositoryId, Sha256Digest, UtcTimestamp, WorkItemObservation};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationMode { Advisory, Authoritative }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreMergeStatus { Blocked, Inconclusive, ReadyToMerge }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeStatus { Blocked, Inconclusive, Shipped }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus { Approved, Blocked, Inconclusive }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity { Low, Medium, High, Critical }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion { Success, Failure, Cancelled, TimedOut, Neutral, Skipped }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod { Merge, Squash, Rebase }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reason { pub code: String, pub message: String }

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DecisionBinding {
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub policy_digest: Sha256Digest,
    pub evidence_digest: Sha256Digest,
    pub evaluated_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreMergeDecision {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub mode: EvaluationMode,
    pub advisory: bool,
    pub status: PreMergeStatus,
    pub binding: DecisionBinding,
    pub reasons: Vec<Reason>,
    pub decision_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct AuthoritativePreMergeDecision(PreMergeDecision);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("decision is not an authoritative ready-to-merge authorization")]
pub struct InvalidAuthoritativeDecision;

impl TryFrom<PreMergeDecision> for AuthoritativePreMergeDecision {
    type Error = InvalidAuthoritativeDecision;
    fn try_from(value: PreMergeDecision) -> Result<Self, Self::Error> {
        if value.mode == EvaluationMode::Authoritative
            && !value.advisory
            && value.status == PreMergeStatus::ReadyToMerge
        {
            Ok(Self(value))
        } else {
            Err(InvalidAuthoritativeDecision)
        }
    }
}

impl AuthoritativePreMergeDecision {
    pub fn as_decision(&self) -> &PreMergeDecision { &self.0 }
    pub fn into_decision(self) -> PreMergeDecision { self.0 }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PostMergeDecision {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub status: PostMergeStatus,
    pub binding: DecisionBinding,
    pub reasons: Vec<Reason>,
    pub decision_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreMergeEvaluationInput {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub mode: EvaluationMode,
    pub evaluation_time: UtcTimestamp,
    pub policy: PolicySnapshot,
    pub work_item: WorkItemObservation,
    pub pull_request: GitHubPullRequestObservation,
    pub evidence: EvidenceBundle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PostMergeEvaluationInput {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub evaluation_time: UtcTimestamp,
    pub policy: PolicySnapshot,
    pub pre_merge: AuthoritativePreMergeDecision,
    pub check: GitHubCheckObservation,
    pub merge: GitHubMergeObservation,
}
