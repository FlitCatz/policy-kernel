#![cfg(unix)]

use evidence_collector::{collect, CollectionRequest};
use policy_contracts::{ExecutionStatus, TrustClass};
use serde_json::{json, Value};
use std::{
    fs,
    path::PathBuf,
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
    time::{Duration, Instant},
};

struct Workspace(PathBuf);
impl Workspace {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "collector-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path.canonicalize().unwrap())
    }
}
impl Drop for Workspace {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn executable() -> &'static PathBuf {
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| {
        let path = std::env::temp_dir().join(format!("collector-fixture-{}", std::process::id()));
        assert!(
            Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .args(["--edition=2021", "tests/support/fixture.rs", "-o"])
                .arg(&path)
                .status()
                .unwrap()
                .success()
        );
        path
    })
}

fn request(dir: &Workspace, args: &[&str]) -> Value {
    let mut argv = vec![executable().to_string_lossy().into_owned()];
    argv.extend(args.iter().map(|s| s.to_string()));
    json!({
        "argv": argv, "cwd": dir.0, "environment_allowlist": [],
        "timeout_ms": 2000, "stdout_limit_bytes": 1024, "stderr_limit_bytes": 1024,
        "version_args": ["--version"], "repository": "example/repo",
        "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "provider": "runner-provider", "model": "runner-model", "execution_id": "run-1",
        "input_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "artifact_paths": []
    })
}
fn run(value: Value) -> evidence_collector::CollectedExecution {
    collect(CollectionRequest::try_from_json(&value.to_string()).unwrap()).unwrap()
}

#[test]
fn success_preserves_execution_identity_bytes_and_bindings() {
    let dir = Workspace::new();
    let value = request(&dir, &["ok"]);
    let result = run(value.clone());
    assert_eq!(
        result.evidence.status,
        ExecutionStatus::Passed,
        "{result:#?}"
    );
    assert_eq!(result.evidence.trust, TrustClass::LocalUntrusted);
    assert_eq!(result.evidence.provider.as_str(), "runner-provider");
    assert_eq!(result.evidence.model.as_str(), "runner-model");
    assert_eq!(result.evidence.execution_id.as_str(), "run-1");
    assert_eq!(
        result.evidence.head_sha.as_ref().map(|head| head.as_str()),
        value["head_sha"].as_str()
    );
    assert_eq!(result.record.execution.stdout, b"ok\n");
    assert_eq!(result.record.execution.stderr, b"note\n");
    assert_eq!(result.record.execution.exit_code, Some(0));
    assert_eq!(result.record.version_probe.stdout, b"fixture 1\n");
    assert!(result.evidence.started_at <= result.evidence.completed_at);
    assert_eq!(result.record.command.repository.as_str(), "example/repo");
    assert_eq!(
        result.record.command.argv,
        serde_json::from_value::<Vec<String>>(value["argv"].clone()).unwrap()
    );
    assert_eq!(
        result.evidence.command_digest,
        policy_core::canonical_sha256(&result.record.command).unwrap()
    );
    assert_eq!(
        result.evidence.output_digest,
        policy_core::canonical_sha256(&result.record).unwrap()
    );
    let mut changed = value;
    changed["head_sha"] = json!("cccccccccccccccccccccccccccccccccccccccc");
    assert_ne!(
        result.evidence.command_digest,
        run(changed).evidence.command_digest
    );
}

#[test]
fn timeout_and_truncation_are_explicit_non_success() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["sleep"]);
    value["timeout_ms"] = json!(100);
    let start = Instant::now();
    assert_eq!(run(value).evidence.status, ExecutionStatus::TimedOut);
    assert!(start.elapsed() < Duration::from_secs(2));
    let mut value = request(&dir, &["bytes", "4096"]);
    value["stdout_limit_bytes"] = json!(64);
    value["stderr_limit_bytes"] = json!(64);
    let result = run(value);
    assert_eq!(
        result.evidence.status,
        ExecutionStatus::Truncated,
        "{result:#?}"
    );
    assert!(result.record.execution.stdout.len() <= 64);
    assert!(result.record.execution.stderr.len() <= 64);
}

