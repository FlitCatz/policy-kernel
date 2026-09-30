mod common;
use project_verification::run::prepare;

#[test]
fn preflight_refuses_missing_changed_inputs_and_existing_report_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let request = common::request(root.path());
    assert!(prepare(&request).is_ok());
    std::fs::write(root.path().join("config.json"), b"changed").unwrap();
    assert!(prepare(&request).is_err());
    std::fs::remove_file(root.path().join("config.json")).unwrap();
    assert!(prepare(&request).is_err());
    std::fs::write(root.path().join("config.json"), b"{}").unwrap();
    std::fs::create_dir(root.path().join("reports")).unwrap();
    std::fs::write(root.path().join("reports/unit.json"), b"keep me").unwrap();
    assert!(prepare(&request).is_err());
    assert_eq!(
        std::fs::read(root.path().join("reports/unit.json")).unwrap(),
        b"keep me"
    );
}

#[cfg(unix)]
#[test]
fn preflight_refuses_symlink_inputs_components_and_report_directories() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("config.json"), b"{}").unwrap();
    let mut request = common::request(root.path());
    std::fs::remove_file(root.path().join("config.json")).unwrap();
    symlink(
        outside.path().join("config.json"),
        root.path().join("config.json"),
    )
    .unwrap();
    assert!(prepare(&request).is_err());
    std::fs::remove_file(root.path().join("config.json")).unwrap();
    std::fs::write(root.path().join("config.json"), b"{}").unwrap();
    symlink(outside.path(), root.path().join("reports")).unwrap();
    assert!(prepare(&request).is_err());
    std::fs::remove_file(root.path().join("reports")).unwrap();
    symlink(outside.path(), root.path().join("component")).unwrap();
    request.profile.components[0].root = "component".into();
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    assert!(prepare(&request).is_err());
    assert!(!outside.path().join("unit.json").exists());
}

#[test]
fn policy_binding_is_independent_of_the_checkout_location() {
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let a = prepare(&common::request(first.path())).unwrap();
    let b = prepare(&common::request(second.path())).unwrap();
    assert_ne!(
        a.configurations[0].component_root,
        b.configurations[0].component_root
    );
    assert_eq!(a.configurations[0].component_path, ".");
    assert_eq!(
        project_verification::run::configuration_digest(&a.configurations[0]).unwrap(),
        project_verification::run::configuration_digest(&b.configurations[0]).unwrap()
    );
    assert_eq!(
        policy_core::canonical_sha256(&a.bound_sensor_profile).unwrap(),
        policy_core::canonical_sha256(&b.bound_sensor_profile).unwrap()
    );

    // Input bytes and the declared component path remain bound.
    let mut changed = common::request(second.path());
    std::fs::write(second.path().join("config.json"), b"[]").unwrap();
    changed.source_files[0].digest = project_verification::bytes_digest(b"[]");
    changed.expected_source_digest = policy_core::canonical_sha256(&changed.source_files).unwrap();
    let c = prepare(&changed).unwrap();
    assert_ne!(
        project_verification::run::configuration_digest(&a.configurations[0]).unwrap(),
        project_verification::run::configuration_digest(&c.configurations[0]).unwrap()
    );
    let mut moved = a.configurations[0].clone();
    moved.component_path = "web".into();
    assert_ne!(
        project_verification::run::configuration_digest(&a.configurations[0]).unwrap(),
        project_verification::run::configuration_digest(&moved).unwrap()
    );
}
