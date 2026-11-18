use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::{fs, path::Path, time::SystemTime};

pub fn emit(root: &Path, mode: &str, sha256: impl FnOnce(&[u8]) -> String) {
    let started = DateTime::<Utc>::from(SystemTime::now());
    let mut run: Value =
        serde_json::from_slice(&fs::read(root.join("run-template.json")).unwrap()).unwrap();
    let mut report: Value = serde_json::from_str(include_str!(
        "../../../policy-report-adapters/tests/fixtures/project-reports/vitest.json"
    ))
    .unwrap();
    // The upstream format records the start with millisecond precision.
    let report_start = DateTime::from_timestamp_millis(started.timestamp_millis() + 1).unwrap();
    report["startTime"] = json!(report_start.timestamp_millis());
    let bytes = serde_json::to_vec(&report).unwrap();
    let raw_digest = json!(sha256(&bytes));
    fs::create_dir(root.join("raw")).unwrap();
    fs::write(root.join("raw/raw.json"), bytes).unwrap();
    run["artifacts"][0]["digest"]["sha256"] = raw_digest.clone();
    run["test_observations"][0]["raw_report"]["digest"]["sha256"] = raw_digest;
    let observation = &mut run["observations"][0];
    let started = if mode == "early" {
        DateTime::<Utc>::from(SystemTime::UNIX_EPOCH)
    } else {
        started
    };
    observation["started_at"] = json!(started.to_rfc3339());
    let completed = DateTime::<Utc>::from(SystemTime::now()).max(report_start);
    // A tiny run may finish within the same millisecond as the raw report start.
    if let Ok(remaining) = completed
        .signed_duration_since(DateTime::<Utc>::from(SystemTime::now()))
        .to_std()
    {
        std::thread::sleep(remaining);
    }
    observation["completed_at"] = json!(completed.to_rfc3339());
    observation["observed_at"] = json!(completed.to_rfc3339());
    if mode == "nonpass" {
        observation["completion"]["code"] = json!(1);
    }
    fs::write(root.join("run.json"), serde_json::to_vec(&run).unwrap()).unwrap();
}
