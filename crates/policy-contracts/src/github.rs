use crate::{
    deserialize_schema_version, CheckConclusion, CommitSha, MergeMethod, PullRequestNumber,
    RepositoryId, Sha256Digest, TrustClass, UtcTimestamp, WorkItemId, WorkflowRunId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MergeActorType {
    Human,
    Bot,
    App,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkItemObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub work_item_id: WorkItemId,
    pub authorized: bool,
    pub observed_at: UtcTimestamp,
}

/// A label observation captured by the trusted integration at an exact PR head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentReadyObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub work_item_id: WorkItemId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub actor: String,
    pub label: String,
    pub observed_at: UtcTimestamp,
    pub trust: TrustClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitHubPullRequestObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub base_sha: CommitSha,
    pub head_sha: CommitSha,
    pub state: PullRequestState,
    pub is_draft: bool,
    pub observed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitHubCheckObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub check_name: String,
    pub workflow_run_id: WorkflowRunId,
    pub decision_digest: Sha256Digest,
    pub policy_digest: Sha256Digest,
    pub conclusion: CheckConclusion,
    pub started_at: UtcTimestamp,
    pub completed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitHubMergeObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub merge_commit_sha: CommitSha,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    pub method: MergeMethod,
    pub protected: bool,
    pub merged_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_type: Option<MergeActorType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub via_merge_queue: Option<bool>,
    pub merged_at: UtcTimestamp,
}
