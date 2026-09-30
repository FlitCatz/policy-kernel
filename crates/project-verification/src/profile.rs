use crate::{invalid, reports::ReportFormat, Error};
use policy_contracts::{
    Risk, SensorDefinition, SensorProfile, SensorResultRequirement, TestAdequacyPolicy,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(transform = profile_schema)]
pub struct ProjectProfile {
    #[schemars(schema_with = "profile_version_schema")]
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub components: Vec<Component>,
    pub sensors: Vec<ProjectSensor>,
    pub adequacy: TestAdequacyPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(with = "crate::selection::Selection")]
    pub selection: Option<crate::selection::Selection>,
}

impl<'de> Deserialize<'de> for ProjectProfile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            schema_version: u32,
            id: String,
            version: String,
            components: Vec<Component>,
            sensors: Vec<ProjectSensor>,
            adequacy: TestAdequacyPolicy,
            #[serde(default, deserialize_with = "selection_field")]
            selection: Option<crate::selection::Selection>,
        }
        fn selection_field<'de, D: serde::Deserializer<'de>>(
            d: D,
        ) -> Result<Option<crate::selection::Selection>, D::Error> {
            let value = serde_json::Value::deserialize(d)?;
            if value.get("schema_version").and_then(|v| v.as_u64()) != Some(3)
                && (value.get("provisioning").is_some() || value.get("contracts").is_some())
            {
                return Err(serde::de::Error::custom(
                    "legacy selection cannot contain project-owned fields",
                ));
            }
            serde_json::from_value(value)
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
        let wire = Wire::deserialize(deserializer)?;
        let profile = Self {
            schema_version: wire.schema_version,
            id: wire.id,
            version: wire.version,
            components: wire.components,
            sensors: wire.sensors,
            adequacy: wire.adequacy,
            selection: wire.selection,
        };
        profile.validate().map_err(serde::de::Error::custom)?;
        Ok(profile)
    }
}

fn profile_schema(schema: &mut schemars::Schema) {
    schema.insert("allOf".into(), serde_json::json!([
        {"if":{"properties":{"schema_version":{"const":2}}},"then":{"properties":{"selection":{
            "properties":{"schema_version":{"const":1}},
            "not":{"required":["consumer"]},
            "allOf":[{"properties":{"tools":{"additionalProperties":{
                "not":{"required":["producer_contract"]},
                "properties":{"version":{"required":["requirement","normalization","provision"]}}
            }}}},
                {"properties":{"packages":{"items":{"required":["requirement"],"not":{"required":["acceptance"]}}}}},
                {"not":{"anyOf":[{"required":["provisioning"]},{"required":["contracts"]}]}}]
        }}}},
        {"if":{"properties":{"schema_version":{"const":3}}},"then":{"properties":{"selection":{
            "properties":{"schema_version":{"const":2},
                "tools":{"additionalProperties":{"properties":{"version":{"required":["requirement","normalization","provision"]}}}},
                "packages":{"items":{"required":["requirement"],"not":{"required":["acceptance"]}}}},"required":["consumer"],
            "not":{"anyOf":[{"required":["provisioning"]},{"required":["contracts"]}]}
        }}}},
        {"if":{"properties":{"schema_version":{"const":4}}},"then":{"properties":{"selection":{
            "properties":{
                "schema_version":{"const":3},
                "tools":{"additionalProperties":{
                    "properties":{"version":{"required":["acceptance"]}},
                    "not":{"required":["producer_contract"]}
                }},
                "packages":{"items":{
                    "required":["acceptance"],
                    "not":{"required":["requirement"]}
                }}
            },
            "required":["contracts"]
        }}}}
    ]));
    schema.insert(
        "if".into(),
        serde_json::json!({"properties":{"schema_version":{"const":1}}}),
    );
    schema.insert("then".into(), serde_json::json!({
        "not":{"required":["selection"]},
        "properties":{
            "version":{"enum":["1.0.0","1.0.1","1.0.2"]},
            "sensors":{"items":{"properties":{
                "version_stdout":{"type":"string","pattern":"\\S"},
                "report":{"not":{"type":"object","required":["format"],"properties":{"format":{"const":"jest_json"}}}}
            }}}
        }
    }));
    schema.insert(
        "else".into(),
        serde_json::json!({"required":["selection"],"properties":{
        "version":{"pattern":"^(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)\\.(0|[1-9][0-9]*)$"},
        "sensors":{"items":{"properties":{"version_stdout":{"const":""}}}}}}),
    );
}

