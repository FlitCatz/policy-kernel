use crate::support;
use policy_native_contracts::*;
use serde_json::{json, Value};

pub fn corpus() -> Value {
    let statement = json!({
        "schema_version":1,"domain":"policy-kernel/local-runner-attestation/v1",
        "subject":support::subject(),"head_tree":"2".repeat(40),"base":"3".repeat(40),
        "run_id":"run-1","scope_id":"scope-1",
        "expected_scope_digest":support::digest("expected_scope"),
        "scope_template_digest":support::digest("scope_template"),
        "native_run_digest":support::digest("native_run"),
        "observations_digest":support::digest("imported_observations"),
        "result_digest":support::digest("local_scope_result"),
        "runner":{"runner_id":"runner-1","os":"macos","arch":"arm64","attestor_version":"example-attestor/1"},
        "started_at":"2026-10-19T01:00:00Z","completed_at":"2026-10-19T01:00:02Z",
        "attested_at":"2026-10-19T01:01:00Z"
    });
    let signed = json!({"statement":statement,"signature":{"format":"sshsig","runner_id":"runner-1","value":"example-signature"}});
    let mut invalid = Vec::new();
    for (name, pointer, value, schema_valid) in [
        ("unknown-version", "/schema_version", json!(2), false),
        ("wrong-domain", "/domain", json!("other/v1"), false),
        ("unknown-field", "/trusted", json!(true), false),
        (
            "wrong-subject-role",
            "/subject/source_set_digest/role",
            json!("command"),
            true,
        ),
        ("invalid-tree", "/head_tree", json!("not-a-tree"), false),
        ("invalid-base", "/base", json!("short"), false),
        ("empty-run", "/run_id", json!(""), false),
        (
            "wrong-expected-role",
            "/expected_scope_digest/role",
            json!("native_run"),
            true,
        ),
        (
            "wrong-run-role",
            "/native_run_digest/role",
            json!("expected_scope"),
            true,
        ),
        (
            "wrong-result-role",
            "/result_digest/role",
            json!("native_run"),
            true,
        ),
        (
            "wrong-template-role",
            "/scope_template_digest/role",
            json!("imported_observations"),
            true,
        ),
        (
            "wrong-observations-role",
            "/observations_digest/role",
            json!("scope_template"),
            true,
        ),
        (
            "raw-in-canonical-slot",
            "/observations_digest",
            support::raw(),
            false,
        ),
        (
            "wrong-digest-schema",
            "/observations_digest/schema",
            json!("other"),
            false,
        ),
        ("empty-runner", "/runner/runner_id", json!(""), false),
        ("nested-unknown", "/runner/trusted", json!(true), false),
        (
            "reversed-run-times",
            "/started_at",
            json!("2026-10-19T01:00:03Z"),
            true,
        ),
        (
            "early-attestation",
            "/attested_at",
            json!("2026-10-19T01:00:01Z"),
            true,
        ),
    ] {
        for (document, prefix) in [("statement", ""), ("signed", "/statement")] {
            invalid.push(json!({"name":name,"document":document,"pointer":format!("{prefix}{pointer}"),"value":value,"schema_valid":schema_valid}));
        }
    }
    for (name, pointer, value, schema_valid) in [
        (
            "runner-mismatch",
            "/signature/runner_id",
            json!("runner-2"),
            true,
        ),
        ("empty-signature", "/signature/value", json!(""), false),
        ("unknown-format", "/signature/format", json!("other"), false),
        (
            "signature-unknown-field",
            "/signature/trusted",
            json!(true),
            false,
        ),
        ("envelope-unknown-field", "/trusted", json!(true), false),
    ] {
        invalid.push(json!({"name":name,"document":"signed","pointer":pointer,"value":value,"schema_valid":schema_valid}));
    }
    json!({"valid":{"statement":statement,"signed":signed},"invalid":invalid})
}

pub fn canonical_vectors() -> Value {
    let original = support::corpus();
    let expected: ExpectedScopeV1 =
        serde_json::from_value(original["valid"]["expected"].clone()).unwrap();
    let run: NativeRunV1 = serde_json::from_value(original["valid"]["run"].clone()).unwrap();
    let result: LocalScopeResultV1 =
        serde_json::from_value(original["valid"]["result"].clone()).unwrap();
    let statement: LocalAttestationV1 =
        serde_json::from_value(corpus()["valid"]["statement"].clone()).unwrap();
    let value = json!({"z":1,"a":2});
    json!({"expected":expected.digest().unwrap(),"run":run.digest().unwrap(),"result":result.digest().unwrap(),
        "template":expected.template_digest().unwrap(),"statement":statement.digest().unwrap(),
        "value":value,"observations":canonical_local_digest(LocalDigestRole::ImportedObservations,&value).unwrap()})
}
