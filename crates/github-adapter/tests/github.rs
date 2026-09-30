use github_adapter::{
    parse_merge, parse_pull_request, parse_trusted_check, serialize_check_binding,
    validate_merge_protection, AdapterError, CheckBinding, CheckExpectation, GhCliApi, GitHubApi,
    PublicationReceipt,
};
use policy_contracts::{
    CommitSha, MergeMethod, PullRequestNumber, RepositoryId, Sha256Digest, WorkflowRunId,
};
use std::{fs, os::unix::fs::PermissionsExt, time::Duration};

fn process_fixture(path: &std::path::Path, mode: &str) {
    static EXE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
    let executable = EXE.get_or_init(|| {
        let output = std::env::temp_dir().join(format!("adapter-fixture-{}", std::process::id()));
        let result =
            std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .args(["--edition=2021", "tests/support/process.rs", "-o"])
                .arg(&output)
                .output()
                .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        output
    });
    if !path.exists() {
        fs::copy(executable, path).unwrap();
    }
    fs::write(path.with_extension("mode"), mode).unwrap();
}

const HEAD: &str = "cccccccccccccccccccccccccccccccccccccccc";
const MERGE: &str = "dddddddddddddddddddddddddddddddddddddddd";
const DECISION: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const POLICY: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

fn repository() -> RepositoryId {
    RepositoryId::parse("FlitCatz/policy-kernel").unwrap()
}

fn receipt() -> PublicationReceipt {
    serde_json::from_str(include_str!(
        "../../../tests/fixtures/github/publication-receipt.json"
    ))
    .unwrap()
}

fn expectation() -> CheckExpectation {
    CheckExpectation {
        repository: repository(),
        pull_request_number: PullRequestNumber::new(42).unwrap(),
        head_sha: CommitSha::parse(HEAD).unwrap(),
        check_name: "policy-kernel/pre-merge".into(),
        decision_digest: Sha256Digest::parse(DECISION).unwrap(),
        policy_digest: Sha256Digest::parse(POLICY).unwrap(),
    }
}

#[test]
fn squash_merge_preserves_pr_head_and_merge_commit_separately() {
    let observation = parse_merge(
        include_bytes!("../../../tests/fixtures/github/pr-merged.json"),
        include_bytes!("../../../tests/fixtures/github/merge-receipt.json"),
        &repository(),
        PullRequestNumber::new(42).unwrap(),
    )
    .unwrap();

    assert_eq!(observation.head_sha.as_str(), HEAD);
    assert_eq!(observation.merge_commit_sha.as_str(), MERGE);
    assert_ne!(observation.head_sha, observation.merge_commit_sha);
    assert_eq!(observation.method, MergeMethod::Squash);
    assert!(observation.protected);
}

#[test]
fn pull_request_is_bound_to_expected_repository_number_and_head() {
    let raw = include_bytes!("../../../tests/fixtures/github/pr-open.json");
    let parsed =
        parse_pull_request(raw, &repository(), PullRequestNumber::new(42).unwrap()).unwrap();
    assert_eq!(parsed.head_sha.as_str(), HEAD);

    let wrong = RepositoryId::parse("attacker/repository").unwrap();
    assert!(matches!(
        parse_pull_request(raw, &wrong, PullRequestNumber::new(42).unwrap()),
        Err(AdapterError::Binding(_))
    ));
}

#[test]
fn arbitrary_check_json_cannot_claim_trusted_provenance() {
    let result = parse_trusted_check(
        include_bytes!("../../../tests/fixtures/github/check-forged.json"),
        include_bytes!("../../../tests/fixtures/github/workflow-run.json"),
        &receipt(),
        &expectation(),
    );
    assert!(matches!(result, Err(AdapterError::Binding(_))));
}

#[test]
fn trusted_check_requires_exact_receipt_workflow_head_and_digest_bindings() {
    let check = include_bytes!("../../../tests/fixtures/github/check-success.json");
    let workflow = include_bytes!("../../../tests/fixtures/github/workflow-run.json");
    let parsed = parse_trusted_check(check, workflow, &receipt(), &expectation()).unwrap();
    assert_eq!(parsed.check_run_id, 6001);
    assert_eq!(parsed.workflow_run_attempt, 2);
    assert_eq!(parsed.observation.decision_digest.as_str(), DECISION);

    let mut wrong_attempt = receipt();
    wrong_attempt.workflow_run_attempt = 3;
    assert!(parse_trusted_check(check, workflow, &wrong_attempt, &expectation()).is_err());

    let mut wrong_head = expectation();
    wrong_head.head_sha = CommitSha::parse("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa").unwrap();
    assert!(parse_trusted_check(check, workflow, &receipt(), &wrong_head).is_err());
}

