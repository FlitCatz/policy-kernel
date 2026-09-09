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
