use clap::{ArgGroup, Parser};
use policy_contracts::{CommitSha, ExecutionId, UtcTimestamp};
use policy_core::canonical_json;
use policy_local_attest::{GateAttestation, GateStatement, SigningKey, DEFAULT_SSH_KEYGEN};
use policy_native_importer::{import_run, raw_digest, ExpectedIdentity};
use policy_native_verifier::{contracts::*, evaluate_scope};
use rustix::termios::{tcflush, QueueSelector};
use std::{
    error::Error,
    ffi::OsString,
    fs,
    io::{self, IsTerminal, Read, Seek, Write},
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    time::SystemTime,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ENVELOPE: &str = "attestation.json";
const MAX_SIGNATURE_BYTES: usize = 16 * 1024;

#[derive(Parser)]
#[command(
    about = "Run one local check command and ask the owner to sign its satisfied scope",
    group(ArgGroup::new("signing_mode").required(true).args(["signer", "signing_key"]))
)]
struct Args {
    /// Git worktree root and runner working directory.
    #[arg(long)]
    repo: PathBuf,
    /// Git commit or ref to bind as the comparison base.
    #[arg(long)]
    base: String,
    /// Independently prepared ExpectedScopeV1. Its evaluation clock is set after the run.
    #[arg(long)]
    expected: PathBuf,
    /// Fresh NativeRunV1 manifest path written by the runner, outside the artifacts.
    #[arg(long)]
    run: PathBuf,
    /// Directory containing the runner's raw artifacts.
    #[arg(long)]
    artifacts: PathBuf,
    /// Fresh output directory for the import record, observations and signed envelope.
    #[arg(long)]
    output: PathBuf,
    #[arg(long, value_parser = |value: &str| ExecutionId::parse(value))]
    run_id: ExecutionId,
    #[arg(long, value_parser = |value: &str| RecordId::parse(value))]
    scope_id: RecordId,
    #[arg(long, value_parser = |value: &str| RecordId::parse(value))]
    runner_id: RecordId,
    /// Controller mode: absolute path to an owner-selected SSHSIG signer.
    #[arg(long)]
    signer: Option<PathBuf>,
    /// An argument to the signer, in order. Use --signer-arg=VALUE for leading hyphens.
    #[arg(long, requires = "signer")]
    signer_arg: Vec<OsString>,
    /// Gate mode: OpenSSH hardware key handle requiring a touch.
    #[arg(long, requires_all = ["pr", "gate_policy"])]
    signing_key: Option<PathBuf>,
    /// Pull request number to bind into the gate signature.
    #[arg(long, requires = "signing_key", value_parser = clap::value_parser!(u64).range(1..))]
    pr: Option<u64>,
    /// Repository-relative gate policy path. The attester reads it from --base.
    #[arg(long, requires = "signing_key")]
    gate_policy: Option<String>,
    /// Absolute path to a trusted OpenSSH ssh-keygen with FIDO support.
    #[arg(long, requires = "signing_key")]
    ssh_keygen: Option<PathBuf>,
    /// One runner command and its arguments, following --.
    #[arg(last = true, required = true, num_args = 1..)]
    runner: Vec<OsString>,
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Signed,
    Unconfirmed,
}

pub(super) fn main() -> ExitCode {
    match attest(Args::parse(), confirm_on_terminal) {
        Ok(Outcome::Signed) => ExitCode::SUCCESS,
        Ok(Outcome::Unconfirmed) => {
            eprintln!("policy-local-attest: owner confirmation was not received");
            ExitCode::from(3)
        }
        Err(error) => {
            eprintln!("policy-local-attest: {error}");
            ExitCode::from(2)
        }
    }
}