#[test]
fn exact_output_limit_is_allowed_and_binary_logs_are_lossless() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["bytes", "64"]);
    value["stdout_limit_bytes"] = json!(64);
    value["stderr_limit_bytes"] = json!(64);
    let result = run(value);
    assert_eq!(result.evidence.status, ExecutionStatus::Passed);
    assert_eq!(result.record.execution.stdout, vec![b'x'; 64]);
    assert_eq!(result.record.execution.stderr, vec![b'x'; 64]);
    assert_eq!(
        run(request(&dir, &["binary"])).record.execution.stdout,
        [0, 255, 128, 10]
    );
}

#[test]
fn ordinary_nonzero_exit_still_records_tool_version_without_passing() {
    let dir = Workspace::new();
    let result = run(request(&dir, &["fail"]));
    assert_eq!(result.record.execution.status, ExecutionStatus::Failed);
    assert_eq!(result.record.execution.exit_code, Some(7));
    assert_eq!(result.record.version_probe.status, ExecutionStatus::Passed);
    assert_eq!(result.record.version_probe.stdout, b"fixture 1\n");
    assert_eq!(result.evidence.status, ExecutionStatus::Failed);
}

#[test]
fn nonzero_spawn_and_probe_failures_never_pass() {
    let dir = Workspace::new();
    let result = run(request(&dir, &["fail"]));
    assert_eq!(result.evidence.status, ExecutionStatus::Failed);
    assert_eq!(result.record.execution.exit_code, Some(7));
    let mut value = request(&dir, &["ok"]);
    value["argv"][0] = json!(dir.0.join("missing"));
    let result = run(value);
    assert_eq!(result.evidence.status, ExecutionStatus::Failed);
    assert!(result.record.execution.error.is_some());
    assert_eq!(result.record.version_probe.status, ExecutionStatus::Skipped);
    let mut value = request(&dir, &["ok"]);
    value["version_args"] = json!(["fail"]);
    assert_eq!(run(value).evidence.status, ExecutionStatus::Failed);
    let mut value = request(&dir, &["ok"]);
    value["version_args"] = json!(["sleep"]);
    value["timeout_ms"] = json!(100);
    assert_eq!(run(value).evidence.status, ExecutionStatus::TimedOut);
}

#[test]
fn shell_strings_invalid_limits_and_forged_trust_are_rejected() {
    assert!(CollectionRequest::try_from_json(r#"{"command":"sh -c echo ok"}"#).is_err());
    let dir = Workspace::new();
    for (key, replacement) in [
        ("argv", json!([])),
        ("argv", json!("echo ok")),
        ("argv", json!(["/bin/sh", "-c", "echo ok"])),
        ("argv", json!(["bash", "-c", "echo ok"])),
        ("argv", json!(["sh -c echo ok"])),
        ("timeout_ms", json!(0)),
        ("timeout_ms", json!(u64::MAX)),
        ("stdout_limit_bytes", json!(0)),
        ("stderr_limit_bytes", json!(u64::MAX)),
        ("environment_allowlist", json!(["SECRET=value"])),
        ("environment_allowlist", json!(["A", "A"])),
        ("artifact_paths", json!(["../escape"])),
        ("artifact_paths", json!(["/etc/passwd"])),
        ("cwd", json!(".")),
        ("version_args", json!([])),
        ("trust", json!("ci_observed")),
    ] {
        let mut value = request(&dir, &["ok"]);
        value[key] = replacement;
        assert!(
            CollectionRequest::try_from_json(&value.to_string()).is_err(),
            "accepted {key}: {value}"
        );
    }
}

#[test]
fn arguments_are_literal_not_shell_expanded() {
    let dir = Workspace::new();
    let result = run(request(
        &dir,
        &["args", "$(touch marker)", "; echo bad", "$HOME", "*"],
    ));
    assert_eq!(
        result.record.execution.stdout,
        b"$(touch marker)\n; echo bad\n$HOME\n*\n"
    );
    assert!(!dir.0.join("marker").exists());
}

#[test]
fn timeout_and_normal_exit_clean_up_descendants_holding_pipes() {
    let dir = Workspace::new();
    for mode in ["descendant", "orphan"] {
        let marker = dir.0.join(mode);
        let mut value = request(&dir, &[mode, marker.to_str().unwrap()]);
        value["timeout_ms"] = json!(100);
        let start = Instant::now();
        let result = run(value);
        assert_ne!(result.evidence.status, ExecutionStatus::Passed);
        assert!(start.elapsed() < Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(600));
        assert!(!marker.exists(), "descendant survived {mode}");
    }
}

#[test]
fn artifacts_are_hashed_and_missing_or_escaping_files_fail_closed() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["artifact"]);
    value["artifact_paths"] = json!(["result.txt"]);
    let result = run(value);
    assert_eq!(result.evidence.status, ExecutionStatus::Passed);
    assert_eq!(
        result.record.artifacts[0].digest.as_str(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let mut value = request(&dir, &["ok"]);
    value["artifact_paths"] = json!(["missing.txt"]);
    assert_ne!(run(value).evidence.status, ExecutionStatus::Passed);
    std::os::unix::fs::symlink("/etc/passwd", dir.0.join("escape")).unwrap();
    let mut value = request(&dir, &["ok"]);
    value["artifact_paths"] = json!(["escape"]);
    assert_ne!(run(value).evidence.status, ExecutionStatus::Passed);
}

#[test]
fn cli_clears_environment_and_records_names_without_values() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["env"]);
    value["environment_allowlist"] = json!(["COLLECTOR_VISIBLE"]);
    let input = dir.0.join("request.json");
    fs::write(&input, value.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_evidence-collector"))
        .arg(&input)
        .env("COLLECTOR_VISIBLE", "visible-value")
        .env("COLLECTOR_SECRET", "secret-value")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: evidence_collector::CollectedExecution =
        serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result.evidence.trust, TrustClass::LocalUntrusted);
    assert_eq!(
        result.record.command.environment_allowlist,
        ["COLLECTOR_VISIBLE"]
    );
    assert_eq!(
        String::from_utf8(result.record.execution.stdout).unwrap(),
        format!("visible-value\nabsent\n{}\n", dir.0.display())
    );
    assert!(!String::from_utf8(output.stdout)
        .unwrap()
        .contains("secret-value"));
}

