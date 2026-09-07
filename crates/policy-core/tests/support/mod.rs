use policy_contracts::PreMergeEvaluationInput;
use policy_core::canonical_sha256;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn fixture_value(relative: &str) -> Value {
    let path = workspace_root().join(relative);
    let bytes = std::fs::read(&path)
        .unwrap_or_else(|error| panic!("failed to read fixture {}: {error}", path.display()));
    serde_json::from_slice(&bytes)
        .unwrap_or_else(|error| panic!("fixture {} is not valid JSON: {error}", path.display()))
}

pub fn ready_value() -> Value {
    fixture_value("tests/fixtures/pre-merge/ready.json")
}

pub fn pre_merge_input(value: Value) -> PreMergeEvaluationInput {
    serde_json::from_value(value).expect("fixture must satisfy PreMergeEvaluationInput")
}

/// Recomputes transport hashes after a test deliberately changes evidence.
/// Expected decisions remain hand-written literals in each test.
pub fn seal_evidence(value: &mut Value) {
    for path in [
        "executions",
        "findings",
        "knowledge",
        "modifications",
        "reviews",
        "sensors",
        "test_adequacy",
        "waivers",
    ] {
        let digest = canonical_sha256(&value["evidence"][path]).unwrap();
        let artifacts = value["evidence"]["manifest"]["child_digests"]
            .as_array_mut()
            .unwrap();
        let artifact = artifacts
            .iter_mut()
            .find(|artifact| artifact["path"] == path)
            .unwrap();
        artifact["digest"] = json!(digest);
    }
}
