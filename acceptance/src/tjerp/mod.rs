//! TJERP local advisory glue. No installation, trusted import or policy-engine copy.
mod catalog;
mod reports;
mod source;
use crate::Result;
pub use catalog::*;
use policy_contracts::*;
use policy_core::{
    canonical_sha256, evaluate_sensor_evidence, validate_bundle, validate_provenance,
    SensorEvaluationStatus,
};
use project_verification::{bytes_digest, ProjectProfile, RunRequest, RunResult};
use project_verification::{profile::Impact, reports::ReportFormat, run::AdequacyMappings};
pub use reports::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use source::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

fn ensure(condition: bool, message: &str) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned().into())
    }
}
fn relative(path: &str) -> Result<()> {
    ensure(
        !path.is_empty()
            && !path.contains(['\\', ':', '\0'])
            && path.split('/').all(|p| !matches!(p, "" | "." | "..")),
        "TJ-PATH: unsafe relative path",
    )
}
/// Refuse symlinks at every level, including parent directories.
pub fn checked_read(root: &Path, path: &str) -> Result<Vec<u8>> {
    relative(path)?;
    let mut current = root.to_path_buf();
    ensure(
        root.canonicalize()? == root && root.is_dir(),
        "TJ-PATH: canonical root required",
    )?;
    for part in path.split('/') {
        current.push(part);
        ensure(
            !fs::symlink_metadata(&current)?.file_type().is_symlink(),
            "TJ-PATH: symlink",
        )?;
    }
    ensure(current.is_file(), "TJ-PATH: regular file required")?;
    Ok(fs::read(current)?)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunIdentity {
    pub repository: RepositoryId,
    pub head_sha: CommitSha,
    pub provider: ProviderId,
    pub model: ModelId,
    pub execution_id: ExecutionId,
}
/// Build typed digests; no hash strings or execution_snapshot IDs are accepted as mappings.
pub fn requests(
    root: &Path,
    profiles: [ProjectProfile; 2],
    inventory: &[ArtifactDigest],
    identity: &RunIdentity,
    catalog: &Catalog,
    suites: &Suites,
    discovery: &[RawTest],
) -> Result<[RunRequest; 2]> {
    for profile in &profiles {
        profile.validate()?;
        ensure(profile.components.len() == 1, "TJ-PROFILE-COMPONENT")?;
    }
    for selector in catalog
        .requirements
        .iter()
        .flat_map(|r| &r.tests)
        .chain(&catalog.required_regression_tests)
    {
        checked_read(root, &selector.path)?;
    }
    let mut files = inventory.to_vec();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    ensure(
        !files.is_empty() && !files.windows(2).any(|p| p[0].path == p[1].path),
        "TJ-SOURCE-INVENTORY",
    )?;
    for f in &files {
        ensure(
            bytes_digest(&checked_read(root, &f.path)?) == f.digest,
            "TJ-SOURCE-CHANGED",
        )?;
    }
    let digest = canonical_sha256(&files)?;
    let make = |profile: ProjectProfile| -> Result<RunRequest> {
        let component = &profile.components[0].id;
        Ok(RunRequest {
            schema_version: 1,
            expected_profile_digest: canonical_sha256(&profile)?,
            root: root.into(),
            repository: identity.repository.clone(),
            head_sha: identity.head_sha.clone(),
            source_files: files.clone(),
            expected_source_digest: digest.clone(),
            provider: identity.provider.clone(),
            model: identity.model.clone(),
            execution_id: ExecutionId::parse(format!("{}:{component}", identity.execution_id))?,
            impact: Impact::Unknown,
            mappings: mappings(component, catalog, suites, discovery)?,
            profile,
        })
    };
    let [a, b] = profiles;
    Ok([make(a)?, make(b)?])
}

#[derive(Debug, Serialize)]
pub struct ScopeReport {
    pub label: String,
    pub trust: TrustClass,
    pub review_gate: String,
    pub reasons: Vec<Reason>,
    pub requirements: BTreeMap<String, String>,
    pub head_sha: CommitSha,
    pub source_digest: Sha256Digest,
    pub profile_digests: Vec<Sha256Digest>,
    pub exclusions: Vec<String>,
}

/// Compose a local manifest so the existing core checks all child/head/freshness bindings.
pub fn bundle(
    request: &RunRequest,
    result: &RunResult,
    time: &UtcTimestamp,
) -> Result<EvidenceBundle> {
    let mut value = serde_json::json!({"schema_version":1,"manifest":{
        "schema_version":1,"repository":request.repository,"work_item_id":"TJERP-local-adoption","base_sha":request.head_sha,"head_sha":request.head_sha,
        "policy_digest":canonical_sha256(&result.bound_sensor_profile)?,"source_set_digest":request.expected_source_digest,
        "workflow_run_id":request.execution_id.as_str(),"created_at":time,"child_digests":[]},
        "modifications":[],"executions":result.sensors.iter().map(|s| &s.collection.evidence).collect::<Vec<_>>(),"reviews":[],"findings":[],"waivers":[],
        "sensors":result.sensors.iter().map(|s| &s.evidence).collect::<Vec<_>>(),"test_adequacy":result.adequacy,"knowledge":[]});
    seal_bundle_value(&mut value)?;
    Ok(serde_json::from_value(value)?)
}
fn seal_bundle_value(value: &mut Value) -> Result<()> {
    let mut digests = vec![];
    for path in [
        "modifications",
        "executions",
        "reviews",
        "findings",
        "waivers",
        "sensors",
        "test_adequacy",
        "knowledge",
    ] {
        digests.push(ArtifactDigest {
            path: path.into(),
            digest: canonical_sha256(&value[path])?,
        });
    }
    value["manifest"]["child_digests"] = serde_json::to_value(digests)?;
    Ok(())
}

/// Called only on results collected by this process. This is not a trusted-result importer.
pub fn evaluate_pair(
    requests: &[RunRequest; 2],
    results: &[RunResult; 2],
    time: &UtcTimestamp,
    max_age_seconds: u64,
    catalog: &Catalog,
    suites: &Suites,
) -> Result<ScopeReport> {
    let a = &requests[0];
    let b = &requests[1];
    ensure(
        a.root == b.root
            && a.repository == b.repository
            && a.head_sha == b.head_sha
            && a.source_files == b.source_files
            && a.expected_source_digest == b.expected_source_digest,
        "TJ-SPLIT-BINDING: HEAD/source inventory differ",
    )?;
    ensure(
        a.profile.components.len() == 1
            && b.profile.components.len() == 1
            && a.profile.components[0].id != b.profile.components[0].id,
        "TJ-SPLIT-COMPONENT",
    )?;
    let mut reasons = vec![];
    let mut requirements = BTreeMap::new();
    for (request, result) in requests.iter().zip(results) {
        ensure(
            result.project_profile_digest == request.expected_profile_digest
                && result.source_set_digest == request.expected_source_digest
                && result.adequacy.head_sha.as_ref() == Some(&request.head_sha),
            "TJ-RESULT-BINDING",
        )?;
        // Re-derive the prepared binding without requiring report output paths to be absent.
        let mut expected = request.profile.sensor_profile();
        for def in &mut expected.sensors {
            def.required = true;
            let sensor = result
                .sensors
                .iter()
                .find(|s| s.evidence.sensor_id == def.id)
                .ok_or("TJ-RESULT-SENSOR-MISSING")?;
            let declared = request
                .profile
                .sensors
                .iter()
                .find(|s| s.definition.id == def.id)
                .ok_or("TJ-PROFILE-SENSOR")?;
            ensure(
                canonical_sha256(declared)? == canonical_sha256(&sensor.configuration.sensor)?,
                "TJ-PROFILE-SENSOR: configuration differs",
            )?;
            ensure(
                sensor.evidence.trust == TrustClass::LocalUntrusted
                    && sensor.collection.evidence.trust == TrustClass::LocalUntrusted
                    && sensor
                        .version_preflight
                        .as_ref()
                        .is_none_or(|v| v.evidence.trust == TrustClass::LocalUntrusted),
                "TJ-TRUST: local observations cannot be promoted",
            )?;
            ensure(
                sensor.evidence.head_sha == request.head_sha
                    && sensor.evidence.source_digest.as_ref()
                        == Some(&request.expected_source_digest)
                    && sensor.configuration.project_profile_digest
                        == request.expected_profile_digest
                    && sensor.evidence.result_digest == canonical_sha256(&sensor.result)?
                    && sensor.result.collection_digest
                        == canonical_sha256(&sensor.collection.record)?
                    && sensor.collection.evidence.output_digest == sensor.result.collection_digest,
                "TJ-RESULT-BINDING",
            )?;
            def.result_binding = Some(SensorResultBinding {
                configuration_digest: project_verification::run::configuration_digest(
                    &sensor.configuration,
                )?,
                parser_version: sensor
                    .configuration
                    .sensor
                    .report
                    .as_ref()
                    .map_or("process-v1", |r| r.format.parser_version())
                    .into(),
                resolved_command: sensor.configuration.sensor.argv.clone(),
            });
        }
        ensure(
            canonical_sha256(&expected)? == canonical_sha256(&result.bound_sensor_profile)?,
            "TJ-PROFILE-BINDING",
        )?;
        let evidence: Vec<_> = result.sensors.iter().map(|s| s.evidence.clone()).collect();
        let required: Vec<_> = expected.sensors.iter().map(|s| s.id.clone()).collect();
        let evaluation =
            evaluate_sensor_evidence(&expected, &required, &evidence, Some(&result.adequacy));
        if evaluation.status != SensorEvaluationStatus::Passed {
            ensure(
                !evaluation.reasons.is_empty(),
                "TJ-CORE-NONPASS: core evaluation did not pass",
            )?;
            reasons.extend(evaluation.reasons);
        }
        let bundle = bundle(request, result, time)?;
        reasons.extend(validate_bundle(&bundle).reasons().iter().cloned());
        reasons.extend(
            validate_provenance(EvaluationMode::Advisory, &bundle, time, max_age_seconds)
                .reasons()
                .iter()
                .cloned(),
        );
        let mut raw = vec![];
        for (id, component, suite) in suites
            .entries()
            .filter(|(id, _, _)| id.starts_with(&format!("{}:", request.profile.components[0].id)))
        {
            let bytes = checked_read(
                &request.root,
                &format!("{}/{}", component.cwd, suite.report),
            )?;
            let sensor = result
                .sensors
                .iter()
                .find(|s| s.evidence.sensor_id.as_str() == id)
                .ok_or("TJ-SUITE-MISSING")?;
            ensure(
                sensor
                    .result
                    .original_report
                    .as_ref()
                    .is_some_and(|a| a.digest == bytes_digest(&bytes)),
                "TJ-RAW-DIGEST",
            )?;
            let parsed = project_verification::reports::parse_report(suite.format, &id, &bytes)?;
            ensure(
                sensor
                    .result
                    .parsed
                    .as_ref()
                    .is_some_and(|p| canonical_sha256(p).ok() == canonical_sha256(&parsed).ok()),
                "TJ-RAW-NORMALIZED: report differs",
            )?;
            raw.extend(link_report(&request.root, &id, suite, &bytes)?);
        }
        let expected_mappings = mappings(&request.profile.components[0].id, catalog, suites, &raw);
        match expected_mappings {
            Ok(m) => {
                ensure(
                    canonical_sha256(&m)? == canonical_sha256(&request.mappings)?
                        && result.adequacy.acceptance_criteria == m.acceptance_criteria
                        && result.adequacy.changed_behaviors == m.changed_behaviors
                        && result.adequacy.negative_case_ids == m.negative_case_ids,
                    "TJ-FINAL-MAPPING: discovery and final execution IDs differ",
                )?;
            }
            Err(e) => reasons.push(Reason {
                code: "TJ-REQUIREMENT-UNPROVEN".into(),
                message: e.to_string(),
            }),
        }
        for r in catalog
            .requirements
            .iter()
            .filter(|r| r.component == request.profile.components[0].id)
        {
            let proven = r.tests.iter().all(|s| resolve(s, &raw, suites).is_ok());
            requirements.insert(
                r.id.clone(),
                if proven {
                    "satisfied"
                } else {
                    "failed/unproven"
                }
                .into(),
            );
        }
    }
    reasons.sort_by(|a, b| (&a.code, &a.message).cmp(&(&b.code, &b.message)));
    reasons.dedup();
    Ok(ScopeReport {
        label: if reasons.is_empty() {
            "local scope satisfied"
        } else {
            "local scope non-pass"
        }
        .into(),
        trust: TrustClass::LocalUntrusted,
        review_gate: "unfulfilled; no independent review supplied; no pre-merge decision".into(),
        reasons,
        requirements,
        head_sha: a.head_sha.clone(),
        source_digest: a.expected_source_digest.clone(),
        profile_digests: requests
            .iter()
            .map(|r| r.expected_profile_digest.clone())
            .collect(),
        exclusions: vec![
            "Flutter/Dart and devices".into(),
            "live authentication handlers".into(),
            "DB2".into(),
            "production CI, required checks, merge and deployment".into(),
        ],
    })
}
