#![cfg(unix)]
use policy_bootstrap::{apply, plan, recover, Action, Installation, Operation, Request};
use std::{collections::BTreeMap, fs, path::Path};

fn request(target: &Path, profile: &str) -> Request {
    Request {
        target: target.canonicalize().unwrap(),
        installation: Installation {
            profile: profile.into(),
            release: "1.0.0".into(),
            components: BTreeMap::new(),
        },
        operation: Operation::Install,
    }
}

// Catches losing receipt ownership across releases or retaining vulnerable pins.
#[test]
fn security_release_upgrades_old_receipts_and_uninstalls_cleanly() {
    for profile in ["react-typescript", "dotnet", "fullstack"] {
        let temp = tempfile::tempdir().unwrap();
        let old = request(temp.path(), profile);
        apply(&plan(old.clone()).unwrap()).unwrap();
        let frozen_receipt = match profile {
            "react-typescript" => include_bytes!(
                "../../tests/fixtures/bootstrap/receipts-1.0.0/react-typescript.json"
            )
            .as_slice(),
            "dotnet" => include_bytes!("../../tests/fixtures/bootstrap/receipts-1.0.0/dotnet.json")
                .as_slice(),
            _ => include_bytes!("../../tests/fixtures/bootstrap/receipts-1.0.0/fullstack.json")
                .as_slice(),
        };
        let receipt = temp.path().join(".policy/bootstrap-manifest.json");
        assert_eq!(fs::read(&receipt).unwrap(), frozen_receipt);
        fs::write(&receipt, frozen_receipt).unwrap();
        let mut updated = old.clone();
        updated.installation.release = "1.0.1".into();
        let upgrade = plan(updated.clone()).unwrap();
        assert!(upgrade.conflicts.is_empty(), "{:?}", upgrade.conflicts);
        assert!(upgrade.files.iter().any(|f| f.action == Action::Update));
        apply(&upgrade).unwrap();
        assert!(plan(updated.clone())
            .unwrap()
            .files
            .iter()
            .all(|f| f.action == Action::Unchanged));
        // Old inventory must still be usable, including a downgrade and re-upgrade.
        apply(&plan(old).unwrap()).unwrap();
        apply(&plan(updated.clone()).unwrap()).unwrap();
        updated.operation = Operation::Uninstall;
        apply(&plan(updated).unwrap()).unwrap();
        assert!(!temp.path().join(".policy/bootstrap-manifest.json").exists());
    }
}

#[test]
fn cross_release_upgrade_preserves_locally_edited_managed_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut req = request(temp.path(), "react-typescript");
    apply(&plan(req.clone()).unwrap()).unwrap();
    let receipt = fs::read(temp.path().join(".policy/bootstrap-manifest.json")).unwrap();
    fs::write(temp.path().join("vitest.config.ts"), "consumer edit").unwrap();
    req.installation.release = "1.0.1".into();
    let upgrade = plan(req).unwrap();
    assert!(!upgrade.conflicts.is_empty());
    assert!(apply(&upgrade).is_err());
    assert_eq!(
        fs::read_to_string(temp.path().join("vitest.config.ts")).unwrap(),
        "consumer edit"
    );
    assert_eq!(
        fs::read(temp.path().join(".policy/bootstrap-manifest.json")).unwrap(),
        receipt
    );
}

#[test]
fn security_release_requires_updated_consumer_dependencies() {
    let temp = tempfile::tempdir().unwrap();
    let mut req = request(temp.path(), "react-typescript");
    req.installation.release = "1.0.1".into();
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies":{"vite":"7.1.3","vitest":"3.2.4"}}"#,
    )
    .unwrap();
    let outdated = plan(req.clone()).unwrap();
    assert!(!outdated.conflicts.is_empty());
    assert!(apply(&outdated).is_err());
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies":{"vite":"7.3.6","vitest":"4.1.11"}}"#,
    )
    .unwrap();
    let updated = plan(req).unwrap();
    assert!(updated.conflicts.is_empty(), "{:?}", updated.conflicts);
    apply(&updated).unwrap();
}

