use crate::{binding_error, AdapterError, GitHubApi, CHECK_NAME, MAX_CHECK_BINDING_BYTES};
use policy_contracts::{
    AgentReadyObservation, CheckConclusion, CommitSha, EvaluationMode, GitHubCheckObservation,
    GitHubMergeObservation, GitHubPullRequestObservation, MergeActorType, PreMergeDecision,
    PreMergeStatus, PullRequestNumber, PullRequestState, RepositoryId, Sha256Digest, TrustClass,
    UtcTimestamp, WorkItemId, WorkItemObservation, WorkflowRunId, SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckBinding {
    pub completed_at: UtcTimestamp,
    pub decision_digest: Sha256Digest,
    pub head_sha: CommitSha,
    pub policy_digest: Sha256Digest,
    pub pull_request_number: PullRequestNumber,
    pub repository: RepositoryId,
    pub schema_version: u32,
    pub workflow_run_attempt: u64,
    pub workflow_run_id: WorkflowRunId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckExpectation {
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub check_name: String,
    pub decision_digest: Sha256Digest,
    pub policy_digest: Sha256Digest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationReceipt {
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub check_name: String,
    pub check_run_id: u64,
    pub check_suite_id: u64,
    pub check_app_id: u64,
    pub check_app_slug: String,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_run_attempt: u64,
    pub workflow_id: u64,
    pub workflow_path: String,
    pub workflow_head_sha: CommitSha,
    pub decision_digest: Sha256Digest,
    pub policy_digest: Sha256Digest,
    pub published_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustedCheck {
    pub observation: GitHubCheckObservation,
    pub check_run_id: u64,
    pub workflow_run_attempt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowExpectation {
    pub repository: RepositoryId,
    pub workflow_run_id: WorkflowRunId,
    pub workflow_run_attempt: u64,
    pub workflow_id: u64,
    pub workflow_path: String,
    pub workflow_head_sha: CommitSha,
    pub check_app_id: u64,
    pub check_app_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowIdentity {
    repository: RepositoryId,
    workflow_run_id: WorkflowRunId,
    workflow_run_attempt: u64,
    workflow_id: u64,
    workflow_path: String,
    workflow_head_sha: CommitSha,
    check_app_id: u64,
    check_app_slug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedCheck {
    pub observation: GitHubCheckObservation,
    pub receipt: PublicationReceipt,
    pub check_run_id: u64,
    pub workflow_run_attempt: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabelReceipt {
    pub schema_version: u32,
    pub repository: RepositoryId,
    pub work_item_id: WorkItemId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub event_id: u64,
    pub label: String,
    pub actor: String,
    pub observed_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReadyExpectation {
    pub repository: RepositoryId,
    pub work_item_id: WorkItemId,
    pub pull_request_number: PullRequestNumber,
    pub head_sha: CommitSha,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkItemExpectation {
    pub repository: RepositoryId,
    pub issue_number: u64,
    pub work_item_id: WorkItemId,
}

pub fn serialize_check_binding(binding: &CheckBinding) -> Result<Vec<u8>, AdapterError> {
    ensure_schema(binding.schema_version)?;
    let bytes = serde_json::to_vec(binding)?;
    if bytes.len() > MAX_CHECK_BINDING_BYTES {
        return Err(AdapterError::PayloadTooLarge {
            actual: bytes.len(),
            limit: MAX_CHECK_BINDING_BYTES,
        });
    }
    Ok(bytes)
}

pub fn parse_pull_request(
    raw: &[u8],
    expected_repository: &RepositoryId,
    expected_number: PullRequestNumber,
) -> Result<GitHubPullRequestObservation, AdapterError> {
    let pull: RawPullRequest = serde_json::from_slice(raw)?;
    ensure(
        pull.number == expected_number.get(),
        "pull request number did not match",
    )?;
    ensure(
        pull.base.repo.full_name == expected_repository.as_str(),
        "pull request repository did not match",
    )?;
    let state = if pull.merged_at.is_some() {
        PullRequestState::Merged
    } else {
        match pull.state.as_str() {
            "open" => PullRequestState::Open,
            "closed" => PullRequestState::Closed,
            _ => return Err(binding_error("unknown pull request state")),
        }
    };
    Ok(GitHubPullRequestObservation {
        schema_version: SCHEMA_VERSION,
        repository: expected_repository.clone(),
        pull_request_number: expected_number,
        base_sha: CommitSha::parse(pull.base.sha)?,
        head_sha: CommitSha::parse(pull.head.sha)?,
        state,
        is_draft: pull.draft,
        observed_at: pull.updated_at,
    })
}

pub fn parse_merge(
    pull_request_raw: &[u8],
    receipt_raw: &[u8],
    expected_repository: &RepositoryId,
    expected_number: PullRequestNumber,
) -> Result<GitHubMergeObservation, AdapterError> {
    let pull: RawPullRequest = serde_json::from_slice(pull_request_raw)?;
    let receipt: GitHubMergeObservation = serde_json::from_slice(receipt_raw)?;
    ensure_schema(receipt.schema_version)?;
    ensure(
        pull.number == expected_number.get(),
        "merge PR number mismatch",
    )?;
    ensure(
        pull.base.repo.full_name == expected_repository.as_str(),
        "merge repository mismatch",
    )?;
    ensure(pull.state == "closed", "pull request was not closed")?;
    let merged_at = pull
        .merged_at
        .ok_or_else(|| binding_error("pull request was not merged"))?;
    let merge_commit_sha = CommitSha::parse(
        pull.merge_commit_sha
            .ok_or_else(|| binding_error("merged pull request lacked merge commit SHA"))?,
    )?;
    let head_sha = CommitSha::parse(pull.head.sha)?;
    let merged_by = pull
        .merged_by
        .ok_or_else(|| binding_error("merged pull request lacked merger"))?;
    let actor_type = match merged_by.kind.as_str() {
        "User" => MergeActorType::Human,
        "Bot" => MergeActorType::Bot,
        _ => return Err(binding_error("unknown GitHub merger actor type")),
    };

    ensure(
        receipt.repository == *expected_repository
            && receipt.pull_request_number == expected_number
            && receipt.head_sha == head_sha
            && receipt.merge_commit_sha == merge_commit_sha
            && receipt.target_branch.as_deref() == Some(pull.base.reference.as_str())
            && receipt.merged_by == merged_by.login
            && receipt.actor_type == Some(actor_type)
            && receipt.merged_at == merged_at
            && receipt.via_merge_queue.is_some(),
        "merge receipt disagreed with GitHub API observation",
    )?;
    Ok(receipt)
}

/// Cross-checks the trusted receipt's protection claim against GitHub's branch
/// observation (`GET /repos/{owner}/{repo}/branches/{branch}`, readable without
/// admin rights). The observation is taken at evaluation time, not at merge time.
pub fn validate_merge_protection(
    merge: &GitHubMergeObservation,
    branch_raw: &[u8],
) -> Result<(), AdapterError> {
    let branch: RawBranch = serde_json::from_slice(branch_raw)?;
    let target = merge
        .target_branch
        .as_deref()
        .ok_or_else(|| binding_error("merge receipt lacks its target branch"))?;
    ensure(
        branch.name == target,
        "branch observation does not match the merge target",
    )?;
    ensure(
        branch.protected == merge.protected,
        "merge receipt protection disagreed with GitHub branch observation",
    )
}

pub fn validate_workflow(
    raw: &[u8],
    expected: &WorkflowExpectation,
) -> Result<WorkflowIdentity, AdapterError> {
    let workflow: RawWorkflowRun = serde_json::from_slice(raw)?;
    ensure(
        workflow.id.to_string() == expected.workflow_run_id.as_str()
            && workflow.run_attempt == expected.workflow_run_attempt
            && workflow.workflow_id == expected.workflow_id
            && workflow.path == expected.workflow_path
            && workflow.head_sha == expected.workflow_head_sha.as_str()
            && workflow.repository.full_name == expected.repository.as_str(),
        "workflow run did not match trusted expectation",
    )?;
    ensure(
        matches!(
            workflow.status.as_str(),
            "queued" | "in_progress" | "completed"
        ),
        "workflow run had unknown status",
    )?;
    ensure(
        expected.workflow_run_attempt > 0
            && expected.workflow_id > 0
            && expected.check_app_id > 0
            && !expected.workflow_path.is_empty()
            && !expected.check_app_slug.is_empty(),
        "trusted workflow expectation was incomplete",
    )?;
    Ok(WorkflowIdentity {
        repository: expected.repository.clone(),
        workflow_run_id: expected.workflow_run_id.clone(),
        workflow_run_attempt: expected.workflow_run_attempt,
        workflow_id: expected.workflow_id,
        workflow_path: expected.workflow_path.clone(),
        workflow_head_sha: expected.workflow_head_sha.clone(),
        check_app_id: expected.check_app_id,
        check_app_slug: expected.check_app_slug.clone(),
    })
}

/// Normalize a fetched check using a receipt retained by trusted runner storage.
/// The receipt is an external trust input; deserializing contributor JSON does
/// not establish its provenance.
pub fn parse_trusted_check(
    check_raw: &[u8],
    workflow_raw: &[u8],
    receipt: &PublicationReceipt,
    expected: &CheckExpectation,
) -> Result<TrustedCheck, AdapterError> {
    ensure_schema(receipt.schema_version)?;
    ensure_receipt_expectation(receipt, expected)?;
    let workflow_expected = WorkflowExpectation {
        repository: receipt.repository.clone(),
        workflow_run_id: receipt.workflow_run_id.clone(),
        workflow_run_attempt: receipt.workflow_run_attempt,
        workflow_id: receipt.workflow_id,
        workflow_path: receipt.workflow_path.clone(),
        workflow_head_sha: receipt.workflow_head_sha.clone(),
        check_app_id: receipt.check_app_id,
        check_app_slug: receipt.check_app_slug.clone(),
    };
    let _workflow = validate_workflow(workflow_raw, &workflow_expected)?;
    let workflow: RawWorkflowRun = serde_json::from_slice(workflow_raw)?;
    ensure(
        workflow.status == "completed",
        "workflow run was not completed",
    )?;

    let check: RawCheckRun = serde_json::from_slice(check_raw)?;
    let binding = validated_check_binding(&check, expected)?;
    ensure(
        check.id == receipt.check_run_id
            && check.check_suite.id == receipt.check_suite_id
            && check.app.id == receipt.check_app_id
            && check.app.slug == receipt.check_app_slug
            && binding.workflow_run_id == receipt.workflow_run_id
            && binding.workflow_run_attempt == receipt.workflow_run_attempt
            && binding.completed_at.as_datetime() <= receipt.published_at.as_datetime(),
        "check run did not match trusted publication receipt",
    )?;
    let mut observation = check_observation(&check, &binding, expected)?;
    // GitHub does not return a server-side creation timestamp for a Check Run.
    // The trusted runner's post-response timestamp is the conservative point at
    // which publication is known to have happened.
    observation.completed_at = receipt.published_at.clone();
    Ok(TrustedCheck {
        observation,
        check_run_id: check.id,
        workflow_run_attempt: receipt.workflow_run_attempt,
    })
}

pub fn publish_pre_merge_check<A: GitHubApi>(
    api: &A,
    decision: &PreMergeDecision,
    workflow: &WorkflowIdentity,
    completed_at: UtcTimestamp,
) -> Result<PublishedCheck, AdapterError> {
    ensure_schema(decision.schema_version)?;
    ensure(
        decision.mode == EvaluationMode::Authoritative && !decision.advisory,
        "advisory decision cannot be published as an authoritative check",
    )?;
    ensure(
        decision.binding.repository == workflow.repository,
        "decision repository did not match workflow repository",
    )?;
    // GitHub's Check Run timestamp format is whole-second UTC. Use the same
    // representation in the posted fields and the canonical embedded binding.
    let completed_at = UtcTimestamp::parse(
        &completed_at
            .as_datetime()
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    )?;
    let started_at = decision
        .binding
        .evaluated_at
        .as_datetime()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    ensure(
        UtcTimestamp::parse(&started_at)? <= completed_at,
        "publication completion preceded evaluation",
    )?;
    let conclusion = match decision.status {
        PreMergeStatus::ReadyToMerge => CheckConclusion::Success,
        PreMergeStatus::Blocked => CheckConclusion::Failure,
        PreMergeStatus::Inconclusive => CheckConclusion::Failure,
    };
    let binding = CheckBinding {
        completed_at: completed_at.clone(),
        decision_digest: decision.decision_digest.clone(),
        head_sha: decision.binding.head_sha.clone(),
        policy_digest: decision.binding.policy_digest.clone(),
        pull_request_number: decision.binding.pull_request_number,
        repository: decision.binding.repository.clone(),
        schema_version: SCHEMA_VERSION,
        workflow_run_attempt: workflow.workflow_run_attempt,
        workflow_run_id: workflow.workflow_run_id.clone(),
    };
    let binding_bytes = serialize_check_binding(&binding)?;
    let external_id = external_id(&binding);
    let conclusion_text = conclusion_wire(conclusion);
    let title = match decision.status {
        PreMergeStatus::ReadyToMerge => "READY_TO_MERGE",
        PreMergeStatus::Blocked => "BLOCKED",
        PreMergeStatus::Inconclusive => "INCONCLUSIVE",
    };
    let payload = serde_json::to_vec(&serde_json::json!({
        "name": CHECK_NAME,
        "head_sha": decision.binding.head_sha,
        "status": "completed",
        "conclusion": conclusion_text,
        "started_at": started_at,
        "completed_at": completed_at,
        "external_id": external_id,
        "output": {
            "title": title,
            "summary": "policy-kernel authoritative decision",
            "text": String::from_utf8(binding_bytes).expect("JSON is UTF-8"),
        }
    }))?;
    let (owner, repository) = split_repository(&decision.binding.repository)?;
    let response = api.create_check_run(owner, repository, &payload)?;
    let check: RawCheckRun = serde_json::from_slice(&response)?;
    let expected = CheckExpectation {
        repository: decision.binding.repository.clone(),
        pull_request_number: decision.binding.pull_request_number,
        head_sha: decision.binding.head_sha.clone(),
        check_name: CHECK_NAME.to_owned(),
        decision_digest: decision.decision_digest.clone(),
        policy_digest: decision.binding.policy_digest.clone(),
    };
    let parsed_binding = validated_check_binding(&check, &expected)?;
    ensure(
        parsed_binding == binding
            && check.app.id == workflow.check_app_id
            && check.app.slug == workflow.check_app_slug,
        "created check response did not match publication request or trusted app",
    )?;
    let mut observation = check_observation(&check, &parsed_binding, &expected)?;
    ensure(
        observation.conclusion == conclusion,
        "created check conclusion disagreed with decision status",
    )?;
    let publication_observed_at = UtcTimestamp::parse(
        &chrono::DateTime::<chrono::Utc>::from(SystemTime::now()).to_rfc3339(),
    )?;
    ensure(
        parsed_binding.completed_at.as_datetime() <= publication_observed_at.as_datetime(),
        "created check completion was later than runner publication observation",
    )?;
    observation.completed_at = publication_observed_at.clone();
    let receipt = PublicationReceipt {
        schema_version: SCHEMA_VERSION,
        repository: expected.repository.clone(),
        pull_request_number: expected.pull_request_number,
        head_sha: expected.head_sha.clone(),
        check_name: expected.check_name.clone(),
        check_run_id: check.id,
        check_suite_id: check.check_suite.id,
        check_app_id: check.app.id,
        check_app_slug: check.app.slug.clone(),
        workflow_run_id: workflow.workflow_run_id.clone(),
        workflow_run_attempt: workflow.workflow_run_attempt,
        workflow_id: workflow.workflow_id,
        workflow_path: workflow.workflow_path.clone(),
        workflow_head_sha: workflow.workflow_head_sha.clone(),
        decision_digest: expected.decision_digest.clone(),
        policy_digest: expected.policy_digest.clone(),
        published_at: publication_observed_at,
    };
    Ok(PublishedCheck {
        observation,
        receipt,
        check_run_id: check.id,
        workflow_run_attempt: workflow.workflow_run_attempt,
    })
}

pub fn parse_agent_ready(
    event_raw: &[u8],
    receipt: &LabelReceipt,
    expected: &AgentReadyExpectation,
) -> Result<AgentReadyObservation, AdapterError> {
    ensure_schema(receipt.schema_version)?;
    let event: RawIssueEvent = serde_json::from_slice(event_raw)?;
    ensure(
        event.event == "labeled",
        "issue event was not a label event",
    )?;
    let label = event
        .label
        .ok_or_else(|| binding_error("label event omitted label"))?;
    ensure(
        receipt.repository == expected.repository
            && receipt.work_item_id == expected.work_item_id
            && receipt.pull_request_number == expected.pull_request_number
            && receipt.head_sha == expected.head_sha
            && receipt.event_id == event.id
            && receipt.label == expected.label
            && receipt.label == label.name
            && receipt.actor == event.actor.login
            && receipt.observed_at == event.created_at,
        "label receipt disagreed with API event or expected PR head",
    )?;
    Ok(AgentReadyObservation {
        schema_version: SCHEMA_VERSION,
        repository: receipt.repository.clone(),
        work_item_id: receipt.work_item_id.clone(),
        pull_request_number: receipt.pull_request_number,
        head_sha: receipt.head_sha.clone(),
        actor: receipt.actor.clone(),
        label: receipt.label.clone(),
        observed_at: receipt.observed_at.clone(),
        trust: TrustClass::TrustedRunnerAttested,
    })
}

pub fn parse_work_item(
    issue_raw: &[u8],
    receipt: &WorkItemObservation,
    expected: &WorkItemExpectation,
) -> Result<WorkItemObservation, AdapterError> {
    ensure_schema(receipt.schema_version)?;
    let issue: RawIssue = serde_json::from_slice(issue_raw)?;
    let repository_suffix = format!("/repos/{}", expected.repository.as_str());
    ensure(
        issue.number == expected.issue_number
            && issue.repository_url.ends_with(&repository_suffix)
            && receipt.repository == expected.repository
            && receipt.work_item_id == expected.work_item_id
            && receipt.observed_at == issue.updated_at,
        "work-item receipt disagreed with GitHub issue",
    )?;
    Ok(receipt.clone())
}

fn ensure_receipt_expectation(
    receipt: &PublicationReceipt,
    expected: &CheckExpectation,
) -> Result<(), AdapterError> {
    ensure(
        receipt.repository == expected.repository
            && receipt.pull_request_number == expected.pull_request_number
            && receipt.head_sha == expected.head_sha
            && receipt.check_name == expected.check_name
            && receipt.decision_digest == expected.decision_digest
            && receipt.policy_digest == expected.policy_digest,
        "publication receipt did not match expected check binding",
    )
}

fn validated_check_binding(
    check: &RawCheckRun,
    expected: &CheckExpectation,
) -> Result<CheckBinding, AdapterError> {
    ensure(
        check.id > 0 && check.check_suite.id > 0 && check.app.id > 0,
        "check API identity must be positive",
    )?;
    ensure(check.status == "completed", "check run was not completed")?;
    if check.output.text.len() > MAX_CHECK_BINDING_BYTES {
        return Err(AdapterError::PayloadTooLarge {
            actual: check.output.text.len(),
            limit: MAX_CHECK_BINDING_BYTES,
        });
    }
    ensure(
        check.name == expected.check_name && check.head_sha == expected.head_sha.as_str(),
        "check run name or head did not match expectation",
    )?;
    let binding: CheckBinding = serde_json::from_str(&check.output.text)?;
    ensure_schema(binding.schema_version)?;
    ensure(
        binding.repository == expected.repository
            && binding.pull_request_number == expected.pull_request_number
            && binding.head_sha == expected.head_sha
            && binding.decision_digest == expected.decision_digest
            && binding.policy_digest == expected.policy_digest
            && binding.completed_at == check.completed_at,
        "check output binding did not match API observation or expectation",
    )?;
    let (external_decision, external_policy) = parse_external_id(&check.external_id)?;
    ensure(
        external_decision == binding.decision_digest && external_policy == binding.policy_digest,
        "check external_id disagreed with output binding",
    )?;
    Ok(binding)
}

fn check_observation(
    check: &RawCheckRun,
    binding: &CheckBinding,
    expected: &CheckExpectation,
) -> Result<GitHubCheckObservation, AdapterError> {
    let conclusion = parse_conclusion(&check.conclusion)?;
    ensure(
        check.started_at.as_datetime() <= check.completed_at.as_datetime(),
        "check completion preceded start",
    )?;
    Ok(GitHubCheckObservation {
        schema_version: SCHEMA_VERSION,
        repository: expected.repository.clone(),
        pull_request_number: expected.pull_request_number,
        head_sha: expected.head_sha.clone(),
        check_name: expected.check_name.clone(),
        workflow_run_id: binding.workflow_run_id.clone(),
        decision_digest: binding.decision_digest.clone(),
        policy_digest: binding.policy_digest.clone(),
        conclusion,
        started_at: check.started_at.clone(),
        completed_at: check.completed_at.clone(),
    })
}

fn parse_external_id(value: &str) -> Result<(Sha256Digest, Sha256Digest), AdapterError> {
    let mut parts = value.split(':');
    ensure(
        parts.next() == Some("policy-kernel"),
        "invalid check external_id",
    )?;
    ensure(
        parts.next() == Some("v1"),
        "invalid check external_id version",
    )?;
    let decision = parts
        .next()
        .ok_or_else(|| binding_error("check external_id omitted decision digest"))?;
    let policy = parts
        .next()
        .ok_or_else(|| binding_error("check external_id omitted policy digest"))?;
    ensure(parts.next().is_none(), "check external_id had extra fields")?;
    Ok((Sha256Digest::parse(decision)?, Sha256Digest::parse(policy)?))
}

fn external_id(binding: &CheckBinding) -> String {
    format!(
        "policy-kernel:v1:{}:{}",
        binding.decision_digest, binding.policy_digest
    )
}

fn parse_conclusion(value: &str) -> Result<CheckConclusion, AdapterError> {
    match value {
        "success" => Ok(CheckConclusion::Success),
        "failure" => Ok(CheckConclusion::Failure),
        "cancelled" => Ok(CheckConclusion::Cancelled),
        "timed_out" => Ok(CheckConclusion::TimedOut),
        // Legacy inconclusive publications must not be trusted as passing checks.
        "neutral" => Err(binding_error("legacy neutral check requires republication")),
        "skipped" => Ok(CheckConclusion::Skipped),
        _ => Err(binding_error("unknown check conclusion")),
    }
}

fn conclusion_wire(conclusion: CheckConclusion) -> &'static str {
    match conclusion {
        CheckConclusion::Success => "success",
        CheckConclusion::Failure => "failure",
        CheckConclusion::Cancelled => "cancelled",
        CheckConclusion::TimedOut => "timed_out",
        CheckConclusion::Neutral => "neutral",
        CheckConclusion::Skipped => "skipped",
    }
}

fn split_repository(repository: &RepositoryId) -> Result<(&str, &str), AdapterError> {
    repository
        .as_str()
        .split_once('/')
        .ok_or_else(|| binding_error("repository was not owner/name"))
}

fn ensure_schema(version: u32) -> Result<(), AdapterError> {
    ensure(version == SCHEMA_VERSION, "unsupported schema version")
}

fn ensure(condition: bool, message: &'static str) -> Result<(), AdapterError> {
    if condition {
        Ok(())
    } else {
        Err(binding_error(message))
    }
}

#[derive(Debug, Deserialize)]
struct RawRepository {
    full_name: String,
}

#[derive(Debug, Deserialize)]
struct RawPullRef {
    #[serde(rename = "ref")]
    reference: String,
    sha: String,
    repo: RawRepository,
}

#[derive(Debug, Deserialize)]
struct RawActor {
    login: String,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Debug, Deserialize)]
struct RawPullRequest {
    number: u64,
    state: String,
    draft: bool,
    updated_at: UtcTimestamp,
    merged_at: Option<UtcTimestamp>,
    merge_commit_sha: Option<String>,
    base: RawPullRef,
    head: RawPullRef,
    merged_by: Option<RawActor>,
}

#[derive(Debug, Deserialize)]
struct RawBranch {
    name: String,
    protected: bool,
}

#[derive(Debug, Deserialize)]
struct RawApp {
    id: u64,
    slug: String,
}

#[derive(Debug, Deserialize)]
struct RawCheckSuite {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RawCheckOutput {
    text: String,
}

#[derive(Debug, Deserialize)]
struct RawCheckRun {
    id: u64,
    name: String,
    head_sha: String,
    status: String,
    conclusion: String,
    external_id: String,
    started_at: UtcTimestamp,
    completed_at: UtcTimestamp,
    output: RawCheckOutput,
    check_suite: RawCheckSuite,
    app: RawApp,
}

#[derive(Debug, Deserialize)]
struct RawWorkflowRun {
    id: u64,
    run_attempt: u64,
    workflow_id: u64,
    path: String,
    head_sha: String,
    status: String,
    repository: RawRepository,
}

#[derive(Debug, Deserialize)]
struct RawLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct RawIssueEvent {
    id: u64,
    event: String,
    created_at: UtcTimestamp,
    actor: RawActor,
    label: Option<RawLabel>,
}

#[derive(Debug, Deserialize)]
struct RawIssue {
    number: u64,
    updated_at: UtcTimestamp,
    repository_url: String,
}
