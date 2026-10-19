use crate::{KnowledgeKind, MergeMethod, ProviderId, SensorId, Sha256Digest};
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicySnapshot {
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
    /// Policy v2 obligations; absent from the serialized legacy v1 projection.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_knowledge: Vec<KnowledgeRequirement>,
    /// Mandatory in v3; omitted from legacy digest preimages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_reviews: Option<RequiredReviewPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeRequirement {
    pub kind: KnowledgeKind,
    #[serde(deserialize_with = "deserialize_knowledge_source")]
    #[schemars(schema_with = "binding_nonblank_string_schema")]
    pub source: String,
    /// Empty means always required. Otherwise, any matching condition applies.
    pub required_when: Vec<KnowledgeCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeCondition {
    /// Empty means any risk; risk and path restrictions must both match.
    pub risks: Vec<Risk>,
    /// Empty means any path; nonempty entries use the sensor glob semantics.
    pub path_globs: Vec<String>,
}

fn deserialize_knowledge_source<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<String, D::Error> {
    let source = String::deserialize(deserializer)?;
    if source.trim().is_empty() || source.contains('\0') {
        return Err(serde::de::Error::custom(
            "knowledge source must be nonblank and contain no NUL",
        ));
    }
    Ok(source)
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct PolicySnapshotWire {
    #[schemars(schema_with = "policy_schema_version_schema")]
    schema_version: u32,
    policy_digest: Sha256Digest,
    required_check_name: String,
    max_evidence_age_seconds: u64,
    authorized_actors: Vec<String>,
    #[serde(default)]
    authorized_waiver_actors: Vec<String>,
    required_sensor_ids: Vec<SensorId>,
    #[serde(default)]
    sensor_profile_digest: Option<Sha256Digest>,
    #[serde(default)]
    protected_target_branches: Vec<String>,
    #[serde(default)]
    allowed_merge_methods: Vec<MergeMethod>,
    #[serde(default, deserialize_with = "deserialize_knowledge_requirements")]
    #[schemars(with = "Vec<KnowledgeRequirement>")]
    required_knowledge: Option<Vec<KnowledgeRequirement>>,
    #[serde(default, deserialize_with = "deserialize_required_reviews")]
    #[schemars(with = "RequiredReviewPolicy")]
    required_reviews: Option<RequiredReviewPolicy>,
}

fn deserialize_required_reviews<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<RequiredReviewPolicy>, D::Error> {
    RequiredReviewPolicy::deserialize(deserializer).map(Some)
}

fn policy_schema_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "integer", "enum": [1, 2, 3]})
}

fn deserialize_knowledge_requirements<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<KnowledgeRequirement>>, D::Error> {
    Vec::<KnowledgeRequirement>::deserialize(deserializer).map(Some)
}

impl PolicySnapshot {
    /// Direct Rust construction must obey the same version boundary as JSON.
    pub fn validate_version(&self) -> Result<(), &'static str> {
        match (self.schema_version, &self.required_reviews) {
            (1 | 2, Some(_)) => return Err("policy v1/v2 cannot declare required_reviews"),
            (3, None) => return Err("policy v3 requires required_reviews"),
            (3, Some(rules)) if rules.minimum_reviewers == 0 => {
                return Err("required_reviews minimum_reviewers must be positive");
            }
            (3, Some(rules)) if !rules.require_provider_independence => {
                return Err("policy v3 requires provider independence");
            }
            _ => {}
        }
        match self.schema_version {
            1 if self.required_knowledge.is_empty() => Ok(()),
            2 | 3 => Ok(()),
            1 => Err("policy v1 cannot declare required_knowledge"),
            _ => Err("unsupported policy schema version"),
        }
    }
}

impl<'de> Deserialize<'de> for PolicySnapshot {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = PolicySnapshotWire::deserialize(deserializer)?;
        if wire.schema_version == 1 && wire.required_knowledge.is_some() {
            return Err(serde::de::Error::custom(
                "policy v1 cannot declare required_knowledge",
            ));
        }
        let policy = Self {
            schema_version: wire.schema_version,
            policy_digest: wire.policy_digest,
            required_check_name: wire.required_check_name,
            max_evidence_age_seconds: wire.max_evidence_age_seconds,
            authorized_actors: wire.authorized_actors,
            authorized_waiver_actors: wire.authorized_waiver_actors,
            required_sensor_ids: wire.required_sensor_ids,
            sensor_profile_digest: wire.sensor_profile_digest,
            protected_target_branches: wire.protected_target_branches,
            allowed_merge_methods: wire.allowed_merge_methods,
            required_knowledge: wire.required_knowledge.unwrap_or_default(),
            required_reviews: wire.required_reviews,
        };
        policy
            .validate_version()
            .map_err(serde::de::Error::custom)?;
        Ok(policy)
    }
}

impl JsonSchema for PolicySnapshot {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PolicySnapshot".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        let mut schema = PolicySnapshotWire::json_schema(generator);
        schema.insert(
            "allOf".into(),
            schemars::json_schema!({"allOf": [{
                "if": {"properties": {"schema_version": {"const": 3}}},
                "then": {"required": ["required_reviews"]},
                "else": {"not": {"required": ["required_reviews"]}}
            }]})
            .get("allOf")
            .unwrap()
            .clone(),
        );
        schema.insert(
            "if".into(),
            schemars::json_schema!({"properties": {"schema_version": {"const": 1}}}).into(),
        );
        schema.insert(
            "then".into(),
            schemars::json_schema!({"not": {"required": ["required_knowledge"]}}).into(),
        );
        schema
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RequiredReviewPolicy {
    #[schemars(range(min = 1, max = 4294967295_u32))]
    pub minimum_reviewers: u32,
    /// V3 cannot disable the kernel's provider-independence invariant.
    #[schemars(schema_with = "required_independence_schema")]
    pub require_provider_independence: bool,
    /// Empty permits any provider; otherwise every review must use a listed one.
    pub allowed_providers: Vec<ProviderId>,
}

fn required_independence_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type": "boolean", "const": true})
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
    #[schemars(schema_with = "binding_nonblank_string_schema")]
    pub parser_version: String,
    #[schemars(schema_with = "binding_command_schema")]
    pub resolved_command: Vec<String>,
}

fn binding_nonblank_string_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    // Match Rust str::trim's Unicode White_Space set, rather than regex \s.
    schemars::json_schema!({
        "type": "string",
        "pattern": r"[^\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]",
        "not": {"pattern": "\u{0000}"}
    })
}

fn binding_command_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({
        "type": "array",
        "minItems": 1,
        "prefixItems": [binding_nonblank_string_schema(generator)],
        "items": {"type": "string", "not": {"pattern": "\u{0000}"}}
    })
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