// Catches incorrect composition and accidental application scaffolding.
#[test]
fn empty_profiles_install_configuration_and_repeat_without_writes() {
    for profile in ["react-typescript", "dotnet", "fullstack"] {
        let temp = tempfile::tempdir().unwrap();
        let req = request(temp.path(), profile);
        let first = plan(req.clone()).unwrap();
        assert!(first.conflicts.is_empty(), "{:?}", first.conflicts);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
        assert!(!first.prerequisites.is_empty());
        apply(&first).unwrap();
        let profile_bytes = fs::read(temp.path().join(".policy/project-profile.json")).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&profile_bytes).unwrap();
        assert_eq!(value["id"], profile);
        assert_eq!(
            value["components"].as_array().unwrap().len(),
            if profile == "fullstack" { 3 } else { 1 }
        );
        assert!(!temp.path().join("package.json").exists());
        assert!(!temp.path().join("App.csproj").exists());
        let receipt = temp.path().join(".policy/bootstrap-manifest.json");
        let before = fs::metadata(&receipt).unwrap().modified().unwrap();
        let second = plan(req).unwrap();
        assert!(second.conflicts.is_empty(), "{:?}", second.conflicts);
        assert!(second.files.iter().all(|f| f.action == Action::Unchanged));
        apply(&second).unwrap();
        assert_eq!(fs::metadata(receipt).unwrap().modified().unwrap(), before);
    }
}

// Catches overwriting unowned files or project manifests.
#[test]
fn existing_project_preserves_manifests_and_conflicts_on_unowned_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let package = include_bytes!("../../tests/fixtures/bootstrap/existing-react/package.json");
    fs::write(temp.path().join("package.json"), package).unwrap();
    let req = request(temp.path(), "react-typescript");
    let first = plan(req.clone()).unwrap();
    assert!(first.conflicts.is_empty(), "{:?}", first.conflicts);
    apply(&first).unwrap();
    assert_eq!(fs::read(temp.path().join("package.json")).unwrap(), package);
    fs::write(temp.path().join(".node-version"), "custom\n").unwrap();
    let conflict = plan(req).unwrap();
    assert!(!conflict.conflicts.is_empty());
    assert!(apply(&conflict).is_err());
    assert_eq!(
        fs::read_to_string(temp.path().join(".node-version")).unwrap(),
        "custom\n"
    );
}

// Catches guessing a framework and any partial apply on conflict.
#[test]
fn ambiguity_and_unmanaged_file_conflicts_do_not_write() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        include_bytes!("../../tests/fixtures/bootstrap/ambiguous/package.json"),
    )
    .unwrap();
    let conflict = plan(request(temp.path(), "react-typescript")).unwrap();
    assert!(!conflict.conflicts.is_empty());
    assert!(apply(&conflict).is_err());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    fs::remove_file(temp.path().join("package.json")).unwrap();
    fs::write(temp.path().join(".editorconfig"), "root = true\n").unwrap();
    let conflict = plan(request(temp.path(), "react-typescript")).unwrap();
    assert!(conflict
        .files
        .iter()
        .any(|f| f.path == ".editorconfig" && f.action == Action::Conflict));
    assert!(apply(&conflict).is_err());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

// Catches stale input digests, newly discovered manifests, and forged plan contents.
#[test]
fn stale_and_tampered_plans_are_rejected_before_writes() {
    let temp = tempfile::tempdir().unwrap();
    let req = request(temp.path(), "react-typescript");
    let saved = plan(req.clone()).unwrap();
    fs::write(temp.path().join("package.json"), "{}").unwrap();
    assert!(apply(&saved).is_err());
    assert!(!temp.path().join(".policy").exists());
    fs::remove_file(temp.path().join("package.json")).unwrap();
    let mut tampered = plan(req).unwrap();
    assert!(tampered.conflicts.is_empty());
    tampered.files[0].content = Some("injected".into());
    assert!(apply(&tampered).is_err());
    assert!(!temp.path().join(".policy").exists());
}