fn profile_version_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
    schemars::json_schema!({"type":"integer", "enum":[1,2,3,4]})
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: String,
    /// Relative to the explicit checkout root; dot denotes that root.
    pub root: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectSensor {
    pub definition: SensorDefinition,
    pub component: String,
    pub argv: Vec<String>,
    pub version_args: Vec<String>,
    /// Exact trimmed stdout of the same executable's version probe.
    pub version_stdout: String,
    #[serde(default = "default_environment")]
    pub environment_allowlist: Vec<String>,
    pub report: Option<ReportSpec>,
    /// Component-relative manifests, configuration and lockfiles read before/after execution.
    pub inputs: Vec<String>,
    /// Exact installed package / lockfile / runtime-manifest values, also content-bound inputs.
    #[serde(default)]
    pub pins: Vec<JsonPin>,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct JsonPin {
    pub path: String,
    pub pointer: String,
    pub expected: String,
}

fn default_environment() -> Vec<String> {
    vec!["PATH".into()]
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReportSpec {
    pub format: ReportFormat,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Impact {
    Known { paths: Vec<String>, risk: Risk },
    Unknown,
}

impl<'de> Deserialize<'de> for Impact {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Internally tagged unit variants ignore remaining map entries. A private
        // zero-field struct variant makes `deny_unknown_fields` apply to `unknown`.
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum StrictImpact {
            Known { paths: Vec<String>, risk: Risk },
            Unknown {},
        }

        Ok(match StrictImpact::deserialize(deserializer)? {
            StrictImpact::Known { paths, risk } => Self::Known { paths, risk },
            StrictImpact::Unknown {} => Self::Unknown,
        })
    }
}

/// Uses the same schema generator as the CLI's schema output, then semantic checks.
pub fn parse_profile(json: &str) -> Result<ProjectProfile, Error> {
    let value: serde_json::Value = serde_json::from_str(json)?;
    let schema = serde_json::to_value(schemars::schema_for!(ProjectProfile))?;
    let validator = jsonschema::draft202012::options()
        .build(&schema)
        .map_err(|e| invalid(format!("project schema: {e}")))?;
    if !validator.is_valid(&value) {
        return Err(invalid("project profile does not match its JSON Schema"));
    }
    // Deserialize original bytes so duplicate fields are rejected, not collapsed by Value.
    let profile: ProjectProfile = serde_json::from_str(json)?;
    profile.validate()?;
    Ok(profile)
}

pub(crate) fn relative(path: &str, allow_root: bool) -> bool {
    if allow_root && path == "." {
        return true;
    }
    !path.is_empty()
        && !path.contains(['\\', '\0', ':'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn identifier(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
}

impl ProjectProfile {
    pub fn validate(&self) -> Result<(), Error> {
        if !matches!(self.schema_version, 1..=4)
            || !identifier(&self.id)
            || (self.schema_version == 1
                && (!matches!(self.version.as_str(), "1.0.0" | "1.0.1" | "1.0.2")
                    || self.selection.is_some()))
            || (self.schema_version >= 2
                && (self.selection.is_none()
                    || crate::selection::numeric_version(&self.version).is_err()))
            || self.components.is_empty()
            || self.sensors.is_empty()
        {
            return Err(invalid("unsupported or empty project profile"));
        }
        let mut components = BTreeSet::new();
        for component in &self.components {
            if !identifier(&component.id)
                || !relative(&component.root, true)
                || !components.insert(component.id.as_str())
            {
                return Err(invalid("invalid or duplicate component"));
            }
        }
        let mut ids = BTreeSet::new();
        for requirement_ids in [
            &self.adequacy.acceptance_criterion_ids,
            &self.adequacy.changed_behavior_ids,
            &self.adequacy.negative_case_ids,
        ] {
            let mut seen = BTreeSet::new();
            for id in requirement_ids {
                if !id.split_once(':').is_some_and(|(component, name)| {
                    components.contains(component) && identifier(name)
                }) || !seen.insert(id)
                {
                    return Err(invalid(
                        "adequacy requirement IDs must have unique component namespaces",
                    ));
                }
            }
        }
        let mut reports = BTreeSet::new();
        for sensor in &self.sensors {
            if self.schema_version == 1
                && sensor
                    .report
                    .as_ref()
                    .is_some_and(|r| r.format == ReportFormat::JestJson)
            {
                return Err(invalid("Jest report contract requires profile schema2"));
            }
            let id = sensor.definition.id.as_str();
            if !components.contains(sensor.component.as_str())
                || !id.starts_with(&format!("{}:", sensor.component))
                || !identifier(id.split_once(':').map_or("", |(_, suffix)| suffix))
                || !ids.insert(id)
                || sensor.argv.is_empty()
                || sensor.argv[0].trim().is_empty()
                || sensor.version_args.is_empty()
                || (self.schema_version == 1 && sensor.version_stdout.trim().is_empty())
                || sensor
                    .argv
                    .iter()
                    .chain(&sensor.version_args)
                    .any(|v| v.contains('\0'))
                || sensor.inputs.is_empty()
                || sensor.inputs.iter().any(|p| !relative(p, false))
                || sensor.inputs.iter().collect::<BTreeSet<_>>().len() != sensor.inputs.len()
                || !(1..=evidence_collector::MAX_TIMEOUT_MS).contains(&sensor.timeout_ms)
                || sensor.definition.accepted_exit_codes != [0]
                || !sensor.definition.not_applicable_when.is_empty()
                || sensor.definition.result_binding.is_some()
                || sensor.environment_allowlist.iter().any(|key| {
                    !matches!(
                        key.as_str(),
                        "PATH" | "HOME" | "DOTNET_ROOT" | "DOTNET_CLI_HOME" | "NUGET_PACKAGES"
                    )
                })
                || sensor
                    .environment_allowlist
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != sensor.environment_allowlist.len()
            {
                return Err(invalid(format!("invalid project sensor {id}")));
            }
            if sensor.pins.iter().any(|pin| {
                !sensor.inputs.contains(&pin.path)
                    || !pin.pointer.starts_with('/')
                    || pin.expected.trim().is_empty()
            }) {
                return Err(invalid("package pins must address checked input files"));
            }
            match (&sensor.report, sensor.definition.result_requirement) {
                (None, Some(SensorResultRequirement::Process)) => {}
                (Some(report), Some(SensorResultRequirement::TestCounts)) => {
                    if !relative(&report.path, false)
                        || sensor.inputs.contains(&report.path)
                        || !reports.insert((sensor.component.as_str(), report.path.as_str()))
                    {
                        return Err(invalid(
                            "unsafe, duplicate or input-overwriting report path",
                        ));
                    }
                }
                _ => return Err(invalid("report and result requirement disagree")),
            }
        }
        // Project family names carry explicit composition obligations.
        let mandatory: &[&str] = match self.id.as_str() {
            "react-typescript" => &["frontend"],
            "dotnet" => &["backend"],
            "fullstack" if self.schema_version == 3 => &["frontend", "backend"],
            "fullstack" => &["frontend", "backend", "shared"],
            _ => return Err(invalid("unsupported project family")),
        };
        if mandatory.iter().any(|id| !components.contains(id)) {
            return Err(invalid("required project component is missing"));
        }
        for component in mandatory.iter().filter(|id| **id != "shared") {
            if !self
                .sensors
                .iter()
                .any(|s| s.component == *component && s.definition.required && s.report.is_some())
            {
                return Err(invalid(
                    "each application component needs a required test suite",
                ));
            }
        }
        if self.schema_version != 3
            && self.id == "fullstack"
            && ["shared:contract", "shared:e2e"].iter().any(|id| {
                !self.sensors.iter().any(|s| {
                    s.definition.id.as_str() == *id && s.definition.required && s.report.is_some()
                })
            })
        {
            return Err(invalid(
                "fullstack requires explicit contract and E2E sensors",
            ));
        }
        policy_core::resolve_required_sensors(&self.sensor_profile(), &[], Risk::Low)
            .map_err(|e| invalid(e.to_string()))?;
        if let Some(selection) = &self.selection {
            selection.validate(self)?;
        }
        Ok(())
    }

    pub fn sensor_profile(&self) -> SensorProfile {
        SensorProfile {
            schema_version: 2,
            sensors: self.sensors.iter().map(|s| s.definition.clone()).collect(),
            adequacy: self.adequacy.clone(),
        }
    }

    pub fn selected(&self, impact: &Impact) -> Result<Vec<&ProjectSensor>, Error> {
        self.validate()?;
        match impact {
            Impact::Unknown => Ok(self.sensors.iter().collect()),
            Impact::Known { paths, risk } => {
                if paths.iter().any(|p| !relative(p, false)) {
                    return Err(invalid("changed paths must be checkout-relative"));
                }
                let refs: Vec<_> = paths.iter().map(String::as_str).collect();
                let ids =
                    policy_core::resolve_required_sensors(&self.sensor_profile(), &refs, *risk)
                        .map_err(|e| invalid(e.to_string()))?;
                Ok(self
                    .sensors
                    .iter()
                    .filter(|s| ids.contains(&s.definition.id))
                    .collect())
            }
        }
    }
}
