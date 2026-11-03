//! Shared alternate-runtime fixture execution and nested npm omission regression.
use policy_contracts::{ArtifactDigest, SensorStatus, TrustClass};
use policy_core::canonical_sha256;
use project_verification::{bytes_digest, parse_profile, RunRequest, RunResult};
use serde_json::json;
use std::{fs, path::Path, process::Command};

pub fn command(root: &Path, argv: &[&str]) -> std::process::Output {
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
pub fn git(root: &Path, args: &[&str]) -> String {
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
pub fn commit(root: &Path, phase: &str) {
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
        "profile":profile,"root":root.canonicalize().unwrap(),"repository":"acceptance/alternate-frontend",
        "head_sha":head,"source_files":files,"expected_source_digest":canonical_sha256(&files).unwrap(),
        "provider":"local","model":"native-alternate","execution_id":format!("native-{head}"),
        "impact":{"kind":"unknown"},"mappings":{"acceptance_criteria":[],"changed_behaviors":[],"negative_case_ids":[]}})).unwrap()
}
pub fn phase(root: &Path, output: &Path, name: &str, pass: bool) -> (RunRequest, RunResult) {
    phase_expected(
        root,
        output,
        name,
        if pass { None } else { Some("frontend:unit") },
    )
}

pub fn phase_expected(
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

pub fn nested_graph_audit(root: &Path, output: &Path) {
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
