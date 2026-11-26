//! Advisory gate for GitHub Actions evidence associated with a pull request.
mod github;
mod local;
mod native;
#[cfg(test)]
mod tests;

use github::{Get, GitHub, Job, Pull, Review, Run};
use github_adapter::GhCliApi;
use local::{LocalPolicy, LocalRequest};
use native::{JobContext, NativeEvidence, NativePolicy};
use policy_contracts::{
    CommitSha, PolicySnapshot, SensorDefinition, SensorId, SensorProfile, SensorResultRequirement,
    Sha256Digest, TestAdequacyPolicy, TrustClass, UtcTimestamp,
};
use policy_core::resolve_required_scope;
use policy_local_attest::GateStatement;
use policy_native_importer::raw_digest;
use serde::{Deserialize, Serialize};
use std::{
    cell::OnceCell,
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    path::Path,
    time::{Duration, SystemTime},
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GatePolicy {
    policy: PolicySnapshot,
    checks: Vec<Check>,
    local_attestation: Option<LocalPolicy>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Check {
    id: SensorId,
    workflow: String,
    job: String,
    source: Source,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Source {
    Light,
    Native(Box<NativePolicy>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Verdict {
    Ready,
    NotReady,
    Unknown,
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NotReady => "not ready",
            Self::Unknown => "unknown",
        }
    }

    fn exit_code(self) -> u8 {
        match self {
            Self::Ready => 0,
            Self::NotReady => super::EXIT_BLOCKED,
            Self::Unknown => super::EXIT_INCONCLUSIVE,
        }
    }
}

#[derive(Debug, Serialize)]
struct Evidence<'a> {
    schema_version: u32,
    advisory: bool,
    repository: &'a str,
    pr: u64,
    policy_path: &'a str,
    evaluated_at: &'a UtcTimestamp,
    verdict: Verdict,
    reason: String,
    pull: Option<Pull>,
    final_pull: Option<Pull>,
    base_tip: Option<CommitSha>,
    final_base_tip: Option<CommitSha>,
    policy_sha256: Option<Sha256Digest>,
    changed_paths: Option<Vec<String>>,
    required_checks: Vec<SensorId>,
    workflows: BTreeMap<String, Run>,
    checks: Vec<CheckEvidence>,
    evidence_classes: Vec<TrustClass>,
    local_attestation: Option<GateStatement>,
    advisory_reviews: Option<Vec<Review>>,
    advisory_review_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckEvidence {
    check: SensorId,
    workflow: String,
    job_name: String,
    kind: &'static str,
    job: Option<Job>,
    native: Option<NativeEvidence>,
    evidence_class: Option<TrustClass>,
    verdict: Verdict,
    reason: String,
}

pub(super) fn run(
    repo: &str,
    pr: u64,
    policy: &str,
    local_attestation: Option<&Path>,
    output: &Path,
) -> std::result::Result<u8, String> {
    let now = UtcTimestamp::try_from(SystemTime::now()).map_err(|error| error.to_string())?;
    let gh = GhCliApi::new("gh", Duration::from_secs(30), 8 * 1024 * 1024);
    let archives = GhCliApi::new("gh", Duration::from_secs(120), native::MAX_ARCHIVE);
    let get = |route: &str| {
        let api = if route.ends_with("/zip") {
            &archives
        } else {
            &gh
        };
        Ok(api.get(route)?)
    };
    let evidence = evaluate_with(&get, repo, pr, policy, (&now, local_attestation));
    write_evidence(output, &evidence)
        .map_err(|error| format!("could not write {}: {error}", output.display()))?;
    let reason: String = evidence
        .reason
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(360)
        .collect();
    writeln!(io::stdout().lock(), "{}", evidence.line(&reason))
        .map_err(|error| error.to_string())?;
    Ok(evidence.verdict.exit_code())
}

fn write_evidence(path: &Path, evidence: &Evidence<'_>) -> Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty());
    let mut file = tempfile::NamedTempFile::new_in(parent.unwrap_or_else(|| Path::new(".")))?;
    serde_json::to_writer_pretty(&mut file, evidence)?;
    writeln!(file)?;
    file.persist_noclobber(path)?;
    Ok(())
}

#[cfg(test)]
fn evaluate<'a>(
    get: &Get<'_>,
    repo: &'a str,
    pr: u64,
    path: &'a str,
    now: &'a UtcTimestamp,
) -> Evidence<'a> {
    evaluate_with(get, repo, pr, path, (now, None))
}

fn evaluate_with<'a>(
    get: &Get<'_>,
    repo: &'a str,
    pr: u64,
    path: &'a str,
    (now, local_attestation): (&'a UtcTimestamp, Option<&Path>),
) -> Evidence<'a> {
    let mut evidence = Evidence {
        schema_version: 1,
        advisory: true,
        repository: repo,
        pr,
        policy_path: path,
        evaluated_at: now,
        verdict: Verdict::Unknown,
        reason: String::new(),
        pull: None,
        final_pull: None,
        base_tip: None,
        final_base_tip: None,
        policy_sha256: None,
        changed_paths: None,
        required_checks: vec![],
        workflows: BTreeMap::new(),
        checks: vec![],
        evidence_classes: vec![],
        local_attestation: None,
        advisory_reviews: None,
        advisory_review_error: None,
    };
    match GitHub::new(get, repo, pr).and_then(|api| evidence.collect(&api, local_attestation)) {
        Ok(verdict) => {
            evidence.verdict = verdict;
            if verdict != Verdict::Unknown {
                evidence.evidence_classes =
                    [TrustClass::CiObserved, TrustClass::LocalRunnerAttested]
                        .into_iter()
                        .filter(|class| {
                            evidence
                                .checks
                                .iter()
                                .any(|check| check.evidence_class == Some(*class))
                        })
                        .collect();
            }
        }
        Err(error) => evidence.reason = error.to_string(),
    }
    evidence
}

