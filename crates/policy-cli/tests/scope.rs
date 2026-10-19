use assert_cmd::Command;
use policy_native_contracts::*;
use policy_native_verifier::ImportedObservationsV1;
use serde_json::{json, Value};

#[test]
fn local_scope_command_exit_codes_and_input_boundaries() {
    let corpus: Value = serde_json::from_str(include_str!(
        "../../policy-native-contracts/tests/corpus/v1.json"
    ))
    .unwrap();
    let mut expected: ExpectedScopeV1 =
        serde_json::from_value(corpus["valid"]["expected"].clone()).unwrap();
    let run: NativeRunV1 = serde_json::from_value(corpus["valid"]["run"].clone()).unwrap();
    let mut bindings = expected.bindings.observations.as_slice().to_vec();
    bindings[0].command_digest =
        canonical_digest(DigestRole::Command, &run.observations[0].command).unwrap();
    expected.bindings.observations = RequiredSet::new(bindings).unwrap();
    let imported = ImportedObservationsV1 {
        schema_version: VersionV1::default(),
        expected_scope_digest: expected.digest().unwrap(),
        expected_run_id: run.subject.run_id.clone(),
        expected_scope_id: run.subject.scope_id.clone(),
        adapter_version: RecordId::parse("fixture/1").unwrap(),
        trust: LocalTrust::LocalUntrusted,
        review: ReviewGate::Unmet,
        unsuccessful_reports: vec![],
        run,
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("expected.json");
    std::fs::write(&path, serde_json::to_vec(&expected).unwrap()).unwrap();
    for case in 0..5 {
        let mut value = serde_json::to_value(&imported).unwrap();
        let code = match case {
            0 => 0,
            1 => {
                value["run"]["mappings"] = json!([]);
                2
            }
            2 => {
                value["run"]["observations"][0]["command"]["cwd"] = json!("other");
                64
            }
            3 => {
                value["trust"] = json!("ci_observed");
                64
            }
            _ => {
                value["run"]["observations"][0]["subject"]["source"]["head"] =
                    json!("2".repeat(40));
                64
            }
        };
        let output = Command::new(assert_cmd::cargo::cargo_bin!("policy-cli"))
            .arg("evaluate-scope")
            .arg("--expected")
            .arg(&path)
            .args(["--observations", "-"])
            .write_stdin(serde_json::to_vec(&value).unwrap())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(code),
            "case {case}: {:?}",
            output
        );
        if case < 3 {
            let result: LocalScopeResultV1 = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(result.trust, LocalTrust::LocalUntrusted);
            assert_eq!(result.review, ReviewGate::Unmet);
        } else {
            assert!(output.stdout.is_empty());
        }
    }
    let output = Command::new(assert_cmd::cargo::cargo_bin!("policy-cli"))
        .args(["evaluate-scope", "--expected", "-", "--observations", "-"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
}
