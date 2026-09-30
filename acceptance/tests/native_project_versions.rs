//! Opt-in native conformance for a project-owned frontend selection.
use policy_contracts::{ArtifactDigest, Risk, SensorStatus, TrustClass};
use policy_core::canonical_sha256;
use project_verification::{bytes_digest, parse_profile, RunRequest};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn command(root: &Path, program: &str, args: &[&str]) -> std::process::Output {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Project versions fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Project versions fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .output()
        .unwrap()
}

fn checked(root: &Path, program: &str, args: &[&str]) -> String {
    let output = command(root, program, args);
    assert!(
        output.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn copy(source: &Path, target: &Path) {
    fs::create_dir(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if [
            ".git",
            "node_modules",
            ".policy-reports",
            "test-results",
            "playwright-report",
        ]
        .iter()
        .any(|ignored| name == *ignored)
        {
            continue;
        }
        let destination = target.join(name);
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn profile(npm: &str) -> project_verification::ProjectProfile {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../examples/tjerp-compatible/project-profile.json"
    ))
    .unwrap();
    value["id"] = json!("react-typescript");
    value["schema_version"] = json!(4);
    value["components"] = json!([{"id":"frontend","root":"."}]);
    value["sensors"]
        .as_array_mut()
        .unwrap()
        .retain(|sensor| sensor["component"] == "frontend");
    let selection = &mut value["selection"];
    selection["schema_version"] = json!(3);
    selection.as_object_mut().unwrap().remove("consumer");
    selection["bindings"]
        .as_object_mut()
        .unwrap()
        .retain(|key, _| key.starts_with("frontend:"));
    selection["graphs"]
        .as_array_mut()
        .unwrap()
        .retain(|graph| graph["component"] == "frontend");
    selection["packages"]
        .as_array_mut()
        .unwrap()
        .retain(|package| package["component"] == "frontend");
    selection["runtime_files"]
        .as_array_mut()
        .unwrap()
        .retain(|file| file["component"] == "frontend");
    for package in selection["packages"].as_array_mut().unwrap() {
        package.as_object_mut().unwrap().remove("requirement");
        package["acceptance"] = json!({"kind":"record"});
    }
    let mut contracts = serde_json::Map::new();
    selection["tools"]
        .as_object_mut()
        .unwrap()
        .retain(|key, _| key.starts_with("frontend:"));
    for (id, tool) in selection["tools"].as_object_mut().unwrap() {
        let capabilities: Vec<_> = tool["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|capability| match capability.as_str().unwrap() {
                "process" => "process/v1",
                "npm_audit" => "npm-ci-audit/v1",
                "vitest_json" => "vitest-json/v1",
                "playwright_json" => "playwright-json/v1",
                other => panic!("unsupported fixture capability {other}"),
            })
            .collect();
        contracts.insert(id.clone(), json!(capabilities));
        tool.as_object_mut().unwrap().remove("producer_contract");
        let normalization = tool["version"]["normalization"].clone();
        tool["version"] = json!({"normalization":normalization,"acceptance":{"kind":"record"}});
        if id == "frontend:audit" {
            tool["executable"] = json!(npm);
        }
    }
    selection["contracts"] = Value::Object(contracts);
    for sensor in value["sensors"].as_array_mut().unwrap() {
        if sensor["definition"]["id"] == "frontend:audit" {
            sensor["argv"][0] = json!(npm);
            sensor["inputs"]
                .as_array_mut()
                .unwrap()
                .push(json!(".npmrc"));
        }
    }
    parse_profile(&value.to_string()).unwrap()
}

fn jest_profile() -> project_verification::ProjectProfile {
    let legacy = parse_profile(include_str!(
        "../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    let candidate = project_verification::selection::upgrade_frontend_profile(legacy).unwrap();
    let mut value = serde_json::to_value(candidate).unwrap();
    value["selection"]
        .as_object_mut()
        .unwrap()
        .remove("provisioning");
    for tool in value["selection"]["tools"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        tool["version"]["acceptance"] = json!({"kind":"record"});
    }
    for package in value["selection"]["packages"].as_array_mut().unwrap() {
        package["acceptance"] = json!({"kind":"record"});
    }
    parse_profile(&value.to_string()).unwrap()
}

fn request(root: &Path) -> RunRequest {
    let profile =
        parse_profile(&fs::read_to_string(root.join("project-profile.json")).unwrap()).unwrap();
    let head = checked(root, "git", &["rev-parse", "HEAD"]);
    let listing = checked(root, "git", &["ls-files", "-z"]);
    let mut source_files: Vec<ArtifactDigest> = listing
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| ArtifactDigest {
            path: path.into(),
            digest: bytes_digest(&fs::read(root.join(path)).unwrap()),
        })
        .collect();
    source_files.sort_by(|a, b| a.path.cmp(&b.path));
    serde_json::from_value(json!({"schema_version":1,"expected_profile_digest":canonical_sha256(&profile).unwrap(),
        "profile":profile,"root":root.canonicalize().unwrap(),"repository":"acceptance/react-project-versions",
        "head_sha":head,"source_files":source_files,"expected_source_digest":canonical_sha256(&source_files).unwrap(),
        "provider":"local","model":"native-project-versions","execution_id":format!("native-{head}"),
        "impact":{"kind":"unknown"},"mappings":{"acceptance_criteria":[],"changed_behaviors":[],"negative_case_ids":[]}})).unwrap()
}

fn commit(root: &Path, label: &str) {
    checked(root, "git", &["add", "--all"]);
    checked(root, "git", &["commit", "--quiet", "-m", label]);
}

fn retain_reports(output: &Path, label: &str, result: &project_verification::RunResult) {
    let destination = output.join(format!("{label}-reports"));
    fs::create_dir(&destination).unwrap();
    for sensor in &result.sensors {
        if let Some(report) = &sensor.result.original_report {
            let bytes = fs::read(sensor.configuration.component_root.join(&report.path)).unwrap();
            assert_eq!(bytes_digest(&bytes), report.digest);
            fs::write(
                destination.join(sensor.evidence.sensor_id.as_str().replace(':', "_")),
                bytes,
            )
            .unwrap();
        }
    }
}

fn phase(root: &Path, output: &Path, label: &str, expected_failure: Option<&str>) -> RunRequest {
    let request = request(root);
    for sensor in &request.profile.sensors {
        if let Some(report) = &sensor.report {
            let path = root.join(&report.path);
            if path.exists() {
                fs::remove_file(path).unwrap();
            }
        }
    }
    let result = project_verification::run(request.clone()).unwrap();
    retain_reports(output, label, &result);
    fs::write(
        output.join(format!("{label}-request.json")),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
    fs::write(
        output.join(format!("{label}-result.json")),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    for sensor in &result.sensors {
        assert_eq!(
            sensor.evidence.status == SensorStatus::Passed,
            expected_failure != Some(sensor.evidence.sensor_id.as_str()),
            "{label}: {} {:?}",
            sensor.evidence.sensor_id,
            sensor.result.errors
        );
    }
    request
}

fn run_candidate(workspace: &Path, output: &Path, source: &Path) {
    let root = output.join("candidate");
    copy(source, &root);
    let profile =
        parse_profile(&fs::read_to_string(root.join("project-profile.json")).unwrap()).unwrap();
    assert_eq!(profile.schema_version, 4);
    fs::write(
        root.join(".gitignore"),
        "node_modules/\n.policy-reports/\ntest-results/\nplaywright-report/\n",
    )
    .unwrap();
    let install = command(
        &root,
        "npm",
        &[
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--include=dev",
            "--include=optional",
            "--include=peer",
        ],
    );
    fs::write(
        output.join("candidate-install.log"),
        [&install.stdout[..], &install.stderr[..]].concat(),
    )
    .unwrap();
    assert!(install.status.success(), "candidate npm ci failed");
    checked(&root, "git", &["init", "--quiet"]);
    commit(&root, "candidate baseline");
    let request = request(&root);
    let mut known_impact = request.clone();
    known_impact.impact = project_verification::profile::Impact::Known {
        paths: vec![],
        risk: Risk::Low,
    };
    let prepared = project_verification::run::prepare(&known_impact).unwrap();
    let selection = request.profile.selection.as_ref().unwrap();
    for runtime in &selection.runtime_files {
        assert!(prepared
            .selected_sensor_ids
            .iter()
            .any(|id| selection.bindings.get(id.as_str()) == Some(&runtime.tool)));
    }
    let result = project_verification::run(request.clone()).unwrap();
    retain_reports(output, "candidate", &result);
    fs::write(
        output.join("candidate-request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
    fs::write(
        output.join("candidate-result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    for sensor in &result.sensors {
        assert_eq!(sensor.evidence.status, SensorStatus::Passed);
        assert_eq!(sensor.evidence.trust, TrustClass::LocalUntrusted);
        if let Some(report) = &sensor.configuration.sensor.report {
            let path = sensor.configuration.component_root.join(&report.path);
            let original = fs::read(&path).unwrap();
            fs::write(&path, b"").unwrap();
            let mut collection = sensor.collection.clone();
            collection.record.artifacts[0].digest = bytes_digest(b"");
            collection.evidence.output_digest = canonical_sha256(&collection.record).unwrap();
            assert_ne!(
                project_verification::run::normalize_collection(
                    &request,
                    sensor.configuration.clone(),
                    collection,
                )
                .unwrap()
                .evidence
                .status,
                SensorStatus::Passed
            );
            fs::write(path, original).unwrap();
        }
    }
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&json!({
            "status":"passed", "scope":"candidate-native",
            "node":checked(&root,"node", &["--version"]),
            "npm":checked(&root,"npm", &["--version"]),
            "profile_digest":canonical_sha256(&profile).unwrap(),
            "source_digest":request.expected_source_digest,
            "candidate_source":source.canonicalize().unwrap(),
            "harness":policy_acceptance::harness::observe(workspace).unwrap()
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "requires Node 26.10.0, cached npm 10.9.3, npm 12.1.0, browser and audit network"]
fn native_project_versions() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let output = PathBuf::from(
        std::env::var_os("POLICY_PROJECT_VERSIONS_OUTPUT").expect("set a fresh output directory"),
    );
    fs::create_dir(&output).unwrap();
    if let Some(source) = std::env::var_os("POLICY_PROJECT_VERSIONS_CANDIDATE") {
        run_candidate(workspace, &output, &PathBuf::from(source));
        return;
    }
    assert_eq!(checked(&output, "node", &["--version"]), "v26.10.0");
    assert_eq!(checked(&output, "npm", &["--version"]), "12.1.0");
    let npm10 = if let Some(path) = std::env::var_os("POLICY_PROJECT_VERSIONS_NPM10") {
        PathBuf::from(path)
    } else {
        let npm10_prefix = output.join("npm10");
        fs::create_dir(&npm10_prefix).unwrap();
        let install_npm10 = command(
            &output,
            "npm",
            &[
                "install",
                "--prefix",
                npm10_prefix.to_str().unwrap(),
                "npm@10.9.3",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
            ],
        );
        fs::write(
            output.join("npm10-install.log"),
            [&install_npm10.stdout[..], &install_npm10.stderr[..]].concat(),
        )
        .unwrap();
        assert!(
            install_npm10.status.success(),
            "npm 10.9.3 installation failed"
        );
        npm10_prefix.join("node_modules/.bin/npm")
    };
    assert_eq!(
        checked(&output, npm10.to_str().unwrap(), &["--version"]),
        "10.9.3"
    );
    for (label, npm) in [("npm10", npm10.to_str().unwrap()), ("npm12", "npm")] {
        let root = output.join(label);
        copy(&workspace.join("examples/react-project-versions"), &root);
        fs::write(
            root.join("project-profile.json"),
            serde_json::to_vec_pretty(&profile(npm)).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join(".gitignore"),
            "node_modules/\n.policy-reports/\ntest-results/\nplaywright-report/\n",
        )
        .unwrap();
        let install = command(
            &root,
            npm,
            &[
                "ci",
                "--ignore-scripts",
                "--no-audit",
                "--no-fund",
                "--include=dev",
                "--include=optional",
                "--include=peer",
            ],
        );
        fs::write(
            output.join(format!("{label}-install.log")),
            [&install.stdout[..], &install.stderr[..]].concat(),
        )
        .unwrap();
        assert!(install.status.success(), "{label} npm ci failed");
        checked(&root, "git", &["init", "--quiet"]);
        checked(&root, "git", &["add", "--all"]);
        checked(&root, "git", &["commit", "--quiet", "-m", "baseline"]);
        let request = request(&root);
        let result = project_verification::run(request.clone()).unwrap();
        retain_reports(&output, label, &result);
        fs::write(
            output.join(format!("{label}-request.json")),
            serde_json::to_vec_pretty(&request).unwrap(),
        )
        .unwrap();
        fs::write(
            output.join(format!("{label}-result.json")),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        for sensor in &result.sensors {
            assert_eq!(
                sensor.evidence.status,
                SensorStatus::Passed,
                "{label}: {} {:?}",
                sensor.evidence.sensor_id,
                sensor.result.errors
            );
            assert_eq!(sensor.evidence.trust, TrustClass::LocalUntrusted);
            if let Some(report) = &sensor.configuration.sensor.report {
                let path = root.join(&report.path);
                let original = fs::read(&path).unwrap();
                for bytes in [b"".as_slice(), &original[..original.len() / 2]] {
                    fs::write(&path, bytes).unwrap();
                    let mut collection = sensor.collection.clone();
                    collection.record.artifacts[0].digest = bytes_digest(bytes);
                    collection.evidence.output_digest =
                        canonical_sha256(&collection.record).unwrap();
                    assert_ne!(
                        project_verification::run::normalize_collection(
                            &request,
                            sensor.configuration.clone(),
                            collection
                        )
                        .unwrap()
                        .evidence
                        .status,
                        SensorStatus::Passed
                    );
                }
                fs::remove_file(&path).unwrap();
                assert_ne!(
                    project_verification::run::normalize_collection(
                        &request,
                        sensor.configuration.clone(),
                        sensor.collection.clone()
                    )
                    .unwrap()
                    .evidence
                    .status,
                    SensorStatus::Passed
                );
                fs::write(path, original).unwrap();
                let mut stale = request.clone();
                stale.head_sha = serde_json::from_value(json!("b".repeat(40))).unwrap();
                assert!(project_verification::run::normalize_collection(
                    &stale,
                    sensor.configuration.clone(),
                    sensor.collection.clone()
                )
                .is_err());
            }
        }
        if label == "npm12" {
            let test_path = root.join("src/Greeting.test.ts");
            let original = fs::read_to_string(&test_path).unwrap();
            fs::write(
                &test_path,
                original.replace("<h1>Hello policy</h1>", "<h1>Wrong greeting</h1>"),
            )
            .unwrap();
            commit(&root, "failing unit test");
            phase(&root, &output, "unit-failure", Some("frontend:unit"));
            fs::write(&test_path, &original).unwrap();
            commit(&root, "corrected unit test");
            phase(&root, &output, "unit-corrected", None);

            let e2e_path = root.join("e2e/greeting.spec.ts");
            let config_path = root.join("playwright.config.ts");
            let e2e_original = fs::read_to_string(&e2e_path).unwrap();
            let config_original = fs::read_to_string(&config_path).unwrap();
            fs::write(
                &config_path,
                config_original.replace("retries: 0", "retries: 1"),
            )
            .unwrap();
            fs::write(&e2e_path, r#"import { expect, test } from '@playwright/test';
import { existsSync, writeFileSync } from 'node:fs';
test('retries the browser check', async ({page}) => {
  const marker = '.policy-reports/retry-marker';
  if (!existsSync(marker)) { writeFileSync(marker, 'first attempt'); throw new Error('first attempt'); }
  await page.goto('/');
  await expect(page.getByRole('heading')).toHaveText('Hello policy');
});
"#).unwrap();
            commit(&root, "browser retry");
            phase(&root, &output, "browser-retry", Some("frontend:e2e"));
            fs::write(&e2e_path, e2e_original).unwrap();
            fs::write(&config_path, config_original).unwrap();
            fs::remove_file(root.join(".policy-reports/retry-marker")).unwrap();
            commit(&root, "corrected browser test");
            phase(&root, &output, "browser-corrected", None);

            for class in ["dev", "optional", "peer"] {
                let mut invalid: Value =
                    serde_json::from_slice(&fs::read(root.join("project-profile.json")).unwrap())
                        .unwrap();
                invalid["sensors"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|sensor| sensor["definition"]["id"] == "frontend:audit")
                    .unwrap()["argv"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|arg| arg != &format!("--include={class}"));
                assert!(
                    parse_profile(&invalid.to_string()).is_err(),
                    "audit must retain {class}"
                );
            }
            let npmrc = root.join(".npmrc");
            let original_npmrc = fs::read(&npmrc).unwrap();
            fs::write(&npmrc, "registry=http://127.0.0.1:9/\nfetch-retries=0\n").unwrap();
            commit(&root, "audit service unavailable");
            phase(
                &root,
                &output,
                "audit-service-error",
                Some("frontend:audit"),
            );
            fs::write(&npmrc, original_npmrc).unwrap();
            commit(&root, "audit service restored");
            phase(&root, &output, "audit-restored", None);

            let install = command(
                &root,
                npm,
                &[
                    "install",
                    "lodash@4.17.20",
                    "--package-lock-only",
                    "--ignore-scripts",
                    "--no-audit",
                    "--no-fund",
                ],
            );
            fs::write(
                output.join("advisory-lock.log"),
                [&install.stdout[..], &install.stderr[..]].concat(),
            )
            .unwrap();
            assert!(
                install.status.success(),
                "failed to prepare vulnerable lockfile"
            );
            let installed = command(
                &root,
                npm,
                &[
                    "ci",
                    "--ignore-scripts",
                    "--no-audit",
                    "--no-fund",
                    "--include=dev",
                    "--include=optional",
                    "--include=peer",
                ],
            );
            fs::write(
                output.join("advisory-install.log"),
                [&installed.stdout[..], &installed.stderr[..]].concat(),
            )
            .unwrap();
            assert!(
                installed.status.success(),
                "failed to install vulnerable lockfile"
            );
            commit(&root, "vulnerable dependency");
            phase(&root, &output, "audit-advisory", Some("frontend:audit"));
        }
    }
    let jest = output.join("jest");
    copy(&workspace.join("examples/react-jest"), &jest);
    fs::write(jest.join("web/.node-version"), "26.10.0\n").unwrap();
    fs::write(
        jest.join("project-profile.json"),
        serde_json::to_vec_pretty(&jest_profile()).unwrap(),
    )
    .unwrap();
    fs::write(jest.join(".gitignore"), "node_modules/\njest-report.json\n").unwrap();
    let install = command(
        &jest.join("web"),
        "npm",
        &[
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--include=dev",
            "--include=optional",
            "--include=peer",
        ],
    );
    fs::write(
        output.join("jest-install.log"),
        [&install.stdout[..], &install.stderr[..]].concat(),
    )
    .unwrap();
    assert!(install.status.success(), "Jest fixture installation failed");
    checked(&jest, "git", &["init", "--quiet"]);
    commit(&jest, "Jest baseline");
    let request = request(&jest);
    let result = project_verification::run(request.clone()).unwrap();
    retain_reports(&output, "jest", &result);
    fs::write(
        output.join("jest-request.json"),
        serde_json::to_vec_pretty(&request).unwrap(),
    )
    .unwrap();
    fs::write(
        output.join("jest-result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    for sensor in &result.sensors {
        assert_eq!(
            sensor.evidence.status,
            SensorStatus::Passed,
            "Jest: {} {:?}",
            sensor.evidence.sensor_id,
            sensor.result.errors
        );
    }
    fs::write(output.join("summary.json"), serde_json::to_vec_pretty(&json!({"status":"passed","scope":"local-native-advisory","node":"26.10.0","npm":["10.9.3","12.1.0"],"jest":"30.2.0","harness":policy_acceptance::harness::observe(workspace).unwrap()})).unwrap()).unwrap();
}
