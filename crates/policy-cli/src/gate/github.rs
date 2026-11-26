//! GitHub REST observation, with completeness and identity checks at the API boundary.
use super::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use github_adapter::encode_route_segment;
use policy_contracts::{CommitSha, RepositoryId, UtcTimestamp};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

const PAGE_SIZE: usize = 100;
const MAX_PR_FILES: usize = 3_000;
const MAX_FILTERED_RUNS: usize = 1_000;

pub(super) type Get<'a> = dyn Fn(&str) -> Result<Vec<u8>> + 'a;

pub(super) struct GitHub<'a> {
    get: &'a Get<'a>,
    repository: &'a str,
    prefix: String,
    number: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(super) struct Pull {
    pub id: u64,
    pub number: u64,
    pub state: String,
    pub merged: bool,
    pub base: Branch,
    pub head: Branch,
    pub labels: Vec<Label>,
    pub changed_files: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(super) struct Branch {
    pub sha: CommitSha,
    #[serde(rename = "ref")]
    name: String,
    pub repo: Repository,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(super) struct Repository {
    pub id: u64,
    pub full_name: RepositoryId,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(super) struct Label {
    pub name: String,
}

#[derive(Debug, Deserialize)]
struct ChangedFile {
    filename: String,
    status: String,
    previous_filename: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Contents {
    #[serde(rename = "type")]
    kind: String,
    path: String,
    encoding: String,
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(super) struct Review {
    id: u64,
    state: String,
    user: Option<User>,
}

#[derive(Debug, Deserialize, Serialize)]
struct User {
    login: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub(super) struct Run {
    pub id: u64,
    run_number: u64,
    workflow_id: u64,
    path: String,
    event: String,
    head_sha: CommitSha,
    head_branch: String,
    repository: Repository,
    head_repository: Repository,
    pull_requests: Vec<PullLink>,
    pub run_attempt: u64,
    pub run_started_at: UtcTimestamp,
    updated_at: UtcTimestamp,
    status: String,
    conclusion: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
struct PullLink {
    id: u64,
    number: u64,
    base: PullLinkBranch,
    head: PullLinkBranch,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
struct PullLinkBranch {
    #[serde(rename = "ref")]
    name: String,
    sha: CommitSha,
    repo: RepoId,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
struct RepoId {
    id: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct Job {
    pub id: u64,
    pub name: String,
    run_id: u64,
    run_attempt: u64,
    head_sha: CommitSha,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: UtcTimestamp,
    pub completed_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct Artifact {
    pub id: u64,
    pub name: String,
    pub size_in_bytes: u64,
    pub expired: bool,
    pub created_at: UtcTimestamp,
    pub expires_at: UtcTimestamp,
    pub digest: Option<String>,
    pub workflow_run: Option<ArtifactRun>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct ArtifactRun {
    pub id: u64,
    pub repository_id: u64,
    pub head_repository_id: u64,
    pub head_sha: CommitSha,
}

#[derive(Debug, Deserialize)]
struct GitCommit {
    sha: CommitSha,
    tree: GitObject,
    parents: Vec<GitObject>,
}

#[derive(Debug, Deserialize)]
struct GitObject {
    sha: CommitSha,
}

#[derive(Debug, Deserialize)]
struct GitRef {
    #[serde(rename = "ref")]
    name: String,
    object: GitRefObject,
}

#[derive(Debug, Deserialize)]
struct GitRefObject {
    #[serde(rename = "type")]
    kind: String,
    sha: CommitSha,
}

enum Page {
    Array(Option<usize>),
    Object(&'static str),
}

impl<'a> GitHub<'a> {
    pub fn new(get: &'a Get<'a>, repository: &'a str, number: u64) -> Result<Self> {
        let (owner, name) = repository
            .split_once('/')
            .ok_or("expected repository in owner/name form")?;
        let (owner, name) = github_adapter::validated_repository(owner, name)?;
        if number == 0 {
            return Err("pull request number must be positive".into());
        }
        Ok(Self {
            get,
            repository,
            prefix: format!("/repos/{owner}/{name}"),
            number,
        })
    }

    fn bytes(&self, path: &str) -> Result<Vec<u8>> {
        (self.get)(&format!("{}{path}", self.prefix))
            .map_err(|error| format!("GitHub GET {path}: {error}").into())
    }

    fn json<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        serde_json::from_slice(&self.bytes(path)?)
            .map_err(|error| format!("invalid GitHub response for {path}: {error}").into())
    }

    fn pages<T: DeserializeOwned>(&self, path: &str, shape: Page, limit: usize) -> Result<Vec<T>> {
        let separator = if path.contains('?') { '&' } else { '?' };
        let mut items = Vec::new();
        let mut total = match shape {
            Page::Array(count) => count,
            Page::Object(_) => None,
        };
        for page in 1..=limit / PAGE_SIZE + 1 {
            let value: Value = self.json(&format!(
                "{path}{separator}per_page={PAGE_SIZE}&page={page}"
            ))?;
            let rows: Vec<T> = match shape {
                Page::Array(_) => serde_json::from_value(value)?,
                Page::Object(key) => {
                    let count = serde_json::from_value(value["total_count"].clone())?;
                    if total.is_some_and(|previous| previous != count) {
                        return Err(
                            format!("GitHub collection changed while reading {path}").into()
                        );
                    }
                    total = Some(count);
                    serde_json::from_value(value[key].clone())?
                }
            };
            let count = rows.len();
            items.extend(rows);
            if items.len() > limit
                || total.is_some_and(|total| total > limit || items.len() > total)
            {
                return Err(
                    format!("GitHub collection is oversized or inconsistent for {path}").into(),
                );
            }
            if count < PAGE_SIZE || total == Some(items.len()) {
                if total.is_some_and(|total| items.len() != total) {
                    return Err(format!("incomplete GitHub collection for {path}").into());
                }
                return Ok(items);
            }
        }
        Err(format!("incomplete GitHub collection for {path}").into())
    }

    pub fn pull(&self) -> Result<Pull> {
        let mut pull: Pull = self.json(&format!("/pulls/{}", self.number))?;
        if pull.id == 0
            || pull.number != self.number
            || pull.base.repo.id == 0
            || pull.head.repo.id == 0
            || !self
                .repository
                .eq_ignore_ascii_case(pull.base.repo.full_name.as_str())
        {
            return Err("GitHub response identifies a different pull request or repository".into());
        }
        pull.labels.sort();
        if pull.labels.windows(2).any(|labels| labels[0] == labels[1]) {
            return Err("ambiguous duplicate PR label".into());
        }
        Ok(pull)
    }

    pub fn files(&self, pull: &Pull) -> Result<Vec<String>> {
        if pull.changed_files > MAX_PR_FILES {
            return Err("PR changed-file list exceeds GitHub's 3000-file API limit".into());
        }
        let files: Vec<ChangedFile> = self.pages(
            &format!("/pulls/{}/files", self.number),
            Page::Array(Some(pull.changed_files)),
            MAX_PR_FILES,
        )?;
        let mut names = BTreeSet::new();
        let mut paths = BTreeSet::new();
        for file in files {
            if !names.insert(file.filename.clone()) {
                return Err("ambiguous duplicate entry in changed-file list".into());
            }
            paths.insert(file.filename);
            match (file.status.as_str(), file.previous_filename) {
                ("renamed", None) => {
                    return Err("changed-file list is missing a rename source".into())
                }
                (_, Some(previous)) => {
                    paths.insert(previous);
                }
                _ => {}
            }
        }
        Ok(paths.into_iter().collect())
    }

    pub fn base_tip(&self, pull: &Pull) -> Result<CommitSha> {
        let branch = encode_path(&pull.base.name)?;
        let reference: GitRef = self.json(&format!("/git/ref/heads/{branch}"))?;
        if reference.name != format!("refs/heads/{}", pull.base.name)
            || reference.object.kind != "commit"
        {
            return Err("GitHub did not resolve the requested base branch to a commit".into());
        }
        Ok(reference.object.sha)
    }

    pub fn require_unique_head(&self, pull: &Pull) -> Result<()> {
        let (owner, _) = pull
            .head
            .repo
            .full_name
            .as_str()
            .split_once('/')
            .ok_or("missing head repository owner")?;
        let head = encode_route_segment(&format!("{owner}:{}", pull.head.name))?;
        // A second open PR can be created after the workflow run. Observe the
        // branch independently of the pull request links on that run.
        let links: Vec<PullLink> =
            self.json(&format!("/pulls?state=open&head={head}&per_page=2"))?;
        match links.as_slice() {
            [link] if link.matches(pull) => Ok(()),
            [_, _, ..] => Err("head branch has multiple open pull requests".into()),
            _ => Err("could not confirm a unique open pull request for the head branch".into()),
        }
    }

    pub fn policy(&self, base_tip: &CommitSha, path: &str) -> Result<Vec<u8>> {
        let encoded = encode_path(path)?;
        let content: Contents = self
            .json(&format!("/contents/{encoded}?ref={base_tip}"))
            .map_err(|error| format!("policy unavailable at base tip {base_tip}: {error}"))?;
        if content.kind != "file" || content.encoding != "base64" || content.path != path {
            return Err("policy at base tip is not the requested readable file".into());
        }
        let encoded: Vec<u8> = content
            .content
            .bytes()
            .filter(|byte| !matches!(byte, b'\r' | b'\n'))
            .collect();
        Ok(STANDARD.decode(encoded)?)
    }

    pub fn reviews(&self) -> Result<Vec<Review>> {
        self.pages(
            &format!("/pulls/{}/reviews", self.number),
            Page::Array(None),
            MAX_PR_FILES,
        )
    }

    pub fn select_run(&self, pull: &Pull, workflow: &str) -> Result<Run> {
        let file = workflow
            .strip_prefix(".github/workflows/")
            .ok_or("invalid workflow path")?;
        let file = encode_route_segment(file)?;
        let runs: Vec<Run> = self.pages(
            &format!(
                "/actions/workflows/{file}/runs?event=pull_request&head_sha={}",
                pull.head.sha
            ),
            Page::Object("workflow_runs"),
            MAX_FILTERED_RUNS,
        )?;
        // The filtered API caps results at 1,000. If the cap is reached,
        // there is no complete observation of the candidate set.
        if runs.len() == MAX_FILTERED_RUNS {
            return Err("GitHub workflow run search reached its result limit".into());
        }
        let mut ids = BTreeSet::new();
        let mut numbers = BTreeSet::new();
        let mut selected = None::<Run>;
        for run in runs {
            if !run.matches_head(pull, workflow)
                || run.id == 0
                || run.run_number == 0
                || run.workflow_id == 0
                || run.run_attempt == 0
                || !ids.insert(run.id)
                || !numbers.insert(run.run_number)
            {
                return Err("inconsistent or duplicate GitHub workflow run identity".into());
            }
            if run.pull_requests.len() > 1 {
                return Err(format!("workflow run {} links multiple pull requests", run.id).into());
            }
            if (run.status == "completed" && run.conclusion.as_deref() == Some("skipped"))
                || (!run.pull_requests.is_empty()
                    && run
                        .pull_requests
                        .iter()
                        .all(|link| link.id != pull.id && link.number != pull.number))
            {
                continue;
            }
            if selected
                .as_ref()
                .is_none_or(|other| run.run_number > other.run_number)
            {
                selected = Some(run);
            }
        }
        let selected = selected.ok_or_else(|| format!("no non-skipped PR run for {workflow}"))?;
        if !selected.links_pull(pull) {
            return Err(format!("latest run for {workflow} has no verified PR association").into());
        }
        Ok(selected)
    }

    pub fn attempt(&self, selected: &Run) -> Result<Run> {
        let latest: Run = self.json(&format!("/actions/runs/{}", selected.id))?;
        let attempt: Run = self.json(&attempt_path(selected))?;
        if *selected != latest || !latest.same_attempt(&attempt) {
            return Err("workflow run or latest attempt changed while reading".into());
        }
        if attempt.status != "completed" || !known_conclusion(attempt.conclusion.as_deref()) {
            return Err("latest workflow attempt has no completed conclusion".into());
        }
        Ok(attempt)
    }

    pub fn jobs(&self, run: &Run) -> Result<Vec<Job>> {
        let jobs: Vec<Job> = self.pages(
            &format!("{}/jobs", attempt_path(run)),
            Page::Object("jobs"),
            MAX_FILTERED_RUNS,
        )?;
        let mut ids = BTreeSet::new();
        if jobs.iter().any(|job| {
            job.id == 0
                || !ids.insert(job.id)
                || job.run_id != run.id
                || job.run_attempt != run.run_attempt
                || job.head_sha != run.head_sha
        }) {
            return Err(
                "job identities do not bind uniquely to the selected workflow attempt".into(),
            );
        }
        Ok(jobs)
    }

    pub fn artifacts(&self, run: &Run) -> Result<Vec<Artifact>> {
        self.pages(
            &format!("/actions/runs/{}/artifacts", run.id),
            Page::Object("artifacts"),
            MAX_FILTERED_RUNS,
        )
    }

    pub fn archive(&self, id: u64) -> Result<Vec<u8>> {
        self.bytes(&format!("/actions/artifacts/{id}/zip"))
    }

    pub fn confirm_checkout(&self, checkout: &CommitSha, head: &CommitSha) -> Result<()> {
        if checkout == head {
            return Ok(());
        }
        // A pull_request workflow may check out GitHub's synthetic merge.
        // Only accept that shape when it tested the exact PR head tree.
        let merge: GitCommit = self.json(&format!("/git/commits/{checkout}"))?;
        let source: GitCommit = self.json(&format!("/git/commits/{head}"))?;
        if &merge.sha != checkout
            || &source.sha != head
            || merge.parents.len() != 2
            || &merge.parents[1].sha != head
            || merge.tree.sha != source.tree.sha
        {
            return Err("native checkout does not match the PR head or its tree".into());
        }
        Ok(())
    }
}

impl Run {
    fn matches_head(&self, pull: &Pull, workflow: &str) -> bool {
        self.event == "pull_request"
            && self.path == workflow
            && self.head_sha == pull.head.sha
            && self.repository.id == pull.base.repo.id
    }

    fn links_pull(&self, pull: &Pull) -> bool {
        self.head_repository.id == pull.head.repo.id
            && self.head_branch == pull.head.name
            && matches!(self.pull_requests.as_slice(), [link] if link.matches(pull))
    }

    fn same_attempt(&self, other: &Self) -> bool {
        self.id == other.id
            && self.run_number == other.run_number
            && self.workflow_id == other.workflow_id
            && self.path == other.path
            && self.event == other.event
            && self.head_sha == other.head_sha
            && self.head_branch == other.head_branch
            && self.repository == other.repository
            && self.head_repository == other.head_repository
            && self.pull_requests == other.pull_requests
            && self.run_attempt == other.run_attempt
            && self.run_started_at == other.run_started_at
            && self.status == other.status
            && self.conclusion == other.conclusion
    }
}

impl PullLink {
    fn matches(&self, pull: &Pull) -> bool {
        self.id == pull.id
            && self.number == pull.number
            && self.head.repo.id == pull.head.repo.id
            && self.head.name == pull.head.name
            && self.head.sha == pull.head.sha
            && self.base.repo.id == pull.base.repo.id
            && self.base.name == pull.base.name
    }
}

fn encode_path(path: &str) -> Result<String> {
    let segments = path
        .split('/')
        .map(encode_route_segment)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(segments.join("/"))
}

fn attempt_path(run: &Run) -> String {
    format!("/actions/runs/{}/attempts/{}", run.id, run.run_attempt)
}

pub(super) fn known_conclusion(conclusion: Option<&str>) -> bool {
    matches!(
        conclusion,
        Some(
            "success"
                | "failure"
                | "neutral"
                | "cancelled"
                | "skipped"
                | "timed_out"
                | "action_required"
                | "stale"
                | "startup_failure"
        )
    )
}
