//! Opt-in transport smoke test. This deliberately publishes a FAILURE check;
//! empty evidence and an unauthorized work item can never authorize a release.
use github_adapter::{parse_pull_request, GhCliApi, GitHubApi, PublicationReceipt};
use policy_contracts::*;
use policy_core::{canonical_sha256, pre_merge_policy_digest};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    env, fs,
    path::Path,
    time::{Duration, SystemTime},
};

#[test]
#[ignore = "requires an explicitly enabled same-repository task10 PR workflow and checks:write"]
fn real_actions_run_publishes_and_reads_back_blocked_decision() {
    assert_eq!(env::var("POLICY_CI_LIVE_SMOKE").as_deref(), Ok("1"));
    assert_eq!(env::var("GITHUB_ACTIONS").as_deref(), Ok("true"));
    assert_eq!(env::var("GITHUB_EVENT_NAME").as_deref(), Ok("pull_request"));
    let repository = RepositoryId::parse(env::var("GITHUB_REPOSITORY").unwrap()).unwrap();
    assert_eq!(repository.as_str(), "FlitCatz/policy-kernel");
    let event: Value =
        serde_json::from_slice(&fs::read(env::var("GITHUB_EVENT_PATH").unwrap()).unwrap()).unwrap();
    assert_eq!(event["pull_request"]["head"]["ref"], "task10-github-ci");
    assert_eq!(
        event["pull_request"]["head"]["repo"]["full_name"],
        repository.as_str()
    );
    let head = CommitSha::parse(event["pull_request"]["head"]["sha"].as_str().unwrap()).unwrap();
    let number = PullRequestNumber::new(event["number"].as_u64().unwrap()).unwrap();
    let run = WorkflowRunId::parse(env::var("GITHUB_RUN_ID").unwrap()).unwrap();
    let attempt: u64 = env::var("GITHUB_RUN_ATTEMPT").unwrap().parse().unwrap();
    let (owner, repo) = repository.as_str().split_once('/').unwrap();
    let api = GhCliApi::new("gh", Duration::from_secs(30), 8 * 1024 * 1024);
    let raw_pr = api.pull_request(owner, repo, number.get()).unwrap();
    let pr = parse_pull_request(&raw_pr, &repository, number).unwrap();
    assert_eq!(pr.head_sha, head, "PR advanced before smoke test");
    assert_eq!(pr.state, PullRequestState::Open);
    let workflow: Value =
        serde_json::from_slice(&api.workflow_run(owner, repo, &run).unwrap()).unwrap();
    assert_eq!(workflow["id"].as_u64().unwrap().to_string(), run.as_str());
    assert_eq!(workflow["run_attempt"], attempt);
    assert_eq!(workflow["repository"]["full_name"], repository.as_str());
    assert_eq!(workflow["path"], ".github/workflows/task-10-live-smoke.yml");

    let root = Path::new(&env::var("RUNNER_TEMP").unwrap())
        .join(format!("policy-ci-live-{}-{attempt}", run.as_str()));
    let trusted = root.join("trusted");
    let artifacts = root.join("artifacts");
    fs::create_dir_all(&trusted).unwrap();
    let now: chrono::DateTime<chrono::Utc> = SystemTime::now().into();
    let now: UtcTimestamp = now.to_rfc3339().parse().unwrap();
    let work_item = WorkItemObservation {
        schema_version: SCHEMA_VERSION,
        repository: repository.clone(),
        work_item_id: WorkItemId::parse("task10-live-api-smoke-not-authorized").unwrap(),
        authorized: false,
        observed_at: now.clone(),
    };
    let profile = SensorProfile {
        schema_version: SCHEMA_VERSION,
        sensors: vec![],
        adequacy: TestAdequacyPolicy {
            acceptance_criterion_ids: vec![],
            changed_behavior_ids: vec![],
            negative_case_ids: vec![],
        },
    };
    let mut policy = PolicySnapshot {
        schema_version: SCHEMA_VERSION,
        policy_digest: Sha256Digest::parse("0".repeat(64)).unwrap(),
        required_check_name: "policy-kernel/pre-merge".into(),
        max_evidence_age_seconds: 600,
        authorized_actors: vec![],
        authorized_waiver_actors: vec![],
        required_sensor_ids: vec![],
        sensor_profile_digest: Some(canonical_sha256(&profile).unwrap()),
        protected_target_branches: vec![],
        allowed_merge_methods: vec![],
    };
    policy.policy_digest = pre_merge_policy_digest(&policy).unwrap();
    let evidence = EvidenceBundle {
        schema_version: SCHEMA_VERSION,
        manifest: EvidenceManifest {
            schema_version: SCHEMA_VERSION,
            repository: repository.clone(),
            work_item_id: work_item.work_item_id.clone(),
            base_sha: pr.base_sha.clone(),
            head_sha: head.clone(),
            policy_digest: policy.policy_digest.clone(),
            source_set_digest: canonical_sha256(
                &json!({"smoke_test": "no source-set evidence collected"}),
            )
            .unwrap(),
            workflow_run_id: run.clone(),
            created_at: now,
            child_digests: vec![],
        },
        modifications: vec![],
        executions: vec![],
        reviews: vec![],
        findings: vec![],
        waivers: vec![],
        sensors: vec![],
        test_adequacy: None,
        knowledge: vec![],
    };
    let context = PreMergeContext {
        schema_version: SCHEMA_VERSION,
        sensor_profile: profile,
        changed_paths: vec![],
        risk: Risk::Low,
        workflow_run_id: run.clone(),
        agent_ready: None,
    };
    write_json(&root.join("policy.json"), &policy);
    write_json(&root.join("evidence.json"), &evidence);
    write_json(&trusted.join("context.json"), &context);
    write_json(&trusted.join("work-item.json"), &work_item);
    // This is test configuration for a deliberately unauthorized input, not a
    // production collector or a way to attest arbitrary contributor evidence.
    write_json(
        &trusted.join("identity.json"),
        &json!({
            "schema_version": SCHEMA_VERSION, "repository": repository, "pull_request_number": number,
            "head_sha": head, "workflow_run_id": run, "workflow_run_attempt": attempt,
            "workflow_trigger_head_sha": env::var("GITHUB_SHA").unwrap(),
            "workflow_head_sha": workflow["head_sha"], "workflow_id": workflow["workflow_id"],
            "workflow_path": ".github/workflows/task-10-live-smoke.yml",
            "check_app_id": 15368, "check_app_slug": "github-actions",
            "policy_digest": policy.policy_digest, "evidence_digest": canonical_sha256(&evidence).unwrap(),
            "context_digest": canonical_sha256(&context).unwrap(), "work_item_digest": canonical_sha256(&work_item).unwrap(),
        }),
    );
    let output = assert_cmd::Command::cargo_bin("policy-ci")
        .unwrap()
        .current_dir(env::var("GITHUB_WORKSPACE").unwrap())
        .args(["pre-merge", "--expected-head", head.as_str()])
        .arg("--policy")
        .arg(root.join("policy.json"))
        .arg("--evidence")
        .arg(root.join("evidence.json"))
        .arg("--trusted-observations")
        .arg(&trusted)
        .arg("--artifact-directory")
        .arg(&artifacts)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decision: PreMergeDecision = read_json(&artifacts.join("decision.json"));
    assert_eq!(decision.status, PreMergeStatus::Blocked);
    assert!(!decision.advisory);
    assert!(decision
        .reasons
        .iter()
        .any(|r| r.code == "PK-PRE-WORK-UNAUTHORIZED"));
    let receipt: PublicationReceipt = read_json(&artifacts.join("publication-receipt.json"));
    let check: Value =
        serde_json::from_slice(&api.check_run(owner, repo, receipt.check_run_id).unwrap()).unwrap();
    assert_eq!(check["status"], "completed");
    assert_eq!(check["conclusion"], "failure");
    assert_eq!(check["head_sha"], head.as_str());
    assert_eq!(check["app"]["id"], 15368);
    assert_eq!(check["name"], "policy-kernel/pre-merge");
    let binding: Value = serde_json::from_str(check["output"]["text"].as_str().unwrap()).unwrap();
    assert_eq!(
        binding["decision_digest"],
        decision.decision_digest.as_str()
    );
    assert_eq!(binding["policy_digest"], policy.policy_digest.as_str());
    assert_eq!(binding["workflow_run_id"], run.as_str());
    assert_eq!(binding["workflow_run_attempt"], attempt);
    assert!(check["output"]["text"].as_str().unwrap().len() <= 4096);
    println!(
        "Live BLOCKED publication verified: check_run_id={}, head={}, decision_digest={}",
        receipt.check_run_id, head, decision.decision_digest
    );
    println!("{}", String::from_utf8_lossy(&output.stdout));
}

fn write_json(path: &Path, value: &impl Serialize) {
    fs::write(
        path,
        serde_json::to_vec(&serde_json::to_value(value).unwrap()).unwrap(),
    )
    .unwrap();
}
fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
