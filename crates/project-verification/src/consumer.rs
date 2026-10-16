//! Explicit opt-in contracts for consumers outside the frozen preset layouts.
use crate::{invalid, selection::*, Error};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const DOTNET11_RC1: &str = "11.0.100-rc.1.26425.128";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProducerContract {
    Dotnet11Rc1,
    Playwright163,
}

impl ProducerContract {
    pub fn version(self) -> &'static str {
        match self {
            Self::Dotnet11Rc1 => DOTNET11_RC1,
            Self::Playwright163 => "1.63.0",
        }
    }
    pub fn validate(self, tool: &ToolSelection) -> Result<(), Error> {
        let version = tool.version.legacy()?;
        if version.provision != self.version()
            || version.requirement != format!("={}", self.version())
            || version.normalization != VersionNormalization::Plain
            || tool.capabilities.iter().any(|c| !match self {
                Self::Dotnet11Rc1 => matches!(
                    c,
                    Capability::Process | Capability::Trx | Capability::NugetAudit
                ),
                Self::Playwright163 => {
                    matches!(c, Capability::Process | Capability::PlaywrightJson)
                }
            })
        {
            return Err(invalid(
                "producer contract requires its exact version and supported capabilities",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsumerContract {
    pub development_only: bool,
    pub suites: Vec<SuiteBinding>,
    pub framework_props: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    pub generated_files_graphs: BTreeSet<String>,
    pub runtime_policies: BTreeMap<String, RuntimePolicy>,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum SuiteRole {
    Unit,
    Integration,
    Contract,
    E2e,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SuiteBinding {
    pub role: SuiteRole,
    pub sensor: String,
    pub graph: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RuntimePolicy {
    pub roll_forward: String,
    pub allow_prerelease: bool,
}

impl ConsumerContract {
    pub fn requires_vstest17(&self, selection: &Selection, graph: &DependencyGraph) -> bool {
        self.suites.iter().filter(|s| s.graph == graph.id).any(|s| {
            selection
                .bindings
                .get(&s.sensor)
                .and_then(|id| selection.tools.get(id))
                .is_some_and(|t| {
                    t.producer_contract == Some(ProducerContract::Dotnet11Rc1)
                        && t.capabilities.contains(&Capability::Trx)
                })
        })
    }

    pub fn validate_suite_reports(
        &self,
        reports: &BTreeMap<String, crate::reports::ParsedReport>,
    ) -> Result<(), Error> {
        let mut seen = BTreeSet::new();
        for suite in &self.suites {
            let report = reports
                .get(&suite.sensor)
                .ok_or_else(|| invalid("required suite report is absent"))?;
            if !report.successful || report.executed == 0 {
                return Err(invalid("required suite did not execute successfully"));
            }
            for test in &report.tests {
                let identity = test
                    .id
                    .rsplit_once(':')
                    .ok_or_else(|| invalid("test identity lacks namespace"))?
                    .1;
                if !seen.insert((&suite.graph, identity)) {
                    return Err(invalid(
                        "same graph test counted in multiple required suites",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn validate(
        &self,
        selection: &Selection,
        profile: &crate::ProjectProfile,
    ) -> Result<(), Error> {
        crate::generated_files::validate_contract(self, selection, profile)?;
        let mut sensors = BTreeSet::new();
        let mut roles = BTreeSet::new();
        for suite in &self.suites {
            let graph = selection
                .graphs
                .iter()
                .find(|g| g.id == suite.graph)
                .ok_or_else(|| invalid("suite references an absent graph"))?;
            let sensor = profile
                .sensors
                .iter()
                .find(|s| s.definition.id.as_str() == suite.sensor)
                .ok_or_else(|| invalid("suite references an absent sensor"))?;
            if !sensors.insert(&suite.sensor)
                || !roles.insert((graph.component.as_str(), suite.role))
                || !sensor.definition.required
                || !sensor.definition.required_when.is_empty()
                || sensor.report.is_none()
                || sensor.component != graph.component
                || !sensor.inputs.contains(&graph.manifest)
                || !sensor.inputs.contains(&graph.lockfile)
                || (graph.adapter == DependencyAdapter::Nuget
                    && (!sensor.argv.contains(&graph.manifest)
                        || sensor.report.as_ref().unwrap().format
                            != crate::reports::ReportFormat::Trx))
                || (suite.role == SuiteRole::E2e
                    && sensor.report.as_ref().unwrap().format
                        != crate::reports::ReportFormat::PlaywrightJson)
            {
                return Err(invalid(
                    "suite must bind a distinct required test sensor to its actual graph",
                ));
            }
        }
        for component in &profile.components {
            let required: &[SuiteRole] = match component.id.as_str() {
                "frontend" => &[SuiteRole::Unit],
                "backend" => &[SuiteRole::Unit, SuiteRole::Integration],
                _ => &[],
            };
            if required
                .iter()
                .any(|r| !roles.contains(&(component.id.as_str(), *r)))
                || !selection
                    .graphs
                    .iter()
                    .any(|g| g.component == component.id && g.role == GraphRole::Application)
            {
                return Err(invalid(
                    "component lacks its required application graph or test suites",
                ));
            }
        }
        if profile.id == "fullstack"
            && [SuiteRole::Contract, SuiteRole::E2e]
                .iter()
                .any(|r| !roles.iter().any(|(_, role)| role == r))
        {
            return Err(invalid(
                "fullstack requires explicit contract and E2E suite bindings",
            ));
        }
        for (graph_id, path) in &self.framework_props {
            if path != "Directory.Build.props"
                || !selection
                    .graphs
                    .iter()
                    .any(|g| g.id == *graph_id && g.adapter == DependencyAdapter::Nuget)
            {
                return Err(invalid("framework inheritance requires a NuGet graph and component-root Directory.Build.props"));
            }
        }
        for (key, policy) in &self.runtime_policies {
            let runtime = selection
                .runtime_files
                .iter()
                .find(|r| {
                    format!("{}:{}", r.component, r.path) == *key
                        && r.format == RuntimeFileFormat::DotnetGlobal
                })
                .ok_or_else(|| invalid("runtime policy must bind an existing .NET runtime file"))?;
            let tool = selection
                .tools
                .get(&runtime.tool)
                .ok_or_else(|| invalid("runtime policy tool is absent"))?;
            if !matches!(policy.roll_forward.as_str(), "disable" | "latestPatch")
                || (policy.allow_prerelease
                    && (!self.development_only
                        || tool.producer_contract != Some(ProducerContract::Dotnet11Rc1)))
                || (policy.roll_forward != "disable"
                    && !tool.version.legacy().is_ok_and(|version| {
                        version.requirement == format!("={}", version.provision)
                    }))
            {
                return Err(invalid("runtime policy requires an exact SDK; prereleases require the development contract"));
            }
        }
        Ok(())
    }
}

/// The RC/TRX adapter is certified only with the selected VSTest producer.
pub fn validate_vstest17(
    project: Option<&[u8]>,
    lock: Option<&[u8]>,
    framework: &str,
) -> Result<(), Error> {
    if let Some(bytes) = project {
        let text = std::str::from_utf8(bytes).map_err(|_| invalid("test project is not UTF-8"))?;
        let doc = roxmltree::Document::parse(text).map_err(|e| invalid(e.to_string()))?;
        let references: Vec<_> = doc
            .descendants()
            .filter(|n| {
                n.has_tag_name("PackageReference")
                    && n.attribute("Include")
                        .is_some_and(|p| p.eq_ignore_ascii_case("Microsoft.NET.Test.Sdk"))
            })
            .collect();
        if references.len() != 1
            || references[0].attribute("Version") != Some("17.14.1")
            || references[0].ancestors().any(|n| {
                n.attributes()
                    .any(|a| a.name().eq_ignore_ascii_case("Condition"))
            })
            || references[0]
                .children()
                .any(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("Version"))
        {
            return Err(invalid(
                "RC TRX contract requires unconditional Microsoft.NET.Test.Sdk 17.14.1",
            ));
        }
    }
    if let Some(bytes) = lock {
        let value: serde_json::Value = serde_json::from_slice(bytes)?;
        let packages = value
            .get("dependencies")
            .and_then(|v| v.get(framework))
            .and_then(|v| v.as_object())
            .ok_or_else(|| invalid("test lock lacks selected framework"))?;
        let sdk: Vec<_> = packages
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("Microsoft.NET.Test.Sdk"))
            .collect();
        if sdk.len() != 1
            || sdk[0].1.get("resolved").and_then(|v| v.as_str()) != Some("17.14.1")
            || sdk[0].1.get("type").and_then(|v| v.as_str()) != Some("Direct")
        {
            return Err(invalid(
                "RC TRX lock must resolve Microsoft.NET.Test.Sdk exactly to 17.14.1",
            ));
        }
        for name in [
            "Microsoft.TestPlatform.TestHost",
            "Microsoft.TestPlatform.ObjectModel",
        ] {
            let resolved: Vec<_> = packages
                .iter()
                .filter(|(k, _)| k.eq_ignore_ascii_case(name))
                .collect();
            if resolved.len() != 1
                || resolved[0].1.get("resolved").and_then(|v| v.as_str()) != Some("17.14.1")
            {
                return Err(invalid(
                    "RC TRX test host and object model must resolve exactly to 17.14.1",
                ));
            }
        }
    }
    Ok(())
}

/// Validate only the explicitly supported MSBuild subset, without executing imports.
pub fn validate_framework_xml(project: &str, props: &str, expected: &str) -> Result<(), Error> {
    validate_framework_xml_with_generated_files(project, props, expected, false)
}

pub(crate) fn validate_framework_xml_with_generated_files(
    project: &str,
    props: &str,
    expected: &str,
    generated_files: bool,
) -> Result<(), Error> {
    fn document(text: &str, generated_files: bool) -> Result<roxmltree::Document<'_>, Error> {
        let doc = roxmltree::Document::parse(text).map_err(|e| invalid(e.to_string()))?;
        crate::generated_files::validate_xml(&doc, generated_files)?;
        // MSBuild property names are case-insensitive. Keep the supported subset
        // explicit: import hook properties can otherwise load unchecked XML.
        const SAFE_PROPERTIES: &[&str] = &[
            "TargetFramework",
            "Nullable",
            "ImplicitUsings",
            "TreatWarningsAsErrors",
            "RestorePackagesWithLockFile",
            "IsTestProject",
            "IsPackable",
            "OutputType",
            "LangVersion",
            "GenerateProgramFile",
        ];
        if !doc.root_element().has_tag_name("Project")
            || doc
                .root_element()
                .attribute("Sdk")
                .is_some_and(|s| !matches!(s, "Microsoft.NET.Sdk" | "Microsoft.NET.Sdk.Web"))
            || doc
                .root_element()
                .children()
                .filter(|n| n.is_element())
                .any(|n| !matches!(n.tag_name().name(), "PropertyGroup" | "ItemGroup"))
            || doc.descendants().any(|n| {
                (n.is_element()
                    && n.parent().is_some_and(|p| p.has_tag_name("PropertyGroup"))
                    && !SAFE_PROPERTIES
                        .iter()
                        .any(|p| p.eq_ignore_ascii_case(n.tag_name().name()))
                    && !(generated_files
                        && crate::generated_files::is_property(n.tag_name().name())))
                    || n.has_tag_name("Import")
                    || n.has_tag_name("ImportGroup")
                    || n.has_tag_name("Target")
                    || n.has_tag_name("TargetFrameworks")
                    || n.has_tag_name("DirectoryBuildPropsPath")
                    || n.has_tag_name("ImportDirectoryBuildProps")
                    || n.has_tag_name("DirectoryBuildTargetsPath")
                    || n.has_tag_name("ImportDirectoryBuildTargets")
                    || n.attributes()
                        .any(|a| a.name().eq_ignore_ascii_case("TreatAsLocalProperty"))
            })
        {
            return Err(invalid("unsupported MSBuild framework override/import"));
        }
        Ok(doc)
    }
    let project = document(project, generated_files)?;
    if project
        .descendants()
        .any(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("TargetFramework"))
    {
        return Err(invalid(
            "inherited framework conflicts with project framework",
        ));
    }
    let props = document(props, false)?;
    if props.descendants().any(|n| n.has_tag_name("ItemGroup")) {
        return Err(invalid(
            "framework props may contain only declared scalar properties",
        ));
    }
    let targets: Vec<_> = props
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name().eq_ignore_ascii_case("TargetFramework"))
        .collect();
    if targets.len() != 1
        || targets[0].text().map(str::trim) != Some(expected)
        || targets[0].ancestors().any(|n| {
            n.attributes()
                .any(|a| a.name().eq_ignore_ascii_case("Condition"))
        })
    {
        return Err(invalid(
            "inherited framework must be single, unconditional and match the graph",
        ));
    }
    Ok(())
}

/// Check absent implicit overrides as well as declared inputs before every collection.
pub fn validate_component_layout(
    selection: &Selection,
    component: &str,
    root: &std::path::Path,
) -> Result<(), Error> {
    let Some(consumer) = &selection.consumer else {
        return Ok(());
    };
    crate::generated_files::validate_layout(consumer, selection, component, root)?;
    if selection
        .graphs
        .iter()
        .any(|g| g.component == component && consumer.framework_props.contains_key(&g.id))
    {
        for ancestor in root.ancestors().skip(1) {
            for name in ["Directory.Build.targets", "Directory.Packages.props"] {
                match std::fs::symlink_metadata(ancestor.join(name)) {
                    Ok(_) => {
                        return Err(invalid(format!(
                            "unbound ancestor MSBuild configuration: {}",
                            ancestor.join(name).display()
                        )))
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Err(e.into()),
                }
            }
        }
    }
    for graph in selection
        .graphs
        .iter()
        .filter(|g| g.component == component && consumer.framework_props.contains_key(&g.id))
    {
        let mut directory = std::path::Path::new(&graph.manifest).parent();
        while let Some(dir) = directory {
            for name in [
                "Directory.Build.props",
                "Directory.Build.targets",
                "Directory.Packages.props",
            ] {
                let path = dir.join(name);
                if path == std::path::Path::new("Directory.Build.props") {
                    continue;
                }
                crate::paths::require_absent(
                    root,
                    path.to_str()
                        .ok_or_else(|| invalid("invalid MSBuild path"))?,
                )?;
            }
            directory = dir.parent();
        }
    }
    Ok(())
}
