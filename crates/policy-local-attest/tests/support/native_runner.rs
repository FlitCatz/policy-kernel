use policy_contracts::UtcTimestamp;
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn timestamp(time: SystemTime) -> UtcTimestamp {
    UtcTimestamp::try_from(time).unwrap()
}

pub fn emit(root: &Path, mode: &str, sha256: impl FnOnce(&[u8]) -> String) {
    let started = SystemTime::now();
    let mut run: Value =
        serde_json::from_slice(&fs::read(root.join("run-template.json")).unwrap()).unwrap();
    let mut report: Value = serde_json::from_str(include_str!(
        "../../../policy-report-adapters/tests/fixtures/project-reports/vitest.json"
    ))
    .unwrap();
    // The upstream format records the start with millisecond precision.
    let report_start_millis = started.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 + 1;
    let report_start = UNIX_EPOCH + Duration::from_millis(report_start_millis);
    report["startTime"] = json!(report_start_millis);
    let bytes = serde_json::to_vec(&report).unwrap();
    let raw_digest = json!(sha256(&bytes));
    fs::create_dir(root.join("raw")).unwrap();
    fs::write(root.join("raw/raw.json"), bytes).unwrap();
    run["artifacts"][0]["digest"]["sha256"] = raw_digest.clone();
    run["test_observations"][0]["raw_report"]["digest"]["sha256"] = raw_digest;
    let observation = &mut run["observations"][0];
    let started = if mode == "early" { UNIX_EPOCH } else { started };
    observation["started_at"] = json!(timestamp(started));
    let completed = SystemTime::now().max(report_start);
    // A tiny run may finish within the same millisecond as the raw report start.
    if let Ok(remaining) = completed.duration_since(SystemTime::now()) {
        std::thread::sleep(remaining);
    }
    observation["completed_at"] = json!(timestamp(completed));
    observation["observed_at"] = json!(timestamp(completed));
    if mode == "nonpass" {
        observation["completion"]["code"] = json!(1);
    }
    fs::write(root.join("run.json"), serde_json::to_vec(&run).unwrap()).unwrap();
}
