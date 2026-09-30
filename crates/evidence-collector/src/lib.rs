#![forbid(unsafe_code)]

mod artifacts;
mod process;

use policy_contracts::{
    ArtifactDigest, CommitSha, ExecutionEvidence, ExecutionId, ExecutionStatus, ModelId,
    ProviderId, RepositoryId, Sha256Digest, TrustClass, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

/// Resource ceilings apply even to requests assembled programmatically.
pub const MAX_OUTPUT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_TIMEOUT_MS: u64 = 60 * 60 * 1000;

#[derive(Debug, thiserror::Error)]
pub enum CollectionError {
    #[error("invalid collection request: {0}")]
    Invalid(String),
    #[error("invalid request JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("cannot construct evidence digest: {0}")]
    Digest(#[from] policy_core::CanonicalError),
    #[error("bounded collection is supported on Linux and macOS only")]
    UnsupportedPlatform,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestFields {
    argv: Vec<String>,
    cwd: PathBuf,
    environment_allowlist: Vec<String>,
    timeout_ms: u64,
    stdout_limit_bytes: usize,
    stderr_limit_bytes: usize,
    version_args: Vec<String>,
    repository: RepositoryId,
    head_sha: CommitSha,
    provider: ProviderId,
    model: ModelId,
    execution_id: ExecutionId,
    input_digest: Sha256Digest,
    artifact_paths: Vec<PathBuf>,
}

/// A validated request. Trust is deliberately not an input field.
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "RequestFields")]
pub struct CollectionRequest(RequestFields);

impl CollectionRequest {
    pub fn try_from_json(json: &str) -> Result<Self, CollectionError> {
        Ok(serde_json::from_str(json)?)
    }
}

impl TryFrom<RequestFields> for CollectionRequest {
    type Error = CollectionError;
    fn try_from(mut fields: RequestFields) -> Result<Self, Self::Error> {
        let invalid = |message: &str| CollectionError::Invalid(message.into());
        if fields.argv.is_empty()
            || fields.argv[0].is_empty()
            || (!fields.argv[0].contains('/') && fields.argv[0].chars().any(char::is_whitespace))
            || fields
                .argv
                .iter()
                .chain(&fields.version_args)
                .any(|s| s.contains('\0'))
            || fields.version_args.is_empty()
        {
            return Err(invalid(
                "explicit nonempty argv and version_args are required",
            ));
        }
        reject_shell(Path::new(&fields.argv[0]))?;
        if !fields.cwd.is_absolute() || !fields.cwd.is_dir() {
            return Err(invalid("cwd must be an existing absolute directory"));
        }
        fields.cwd = fields
            .cwd
            .canonicalize()
            .map_err(|_| invalid("cwd cannot be resolved"))?;
        if !(1..=MAX_TIMEOUT_MS).contains(&fields.timeout_ms)
            || !(1..=MAX_OUTPUT_BYTES).contains(&fields.stdout_limit_bytes)
            || !(1..=MAX_OUTPUT_BYTES).contains(&fields.stderr_limit_bytes)
        {
            return Err(invalid(
                "timeout or output limit is outside supported bounds",
            ));
        }
        let mut names = BTreeSet::new();
        for name in &fields.environment_allowlist {
            if name.is_empty() || name.contains(['=', '\0']) || !names.insert(name.clone()) {
                return Err(invalid("environment names must be valid and unique"));
            }
        }
        fields.environment_allowlist = names.into_iter().collect();
        if fields.artifact_paths.len() > 64
            || fields.artifact_paths.iter().any(|path| {
                path.as_os_str().is_empty()
                    || path
                        .components()
                        .any(|c| !matches!(c, Component::Normal(_)))
            })
        {
            return Err(invalid(
                "artifacts must be at most 64 relative paths without traversal",
            ));
        }
        fields.artifact_paths.sort();
        if fields
            .artifact_paths
            .windows(2)
            .any(|pair| pair[0] == pair[1])
        {
            return Err(invalid("artifact paths must be unique"));
        }
        Ok(Self(fields))
    }
}

fn reject_shell(path: &Path) -> Result<(), CollectionError> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if matches!(
        name.as_str(),
        "sh" | "bash"
            | "dash"
            | "zsh"
            | "ksh"
            | "csh"
            | "tcsh"
            | "fish"
            | "ash"
            | "pwsh"
            | "powershell"
            | "powershell.exe"
            | "cmd"
            | "cmd.exe"
    ) {
        return Err(CollectionError::Invalid(
            "shell executables are not accepted".into(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandRecord {
    pub argv: Vec<String>,
    pub resolved_executable: Option<PathBuf>,
    pub cwd: PathBuf,
    pub environment_allowlist: Vec<String>,
    pub timeout_ms: u64,
    pub stdout_limit_bytes: usize,
    pub stderr_limit_bytes: usize,
    pub version_args: Vec<String>,
    pub repository: RepositoryId,
    pub head_sha: CommitSha,
    pub provider: ProviderId,
    pub model: ModelId,
    pub execution_id: ExecutionId,
    pub input_digest: Sha256Digest,
    pub artifact_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessRecord {
    pub status: ExecutionStatus,
    pub started_at: UtcTimestamp,
    pub completed_at: UtcTimestamp,
    pub exit_code: Option<i32>,
    /// Byte arrays retain invalid UTF-8 without changing the evidence.
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub error: Option<String>,
}

impl ProcessRecord {
    fn empty(status: ExecutionStatus, error: Option<String>) -> Self {
        let time = now();
        Self {
            status,
            started_at: time.clone(),
            completed_at: time,
            exit_code: None,
            stdout: vec![],
            stderr: vec![],
            error,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionRecord {
    pub command: CommandRecord,
    pub execution: ProcessRecord,
    pub version_probe: ProcessRecord,
    pub artifacts: Vec<ArtifactDigest>,
    pub artifact_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollectedExecution {
    pub evidence: ExecutionEvidence,
    pub record: ExecutionRecord,
}

fn now() -> UtcTimestamp {
    let time: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    UtcTimestamp::parse(&time.to_rfc3339()).expect("chrono emits valid UTC timestamps")
}

fn resolve(fields: &RequestFields, environment: &BTreeMap<String, OsString>) -> Option<PathBuf> {
    let executable = Path::new(&fields.argv[0]);
    let candidates = if executable.is_absolute() {
        vec![executable.to_path_buf()]
    } else if executable.components().count() > 1 {
        vec![fields.cwd.join(executable)]
    } else {
        environment
            .get("PATH")
            .map(|path| {
                std::env::split_paths(path)
                    .map(|directory| {
                        if directory.is_absolute() {
                            directory.join(executable)
                        } else {
                            fields.cwd.join(directory).join(executable)
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    candidates.into_iter().find_map(|candidate| {
        let resolved = candidate.canonicalize().ok()?;
        let metadata = resolved.metadata().ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o111 == 0 {
                return None;
            }
        }
        metadata.is_file().then_some(resolved)
    })
}

/// Collects local observations only. Runner authentication belongs to the CI adapter.
pub fn collect(request: CollectionRequest) -> Result<CollectedExecution, CollectionError> {
    if !cfg!(any(target_os = "linux", target_os = "macos")) {
        return Err(CollectionError::UnsupportedPlatform);
    }
    let fields = request.0;
    let started_at = now();
    let deadline = Instant::now() + Duration::from_millis(fields.timeout_ms);
    let environment: BTreeMap<_, _> = fields
        .environment_allowlist
        .iter()
        .filter_map(|name| std::env::var_os(name).map(|value| (name.clone(), value)))
        .collect();
    let resolved_executable = resolve(&fields, &environment);
    if let Some(path) = &resolved_executable {
        reject_shell(path)?;
    }
    let command = CommandRecord {
        argv: fields.argv,
        resolved_executable,
        cwd: fields.cwd,
        environment_allowlist: fields.environment_allowlist,
        timeout_ms: fields.timeout_ms,
        stdout_limit_bytes: fields.stdout_limit_bytes,
        stderr_limit_bytes: fields.stderr_limit_bytes,
        version_args: fields.version_args,
        repository: fields.repository,
        head_sha: fields.head_sha,
        provider: fields.provider,
        model: fields.model,
        execution_id: fields.execution_id,
        input_digest: fields.input_digest,
        artifact_paths: fields.artifact_paths,
    };
    let execution = process::run(&command, &command.argv[1..], &environment, deadline);
    // Completed negative checks still need tool identity; incomplete or damaged
    // executions cannot justify a probe. Both processes retain one deadline.
    let version_probe = if execution.status == ExecutionStatus::Passed
        || (execution.status == ExecutionStatus::Failed
            && execution.exit_code.is_some()
            && execution.error.is_none())
    {
        process::run(&command, &command.version_args, &environment, deadline)
    } else {
        ProcessRecord::empty(
            ExecutionStatus::Skipped,
            Some("main execution did not complete normally".into()),
        )
    };
    let (artifacts, artifact_errors) =
        artifacts::capture(&command.cwd, &command.artifact_paths, deadline);
    let status = if execution.status != ExecutionStatus::Passed {
        execution.status
    } else if version_probe.status != ExecutionStatus::Passed {
        version_probe.status
    } else if !artifact_errors.is_empty() {
        ExecutionStatus::Failed
    } else {
        ExecutionStatus::Passed
    };
    let record = ExecutionRecord {
        command,
        execution,
        version_probe,
        artifacts,
        artifact_errors,
    };
    let evidence = ExecutionEvidence {
        execution_id: record.command.execution_id.clone(),
        provider: record.command.provider.clone(),
        model: record.command.model.clone(),
        status,
        command_digest: policy_core::canonical_sha256(&record.command)?,
        input_digest: record.command.input_digest.clone(),
        output_digest: policy_core::canonical_sha256(&record)?,
        started_at,
        completed_at: now(),
        trust: TrustClass::LocalUntrusted,
        head_sha: Some(record.command.head_sha.clone()),
    };
    Ok(CollectedExecution { evidence, record })
}
