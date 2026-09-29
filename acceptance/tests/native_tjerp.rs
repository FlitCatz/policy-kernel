//! Opt-in native producer and consumer acceptance; no simulated success reports.
use policy_contracts::{ArtifactDigest, SensorStatus, TrustClass};
use policy_core::canonical_sha256;
use project_verification::{bytes_digest, parse_profile, RunRequest};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn command(root: &Path, argv: &[&str]) -> String {
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Consumer fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Consumer fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{argv:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn request(root: &Path) -> RunRequest {
    let profile =
        parse_profile(&fs::read_to_string(root.join("project-profile.json")).unwrap()).unwrap();
    let mut files: Vec<_> = command(root, &["git", "ls-files", "-z"])
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|p| ArtifactDigest {
            path: p.into(),
            digest: bytes_digest(&fs::read(root.join(p)).unwrap()),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    let head = command(root, &["git", "rev-parse", "HEAD"]);
    serde_json::from_value(json!({"schema_version":1,"profile":profile,"expected_profile_digest":canonical_sha256(&profile).unwrap(),
        "root":root,"repository":"acceptance/tjerp","head_sha":head,"source_files":files,"expected_source_digest":canonical_sha256(&files).unwrap(),
        "provider":"local","model":"native-tjerp","execution_id":"tjerp-native","impact":{"kind":"unknown"},
        "mappings":{"acceptance_criteria":[],"changed_behaviors":[],"negative_case_ids":[]}})).unwrap()
}

#[test]
fn schema_three_fixture_stages_without_preset_layout() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = tempfile::tempdir().unwrap();
    let target = output.path().canonicalize().unwrap().join("consumer");
    policy_acceptance::stage_profile(
        &workspace.join("examples/tjerp-compatible"),
        "project-profile.json",
        &target,
    )
    .unwrap();
    assert!(!target.join("package.json").exists());
    assert!(!target.join("shared").exists());
    assert_eq!(
        parse_profile(&fs::read_to_string(target.join("project-profile.json")).unwrap())
            .unwrap()
            .schema_version,
        3
    );
}

#[test]
#[ignore = "requires exact fixture Node/npm/.NET tools, browser, network, and POLICY_TJERP_OUTPUT"]
fn native_tjerp_reports_audits_and_rejection_paths() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = PathBuf::from(std::env::var_os("POLICY_TJERP_OUTPUT").expect("fresh output path"));
    fs::create_dir(&output).unwrap();
    let root = output.join("consumer");
    policy_acceptance::stage_profile(
        &workspace.join("examples/tjerp-compatible"),
        "project-profile.json",
        &root,
    )
    .unwrap();
    command(
        &root.join("frontend"),
        &[
            "npm",
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--include=dev",
            "--include=optional",
            "--include=peer",
        ],
    );
    command(
        &root.join("backend"),
        &[
            "dotnet",
            "restore",
            "tests/Tests/Tests.csproj",
            "--locked-mode",
            "--configfile",
            "NuGet.Config",
        ],
    );
    command(&root, &["git", "init", "--quiet"]);
    command(&root, &["git", "add", "."]);
    command(
        &root,
        &["git", "commit", "--quiet", "-m", "synthetic consumer"],
    );
    let req = request(&root);
    fs::write(
        output.join("request.json"),
        serde_json::to_vec_pretty(&req).unwrap(),
    )
    .unwrap();
    let result = project_verification::run(req.clone()).expect("native baseline must run");
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    fs::write(
        output.join("harness.json"),
        serde_json::to_vec_pretty(&policy_acceptance::harness::observe(workspace).unwrap())
            .unwrap(),
    )
    .unwrap();
    for sensor in &result.sensors {
        assert_eq!(
            sensor.evidence.status,
            SensorStatus::Passed,
            "{}: {:?}",
            sensor.evidence.sensor_id,
            sensor.result.errors
        );
        assert_eq!(sensor.evidence.trust, TrustClass::LocalUntrusted);
        if sensor.configuration.sensor.report.is_some() {
            assert!(sensor.evidence.executed.unwrap() > 0);
        }
    }
    // Re-normalize real collected evidence after tampering with its report or binding.
    let mut checks = Vec::new();
    for sensor in result
        .sensors
        .iter()
        .filter(|s| s.configuration.sensor.report.is_some())
    {
        let report = sensor.configuration.sensor.report.as_ref().unwrap();
        let path = sensor.configuration.component_root.join(&report.path);
        let original = fs::read(&path).unwrap();
        for (name, bytes) in [
            ("empty", b"".as_slice()),
            ("truncated", &original[..original.len() / 2]),
        ] {
            fs::write(&path, bytes).unwrap();
            let mut collection = sensor.collection.clone();
            collection.record.artifacts[0].digest = bytes_digest(bytes);
            collection.evidence.output_digest = canonical_sha256(&collection.record).unwrap();
            let normalized = project_verification::run::normalize_collection(
                &req,
                sensor.configuration.clone(),
                collection,
            )
            .unwrap();
            assert_ne!(normalized.evidence.status, SensorStatus::Passed);
            checks.push(format!("{}:{name}", sensor.evidence.sensor_id));
        }
        fs::remove_file(&path).unwrap();
        let normalized = project_verification::run::normalize_collection(
            &req,
            sensor.configuration.clone(),
            sensor.collection.clone(),
        )
        .unwrap();
        assert_ne!(normalized.evidence.status, SensorStatus::Passed);
        checks.push(format!("{}:missing", sensor.evidence.sensor_id));
        fs::write(&path, original).unwrap();
        let mut stale = req.clone();
        stale.head_sha = serde_json::from_value(json!("b".repeat(40))).unwrap();
        assert!(project_verification::run::normalize_collection(
            &stale,
            sensor.configuration.clone(),
            sensor.collection.clone()
        )
        .is_err());
        checks.push(format!("{}:stale-head", sensor.evidence.sensor_id));
    }
    let mut stale = req.clone();
    stale.profile.version = "1.2.1".into();
    assert!(project_verification::run::prepare(&stale).is_err());
    checks.push("stale-profile".into());
    let props = root.join("backend/Directory.Build.props");
    fs::write(
        &props,
        fs::read_to_string(&props)
            .unwrap()
            .replace("net11.0", "net9.0"),
    )
    .unwrap();
    assert!(project_verification::run::prepare(&req).is_err());
    checks.push("changed-source-props".into());
    fs::write(
        output.join("negative-checks.json"),
        serde_json::to_vec_pretty(&checks).unwrap(),
    )
    .unwrap();
}