impl Evidence<'_> {
    fn line(&self, reason: &str) -> String {
        let classes = match self.evidence_classes.as_slice() {
            [TrustClass::CiObserved] => "[ci_observed] ",
            [TrustClass::LocalRunnerAttested] => "[local_runner_attested] ",
            [TrustClass::CiObserved, TrustClass::LocalRunnerAttested] => {
                "[ci_observed, local_runner_attested] "
            }
            _ => "",
        };
        format!("{}: {classes}{reason}", self.verdict.label())
    }

    fn collect(&mut self, api: &GitHub<'_>, local: Option<&Path>) -> Result<Verdict> {
        let pull = api.pull()?;
        self.pull = Some(pull.clone());
        if pull.state != "open" || pull.merged {
            return Err("pull request is not open".into());
        }
        let base_tip = api.base_tip(&pull)?;
        self.base_tip = Some(base_tip.clone());
        let result = self.read_checks(api, &pull, &base_tip, local);
        match api.reviews() {
            Ok(observed) => self.advisory_reviews = Some(observed),
            Err(error) => self.advisory_review_error = Some(error.to_string()),
        }
        let result = result.and_then(|verdict| {
            if verdict != Verdict::Unknown {
                api.require_unique_head(&pull)?;
            }
            Ok(verdict)
        });
        let final_pull = api
            .pull()
            .map_err(|error| format!("could not confirm PR snapshot: {error}"))?;
        self.final_pull = Some(final_pull.clone());
        let final_base_tip = api
            .base_tip(&final_pull)
            .map_err(|error| format!("could not confirm base branch tip: {error}"))?;
        self.final_base_tip = Some(final_base_tip.clone());
        if final_pull != pull || final_base_tip != base_tip {
            return Err(
                "obsolete evidence: PR state, head, base, labels, file count or base branch tip changed while reading"
                    .into(),
            );
        }
        result
    }

    fn read_checks(
        &mut self,
        api: &GitHub<'_>,
        pull: &Pull,
        base_tip: &CommitSha,
        local: Option<&Path>,
    ) -> Result<Verdict> {
        if !native::safe_path(self.policy_path) {
            return Err("policy must be a portable repository-relative path".into());
        }
        let policy_bytes = api.policy(base_tip, self.policy_path)?;
        self.policy_sha256 = Some(raw_digest(&policy_bytes).sha256);
        let settings: GatePolicy = serde_json::from_slice(&policy_bytes)
            .map_err(|error| format!("invalid gate policy at base tip: {error}"))?;
        let checks = settings.validate()?;
        let paths = api.files(pull)?;
        if let Some(path) = paths.iter().find(|path| {
            path.starts_with(".github/workflows/") || path.starts_with(".github/actions/")
        }) {
            let reason = format!(
                "PR modifies a CI workflow or action, so its evidence cannot be trusted: {path}"
            );
            self.changed_paths = Some(paths);
            return Err(reason.into());
        }
        let labels: Vec<_> = pull.labels.iter().map(|label| label.name.clone()).collect();
        let profile = scope_profile(&settings.checks);
        if settings
            .policy
            .sensor_profile_digest
            .as_ref()
            .is_some_and(|digest| {
                policy_core::canonical_sha256(&profile).as_ref().ok() != Some(digest)
            })
        {
            return Err("gate check profile does not match policy's profile digest".into());
        }
        self.required_checks = resolve_required_scope(&settings.policy, &profile, &paths, &labels)?;
        self.changed_paths = Some(paths);
        if self.required_checks.is_empty() {
            return Err("gate policy selected no required checks".into());
        }
        let local_checks = match settings.local_attestation.as_ref() {
            Some(config) => {
                let paths = self
                    .changed_paths
                    .as_deref()
                    .expect("observed changed paths");
                let request = LocalRequest {
                    pull,
                    base_tip,
                    gate: self,
                    directory: local,
                };
                if config.needs_ci(&settings.policy, paths, &request, &labels)? {
                    None
                } else {
                    Some(config)
                }
            }
            None if local.is_some() => {
                return Err("local attestation requires opt-in in the base gate policy".into())
            }
            None => None,
        };
        let mut groups = BTreeMap::<&str, Vec<&Check>>::new();
        let mut local_group = Vec::new();
        for id in &self.required_checks {
            let check = checks
                .get(id)
                .ok_or_else(|| format!("scoped check {id} has no CI binding"))?;
            if local_checks.is_some_and(|config| config.checks.contains(id)) {
                local_group.push(*check);
            } else {
                groups.entry(&check.workflow).or_default().push(check);
            }
        }
        api.require_unique_head(pull)?;
        if let (Some(config), false) = (local_checks, local_group.is_empty()) {
            let request = LocalRequest {
                pull,
                base_tip,
                gate: self,
                directory: local,
            };
            let scope = match config.evaluate(api, &settings.policy, &request) {
                Ok((statement, scope)) => {
                    self.local_attestation = Some(statement);
                    Ok(scope)
                }
                Err(error) => Err(error.to_string()),
            };
            self.checks.extend(local_group.into_iter().map(|check| {
                let failure = match &scope {
                    Ok(scope) => local::uncovered(scope, check),
                    Err(error) => Some(error.clone()),
                };
                let (verdict, reason, class) = match failure {
                    None => (
                        Verdict::Ready,
                        "verified signed local scope satisfied".into(),
                        Some(TrustClass::LocalRunnerAttested),
                    ),
                    Some(reason) => (Verdict::Unknown, reason, None),
                };
                CheckEvidence {
                    check: check.id.clone(),
                    workflow: check.workflow.clone(),
                    job_name: check.job.clone(),
                    kind: "local_runner_attested",
                    job: None,
                    native: None,
                    evidence_class: class,
                    verdict,
                    reason,
                }
            }));
        }
        for group in groups {
            self.read_workflow(api, pull, group, settings.policy.max_evidence_age_seconds);
        }
        // A new run or rerun can arrive with the same PR head and base.
        for (workflow, observed) in &self.workflows {
            let current = api.select_run(pull, workflow)?;
            if current != *observed {
                return Err(format!(
                    "obsolete evidence: latest run or attempt changed for {workflow}"
                )
                .into());
            }
        }

        for verdict in [Verdict::Unknown, Verdict::NotReady] {
            if let Some(check) = self.checks.iter().find(|check| check.verdict == verdict) {
                self.reason = format!("{}: {}", check.check, check.reason);
                return Ok(verdict);
            }
        }
        self.reason = format!("all {} required checks passed", self.required_checks.len());
        Ok(Verdict::Ready)
    }

    fn read_workflow(
        &mut self,
        api: &GitHub<'_>,
        pull: &Pull,
        (workflow, checks): (&str, Vec<&Check>),
        maximum_age_seconds: u64,
    ) {
        let jobs = (|| -> Result<_> {
            let selected = api.select_run(pull, workflow)?;
            self.workflows.insert(workflow.to_owned(), selected.clone());
            let run = api.attempt(&selected)?;
            let jobs = api.jobs(&run)?;
            Ok((run, jobs))
        })();
        let artifacts = OnceCell::new();
        for check in checks {
            let mut result = CheckEvidence {
                check: check.id.clone(),
                workflow: workflow.to_owned(),
                job_name: check.job.clone(),
                kind: match check.source {
                    Source::Light => "light",
                    Source::Native(_) => "native_run_v1",
                },
                job: None,
                native: None,
                evidence_class: None,
                verdict: Verdict::Unknown,
                reason: String::new(),
            };
            let evaluated = (|| {
                let (run, jobs) = jobs.as_ref().map_err(ToString::to_string)?;
                let mut matching = jobs.iter().filter(|job| job.name == check.job);
                let job = matching
                    .next()
                    .ok_or("required job is missing from latest attempt")?;
                if matching.next().is_some() {
                    return Err("conflicting duplicate job name in workflow attempt".into());
                }
                result.job = Some(job.clone());
                let context = JobContext {
                    api,
                    pull,
                    run,
                    job,
                    evaluated_at: self.evaluated_at,
                    maximum_age_seconds,
                };
                evaluate_job(check, &context, &mut result, &artifacts)
            })();
            match evaluated {
                Ok((verdict, reason)) => {
                    result.verdict = verdict;
                    result.reason = reason;
                    result.evidence_class = Some(TrustClass::CiObserved);
                }
                Err(error) => result.reason = error.to_string(),
            }
            self.checks.push(result);
        }
    }
}

