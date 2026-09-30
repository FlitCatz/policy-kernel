#![forbid(unsafe_code)]

use clap::{Parser, Subcommand};
use policy_contracts::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::{
    env,
    error::Error,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[command(about = "Authoritative GitHub Actions policy evaluation")]
struct Cli {
    #[command(subcommand)]
    command: Mode,
}
#[derive(Subcommand)]
enum Mode {
    PreMerge(Options),
    PostMerge(Options),
}
#[derive(clap::Args)]
struct Options {
    #[arg(long, default_value = "authoritative")]
    mode: String,
    #[arg(long)]
    policy: Option<PathBuf>,
    #[arg(long)]
    evidence: Option<PathBuf>,
    #[arg(long)]
    trusted_observations: Option<PathBuf>,
    #[arg(long, value_parser = parse_sha)]
    expected_head: Option<CommitSha>,
    /// Additional caller assertion; git HEAD is always independently inspected.
    #[arg(long, value_parser = parse_sha)]
    checked_out_head: Option<CommitSha>,
    #[arg(long)]
    artifact_directory: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityReceipt {
    schema_version: u32,
    repository: RepositoryId,
    pull_request_number: PullRequestNumber,
    head_sha: CommitSha,
    workflow_run_id: WorkflowRunId,
    workflow_run_attempt: u64,
    workflow_trigger_head_sha: CommitSha,
    workflow_head_sha: CommitSha,
    workflow_id: u64,
    workflow_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow_source_sha: Option<CommitSha>,
    check_app_id: u64,
    check_app_slug: String,
    policy_digest: Sha256Digest,
    /// Digest of the entire typed evidence bundle, including its manifest.
    evidence_digest: Sha256Digest,
    context_digest: Sha256Digest,
    work_item_digest: Sha256Digest,
}

struct Runtime {
    receipt: IdentityReceipt,
    trusted: PathBuf,
    artifacts: PathBuf,
    policy: PolicySnapshot,
    now: UtcTimestamp,
}

fn main() {
    match run(Cli::parse()) {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("policy-ci: {error}");
            std::process::exit(2);
        }
    }
}
fn workflow_source_matches(receipt: &IdentityReceipt, workflow_ref: &str) -> bool {
    match receipt.schema_version {
        1 => receipt.workflow_ref.is_none() && receipt.workflow_source_sha.is_none(),
        2 => {
            receipt.workflow_ref.as_deref() == Some(workflow_ref)
                && receipt.workflow_source_sha.as_ref().is_some_and(|sha| {
                    env::var("GITHUB_WORKFLOW_SHA").as_deref() == Ok(sha.as_str())
                })
        }
        _ => false,
    }
}

fn run(cli: Cli) -> Result<bool> {
    let (options, post_merge) = match cli.command {
        Mode::PreMerge(options) => (options, false),
        Mode::PostMerge(options) => (options, true),
    };
    if options.mode != "authoritative" {
        return Err("policy-ci accepts only authoritative mode".into());
    }
    if let (Some(expected), Some(asserted)) = (&options.expected_head, &options.checked_out_head) {
        if expected != asserted {
            return Err("checked-out head does not match expected head".into());
        }
    }
    if env::var("GITHUB_ACTIONS").as_deref() != Ok("true") {
        return Err("authoritative evaluation requires GitHub Actions identity".into());
    }
    let expected = options
        .expected_head
        .as_ref()
        .ok_or("--expected-head is required")?;
    let checkout = Command::new("git")
        .args(["rev-parse", "--verify", "HEAD"])
        .output()?;
    if !checkout.status.success()
        || std::str::from_utf8(&checkout.stdout)?.trim() != expected.as_str()
    {
        return Err("checked-out head does not match expected head".into());
    }
    let workspace = PathBuf::from(required_env("GITHUB_WORKSPACE")?).canonicalize()?;
    let current = env::current_dir()?.canonicalize()?;
    if current != workspace {
        return Err("policy-ci must run from GITHUB_WORKSPACE".into());
    }
    let trusted = options
        .trusted_observations
        .as_ref()
        .ok_or("--trusted-observations is required")?
        .canonicalize()?;
    if trusted.starts_with(&workspace) || !trusted.is_dir() {
        return Err(
            "trusted observations must be a runner-controlled directory outside the checkout"
                .into(),
        );
    }
    let receipt: IdentityReceipt = read_trusted(&trusted, "identity.json")?;
    let repository = required_env("GITHUB_REPOSITORY")?;
    let run_id = required_env("GITHUB_RUN_ID")?;
    let attempt: u64 = required_env("GITHUB_RUN_ATTEMPT")?.parse()?;
    let trigger = required_env("GITHUB_SHA")?;
    let workflow_ref = required_env("GITHUB_WORKFLOW_REF")?;
    let expected_ref = format!("{}/{}@", receipt.repository, receipt.workflow_path);
    if !workflow_source_matches(&receipt, &workflow_ref)
        || receipt.repository.as_str() != repository
        || receipt.head_sha != *expected
        || receipt.workflow_run_id.as_str() != run_id
        || receipt.workflow_run_attempt != attempt
        || attempt == 0
        || receipt.workflow_trigger_head_sha.as_str() != trigger
        || !workflow_ref.starts_with(&expected_ref)
        || workflow_ref.len() == expected_ref.len()
        || receipt.workflow_id == 0
        || receipt.check_app_id == 0
        || receipt.check_app_slug.is_empty()
    {
        return Err("trusted identity does not match the current workflow/head".into());
    }
    let policy: PolicySnapshot = read_json(options.policy.as_ref().ok_or("--policy is required")?)?;
    if policy.policy_digest != receipt.policy_digest
        || policy_core::pre_merge_policy_digest(&policy)? != receipt.policy_digest
        || policy.required_check_name != "policy-kernel/pre-merge"
    {
        return Err("policy must match the trusted policy digest and dedicated check name".into());
    }
    let artifacts = options
        .artifact_directory
        .clone()
        .ok_or("--artifact-directory is required")?;
    fs::create_dir_all(&artifacts)?;
    let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    let runtime = Runtime {
        receipt,
        trusted,
        artifacts,
        policy,
        now: now.to_rfc3339().parse()?,
    };
    evaluate(options, runtime, post_merge)
}

fn required_env(name: &str) -> Result<String> {
    let value =
        env::var(name).map_err(|_| format!("missing GitHub Actions identity field {name}"))?;
    if value.is_empty() {
        return Err(format!("empty GitHub Actions identity field {name}").into());
    }
    Ok(value)
}
/// Shares the controller's bounded reader: no symlink, FIFO or oversized input can
/// hang or widen the authoritative job.
fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    policy_ci::controller::read_json(path)
}
fn read_trusted<T: DeserializeOwned>(directory: &Path, name: &str) -> Result<T> {
    let file = directory.join(name);
    if fs::symlink_metadata(&file)?.file_type().is_symlink()
        || file.canonicalize()?.parent() != Some(directory)
    {
        return Err(
            "trusted observation must be a regular file directly inside its directory".into(),
        );
    }
    read_json(&file)
}
fn canonical<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    Ok(policy_core::canonical_json(value)?)
}
fn write_artifact<T: Serialize>(directory: &Path, name: &str, value: &T) -> Result<()> {
    let path = directory.join(name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&canonical(value)?)?;
    Ok(())
}
fn outputs(
    runtime: &Runtime,
    status: &str,
    decision_digest: &Sha256Digest,
    check_id: Option<u64>,
) -> Result<()> {
    let mut text = format!("status={status}\ndecision_digest={decision_digest}\npolicy_digest={}\nhead_sha={}\nworkflow_run_id={}\nworkflow_run_attempt={}\n", runtime.policy.policy_digest, runtime.receipt.head_sha, runtime.receipt.workflow_run_id, runtime.receipt.workflow_run_attempt);
    if let Some(id) = check_id {
        text.push_str(&format!("check_run_id={id}\n"));
    }
    let output = required_env("GITHUB_OUTPUT")?;
    let summary = required_env("GITHUB_STEP_SUMMARY")?;
    fs::OpenOptions::new()
        .append(true)
        .open(output)?
        .write_all(text.as_bytes())?;
    fs::OpenOptions::new()
        .append(true)
        .open(summary)?
        .write_all(
            format!(
                "Policy kernel: **{status}**\n\nHead: `{}`\n\nDecision: `{decision_digest}`\n",
                runtime.receipt.head_sha
            )
            .as_bytes(),
        )?;
    print!("{text}");
    Ok(())
}

