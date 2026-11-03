//! Opt-in native acceptance: the ordinary offline suite never substitutes for this run.
#[path = "common/alternate.rs"]
mod alternate;
use alternate::{command, commit, git, nested_graph_audit, phase};
use project_verification::bytes_digest;
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
#[ignore = "requires provisioned Node/npm, npm advisory network, and POLICY_VITEST_OUTPUT"]
fn native_vitest_selected_paths_failures_corrections_and_prior_head_rejection() {
    let output = PathBuf::from(
        std::env::var_os("POLICY_VITEST_OUTPUT").expect("set a new output directory"),
    );
    fs::create_dir(&output).unwrap();
    for (tool, expected) in [("node", "v22.19.0"), ("npm", "10.9.4")] {
        let probe = command(&output, &[tool, "--version"]);
        assert!(probe.status.success());
        assert_eq!(String::from_utf8(probe.stdout).unwrap().trim(), expected);
    }

    let root = output.join("consumer");
    fs::create_dir(&root).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let source = workspace.join("examples/react-vitest");
    for path in [
        "project-profile.json",
        "web/package.json",
        "web/package-lock.json",
        "web/.node-version",
        "web/vitest.config.ts",
        "web/src/Greeting.cjs",
        "web/tests/Greeting.test.mjs",
    ] {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(source.join(path), target).unwrap();
    }
    fs::write(
        root.join(".gitignore"),
        "node_modules/\n.policy-reports/\nvitest-report.json\n",
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
    let test_path = root.join("web/tests/Greeting.test.mjs");
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
    let (_, skipped) = phase(&root, &output, "skipped", false);
    let unit = skipped
        .sensors
        .iter()
        .find(|s| s.evidence.sensor_id.as_str() == "frontend:unit")
        .unwrap();
    assert_eq!(unit.collection.record.execution.exit_code, Some(0));
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("skipped/original-report.json")).unwrap())
            .unwrap();
    assert_eq!(report["success"], true);
    assert_eq!(report["numPendingTests"], 2);
    assert_eq!(report["numPassedTests"], 0);
    fs::write(&test_path, &original).unwrap();
    let original_profile = fs::read_to_string(root.join("project-profile.json")).unwrap();
    let mut profile: serde_json::Value = serde_json::from_str(&original_profile).unwrap();
    profile["sensors"][2]["argv"] = json!([
        "node",
        "node_modules/vitest/vitest.mjs",
        "run",
        "--reporter=json",
        "--outputFile=vitest-report.json",
        "does-not-exist",
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
        "harness":policy_acceptance::harness::observe(workspace).unwrap(),"cases":["baseline","failed","corrected","skipped","empty","missing","final-corrected","nested-audit-failed","nested-audit-corrected"],"previous_head_rejected":true})).unwrap()).unwrap();
}
