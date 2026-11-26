use super::*;
use serde_json::{json, Value};
use std::fs::OpenOptions;

#[path = "../tests/support/native_expected.rs"]
mod native_expected;
#[path = "../tests/support/native_runner.rs"]
mod native_runner;

struct Fixture {
    root: PathBuf,
    _temp: tempfile::TempDir,
}

impl Fixture {
    fn new(mode: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let repo = root.join("fixture-checkout");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "--quiet"]).unwrap();
        commit(&repo);
        fs::write(root.join("runner-mode"), mode).unwrap();
        let corpus: Value = serde_json::from_str(include_str!(
            "../../policy-native-contracts/tests/corpus/v1.json"
        ))
        .unwrap();
        let mut expected = corpus["valid"]["expected"].clone();
        let mut run = corpus["valid"]["run"].clone();
        expected["subject"]["head"] = json!(revision(&repo, "HEAD").unwrap());
        run["subject"]["source"] = expected["subject"].clone();
        run["observations"][0]["subject"] = run["subject"].clone();
        let command =
            serde_json::from_value::<CommandV1>(run["observations"][0]["command"].clone()).unwrap();
        expected["bindings"]["observations"][0]["command_digest"] =
            json!(canonical_digest(DigestRole::Command, &command).unwrap());
        native_expected::prepare(&mut expected, &mut run);
        fs::write(
            root.join("expected.json"),
            serde_json::to_vec(&expected).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("run-template.json"),
            serde_json::to_vec(&run).unwrap(),
        )
        .unwrap();
        Self { root, _temp: temp }
    }

    fn args(&self) -> Args {
        Args {
            repo: self.root.join("fixture-checkout"),
            base: "HEAD".into(),
            expected: self.root.join("expected.json"),
            run: self.root.join("run.json"),
            artifacts: self.root.join("raw"),
            output: self.root.join("out"),
            run_id: ExecutionId::parse("run-1").unwrap(),
            scope_id: RecordId::parse("scope-1").unwrap(),
            runner_id: RecordId::parse("fixture").unwrap(),
            signer: Some(PathBuf::from("/usr/bin/touch")),
            signer_arg: vec![self.root.join("signer-invoked").into_os_string()],
            signing_key: None,
            gate_policy: None,
            pr: None,
            ssh_keygen: None,
            runner: vec![
                std::env::current_exe().unwrap().into_os_string(),
                "--exact".into(),
                "attest::tests::runner_child".into(),
            ],
        }
    }

    fn reject(&self, message: &str) {
        let error = attest(self.args(), |_| panic!("rejected run must never prompt"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(message),
            "expected {message:?}, got {error:?}\n{}",
            self.import_log()
        );
        self.assert_unsigned();
    }

    fn assert_unsigned(&self) {
        assert!(!self.root.join("signer-invoked").exists());
        assert!(!self.root.join("out").join(ENVELOPE).exists());
    }

    fn import_log(&self) -> String {
        fs::read_to_string(self.root.join("out/import.json")).unwrap_or_default()
    }
}

fn commit(repo: &Path) {
    git(
        repo,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "fixture",
        ],
    )
    .unwrap();
}

#[test]
fn runner_child() {
    let repo = std::env::current_dir().unwrap();
    if !repo.ends_with("fixture-checkout") {
        return;
    }
    let root = repo.parent().unwrap();
    let mode = fs::read_to_string(root.join("runner-mode")).unwrap();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("ran-once"))
        .unwrap();
    match mode.as_str() {
        "abnormal" => std::process::exit(17),
        "change-head" => commit(&repo),
        _ => native_runner::emit(root, &mode, |bytes| raw_digest(bytes).sha256.to_string()),
    }
}

#[test]
fn dirty_tree_rejected_before_runner() {
    let fixture = Fixture::new("pass");
    fs::write(fixture.args().repo.join("untracked"), "dirty").unwrap();
    fixture.reject("Git worktree is dirty");
    assert!(!fixture.root.join("ran-once").exists());
}

fn gate_args(fixture: &Fixture, recorded_key: &str) -> Args {
    let keys: Value =
        serde_json::from_str(include_str!("../tests/fixtures/openssh-handles.json")).unwrap();
    let handle = fixture.root.join("signing-handle");
    fs::write(
        &handle,
        format!(
            "-----BEGIN OPENSSH PRIVATE KEY-----\n{}\n-----END OPENSSH PRIVATE KEY-----\n",
            keys[recorded_key].as_str().unwrap()
        ),
    )
    .unwrap();
    let mut args = fixture.args();
    args.signer = None;
    args.signer_arg.clear();
    args.signing_key = Some(handle);
    args.pr = Some(42);
    args.gate_policy = Some("gate.json".into());
    args.ssh_keygen = Some("/usr/bin/false".into());
    args
}

#[test]
fn gate_rejects_software_and_no_touch_handles_before_runner_or_confirmation() {
    for (key, reason) in [
        ("software", "OpenSSH hardware security key"),
        ("no_touch", "no-touch-required keys are refused"),
    ] {
        let fixture = Fixture::new("pass");
        let error =
            attest(gate_args(&fixture, key), |_| panic!("invalid key prompted")).unwrap_err();
        assert!(error.to_string().contains(reason), "{error}");
        assert!(!fixture.root.join("ran-once").exists());
        assert!(!fixture.args().output.exists());
    }
}

