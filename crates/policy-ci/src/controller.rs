//! Reference authentication boundary. Keys and expectations come from an owner
//! controlled configuration, never from the submitted observation. Signatures
//! authenticate an attester's statement; the attester must actually observe it.
use base64::{engine::general_purpose::STANDARD, Engine};
use policy_contracts::*;
use policy_native_verifier::{contracts::*, evaluate_scope, ImportedObservationsV1};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    error::Error,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
const DOMAIN: &str = "policy-kernel/controller-observation/v1";
const PRODUCER_DOMAIN: &str = "policy-kernel/controller-producer/v1";
const LOCAL_DOMAIN: &str = "policy-kernel/local-runner-attestation/v1";
const HARDWARE_KEYS: [&str; 2] = [
    "sk-ssh-ed25519@openssh.com",
    "sk-ecdsa-sha2-nistp256@openssh.com",
];
const LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunBinding {
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_run_attempt: u64,
    pub workflow_trigger_head_sha: CommitSha,
    pub workflow_head_sha: CommitSha,
    pub workflow_id: u64,
    pub workflow_path: String,
    pub workflow_ref: String,
    pub workflow_source_sha: CommitSha,
    pub check_app_id: u64,
    pub check_app_slug: String,
    pub policy_digest: Sha256Digest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerKey {
    pub key_id: String,
    pub public_key: PathBuf,
    pub provider: ProviderId,
    pub model: ModelId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControllerConfig {
    pub schema_version: u32,
    pub expected: RunBinding,
    pub observer_public_key: PathBuf,
    pub producer_keys: Vec<ProducerKey>,
    /// An owner-installed absolute path, never a checkout/PATH executable.
    pub openssl: PathBuf,
    /// This reference requires at least two independent review providers.
    pub minimum_reviewers: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local_runners: Vec<LocalRunnerKey>,
    /// Owner-installed SSHSIG verifier. Required by configuration schema 2.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_keygen: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalRunnerKey {
    pub runner_id: String,
    /// One OpenSSH public key or one exact-principal allowed_signers entry.
    pub public_key: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalAttestationInput {
    pub attestation: SignedLocalAttestationV1,
    pub expected_scope: ExpectedScopeV1,
    pub observations: ImportedObservationsV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    /// The observer certifies its complete execution/commit ledger was read.
    pub complete: bool,
    pub contributors: Vec<ExecutionId>,
    pub executions: Vec<ExecutionId>,
    pub reviews: Vec<ExecutionId>,
    pub sensors: Vec<SensorId>,
    pub knowledge: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub domain: String,
    pub binding: RunBinding,
    pub policy: PolicySnapshot,
    pub context: PreMergeContext,
    pub work_item: WorkItemObservation,
    /// Original collector evidence; every trust field must remain local_untrusted.
    pub evidence: EvidenceBundle,
    pub inventory: Inventory,
    /// Commit tree independently read by the observer from GitHub, never the runner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_head_tree: Option<CommitSha>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local_attestations: Vec<LocalAttestationInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProducerSignature {
    pub key_id: String,
    pub execution_id: ExecutionId,
    /// Hex encoding of a detached RSA PKCS#1 v1.5 / SHA-256 signature.
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedObservation {
    pub observation: Observation,
    pub observer_signature: String,
    pub producers: Vec<ProducerSignature>,
}

/// Bytes signed by each execution/review producer after the bundle is assembled.
/// Signing the full digest also binds findings and omissions, not only a verdict.
pub fn producer_payload(observation: &Observation, execution_id: &ExecutionId) -> Result<Vec<u8>> {
    canonical(&serde_json::json!({
        "domain": PRODUCER_DOMAIN,
        "binding": observation.binding,
        "evidence_digest": policy_core::canonical_sha256(&observation.evidence)?,
        "execution_id": execution_id,
    }))
}

pub fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(policy_core::canonical_json(value)?)
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    Ok(serde_json::from_slice(&read_file(path)?)?)
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    if fs::symlink_metadata(path)?.file_type().is_symlink() {
        return Err("authentication inputs cannot be symlinks".into());
    }
    #[cfg(unix)]
    let file = open_input(path)?;
    #[cfg(not(unix))]
    let file = Err::<fs::File, _>("bounded authentication input requires Unix")?;
    if !file.metadata()?.is_file() {
        return Err("authentication input must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1).read_to_end(&mut bytes)?;
    if (bytes.len() as u64) > LIMIT {
        return Err("authentication input exceeds 8 MiB".into());
    }
    Ok(bytes)
}

#[cfg(unix)]
fn open_input(path: &Path) -> Result<fs::File> {
    use rustix::fs::{open, Mode, OFlags};
    Ok(open(
        path,
        OFlags::RDONLY
            .union(OFlags::CLOEXEC)
            .union(OFlags::NONBLOCK)
            .union(OFlags::NOFOLLOW),
        Mode::empty(),
    )
    .map_err(std::io::Error::from)?
    .into())
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "policy-controller-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path)?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn verify(openssl: &Path, key: &Path, signature: &str, payload: &[u8]) -> Result<()> {
    if signature.is_empty() || signature.len() > 16384 || signature.len() % 2 != 0 {
        return Err("invalid detached signature encoding".into());
    }
    let signature = signature
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair)?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect::<Result<Vec<_>>>()?;
    let scratch = Scratch::new()?;
    fs::write(scratch.0.join("key.pem"), read_file(key)?)?;
    fs::write(scratch.0.join("signature"), signature)?;
    fs::write(scratch.0.join("payload"), payload)?;
    let mut command = Command::new(openssl);
    command
        .stdin(Stdio::null())
        .args([
            "dgst",
            "-sha256",
            "-sigopt",
            "rsa_padding_mode:pkcs1",
            "-verify",
        ])
        .arg(scratch.0.join("key.pem"))
        .arg("-signature")
        .arg(scratch.0.join("signature"))
        .arg(scratch.0.join("payload"));
    bounded_crypto(&mut command)
}

fn bounded_crypto(command: &mut Command) -> Result<()> {
    let mut child = command
        .env_clear()
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err("signature authentication failed".into())
            };
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("signature verification timed out".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn public_key_identity(openssl: &Path, key: &Path) -> Result<Vec<u8>> {
    let scratch = Scratch::new()?;
    fs::write(scratch.0.join("key.pem"), read_file(key)?)?;
    let mut command = Command::new(openssl);
    command
        .stdin(Stdio::null())
        .args(["pkey", "-pubin", "-in"])
        .arg(scratch.0.join("key.pem"))
        .args(["-outform", "DER", "-out"])
        .arg(scratch.0.join("key.der"));
    bounded_crypto(&mut command)?;
    read_file(&scratch.0.join("key.der"))
}

/// Location checks supplement the required separate OS identity / runner boundary;
/// they cannot prove that another process sharing the controller's UID is absent.
pub fn validate_isolation(
    config_path: &Path,
    config: &ControllerConfig,
    checkout: &Path,
    output: &Path,
) -> Result<()> {
    let checkout = checkout.canonicalize()?;
    let mut protected = vec![
        config_path.to_path_buf(),
        config.openssl.clone(),
        config.observer_public_key.clone(),
        std::env::current_exe()?,
    ];
    protected.extend(
        config
            .producer_keys
            .iter()
            .map(|key| key.public_key.clone()),
    );
    protected.extend(config.ssh_keygen.iter().cloned());
    protected.extend(
        config
            .local_runners
            .iter()
            .map(|key| key.public_key.clone()),
    );
    for path in protected {
        if path.canonicalize()?.starts_with(&checkout) {
            return Err("controller configuration, keys and executables must be outside the contributor checkout".into());
        }
    }
    let parent = output
        .parent()
        .ok_or("output requires a parent directory")?
        .canonicalize()?;
    if parent.starts_with(&checkout) {
        return Err("controller output must be outside the contributor checkout".into());
    }
    Ok(())
}

fn exact_inventory<'a>(
    expected: impl Iterator<Item = &'a str>,
    actual: impl Iterator<Item = &'a str>,
) -> bool {
    let expected: Vec<_> = expected.collect();
    let actual: Vec<_> = actual.collect();
    let left: BTreeSet<_> = expected.iter().collect();
    let right: BTreeSet<_> = actual.iter().collect();
    left.len() == expected.len() && right.len() == actual.len() && left == right
}

struct RegisteredLocalKey {
    runner_id: String,
    allowed_signers: String,
    blob: Vec<u8>,
}

/// Deliberately narrower than OpenSSH's pattern registry: one key and one exact
/// principal per owner entry. OpenSSH enforces namespaces and key validity options.
fn local_key(entry: &LocalRunnerKey) -> Result<RegisteredLocalKey> {
    if entry.runner_id.is_empty()
        || !entry
            .runner_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b))
        || !entry.public_key.is_absolute()
    {
        return Err("invalid local runner registry entry".into());
    }
    let bytes = read_file(&entry.public_key)?;
    let text = std::str::from_utf8(&bytes)?;
    let lines: Vec<_> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    if lines.len() != 1 || text.contains('\0') {
        return Err("local runner registry requires exactly one key".into());
    }
    let fields: Vec<_> = lines[0].split_ascii_whitespace().collect();
    let index = fields
        .iter()
        .enumerate()
        .position(|(index, _)| {
            // A principal or comment can share an algorithm prefix. Identify
            // the key by its encoded algorithm, not the spelling alone.
            fields.get(index + 1).is_some_and(|encoded| {
                STANDARD
                    .decode(encoded)
                    .is_ok_and(|blob| ssh_string(&mut blob.as_slice()).is_ok())
            })
        })
        .ok_or("invalid local runner public key")?;
    if fields[..index]
        .iter()
        .any(|field| field.contains("no-touch-required"))
    {
        return Err("local runner no-touch-required option is forbidden".into());
    }
    if !HARDWARE_KEYS.contains(&fields[index]) {
        return Err("local runner key type is not hardware backed".into());
    }
    if index > 2
        || (index > 0 && fields[0] != entry.runner_id)
        || (index == 2 && fields[1].contains("cert-authority"))
    {
        return Err("local runner registry requires an exact principal and direct key".into());
    }
    let blob = STANDARD.decode(
        fields
            .get(index + 1)
            .ok_or("missing local runner key bytes")?,
    )?;
    let mut encoded = blob.as_slice();
    if ssh_string(&mut encoded)? != fields[index].as_bytes() {
        return Err("local runner key type does not match key bytes".into());
    }
    let allowed_signers = if index == 0 {
        format!("{} {}\n", entry.runner_id, lines[0])
    } else {
        format!("{}\n", lines[0])
    };
    Ok(RegisteredLocalKey {
        runner_id: entry.runner_id.clone(),
        allowed_signers,
        blob,
    })
}

fn local_registry(config: &ControllerConfig, raw: &Observation) -> Result<Vec<RegisteredLocalKey>> {
    match config.schema_version {
        1 if config.local_runners.is_empty()
            && config.ssh_keygen.is_none()
            && raw.local_attestations.is_empty()
            && raw.github_head_tree.is_none() =>
        {
            return Ok(vec![])
        }
        2 if config
            .ssh_keygen
            .as_ref()
            .is_some_and(|path| path.is_absolute()) => {}
        _ => return Err("controller configuration/current-run binding mismatch".into()),
    }
    let mut ids = BTreeSet::new();
    let mut keys = BTreeSet::new();
    config
        .local_runners
        .iter()
        .map(|entry| {
            let key = local_key(entry)?;
            if !ids.insert(key.runner_id.clone()) || !keys.insert(key.blob.clone()) {
                return Err("local runner keys must be distinct and unambiguous".into());
            }
            Ok(key)
        })
        .collect()
}

fn ssh_bytes<'a>(bytes: &mut &'a [u8], length: usize) -> Result<&'a [u8]> {
    if bytes.len() < length {
        return Err("malformed local SSH signature".into());
    }
    let (value, rest) = bytes.split_at(length);
    *bytes = rest;
    Ok(value)
}