// Catches path rewriting omissions and accidental acceptance of escape spellings.
#[test]
fn explicit_component_paths_are_bound_and_unsafe_paths_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let mut req = request(temp.path(), "fullstack");
    req.installation
        .components
        .insert("frontend".into(), "apps/web".into());
    req.installation
        .components
        .insert("backend".into(), "apps/api".into());
    let planned = plan(req.clone()).unwrap();
    apply(&planned).unwrap();
    let value: serde_json::Value = serde_json::from_slice(
        &fs::read(temp.path().join(".policy/project-profile.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(value["components"][0]["root"], "apps/web");
    assert_eq!(value["components"][1]["root"], "apps/api");
    for path in [
        "../outside",
        "/outside",
        "apps/../../out",
        "a\\b",
        "a//b",
        "C:drive",
    ] {
        req.installation
            .components
            .insert("frontend".into(), path.into());
        assert!(plan(req.clone()).is_err(), "{path}");
    }
}

#[cfg(unix)]
#[test]
fn symlinks_at_target_component_and_output_are_rejected() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let req = request(temp.path(), "fullstack");
    symlink(outside.path(), temp.path().join("frontend")).unwrap();
    assert!(plan(req.clone()).is_err());
    fs::remove_file(temp.path().join("frontend")).unwrap();
    let saved = plan(req).unwrap();
    symlink(outside.path(), temp.path().join(".policy")).unwrap();
    assert!(apply(&saved).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    let link = temp.path().canonicalize().unwrap().join("target-link");
    symlink(outside.path(), &link).unwrap();
    let mut req = request(temp.path(), "dotnet");
    req.target = link;
    assert!(plan(req).is_err());
}

// Catches uninstall deleting edits/unowned files and component migration losing files.
#[test]
fn managed_configuration_can_move_and_uninstall_preserves_project_files() {
    let temp = tempfile::tempdir().unwrap();
    let mut req = request(temp.path(), "dotnet");
    apply(&plan(req.clone()).unwrap()).unwrap();
    req.installation
        .components
        .insert("backend".into(), "api".into());
    let update = plan(req.clone()).unwrap();
    assert!(update
        .files
        .iter()
        .any(|f| f.action == Action::Update && f.previous.is_some()));
    apply(&update).unwrap();
    assert!(!temp.path().join("global.json").exists());
    assert!(temp.path().join("api/global.json").exists());
    fs::write(temp.path().join("business.txt"), "keep me").unwrap();
    req.operation = Operation::Uninstall;
    apply(&plan(req).unwrap()).unwrap();
    assert!(!temp.path().join("api/global.json").exists());
    assert!(!temp.path().join(".policy/bootstrap-manifest.json").exists());
    assert_eq!(
        fs::read_to_string(temp.path().join("business.txt")).unwrap(),
        "keep me"
    );
    assert!(recover(temp.path()).is_err());
}

#[test]
fn unsupported_release_and_overlapping_application_roots_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let mut req = request(temp.path(), "dotnet");
    req.installation.release = "latest".into();
    assert!(plan(req).is_err());
    let mut req = request(temp.path(), "fullstack");
    req.installation
        .components
        .insert("backend".into(), "frontend/api".into());
    assert!(plan(req).is_err());
}

// Catches silently migrating a runtime or treating a comment as project configuration.
#[test]
fn existing_dotnet_requires_an_unambiguous_compatible_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let req = request(temp.path(), "dotnet");
    let project = temp.path().join("App.csproj");
    let fixture = include_bytes!("../../tests/fixtures/bootstrap/existing-dotnet/App.csproj");
    fs::write(&project, fixture).unwrap();
    let compatible = plan(req.clone()).unwrap();
    assert!(compatible.conflicts.is_empty());
    for xml in [
        "<Project><!-- <TargetFramework>net9.0</TargetFramework> --><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>",
        "<Project><PropertyGroup><TargetFrameworks>net9.0;net8.0</TargetFrameworks></PropertyGroup></Project>",
        "<Project><PropertyGroup Condition=\"'$(Flavor)' == 'one'\"><TargetFramework>net9.0</TargetFramework></PropertyGroup></Project>",
    ] {
        fs::write(&project, xml).unwrap();
        let conflict = plan(req.clone()).unwrap();
        assert!(!conflict.conflicts.is_empty());
        assert!(apply(&conflict).is_err());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    }
    fs::write(&project, fixture).unwrap();
    apply(&plan(req).unwrap()).unwrap();
    assert_eq!(fs::read(project).unwrap(), fixture);
}