#[test]
fn cli_stdin_exit_codes_and_invalid_input() {
    use std::io::Write;
    let dir = Workspace::new();
    for (input, code) in [
        (request(&dir, &["ok"]).to_string(), 0),
        (request(&dir, &["fail"]).to_string(), 2),
        ("{}".into(), 64),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_evidence-collector"))
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(code));
        if code == 64 {
            assert!(output.stdout.is_empty());
            assert!(!output.stderr.is_empty());
        } else {
            assert!(serde_json::from_slice::<Value>(&output.stdout)
                .unwrap()
                .is_object());
        }
    }
}

#[test]
fn stderr_and_probe_output_limits_cannot_pass() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["stderr"]);
    value["stderr_limit_bytes"] = json!(64);
    let result = run(value);
    assert_eq!(result.evidence.status, ExecutionStatus::Truncated);
    assert_eq!(result.record.execution.stderr.len(), 64);
    assert_eq!(result.record.version_probe.status, ExecutionStatus::Skipped);
    let mut value = request(&dir, &["ok"]);
    value["version_args"] = json!(["bytes", "4096"]);
    assert_eq!(run(value).evidence.status, ExecutionStatus::Truncated);
    let result = run(request(&dir, &["signal"]));
    assert_eq!(result.evidence.status, ExecutionStatus::Failed);
    assert_eq!(result.record.execution.exit_code, None);
    assert_eq!(result.record.version_probe.status, ExecutionStatus::Skipped);
}

#[test]
fn main_and_probe_share_a_single_timeout() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["delay", "150"]);
    value["timeout_ms"] = json!(250);
    value["version_args"] = json!(["delay", "150"]);
    assert_eq!(run(value).evidence.status, ExecutionStatus::TimedOut);
}

#[test]
fn cleanup_covers_truncation_and_descendants_without_inherited_pipes() {
    let dir = Workspace::new();
    for (mode, expected) in [
        ("flood-descendant", ExecutionStatus::Truncated),
        ("orphan-closed", ExecutionStatus::Passed),
    ] {
        let marker = dir.0.join(mode);
        let value = request(&dir, &[mode, marker.to_str().unwrap()]);
        assert_eq!(run(value).evidence.status, expected);
        std::thread::sleep(Duration::from_millis(600));
        assert!(!marker.exists());
    }
}