fn ssh_u32(bytes: &mut &[u8]) -> Result<u32> {
    Ok(u32::from_be_bytes(ssh_bytes(bytes, 4)?.try_into()?))
}

fn ssh_string<'a>(bytes: &mut &'a [u8]) -> Result<&'a [u8]> {
    let length = ssh_u32(bytes)? as usize;
    ssh_bytes(bytes, length)
}

/// OpenSSH verifies FIDO cryptography but does not enforce the user-presence bit
/// in SSHSIG. Inspect the same signed flags byte, then require crypto verification.
fn local_signature_presence(signature: &str, key: &[u8]) -> Result<()> {
    if signature.len() > 16384 {
        return Err("local SSH signature exceeds size limit".into());
    }
    let armor = signature
        .trim()
        .strip_prefix("-----BEGIN SSH SIGNATURE-----")
        .and_then(|value| value.strip_suffix("-----END SSH SIGNATURE-----"))
        .ok_or("malformed local SSH signature armor")?;
    // Decode only the armored body, retaining no interpretation of crypto data.
    let body: String = armor.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    let decoded = STANDARD.decode(body)?;
    let mut bytes = decoded.as_slice();
    if ssh_bytes(&mut bytes, 6)? != b"SSHSIG" || ssh_u32(&mut bytes)? != 1 {
        return Err("malformed local SSH signature version".into());
    }
    if ssh_string(&mut bytes)? != key {
        return Err("local signature key mismatch".into());
    }
    if ssh_string(&mut bytes)? != LOCAL_DOMAIN.as_bytes() {
        return Err("local signature domain mismatch".into());
    }
    if !ssh_string(&mut bytes)?.is_empty() {
        return Err("local signature reserved field is not empty".into());
    }
    if ![b"sha256".as_slice(), b"sha512".as_slice()].contains(&ssh_string(&mut bytes)?) {
        return Err("unsupported local signature hash".into());
    }
    let mut signature = ssh_string(&mut bytes)?;
    let mut key = key;
    if !bytes.is_empty() || ssh_string(&mut signature)? != ssh_string(&mut key)? {
        return Err("local signature key algorithm mismatch".into());
    }
    let _crypto = ssh_string(&mut signature)?;
    let flags = ssh_bytes(&mut signature, 1)?[0];
    let _counter = ssh_u32(&mut signature)?;
    if !signature.is_empty() {
        return Err("malformed local SSH signature trailing data".into());
    }
    if flags & 1 == 0 {
        return Err("local signature requires user presence".into());
    }
    Ok(())
}

