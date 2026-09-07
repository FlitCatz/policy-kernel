use assert_cmd::Command;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const READY_INPUT: &str = include_str!("../../../tests/fixtures/pre-merge/ready.json");
const INCONCLUSIVE_INPUT: &str =
    include_str!("../../../tests/fixtures/pre-merge/missing-agent-ready.json");
const VALID_BUNDLE: &str = include_str!("../../../tests/fixtures/evidence/valid.json");
const TAMPERED_BUNDLE: &str = include_str!("../../../tests/fixtures/evidence/tampered.json");

fn policy_cli() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("policy-cli"))
}

fn run_stdin(args: &[&str], input: &str) -> std::process::Output {
    policy_cli()
        .args(args)
        .write_stdin(input)
        .output()
        .expect("policy-cli should run")
}

fn stdout_json(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).expect("stdout should contain one JSON value")
}

#[test]
fn evaluate_pre_forces_advisory_and_accepts_a_path() {
    let path = fixture("pre-merge/ready.json");
    let output = policy_cli()
        .args(["evaluate-pre", path.to_str().unwrap()])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let decision = stdout_json(&output);
    assert_eq!(decision["mode"], "advisory");
    assert_eq!(decision["advisory"], true);
    assert_eq!(decision["status"], "ready_to_merge");
}

#[test]
fn evaluate_pre_reads_stdin_and_maps_blocked_and_inconclusive_statuses() {
    let blocked = READY_INPUT.replacen("\"authorized\": true", "\"authorized\": false", 1);
    let blocked_output = run_stdin(&["evaluate-pre", "-"], &blocked);
    assert_eq!(blocked_output.status.code(), Some(2));
    assert_eq!(stdout_json(&blocked_output)["status"], "blocked");
    assert!(blocked_output.stderr.is_empty());

    let inconclusive_output = run_stdin(&["evaluate-pre", "-"], INCONCLUSIVE_INPUT);
    assert_eq!(inconclusive_output.status.code(), Some(3));
    assert_eq!(stdout_json(&inconclusive_output)["status"], "inconclusive");
    assert!(inconclusive_output.stderr.is_empty());
}

#[test]
fn evaluate_pre_rejects_invalid_input_without_stdout() {
    let duplicate = READY_INPUT.replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"schema_version\": 1,",
        1,
    );
    let mut unknown_field: Value = serde_json::from_str(READY_INPUT).unwrap();
    unknown_field
        .as_object_mut()
        .unwrap()
        .insert("unexpected".into(), Value::Null);
    let mut wrong_version: Value = serde_json::from_str(READY_INPUT).unwrap();
    wrong_version["schema_version"] = json!(2);
    let mut wrong_enum: Value = serde_json::from_str(READY_INPUT).unwrap();
    wrong_enum["mode"] = json!("unsafe");
    let invalid_values = [
        "not json".to_owned(),
        duplicate,
        serde_json::to_string(&unknown_field).unwrap(),
        serde_json::to_string(&wrong_version).unwrap(),
        serde_json::to_string(&wrong_enum).unwrap(),
    ];

    for invalid in invalid_values {
        let output = run_stdin(&["evaluate-pre", "-"], &invalid);
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn inspect_preserves_a_pre_merge_decision_exactly_as_json_data() {
    let supplied = json!({
        "schema_version": 1,
        "mode": "advisory",
        "advisory": true,
        "status": "inconclusive",
        "binding": {
            "repository": "FlitCatz/policy-kernel",
            "pull_request_number": 8,
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "evidence_digest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "evaluated_at": "2026-09-07T18:00:00+09:00"
        },
        "reasons": [],
        "decision_digest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    });
    let input = serde_json::to_string(&supplied).unwrap();
    let output = run_stdin(&["inspect", "-"], &input);

    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(stdout_json(&output), supplied);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("2026-09-07T18:00:00+09:00"));
}

#[test]
fn inspect_preserves_an_omitted_post_merge_optional_field() {
    let supplied = json!({
        "schema_version": 1,
        "status": "inconclusive",
        "binding": {
            "repository": "FlitCatz/policy-kernel",
            "pull_request_number": 8,
            "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
            "policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "evidence_digest": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
            "evaluated_at": "2026-09-07T09:00:00Z"
        },
        "reasons": [],
        "decision_digest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
    });
    let output = run_stdin(
        &["inspect", "-"],
        &serde_json::to_string(&supplied).unwrap(),
    );

    assert_eq!(output.status.code(), Some(0));
    let displayed = stdout_json(&output);
    assert_eq!(displayed, supplied);
    assert!(displayed.get("merge_commit_sha").is_none());
}

#[test]
fn inspect_validates_bundle_integrity() {
    let valid = run_stdin(&["inspect", "-"], VALID_BUNDLE);
    assert_eq!(valid.status.code(), Some(0));
    assert_eq!(
        stdout_json(&valid),
        serde_json::from_str::<Value>(VALID_BUNDLE).unwrap()
    );

    let tampered = run_stdin(&["inspect", "-"], TAMPERED_BUNDLE);
    assert_eq!(tampered.status.code(), Some(64));
    assert!(tampered.stdout.is_empty());
    assert!(!tampered.stderr.is_empty());
}

#[test]
fn inspect_rejects_duplicate_fields_before_display() {
    let duplicate = VALID_BUNDLE.replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"schema_version\": 1,",
        1,
    );
    let output = run_stdin(&["inspect", "-"], &duplicate);

    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[test]
fn invalid_command_and_options_use_usage_exit_code() {
    for args in [vec!["evaluate-post"], vec!["inspect", "-", "--unknown"]] {
        let output = policy_cli().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

fn fixture(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(relative)
}

#[test]
fn missing_path_is_invalid_input() {
    let path = fixture("does-not-exist.json");
    let output = policy_cli()
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
    assert!(!path.exists());
}