impl GatePolicy {
    fn validate(&self) -> Result<BTreeMap<&SensorId, &Check>> {
        if self.policy.max_evidence_age_seconds == 0 {
            return Err("gate policy maximum evidence age must be positive".into());
        }
        let mut checks = BTreeMap::new();
        let mut jobs = BTreeSet::new();
        let mut native_artifacts = BTreeSet::new();
        for check in &self.checks {
            let file = check.workflow.strip_prefix(".github/workflows/");
            let valid_workflow = file.is_some_and(|path| {
                native::safe_path(path)
                    && !path.contains('/')
                    && (path.ends_with(".yml") || path.ends_with(".yaml"))
            });
            if !valid_workflow
                || check.job.trim().is_empty()
                || check.job.chars().any(char::is_control)
                || !valid_key(&check.id)
            {
                return Err(format!("invalid CI identity for check {}", check.id).into());
            }
            if checks.insert(&check.id, check).is_some()
                || !jobs.insert((&check.workflow, &check.job))
            {
                return Err("gate policy contains a duplicate check ID or workflow job".into());
            }
            if let Source::Native(native) = &check.source {
                let key = native.artifact_key(&check.id);
                if !valid_key(key) || !native_artifacts.insert((&check.workflow, key)) {
                    return Err(
                        "gate policy contains an invalid or duplicate workflow artifact key".into(),
                    );
                }
            }
        }
        if let Some(local) = &self.local_attestation {
            local.validate(&checks)?;
        }
        Ok(checks)
    }
}

