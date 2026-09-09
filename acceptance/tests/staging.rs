use policy_acceptance::stage;
use std::fs;

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
