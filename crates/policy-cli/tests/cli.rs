use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

fn workspace_path(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path)
}

fn run(args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = Command::new(cargo_bin!("policy-cli"));
    command.args(args);
    if let Some(input) = stdin {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.output().unwrap()
    }
}

fn temporary_input(value: &Value) -> PathBuf {
    let id = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("policy-cli-{}-{id}.json", std::process::id()));
    fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    path
}

#[test]
fn evaluate_pre_forces_authoritative_input_to_advisory_ready() {
    let path = workspace_path("tests/fixtures/pre-merge/ready.json");
    let output = run(&["evaluate-pre", path.to_str().unwrap()], None);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let decision: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(decision["mode"], "advisory");
    assert_eq!(decision["advisory"], true);
    assert_eq!(decision["status"], "ready_to_merge");
}

#[test]
fn evaluate_pre_returns_blocked_exit_code() {
    let mut value: Value = serde_json::from_str(
        &fs::read_to_string(workspace_path("tests/fixtures/pre-merge/ready.json")).unwrap(),
    )
    .unwrap();
    value["work_item"]["authorized"] = json!(false);
    let path = temporary_input(&value);

    let output = run(&["evaluate-pre", path.to_str().unwrap()], None);

    fs::remove_file(path).unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
        "blocked"
    );
}

#[test]
fn evaluate_pre_returns_inconclusive_exit_code() {
    let path = workspace_path("tests/fixtures/pre-merge/head-mismatch.json");
    let output = run(&["evaluate-pre", path.to_str().unwrap()], None);

    assert_eq!(output.status.code(), Some(3));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["status"],
        "inconclusive"
    );
}

#[test]
fn evaluate_pre_reads_stdin_when_path_is_dash() {
    let input = fs::read_to_string(workspace_path("tests/fixtures/pre-merge/ready.json")).unwrap();
    let output = run(&["evaluate-pre", "-"], Some(&input));

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["advisory"],
        true
    );
}

#[test]
fn invalid_input_uses_data_error_exit_and_stderr_only() {
    let output = run(&["evaluate-pre", "-"], Some("{not-json}"));

    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("invalid input"));
}

#[test]
fn inspect_preserves_pre_merge_decision_json_value() {
    let input = fs::read_to_string(workspace_path(
        "tests/fixtures/contracts/advisory-decision.json",
    ))
    .unwrap();
    let expected: Value = serde_json::from_str(&input).unwrap();
    let output = run(&["inspect", "-"], Some(&input));

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

#[test]
fn inspect_accepts_evidence_bundle_without_evaluating_it() {
    let path = workspace_path("tests/fixtures/evidence/valid.json");
    let expected: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    let output = run(&["inspect", path.to_str().unwrap()], None);

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

#[test]
fn inspect_accepts_post_merge_decision_without_changing_authority_metadata() {
    let value = json!({
        "schema_version": 1,
        "status": "shipped",
        "binding": {
            "repository": "FlitCatz/policy-kernel",
            "pull_request_number": 8,
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "evidence_digest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "evaluated_at": "2026-09-08T00:00:00Z"
        },
        "merge_commit_sha": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "reasons": [],
        "decision_digest": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    });
    let output = run(&["inspect", "-"], Some(&value.to_string()));

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        value
    );
}

#[test]
fn inspect_rejects_unknown_fields_and_versions() {
    let valid: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/contracts/advisory-decision.json"
    ))
    .unwrap();
    let mut future = valid.clone();
    future["schema_version"] = json!(99);
    let mut unknown = valid;
    unknown["unexpected"] = json!(true);
    for input in [future.to_string(), unknown.to_string()] {
        let output = run(&["inspect", "-"], Some(&input));
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("invalid input"));
    }
}

#[test]
fn inspect_rejects_duplicate_root_and_nested_contract_fields() {
    let valid = include_str!("../../../tests/fixtures/contracts/advisory-decision.json");
    for duplicate in [
        valid.replacen(
            r#""schema_version":1"#,
            r#""schema_version":99,"schema_version":1"#,
            1,
        ),
        valid.replacen(
            r#""advisory":true"#,
            r#""advisory":false,"advisory":true"#,
            1,
        ),
        valid.replacen(
            r#""head_sha":"cccccccccccccccccccccccccccccccccccccccc""#,
            r#""head_sha":"deadbeef","head_sha":"cccccccccccccccccccccccccccccccccccccccc""#,
            1,
        ),
    ] {
        let output = run(&["inspect", "-"], Some(&duplicate));
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn local_post_merge_evaluation_is_not_exposed() {
    let output = run(&["evaluate-post", "-"], Some("{}"));

    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("evaluate-post"));
}
