#![cfg(unix)]
#[path = "../../../tests/support/isolated_runtime.rs"]
mod isolated;
use isolated::{success, Runtime};
use serde_json::json;
use std::{fs, path::Path, process::Command};

#[test]
fn runtime_independence_collects_fixture_without_frameworks_or_rust() {
    let runtime = Runtime::new(Path::new(env!("CARGO_BIN_EXE_evidence-collector")));
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
    let request = json!({
        "argv": [fixture, "report"], "cwd": runtime.project, "environment_allowlist": [],
        "timeout_ms": 5000, "stdout_limit_bytes": 4096, "stderr_limit_bytes": 4096,
        "version_args": ["--version"], "repository": "example/repo", "head_sha": "a".repeat(40),
        "provider": "fixture-provider", "model": "fixture-model", "execution_id": "isolated-1",
        "input_digest": "b".repeat(64), "artifact_paths": ["reports/unit.json"]
    });
    let input = runtime.root.join("request.json");
    fs::write(&input, serde_json::to_vec(&request).unwrap()).unwrap();
    let result = success(runtime.command().arg(&input).output().unwrap());
    assert_eq!(result["evidence"]["status"], "passed");
    assert_eq!(result["evidence"]["trust"], "local_untrusted");
    assert_eq!(
        result["record"]["version_probe"]["stdout"],
        json!(b"fixture-1\n".to_vec())
    );
    assert_eq!(
        result["record"]["artifacts"][0]["path"],
        "reports/unit.json"
    );
    runtime.assert_no_agent_state();
}