fn verify_local(
    executable: &Path,
    key: &RegisteredLocalKey,
    input: &LocalAttestationInput,
    now: &UtcTimestamp,
) -> Result<()> {
    input.attestation.validate()?;
    local_signature_presence(input.attestation.signature.value.as_str(), &key.blob)?;
    let scratch = Scratch::new()?;
    fs::write(scratch.0.join("allowed_signers"), &key.allowed_signers)?;
    fs::write(
        scratch.0.join("signature"),
        input.attestation.signature.value.as_str(),
    )?;
    fs::write(
        scratch.0.join("payload"),
        canonical(&input.attestation.statement)?,
    )?;
    let mut command = Command::new(executable);
    command
        .args(["-Y", "verify", "-f"])
        .arg(scratch.0.join("allowed_signers"))
        .args(["-I", &key.runner_id, "-n", LOCAL_DOMAIN, "-s"])
        .arg(scratch.0.join("signature"))
        .arg("-O")
        // ssh-keygen takes YYYYMMDDHHMMSSZ: the canonical whole-second form
        // without its separators.
        .arg(format!(
            "verify-time={}",
            now.truncated_to_seconds()
                .to_string()
                .replace(['-', ':', 'T'], "")
        ))
        .stdin(fs::File::open(scratch.0.join("payload"))?);
    bounded_crypto(&mut command)
}