#[test]
fn check_run_binding_payload_stays_bounded_and_rejects_oversized_fields() {
    let binding = CheckBinding {
        schema_version: 1,
        repository: repository(),
        pull_request_number: PullRequestNumber::new(42).unwrap(),
        head_sha: CommitSha::parse(HEAD).unwrap(),
        policy_digest: Sha256Digest::parse(POLICY).unwrap(),
        decision_digest: Sha256Digest::parse(DECISION).unwrap(),
        workflow_run_id: WorkflowRunId::parse("9001").unwrap(),
        workflow_run_attempt: 2,
        completed_at: "2026-09-07T08:59:00Z".parse().unwrap(),
    };
    let payload = serialize_check_binding(&binding).unwrap();
    assert!(payload.len() <= 4_096);

    let mut oversized = binding;
    oversized.workflow_run_id = WorkflowRunId::parse("x".repeat(4_096)).unwrap();
    assert!(matches!(
        serialize_check_binding(&oversized),
        Err(AdapterError::PayloadTooLarge { .. })
    ));
}

#[test]
fn gh_cli_rejects_route_injection_before_process_execution() {
    let api = GhCliApi::new("/path/that/does/not/exist", Duration::from_secs(1), 4096);
    let error = api
        .pull_request("owner", "repo/../../attacker", 42)
        .unwrap_err();
    assert!(matches!(error, AdapterError::InvalidRouteSegment(_)));
}

