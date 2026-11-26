//! Binds a GitHub Actions artifact to its job and imports its raw native evidence.
use super::{
    github::{Artifact, GitHub, Job, Pull, Run},
    CheckEvidence, Result, Verdict,
};
use flate2::bufread::DeflateDecoder;
use policy_contracts::{ExecutionId, SensorId, UtcTimestamp};
use policy_native_contracts::{
    ArtifactRole, BindingsV1, ExpectedScopeV1, LocalMode, LocalScopeResultV1, NativeRunV1,
    ObservationBindingV1, RecordId, RequiredSet, RequirementsV1, ScopeStatus, SubjectV1, VersionV1,
};
use policy_native_importer::{
    import_run, populate_claims, raw_digest, ExpectedIdentity, ImportRecord, MappingSelection,
    ReportSelection,
};
use policy_native_verifier::evaluate_scope;
use rawzip::{CompressionMethod, ZipArchive};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

pub(super) const MAX_ARCHIVE: usize = 64 * 1024 * 1024;
const MAX_UNCOMPRESSED: u64 = 256 * 1024 * 1024;
const MAX_ENTRIES: u64 = 4_096;
const MANIFEST: &str = "run.json";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct NativePolicy {
    artifact_key: Option<SensorId>,
    requirements: RequirementsV1,
    bindings: RequiredSet<ObservationBindingV1>,
    #[serde(default)]
    reports: Vec<ReportSelection>,
    #[serde(default)]
    mappings: Vec<MappingSelection>,
}

#[derive(Debug, Serialize)]
pub(super) struct NativeEvidence {
    artifact: Artifact,
    import: Option<ImportRecord>,
    evaluation: Option<LocalScopeResultV1>,
    observations: Vec<SensorId>,
}

pub(super) struct JobContext<'a> {
    pub api: &'a GitHub<'a>,
    pub pull: &'a Pull,
    pub run: &'a Run,
    pub job: &'a Job,
    pub evaluated_at: &'a UtcTimestamp,
    pub maximum_age_seconds: u64,
}

impl NativePolicy {
    pub fn artifact_key<'a>(&'a self, check: &'a SensorId) -> &'a SensorId {
        self.artifact_key.as_ref().unwrap_or(check)
    }

    pub fn evaluate(
        &self,
        context: &JobContext<'_>,
        evidence: &mut CheckEvidence,
        artifacts: &[Artifact],
    ) -> Result<Verdict> {
        let key = self.artifact_key(&evidence.check);
        let name = format!("native-run-{key}");
        // Artifacts are listed by run, including older attempts. Job timestamps
        // and the upload window bind a stable artifact name to the latest attempt.
        let mut matches = artifacts.iter().filter(|artifact| {
            artifact.name == name && artifact.created_at >= context.run.run_started_at
        });
        let artifact = matches
            .next()
            .ok_or_else(|| format!("missing artifact {name}"))?;
        if matches.next().is_some() {
            return Err(format!("conflicting duplicate artifact {name}").into());
        }
        let mut details = NativeEvidence {
            artifact: artifact.clone(),
            import: None,
            evaluation: None,
            observations: vec![],
        };
        let verdict = self.import(context, key, &mut details);
        evidence.native = Some(details);
        verdict
    }

    fn expected(&self, context: &JobContext<'_>, subject: SubjectV1) -> ExpectedScopeV1 {
        ExpectedScopeV1 {
            schema_version: VersionV1::default(),
            subject,
            requirements: self.requirements.clone(),
            bindings: BindingsV1 {
                observations: self.bindings.clone(),
                evaluated_at: context.evaluated_at.clone(),
                maximum_age_seconds: context.maximum_age_seconds,
                mode: LocalMode::LocalAdvisory,
            },
        }
    }

