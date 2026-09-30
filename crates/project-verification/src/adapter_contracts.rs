use crate::{
    invalid,
    profile::ProjectProfile,
    selection::{Capability, DependencyAdapter, Selection},
    Error,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum AdapterContract {
    #[serde(rename = "process/v1")]
    ProcessV1,
    #[serde(rename = "vitest-json/v1")]
    VitestJsonV1,
    #[serde(rename = "jest-json/v1")]
    JestJsonV1,
    #[serde(rename = "playwright-json/v1")]
    PlaywrightJsonV1,
    #[serde(rename = "npm-ci-audit/v1")]
    NpmCiAuditV1,
}

/// A selected adapter contract names behavior validated by this build, not a producer version.
pub fn validate_adapter_contracts(
    selection: &Selection,
    profile: &ProjectProfile,
) -> Result<(), Error> {
    if selection.contracts.len() != selection.tools.len() {
        return Err(invalid("every project tool needs adapter contracts"));
    }
    for (id, tool) in &selection.tools {
        let declared = selection
            .contracts
            .get(id)
            .ok_or_else(|| invalid("missing adapter contracts"))?;
        if declared.is_empty() {
            return Err(invalid("empty adapter contracts"));
        }
        let mut expected = BTreeSet::new();
        for capability in &tool.capabilities {
            expected.insert(match capability {
                Capability::Process => AdapterContract::ProcessV1,
                Capability::VitestJson => AdapterContract::VitestJsonV1,
                Capability::JestJson => AdapterContract::JestJsonV1,
                Capability::PlaywrightJson => AdapterContract::PlaywrightJsonV1,
                Capability::NpmAudit => AdapterContract::NpmCiAuditV1,
                Capability::Trx | Capability::NugetAudit => {
                    return Err(invalid(
                        ".NET adapters are not supported in project selection",
                    ))
                }
            });
        }
        if &expected != declared {
            return Err(invalid(format!(
                "adapter contracts disagree with capabilities for {id}"
            )));
        }
    }
    for graph in &selection.graphs {
        if graph.adapter != DependencyAdapter::Npm
            || !selection
                .contracts
                .get(&graph.tool)
                .is_some_and(|contracts| contracts.contains(&AdapterContract::NpmCiAuditV1))
        {
            return Err(invalid("project graph needs npm-ci-audit/v1"));
        }
    }
    for sensor in &profile.sensors {
        let id = selection
            .bindings
            .get(sensor.definition.id.as_str())
            .ok_or_else(|| invalid("missing sensor binding"))?;
        let contracts = selection
            .contracts
            .get(id)
            .ok_or_else(|| invalid("missing sensor contracts"))?;
        let needed = match sensor.report.as_ref().map(|report| report.format) {
            None => AdapterContract::ProcessV1,
            Some(crate::reports::ReportFormat::VitestJson) => AdapterContract::VitestJsonV1,
            Some(crate::reports::ReportFormat::JestJson) => AdapterContract::JestJsonV1,
            Some(crate::reports::ReportFormat::PlaywrightJson) => AdapterContract::PlaywrightJsonV1,
            Some(crate::reports::ReportFormat::Trx) => {
                return Err(invalid("TRX is not supported in project selection"))
            }
        };
        if !contracts.contains(&needed) {
            return Err(invalid("sensor lacks required adapter contract"));
        }
    }
    Ok(())
}