#[test]
fn gh_cli_uses_only_the_fixed_pull_request_route() {
    let temp = std::env::temp_dir().join(format!("policy-kernel-fake-gh-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let script = temp.join("gh");
    process_fixture(&script, "args");
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&script, permissions).unwrap();

    let api = GhCliApi::new(&script, Duration::from_secs(1), 4096);
    let output = api.pull_request("FlitCatz", "policy-kernel", 42).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(
        text,
        "api\n--hostname\ngithub.com\n--method\nGET\n/repos/FlitCatz/policy-kernel/pulls/42\n{}"
    );
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn parser_rejects_oversized_binding_and_zero_check_identity() {
    let raw = include_bytes!("../../../tests/fixtures/github/check-success.json");
    let workflow = include_bytes!("../../../tests/fixtures/github/workflow-run.json");
    let mut check: serde_json::Value = serde_json::from_slice(raw).unwrap();
    check["output"]["text"] = serde_json::json!(format!(
        "{}{}",
        " ".repeat(4096),
        check["output"]["text"].as_str().unwrap()
    ));
    assert!(matches!(
        parse_trusted_check(
            &serde_json::to_vec(&check).unwrap(),
            workflow,
            &receipt(),
            &expectation()
        ),
        Err(AdapterError::PayloadTooLarge { .. })
    ));
    let mut check: serde_json::Value = serde_json::from_slice(raw).unwrap();
    check["id"] = serde_json::json!(0);
    let mut trusted = receipt();
    trusted.check_run_id = 0;
    assert!(parse_trusted_check(
        &serde_json::to_vec(&check).unwrap(),
        workflow,
        &trusted,
        &expectation()
    )
    .is_err());
}

#[test]
fn gh_rejects_zero_ids_dot_branches_and_large_request_before_execution() {
    let api = GhCliApi::new("/does/not/exist", Duration::from_secs(1), 4096);
    assert!(matches!(
        api.issue("owner", "repo", 0),
        Err(AdapterError::InvalidRouteSegment(_))
    ));
    assert!(matches!(
        api.workflow_run("owner", "repo", &WorkflowRunId::parse("00").unwrap()),
        Err(AdapterError::InvalidRouteSegment(_))
    ));
    assert!(matches!(
        api.branch_protection("owner", "repo", ".."),
        Err(AdapterError::InvalidRouteSegment(_))
    ));
    assert!(matches!(
        api.create_check_run("owner", "repo", &vec![b'x'; 16 * 1024 + 1]),
        Err(AdapterError::PayloadTooLarge { .. })
    ));
}

#[test]
fn inherited_output_pipe_does_not_extend_gh_timeout() {
    let temp = std::env::temp_dir().join(format!("policy-gh-inherited-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let script = temp.join("gh");
    process_fixture(&script, "inherited");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let api = GhCliApi::new(&script, Duration::from_millis(100), 4096);
    let started = std::time::Instant::now();
    assert!(matches!(
        api.pull_request("owner", "repo", 1),
        Err(AdapterError::Timeout(_))
    ));
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(script.with_extension("started").exists());
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        !script.with_extension("survived").exists(),
        "descendant survived API timeout"
    );
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn gh_fails_closed_on_output_overflow_and_nonzero_exit() {
    let temp = std::env::temp_dir().join(format!("policy-gh-failure-{}", std::process::id()));
    fs::create_dir_all(&temp).unwrap();
    let script = temp.join("gh");
    process_fixture(&script, "overflow");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
    let api = GhCliApi::new(&script, Duration::from_secs(1), 4);
    assert!(matches!(
        api.pull_request("owner", "repo", 1),
        Err(AdapterError::OutputTooLarge { limit: 4 })
    ));
    process_fixture(&script, "failure");
    assert!(matches!(
        api.pull_request("owner", "repo", 1),
        Err(AdapterError::ProcessFailure {
            status: Some(7),
            ..
        })
    ));
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn gh_cleans_descendants_on_every_process_exit_path() {
    for mode in [
        "success-descendant",
        "failure-descendant",
        "overflow-descendant",
        "blocked-input",
    ] {
        let temp =
            std::env::temp_dir().join(format!("policy-gh-cleanup-{mode}-{}", std::process::id()));
        fs::create_dir_all(&temp).unwrap();
        let executable = temp.join("gh");
        process_fixture(&executable, mode);
        let api = GhCliApi::new(&executable, Duration::from_millis(200), 4);
        let result = if mode == "blocked-input" {
            api.create_check_run("owner", "repo", &vec![b'x'; 16 * 1024])
        } else {
            api.pull_request("owner", "repo", 1)
        };
        match mode {
            "success-descendant" => assert_eq!(result.unwrap(), b"{}"),
            "failure-descendant" => assert!(matches!(
                result,
                Err(AdapterError::ProcessFailure {
                    status: Some(7),
                    ..
                })
            )),
            "overflow-descendant" => assert!(matches!(
                result,
                Err(AdapterError::OutputTooLarge { limit: 4 })
            )),
            "blocked-input" => assert!(matches!(result, Err(AdapterError::Timeout(_)))),
            _ => unreachable!(),
        }
        assert!(executable.with_extension("started").exists());
        std::thread::sleep(Duration::from_millis(500));
        assert!(
            !executable.with_extension("survived").exists(),
            "descendant survived {mode}"
        );
        fs::remove_dir_all(temp).unwrap();
    }
}

#[test]
fn merge_method_is_taken_from_trusted_receipt_and_missing_queue_fails_closed() {
    let raw = include_bytes!("../../../tests/fixtures/github/pr-merged.json");
    let mut supplemental: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/github/merge-receipt.json"
    ))
    .unwrap();
    supplemental["method"] = serde_json::json!("merge");
    let observed = parse_merge(
        raw,
        &serde_json::to_vec(&supplemental).unwrap(),
        &repository(),
        PullRequestNumber::new(42).unwrap(),
    )
    .unwrap();
    assert_eq!(observed.method, MergeMethod::Merge);
    supplemental
        .as_object_mut()
        .unwrap()
        .remove("via_merge_queue");
    assert!(parse_merge(
        raw,
        &serde_json::to_vec(&supplemental).unwrap(),
        &repository(),
        PullRequestNumber::new(42).unwrap()
    )
    .is_err());
}

#[test]
fn label_event_cannot_be_rebound_to_a_new_head_or_actor() {
    use github_adapter::{parse_agent_ready, AgentReadyExpectation, LabelReceipt};
    use policy_contracts::{TrustClass, WorkItemId};
    let event = serde_json::json!({"id":123, "event":"labeled", "created_at":"2026-09-07T08:58:00Z", "actor":{"login":"release-owner", "type":"User"}, "label":{"name":"agent-ready"}});
    let receipt = LabelReceipt {
        schema_version: 1,
        repository: repository(),
        work_item_id: WorkItemId::parse("task-10").unwrap(),
        pull_request_number: PullRequestNumber::new(42).unwrap(),
        head_sha: CommitSha::parse(HEAD).unwrap(),
        event_id: 123,
        label: "agent-ready".into(),
        actor: "release-owner".into(),
        observed_at: "2026-09-07T08:58:00Z".parse().unwrap(),
    };
    let mut expected = AgentReadyExpectation {
        repository: repository(),
        work_item_id: receipt.work_item_id.clone(),
        pull_request_number: receipt.pull_request_number,
        head_sha: receipt.head_sha.clone(),
        label: "agent-ready".into(),
    };
    let raw = serde_json::to_vec(&event).unwrap();
    let observation = parse_agent_ready(&raw, &receipt, &expected).unwrap();
    assert_eq!(observation.trust, TrustClass::TrustedRunnerAttested);
    assert_eq!(observation.actor, "release-owner");
    expected.head_sha = CommitSha::parse(MERGE).unwrap();
    assert!(parse_agent_ready(&raw, &receipt, &expected).is_err());
    expected.head_sha = receipt.head_sha.clone();
    let mut event = event;
    event["actor"]["login"] = serde_json::json!("someone-else");
    assert!(parse_agent_ready(&serde_json::to_vec(&event).unwrap(), &receipt, &expected).is_err());
}

#[test]
fn issue_number_is_independent_from_work_item_identity() {
    use github_adapter::{parse_work_item, WorkItemExpectation};
    use policy_contracts::{WorkItemId, WorkItemObservation};
    let mut expected = WorkItemExpectation {
        repository: repository(),
        issue_number: 42,
        work_item_id: WorkItemId::parse("task-10").unwrap(),
    };
    let receipt = WorkItemObservation {
        schema_version: 1,
        repository: repository(),
        work_item_id: expected.work_item_id.clone(),
        authorized: false,
        observed_at: "2026-09-07T08:58:00Z".parse().unwrap(),
    };
    let raw = include_bytes!("../../../tests/fixtures/github/issue.json");
    let observation = parse_work_item(raw, &receipt, &expected).unwrap();
    assert_eq!(observation.work_item_id.as_str(), "task-10");
    assert!(!observation.authorized);
    expected.issue_number = 10;
    assert!(parse_work_item(raw, &receipt, &expected).is_err());
}

#[test]
fn maximum_github_identity_binding_fits_four_kibibytes() {
    let binding = CheckBinding {
        schema_version: 1,
        repository: RepositoryId::parse(format!("{}/{}", "a".repeat(39), "b".repeat(100))).unwrap(),
        pull_request_number: PullRequestNumber::new(u64::MAX).unwrap(),
        head_sha: CommitSha::parse(HEAD).unwrap(),
        policy_digest: Sha256Digest::parse(POLICY).unwrap(),
        decision_digest: Sha256Digest::parse(DECISION).unwrap(),
        workflow_run_id: WorkflowRunId::parse(u64::MAX.to_string()).unwrap(),
        workflow_run_attempt: u64::MAX,
        completed_at: "9999-12-31T23:59:59Z".parse().unwrap(),
    };
    let payload = serialize_check_binding(&binding).unwrap();
    assert!(payload.len() <= 4096);
    let value: serde_json::Value = serde_json::from_slice(&payload).unwrap();
    assert_eq!(
        payload,
        serde_json::to_vec(&value).unwrap(),
        "binding must be canonical JSON"
    );
}

#[test]
fn merge_protection_claim_must_match_the_branch_observation() {
    let receipt: policy_contracts::GitHubMergeObservation = serde_json::from_str(include_str!(
        "../../../tests/fixtures/github/merge-receipt.json"
    ))
    .unwrap();
    let branch = |name: &str, protected: bool| {
        serde_json::to_vec(&serde_json::json!({"name": name, "protected": protected})).unwrap()
    };
    validate_merge_protection(&receipt, &branch("main", true)).unwrap();
    // A receipt cannot claim protection GitHub does not observe, or the reverse.
    assert!(validate_merge_protection(&receipt, &branch("main", false)).is_err());
    let mut unprotected = receipt.clone();
    unprotected.protected = false;
    validate_merge_protection(&unprotected, &branch("main", false)).unwrap();
    assert!(validate_merge_protection(&unprotected, &branch("main", true)).is_err());
    // The observation must describe the merge target.
    assert!(validate_merge_protection(&receipt, &branch("release", true)).is_err());
    let mut untargeted = receipt;
    untargeted.target_branch = None;
    assert!(validate_merge_protection(&untargeted, &branch("main", true)).is_err());
    assert!(validate_merge_protection(&unprotected, b"{\"name\":\"main\"}").is_err());
}