    fn import(
        &self,
        context: &JobContext<'_>,
        key: &SensorId,
        evidence: &mut NativeEvidence,
    ) -> Result<Verdict> {
        let artifact = &evidence.artifact;
        let end = context
            .job
            .completed_at
            .as_ref()
            .ok_or("job has no completion time")?;
        if artifact.id == 0
            || artifact.expired
            || artifact.expires_at <= *context.evaluated_at
            || artifact.size_in_bytes == 0
            || artifact.size_in_bytes > MAX_ARCHIVE as u64
            || artifact.created_at < context.job.started_at
            || artifact.created_at > *end
            || !artifact.workflow_run.as_ref().is_some_and(|binding| {
                binding.id == context.run.id
                    && binding.repository_id == context.pull.base.repo.id
                    && binding.head_repository_id == context.pull.head.repo.id
                    && binding.head_sha == context.pull.head.sha
            })
        {
            return Err("artifact is expired, oversized, or outside the selected run attempt and job time window".into());
        }
        let archive = context.api.archive(artifact.id)?;
        let digest = format!("sha256:{}", raw_digest(&archive).sha256);
        if artifact.size_in_bytes != archive.len() as u64
            || artifact.digest.as_deref() != Some(digest.as_str())
        {
            return Err("artifact archive size or SHA-256 does not match GitHub metadata".into());
        }
        let scratch = tempfile::tempdir()?;
        let root = scratch.path().canonicalize()?;
        let input = root.join("artifacts");
        fs::create_dir(&input)?;
        let mut run = unpack(&archive, &input)?;
        if run.observations.iter().any(|observation| {
            observation.started_at.truncated_to_seconds() < context.job.started_at
                || observation.observed_at.truncated_to_seconds() > *end
        }) {
            return Err(
                "native observations occurred outside the selected job's time window".into(),
            );
        }
        let subject = &run.subject.source;
        if !subject
            .repository
            .as_str()
            .eq_ignore_ascii_case(context.pull.base.repo.full_name.as_str())
        {
            return Err("native repository does not match the PR base repository".into());
        }
        context
            .api
            .confirm_checkout(&subject.head, &context.pull.head.sha)?;
        let mut inventories = run
            .artifacts
            .iter()
            .filter(|item| item.role == ArtifactRole::SourceInventory);
        if inventories
            .next()
            .is_some_and(|item| item.digest != subject.source_inventory_digest)
            || inventories.next().is_some()
        {
            return Err("native source inventory digest is inconsistent or ambiguous".into());
        }
        // Source/scope set digests are producer metadata. The repository and
        // checkout above are independently bound by GitHub, and the policy
        // supplies the approved commands, configurations and test requirements.
        let expected = self.expected(context, subject.clone());
        let identity = ExpectedIdentity {
            run_id: ExecutionId::parse(format!("{}-{key}", context.run.id))?,
            scope_id: RecordId::parse(key.as_str())?,
        };
        if !self.reports.is_empty() || !self.mappings.is_empty() {
            run = populate_claims(&input, &expected, &run, &self.reports, &self.mappings)?;
        }
        evidence.import = Some(import_run(
            &input,
            &root.join("import"),
            &expected,
            &identity,
            &run,
        )?);
        let verified = evidence
            .import
            .as_ref()
            .and_then(ImportRecord::verified_observations)
            .ok_or("NativeRunV1 importer rejected the artifact")?;
        let evaluation = evaluate_scope(&expected, verified)?.result;
        evidence.observations = verified
            .run
            .observations
            .iter()
            .map(|observation| observation.id.clone())
            .collect();
        let status = evaluation.status;
        evidence.evaluation = Some(evaluation);
        match status {
            ScopeStatus::Satisfied => Ok(Verdict::Ready),
            ScopeStatus::NonPass => Ok(Verdict::NotReady),
            ScopeStatus::Error => Err("native scope evidence is incomplete or invalid".into()),
        }
    }
}

fn unpack(bytes: &[u8], destination: &Path) -> Result<NativeRunV1> {
    let archive = ZipArchive::from_slice(bytes)?;
    let count = archive.entries_hint();
    if count == 0 || count > MAX_ENTRIES {
        return Err("invalid artifact ZIP entry count".into());
    }
    let mut total = 0;
    let mut paths = BTreeSet::new();
    let mut ranges = Vec::new();
    let mut manifest = None;
    let mut entries = archive.entries();
    while let Some(entry) = entries.next_entry()? {
        let path = entry.file_path();
        let path = std::str::from_utf8(path.as_ref())?;
        let path = if entry.is_dir() {
            path.strip_suffix('/').unwrap_or(path)
        } else {
            path
        };
        let mode = entry.mode().value() & 0o170000;
        if !safe_path(path)
            || !paths.insert(path.to_owned())
            || paths.len() as u64 > count
            || entry.flags().is_encrypted()
            || (if entry.is_dir() {
                mode != 0 && mode != 0o040000
            } else {
                mode != 0 && mode != 0o100000
            })
        {
            return Err("artifact ZIP contains a duplicate or unsupported path or entry".into());
        }
        if entry.is_dir() {
            if path != "artifacts" && !path.starts_with("artifacts/") {
                return Err("artifact ZIP directory is outside artifacts/".into());
            }
            continue;
        }
        let size = entry.uncompressed_size_hint();
        let limit = (MAX_ARCHIVE as u64).min(MAX_UNCOMPRESSED - total);
        if size > limit {
            return Err("artifact ZIP exceeds extraction limits".into());
        }
        let local = archive.get_entry(entry.wayfinder())?;
        if local.local_header().file_path().as_ref() != entry.file_path().as_ref() {
            return Err("artifact ZIP local and central entry paths differ".into());
        }
        let range = local.compressed_data_range();
        if range.0 != range.1 {
            ranges.push(range);
        }
        let reader: Box<dyn Read> = match entry.compression_method() {
            CompressionMethod::STORE => Box::new(local.data()),
            CompressionMethod::DEFLATE => Box::new(DeflateDecoder::new(local.data())),
            _ => return Err("unsupported ZIP compression in artifact".into()),
        };
        let mut reader = local.verifying_reader(reader).take(size + 1);
        if path == MANIFEST {
            let mut data = Vec::new();
            reader.read_to_end(&mut data)?;
            if data.len() as u64 != size {
                return Err("artifact ZIP manifest size differs from metadata".into());
            }
            manifest = Some(serde_json::from_slice(&data)?);
        } else {
            let relative = path
                .strip_prefix("artifacts/")
                .ok_or("native artifact ZIP entries must be under artifacts/")?;
            let path = destination.join(relative);
            fs::create_dir_all(path.parent().ok_or("missing ZIP entry parent")?)?;
            let mut file = File::create_new(path)?;
            if io::copy(&mut reader, &mut file)? != size {
                return Err("artifact ZIP entry size differs from metadata".into());
            }
        }
        total += size;
    }
    if paths.len() as u64 != count {
        return Err("artifact ZIP entry count differs from metadata".into());
    }
    ranges.sort_unstable();
    if ranges.windows(2).any(|pair| pair[0].1 > pair[1].0) {
        return Err("artifact ZIP contains overlapping file data".into());
    }
    manifest.ok_or_else(|| format!("artifact ZIP is missing {MANIFEST}").into())
}

pub(super) fn safe_path(path: &str) -> bool {
    !path.bytes().any(|byte| byte.is_ascii_control())
        && !path.contains(['\\', ':'])
        && path.split('/').all(|part| !matches!(part, "" | "." | ".."))
}