#[test]
fn planned_file_directory_aliases_fail_without_target_mutation() {
    let temp = tempfile::tempdir().unwrap();
    for component in [".editorconfig", ".POLICY", ".Git"] {
        let mut req = request(temp.path(), "dotnet");
        req.installation
            .components
            .insert("backend".into(), component.into());
        assert!(plan(req).is_err());
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    }
    let mut req = request(temp.path(), "fullstack");
    req.installation
        .components
        .insert("frontend".into(), "app".into());
    req.installation
        .components
        .insert("backend".into(), "APP".into());
    assert!(plan(req).is_err());
}

#[test]
fn mismatched_existing_lockfile_is_a_conflict_not_a_toolchain_migration() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("package-lock.json"),
        r#"{"packages":{"node_modules/typescript":{"version":"4.9.5"}}}"#,
    )
    .unwrap();
    let planned = plan(request(temp.path(), "react-typescript")).unwrap();
    assert!(!planned.conflicts.is_empty());
    assert!(apply(&planned).is_err());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

// Catches installing a profile whose command/configuration bindings omit installed defaults.
#[test]
fn rendered_fullstack_binds_installed_native_configurations_and_report_paths() {
    let temp = tempfile::tempdir().unwrap();
    let saved = plan(request(temp.path(), "fullstack")).unwrap();
    apply(&saved).unwrap();
    let text = fs::read_to_string(temp.path().join(".policy/project-profile.json")).unwrap();
    let profile = project_verification::parse_profile(&text).unwrap();
    let lint = profile
        .sensors
        .iter()
        .find(|s| s.definition.id.as_str() == "frontend:lint")
        .unwrap();
    assert_eq!(
        &lint.argv[lint.argv.len() - 2..],
        ["--config", "eslint.config.mjs"]
    );
    assert!(lint.pins.iter().any(|p| p.path
        == "node_modules/@typescript-eslint/parser/package.json"
        && p.expected == "8.42.0"));
    for path in [
        "frontend/eslint.config.mjs",
        "frontend/vite.config.ts",
        "frontend/vitest.config.ts",
        "vitest.contract.config.ts",
        "playwright.config.ts",
    ] {
        assert!(temp.path().join(path).is_file(), "{path}");
        assert!(!saved.prerequisites.iter().any(|p| p == path));
    }
    let e2e = profile
        .sensors
        .iter()
        .find(|s| s.definition.id.as_str() == "shared:e2e")
        .unwrap();
    assert_eq!(
        e2e.report.as_ref().unwrap().path,
        ".policy-reports/e2e.json"
    );
    assert!(!temp.path().join(".github/workflows").exists());
}

#[test]
fn edited_uninstall_is_all_or_nothing_and_missing_managed_files_can_be_reinstalled() {
    let temp = tempfile::tempdir().unwrap();
    let mut req = request(temp.path(), "dotnet");
    apply(&plan(req.clone()).unwrap()).unwrap();
    fs::remove_file(temp.path().join("global.json")).unwrap();
    apply(&plan(req.clone()).unwrap()).unwrap();
    assert!(temp.path().join("global.json").is_file());
    fs::write(temp.path().join("Directory.Build.props"), "local edit").unwrap();
    req.operation = Operation::Uninstall;
    let uninstall = plan(req).unwrap();
    assert!(!uninstall.conflicts.is_empty());
    assert!(apply(&uninstall).is_err());
    assert!(temp.path().join("global.json").is_file());
    assert!(temp
        .path()
        .join(".policy/bootstrap-manifest.json")
        .is_file());
}