#[test]
fn oversized_and_nonregular_artifacts_fail_without_blocking() {
    let dir = Workspace::new();
    fs::File::create(dir.0.join("large"))
        .unwrap()
        .set_len(16 * 1024 * 1024 + 1)
        .unwrap();
    assert!(Command::new("mkfifo")
        .arg(dir.0.join("fifo"))
        .status()
        .unwrap()
        .success());
    for path in ["large", "fifo"] {
        let mut value = request(&dir, &["ok"]);
        value["artifact_paths"] = json!([path]);
        let start = Instant::now();
        let result = run(value);
        assert_eq!(result.evidence.status, ExecutionStatus::Failed);
        assert!(!result.record.artifact_errors.is_empty());
        assert!(start.elapsed() < Duration::from_secs(3));
    }
}

#[test]
fn explicit_executable_paths_support_spaces_and_shell_symlinks_are_rejected() {
    let dir = Workspace::new();
    let copied = dir.0.join("fixture with spaces");
    fs::copy(executable(), &copied).unwrap();
    let mut value = request(&dir, &["ok"]);
    value["argv"][0] = json!(copied);
    assert_eq!(run(value).evidence.status, ExecutionStatus::Passed);
    let shell = dir.0.join("alias");
    std::os::unix::fs::symlink("/bin/sh", &shell).unwrap();
    let mut value = request(&dir, &["ok"]);
    value["argv"][0] = json!(shell);
    assert!(collect(CollectionRequest::try_from_json(&value.to_string()).unwrap()).is_err());
}

#[test]
fn executable_search_requires_explicitly_allowlisted_path() {
    let dir = Workspace::new();
    let mut value = request(&dir, &["ok"]);
    value["argv"][0] = json!(executable().file_name().unwrap().to_str().unwrap());
    let input = dir.0.join("request.json");
    fs::write(&input, value.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_evidence-collector"))
        .arg(&input)
        .env("PATH", executable().parent().unwrap())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    value["environment_allowlist"] = json!(["PATH"]);
    fs::write(&input, value.to_string()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_evidence-collector"))
        .arg(&input)
        .env("PATH", executable().parent().unwrap())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn checked_in_process_boundary_fixtures() {
    let dir = Workspace::new();
    for (source, expected) in [
        (
            include_str!("../../../tests/fixtures/collector/success.json"),
            ExecutionStatus::Passed,
        ),
        (
            include_str!("../../../tests/fixtures/collector/timeout.json"),
            ExecutionStatus::TimedOut,
        ),
        (
            include_str!("../../../tests/fixtures/collector/truncated.json"),
            ExecutionStatus::Truncated,
        ),
    ] {
        let mut value: Value = serde_json::from_str(source).unwrap();
        value["argv"][0] = json!(executable());
        value["cwd"] = json!(dir.0);
        assert_eq!(run(value).evidence.status, expected);
    }
}

#[test]
fn artifact_walk_rejects_symlinks_in_every_component() {
    let dir = Workspace::new();
    fs::create_dir(dir.0.join("nested")).unwrap();
    fs::write(dir.0.join("nested/result.txt"), b"abc").unwrap();
    std::os::unix::fs::symlink("nested", dir.0.join("alias")).unwrap();
    std::os::unix::fs::symlink("nested/result.txt", dir.0.join("leaf")).unwrap();
    for (path, expected) in [
        ("nested/result.txt", ExecutionStatus::Passed),
        ("alias/result.txt", ExecutionStatus::Failed),
        ("leaf", ExecutionStatus::Failed),
    ] {
        let mut value = request(&dir, &["ok"]);
        value["artifact_paths"] = json!([path]);
        assert_eq!(run(value).evidence.status, expected, "{path}");
    }
}

#[test]
fn actual_argv_zero_matches_record_for_symlink_dispatch() {
    let dir = Workspace::new();
    let alias = dir.0.join("different-invocation-name");
    std::os::unix::fs::symlink(executable(), &alias).unwrap();
    let mut value = request(&dir, &["argv0"]);
    value["argv"][0] = json!(alias);
    value["version_args"] = json!(["argv0"]);
    let result = run(value);
    let expected = format!("{}\n", alias.display()).into_bytes();
    assert_eq!(result.record.execution.stdout, expected);
    assert_eq!(result.record.version_probe.stdout, expected);
    assert_eq!(result.record.command.argv[0], alias.to_str().unwrap());
}