fn local_evidence(
    config: &ControllerConfig,
    raw: &Observation,
    registry: &[RegisteredLocalKey],
    now: &UtcTimestamp,
) -> Result<Vec<EvidenceBundle>> {
    let mut results = vec![];
    let mut executions = BTreeSet::new();
    let mut sensors = BTreeSet::new();
    let mut runs = BTreeSet::new();
    for input in &raw.local_attestations {
        let s = &input.attestation.statement;
        let key = registry
            .iter()
            .find(|key| key.runner_id == s.runner.runner_id.as_str())
            .ok_or("local runner is not registered")?;
        verify_local(
            config.ssh_keygen.as_deref().ok_or("missing SSH verifier")?,
            key,
            input,
            now,
        )?;
        let policy = raw
            .policy
            .local_runner_evidence
            .as_ref()
            .filter(|_| raw.policy.schema_version == 4)
            .ok_or("local runner requires policy v4 opt-in")?;
        if !policy
            .allowed_runner_ids
            .iter()
            .any(|id| id == s.runner.runner_id.as_str())
        {
            return Err("local runner is not allowed by policy".into());
        }
        // evaluate_scope independently binds the native run to this scope.
        if s.subject != input.expected_scope.subject
            || s.subject.repository != raw.binding.repository
            || s.subject.head != raw.binding.head_sha
            || s.subject.source_set_digest.sha256 != raw.evidence.manifest.source_set_digest
            || Some(&s.head_tree) != raw.github_head_tree.as_ref()
            || s.base != raw.evidence.manifest.base_sha
            || s.run_id != input.observations.run.subject.run_id
            || s.scope_id != input.observations.run.subject.scope_id
        {
            return Err("local subject/GitHub binding mismatch".into());
        }
        let template = input.expected_scope.template_digest()?;
        if s.expected_scope_digest != input.expected_scope.digest()?
            || s.scope_template_digest != template
            || s.native_run_digest != input.observations.run.digest()?
            || s.observations_digest
                != canonical_local_digest(
                    LocalDigestRole::ImportedObservations,
                    &input.observations,
                )?
        {
            return Err("local digest mismatch".into());
        }
        if !policy
            .approved_scope_template_digests
            .contains(&template.sha256)
        {
            return Err("local scope template is not approved".into());
        }
        let evaluation = evaluate_scope(&input.expected_scope, &input.observations)?;
        if evaluation.result.status != ScopeStatus::Satisfied {
            return Err("local scope is not satisfied".into());
        }
        if s.result_digest != evaluation.result.digest()? {
            return Err("local digest mismatch".into());
        }
        let maximum_age = policy
            .maximum_age_seconds
            .min(input.expected_scope.bindings.maximum_age_seconds);
        if [
            &s.started_at,
            &s.completed_at,
            &s.attested_at,
            &input.expected_scope.bindings.evaluated_at,
        ]
        .into_iter()
        .any(|time| !local_time_is_fresh(time, now, maximum_age))
        {
            return Err("local evidence is stale or future dated".into());
        }
        if input.expected_scope.bindings.evaluated_at < s.completed_at
            || input.expected_scope.bindings.evaluated_at > s.attested_at
            || input
                .observations
                .run
                .observations
                .iter()
                .any(|observation| {
                    observation.started_at < s.started_at
                        || observation.completed_at > s.completed_at
                        || observation.observed_at > s.completed_at
                })
        {
            return Err("local observation is outside attested run interval".into());
        }
        let evidence = evaluation
            .evidence
            .ok_or("satisfied local scope has no evidence")?;
        if !runs.insert(s.run_id.clone())
            || evidence.executions.iter().any(|e| {
                !executions.insert(e.execution_id.clone())
                    || raw
                        .evidence
                        .modifications
                        .iter()
                        .any(|item| item.execution_id == e.execution_id)
                    || raw
                        .evidence
                        .reviews
                        .iter()
                        .any(|item| item.execution_id == e.execution_id)
            })
            || evidence
                .sensors
                .iter()
                .any(|s| !sensors.insert(s.sensor_id.clone()))
        {
            return Err("local coverage must be complete and disjoint".into());
        }
        results.push(evidence);
    }
    Ok(results)
}

fn local_time_is_fresh(time: &UtcTimestamp, now: &UtcTimestamp, maximum_age: u64) -> bool {
    now.duration_since(time)
        .is_some_and(|age| age <= Duration::from_secs(maximum_age))
}

fn rehash_bundle(evidence: &mut EvidenceBundle) -> Result<()> {
    let value = serde_json::to_value(&evidence)?;
    for child in &mut evidence.manifest.child_digests {
        child.digest = policy_core::canonical_sha256(&value[&child.path])?;
    }
    Ok(())
}

