use crate::{deserialize_schema_version, ProviderId, SensorId, Sha256Digest};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Risk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicySnapshot {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub policy_digest: Sha256Digest,
    pub required_check_name: String,
    pub max_evidence_age_seconds: u64,
    pub authorized_actors: Vec<String>,
    pub required_sensor_ids: Vec<SensorId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequiredReviewPolicy {
    pub minimum_reviewers: u32,
    pub require_provider_independence: bool,
    pub allowed_providers: Vec<ProviderId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensorProfile {
    #[serde(deserialize_with = "deserialize_schema_version")]
    pub schema_version: u32,
    pub sensors: Vec<SensorDefinition>,
    pub adequacy: TestAdequacyPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensorDefinition {
    pub id: SensorId,
    pub required: bool,
    pub required_when: Vec<SensorCondition>,
    #[serde(default)]
    pub not_applicable_when: Vec<SensorCondition>,
    #[serde(default)]
    pub accepted_exit_codes: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensorCondition {
    pub id: String,
    pub risks: Vec<Risk>,
    pub path_globs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TestAdequacyPolicy {
    pub acceptance_criterion_ids: Vec<String>,
    pub changed_behavior_ids: Vec<String>,
    pub negative_case_ids: Vec<String>,
}