fn evaluate(options: Options, runtime: Runtime, post_merge: bool) -> Result<bool> {
    use github_adapter::{
        parse_merge, parse_pull_request, parse_trusted_check, publish_pre_merge_check,
        validate_workflow, CheckExpectation, GhCliApi, GitHubApi, PublicationReceipt,
        WorkflowExpectation,
    };
    let api = GhCliApi::new("gh", Duration::from_secs(30), 8 * 1024 * 1024);
    let receipt = &runtime.receipt;
    let (owner, repo) = receipt
        .repository
        .as_str()
        .split_once('/')
        .ok_or("invalid repository")?;
    let workflow_raw = api.workflow_run(owner, repo, &receipt.workflow_run_id)?;
    let workflow = validate_workflow(
        &workflow_raw,
        &WorkflowExpectation {
            repository: receipt.repository.clone(),
            workflow_run_id: receipt.workflow_run_id.clone(),
            workflow_run_attempt: receipt.workflow_run_attempt,
            workflow_id: receipt.workflow_id,
            workflow_path: receipt.workflow_path.clone(),
            workflow_head_sha: receipt.workflow_head_sha.clone(),
            check_app_id: receipt.check_app_id,
            check_app_slug: receipt.check_app_slug.clone(),
        },
    )?;
    let pr_raw = api.pull_request(owner, repo, receipt.pull_request_number.get())?;
    let mut pr = parse_pull_request(&pr_raw, &receipt.repository, receipt.pull_request_number)?;
    if pr.head_sha != receipt.head_sha {
        return Err("GitHub PR head does not match checked-out head".into());
    }
    // Freshness is the time this trusted boundary fetched the observation, not
    // the PR's last metadata modification time.
    pr.observed_at = runtime.now.clone();
    if !post_merge {
        let evidence: EvidenceBundle =
            read_json(options.evidence.as_ref().ok_or("--evidence is required")?)?;
        if policy_core::canonical_sha256(&evidence)? != receipt.evidence_digest
            || evidence.manifest.workflow_run_id != receipt.workflow_run_id
        {
            return Err("evidence does not match trusted current-run receipt".into());
        }
        let context: PreMergeContext = read_trusted(&runtime.trusted, "context.json")?;
        if context.workflow_run_id != receipt.workflow_run_id {
            return Err("context workflow run mismatch".into());
        }
        let work_item: WorkItemObservation = read_trusted(&runtime.trusted, "work-item.json")?;
        if policy_core::canonical_sha256(&context)? != receipt.context_digest
            || policy_core::canonical_sha256(&work_item)? != receipt.work_item_digest
        {
            return Err("trusted observation digest mismatch".into());
        }
        let input = PreMergeEvaluationInput {
            schema_version: SCHEMA_VERSION,
            mode: EvaluationMode::Authoritative,
            evaluation_time: runtime.now.clone(),
            policy: runtime.policy.clone(),
            work_item,
            pull_request: pr,
            evidence,
            context: Some(context),
        };
        let decision = policy_core::evaluate_pre_merge(&input);
        write_artifact(&runtime.artifacts, "pre-merge-input.json", &input)?;
        write_artifact(&runtime.artifacts, "decision.json", &decision)?;
        let published = publish_pre_merge_check(&api, &decision, &workflow, runtime.now.clone())?;
        write_artifact(
            &runtime.artifacts,
            "publication-receipt.json",
            &published.receipt,
        )?;
        write_artifact(&runtime.artifacts, "check.json", &published.observation)?;
        let status = match decision.status {
            PreMergeStatus::ReadyToMerge => "READY_TO_MERGE",
            PreMergeStatus::Blocked => "BLOCKED",
            PreMergeStatus::Inconclusive => "INCONCLUSIVE",
        };
        outputs(
            &runtime,
            status,
            &decision.decision_digest,
            Some(published.check_run_id),
        )?;
        Ok(decision.status == PreMergeStatus::ReadyToMerge)
    } else {
        let publication: PublicationReceipt =
            read_trusted(&runtime.trusted, "publication-receipt.json")?;
        let original: PreMergeEvaluationInput =
            read_trusted(&runtime.trusted, "pre-merge-input.json")?;
        let pre_merge = policy_core::evaluate_pre_merge(&original);
        if original.policy != runtime.policy
            || pre_merge.binding.repository != receipt.repository
            || pre_merge.binding.head_sha != receipt.head_sha
            || pre_merge.binding.pull_request_number != receipt.pull_request_number
            || original
                .context
                .as_ref()
                .map(|context| &context.workflow_run_id)
                != Some(&publication.workflow_run_id)
        {
            return Err("original authorization does not bind current policy, PR, head and published workflow".into());
        }
        let raw_check = api.check_run(owner, repo, publication.check_run_id)?;
        let raw_original_workflow = api.workflow_run(owner, repo, &publication.workflow_run_id)?;
        let check = parse_trusted_check(
            &raw_check,
            &raw_original_workflow,
            &publication,
            &CheckExpectation {
                repository: receipt.repository.clone(),
                pull_request_number: receipt.pull_request_number,
                head_sha: receipt.head_sha.clone(),
                check_name: runtime.policy.required_check_name.clone(),
                decision_digest: pre_merge.decision_digest.clone(),
                policy_digest: runtime.policy.policy_digest.clone(),
            },
        )?;
        let merge_receipt: serde_json::Value =
            read_trusted(&runtime.trusted, "merge-receipt.json")?;
        let merge = parse_merge(
            &pr_raw,
            &canonical(&merge_receipt)?,
            &receipt.repository,
            receipt.pull_request_number,
        )?;
        let target = merge
            .target_branch
            .as_deref()
            .ok_or("merge receipt lacks its target branch")?;
        github_adapter::validate_merge_protection(&merge, &api.branch(owner, repo, target)?)?;
        let input = PostMergeEvaluationInput {
            schema_version: SCHEMA_VERSION,
            evaluation_time: runtime.now.clone(),
            policy: runtime.policy.clone(),
            pre_merge: pre_merge.try_into()?,
            check: Some(check.observation),
            merge,
        };
        let decision = policy_core::evaluate_post_merge(&input);
        write_artifact(&runtime.artifacts, "post-merge-input.json", &input)?;
        write_artifact(&runtime.artifacts, "decision.json", &decision)?;
        let status = match decision.status {
            PostMergeStatus::Shipped => "SHIPPED",
            PostMergeStatus::Blocked => "BLOCKED",
            PostMergeStatus::Inconclusive => "INCONCLUSIVE",
        };
        outputs(
            &runtime,
            status,
            &decision.decision_digest,
            Some(check.check_run_id),
        )?;
        Ok(decision.status == PostMergeStatus::Shipped)
    }
}

fn parse_sha(value: &str) -> std::result::Result<CommitSha, IdentityError> {
    CommitSha::parse(value)
}
