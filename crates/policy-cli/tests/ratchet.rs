use assert_cmd::Command;
use serde_json::{json, Value};
use std::{fs, path::Path, process::Output};

fn entry(count: u64) -> Value {
    json!({
        "key": {
            "tool": "analyzer",
            "rule": "R1",
            "path": "src/lib.rs",
            "symbol": "compute",
            "context_hash": "a".repeat(64)
        },
        "count": count
    })
}

fn inputs() -> (Value, Value) {
    (
        json!({"schema_version": 1, "approved_baseline": [entry(2)], "head_baseline": [entry(1)]}),
        json!({"schema_version": 1, "entries": [entry(1)]}),
    )
}

fn command(rule: &Path, observation: &Path) -> Command {
    let mut command = Command::new(assert_cmd::cargo::cargo_bin!("policy-cli"));
    command
        .args(["evaluate-ratchet", "--rule"])
        .arg(rule)
        .arg("--observation")
        .arg(observation);
    command
}

fn run(rule: &[u8], observation: &[u8]) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let rule_path = temp.path().join("rule.json");
    let observation_path = temp.path().join("observation.json");
    fs::write(&rule_path, rule).unwrap();
    fs::write(&observation_path, observation).unwrap();
    command(&rule_path, &observation_path).output().unwrap()
}

fn assert_result(output: &Output, exit: i32, status: &str) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["schema_version"], 1);
    assert_eq!(result["status"], status);
    assert_eq!(output.stdout.last(), Some(&b'\n'));
    result
}

#[test]
fn ratchet_pass_prints_the_verifier_result_deterministically() {
    let (rule, observation) = inputs();
    for (rule, observation) in [
        (rule, observation),
        (
            json!({"schema_version": 1, "approved_baseline": [], "head_baseline": []}),
            json!({"schema_version": 1, "entries": []}),
        ),
    ] {
        let rule = serde_json::to_vec(&rule).unwrap();
        let observation = serde_json::to_vec(&observation).unwrap();
        let first = run(&rule, &observation);
        let result = assert_result(&first, 0, "pass");
        assert_eq!(result["violations"], json!([]));
        // The existing verifier owns the complete wire result, including digests.
        let expected = policy_native_verifier::evaluate_ratchet_json(&rule, &observation).unwrap();
        let mut expected_bytes = serde_json::to_vec(&expected).unwrap();
        expected_bytes.push(b'\n');
        assert_eq!(first.stdout, expected_bytes);
        assert_eq!(run(&rule, &observation).stdout, first.stdout);
    }
}

#[test]
fn ratchet_comparison_failures_preserve_each_reason_and_digests() {
    for code in [
        "ALLOWANCE_INCREASED",
        "DEBT_INCREASED",
        "CONTEXT_REPLACEMENT",
        "NEW_DEBT",
    ] {
        let (mut rule, mut observation) = inputs();
        let input = match code {
            "ALLOWANCE_INCREASED" => {
                rule["head_baseline"][0]["count"] = json!(3);
                "head_baseline"
            }
            "DEBT_INCREASED" => {
                observation["entries"][0]["count"] = json!(2);
                "observation"
            }
            "CONTEXT_REPLACEMENT" => {
                observation["entries"][0]["key"]["context_hash"] = json!("b".repeat(64));
                "observation"
            }
            _ => {
                observation["entries"][0]["key"]["symbol"] = json!("new_symbol");
                "observation"
            }
        };
        let output = run(
            &serde_json::to_vec(&rule).unwrap(),
            &serde_json::to_vec(&observation).unwrap(),
        );
        let result = assert_result(&output, 2, "fail");
        let key = if input == "head_baseline" {
            &rule["head_baseline"][0]["key"]
        } else {
            &observation["entries"][0]["key"]
        };
        assert_eq!(
            result["violations"],
            json!([{"input": input, "key": key, "code": code}])
        );
        assert_eq!(result["rule_digest"]["role"], "ratchet_rule");
        assert_eq!(result["observation_digest"]["role"], "ratchet_observation");
    }
}

