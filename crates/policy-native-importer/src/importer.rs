use crate::{fs, reports};
use policy_contracts::ExecutionId;
use policy_native_contracts::*;
use policy_native_verifier::ImportedObservationsV1;
use policy_report_adapters::ParsedReport;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::Path,
};

/// Independently supplied replay expectations; v1 ExpectedScope has no run/scope IDs.
#[derive(Debug, Clone, Serialize)]
pub struct ExpectedIdentity {
    pub run_id: ExecutionId,
    pub scope_id: RecordId,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueCode {
    Contract,
    Subject,
    Time,
    Artifact,
    Report,
    Observation,
}
#[derive(Debug, Clone, Serialize)]
pub struct ImportIssue {
    pub code: IssueCode,
    pub location: String,
    pub detail: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct PreservedArtifact {
    pub original_path: String,
    pub stored_file: String,
    pub size: u64,
    pub digest: RawByteDigest,
}
#[derive(Debug, Clone, Serialize)]
pub struct RecomputedReport {
    pub observation_id: String,
    pub report: ParsedReport,
    pub observation: TestObservationV1,
}
/// An import audit record, NOT LocalScopeResultV1. Even no issues does not mean
/// requirements are satisfied. Recomputed reports may contain failing tests.
#[derive(Debug, Clone, Serialize)]
pub struct ImportRecord {
    #[serde(skip)]
    verified: Option<ImportedObservationsV1>,
    pub adapter_version: &'static str,
    pub trust: LocalTrust,
    pub review: ReviewGate,
    pub issues: Vec<ImportIssue>,
    pub artifacts: Vec<PreservedArtifact>,
    pub reports: Vec<RecomputedReport>,
}
impl ImportRecord {
    /// Available only after successful raw verification. It remains an
    /// unauthenticated local observation, not a scope verdict.
    pub fn verified_observations(&self) -> Option<&ImportedObservationsV1> {
        self.verified.as_ref()
    }

