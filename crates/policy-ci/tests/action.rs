#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn action_preserves_literal_arguments_and_rejects_unknown_modes() {
    let action: serde_json::Value =
        serde_saphyr::from_str(include_str!("../../../action/action.yml")).unwrap();
    let script = action["runs"]["steps"][0]["run"].as_str().unwrap();
    let root = std::env::temp_dir().join(format!("policy-action-test-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let fake = root.join("policy-ci");
    let compiled = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2021", "tests/support/argv.rs", "-o"])
        .arg(&fake)
        .output()
        .unwrap();
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o700)).unwrap();
    let invoke = |mode: &str| {
        Command::new("bash")
            .args(["-c", script])
            .env(
                "PATH",
                format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
            )
            .env("POLICY_COMMAND", mode)
            .env("POLICY_EXECUTABLE", &fake)
            .env("POLICY_PATH", "a path/$(exit 9).json")
            .env("POLICY_EVIDENCE", "evidence with spaces.json")
            .env("POLICY_HEAD", "head")
            .env("POLICY_ARTIFACTS", "artifacts")
            .env("POLICY_OBSERVATIONS", "trusted")
            .output()
            .unwrap()
    };
    let output = invoke("pre-merge");
    assert!(output.status.success());
    let args: Vec<_> = output
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).to_string())
        .collect();
    assert_eq!(
        args,
        [
            "pre-merge",
            "--mode",
            "authoritative",
            "--policy",
            "a path/$(exit 9).json",
            "--expected-head",
            "head",
            "--artifact-directory",
            "artifacts",
            "--trusted-observations",
            "trusted",
            "--evidence",
            "evidence with spaces.json"
        ]
    );
    let output = invoke("advisory");
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    fs::remove_dir_all(root).unwrap();
}
