use crate::{
    deserialize_schema_version, AgentReadyObservation, CommitSha, EvidenceBundle,
    GitHubCheckObservation, GitHubMergeObservation, GitHubPullRequestObservation, PolicySnapshot,
    PullRequestNumber, RepositoryId, Risk, SensorProfile, Sha256Digest, UtcTimestamp,
    WorkItemObservation, WorkflowRunId,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationMode {
    Advisory,
    Authoritative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PreMergeStatus {
    Blocked,
    Inconclusive,
    ReadyToMerge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PostMergeStatus {
    Blocked,
    Inconclusive,
    Shipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReviewStatus {
    Approved,
    Blocked,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckConclusion {
    Success,
    Failure,
    Cancelled,
    TimedOut,
    Neutral,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Reason {
    pub code: String,
    pub message: String,
}

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
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub mode: EvaluationMode,
    pub advisory: bool,
    pub status: PreMergeStatus,
    pub binding: DecisionBinding,
    pub reasons: Vec<Reason>,
    pub decision_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
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

impl<'de> Deserialize<'de> for AuthoritativePreMergeDecision {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        PreMergeDecision::deserialize(deserializer)?
            .try_into()
            .map_err(serde::de::Error::custom)
    }
}

impl AuthoritativePreMergeDecision {
    pub fn as_decision(&self) -> &PreMergeDecision {
        &self.0
    }
    pub fn into_decision(self) -> PreMergeDecision {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PostMergeDecision {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub status: PostMergeStatus,
    pub binding: DecisionBinding,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_commit_sha: Option<CommitSha>,
    pub reasons: Vec<Reason>,
    pub decision_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(transform = require_legacy_risk)]
pub struct PreMergeEvaluationInput {
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub mode: EvaluationMode,
    pub evaluation_time: UtcTimestamp,
    pub policy: PolicySnapshot,
    pub work_item: WorkItemObservation,
    pub pull_request: GitHubPullRequestObservation,
    pub evidence: EvidenceBundle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<PreMergeContext>,
}

impl<'de> Deserialize<'de> for PreMergeEvaluationInput {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(remote = "PreMergeEvaluationInput", deny_unknown_fields)]
        struct Wire {
            #[serde(deserialize_with = "deserialize_schema_version")]
            schema_version: u32,
            mode: EvaluationMode,
            evaluation_time: UtcTimestamp,
            policy: PolicySnapshot,
            work_item: WorkItemObservation,
            pull_request: GitHubPullRequestObservation,
            evidence: EvidenceBundle,
            context: Option<PreMergeContext>,
        }

        let input = Wire::deserialize(deserializer)?;
        if let Some(context) = &input.context {
            context
                .validate_for_policy(&input.policy)
                .map_err(serde::de::Error::custom)?;
        }
        Ok(input)
    }
}

fn require_legacy_risk(schema: &mut schemars::Schema) {
    schema.insert(
        "if".into(),
        schemars::json_schema!({"properties": {"policy": {"properties": {
            "schema_version": {"enum": [1, 2, 3, 4]}
        }}}})
        .into(),
    );
    schema.insert(
        "then".into(),
        schemars::json_schema!({"properties": {"context": {
            "required": ["risk"], "properties": {"risk": {"type": "string"}}
        }}})
        .into(),
    );
}

/// Explicit current-run inputs. Absent legacy context cannot establish readiness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreMergeContext {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub sensor_profile: SensorProfile,
    /// Caller-supplied changed paths. An empty list means a known empty change.
    pub changed_paths: Vec<String>,
    /// Required for policy v1 through v4. V5 scope rules ignore it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<Risk>,
    pub workflow_run_id: WorkflowRunId,
    pub agent_ready: Option<AgentReadyObservation>,
    /// Caller-supplied complete PR label list. Empty means a known absence of labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub labels: Option<Vec<String>>,
}

impl PreMergeContext {
    /// Check the version-dependent context contract after pairing it with a policy.
    ///
    /// # Errors
    /// Returns an error when a policy version from 1 through 4 has no observed risk.
    pub fn validate_for_policy(&self, policy: &PolicySnapshot) -> Result<(), &'static str> {
        if (1..=4).contains(&policy.schema_version) && self.risk.is_none() {
            return Err("context.risk is required for policy v1 through v4");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PostMergeEvaluationInput {
    #[serde(deserialize_with = "deserialize_schema_version")]
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub evaluation_time: UtcTimestamp,
    pub policy: PolicySnapshot,
    pub pre_merge: AuthoritativePreMergeDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub check: Option<GitHubCheckObservation>,
    pub merge: GitHubMergeObservation,
}