#[test]
fn gate_confirms_the_pr_and_base_policy_once_before_invoking_the_signer() {
    let fixture = Fixture::new("pass");
    let mut args = gate_args(&fixture, "touch");
    let base_policy = b"{\"policy\":\"base\"}\n";
    fs::write(args.repo.join("gate.json"), base_policy).unwrap();
    git(&args.repo, &["add", "gate.json"]).unwrap();
    commit(&args.repo);
    args.base = revision(&args.repo, "HEAD").unwrap().to_string();
    fs::write(args.repo.join("gate.json"), b"{\"policy\":\"head\"}\n").unwrap();
    git(&args.repo, &["add", "gate.json"]).unwrap();
    commit(&args.repo);
    let head = revision(&args.repo, "HEAD").unwrap();
    let mut expected: Value = serde_json::from_slice(&fs::read(&args.expected).unwrap()).unwrap();
    let run_template = fixture.root.join("run-template.json");
    let mut run: Value = serde_json::from_slice(&fs::read(&run_template).unwrap()).unwrap();
    expected["subject"]["head"] = json!(head);
    run["subject"]["source"] = expected["subject"].clone();
    run["observations"][0]["subject"] = run["subject"].clone();
    fs::write(&args.expected, serde_json::to_vec(&expected).unwrap()).unwrap();
    fs::write(run_template, serde_json::to_vec(&run).unwrap()).unwrap();
    let outcome = attest(args, |preview| {
        assert!(preview.contains(&format!(
            "PR 42, base policy SHA-256 {}",
            raw_digest(base_policy).sha256
        )));
        assert!(preview.contains(head.as_str()));
        assert!(preview.contains("Statement SHA-256"));
        false
    })
    .unwrap();
    assert_eq!(outcome, Outcome::Unconfirmed);
    assert!(!fixture.root.join("out/gate-attestation.json").exists());
}

#[test]
fn edits_hidden_by_index_flags_are_rejected_before_runner() {
    for flag in ["--assume-unchanged", "--skip-worktree"] {
        let fixture = Fixture::new("pass");
        let repo = fixture.args().repo;
        fs::write(repo.join("tracked"), "committed").unwrap();
        git(&repo, &["add", "tracked"]).unwrap();
        commit(&repo);
        git(&repo, &["update-index", flag, "tracked"]).unwrap();
        fs::write(repo.join("tracked"), "edited").unwrap();
        assert!(
            git(&repo, &["status", "--porcelain"]).unwrap().is_empty(),
            "{flag} must hide the edit from git status"
        );
        fixture.reject("Git worktree is dirty");
        assert!(!fixture.root.join("ran-once").exists());
    }
}

#[test]
fn runner_cannot_change_head_even_if_the_tree_is_unchanged() {
    let fixture = Fixture::new("change-head");
    fixture.reject("Git head or tree changed");
    assert!(fixture.root.join("ran-once").exists());
}

#[test]
fn observation_before_start_is_rejected() {
    Fixture::new("early").reject("observation time is outside the runner interval");
}

#[test]
fn failed_runner_and_nonpass_scope_are_unsigned() {
    Fixture::new("abnormal").reject("runner failed");
    Fixture::new("nonpass").reject("scope result is NonPass");
}

#[test]
fn signer_failure_preserves_import_without_an_envelope() {
    let fixture = Fixture::new("pass");
    let mut args = fixture.args();
    args.signer = Some("/usr/bin/false".into());
    args.signer_arg.clear();
    let error = attest(args, |_| true).unwrap_err().to_string();
    assert!(
        error.contains("signer failed"),
        "{error}\n{}",
        fixture.import_log()
    );
    assert!(fixture.root.join("out/observations.json").exists());
    fixture.assert_unsigned();
}

#[test]
fn an_existing_output_is_never_replaced() {
    let fixture = Fixture::new("pass");
    let args = fixture.args();
    fs::create_dir(&args.output).unwrap();
    let envelope = args.output.join(ENVELOPE);
    fs::write(&envelope, "existing envelope").unwrap();
    let error = attest(args, |_| panic!("must reject existing output")).unwrap_err();
    assert!(error.to_string().contains("already exists"));
    assert_eq!(fs::read_to_string(envelope).unwrap(), "existing envelope");
    assert!(!fixture.root.join("ran-once").exists());
}

#[test]
fn declining_owner_confirmation_does_not_call_the_signer() {
    let fixture = Fixture::new("pass");
    let outcome = attest(fixture.args(), |summary| {
        assert!(summary.contains("Local scope Satisfied"));
        assert!(summary.contains("Statement SHA-256 (signing bytes)"));
        assert!(!fixture.root.join("signer-invoked").exists());
        false
    })
    .unwrap_or_else(|error| panic!("{error}\n{}", fixture.import_log()));
    assert_eq!(outcome, Outcome::Unconfirmed);
    fixture.assert_unsigned();
}

#[test]
fn process_only_scope_is_refused_before_the_runner() {
    let fixture = Fixture::new("pass");
    let path = fixture.root.join("expected.json");
    let mut expected: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let check = expected["requirements"]["observations"][0]["id"].clone();
    expected["requirements"]["observations"][0]["tests"] = Value::Null;
    expected["bindings"]["observations"][0]["parser"] = Value::Null;
    expected["requirements"]["requirements"] =
        json!([{"id": "formatting", "process_checks": [check]}]);
    serde_json::from_value::<ExpectedScopeV1>(expected.clone())
        .unwrap()
        .validate()
        .expect("fixture is a valid process-only scope");
    fs::write(&path, serde_json::to_vec(&expected).unwrap()).unwrap();
    fixture.reject("local attestation of process-only scopes is not supported yet");
    assert!(!fixture.root.join("ran-once").exists());
}
