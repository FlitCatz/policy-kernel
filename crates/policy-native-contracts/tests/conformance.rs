use policy_native_contracts::*;
use serde_json::json;

#[test]
fn digest_domains_are_distinct_and_canonical() {
    let a = canonical_digest(DigestRole::Command, &json!({"z": 1, "a": 2})).unwrap();
    let b = canonical_digest(DigestRole::Command, &json!({"a": 2, "z": 1})).unwrap();
    assert_eq!(a, b);
    assert_ne!(
        a,
        canonical_digest(DigestRole::Configuration, &json!({"a": 2, "z": 1})).unwrap()
    );
    assert!(canonical_digest(DigestRole::Command, &json!(1.5)).is_err());
    assert!(serde_json::from_value::<RawByteDigest>(serde_json::to_value(a).unwrap()).is_err());
}

#[test]
fn required_sets_reject_empty_and_duplicate_members() {
    assert!(serde_json::from_value::<RequiredSet<String>>(json!([])).is_err());
    assert!(serde_json::from_value::<RequiredSet<String>>(json!(["a", "a"])).is_err());
    assert!(serde_json::from_value::<RequiredSet<String>>(json!(["a", "b"])).is_ok());
}

mod support;

fn decode(kind: &str, value: serde_json::Value) -> bool {
    match kind {
        "expected" => serde_json::from_value::<ExpectedScopeV1>(value).is_ok(),
        "run" => serde_json::from_value::<NativeRunV1>(value).is_ok(),
        "result" => serde_json::from_value::<LocalScopeResultV1>(value).is_ok(),
        _ => panic!("unknown corpus document"),
    }
}

#[test]
fn serialized_corpus_rejects_invalid_documents() {
    let corpus = support::corpus();
    for kind in ["expected", "run", "result"] {
        assert!(decode(kind, corpus["valid"][kind].clone()), "{kind}");
    }
    for case in corpus["invalid"].as_array().unwrap() {
        let kind = case["document"].as_str().unwrap();
        let source = if kind == "pair" { "run" } else { kind };
        let mut value = corpus["valid"][source].clone();
        let pointer = case["pointer"].as_str().unwrap();
        if let Some(slot) = value.pointer_mut(pointer) {
            *slot = case["value"].clone();
        } else {
            let (parent, field) = pointer.rsplit_once('/').unwrap();
            value
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(field.into(), case["value"].clone());
        }
        if kind == "pair" {
            value["observations"][0]["subject"] = value["subject"].clone();
            let expected: ExpectedScopeV1 = serde_json::from_value(support::expected()).unwrap();
            let run: NativeRunV1 = serde_json::from_value(value).unwrap();
            assert!(
                validate_subject_binding(&expected, &run).is_err(),
                "{}",
                case["name"]
            );
        } else {
            assert!(!decode(kind, value), "{}", case["name"]);
        }
    }
}