/// The single aggregate adequacy slot cannot separate submitted local claims
/// from CI claims. With local input, derive it solely from verified full scopes.
fn install_local(evidence: &mut EvidenceBundle, local: Vec<EvidenceBundle>) -> Result<()> {
    if local.is_empty() {
        return Ok(());
    }
    let mut adequacy: Option<TestAdequacyEvidence> = None;
    for bundle in local {
        for mut item in bundle.executions {
            item.trust = TrustClass::LocalRunnerAttested;
            let slot = evidence
                .executions
                .iter_mut()
                .find(|v| v.execution_id == item.execution_id)
                .ok_or("missing covered local execution")?;
            *slot = item;
        }
        for mut item in bundle.sensors {
            item.trust = TrustClass::LocalRunnerAttested;
            let slot = evidence
                .sensors
                .iter_mut()
                .find(|v| v.sensor_id == item.sensor_id)
                .ok_or("missing covered local sensor")?;
            *slot = item;
        }
        let next = bundle.test_adequacy.ok_or("missing local adequacy")?;
        if let Some(total) = &mut adequacy {
            if total
                .acceptance_criteria
                .iter()
                .chain(&total.changed_behaviors)
                .any(|mapping| {
                    next.acceptance_criteria
                        .iter()
                        .chain(&next.changed_behaviors)
                        .any(|other| mapping.requirement_id == other.requirement_id)
                })
            {
                return Err("local adequacy requirements overlap".into());
            }
            total.discovered = total
                .discovered
                .checked_add(next.discovered)
                .ok_or("local adequacy count overflow")?;
            total.executed = total
                .executed
                .checked_add(next.executed)
                .ok_or("local adequacy count overflow")?;
            total.passed = total
                .passed
                .checked_add(next.passed)
                .ok_or("local adequacy count overflow")?;
            total.acceptance_criteria.extend(next.acceptance_criteria);
            total.changed_behaviors.extend(next.changed_behaviors);
            total.negative_case_ids.extend(next.negative_case_ids);
            total.executed_test_ids.extend(next.executed_test_ids);
        } else {
            adequacy = Some(next);
        }
    }
    evidence.test_adequacy = adequacy;
    Ok(())
}