fn valid_key(id: &SensorId) -> bool {
    let id = id.as_str();
    id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn scope_profile(checks: &[Check]) -> SensorProfile {
    SensorProfile {
        schema_version: 2,
        sensors: checks
            .iter()
            .map(|check| SensorDefinition {
                id: check.id.clone(),
                required: false,
                required_when: vec![],
                not_applicable_when: vec![],
                accepted_exit_codes: vec![0],
                result_requirement: Some(SensorResultRequirement::Process),
                result_binding: None,
            })
            .collect(),
        adequacy: TestAdequacyPolicy {
            acceptance_criterion_ids: vec![],
            changed_behavior_ids: vec![],
            negative_case_ids: vec![],
        },
    }
}

fn evaluate_job(
    check: &Check,
    context: &JobContext<'_>,
    result: &mut CheckEvidence,
    artifacts: &OnceCell<Result<Vec<github::Artifact>>>,
) -> Result<(Verdict, String)> {
    let job = context.job;
    let conclusion = job.conclusion.as_deref();
    if job.status != "completed" || !github::known_conclusion(conclusion) {
        return Err("required job has no completed conclusion".into());
    }
    if job.started_at < context.run.run_started_at {
        return Err("required job result was carried forward from an older attempt".into());
    }
    if job
        .completed_at
        .as_ref()
        .is_none_or(|end| *end < job.started_at || *end > *context.evaluated_at)
        || context
            .evaluated_at
            .duration_since(&job.started_at)
            .is_none_or(|age| age > Duration::from_secs(context.maximum_age_seconds))
    {
        return Err("required job has stale or invalid execution times".into());
    }
    let concluded = || format!("required job concluded {}", conclusion.unwrap_or_default());
    match conclusion {
        Some("success") => {}
        Some("failure" | "timed_out") => return Ok((Verdict::NotReady, concluded())),
        _ => return Err(concluded().into()),
    }
    match &check.source {
        Source::Light => Ok((
            Verdict::Ready,
            "named CI job succeeded (light evidence)".into(),
        )),
        Source::Native(policy) => {
            let artifacts = artifacts
                .get_or_init(|| context.api.artifacts(context.run))
                .as_ref()
                .map_err(ToString::to_string)?;
            match policy.evaluate(context, result, artifacts)? {
                Verdict::Ready => Ok((Verdict::Ready, "verified native scope satisfied".into())),
                Verdict::NotReady => Ok((
                    Verdict::NotReady,
                    "verified native scope did not pass".into(),
                )),
                Verdict::Unknown => Err("native scope evidence is unknown".into()),
            }
        }
    }
}
