use std::{fs, process::Command};

#[cfg(unix)]
#[test]
fn missing_native_tools_leave_explicit_unverified_summary_and_no_cases() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp = tmp.path().canonicalize().unwrap();
    let bin = tmp.join("bin");
    fs::create_dir(&bin).unwrap();
    let git = std::env::split_paths(&std::env::var_os("PATH").unwrap())
        .map(|path| path.join("git"))
        .find(|path| path.is_file())
        .unwrap();
    std::os::unix::fs::symlink(git, bin.join("git")).unwrap();
    let output = tmp.join("observations");
    let execution = Command::new(env!("CARGO_BIN_EXE_policy-acceptance"))
        .current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/.."))
        .env("PATH", bin)
        .args(["run", "--profile", "react-typescript", "--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(
        execution.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&execution.stderr)
    );
    let summary: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("summary.json")).unwrap()).unwrap();
    assert_eq!(summary["status"], "unverified");
    assert_eq!(summary["local_advisory"], "not_run");
    assert!(summary["ci_authoritative"]
        .as_str()
        .unwrap()
        .starts_with("unverified"));
    assert_eq!(summary["cases"], serde_json::json!([]));
    assert!(!output.join("baseline").exists());
}

#[test]
fn incompatible_scenario_is_rejected_before_output_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let output = tmp.path().join("observations");
    let execution = Command::new(env!("CARGO_BIN_EXE_policy-acceptance"))
        .args([
            "run",
            "--profile",
            "dotnet",
            "--scenario",
            "type-error",
            "--output",
        ])
        .arg(&output)
        .output()
        .unwrap();
    assert_eq!(execution.status.code(), Some(64));
    assert!(!output.exists());
}
