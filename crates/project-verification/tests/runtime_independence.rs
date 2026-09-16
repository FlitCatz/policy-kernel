#![cfg(unix)]
mod common;
#[path = "../../../tests/support/isolated_runtime.rs"]
mod isolated;
use isolated::{success, Runtime};
use policy_core::canonical_sha256;
use std::{fs, path::Path, process::Command};

#[test]
fn runtime_independence_prepares_and_runs_without_frameworks_or_rust() {
    let runtime = Runtime::new(Path::new(env!("CARGO_BIN_EXE_project-verification")));
    let fixture = runtime.root.join("fixture");
    assert!(
        Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
            .args([
                "--edition=2021",
                "../../tests/support/runtime_fixture.rs",
                "-o"
            ])
            .arg(&fixture)
            .status()
            .unwrap()
            .success()
    );
    let mut request = common::request(&runtime.project);
    request.profile.sensors[0].argv = vec![fixture.to_str().unwrap().into(), "report".into()];
    request.profile.sensors[0].environment_allowlist.clear();
    request.profile.sensors[0].timeout_ms = 5000;
    request.expected_profile_digest = canonical_sha256(&request.profile).unwrap();
    let profile = runtime.root.join("profile.json");
    fs::write(&profile, serde_json::to_vec(&request.profile).unwrap()).unwrap();
    success(
        runtime
            .command()
            .arg("validate-profile")
            .arg(&profile)
            .output()
            .unwrap(),
    );
    let input = runtime.root.join("request.json");
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let prepared = success(
        runtime
            .command()
            .args(["prepare", "--input"])
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_eq!(
        prepared["selected_sensor_ids"],
        serde_json::json!(["frontend:unit"])
    );
    assert!(!runtime.project.join("reports/unit.json").exists());
    let result = success(
        runtime
            .command()
            .args(["run", "--input"])
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_eq!(result["sensors"][0]["evidence"]["status"], "passed");
    assert_eq!(result["sensors"][0]["evidence"]["trust"], "local_untrusted");
    assert_eq!(result["adequacy"]["executed"], 1);
    assert_eq!(result["adequacy"]["passed"], 1);
    // A second execution must not reuse the first report even in this minimal environment.
    let replay = runtime
        .command()
        .args(["run", "--input"])
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(replay.status.code(), Some(64));
    assert!(replay.stdout.is_empty());
    runtime.assert_no_agent_state();
}
