use crate::{deserialize_schema_version, MergeMethod, ProviderId, SensorId, Sha256Digest};
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
    #[schemars(schema_with = "crate::schema_version_schema")]
    pub schema_version: u32,
    pub policy_digest: Sha256Digest,
    pub required_check_name: String,
    pub max_evidence_age_seconds: u64,
    pub authorized_actors: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authorized_waiver_actors: Vec<String>,
    pub required_sensor_ids: Vec<SensorId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensor_profile_digest: Option<Sha256Digest>,
    /// Branches on which a protected merge can authorize shipped state.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub protected_target_branches: Vec<String>,
    /// Merge methods explicitly accepted by this policy snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_merge_methods: Vec<MergeMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequiredReviewPolicy {
    pub minimum_reviewers: u32,
    pub require_provider_independence: bool,
    pub allowed_providers: Vec<ProviderId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SensorProfile {
    pub schema_version: u32,
    pub sensors: Vec<SensorDefinition>,
    pub adequacy: TestAdequacyPolicy,
}

impl SensorProfile {
    /// Validate version semantics even for profiles constructed directly in Rust.
    pub fn validate_version(&self) -> Result<(), &'static str> {
        if self
            .sensors
            .iter()
            .filter_map(|sensor| sensor.result_binding.as_ref())
            .any(|binding| {
                binding.parser_version.trim().is_empty()
                    || binding.parser_version.contains('\0')
                    || binding.resolved_command.is_empty()
                    || binding.resolved_command[0].trim().is_empty()
                    || binding
                        .resolved_command
                        .iter()
                        .any(|argument| argument.contains('\0'))
            })
        {
            return Err("sensor result binding requires a nonempty parser and command without NUL");
        }
        match self.schema_version {
            1 if self.sensors.iter().all(|sensor| {
                sensor.result_requirement.is_none() && sensor.result_binding.is_none()
            }) =>
            {
                Ok(())
            }
            2 if self
                .sensors
                .iter()
                .all(|sensor| sensor.result_requirement.is_some()) =>
            {
                Ok(())
            }
            1 => Err("sensor profile v1 cannot declare result requirements or bindings"),
            2 => Err("sensor profile v2 requires result_requirement for every sensor"),
            _ => Err("unsupported sensor profile schema version"),
        }
    }
}

impl<'de> Deserialize<'de> for SensorProfile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: u32,
            sensors: Vec<SensorDefinition>,
            adequacy: TestAdequacyPolicy,
        }
        let wire = Wire::deserialize(deserializer)?;
        let profile = Self {
            schema_version: wire.schema_version,
            sensors: wire.sensors,
            adequacy: wire.adequacy,
        };
        profile
            .validate_version()
            .map_err(serde::de::Error::custom)?;
        Ok(profile)
    }
}

impl JsonSchema for SensorProfile {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "SensorProfile".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let sensors = generator.subschema_for::<SensorDefinition>();
        let adequacy = generator.subschema_for::<TestAdequacyPolicy>();
        schemars::json_schema!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "schema_version": {"type": "integer", "enum": [1, 2]},
                "sensors": {"type": "array", "items": sensors},
                "adequacy": adequacy
            },
            "required": ["schema_version", "sensors", "adequacy"],
            "if": {"properties": {"schema_version": {"const": 1}}},
            "then": {"properties": {"sensors": {"items": {"not": {"anyOf": [{"required": ["result_requirement"]}, {"required": ["result_binding"]}]}}}}},
            "else": {"properties": {"sensors": {"items": {"required": ["result_requirement"]}}}}
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SensorResultRequirement {
    Process,
    TestCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SensorResultBinding {
    pub configuration_digest: Sha256Digest,
    pub parser_version: String,
    pub resolved_command: Vec<String>,
}

fn deserialize_result_binding<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SensorResultBinding>, D::Error> {
    SensorResultBinding::deserialize(deserializer).map(Some)
}

fn deserialize_result_requirement<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<SensorResultRequirement>, D::Error> {
    // An explicit null is invalid; absence alone denotes a legacy definition.
    SensorResultRequirement::deserialize(deserializer).map(Some)
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_result_requirement"
    )]
    #[schemars(with = "SensorResultRequirement")]
    pub result_requirement: Option<SensorResultRequirement>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_result_binding"
    )]
    #[schemars(with = "SensorResultBinding")]
    pub result_binding: Option<SensorResultBinding>,
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
