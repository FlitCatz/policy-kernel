use policy_acceptance::stage;
use std::fs;

#[test]
fn each_dependency_graph_failure_or_missing_audit_blocks_fullstack_acceptance() {
    use policy_contracts::{Risk, SensorEvidence};
    use policy_core::{evaluate_sensor_evidence_for_change, SensorEvaluationStatus};
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().canonicalize().unwrap().join("consumer");
    let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples");
    stage(&examples, "fullstack", &target).unwrap();
    let profile = project_verification::parse_profile(
        &fs::read_to_string(target.join(".policy/project-profile.json")).unwrap(),
    )
    .unwrap()
    .sensor_profile();
    let evidence: Vec<SensorEvidence> = profile
        .sensors
        .iter()
        .map(|sensor| {
            serde_json::from_value(serde_json::json!({
                "sensor_id": sensor.id, "status": "passed", "head_sha": "a".repeat(40),
                "command_digest": "b".repeat(64), "result_digest": "c".repeat(64),
                "observed_at": "2026-09-10T00:00:00Z", "trust": "local_untrusted",
                "resolved_command": ["tool"], "source_digest": "d".repeat(64),
                "configuration_digest": "e".repeat(64), "parser_version": "1", "exit_code": 0,
                "discovered": 1, "executed": 1, "passed": 1
            }))
            .unwrap()
        })
        .collect();
    let evaluate = |items: &[SensorEvidence]| {
        evaluate_sensor_evidence_for_change(
            &profile,
            &["package-lock.json"],
            Risk::Low,
            items,
            None,
        )
        .unwrap()
    };
    assert_eq!(evaluate(&evidence).status, SensorEvaluationStatus::Passed);
    for audit in [
        "frontend:dependencies",
        "shared:dependencies",
        "backend:dependencies",
        "backend:unit-dependencies",
        "backend:integration-dependencies",
    ] {
        let mut changed = evidence.clone();
        let item = changed
            .iter_mut()
            .find(|item| item.sensor_id.as_str() == audit)
            .unwrap();
        item.status = policy_contracts::SensorStatus::Failed;
        item.exit_code = Some(1);
        let failed = evaluate(&changed);
        assert_eq!(failed.status, SensorEvaluationStatus::Blocked, "{audit}");
        assert!(failed.has_reason("PK-SENSOR-FAILED"), "{audit}");
        for status in [
            policy_contracts::SensorStatus::TimedOut,
            policy_contracts::SensorStatus::Skipped,
        ] {
            changed
                .iter_mut()
                .find(|item| item.sensor_id.as_str() == audit)
                .unwrap()
                .status = status;
            assert_ne!(
                evaluate(&changed).status,
                SensorEvaluationStatus::Passed,
                "{audit}"
            );
        }
        changed.retain(|item| item.sensor_id.as_str() != audit);
        assert!(
            evaluate(&changed).has_reason("PK-SENSOR-MISSING"),
            "{audit}"
        );
    }
}

#[test]
fn installs_real_profile_without_clobbering_an_existing_destination() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp = tmp.path().canonicalize().unwrap();
    let examples = tmp.join("examples");
    fs::create_dir_all(examples.join("react-typescript/src")).unwrap();
    fs::write(
        examples.join("react-typescript/package.json"),
        r#"{"devDependencies":{"vite":"7.3.6"}}"#,
    )
    .unwrap();
    fs::write(
        examples.join("react-typescript/src/fixture.txt"),
        "consumer owned",
    )
    .unwrap();
    let target = tmp.join("consumer");
    stage(&examples, "react-typescript", &target).unwrap();
    let profile = fs::read_to_string(target.join(".policy/project-profile.json")).unwrap();
    let profile = project_verification::parse_profile(&profile).unwrap();
    assert!(profile
        .sensors
        .iter()
        .any(|s| s.definition.id.as_str() == "frontend:unit"));
    assert!(target.join(".policy/bootstrap-manifest.json").is_file());
    fs::write(target.join("src/fixture.txt"), "user edit").unwrap();
    assert!(stage(&examples, "react-typescript", &target).is_err());
    assert_eq!(
        fs::read_to_string(target.join("src/fixture.txt")).unwrap(),
        "user edit"
    );
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_fixture_without_copying_external_content() {
    let tmp = tempfile::tempdir().unwrap();
    let tmp = tmp.path().canonicalize().unwrap();
    let examples = tmp.join("examples");
    fs::create_dir_all(examples.join("react-typescript")).unwrap();
    fs::write(tmp.join("private"), "private").unwrap();
    std::os::unix::fs::symlink(tmp.join("private"), examples.join("react-typescript/leak"))
        .unwrap();
    let target = tmp.join("consumer");
    assert!(stage(&examples, "react-typescript", &target).is_err());
    assert!(!target.join("leak").exists());
}

#[test]
fn rejects_destination_inside_fixture_tree_before_creating_it() {
    let tmp = tempfile::tempdir().unwrap();
    let examples = tmp.path().join("examples");
    fs::create_dir_all(examples.join("react-typescript")).unwrap();
    let target = examples.join("react-typescript/generated");
    // An invalid child destination must not recursively copy its own output.
    assert!(stage(&examples, "dotnet", &target).is_err());
    assert!(!target.exists());
}