    fn issue(&mut self, code: IssueCode, location: impl Into<String>, detail: impl ToString) {
        self.issues.push(ImportIssue {
            code,
            location: location.into(),
            detail: detail.to_string(),
        });
    }
}

pub fn raw_digest(bytes: &[u8]) -> RawByteDigest {
    RawByteDigest {
        kind: RawHashKind::RawByteSha256,
        sha256: policy_contracts::Sha256Digest::parse(hex::encode(Sha256::digest(bytes)))
            .expect("SHA-256 hex"),
    }
}

/// Writes inputs, bounded copies of regular files (including undeclared files),
/// and import.json to a fresh directory. The final `complete` marker is published
/// atomically for BOTH accepted and rejected imports. It only marks persistence.
/// Errors before marker publication leave any partial directory intact without
/// a complete marker. Input files are never modified, deleted or followed through
/// symlinks. Unsafe/oversize/unreadable files remain at source with diagnostics.
///
/// Parents must exist and all input/output path components must be symlink-free.
/// The producer must quiesce the run directory while importing. This detects
/// ordinary concurrent changes, but does not authenticate a hostile producer.
/// The output must be outside the run directory. Unix only; no weaker fallback.
/// No command, Git or network access occurs.
pub fn import_run(
    run_directory: &Path,
    output_directory: &Path,
    expected: &ExpectedScopeV1,
    identity: &ExpectedIdentity,
    run: &NativeRunV1,
) -> io::Result<ImportRecord> {
    let input = fs::absolute(run_directory)?;
    let output = fs::absolute(output_directory)?;
    if output.starts_with(&input) || input.starts_with(&output) {
        return Err(fs::invalid("input and output directories must be disjoint"));
    }
    let root = fs::directory(&input);
    let out = fs::fresh(&output, root.as_ref().ok())?;
    for (name, value) in [
        ("expected.json", serde_json::to_vec_pretty(expected)?),
        ("identity.json", serde_json::to_vec_pretty(identity)?),
        ("run.json", serde_json::to_vec_pretty(run)?),
    ] {
        fs::write(&out, name, &value)?;
    }
    let mut record = ImportRecord {
        verified: None,
        adapter_version: env!("CARGO_PKG_VERSION"),
        trust: LocalTrust::LocalUntrusted,
        review: ReviewGate::Unmet,
        issues: vec![],
        artifacts: vec![],
        reports: vec![],
    };
    let mut bytes = BTreeMap::new();
    match root.and_then(|root| fs::inventory(&root).map(|entries| (root, entries))) {
        Err(error) => record.issue(IssueCode::Artifact, "run_directory", error),
        Ok((root, entries)) => {
            let mut total = 0;
            let paths: BTreeSet<_> = run.artifacts.iter().map(|a| a.path.as_str()).collect();
            for (index, entry) in entries.iter().enumerate() {
                let is_file = entry.kind == rustix::fs::FileType::RegularFile;
                let allowed = if entry.kind == rustix::fs::FileType::Directory {
                    paths
                        .iter()
                        .any(|p| p.starts_with(&format!("{}/", entry.path)))
                } else {
                    is_file && paths.contains(entry.path.as_str())
                };
                if !allowed || !fs::safe(&entry.path) {
                    record.issue(
                        IssueCode::Artifact,
                        &entry.path,
                        "extra or unsafe path (symlinks and special files forbidden)",
                    );
                }
                if !is_file {
                    continue;
                }
                if entry.stat.2 > fs::FILE_LIMIT || total + entry.stat.2 > fs::TOTAL_LIMIT {
                    record.issue(
                        IssueCode::Artifact,
                        &entry.path,
                        "artifact size or total 256 MiB limit exceeded; original retained",
                    );
                    continue;
                }
                total += entry.stat.2;
                match fs::read(&root, entry) {
                    Err(error) => record.issue(IssueCode::Artifact, &entry.path, error),
                    Ok(data) => {
                        let stored_file = format!("artifact-{index:04}");
                        fs::write(&out, &stored_file, &data)?;
                        record.artifacts.push(PreservedArtifact {
                            original_path: entry.path.clone(),
                            stored_file,
                            size: data.len() as u64,
                            digest: raw_digest(&data),
                        });
                        bytes.insert(entry.path.clone(), data);
                    }
                }
            }
            match fs::inventory(&root) {
                Ok(after) if after == entries => (),
                _ => record.issue(
                    IssueCode::Artifact,
                    "run_directory",
                    "directory changed during import",
                ),
            }
        }
    }
    for artifact in &run.artifacts {
        match bytes.get(&artifact.path) {
            Some(data) if raw_digest(data) == artifact.digest => (),
            Some(_) => record.issue(IssueCode::Artifact, &artifact.path, "raw SHA-256 mismatch"),
            None => record.issue(
                IssueCode::Artifact,
                &artifact.path,
                "missing or unreadable artifact",
            ),
        }
    }
    verify(expected, identity, run, &bytes, &mut record);
    if record.issues.is_empty() {
        let mut verified_run = run.clone();
        verified_run.test_observations = record
            .reports
            .iter()
            .map(|r| r.observation.clone())
            .collect();
        let verified = ImportedObservationsV1 {
            schema_version: VersionV1::default(),
            expected_scope_digest: expected.digest().map_err(|e| fs::invalid(e.to_string()))?,
            expected_run_id: identity.run_id.clone(),
            expected_scope_id: identity.scope_id.clone(),
            adapter_version: RecordId::parse(concat!(
                "policy-native-importer/",
                env!("CARGO_PKG_VERSION")
            ))
            .expect("static version"),
            trust: LocalTrust::LocalUntrusted,
            review: ReviewGate::Unmet,
            unsuccessful_reports: record
                .reports
                .iter()
                .filter(|r| !r.report.successful)
                .map(|r| r.observation.observation_id.clone())
                .collect(),
            run: verified_run,
        };
        fs::write(
            &out,
            "observations.json",
            &serde_json::to_vec_pretty(&verified)?,
        )?;
        record.verified = Some(verified);
    }
    fs::write(&out, "import.json", &serde_json::to_vec_pretty(&record)?)?;
    fs::complete(&out)?;
    Ok(record)
}

fn verify(
    expected: &ExpectedScopeV1,
    identity: &ExpectedIdentity,
    run: &NativeRunV1,
    bytes: &BTreeMap<String, Vec<u8>>,
    record: &mut ImportRecord,
) {
    for (name, result) in [("expected", expected.validate()), ("run", run.validate())] {
        if let Err(error) = result {
            record.issue(IssueCode::Contract, name, error);
        }
    }
    if record
        .issues
        .iter()
        .any(|i| matches!(i.code, IssueCode::Contract))
    {
        return;
    }
    if run.subject.source != expected.subject
        || run.subject.run_id != identity.run_id
        || run.subject.scope_id != identity.scope_id
    {
        record.issue(
            IssueCode::Subject,
            "subject",
            "independent subject/run/scope expectation mismatch",
        );
    }
    for observation in &run.observations {
        let evaluated = expected.bindings.evaluated_at.as_datetime();
        let elapsed = *evaluated - *observation.started_at.as_datetime();
        // Check subsecond age precisely without converting an unbounded u64 to i64.
        if observation.observed_at > expected.bindings.evaluated_at
            || elapsed.num_seconds() < 0
            || elapsed.num_seconds() as u64 > expected.bindings.maximum_age_seconds
            || (elapsed.num_seconds() as u64 == expected.bindings.maximum_age_seconds
                && elapsed.subsec_nanos() > 0)
        {
            record.issue(
                IssueCode::Time,
                observation.id.as_str(),
                "future or stale observation",
            );
        }
    }
    for claim in &run.test_observations {
        let artifact = run
            .artifacts
            .iter()
            .find(|a| a.id == claim.raw_report.id)
            .expect("validated artifact reference");
        let Some(data) = bytes.get(&artifact.path) else {
            continue;
        };
        if raw_digest(data) != artifact.digest {
            continue;
        }
        let observation = run
            .observations
            .iter()
            .find(|o| o.id == claim.observation_id)
            .expect("validated observation");
        let binding = expected
            .bindings
            .observations
            .as_slice()
            .iter()
            .find(|b| b.id == claim.observation_id);
        if binding.and_then(|b| b.parser.as_ref()) != Some(&claim.parser) {
            record.issue(
                IssueCode::Observation,
                claim.observation_id.as_str(),
                "unapproved parser binding",
            );
            continue;
        }
        match reports::recompute(claim, data) {
            Err(error) => record.issue(IssueCode::Report, claim.observation_id.as_str(), error),
            Ok((parsed, recomputed)) => {
                if parsed.started_at < observation.started_at
                    || parsed.started_at > observation.completed_at
                {
                    record.issue(
                        IssueCode::Time,
                        claim.observation_id.as_str(),
                        "raw report start outside observation interval (replay)",
                    );
                }
                if &recomputed != claim {
                    record.issue(
                        IssueCode::Observation,
                        claim.observation_id.as_str(),
                        "raw report and claimed tests/counts/identity/attempts disagree",
                    );
                }
                if recomputed.counts.skipped != 0
                    || recomputed.counts.retries != 0
                    || recomputed.counts.discovered == 0
                {
                    record.issue(
                        IssueCode::Report,
                        claim.observation_id.as_str(),
                        "empty, skipped or retried report is incomplete",
                    );
                }
                record.reports.push(RecomputedReport {
                    observation_id: claim.observation_id.to_string(),
                    report: parsed,
                    observation: recomputed,
                });
            }
        }
    }
    // Every declared suite observation must have raw-backed test claims. Required
    // observation coverage and requirement satisfaction belong to the later verifier.
    for observation in &run.observations {
        if expected
            .bindings
            .observations
            .as_slice()
            .iter()
            .any(|b| b.id == observation.id && b.parser.is_some())
            && !run
                .test_observations
                .iter()
                .any(|t| t.observation_id == observation.id)
        {
            record.issue(
                IssueCode::Observation,
                observation.id.as_str(),
                "suite observation has no raw report claim",
            );
        }
    }
}
