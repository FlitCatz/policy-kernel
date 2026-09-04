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
