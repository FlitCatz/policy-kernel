//! Reference authentication boundary. Keys and expectations come from an owner
//! controlled configuration, never from the submitted observation. Signatures
//! authenticate an attester's statement; the attester must actually observe it.
use policy_contracts::*;
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
        .stdin(Stdio::null())
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
    if config.schema_version != 1
        || raw.domain != DOMAIN
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
        records.iter().map(|v| v.0.as_str()),
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
    for (id, provider, model, _) in records {
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
    let value = serde_json::to_value(&evidence)?;
    for child in &mut evidence.manifest.child_digests {
        child.digest = policy_core::canonical_sha256(&value[&child.path])?;
    }
    let now: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    let provenance = policy_core::validate_provenance(
        EvaluationMode::Authoritative,
        &evidence,
        &now.to_rfc3339().parse()?,
        raw.policy.max_evidence_age_seconds,
    );
    if !provenance.is_valid() {
        return Err(format!(
            "authenticated provenance rejected: {:?}",
            provenance.reason_codes()
        )
        .into());
    }
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
