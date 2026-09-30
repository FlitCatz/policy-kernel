use policy_bootstrap::{apply, plan, Installation, Operation, Request, UserProfile};
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs};

#[test]
fn schema_three_bootstrap_is_explicit_and_preserves_native_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let mut value: Value = serde_json::from_str(include_str!(
        "../templates/1.1.0/profiles/dotnet/profile.json"
    ))
    .unwrap();
    value["schema_version"] = json!(3);
    value["selection"]["schema_version"] = json!(2);
    value["selection"]["consumer"] = json!({"development_only":false,"suites":[
        {"role":"unit","sensor":"backend:unit","graph":"backend-unit-dependencies"},
        {"role":"integration","sensor":"backend:integration","graph":"backend-integration-dependencies"}
    ],"framework_props":{},"runtime_policies":{}});
    fs::write(
        root.join("profile.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    fs::write(root.join("native.txt"), "user-owned").unwrap();
    let mut request = Request {
        target: root.clone(),
        operation: Operation::Install,
        installation: Installation {
            profile: String::new(),
            release: "1.2.0".into(),
            components: BTreeMap::new(),
            user_profile: Some(UserProfile {
                path: "profile.json".into(),
                digest: String::new(),
            }),
        },
    };
    let installation = plan(request.clone()).expect("new explicit custom release");
    assert!(
        installation.conflicts.is_empty(),
        "{:?}",
        installation.conflicts
    );
    assert!(!installation.prerequisites.is_empty());
    apply(&installation).unwrap();
    assert!(!root.join(".policy/project-profile.json").exists());
    let receipt: Value =
        serde_json::from_slice(&fs::read(root.join(".policy/bootstrap-manifest.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["project_profile_schema"], 3);
    assert_eq!(receipt["schema_version"], 3);
    request.installation.release = "1.1.0".into();
    assert!(plan(request.clone()).is_err());
    request.installation.release = "1.2.0".into();
    request.operation = Operation::Uninstall;
    // Removing invalid user input must not make receipt-based uninstall destructive.
    fs::write(root.join("profile.json"), "edited by user").unwrap();
    apply(&plan(request).unwrap()).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("profile.json")).unwrap(),
        "edited by user"
    );
    assert_eq!(
        fs::read_to_string(root.join("native.txt")).unwrap(),
        "user-owned"
    );
}

#[test]
fn schema_four_custom_bootstrap_preserves_user_owned_profile_and_receipt() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let legacy = project_verification::parse_profile(include_str!(
        "../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    let project = project_verification::selection::upgrade_frontend_profile(legacy).unwrap();
    let mut value = serde_json::to_value(project).unwrap();
    value["selection"]
        .as_object_mut()
        .unwrap()
        .remove("provisioning");
    fs::write(
        root.join("profile.json"),
        serde_json::to_vec(&value).unwrap(),
    )
    .unwrap();
    fs::write(root.join("native.txt"), "user-owned").unwrap();
    let mut request = Request {
        target: root.clone(),
        operation: Operation::Install,
        installation: Installation {
            profile: String::new(),
            release: "1.3.0".into(),
            components: BTreeMap::new(),
            user_profile: Some(UserProfile {
                path: "profile.json".into(),
                digest: String::new(),
            }),
        },
    };
    let installation = plan(request.clone()).unwrap();
    assert!(
        installation.conflicts.is_empty(),
        "{:?}",
        installation.conflicts
    );
    apply(&installation).unwrap();
    let receipt: Value =
        serde_json::from_slice(&fs::read(root.join(".policy/bootstrap-manifest.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["schema_version"], 4);
    assert_eq!(receipt["project_profile_schema"], 4);
    assert!(!root.join(".policy/project-profile.json").exists());
    request.installation.release = "1.2.0".into();
    assert!(plan(request.clone()).is_err());
    request.installation.release = "1.3.0".into();
    request.operation = Operation::Uninstall;
    fs::write(root.join("profile.json"), "edited by user").unwrap();
    apply(&plan(request).unwrap()).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("profile.json")).unwrap(),
        "edited by user"
    );
    assert_eq!(
        fs::read_to_string(root.join("native.txt")).unwrap(),
        "user-owned"
    );
}