fn attest(args: Args, confirm: impl FnOnce(&str) -> bool) -> Result<Outcome> {
    require_absent(&args.output)?;
    require_absent(&args.run)?;
    if args
        .signer
        .as_ref()
        .is_some_and(|signer| !signer.is_absolute())
    {
        return Err("signer must be an absolute executable path".into());
    }
    let mut expected: ExpectedScopeV1 = serde_json::from_slice(&fs::read(&args.expected)?)?;
    if expected
        .requirements
        .observations
        .as_slice()
        .iter()
        .all(|rule| rule.tests.is_none())
    {
        return Err("local attestation of process-only scopes is not supported yet".into());
    }
    let before = GitSnapshot::read(&args.repo)?;
    let base = revision(&args.repo, &format!("{}^{{commit}}", args.base))?;
    let gate = args
        .signing_key
        .as_ref()
        .map(|path| {
            let key = SigningKey::read(path)?;
            let policy = args
                .gate_policy
                .as_deref()
                .ok_or("gate mode requires a policy path")?;
            if policy
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
                || policy.contains(['\\', ':'])
                || policy.chars().any(char::is_control)
            {
                return Err("gate policy must be a portable repository-relative path".into());
            }
            let policy_bytes = git(
                &args.repo,
                &["cat-file", "blob", &format!("{base}:{policy}")],
            )?;
            let pr = args.pr.ok_or("gate mode requires the PR number")?;
            Ok::<_, Box<dyn Error>>((key, pr, raw_digest(&policy_bytes).sha256))
        })
        .transpose()?;
    if before.head != expected.subject.head {
        return Err("expected scope head does not match the Git worktree".into());
    }

    let started_at = now();
    let status = Command::new(&args.runner[0])
        .args(&args.runner[1..])
        .current_dir(&args.repo)
        .stdin(Stdio::null())
        .status()?;
    let completed_at = now();
    if !status.success() {
        return Err(format!("runner failed ({status})").into());
    }
    if GitSnapshot::read(&args.repo)? != before {
        return Err("Git head or tree changed during the run".into());
    }
    if completed_at <= started_at {
        return Err("runner completion must follow its start".into());
    }

    let run: NativeRunV1 = serde_json::from_slice(&fs::read(&args.run)?)?;
    if run.observations.iter().any(|observation| {
        [
            &observation.started_at,
            &observation.completed_at,
            &observation.observed_at,
        ]
        .into_iter()
        .any(|time| time < &started_at || time > &completed_at)
    }) {
        return Err("observation time is outside the runner interval".into());
    }
    let identity = ExpectedIdentity {
        run_id: args.run_id,
        scope_id: args.scope_id,
    };
    expected.bindings.evaluated_at = completed_at.clone();
    let record = import_run(&args.artifacts, &args.output, &expected, &identity, &run)?;
    let imported = record
        .verified_observations()
        .ok_or("native import rejected the run; see import.json")?;
    let result = evaluate_scope(&expected, imported)?.result;
    if result.status != ScopeStatus::Satisfied {
        return Err(format!(
            "scope result is {:?}; signing requires Satisfied",
            result.status
        )
        .into());
    }
    let statement = LocalAttestationV1 {
        schema_version: VersionV1::default(),
        domain: LocalAttestationDomain::LocalRunnerV1,
        subject: expected.subject.clone(),
        head_tree: before.tree,
        base,
        run_id: identity.run_id,
        scope_id: identity.scope_id,
        expected_scope_digest: expected.digest()?,
        scope_template_digest: expected.template_digest()?,
        native_run_digest: imported.run.digest()?,
        observations_digest: canonical_local_digest(
            LocalDigestRole::ImportedObservations,
            imported,
        )?,
        result_digest: result.digest()?,
        runner: LocalRunnerV1 {
            runner_id: args.runner_id,
            os: RecordId::parse(std::env::consts::OS)?,
            arch: RecordId::parse(std::env::consts::ARCH)?,
            attestor_version: RecordId::parse(concat!(
                "policy-local-attest/",
                env!("CARGO_PKG_VERSION")
            ))?,
        },
        started_at,
        completed_at,
        attested_at: now(),
    };
    statement.validate()?;
    match gate {
        Some((key, pr, policy_sha256)) => {
            let statement = GateStatement {
                pr,
                policy_sha256,
                run: statement,
            };
            let mut handle = tempfile::NamedTempFile::new()?;
            let executable = args
                .ssh_keygen
                .as_deref()
                .unwrap_or_else(|| Path::new(DEFAULT_SSH_KEYGEN));
            let mut command = key.command(executable, &mut handle)?;
            let payload = canonical_json(&statement)?;
            let context = format!(
                "PR {}, base policy SHA-256 {}\n",
                statement.pr, statement.policy_sha256
            );
            let summary = signing_summary(&statement.run, &context, &payload);
            let Some(signature) = sign(&statement.run, &mut command, &payload, &summary, confirm)?
            else {
                return Ok(Outcome::Unconfirmed);
            };
            key.check_signature(signature.value.as_str())?;
            publish(
                &args.output,
                "gate-attestation.json",
                &GateAttestation {
                    statement,
                    signature,
                },
            )
        }
        None => {
            let mut command = Command::new(args.signer.ok_or("missing controller signer")?);
            command.args(args.signer_arg);
            let payload = canonical_json(&statement)?;
            let summary = signing_summary(&statement, "", &payload);
            let Some(signature) = sign(&statement, &mut command, &payload, &summary, confirm)?
            else {
                return Ok(Outcome::Unconfirmed);
            };
            let signed = SignedLocalAttestationV1 {
                statement,
                signature,
            };
            signed.validate()?;
            publish(&args.output, ENVELOPE, &signed)
        }
    }
}