/// Authenticate all statements before issuing any files. This function performs
/// no GitHub writes and does not issue an authorization decision.
pub fn produce(config: &ControllerConfig, signed: &SignedObservation, output: &Path) -> Result<()> {
    let raw = &signed.observation;
    let binding = &config.expected;
    raw.policy
        .validate_version()
        .map_err(|error| format!("invalid review policy: {error}"))?;
    let reviews =
        raw.policy.required_reviews.as_ref().ok_or(
            "PK-POLICY-V3-MIGRATION-REQUIRED: reapprove policy as v3 with required_reviews",
        )?;
    if config.minimum_reviewers as u64 != u64::from(reviews.minimum_reviewers.max(2)) {
        return Err("controller configuration/current-run binding mismatch: minimum_reviewers must equal max(2, policy required_reviews.minimum_reviewers)".into());
    }
    let registry = local_registry(config, raw)?;
    if raw.domain != DOMAIN
        || &raw.binding != binding
        || binding.workflow_run_attempt == 0
        || binding.workflow_id == 0
        || binding.check_app_id == 0
        || binding.check_app_slug.trim().is_empty()
        || !binding.workflow_path.starts_with(".github/workflows/")
        || !binding.workflow_ref.starts_with(&format!(
            "{}/{}@",
            binding.repository, binding.workflow_path
        ))
        || config.minimum_reviewers < 2
        || !config.openssl.is_absolute()
        || !config.observer_public_key.is_absolute()
        || config
            .producer_keys
            .iter()
            .any(|key| !key.public_key.is_absolute())
    {
        return Err("controller configuration/current-run binding mismatch".into());
    }
    let mut key_ids = BTreeSet::new();
    let mut key_contents = BTreeSet::new();
    key_contents.insert(public_key_identity(
        &config.openssl,
        &config.observer_public_key,
    )?);
    for key in &config.producer_keys {
        if key.key_id.is_empty()
            || !key_ids.insert(&key.key_id)
            || !key_contents.insert(public_key_identity(&config.openssl, &key.public_key)?)
        {
            return Err("observer and producer keys must be distinct and unambiguous".into());
        }
    }
    verify(
        &config.openssl,
        &config.observer_public_key,
        &signed.observer_signature,
        &canonical(raw)?,
    )?;
    let manifest = &raw.evidence.manifest;
    if policy_core::pre_merge_policy_digest(&raw.policy)? != binding.policy_digest
        || raw.policy.policy_digest != binding.policy_digest
        || raw.policy.required_check_name != "policy-kernel/pre-merge"
        || manifest.policy_digest != binding.policy_digest
        || manifest.repository != binding.repository
        || manifest.head_sha != binding.head_sha
        || manifest.workflow_run_id != binding.workflow_run_id
        || raw.context.workflow_run_id != binding.workflow_run_id
        || raw.work_item.repository != binding.repository
        || raw.work_item.work_item_id != manifest.work_item_id
        || raw.policy.sensor_profile_digest.as_ref()
            != Some(&policy_core::canonical_sha256(&raw.context.sensor_profile)?)
        || !policy_core::validate_bundle(&raw.evidence).is_valid()
    {
        return Err(
            "authenticated observation policy/profile/head/content binding mismatch".into(),
        );
    }
    let inv = &raw.inventory;
    let bundle = &raw.evidence;
    let knowledge: Vec<_> = bundle
        .knowledge
        .iter()
        .map(|item| format!("{}:{}", knowledge_kind(item.kind), item.source))
        .collect();
    if !inv.complete
        || inv.contributors.is_empty()
        || !exact_inventory(
            inv.contributors.iter().map(ExecutionId::as_str),
            bundle.modifications.iter().map(|v| v.execution_id.as_str()),
        )
        || !exact_inventory(
            inv.executions.iter().map(ExecutionId::as_str),
            bundle.executions.iter().map(|v| v.execution_id.as_str()),
        )
        || !exact_inventory(
            inv.reviews.iter().map(ExecutionId::as_str),
            bundle.reviews.iter().map(|v| v.execution_id.as_str()),
        )
        || !exact_inventory(
            inv.sensors.iter().map(SensorId::as_str),
            bundle.sensors.iter().map(|v| v.sensor_id.as_str()),
        )
        || !exact_inventory(
            inv.knowledge.iter().map(String::as_str),
            knowledge.iter().map(String::as_str),
        )
        || bundle
            .reviews
            .iter()
            .filter(|review| review.status == policy_contracts::ReviewStatus::Approved)
            .count()
            < config.minimum_reviewers
        || raw
            .policy
            .required_sensor_ids
            .iter()
            .any(|id| !inv.sensors.contains(id))
        || raw
            .context
            .sensor_profile
            .sensors
            .iter()
            .any(|sensor| sensor.required && !inv.sensors.contains(&sensor.id))
    {
        return Err(
            "authenticated inventory is incomplete or missing required reviews/sensors".into(),
        );
    }
    let now = UtcTimestamp::try_from(std::time::SystemTime::now())?;
    let local = local_evidence(config, raw, &registry, &now)?;
    let local_ids: BTreeSet<_> = local
        .iter()
        .flat_map(|bundle| &bundle.executions)
        .map(|item| &item.execution_id)
        .collect();
    let local_sensors: BTreeSet<_> = local
        .iter()
        .flat_map(|bundle| &bundle.sensors)
        .map(|item| &item.sensor_id)
        .collect();
    let records: Vec<_> = bundle
        .modifications
        .iter()
        .map(|v| (&v.execution_id, &v.provider, &v.model, v.trust))
        .chain(
            bundle
                .executions
                .iter()
                .map(|v| (&v.execution_id, &v.provider, &v.model, v.trust)),
        )
        .chain(
            bundle
                .reviews
                .iter()
                .map(|v| (&v.execution_id, &v.provider, &v.model, v.trust)),
        )
        .collect();
    if !exact_inventory(
        records
            .iter()
            .filter(|v| !local_ids.contains(v.0))
            .map(|v| v.0.as_str()),
        signed.producers.iter().map(|v| v.execution_id.as_str()),
    ) || records.iter().any(|v| v.3 != TrustClass::LocalUntrusted)
        || bundle
            .sensors
            .iter()
            .any(|v| v.trust != TrustClass::LocalUntrusted)
        || bundle
            .knowledge
            .iter()
            .any(|v| v.trust != TrustClass::LocalUntrusted)
    {
        return Err(
            "each local producer record requires exactly one authenticated signature".into(),
        );
    }
    for (id, provider, model, _) in records.iter().copied().filter(|v| !local_ids.contains(v.0)) {
        let signature = signed
            .producers
            .iter()
            .find(|v| &v.execution_id == id)
            .ok_or("missing producer signature")?;
        let key = config
            .producer_keys
            .iter()
            .find(|v| v.key_id == signature.key_id)
            .ok_or("unknown producer key")?;
        if &key.provider != provider || &key.model != model {
            return Err("producer key does not authenticate claimed provider/model".into());
        }
        verify(
            &config.openssl,
            &key.public_key,
            &signature.signature,
            &producer_payload(raw, id)?,
        )?;
    }
    let mut evidence = bundle.clone();
    for item in &mut evidence.modifications {
        item.trust = TrustClass::TrustedRunnerAttested;
    }
    for item in &mut evidence.executions {
        item.trust = TrustClass::TrustedRunnerAttested;
    }
    for item in &mut evidence.reviews {
        item.trust = TrustClass::TrustedRunnerAttested;
    }
    for item in &mut evidence.sensors {
        item.trust = TrustClass::TrustedRunnerAttested;
    }
    for item in &mut evidence.knowledge {
        item.trust = TrustClass::TrustedRunnerAttested;
    }
    // The context-free provenance API intentionally remains CI-grade only.
    // Validate CI records under their original window, and local records under
    // the separately approved window above; never relabel local data as CI.
    let mut ci_evidence = evidence.clone();
    ci_evidence
        .executions
        .retain(|item| !local_ids.contains(&item.execution_id));
    ci_evidence
        .sensors
        .retain(|item| !local_sensors.contains(&item.sensor_id));
    if !local.is_empty() {
        ci_evidence.test_adequacy = None;
    }
    rehash_bundle(&mut ci_evidence)?;
    let provenance = policy_core::validate_provenance(
        EvaluationMode::Authoritative,
        &ci_evidence,
        &now,
        raw.policy.max_evidence_age_seconds,
    );
    if !provenance.is_valid() {
        return Err(format!(
            "authenticated provenance rejected: {:?}",
            provenance.reason_codes()
        )
        .into());
    }
    install_local(&mut evidence, local)?;
    rehash_bundle(&mut evidence)?;
    let mut context = raw.context.clone();
    if let Some(ready) = &mut context.agent_ready {
        if ready.trust != TrustClass::LocalUntrusted
            || ready.repository != binding.repository
            || ready.head_sha != binding.head_sha
            || ready.pull_request_number != binding.pull_request_number
            || ready.work_item_id != manifest.work_item_id
        {
            return Err(
                "agent-ready observation does not match authenticated head/work item".into(),
            );
        }
        ready.trust = TrustClass::TrustedRunnerAttested;
    }
    let mut identity = serde_json::to_value(binding)?;
    identity["schema_version"] = 2.into();
    identity["evidence_digest"] = serde_json::to_value(policy_core::canonical_sha256(&evidence)?)?;
    identity["context_digest"] = serde_json::to_value(policy_core::canonical_sha256(&context)?)?;
    identity["work_item_digest"] =
        serde_json::to_value(policy_core::canonical_sha256(&raw.work_item)?)?;
    // A fresh private directory is mandatory. A receipt is written last; a
    // partial failed write cannot become a consumable identity directory.
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(output)?;
    for (name, bytes) in [
        ("policy.json", canonical(&raw.policy)?),
        ("evidence.json", canonical(&evidence)?),
        ("context.json", canonical(&context)?),
        ("work-item.json", canonical(&raw.work_item)?),
        ("authentication.json", canonical(signed)?),
        ("identity.json", canonical(&identity)?),
    ] {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))?;
        file.write_all(&bytes)?;
    }
    Ok(())
}

