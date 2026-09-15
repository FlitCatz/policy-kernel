//! Opt-in native acceptance: the ordinary offline suite never substitutes for this run.
use policy_contracts::{ArtifactDigest, SensorStatus, TrustClass};
use policy_core::canonical_sha256;
use project_verification::{bytes_digest, parse_profile, RunRequest, RunResult};
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn command(root: &Path, argv: &[&str]) -> std::process::Output {
    Command::new(argv[0])
        .args(&argv[1..])
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Native fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Native fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap()
}
fn git(root: &Path, args: &[&str]) -> String {
    let mut argv = vec!["git"];
    argv.extend(args);
    let output = command(root, &argv);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn commit(root: &Path, phase: &str) {
    git(root, &["add", "--all"]);
    git(root, &["commit", "--quiet", "--allow-empty", "-m", phase]);
}
fn request(root: &Path) -> RunRequest {
    assert!(git(root, &["status", "--porcelain"]).is_empty());
    let profile =
        parse_profile(&fs::read_to_string(root.join("project-profile.json")).unwrap()).unwrap();
    let head = git(root, &["rev-parse", "HEAD"]);
    let mut files: Vec<_> = git(root, &["ls-files", "-z"])
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(|path| ArtifactDigest {
            path: path.into(),
            digest: bytes_digest(&fs::read(root.join(path)).unwrap()),
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));
    serde_json::from_value(json!({"schema_version":1,"expected_profile_digest":canonical_sha256(&profile).unwrap(),
        "profile":profile,"root":root.canonicalize().unwrap(),"repository":"acceptance/react-jest",
        "head_sha":head,"source_files":files,"expected_source_digest":canonical_sha256(&files).unwrap(),
        "provider":"local","model":"native-jest","execution_id":format!("native-{head}"),
        "impact":{"kind":"unknown"},"mappings":{"acceptance_criteria":[],"changed_behaviors":[],"negative_case_ids":[]}})).unwrap()
}
fn phase(root: &Path, output: &Path, name: &str, pass: bool) -> (RunRequest, RunResult) {
    phase_expected(
        root,
        output,
        name,
        if pass { None } else { Some("frontend:unit") },
    )
}

fn phase_expected(
    root: &Path,
    output: &Path,
    name: &str,
    failed_sensor: Option<&str>,
) -> (RunRequest, RunResult) {
    let directory = output.join(name);
    fs::create_dir(&directory).unwrap();
    let request = request(root);
    fs::write(
        directory.join("request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
    let result = project_verification::run(request.clone()).unwrap();
    fs::write(
        directory.join("result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    for sensor in &result.sensors {
        assert_eq!(sensor.evidence.trust, TrustClass::LocalUntrusted);
        assert_eq!(sensor.evidence.head_sha, request.head_sha);
        assert_eq!(
            sensor.evidence.status == SensorStatus::Passed,
            failed_sensor != Some(sensor.evidence.sensor_id.as_str()),
            "{name}: {} {:?}",
            sensor.evidence.sensor_id,
            sensor.result.errors
        );
        if let Some(report) = &sensor.configuration.sensor.report {
            let file = sensor.configuration.component_root.join(&report.path);
            if file.exists() {
                fs::copy(&file, directory.join("original-report.json")).unwrap();
                fs::remove_file(file).unwrap();
            }
        }
    }
    (request, result)
}

#[test]
#[ignore = "requires provisioned Node/npm, npm advisory network, and POLICY_JEST_OUTPUT"]
fn native_jest_selected_paths_failures_corrections_and_prior_head_rejection() {
    let output =
        PathBuf::from(std::env::var_os("POLICY_JEST_OUTPUT").expect("set a new output directory"));
    fs::create_dir(&output).unwrap();
    let root = output.join("consumer");
    fs::create_dir(&root).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let source = workspace.join("examples/react-jest");
    for path in [
        "project-profile.json",
        "web/package.json",
        "web/package-lock.json",
        "web/.node-version",
        "web/jest.config.cjs",
        "web/src/Greeting.cjs",
        "web/tests/Greeting.test.cjs",
    ] {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(path), target).unwrap();
    }
    fs::write(
        root.join(".gitignore"),
        "node_modules/\n.policy-reports/\njest-report.json\n",
    )
    .unwrap();
    let selected = policy_bootstrap::plan(policy_bootstrap::Request {
        target: root.clone(),
        installation: policy_bootstrap::Installation {
            profile: "react-typescript".into(),
            release: "1.1.0".into(),
            components: Default::default(),
            user_profile: Some(policy_bootstrap::UserProfile {
                path: "project-profile.json".into(),
                digest: bytes_digest(&fs::read(root.join("project-profile.json")).unwrap())
                    .to_string(),
            }),
        },
        operation: policy_bootstrap::Operation::Install,
    })
    .unwrap();
    assert!(selected.conflicts.is_empty(), "{:?}", selected.conflicts);
    policy_bootstrap::apply(&selected).unwrap();
    let install = command(
        &root.join("web"),
        &[
            "npm",
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--fetch-retries=0",
        ],
    );
    fs::write(output.join("install.stdout.log"), &install.stdout).unwrap();
    fs::write(output.join("install.stderr.log"), &install.stderr).unwrap();
    assert!(install.status.success());
    git(&root, &["init", "--quiet"]);
    commit(&root, "baseline");
    let (baseline_request, baseline) = phase(&root, &output, "baseline", true);
    let test_path = root.join("web/tests/Greeting.test.cjs");
    let original = fs::read_to_string(&test_path).unwrap();
    fs::write(
        &test_path,
        original.replace("<h1>Hello policy</h1>", "<h1>Wrong greeting</h1>"),
    )
    .unwrap();
    commit(&root, "test failure");
    phase(&root, &output, "failed", false);
    fs::write(&test_path, &original).unwrap();
    commit(&root, "corrected");
    let (corrected, _) = phase(&root, &output, "corrected", true);
    let prior = baseline
        .sensors
        .iter()
        .find(|s| s.evidence.sensor_id.as_str() == "frontend:unit")
        .unwrap();
    let config = project_verification::run::prepare(&corrected)
        .unwrap()
        .configurations
        .into_iter()
        .find(|c| c.sensor.definition.id.as_str() == "frontend:unit")
        .unwrap();
    assert_ne!(baseline_request.head_sha, corrected.head_sha);
    assert!(project_verification::run::normalize_collection(
        &corrected,
        config,
        prior.collection.clone()
    )
    .is_err());
    fs::write(output.join("previous-head-rejection.json"), serde_json::to_vec_pretty(&json!({"previous":baseline_request.head_sha,"corrected":corrected.head_sha,"rejected":true})).unwrap()).unwrap();
    fs::write(&test_path, original.replace("test(", "test.skip(")).unwrap();
    commit(&root, "skipped tests");
    phase(&root, &output, "skipped", false);
    fs::write(&test_path, "jest.retryTimes(1); let calls = 0; test('retry is not clean evidence', () => { expect(++calls).toBe(2); });\n").unwrap();
    commit(&root, "retried test");
    phase(&root, &output, "retry", false);
    fs::write(&test_path, &original).unwrap();
    let original_profile = fs::read_to_string(root.join("project-profile.json")).unwrap();
    let mut profile: serde_json::Value = serde_json::from_str(&original_profile).unwrap();
    profile["sensors"][2]["argv"] = json!([
        "node",
        "node_modules/jest/bin/jest.js",
        "--runInBand",
        "--json",
        "--outputFile=jest-report.json",
        "--testMatch=**/does-not-exist/*.test.cjs",
        "--passWithNoTests"
    ]);
    fs::write(
        root.join("project-profile.json"),
        serde_json::to_vec_pretty(&profile).unwrap(),
    )
    .unwrap();
    commit(&root, "empty report");
    phase(&root, &output, "empty", false);
    profile["sensors"][2]["argv"] = json!(["node", "-e", ""]);
    fs::write(
        root.join("project-profile.json"),
        serde_json::to_vec_pretty(&profile).unwrap(),
    )
    .unwrap();
    commit(&root, "missing report");
    phase(&root, &output, "missing", false);
    fs::write(root.join("project-profile.json"), original_profile).unwrap();
    commit(&root, "final correction");
    phase(&root, &output, "final-corrected", true);
    nested_graph_audit(&root, &output);
    fs::write(output.join("summary.json"), serde_json::to_vec_pretty(&json!({"scope":"local-native-advisory", "ci_authoritative":"unverified", "status":"passed",
        "harness":policy_acceptance::harness::observe(workspace).unwrap(),"cases":["baseline","failed","corrected","skipped","retry","empty","missing","final-corrected","nested-audit-failed","nested-audit-corrected"],"previous_head_rejected":true})).unwrap()).unwrap();
}

fn nested_graph_audit(root: &Path, output: &Path) {
    let directory = root.join("web/packages/extra");
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join(".npmrc"), "omit=dev\n").unwrap();
    let profile_path = root.join("project-profile.json");
    let mut profile = parse_profile(&fs::read_to_string(&profile_path).unwrap()).unwrap();
    let mut graph = profile.selection.as_ref().unwrap().graphs[0].clone();
    let mut sensor = profile
        .sensors
        .iter()
        .find(|s| s.definition.id.as_str() == graph.audit_sensor)
        .unwrap()
        .clone();
    graph.id = "extra-packages".into();
    graph.audit_sensor = "frontend:extra-audit".into();
    graph.manifest = "packages/extra/package.json".into();
    graph.lockfile = "packages/extra/package-lock.json".into();
    let selection = profile.selection.as_mut().unwrap();
    sensor.definition.id = policy_contracts::SensorId::parse(&graph.audit_sensor).unwrap();
    sensor.inputs = vec![graph.manifest.clone(), graph.lockfile.clone()];
    sensor.argv = graph.audit_argv(&selection.tools[&graph.tool]);
    selection
        .bindings
        .insert(graph.audit_sensor.clone(), graph.tool.clone());
    selection.graphs.push(graph.clone());
    profile.sensors.push(sensor);
    profile.validate().unwrap();
    fs::write(&profile_path, serde_json::to_vec_pretty(&profile).unwrap()).unwrap();
    for (phase_name, manifest, rejected) in [
        (
            "nested-audit-failed",
            r#"{"name":"nested-fixture","version":"1.0.0","private":true,"devDependencies":{"lodash":"4.17.20"}}"#,
            true,
        ),
        (
            "nested-audit-corrected",
            r#"{"name":"nested-fixture","version":"1.0.0","private":true}"#,
            false,
        ),
    ] {
        fs::write(directory.join("package.json"), manifest).unwrap();
        let lock = command(
            &root.join("web"),
            &[
                "npm",
                "--prefix",
                "packages/extra",
                "install",
                "--package-lock-only",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
            ],
        );
        fs::write(
            output.join(format!("{phase_name}-lock.log")),
            [lock.stdout, lock.stderr].concat(),
        )
        .unwrap();
        assert!(lock.status.success());
        if rejected {
            let omitted = command(
                &root.join("web"),
                &[
                    "npm",
                    "--prefix",
                    "packages/extra",
                    "audit",
                    "--audit-level=low",
                ],
            );
            fs::write(
                output.join("nested-legacy-omit-audit.log"),
                [&omitted.stdout[..], &omitted.stderr[..]].concat(),
            )
            .unwrap();
            assert!(
                omitted.status.success(),
                "fixture must reproduce the old omission before testing the selected audit"
            );
            assert!(String::from_utf8_lossy(&omitted.stdout).contains("found 0 vulnerabilities"));
        }
        let install = graph.install_argv(&profile.selection.as_ref().unwrap().tools[&graph.tool]);
        let installed = command(
            &root.join("web"),
            &install.iter().map(String::as_str).collect::<Vec<_>>(),
        );
        fs::write(
            output.join(format!("{phase_name}-install.log")),
            [installed.stdout, installed.stderr].concat(),
        )
        .unwrap();
        assert!(installed.status.success());
        assert_eq!(
            directory.join("node_modules/lodash/package.json").exists(),
            rejected
        );
        commit(root, phase_name);
        let (_, result) = phase_expected(
            root,
            output,
            phase_name,
            if rejected {
                Some("frontend:extra-audit")
            } else {
                None
            },
        );
        if rejected {
            let audit = result
                .sensors
                .iter()
                .find(|s| s.evidence.sensor_id.as_str() == "frontend:extra-audit")
                .unwrap();
            let diagnostic = String::from_utf8_lossy(&audit.collection.record.execution.stdout);
            assert!(
                diagnostic.contains("lodash") && diagnostic.contains("Severity:"),
                "{diagnostic}"
            );
        }
    }
}
