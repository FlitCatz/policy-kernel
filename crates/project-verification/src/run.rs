use crate::{
    bytes_digest, invalid, paths,
    profile::{Impact, ProjectProfile, ProjectSensor},
    reports::{parse_report, ParsedReport, TestOutcome},
    Error,
};
use evidence_collector::{collect, CollectedExecution, CollectionRequest};
use policy_contracts::*;
use policy_core::canonical_sha256;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunRequest {
    pub schema_version: u32,
    pub profile: ProjectProfile,
    pub expected_profile_digest: Sha256Digest,
    pub root: PathBuf,
    pub repository: RepositoryId,
    pub head_sha: CommitSha,
    /// Explicit source inventory. CI is responsible for its completeness and actual head.
    pub source_files: Vec<ArtifactDigest>,
    pub expected_source_digest: Sha256Digest,
    pub provider: ProviderId,
    pub model: ModelId,
    pub execution_id: ExecutionId,
    pub impact: Impact,
    pub mappings: AdequacyMappings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdequacyMappings {
    pub acceptance_criteria: Vec<TestMapping>,
    pub changed_behaviors: Vec<TestMapping>,
    pub negative_case_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationBinding {
    pub schema_version: u32,
    pub project_profile_digest: Sha256Digest,
    pub sensor: ProjectSensor,
    pub component_root: PathBuf,
    pub inputs: Vec<ArtifactDigest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedResult {
    pub schema_version: u32,
    pub collection_digest: Sha256Digest,
    pub original_report: Option<ArtifactDigest>,
    pub parsed: Option<ParsedReport>,
    pub errors: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_preflight_digest: Option<Sha256Digest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedSensor {
    pub evidence: SensorEvidence,
    pub collection: CollectedExecution,
    pub configuration: ConfigurationBinding,
    pub result: NormalizedResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_preflight: Option<CollectedExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunResult {
    pub schema_version: u32,
    pub project_profile_digest: Sha256Digest,
    pub source_set_digest: Sha256Digest,
    /// Candidate for explicit policy adoption, never an approval or trusted policy update.
    pub bound_sensor_profile: SensorProfile,
    pub sensors: Vec<NormalizedSensor>,
    pub adequacy: TestAdequacyEvidence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedRun {
    pub schema_version: u32,
    pub bound_sensor_profile: SensorProfile,
    pub selected_sensor_ids: BTreeSet<SensorId>,
    pub configurations: Vec<ConfigurationBinding>,
}

fn source_files(request: &RunRequest) -> Result<Vec<ArtifactDigest>, Error> {
    let mut files = request.source_files.clone();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    if files.is_empty() || files.windows(2).any(|p| p[0].path == p[1].path) {
        return Err(invalid("source inventory must be nonempty and unique"));
    }
    for file in &files {
        if bytes_digest(&paths::read(&request.root, &file.path)?) != file.digest {
            return Err(invalid(format!("source content changed: {}", file.path)));
        }
    }
    if canonical_sha256(&files)? != request.expected_source_digest {
        return Err(invalid("source inventory digest mismatch"));
    }
    Ok(files)
}

fn configuration(
    request: &RunRequest,
    sensor: &ProjectSensor,
) -> Result<ConfigurationBinding, Error> {
    let component = request
        .profile
        .components
        .iter()
        .find(|c| c.id == sensor.component)
        .ok_or_else(|| invalid("missing component"))?;
    let cwd = paths::directory(&request.root, &component.root)?;
    if let Some(selection) = &request.profile.selection {
        crate::consumer::validate_component_layout(selection, &component.id, &cwd)?;
    }
    let mut inputs = Vec::new();
    let mut selected_inputs: BTreeSet<_> = sensor.inputs.iter().cloned().collect();
    if let Some(selection) = &request.profile.selection {
        selected_inputs.extend(selection.inputs(&sensor.component));
    }
    let mut selected_files = BTreeMap::new();
    for path in &selected_inputs {
        let bytes = paths::read(&cwd, path)?;
        let nuget_audit = request.profile.selection.as_ref().map_or(
            sensor.definition.id.as_str() == "backend:dependencies",
            |selection| {
                selection.graphs.iter().any(|g| {
                    g.audit_sensor == sensor.definition.id.as_str()
                        && g.adapter == crate::selection::DependencyAdapter::Nuget
                })
            },
        );
        if nuget_audit {
            validate_audit_input(path, &bytes)?;
        }
        for pin in sensor.pins.iter().filter(|pin| &pin.path == path) {
            let value: serde_json::Value = serde_json::from_slice(&bytes)?;
            if value
                .pointer(&pin.pointer)
                .and_then(serde_json::Value::as_str)
                != Some(pin.expected.as_str())
            {
                return Err(invalid(format!(
                    "pinned value differs: {path}{}",
                    pin.pointer
                )));
            }
        }
        inputs.push(ArtifactDigest {
            path: path.clone(),
            digest: bytes_digest(&bytes),
        });
        selected_files.insert(path.clone(), bytes);
    }
    if let Some(selection) = &request.profile.selection {
        crate::selection::validate_component_files(selection, &sensor.component, &selected_files)?;
    }
    inputs.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(ConfigurationBinding {
        schema_version: 1,
        project_profile_digest: request.expected_profile_digest.clone(),
        sensor: sensor.clone(),
        component_root: cwd,
        inputs,
    })
}

fn validate_audit_input(path: &str, bytes: &[u8]) -> Result<(), Error> {
    if !(path.ends_with(".csproj")
        || path.ends_with(".props")
        || path.ends_with(".targets")
        || path.ends_with("NuGet.Config"))
    {
        return Ok(());
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| invalid("audit configuration must be UTF-8"))?;
    let document =
        roxmltree::Document::parse(text).map_err(|e| invalid(format!("audit XML: {e}")))?;
    if document.descendants().any(|n| {
        n.has_tag_name("NuGetAuditSuppress")
            || n.has_tag_name("NuGetLockFilePath")
            || n.attribute("TreatAsLocalProperty")
                .is_some_and(|properties| {
                    properties.split(';').any(|property| {
                        [
                            "NuGetAudit",
                            "NuGetAuditMode",
                            "NuGetAuditLevel",
                            "WarningsAsErrors",
                            "NoWarn",
                            "WarningsNotAsErrors",
                            "RestoreLockedMode",
                            "NuGetLockFilePath",
                        ]
                        .iter()
                        .any(|protected| property.trim().eq_ignore_ascii_case(protected))
                    })
                })
    }) {
        return Err(invalid(
            "audit contract rejects suppressed advisories or locally overridden audit properties",
        ));
    }
    if path.ends_with("NuGet.Config")
        && !document.descendants().any(|n| {
            n.has_tag_name("auditSources")
                && n.children().any(|c| {
                    c.has_tag_name("add")
                        && c.attribute("value")
                            .is_some_and(|value| value.starts_with("https://"))
                })
        })
    {
        return Err(invalid(
            "NuGet.Config must explicitly configure an HTTPS audit source",
        ));
    }
    Ok(())
}

fn validate_request(request: &RunRequest) -> Result<(), Error> {
    if request.schema_version != 1 {
        return Err(invalid("unsupported run request version"));
    }
    // Reuse schema validation for programmatically constructed requests too.
    crate::parse_profile(&serde_json::to_string(&request.profile)?)?;
    if !request.root.is_absolute() || request.root.canonicalize()? != request.root {
        return Err(invalid(
            "root must be an existing canonical absolute directory",
        ));
    }
    if canonical_sha256(&request.profile)? != request.expected_profile_digest {
        return Err(invalid("project profile digest mismatch"));
    }
    source_files(request)?;
    Ok(())
}

/// Read-only preflight: review the candidate policy-bound profile before execution.
pub fn prepare(request: &RunRequest) -> Result<PreparedRun, Error> {
    validate_request(request)?;
    let selected = request.profile.selected(&request.impact)?;
    let selected_ids: BTreeSet<_> = selected.iter().map(|s| s.definition.id.clone()).collect();
    let mut bound = request.profile.sensor_profile();
    let mut configurations = Vec::new();
    // Preflight all components and files before the first command can run.
    for sensor in &request.profile.sensors {
        let config = configuration(request, sensor)?;
        let definition = bound
            .sensors
            .iter_mut()
            .find(|s| s.id == sensor.definition.id)
            .unwrap();
        definition.result_binding = Some(SensorResultBinding {
            configuration_digest: canonical_sha256(&config)?,
            parser_version: parser_version(sensor).to_owned(),
            resolved_command: sensor.argv.clone(),
        });
        if matches!(request.impact, Impact::Unknown) {
            definition.required = true;
        }
        if selected_ids.contains(&sensor.definition.id) {
            collection_request(request, &config)?;
        }
        configurations.push(config);
    }
    Ok(PreparedRun {
        schema_version: 1,
        bound_sensor_profile: bound,
        selected_sensor_ids: selected_ids,
        configurations,
    })
}

/// No policy/CI trust is accepted as input. All returned observations are local advisory.
pub fn run(request: RunRequest) -> Result<RunResult, Error> {
    let prepared = prepare(&request)?;
    let configurations = prepared.configurations;
    let selected_ids = prepared.selected_sensor_ids;
    let bound = prepared.bound_sensor_profile;
    let mut sensors = Vec::new();
    for config in configurations
        .iter()
        .filter(|c| selected_ids.contains(&c.sensor.definition.id))
    {
        // A preceding command cannot silently change a later command's checked inputs.
        if canonical_sha256(&configuration(&request, &config.sensor)?)? != canonical_sha256(config)?
        {
            return Err(invalid("configuration changed after preflight"));
        }
        source_files(&request)?;
        sensors.push(run_sensor(&request, config.clone())?);
    }
    source_files(&request)?;
    for config in &configurations {
        if canonical_sha256(&configuration(&request, &config.sensor)?)? != canonical_sha256(config)?
        {
            return Err(invalid("configuration changed during collection"));
        }
    }
    if let Some(consumer) = request
        .profile
        .selection
        .as_ref()
        .and_then(|s| s.consumer.as_ref())
    {
        consumer.validate_suite_reports(
            &sensors
                .iter()
                .filter_map(|s| {
                    s.result
                        .parsed
                        .as_ref()
                        .map(|p| (s.evidence.sensor_id.to_string(), p.clone()))
                })
                .collect(),
        )?;
    }
    let mut adequacy = TestAdequacyEvidence {
        acceptance_criteria: request.mappings.acceptance_criteria,
        changed_behaviors: request.mappings.changed_behaviors,
        negative_case_ids: request.mappings.negative_case_ids,
        executed_test_ids: vec![],
        discovered: 0,
        executed: 0,
        passed: 0,
    };
    for parsed in sensors.iter().filter_map(|s| s.result.parsed.as_ref()) {
        adequacy.discovered = adequacy
            .discovered
            .checked_add(parsed.discovered)
            .ok_or_else(|| invalid("count overflow"))?;
        adequacy.executed = adequacy
            .executed
            .checked_add(parsed.executed)
            .ok_or_else(|| invalid("count overflow"))?;
        adequacy.passed = adequacy
            .passed
            .checked_add(parsed.passed)
            .ok_or_else(|| invalid("count overflow"))?;
        adequacy.executed_test_ids.extend(
            parsed
                .tests
                .iter()
                .filter(|t| t.outcome != TestOutcome::Skipped)
                .map(|t| t.id.clone()),
        );
    }
    adequacy.executed_test_ids.sort();
    if adequacy.executed_test_ids.windows(2).any(|p| p[0] == p[1]) {
        return Err(invalid("duplicate composed test identity"));
    }
    let executed: BTreeSet<_> = adequacy.executed_test_ids.iter().collect();
    if adequacy
        .acceptance_criteria
        .iter()
        .chain(&adequacy.changed_behaviors)
        .flat_map(|m| &m.test_ids)
        .any(|id| !executed.contains(id))
    {
        return Err(invalid("adequacy mapping references an unexecuted test"));
    }
    for mappings in [&adequacy.acceptance_criteria, &adequacy.changed_behaviors] {
        let mut seen = BTreeSet::new();
        for mapping in mappings {
            let namespace = mapping
                .requirement_id
                .split_once(':')
                .map(|(c, _)| c)
                .ok_or_else(|| invalid("adequacy mapping lacks a component namespace"))?;
            if !seen.insert(&mapping.requirement_id)
                || mapping.test_ids.is_empty()
                || mapping
                    .test_ids
                    .iter()
                    .any(|id| !id.starts_with(&format!("{namespace}:")))
            {
                return Err(invalid(
                    "adequacy mapping crosses components or is empty/duplicate",
                ));
            }
        }
    }
    Ok(RunResult {
        schema_version: 1,
        project_profile_digest: request.expected_profile_digest,
        source_set_digest: request.expected_source_digest,
        bound_sensor_profile: bound,
        sensors,
        adequacy,
    })
}

fn parser_version(sensor: &ProjectSensor) -> &'static str {
    sensor
        .report
        .as_ref()
        .map_or("process-v1", |r| r.format.parser_version())
}

fn collection_request(
    request: &RunRequest,
    configuration: &ConfigurationBinding,
) -> Result<CollectionRequest, Error> {
    Ok(CollectionRequest::try_from_json(&serde_json::to_string(
        &collection_request_value(request, configuration)?,
    )?)?)
}

fn collection_request_value(
    request: &RunRequest,
    configuration: &ConfigurationBinding,
) -> Result<serde_json::Value, Error> {
    let sensor = &configuration.sensor;
    if let Some(report) = &sensor.report {
        paths::require_absent(&configuration.component_root, &report.path)?;
    }
    let configuration_digest = canonical_sha256(configuration)?;
    let input_digest = canonical_sha256(&serde_json::json!({
        "schema_version":1, "configuration_digest": configuration_digest,
        "repository":request.repository, "head_sha":request.head_sha,
        "source_set_digest":request.expected_source_digest
    }))?;
    let value = serde_json::json!({
        "argv":sensor.argv, "cwd":configuration.component_root,
        "environment_allowlist":sensor.environment_allowlist, "timeout_ms":sensor.timeout_ms,
        "stdout_limit_bytes":evidence_collector::MAX_OUTPUT_BYTES,
        "stderr_limit_bytes":evidence_collector::MAX_OUTPUT_BYTES,
        "version_args":sensor.version_args,
        "repository":request.repository, "head_sha":request.head_sha,
        "provider":request.provider, "model":request.model,
        "execution_id":format!("{}:{}", request.execution_id, sensor.definition.id),
        "input_digest":input_digest,
        "artifact_paths":sensor.report.iter().map(|r| r.path.clone()).collect::<Vec<_>>()
    });
    Ok(value)
}

fn run_sensor(
    request: &RunRequest,
    configuration: ConfigurationBinding,
) -> Result<NormalizedSensor, Error> {
    let preflight = if let Some(selection) = &request.profile.selection {
        let tool = selection.tool_for(&configuration.sensor)?;
        let mut value = collection_request_value(request, &configuration)?;
        value["argv"] = serde_json::json!(std::iter::once(tool.executable.clone())
            .chain(tool.version_args.clone())
            .collect::<Vec<_>>());
        value["artifact_paths"] = serde_json::json!([]);
        value["execution_id"] = serde_json::json!(format!(
            "{}:{}:version-preflight",
            request.execution_id, configuration.sensor.definition.id
        ));
        let probe = collect(CollectionRequest::try_from_json(&serde_json::to_string(
            &value,
        )?)?)?;
        if probe.evidence.status != ExecutionStatus::Passed {
            return Err(invalid(format!(
                "version preflight failed for {}",
                configuration.sensor.definition.id
            )));
        }
        let actual = tool.observe(&probe.record.execution.stdout)?;
        if tool.observe(&probe.record.version_probe.stdout)? != actual {
            return Err(invalid("version changed during preflight"));
        }
        source_files(request)?;
        if canonical_sha256(&self::configuration(request, &configuration.sensor)?)?
            != canonical_sha256(&configuration)?
        {
            return Err(invalid("configuration changed during version preflight"));
        }
        Some((probe, actual))
    } else {
        None
    };
    let collection = collect(collection_request(request, &configuration)?)?;
    let mut normalized = normalize_collection(request, configuration, collection)?;
    if let Some((probe, actual)) = preflight {
        if normalized.result.observed_version.as_ref() != Some(&actual)
            || normalized.collection.record.command.resolved_executable
                != probe.record.command.resolved_executable
        {
            return Err(invalid(
                "selected executable/version changed after preflight",
            ));
        }
        normalized.result.version_preflight_digest = Some(canonical_sha256(&probe)?);
        normalized.version_preflight = Some(probe);
        normalized.evidence.result_digest = canonical_sha256(&normalized.result)?;
    }
    Ok(normalized)
}

/// Normalization is exposed separately for deterministic transport and failure regressions.
/// A submitted collection remains untrusted even if its JSON claims CI provenance.
pub fn normalize_collection(
    request: &RunRequest,
    configuration: ConfigurationBinding,
    mut collection: CollectedExecution,
) -> Result<NormalizedSensor, Error> {
    validate_request(request)?;
    if canonical_sha256(&request.profile)? != request.expected_profile_digest
        || configuration.project_profile_digest != request.expected_profile_digest
        || !request
            .profile
            .sensors
            .iter()
            .any(|s| canonical_sha256(s).ok() == canonical_sha256(&configuration.sensor).ok())
        || canonical_sha256(&self::configuration(request, &configuration.sensor)?)?
            != canonical_sha256(&configuration)?
    {
        return Err(invalid(
            "normalization configuration is not the requested profile",
        ));
    }
    let sensor = &configuration.sensor;
    let configuration_digest = canonical_sha256(&configuration)?;
    let record = &collection.record;
    let expected_input = canonical_sha256(&serde_json::json!({
        "schema_version":1, "configuration_digest":configuration_digest,
        "repository":request.repository, "head_sha":request.head_sha,
        "source_set_digest":request.expected_source_digest
    }))?;
    if record.command.argv != sensor.argv
        || record.command.cwd != configuration.component_root
        || record.command.repository != request.repository
        || record.command.head_sha != request.head_sha
        || record.command.input_digest != expected_input
        || collection.evidence.input_digest != expected_input
        || record.command.version_args != sensor.version_args
        || record.command.timeout_ms != sensor.timeout_ms
        || record.command.environment_allowlist != {
            let mut keys = sensor.environment_allowlist.clone();
            keys.sort();
            keys
        }
        || record.command.stdout_limit_bytes != evidence_collector::MAX_OUTPUT_BYTES
        || record.command.stderr_limit_bytes != evidence_collector::MAX_OUTPUT_BYTES
        || record.command.artifact_paths
            != sensor
                .report
                .iter()
                .map(|r| PathBuf::from(&r.path))
                .collect::<Vec<_>>()
        || record.command.provider != request.provider
        || record.command.model != request.model
        || record.command.execution_id.as_str()
            != format!("{}:{}", request.execution_id, sensor.definition.id)
        || collection.evidence.execution_id != record.command.execution_id
        || collection.evidence.provider != record.command.provider
        || collection.evidence.model != record.command.model
        || collection.evidence.started_at > record.execution.started_at
        || collection.evidence.completed_at < record.execution.completed_at
        || record.execution.started_at > record.execution.completed_at
        || canonical_sha256(&record.command)? != collection.evidence.command_digest
        || canonical_sha256(record)? != collection.evidence.output_digest
    {
        return Err(invalid(
            "collection does not match its checked configuration or digest",
        ));
    }
    let mut result = NormalizedResult {
        schema_version: 1,
        collection_digest: collection.evidence.output_digest.clone(),
        original_report: None,
        parsed: None,
        errors: vec![],
        observed_version: None,
        version_preflight_digest: None,
    };
    let mut status = match collection.evidence.status {
        ExecutionStatus::Passed => SensorStatus::Passed,
        ExecutionStatus::Failed => SensorStatus::Failed,
        ExecutionStatus::TimedOut => SensorStatus::TimedOut,
        ExecutionStatus::Skipped => SensorStatus::Skipped,
        ExecutionStatus::Truncated | ExecutionStatus::Unparsable => SensorStatus::Unparsable,
    };
    if (collection.record.execution.status != ExecutionStatus::Passed
        || collection.record.execution.exit_code != Some(0)
        || collection.record.execution.error.is_some()
        || !collection.record.artifact_errors.is_empty())
        && status == SensorStatus::Passed
    {
        status = SensorStatus::Failed;
        result.errors.push("execution record did not pass".into());
    }
    let version_result = if let Some(selection) = &request.profile.selection {
        selection
            .tool_for(sensor)?
            .observe(&collection.record.version_probe.stdout)
            .map(Some)
    } else if std::str::from_utf8(&collection.record.version_probe.stdout)
        .ok()
        .map(str::trim)
        == Some(sensor.version_stdout.trim())
    {
        Ok(None)
    } else {
        Err(invalid(
            "tool version probe did not match the pinned output",
        ))
    };
    if collection.record.version_probe.status != ExecutionStatus::Passed
        || collection.record.version_probe.exit_code != Some(0)
        || collection.record.version_probe.error.is_some()
        || version_result.is_err()
    {
        result.errors.push(
            version_result
                .err()
                .map_or_else(|| "tool version probe failed".into(), |e| e.to_string()),
        );
        if status == SensorStatus::Passed {
            status = SensorStatus::Unparsable;
        }
    } else {
        result.observed_version = version_result?;
    }
    if let Some(report) = &sensor.report {
        let parsed = (|| -> Result<ParsedReport, Error> {
            let bytes = paths::read(&configuration.component_root, &report.path)?;
            let artifact = ArtifactDigest {
                path: report.path.clone(),
                digest: bytes_digest(&bytes),
            };
            if collection
                .record
                .artifacts
                .iter()
                .filter(|a| a.path == report.path)
                .count()
                != 1
                || !collection.record.artifacts.contains(&artifact)
            {
                return Err(invalid(
                    "original report digest differs from collected bytes",
                ));
            }
            result.original_report = Some(artifact);
            let parsed = parse_report(report.format, sensor.definition.id.as_str(), &bytes)
                .map_err(|e| invalid(e.to_string()))?;
            let start = parsed.started_at.as_datetime();
            if *start
                < *collection.record.execution.started_at.as_datetime()
                    - chrono::Duration::milliseconds(1)
                || start > collection.record.execution.completed_at.as_datetime()
            {
                return Err(invalid("report start time is outside this execution"));
            }
            Ok(parsed)
        })();
        match parsed {
            Ok(parsed) => {
                if !parsed.successful && status == SensorStatus::Passed {
                    status = SensorStatus::Failed;
                }
                result.parsed = Some(parsed);
            }
            Err(e) => {
                result.errors.push(e.to_string());
                if status == SensorStatus::Passed {
                    status = SensorStatus::Unparsable;
                }
            }
        }
    }
    collection.evidence.trust = TrustClass::LocalUntrusted;
    let evidence = SensorEvidence {
        sensor_id: sensor.definition.id.clone(),
        status,
        head_sha: request.head_sha.clone(),
        command_digest: collection.evidence.command_digest.clone(),
        result_digest: canonical_sha256(&result)?,
        observed_at: collection.evidence.completed_at.clone(),
        trust: TrustClass::LocalUntrusted,
        resolved_command: sensor.argv.clone(),
        source_digest: Some(request.expected_source_digest.clone()),
        configuration_digest: Some(configuration_digest),
        parser_version: Some(parser_version(sensor).into()),
        exit_code: collection.record.execution.exit_code,
        discovered: result.parsed.as_ref().map(|r| r.discovered),
        executed: result.parsed.as_ref().map(|r| r.executed),
        passed: result.parsed.as_ref().map(|r| r.passed),
        not_applicable_condition_id: None,
        not_applicable_reason: None,
    };
    Ok(NormalizedSensor {
        evidence,
        collection,
        configuration,
        result,
        version_preflight: None,
    })
}