#[test]
fn ratchet_structural_failures_are_invalid_input_with_json_reasons() {
    for code in ["INVALID_COUNT", "INVALID_PATH", "DUPLICATE_KEY"] {
        for input in ["approved_baseline", "head_baseline", "observation"] {
            let (mut rule, mut observation) = inputs();
            let entries = if input == "observation" {
                &mut observation["entries"]
            } else {
                &mut rule[input]
            };
            match code {
                "INVALID_COUNT" => entries[0]["count"] = json!(0),
                "INVALID_PATH" => entries[0]["key"]["path"] = json!("../outside.rs"),
                _ => {
                    let duplicate = entries[0].clone();
                    entries.as_array_mut().unwrap().push(duplicate);
                }
            }
            let key = entries[0]["key"].clone();
            let output = run(
                &serde_json::to_vec(&rule).unwrap(),
                &serde_json::to_vec(&observation).unwrap(),
            );
            let result = assert_result(&output, 64, "fail");
            assert_eq!(
                result["violations"],
                json!([{"input": input, "key": key, "code": code}])
            );
            assert_eq!(result["rule_digest"], Value::Null);
            assert_eq!(result["observation_digest"], Value::Null);
        }
    }
}

#[test]
fn ratchet_malformed_documents_preserve_stable_verifier_diagnostics() {
    let (rule, observation) = inputs();
    let rule = serde_json::to_vec(&rule).unwrap();
    let observation = serde_json::to_vec(&observation).unwrap();
    for (rule, observation, invalid_inputs) in [
        (b"not json".as_slice(), observation.as_slice(), vec!["rule"]),
        (rule.as_slice(), b"{}".as_slice(), vec!["observation"]),
        (
            b"{\"schema_version\":1,\"schema_version\":1}".as_slice(),
            b"null".as_slice(),
            vec!["rule", "observation"],
        ),
    ] {
        let output = run(rule, observation);
        let result = assert_result(&output, 64, "fail");
        let violations: Vec<_> = invalid_inputs
            .into_iter()
            .map(|input| {
                json!({
                    "input": input, "key": null, "code": "MALFORMED_INPUT"
                })
            })
            .collect();
        assert_eq!(result["violations"], json!(violations));
        assert_eq!(result["rule_digest"], Value::Null);
        assert_eq!(result["observation_digest"], Value::Null);
    }
}

#[test]
fn ratchet_accepts_either_input_from_stdin_but_rejects_two_stdin_inputs() {
    let (rule, observation) = inputs();
    let temp = tempfile::tempdir().unwrap();
    for (stdin_rule, value) in [(true, &observation), (false, &rule)] {
        let path = temp.path().join("input.json");
        fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
        let (rule_path, observation_path, stdin) = if stdin_rule {
            (Path::new("-"), path.as_path(), &rule)
        } else {
            (path.as_path(), Path::new("-"), &observation)
        };
        let output = command(rule_path, observation_path)
            .write_stdin(serde_json::to_vec(stdin).unwrap())
            .output()
            .unwrap();
        assert_result(&output, 0, "pass");
    }
    let output = command(Path::new("-"), Path::new("-")).output().unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cannot both use stdin"));
}

#[test]
fn ratchet_missing_files_and_options_are_invalid_input_without_stdout() {
    let temp = tempfile::tempdir().unwrap();
    let (rule, observation) = inputs();
    let rule_path = temp.path().join("rule.json");
    let observation_path = temp.path().join("observation.json");
    let missing = temp.path().join("missing.json");
    fs::write(&rule_path, serde_json::to_vec(&rule).unwrap()).unwrap();
    fs::write(&observation_path, serde_json::to_vec(&observation).unwrap()).unwrap();
    for (rule, observation) in [(&missing, &observation_path), (&rule_path, &missing)] {
        let output = command(rule, observation).output().unwrap();
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8(output.stderr)
            .unwrap()
            .contains("failed to read"));
    }
    for args in [
        vec!["evaluate-ratchet"],
        vec!["evaluate-ratchet", "--rule", "-"],
        vec!["evaluate-ratchet", "--observation", "-"],
    ] {
        let output = Command::new(assert_cmd::cargo::cargo_bin!("policy-cli"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(64));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn ratchet_is_discoverable_in_command_help() {
    for args in [vec!["--help"], vec!["evaluate-ratchet", "--help"]] {
        let output = Command::new(assert_cmd::cargo::cargo_bin!("policy-cli"))
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(output.stderr.is_empty());
        let help = String::from_utf8(output.stdout).unwrap();
        assert!(help.contains("evaluate-ratchet"));
        if args.len() == 2 {
            assert!(help.contains("--rule <RULE>"));
            assert!(help.contains("--observation <OBSERVATION>"));
        }
    }
}