fn signing_summary(statement: &LocalAttestationV1, context: &str, payload: &[u8]) -> String {
    format!(
        "Local scope Satisfied for {:?}\n{context}Head {} (tree {}), base {}\nRunner {:?}, run {:?}, scope {:?}\nScope template SHA-256 {}\nStatement SHA-256 (signing bytes) {}",
        statement.subject.repository.as_str(), statement.subject.head, statement.head_tree, statement.base,
        statement.runner.runner_id.as_str(), statement.run_id.as_str(), statement.scope_id.as_str(),
        statement.scope_template_digest.sha256,
        raw_digest(payload).sha256,
    )
}

fn sign(
    statement: &LocalAttestationV1,
    command: &mut Command,
    payload: &[u8],
    summary: &str,
    confirm: impl FnOnce(&str) -> bool,
) -> Result<Option<LocalSignatureV1>> {
    // A file-backed stdin cannot deadlock if the signer writes before reading.
    let mut input = tempfile::tempfile()?;
    input.write_all(payload)?;
    input.rewind()?;
    command
        .stdin(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    if !confirm(summary) {
        return Ok(None);
    }
    let mut child = command.spawn()?;
    let mut signature_bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .expect("piped signer stdout")
        .take((MAX_SIGNATURE_BYTES + 1) as u64)
        .read_to_end(&mut signature_bytes);
    if read.is_err() || signature_bytes.len() > MAX_SIGNATURE_BYTES {
        let _ = child.kill();
    }
    let status = child.wait()?;
    read?;
    if signature_bytes.len() > MAX_SIGNATURE_BYTES {
        return Err("signer output is too large".into());
    }
    if !status.success() {
        return Err(format!("signer failed ({status})").into());
    }
    let value = String::from_utf8(signature_bytes)?;
    let mut armor = value.trim().lines();
    if armor.next() != Some("-----BEGIN SSH SIGNATURE-----")
        || armor.next_back() != Some("-----END SSH SIGNATURE-----")
        || !armor.any(|line| !line.trim().is_empty())
    {
        return Err("signer did not return an armored SSHSIG".into());
    }
    Ok(Some(LocalSignatureV1 {
        format: LocalSignatureFormat::Sshsig,
        runner_id: statement.runner.runner_id.clone(),
        value: RecordId::parse(value)?,
    }))
}

fn publish(output: &Path, file: &str, signed: &impl serde::Serialize) -> Result<Outcome> {
    let mut temp = tempfile::NamedTempFile::new_in(output)?;
    serde_json::to_writer_pretty(&mut temp, signed)?;
    temp.write_all(b"\n")?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(output.join(file))?;
    Ok(Outcome::Signed)
}

fn confirm_on_terminal(summary: &str) -> bool {
    let result = (|| -> io::Result<bool> {
        let input = io::stdin();
        let mut display = io::stderr().lock();
        if !input.is_terminal() || !display.is_terminal() {
            return Ok(false);
        }
        // Only an answer entered after the owner is shown this statement counts.
        tcflush(&input, QueueSelector::IFlush)?;
        writeln!(display, "{summary}\nType yes to authorize signing:")?;
        display.flush()?;
        let mut line = String::new();
        input.read_line(&mut line)?;
        Ok(line.trim_end_matches(['\r', '\n']) == "yes")
    })();
    result.unwrap_or(false)
}

fn require_absent(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", path.display()),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(PartialEq, Eq)]
struct GitSnapshot {
    head: CommitSha,
    tree: CommitSha,
}

impl GitSnapshot {
    fn read(repo: &Path) -> Result<Self> {
        if !git(repo, &["rev-parse", "--show-prefix"])?
            .trim_ascii()
            .is_empty()
        {
            return Err("repo must be the Git worktree root".into());
        }
        let head = revision(repo, "HEAD^{commit}")?;
        let tree = revision(repo, &format!("{head}^{{tree}}"))?;
        let status = git(
            repo,
            &[
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=none",
            ],
        )?;
        let index = git(repo, &["ls-files", "-v", "-z"])?;
        if !status.is_empty()
            || index
                .split(|byte| *byte == 0)
                .any(|entry| !entry.is_empty() && !entry.starts_with(b"H "))
        {
            return Err("Git worktree is dirty".into());
        }
        Ok(Self { head, tree })
    }
}

fn revision(repo: &Path, expression: &str) -> Result<CommitSha> {
    let bytes = git(
        repo,
        &["rev-parse", "--verify", "--end-of-options", expression],
    )?;
    Ok(CommitSha::parse(std::str::from_utf8(&bytes)?.trim())?)
}

fn git(repo: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new("git")
        .args(["-c", "core.fsmonitor=false"])
        .args(args)
        .current_dir(repo)
        .env_clear()
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()?;
    if !out.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args[0],
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into());
    }
    Ok(out.stdout)
}

fn now() -> UtcTimestamp {
    UtcTimestamp::try_from(SystemTime::now()).expect("system UTC timestamp")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
