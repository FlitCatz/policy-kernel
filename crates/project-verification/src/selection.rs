//! Versioned consumer choices. Ranges authorize compatibility, never provenance.
use crate::{
    invalid,
    profile::{relative, ProjectProfile, ProjectSensor},
    Error,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub schema_version: u32,
    pub tools: BTreeMap<String, ToolSelection>,
    /// Sensor ID to selected executable/probe contract. Every sensor must be bound.
    pub bindings: BTreeMap<String, String>,
    pub packages: Vec<PackageSelection>,
    pub graphs: Vec<DependencyGraph>,
    pub runtime_files: Vec<RuntimeFile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consumer: Option<crate::consumer::ConsumerContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolSelection {
    pub executable: String,
    pub version_args: Vec<String>,
    pub version: VersionRule,
    pub capabilities: BTreeSet<Capability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_contract: Option<crate::consumer::ProducerContract>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct VersionRule {
    /// Comma-separated numeric comparisons: =, >=, >, <=, <. No implicit wildcards.
    pub requirement: String,
    pub normalization: VersionNormalization,
    /// Exact provisioning choice, distinct from a runtime observation.
    pub provision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VersionNormalization {
    Plain,
    Node,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Process,
    VitestJson,
    Trx,
    PlaywrightJson,
    JestJson,
    NpmAudit,
    NugetAudit,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PackageSelection {
    pub component: String,
    pub installed_manifest: String,
    pub lockfile: String,
    pub lock_pointer: String,
    pub requirement: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GraphRole {
    Application,
    Unit,
    Integration,
    Shared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DependencyAdapter {
    Npm,
    Nuget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DependencyGraph {
    pub id: String,
    pub component: String,
    pub role: GraphRole,
    pub manifest: String,
    pub lockfile: String,
    pub adapter: DependencyAdapter,
    pub tool: String,
    pub audit_sensor: String,
    pub target_framework: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeFileFormat {
    NodeVersion,
    DotnetGlobal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimeFile {
    pub component: String,
    pub path: String,
    pub tool: String,
    pub format: RuntimeFileFormat,
}

/// Stable numeric versions only; rejecting unsupported syntax avoids ambiguous widening.
pub fn numeric_version(text: &str) -> Result<[u64; 3], Error> {
    let parts: Vec<_> = text.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.is_empty()
                || !p.bytes().all(|b| b.is_ascii_digit())
                || (p.len() > 1 && p.starts_with('0'))
        })
    {
        return Err(invalid(format!("unparseable stable version: {text}")));
    }
    let mut version = [0; 3];
    for (slot, part) in version.iter_mut().zip(parts) {
        *slot = part
            .parse()
            .map_err(|_| invalid("version integer overflow"))?;
    }
    Ok(version)
}

pub fn version_matches(requirement: &str, actual: &str) -> Result<bool, Error> {
    let actual = numeric_version(actual)?;
    let mut matched = true;
    if requirement.trim().is_empty() {
        return Err(invalid("empty version requirement"));
    }
    for clause in requirement.split(',') {
        let clause = clause.trim();
        let (operator, version) = [">=", "<=", "=", ">", "<"]
            .into_iter()
            .find_map(|op| clause.strip_prefix(op).map(|v| (op, v.trim())))
            .ok_or_else(|| invalid(format!("unsupported version requirement: {clause}")))?;
        let version = numeric_version(version)?;
        matched &= match operator {
            ">=" => actual >= version,
            "<=" => actual <= version,
            ">" => actual > version,
            "<" => actual < version,
            _ => actual == version,
        };
    }
    Ok(matched)
}

impl VersionRule {
    pub fn validate(&self) -> Result<(), Error> {
        if !version_matches(&self.requirement, &self.provision)? {
            return Err(invalid("provisioning version is outside requested range"));
        }
        Ok(())
    }
    pub fn observe(&self, stdout: &[u8]) -> Result<String, Error> {
        let text = std::str::from_utf8(stdout)
            .map_err(|_| invalid("version output is not UTF-8"))?
            .trim();
        let normalized = match self.normalization {
            VersionNormalization::Plain => text,
            VersionNormalization::Node => text
                .strip_prefix('v')
                .ok_or_else(|| invalid("Node version requires one leading v"))?,
        };
        if !version_matches(&self.requirement, normalized)? {
            return Err(invalid(format!(
                "version {normalized} is outside {}",
                self.requirement
            )));
        }
        Ok(normalized.into())
    }
}

impl ToolSelection {
    fn validate_capabilities(&self, version: &str) -> Result<(), Error> {
        for capability in &self.capabilities {
            let supported = match capability {
                Capability::Process => continue,
                Capability::VitestJson => ">=3.2.4, <5.0.0",
                Capability::JestJson => ">=30.2.0, <31.0.0",
                Capability::PlaywrightJson => ">=1.55.0, <1.56.0",
                Capability::Trx | Capability::NugetAudit => ">=9.0.100, <10.0.0",
                Capability::NpmAudit => ">=10.9.3, <11.0.0",
            };
            if !version_matches(supported, version)? {
                return Err(invalid(format!("unsupported producer capability {capability:?} at {version}; adapter requires {supported}")));
            }
        }
        Ok(())
    }

    pub fn observe(&self, stdout: &[u8]) -> Result<String, Error> {
        if let Some(producer) = self.producer_contract {
            producer.validate(self)?;
            if std::str::from_utf8(stdout).ok().map(str::trim) != Some(producer.version()) {
                return Err(invalid(
                    "observed version differs from exact producer contract",
                ));
            }
            return Ok(producer.version().into());
        }
        let version = self.version.observe(stdout)?;
        self.validate_capabilities(&version)?;
        Ok(version)
    }
}

fn capability(sensor: &ProjectSensor) -> Capability {
    use crate::reports::ReportFormat;
    match sensor.report.as_ref().map(|r| r.format) {
        Some(ReportFormat::VitestJson) => Capability::VitestJson,
        Some(ReportFormat::Trx) => Capability::Trx,
        Some(ReportFormat::PlaywrightJson) => Capability::PlaywrightJson,
        Some(ReportFormat::JestJson) => Capability::JestJson,
        None => Capability::Process,
    }
}

impl Selection {
    pub fn tool_for(&self, sensor: &ProjectSensor) -> Result<&ToolSelection, Error> {
        self.bindings
            .get(sensor.definition.id.as_str())
            .and_then(|key| self.tools.get(key))
            .ok_or_else(|| {
                invalid(format!(
                    "missing selected tool for {}",
                    sensor.definition.id
                ))
            })
    }

    pub fn validate(&self, profile: &ProjectProfile) -> Result<(), Error> {
        if !matches!(
            (profile.schema_version, self.schema_version),
            (2, 1) | (3, 2)
        ) || (self.schema_version == 1
            && (self.consumer.is_some()
                || self.tools.values().any(|t| t.producer_contract.is_some())))
            || (self.schema_version == 2 && self.consumer.is_none())
            || self.tools.is_empty()
            || self.graphs.is_empty()
            || self.bindings.len() != profile.sensors.len()
        {
            return Err(invalid("unsupported or incomplete selection contract"));
        }
        let component = |id: &str| profile.components.iter().any(|c| c.id == id);
        let used: BTreeSet<_> = self
            .bindings
            .values()
            .chain(self.graphs.iter().map(|g| &g.tool))
            .chain(self.runtime_files.iter().map(|r| &r.tool))
            .collect();
        for (id, tool) in &self.tools {
            if id.trim().is_empty()
                || !used.contains(id)
                || tool.executable.trim().is_empty()
                || tool.executable.contains('\0')
                || tool.version_args.is_empty()
                || tool.version_args.iter().any(|a| a.contains('\0'))
                || tool.capabilities.is_empty()
            {
                return Err(invalid("invalid or unused selected tool"));
            }
            if let Some(producer) = tool.producer_contract {
                producer.validate(tool)?;
                if producer == crate::consumer::ProducerContract::Dotnet11Rc1
                    && !self.consumer.as_ref().is_some_and(|c| c.development_only)
                {
                    return Err(invalid(
                        "RC SDK requires an explicit development-only consumer",
                    ));
                }
            } else {
                tool.version.validate()?;
                tool.validate_capabilities(&tool.version.provision)?;
            }
        }
        for sensor in &profile.sensors {
            let tool = self.tool_for(sensor)?;
            if sensor.argv.first() != Some(&tool.executable)
                || sensor.version_args != tool.version_args
                || !sensor.version_stdout.is_empty()
                || !tool.capabilities.contains(&capability(sensor))
            {
                return Err(invalid(format!(
                    "conflicting command, probe or unsupported capability: {}",
                    sensor.definition.id
                )));
            }
        }
        let mut graph_ids = BTreeSet::new();
        let mut manifests = BTreeSet::new();
        let mut audits = BTreeSet::new();
        for graph in &self.graphs {
            if graph.id.is_empty()
                || !graph_ids.insert(&graph.id)
                || !component(&graph.component)
                || !relative(&graph.manifest, false)
                || !relative(&graph.lockfile, false)
                || !manifests.insert((&graph.component, &graph.manifest))
                || !audits.insert(&graph.audit_sensor)
            {
                return Err(invalid("invalid or duplicate dependency graph"));
            }
            let tool = self
                .tools
                .get(&graph.tool)
                .ok_or_else(|| invalid("missing dependency tool"))?;
            let sensor = profile
                .sensors
                .iter()
                .find(|s| s.definition.id.as_str() == graph.audit_sensor)
                .ok_or_else(|| invalid("missing graph audit sensor"))?;
            let required_capability = match graph.adapter {
                DependencyAdapter::Npm => Capability::NpmAudit,
                DependencyAdapter::Nuget => Capability::NugetAudit,
            };
            let (directory, filename) = graph
                .manifest
                .rsplit_once('/')
                .unwrap_or(("", &graph.manifest));
            let lock_name = match graph.adapter {
                DependencyAdapter::Npm => "package-lock.json",
                DependencyAdapter::Nuget => "packages.lock.json",
            };
            let expected_lock = if directory.is_empty() {
                lock_name.into()
            } else {
                format!("{directory}/{lock_name}")
            };
            if graph.lockfile != expected_lock
                || match graph.adapter {
                    DependencyAdapter::Npm => filename != "package.json",
                    DependencyAdapter::Nuget => !filename.ends_with(".csproj"),
                }
            {
                return Err(invalid("graph must bind the manifest and adjacent lockfile actually consumed by its adapter"));
            }
            if !tool.capabilities.contains(&required_capability)
                || !sensor.definition.required
                || sensor.component != graph.component
                || sensor.report.is_some()
                || !sensor.inputs.contains(&graph.manifest)
                || !sensor.inputs.contains(&graph.lockfile)
                || self.bindings.get(&graph.audit_sensor) != Some(&graph.tool)
                || sensor.argv != graph.audit_argv(tool)
            {
                return Err(invalid(format!(
                    "incomplete or conflicting audit contract: {}",
                    graph.id
                )));
            }
            match graph.adapter {
                DependencyAdapter::Npm if graph.target_framework.is_some() => {
                    return Err(invalid("npm graph cannot select .NET framework"))
                }
                DependencyAdapter::Nuget
                    if !graph.target_framework.as_ref().is_some_and(|v| {
                        v.strip_prefix("net").is_some_and(|v| {
                            v.split('.').count() == 2
                                && v.split('.').all(|part| {
                                    !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())
                                })
                        })
                    }) =>
                {
                    return Err(invalid("NuGet graph needs an explicit target framework"))
                }
                _ => {}
            }
            if graph.adapter == DependencyAdapter::Nuget
                && !sensor.inputs.iter().any(|path| path == "NuGet.Config")
            {
                return Err(invalid(
                    "NuGet audit requires its explicit NuGet.Config input",
                ));
            }
            if matches!(graph.role, GraphRole::Unit | GraphRole::Integration)
                && !profile.sensors.iter().any(|s| {
                    s.component == graph.component
                        && s.definition.required
                        && s.report.is_some()
                        && s.inputs.contains(&graph.manifest)
                        && s.inputs.contains(&graph.lockfile)
                        && s.argv.contains(&graph.manifest)
                })
            {
                return Err(invalid(format!(
                    "missing required test for dependency graph: {}",
                    graph.id
                )));
            }
        }
        if let Some(consumer) = &self.consumer {
            consumer.validate(self, profile)?;
        }
        // Frozen profiles retain their preset composition obligations.
        for c in profile
            .components
            .iter()
            .filter(|_| self.schema_version == 1)
        {
            let roles: &[GraphRole] = match c.id.as_str() {
                "frontend" => &[GraphRole::Application],
                "backend" => &[
                    GraphRole::Application,
                    GraphRole::Unit,
                    GraphRole::Integration,
                ],
                "shared" => &[GraphRole::Shared],
                _ => &[],
            };
            for role in roles {
                if !self
                    .graphs
                    .iter()
                    .any(|g| g.component == c.id && g.role == *role)
                {
                    return Err(invalid(format!(
                        "missing {:?} dependency graph for {}",
                        role, c.id
                    )));
                }
            }
        }
        // A selected project/lockfile used by any sensor cannot escape the graph inventory.
        for sensor in &profile.sensors {
            for input in &sensor.inputs {
                if (input.ends_with(".csproj")
                    || input == "package.json"
                    || input.ends_with("/package.json") && !input.starts_with("node_modules/"))
                    && !self
                        .graphs
                        .iter()
                        .any(|g| g.component == sensor.component && g.manifest == *input)
                {
                    return Err(invalid(format!(
                        "unaudited manifest: {}:{input}",
                        sensor.component
                    )));
                }
            }
        }
        let mut packages = BTreeSet::new();
        for package in &self.packages {
            if !component(&package.component)
                || !relative(&package.installed_manifest, false)
                || !relative(&package.lockfile, false)
                || !package.lock_pointer.starts_with('/')
                || !packages.insert((&package.component, &package.installed_manifest))
                || !self
                    .graphs
                    .iter()
                    .any(|g| g.component == package.component && g.lockfile == package.lockfile)
            {
                return Err(invalid("invalid package selection or missing graph"));
            }
            // Parse every comparison without assuming that a particular installed version matches.
            version_matches(&package.requirement, "0.0.0")?;
        }
        let mut runtime_files = BTreeSet::new();
        for file in &self.runtime_files {
            if !component(&file.component)
                || !relative(&file.path, false)
                || !self.tools.contains_key(&file.tool)
                || !runtime_files.insert((&file.component, &file.path))
            {
                return Err(invalid("invalid or duplicate runtime file"));
            }
        }
        Ok(())
    }

    /// Inputs added to each component's binding, so policy approval covers all choices.
    pub fn inputs(&self, component: &str) -> BTreeSet<String> {
        self.graphs
            .iter()
            .filter(|g| g.component == component)
            .flat_map(|g| [g.manifest.clone(), g.lockfile.clone()])
            .chain(
                self.packages
                    .iter()
                    .filter(|p| p.component == component)
                    .flat_map(|p| [p.installed_manifest.clone(), p.lockfile.clone()]),
            )
            .chain(
                self.runtime_files
                    .iter()
                    .filter(|r| r.component == component)
                    .map(|r| r.path.clone()),
            )
            .chain(
                self.consumer
                    .iter()
                    .flat_map(|c| c.framework_props.iter())
                    .filter(|(id, _)| {
                        self.graphs
                            .iter()
                            .any(|g| &g.id == *id && g.component == component)
                    })
                    .map(|(_, path)| path.clone()),
            )
            .collect()
    }

    pub fn validate_runtime_file(&self, file: &RuntimeFile, bytes: &[u8]) -> Result<(), Error> {
        let expected = &self
            .tools
            .get(&file.tool)
            .ok_or_else(|| invalid("runtime tool absent"))?
            .version
            .provision;
        let text = std::str::from_utf8(bytes)
            .map_err(|_| invalid("runtime configuration is not UTF-8"))?;
        let actual = match file.format {
            RuntimeFileFormat::NodeVersion => text.trim().to_owned(),
            RuntimeFileFormat::DotnetGlobal => {
                let value: serde_json::Value = serde_json::from_str(text)?;
                if let Some(policy) = self.consumer.as_ref().and_then(|c| {
                    c.runtime_policies
                        .get(&format!("{}:{}", file.component, file.path))
                }) {
                    if !matches!(policy.roll_forward.as_str(), "disable" | "latestPatch")
                        || value.pointer("/sdk/rollForward").and_then(|v| v.as_str())
                            != Some(policy.roll_forward.as_str())
                        || value
                            .pointer("/sdk/allowPrerelease")
                            .and_then(|v| v.as_bool())
                            != Some(policy.allow_prerelease)
                    {
                        return Err(invalid("global.json differs from declared runtime policy"));
                    }
                } else if value.pointer("/sdk/rollForward").and_then(|v| v.as_str())
                    != Some("disable")
                    || value
                        .pointer("/sdk/allowPrerelease")
                        .and_then(|v| v.as_bool())
                        == Some(true)
                {
                    return Err(invalid(
                        "global.json must disable implicit SDK migration/prereleases",
                    ));
                }
                value
                    .pointer("/sdk/version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| invalid("missing SDK selection"))?
                    .into()
            }
        };
        if &actual != expected {
            return Err(invalid(format!(
                "conflicting runtime selection in {}",
                file.path
            )));
        }
        Ok(())
    }
}

impl DependencyGraph {
    fn command_prefix(&self, tool: &ToolSelection) -> Vec<String> {
        let mut argv = vec![tool.executable.clone()];
        if self.adapter == DependencyAdapter::Npm {
            if let Some((directory, _)) = self.manifest.rsplit_once('/') {
                argv.extend(["--prefix".into(), directory.into()]);
            }
        }
        argv
    }

    pub fn install_argv(&self, tool: &ToolSelection) -> Vec<String> {
        let args: Vec<&str> = match self.adapter {
            DependencyAdapter::Npm => vec![
                "ci",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
                "--include=dev",
                "--include=optional",
                "--include=peer",
            ],
            DependencyAdapter::Nuget => vec!["restore", &self.manifest, "--locked-mode"],
        };
        self.command_prefix(tool)
            .into_iter()
            .chain(args.into_iter().map(str::to_owned))
            .collect()
    }
    pub fn audit_argv(&self, tool: &ToolSelection) -> Vec<String> {
        let args: Vec<&str> = match self.adapter {
            DependencyAdapter::Npm => vec![
                "audit",
                "--audit-level=low",
                "--include=dev",
                "--include=optional",
                "--include=peer",
            ],
            DependencyAdapter::Nuget => vec![
                "restore",
                &self.manifest,
                "--locked-mode",
                "--force",
                "--no-http-cache",
                "-p:NuGetAudit=true",
                "-p:NuGetAuditMode=all",
                "-p:NuGetAuditLevel=low",
                "-p:WarningsAsErrors=NU1900%3BNU1901%3BNU1902%3BNU1903%3BNU1904%3BNU1905",
                "--configfile",
                "NuGet.Config",
                "-p:NoWarn=",
                "-p:WarningsNotAsErrors=",
            ],
        };
        self.command_prefix(tool)
            .into_iter()
            .chain(args.into_iter().map(str::to_owned))
            .collect()
    }
}

/// Explicit migration used to publish new assets; old releases never call this implicitly.
pub fn upgrade_profile(mut profile: ProjectProfile) -> Result<ProjectProfile, Error> {
    profile.validate()?;
    if profile.schema_version != 1 || profile.version != "1.0.2" {
        return Err(invalid("migration requires a legacy 1.0.2 profile"));
    }
    let mut selection = Selection {
        schema_version: 1,
        tools: BTreeMap::new(),
        bindings: BTreeMap::new(),
        packages: vec![],
        graphs: vec![],
        runtime_files: vec![],
        consumer: None,
    };
    let mut extra_sensors = Vec::new();
    for component in &profile.components {
        let audit_id = format!("{}:dependencies", component.id);
        if component.id == "backend" {
            let audit = profile
                .sensors
                .iter()
                .find(|s| s.definition.id.as_str() == audit_id)
                .ok_or_else(|| invalid("legacy profile has no backend audit"))?;
            for (role, manifest, lockfile, suffix) in [
                (
                    GraphRole::Application,
                    "App.csproj",
                    "packages.lock.json",
                    "dependencies",
                ),
                (
                    GraphRole::Unit,
                    "Tests/Unit/Unit.Tests.csproj",
                    "Tests/Unit/packages.lock.json",
                    "unit-dependencies",
                ),
                (
                    GraphRole::Integration,
                    "Tests/Integration/Integration.Tests.csproj",
                    "Tests/Integration/packages.lock.json",
                    "integration-dependencies",
                ),
            ] {
                let id = format!("backend:{suffix}");
                let graph = DependencyGraph {
                    id: format!("backend-{suffix}"),
                    component: component.id.clone(),
                    role,
                    manifest: manifest.into(),
                    lockfile: lockfile.into(),
                    adapter: DependencyAdapter::Nuget,
                    tool: id.clone(),
                    audit_sensor: id.clone(),
                    target_framework: Some("net9.0".into()),
                };
                if role != GraphRole::Application {
                    let mut sensor = audit.clone();
                    sensor.definition.id = policy_contracts::SensorId::parse(&id)
                        .map_err(|e| invalid(e.to_string()))?;
                    sensor.argv[2] = manifest.into();
                    sensor.inputs.extend([manifest.into(), lockfile.into()]);
                    extra_sensors.push(sensor);
                }
                selection.graphs.push(graph);
            }
            selection.runtime_files.push(RuntimeFile {
                component: component.id.clone(),
                path: "global.json".into(),
                tool: audit_id,
                format: RuntimeFileFormat::DotnetGlobal,
            });
        } else {
            selection.graphs.push(DependencyGraph {
                id: format!("{}-packages", component.id),
                component: component.id.clone(),
                role: if component.id == "shared" {
                    GraphRole::Shared
                } else {
                    GraphRole::Application
                },
                manifest: "package.json".into(),
                lockfile: "package-lock.json".into(),
                adapter: DependencyAdapter::Npm,
                tool: audit_id.clone(),
                audit_sensor: audit_id,
                target_framework: None,
            });
            let tool = profile
                .sensors
                .iter()
                .find(|s| s.component == component.id && s.version_stdout.starts_with('v'))
                .or_else(|| {
                    profile
                        .sensors
                        .iter()
                        .find(|s| s.version_stdout.starts_with('v'))
                })
                .ok_or_else(|| invalid("no Node runtime contract"))?;
            selection.runtime_files.push(RuntimeFile {
                component: component.id.clone(),
                path: ".node-version".into(),
                tool: tool.definition.id.to_string(),
                format: RuntimeFileFormat::NodeVersion,
            });
        }
    }
    profile.sensors.extend(extra_sensors);
    let mut packages = BTreeSet::new();
    for sensor in &mut profile.sensors {
        let id = sensor.definition.id.to_string();
        let (normalization, version) = match sensor.version_stdout.strip_prefix('v') {
            Some(v) => (VersionNormalization::Node, v.to_owned()),
            None => (VersionNormalization::Plain, sensor.version_stdout.clone()),
        };
        let mut capabilities = BTreeSet::from([Capability::Process, capability(sensor)]);
        if let Some(graph) = selection.graphs.iter().find(|g| g.audit_sensor == id) {
            capabilities.insert(match graph.adapter {
                DependencyAdapter::Npm => Capability::NpmAudit,
                DependencyAdapter::Nuget => Capability::NugetAudit,
            });
        }
        let requirement = if normalization == VersionNormalization::Node {
            ">=22.18.0, <23.0.0".into()
        } else if sensor.argv[0] == "npm" {
            ">=10.9.3, <11.0.0".into()
        } else if sensor.argv[0] == "dotnet" {
            ">=9.0.100, <10.0.0".into()
        } else {
            format!("={version}")
        };
        selection.tools.insert(
            id.clone(),
            ToolSelection {
                producer_contract: None,
                executable: sensor.argv[0].clone(),
                version_args: sensor.version_args.clone(),
                version: VersionRule {
                    requirement,
                    normalization,
                    provision: version,
                },
                capabilities,
            },
        );
        selection.bindings.insert(id.clone(), id);
        sensor.version_stdout.clear();
        for pin in &sensor.pins {
            if pin.path.starts_with("node_modules/")
                && pin.pointer == "/version"
                && packages.insert((sensor.component.clone(), pin.path.clone()))
            {
                let installed = pin
                    .path
                    .strip_suffix("/package.json")
                    .ok_or_else(|| invalid("installed package manifest path"))?;
                selection.packages.push(PackageSelection {
                    component: sensor.component.clone(),
                    installed_manifest: pin.path.clone(),
                    lockfile: "package-lock.json".into(),
                    lock_pointer: format!(
                        "/packages/{}/version",
                        installed.replace('~', "~0").replace('/', "~1")
                    ),
                    requirement: format!("={}", pin.expected),
                });
            }
        }
        sensor.pins.retain(|pin| {
            !pin.path.starts_with("node_modules/")
                && pin.path != "package-lock.json"
                && pin.path != "global.json"
        });
    }
    for graph in &selection.graphs {
        let sensor = profile
            .sensors
            .iter_mut()
            .find(|sensor| sensor.definition.id.as_str() == graph.audit_sensor)
            .ok_or_else(|| invalid("missing migrated audit sensor"))?;
        sensor.argv = graph.audit_argv(&selection.tools[&graph.tool]);
    }
    profile.schema_version = 2;
    profile.version = "1.1.0".into();
    profile.selection = Some(selection);
    profile.validate()?;
    Ok(profile)
}

/// Validate selected content using bytes read by the caller's safe filesystem boundary.
/// Missing files are errors at runtime; an installer can report them as prerequisites.
pub fn validate_component_files(
    selection: &Selection,
    component: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), Error> {
    validate_files(selection, component, files, false)
}

/// Installation may list absent prerequisites, while validating each available input.
pub fn validate_available_component_files(
    selection: &Selection,
    component: &str,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), Error> {
    validate_files(selection, component, files, true)
}

fn validate_files(
    selection: &Selection,
    component: &str,
    files: &BTreeMap<String, Vec<u8>>,
    allow_missing: bool,
) -> Result<(), Error> {
    let read = |path: &str| {
        files
            .get(path)
            .map(Vec::as_slice)
            .ok_or_else(|| invalid(format!("missing selected input: {component}:{path}")))
    };
    for runtime in selection
        .runtime_files
        .iter()
        .filter(|r| r.component == component)
    {
        if allow_missing && !files.contains_key(&runtime.path) {
            continue;
        }
        selection.validate_runtime_file(runtime, read(&runtime.path)?)?;
    }
    let optional_read = |path: &str| -> Result<Option<&[u8]>, Error> {
        if allow_missing && !files.contains_key(path) {
            Ok(None)
        } else {
            read(path).map(Some)
        }
    };
    for package in selection
        .packages
        .iter()
        .filter(|p| p.component == component)
    {
        let installed = optional_read(&package.installed_manifest)?
            .map(serde_json::from_slice::<serde_json::Value>)
            .transpose()?;
        let locked = optional_read(&package.lockfile)?
            .map(serde_json::from_slice::<serde_json::Value>)
            .transpose()?;
        let installed_version = installed
            .as_ref()
            .map(|value| {
                value
                    .get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| invalid("missing installed package version"))
            })
            .transpose()?;
        let locked_version = locked
            .as_ref()
            .map(|value| {
                value
                    .pointer(&package.lock_pointer)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| invalid("missing selected package lock version"))
            })
            .transpose()?;
        for version in [installed_version, locked_version].into_iter().flatten() {
            if !version_matches(&package.requirement, version)? {
                return Err(invalid(format!(
                    "package range mismatch: {}",
                    package.installed_manifest
                )));
            }
        }
        if installed_version
            .zip(locked_version)
            .is_some_and(|(installed, locked)| installed != locked)
        {
            return Err(invalid(format!(
                "installed/lock version mismatch: {}",
                package.installed_manifest
            )));
        }
    }
    for graph in selection.graphs.iter().filter(|g| g.component == component) {
        let manifest = optional_read(&graph.manifest)?;
        let lock = optional_read(&graph.lockfile)?;
        match graph.adapter {
            DependencyAdapter::Npm => {
                let manifest = manifest
                    .map(serde_json::from_slice::<serde_json::Value>)
                    .transpose()?;
                let lock = lock
                    .map(serde_json::from_slice::<serde_json::Value>)
                    .transpose()?;
                if manifest.as_ref().is_some_and(|value| !value.is_object()) {
                    return Err(invalid("npm manifest must be an object"));
                }
                if let Some(lock) = lock {
                    let locked_root = lock
                        .pointer("/packages/")
                        .filter(|value| value.is_object())
                        .ok_or_else(|| invalid("npm lockfile v2/v3 package inventory required"))?;
                    if !matches!(
                        lock.get("lockfileVersion").and_then(|v| v.as_u64()),
                        Some(2 | 3)
                    ) {
                        return Err(invalid("unsupported npm lockfile format"));
                    }
                    if let Some(manifest) = manifest {
                        for field in ["dependencies", "devDependencies", "optionalDependencies"] {
                            if manifest.get(field) != locked_root.get(field) {
                                return Err(invalid(format!(
                                    "manifest/lock declarations disagree: {} {field}",
                                    graph.id
                                )));
                            }
                        }
                    }
                }
            }
            DependencyAdapter::Nuget => {
                let framework = graph
                    .target_framework
                    .as_deref()
                    .ok_or_else(|| invalid("missing NuGet target framework"))?;
                if selection
                    .consumer
                    .as_ref()
                    .is_some_and(|c| c.requires_vstest17(selection, graph))
                {
                    crate::consumer::validate_vstest17(manifest, lock, framework)?;
                }
                if let Some(lock) = lock {
                    let lock: serde_json::Value = serde_json::from_slice(lock)?;
                    if !lock
                        .get("dependencies")
                        .and_then(|v| v.get(framework))
                        .is_some_and(|v| v.is_object())
                    {
                        return Err(invalid("missing target framework in NuGet lock inventory"));
                    }
                }
                if let Some(manifest) = manifest {
                    let text = std::str::from_utf8(manifest)
                        .map_err(|_| invalid("project XML is not UTF-8"))?;
                    let document =
                        roxmltree::Document::parse(text).map_err(|e| invalid(e.to_string()))?;
                    let targets: Vec<_> = document
                        .descendants()
                        .filter(|n| {
                            n.has_tag_name("TargetFramework") || n.has_tag_name("TargetFrameworks")
                        })
                        .collect();
                    let inherited = selection
                        .consumer
                        .as_ref()
                        .and_then(|c| c.framework_props.get(&graph.id));
                    if let Some(path) = inherited {
                        if let Some(props) = optional_read(path)? {
                            crate::consumer::validate_framework_xml(
                                text,
                                std::str::from_utf8(props)
                                    .map_err(|_| invalid("props XML is not UTF-8"))?,
                                framework,
                            )?;
                        }
                    } else if targets.len() != 1
                        || !targets[0].has_tag_name("TargetFramework")
                        || targets[0].text().map(str::trim) != Some(framework)
                        || targets[0]
                            .ancestors()
                            .any(|n| n.attribute("Condition").is_some())
                    {
                        return Err(invalid(format!(
                            "conflicting target framework: {}",
                            graph.manifest
                        )));
                    }
                    for reference in document
                        .descendants()
                        .filter(|n| n.has_tag_name("ProjectReference"))
                    {
                        let include = reference
                            .attribute("Include")
                            .ok_or_else(|| invalid("project reference without Include"))?;
                        let mut path: Vec<_> = graph.manifest.split('/').collect();
                        path.pop();
                        for part in include.split('/') {
                            match part {
                                ".." => {
                                    if path.pop().is_none() {
                                        return Err(invalid("project reference escapes component"));
                                    }
                                }
                                "." => {}
                                p => path.push(p),
                            }
                        }
                        let path = path.join("/");
                        if !relative(&path, false)
                            || !selection
                                .graphs
                                .iter()
                                .any(|g| g.component == component && g.manifest == path)
                        {
                            return Err(invalid(format!("unaudited project reference: {include}")));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

/// Provisioning inputs for hosted setup actions; values remain exact selections.
pub fn ci_inputs(profile: &ProjectProfile) -> Result<BTreeMap<String, String>, Error> {
    profile.validate()?;
    let selection = profile
        .selection
        .as_ref()
        .ok_or_else(|| invalid("CI inputs require an explicitly selected schema2 profile"))?;
    let mut result = BTreeMap::from([(
        "project_profile_digest".into(),
        policy_core::canonical_sha256(profile)?.to_string(),
    )]);
    let mut insert = |kind: &str, tool: &str| -> Result<(), Error> {
        let version = selection
            .tools
            .get(tool)
            .ok_or_else(|| invalid("missing CI tool"))?
            .version
            .provision
            .clone();
        if result
            .insert(kind.into(), version.clone())
            .is_some_and(|old| old != version)
        {
            return Err(invalid(format!(
                "CI job requires conflicting {kind} selections; split jobs"
            )));
        }
        Ok(())
    };
    for runtime in &selection.runtime_files {
        insert(
            match runtime.format {
                RuntimeFileFormat::NodeVersion => "node_version",
                RuntimeFileFormat::DotnetGlobal => "dotnet_version",
            },
            &runtime.tool,
        )?;
    }
    for graph in &selection.graphs {
        if graph.adapter == DependencyAdapter::Npm {
            insert("npm_version", &graph.tool)?;
        }
    }
    Ok(result)
}