/// Stable inventory spelling, independent of the `Debug` representation that
/// originally produced these keys.
fn knowledge_kind(kind: KnowledgeKind) -> &'static str {
    match kind {
        KnowledgeKind::LatClaim => "LatClaim",
        KnowledgeKind::CodeGraphIndex => "CodeGraphIndex",
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn local_freshness_has_inclusive_subsecond_boundaries_and_rejects_future_times() {
        let time: UtcTimestamp = "2026-11-16T01:00:00.123456789Z".parse().unwrap();
        let boundary: UtcTimestamp = "2026-11-16T01:01:00.123456789Z".parse().unwrap();
        let expired: UtcTimestamp = "2026-11-16T01:01:00.123456790Z".parse().unwrap();
        let future: UtcTimestamp = "2026-11-16T01:00:00.123456790Z".parse().unwrap();
        assert!(local_time_is_fresh(&time, &boundary, 60));
        assert!(!local_time_is_fresh(&time, &expired, 60));
        assert!(!local_time_is_fresh(&future, &time, 60));
    }

    #[test]
    fn local_registry_accepts_both_hardware_algorithms_and_rejects_ambiguous_entries() {
        let temp = tempfile::tempdir().unwrap();
        let public_key = temp.path().join("runner.pub");
        let entry = LocalRunnerKey {
            runner_id: "runner-1".into(),
            public_key: public_key.clone(),
        };
        for algorithm in HARDWARE_KEYS {
            // Parse-level key policy only. Cryptographic fixtures independently
            // exercise complete FIDO-format keys through the OpenSSH executable.
            let mut blob = (algorithm.len() as u32).to_be_bytes().to_vec();
            blob.extend(algorithm.as_bytes());
            let line = format!("{algorithm} {}\n", STANDARD.encode(&blob));
            fs::write(&public_key, &line).unwrap();
            assert_eq!(local_key(&entry).unwrap().blob, blob);
            fs::write(&public_key, format!("runner-1 {line}")).unwrap();
            local_key(&entry).unwrap();
            fs::write(&public_key, format!("\n# registered runner\n{line}\n")).unwrap();
            local_key(&entry).unwrap();
            for runner_id in ["", "runner*", "runner,other", "runner space"] {
                let invalid = LocalRunnerKey {
                    runner_id: runner_id.into(),
                    public_key: public_key.clone(),
                };
                assert!(local_key(&invalid).is_err());
            }
            let relative = LocalRunnerKey {
                runner_id: entry.runner_id.clone(),
                public_key: PathBuf::from("runner.pub"),
            };
            assert!(local_key(&relative).is_err());
            for runner_id in ["ssh-runner", "sk-runner", "ecdsa-runner", algorithm] {
                let prefixed = LocalRunnerKey {
                    runner_id: runner_id.into(),
                    public_key: public_key.clone(),
                };
                fs::write(&public_key, format!("{runner_id} {line}")).unwrap();
                local_key(&prefixed).unwrap();
            }
            for prefix in ["runner-2 ", "* ", "runner-1 cert-authority "] {
                fs::write(&public_key, format!("{prefix}{line}")).unwrap();
                assert!(local_key(&entry).is_err());
            }
            fs::write(&public_key, format!("{line}{line}")).unwrap();
            assert!(local_key(&entry).is_err());
        }
    }

    #[test]
    fn local_signature_size_limit_accepts_the_boundary_and_rejects_one_more_byte() {
        fn string(value: &[u8]) -> Vec<u8> {
            let mut result = (value.len() as u32).to_be_bytes().to_vec();
            result.extend(value);
            result
        }
        // Parser-only fixture: the external verifier separately authenticates
        // key material and cryptography, so no signing device is needed here.
        let algorithm = HARDWARE_KEYS[0].as_bytes();
        let key = string(algorithm);
        let mut inner = string(algorithm);
        inner.extend(string(b"parser fixture"));
        inner.push(1);
        inner.extend(0u32.to_be_bytes());
        let mut bytes = b"SSHSIG".to_vec();
        bytes.extend(1u32.to_be_bytes());
        bytes.extend(string(&key));
        bytes.extend(string(LOCAL_DOMAIN.as_bytes()));
        bytes.extend(string(b""));
        bytes.extend(string(b"sha512"));
        bytes.extend(string(&inner));
        let signature = format!(
            "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----",
            STANDARD.encode(bytes)
        );
        let at_limit = format!("{}{}", " ".repeat(16384 - signature.len()), signature);
        local_signature_presence(&at_limit, &key).unwrap();
        assert!(local_signature_presence(&format!(" {at_limit}"), &key).is_err());
    }

    #[test]
    fn knowledge_inventory_keys_keep_their_signed_spelling() {
        assert_eq!(knowledge_kind(KnowledgeKind::LatClaim), "LatClaim");
        assert_eq!(
            knowledge_kind(KnowledgeKind::CodeGraphIndex),
            "CodeGraphIndex"
        );
    }

    #[cfg(unix)]
    #[test]
    fn named_pipe_input_is_rejected_without_waiting_for_a_writer() {
        let temp = tempfile::tempdir().unwrap();
        let fifo = temp.path().join("observation.fifo");
        assert!(Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success());
        let (sender, receiver) = std::sync::mpsc::channel();
        let input = fifo.clone();
        let reader = std::thread::spawn(move || {
            sender
                .send(read_file(&input).map_err(|e| e.to_string()))
                .unwrap();
        });
        let result = receiver.recv_timeout(Duration::from_millis(500));
        if result.is_err() {
            // Release a regressed blocking open so the regression fails promptly,
            // instead of converting a caught mutation into a suite timeout.
            let _writer = rustix::fs::open(
                &fifo,
                rustix::fs::OFlags::WRONLY.union(rustix::fs::OFlags::NONBLOCK),
                rustix::fs::Mode::empty(),
            )
            .unwrap();
            reader.join().unwrap();
        } else {
            reader.join().unwrap();
        }
        assert!(result
            .expect("input open waited for a FIFO writer")
            .unwrap_err()
            .contains("regular file"));
    }

    #[test]
    fn input_bound_rejects_oversized_valid_json_and_keeps_boundary_content() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("observation.json");
        let mut bytes = b"{}".to_vec();
        bytes.resize(8_388_608, b' ');
        fs::write(&path, &bytes).unwrap();
        assert_eq!(read_file(&path).unwrap().len(), 8_388_608);
        assert_eq!(
            read_json::<serde_json::Value>(&path).unwrap(),
            serde_json::json!({})
        );
        for added in [1, 2, 4096] {
            let mut oversized = bytes.clone();
            oversized.resize(8_388_608 + added, b' ');
            fs::write(&path, oversized).unwrap();
            assert!(read_json::<serde_json::Value>(&path).is_err());
        }
        assert!(read_file(temp.path()).is_err());
        #[cfg(unix)]
        {
            let link = temp.path().join("link.json");
            std::os::unix::fs::symlink(&path, &link).unwrap();
            assert!(read_file(&link).is_err());
        }
    }

    #[test]
    fn private_signature_scratch_is_removed_and_invalid_encodings_never_spawn_crypto() {
        let scratch = Scratch::new().unwrap();
        let path = scratch.0.clone();
        fs::write(path.join("payload"), b"private observation").unwrap();
        drop(scratch);
        assert!(!path.exists());
        for signature in [String::new(), "0".into(), "00".repeat(8193), "gg".into()] {
            assert!(verify(
                Path::new("/no-such-openssl"),
                Path::new("/no-such-key"),
                &signature,
                b"payload"
            )
            .is_err());
        }
        let temp = tempfile::tempdir().unwrap();
        let key = temp.path().join("invalid-key");
        fs::write(&key, "invalid key").unwrap();
        // A syntactically bounded signature still requires cryptographic authentication.
        let error = verify(
            Path::new("/usr/bin/false"),
            &key,
            &"00".repeat(8192),
            b"payload",
        )
        .unwrap_err();
        assert!(error.to_string().contains("authentication failed"));
        let error = verify(
            Path::new("/usr/bin/false"),
            &key,
            &"00".repeat(8193),
            b"payload",
        )
        .unwrap_err();
        assert!(error.to_string().contains("signature encoding"));
    }
}
