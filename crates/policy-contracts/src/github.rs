use crate::{
    deserialize_schema_version, CheckConclusion, CommitSha, MergeMethod, PullRequestNumber,
    RepositoryId, Sha256Digest, UtcTimestamp, WorkItemId, WorkflowRunId,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkItemObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub work_item_id: WorkItemId,
    pub authorized: bool,
    pub observed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GitHubPullRequestObservation {
    #[serde(deserialize_with = "deserialize_schema_version")]
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
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub merge_commit_sha: CommitSha,
    pub method: MergeMethod,
    pub protected: bool,
    pub merged_by: String,
    pub merged_at: UtcTimestamp,
}
