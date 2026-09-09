#![allow(dead_code)]
use evidence_collector::CollectedExecution;
use policy_contracts::ArtifactDigest;
use policy_core::canonical_sha256;
use project_verification::{
    bytes_digest,
    profile::ProjectProfile,
    run::{ConfigurationBinding, RunRequest},
};
use serde_json::json;

pub fn request(root: &std::path::Path) -> RunRequest {
    std::fs::write(root.join("config.json"), b"{}").unwrap();
    let profile: ProjectProfile = serde_json::from_value(json!({
        "schema_version":1,"id":"react-typescript","version":"1.0.0",
        "components":[{"id":"frontend","root":"."}],
        "sensors":[{"definition":{"id":"frontend:unit","required":true,"required_when":[],
            "not_applicable_when":[],"accepted_exit_codes":[0],"result_requirement":"test_counts"},
            "component":"frontend","argv":["this-command-must-not-exist"],
            "version_args":["--version"],"version_stdout":"fixture-1",
            "report":{"format":"vitest_json","path":"reports/unit.json"},
            "inputs":["config.json"],"timeout_ms":1000}],
        "adequacy":{"acceptance_criterion_ids":[],"changed_behavior_ids":[],"negative_case_ids":[]}
    }))
    .unwrap();
    let files = vec![ArtifactDigest {
        path: "config.json".into(),
        digest: bytes_digest(b"{}"),
    }];
    serde_json::from_value(json!({"schema_version":1,
        "expected_profile_digest":canonical_sha256(&profile).unwrap(),"profile":profile,
        "root":root.canonicalize().unwrap(),"repository":"owner/repo","head_sha":"a".repeat(40),
        "source_files":files,"expected_source_digest":canonical_sha256(&files).unwrap(),
        "provider":"local","model":"fixture","execution_id":"run-1","impact":{"kind":"unknown"},
        "mappings":{"acceptance_criteria":[],"changed_behaviors":[],"negative_case_ids":[]}
    }))
    .unwrap()
}

pub fn collection(
    request: &RunRequest,
    config: &ConfigurationBinding,
    report: &[u8],
) -> CollectedExecution {
    let input = canonical_sha256(&json!({"schema_version":1,
        "configuration_digest":canonical_sha256(config).unwrap(),"repository":request.repository,
        "head_sha":request.head_sha,"source_set_digest":request.expected_source_digest}))
    .unwrap();
    let process = json!({"status":"passed","started_at":"2025-09-01T00:00:00Z",
        "completed_at":"2025-09-01T00:00:01Z","exit_code":0,"stdout":[],"stderr":[],"error":null});
    let mut version = process.clone();
    version["stdout"] = json!(b"fixture-1\n".to_vec());
    let record = json!({"command":{
        "argv":config.sensor.argv,"resolved_executable":"/bin/fixture","cwd":config.component_root,
        "environment_allowlist":["PATH"],"timeout_ms":1000,
        "stdout_limit_bytes":evidence_collector::MAX_OUTPUT_BYTES,"stderr_limit_bytes":evidence_collector::MAX_OUTPUT_BYTES,
        "version_args":["--version"],"repository":request.repository,"head_sha":request.head_sha,
        "provider":"local","model":"fixture","execution_id":"run-1:frontend:unit",
        "input_digest":input,"artifact_paths":["reports/unit.json"]},
        "execution":process,"version_probe":version,
        "artifacts":[{"path":"reports/unit.json","digest":bytes_digest(report)}],"artifact_errors":[]});
    serde_json::from_value(json!({"record":record,"evidence":{
        "execution_id":"run-1:frontend:unit","provider":"local","model":"fixture","status":"passed",
        "command_digest":canonical_sha256(&record["command"]).unwrap(),"input_digest":input,
        "output_digest":canonical_sha256(&record).unwrap(),"started_at":"2025-09-01T00:00:00Z",
        "completed_at":"2025-09-01T00:00:02Z","trust":"ci_observed"}}))
    .unwrap()
}

pub fn reseal(collection: &mut CollectedExecution) {
    collection.evidence.command_digest = canonical_sha256(&collection.record.command).unwrap();
    collection.evidence.output_digest = canonical_sha256(&collection.record).unwrap();
}