#[test]
fn committed_schemas_and_corpus_conform() {
    let corpus: serde_json::Value = serde_json::from_str(include_str!("corpus/v1.json")).unwrap();
    assert_eq!(corpus, support::corpus());
    for (kind, generated, committed) in [
        (
            "expected",
            schemars::schema_for!(ExpectedScopeV1),
            include_str!("../schemas/expected-scope-v1.schema.json"),
        ),
        (
            "run",
            schemars::schema_for!(NativeRunV1),
            include_str!("../schemas/native-run-v1.schema.json"),
        ),
        (
            "result",
            schemars::schema_for!(LocalScopeResultV1),
            include_str!("../schemas/local-scope-result-v1.schema.json"),
        ),
    ] {
        assert_eq!(
            format!("{}\n", serde_json::to_string_pretty(&generated).unwrap()),
            committed
        );
        let schema = serde_json::to_value(generated).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&corpus["valid"][kind]), "{kind}");
        for case in corpus["invalid"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["document"] == kind)
        {
            let mut value = corpus["valid"][kind].clone();
            let pointer = case["pointer"].as_str().unwrap();
            let (parent, field) = pointer.rsplit_once('/').unwrap();
            if let Some(slot) = value.pointer_mut(pointer) {
                *slot = case["value"].clone();
            } else {
                value
                    .pointer_mut(parent)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert(field.into(), case["value"].clone());
            }
            assert_eq!(
                validator.is_valid(&value),
                case["schema_valid"].as_bool().unwrap(),
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn canonical_vectors_pin_preimage_and_byte_digest_boundary() {
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("corpus/canonical-v1.json")).unwrap();
    for (name, role) in [
        ("command", DigestRole::Command),
        ("configuration", DigestRole::Configuration),
    ] {
        let digest = canonical_digest(role, &vectors["value"]).unwrap();
        assert_eq!(serde_json::to_value(&digest).unwrap(), vectors[name]);
        let envelope = json!({"schema":"policy_native_v1", "role":name,"value":{"a":2,"z":1}});
        assert_eq!(
            digest.sha256,
            policy_core::canonical_sha256(&envelope).unwrap()
        );
        assert_ne!(
            digest.sha256,
            policy_core::canonical_sha256(&vectors["value"]).unwrap()
        );
    }
}

#[test]
fn failed_and_incomplete_records_preserve_diagnostics_without_success() {
    let mut run = support::run();
    run["observations"][0]["completion"] = json!({"kind":"timed_out","detail":"deadline"});
    run["test_observations"] = json!([]);
    run["mappings"] = json!([]);
    let run: NativeRunV1 = serde_json::from_value(run).unwrap();
    assert_eq!(run.artifacts.len(), 1);
    let mut result = support::result();
    result["status"] = json!("error");
    result["digests"]["evidence"] = serde_json::Value::Null;
    result["requirements"] = json!([]);
    result["reasons"] = json!([{"code":"INCOMPLETE_RUN","location":{"suite_id":"unit","requirement_id":null,"artifact_id":"raw-1"}}]);
    assert!(serde_json::from_value::<LocalScopeResultV1>(result).is_ok());
}

#[test]
fn identifiers_counts_paths_and_direct_call_invariants_are_checked() {
    let expected: ExpectedScopeV1 = serde_json::from_value(support::expected()).unwrap();
    let run: NativeRunV1 = serde_json::from_value(support::run()).unwrap();
    validate_subject_binding(&expected, &run).unwrap();
    for array in ["observations", "test_observations", "artifacts", "mappings"] {
        let mut value = support::run();
        let duplicate = value[array][0].clone();
        value[array].as_array_mut().unwrap().push(duplicate);
        assert!(!decode("run", value), "{array}");
    }
    for path in [
        "/absolute",
        "a/../b",
        "a\\b",
        "a//b",
        "C:/raw.json",
        "a/./b",
        "",
    ] {
        let mut value = support::run();
        value["artifacts"][0]["path"] = json!(path);
        assert!(!decode("run", value), "{path}");
    }
    for document in ["expected", "run", "result"] {
        let corpus = support::corpus();
        for version in [json!(0), json!(2), json!(1.5), json!("1"), json!(null)] {
            let mut value = corpus["valid"][document].clone();
            value["schema_version"] = version;
            assert!(!decode(document, value));
        }
    }
    let mut invalid = run.clone();
    invalid.observations[0].subject.scope_id =
        policy_contracts::WorkItemId::parse("different").unwrap();
    assert!(invalid.validate().is_err());
    let mut invalid = expected.clone();
    invalid.bindings.maximum_age_seconds = 0;
    assert!(invalid.validate().is_err());
    // Repeated argv tokens are valid; argv is not a set.
    let mut repeated = run.clone();
    repeated.observations[0].command.argv =
        vec!["program".into(), "x".into(), "x".into(), "".into()];
    repeated.validate().unwrap();
    // Test IDs are namespaced by suite; raw identity remains opaque.
    assert_eq!(
        run.test_observations[0].tests[0].raw_identity.as_str(),
        "/original/checkout/tests/test-1"
    );
}

#[test]
fn valid_document_hashes_roundtrip_and_use_distinct_roles() {
    let expected: ExpectedScopeV1 = serde_json::from_value(support::expected()).unwrap();
    let run: NativeRunV1 = serde_json::from_value(support::run()).unwrap();
    let result: LocalScopeResultV1 = serde_json::from_value(support::result()).unwrap();
    assert_eq!(expected.digest().unwrap().role, DigestRole::ExpectedScope);
    assert_eq!(run.digest().unwrap().role, DigestRole::NativeRun);
    assert_eq!(result.digest().unwrap().role, DigestRole::LocalScopeResult);
    let roundtrip: ExpectedScopeV1 =
        serde_json::from_slice(&serde_json::to_vec(&expected).unwrap()).unwrap();
    assert_eq!(expected.digest().unwrap(), roundtrip.digest().unwrap());
    let roundtrip: NativeRunV1 =
        serde_json::from_slice(&serde_json::to_vec(&run).unwrap()).unwrap();
    assert_eq!(run.digest().unwrap(), roundtrip.digest().unwrap());
    let roundtrip: LocalScopeResultV1 =
        serde_json::from_slice(&serde_json::to_vec(&result).unwrap()).unwrap();
    assert_eq!(result.digest().unwrap(), roundtrip.digest().unwrap());
    // Wire duplicate fields must not be accepted through a Value preprocessing path.
    let serialized = serde_json::to_string(&expected).unwrap();
    let duplicate = serialized.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(serde_json::from_str::<ExpectedScopeV1>(&duplicate).is_err());
}

#[test]
fn unexecuted_mapping_and_retry_observations_are_not_hidden() {
    let mut value = support::run();
    value["test_observations"][0]["tests"][0]["outcome"] = json!("skipped");
    value["test_observations"][0]["tests"][0]["attempts"] = json!(0);
    value["test_observations"][0]["counts"] =
        json!({"discovered":1,"executed":0,"passed":0,"failed":0,"skipped":1,"retries":0});
    assert!(!decode("run", value.clone()));
    value["mappings"] = json!([]);
    assert!(decode("run", value));
    let mut retry = support::run();
    retry["test_observations"][0]["tests"][0]["attempts"] = json!(2);
    assert!(!decode("run", retry.clone()));
    retry["test_observations"][0]["counts"]["retries"] = json!(1);
    let decoded: NativeRunV1 = serde_json::from_value(retry).unwrap();
    assert_eq!(decoded.test_observations[0].counts.retries, 1);
    // Parsing a structurally consistent failed run is not scope satisfaction.
    let mut failed = support::run();
    failed["test_observations"][0]["tests"][0]["outcome"] = json!("failed");
    failed["test_observations"][0]["counts"]["passed"] = json!(0);
    failed["test_observations"][0]["counts"]["failed"] = json!(1);
    failed["observations"][0]["completion"]["code"] = json!(1);
    assert!(decode("run", failed));
}

#[test]
fn independent_minimum_count_thresholds_do_not_reject_stronger_bounds() {
    let mut value = support::expected();
    // These lower bounds are consistent: a ten-test run satisfies all three.
    value["requirements"]["observations"][0]["tests"]["minimum_passed"] = json!(10);
    assert!(decode("expected", value));
}

#[test]
fn every_wire_object_rejects_unknown_fields() {
    fn object_paths(value: &serde_json::Value, path: String, output: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(fields) => {
                output.push(path.clone());
                for (key, child) in fields {
                    object_paths(child, format!("{path}/{key}"), output);
                }
            }
            serde_json::Value::Array(values) => {
                for (i, child) in values.iter().enumerate() {
                    object_paths(child, format!("{path}/{i}"), output);
                }
            }
            _ => {}
        }
    }
    let corpus = support::corpus();
    for kind in ["expected", "run", "result"] {
        let value = &corpus["valid"][kind];
        let mut paths = Vec::new();
        object_paths(value, String::new(), &mut paths);
        for path in paths {
            let mut invalid = value.clone();
            invalid
                .pointer_mut(&path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unknown_authorization".into(), json!(true));
            assert!(!decode(kind, invalid), "{kind}{path}");
        }
    }
}
