use project_verification::{parse_profile, run::prepare};
use serde_json::{json, Value};
use std::{fs, path::Path};
mod common;
#[path = "common/npm_workspace.rs"]
mod workspace;
use workspace::fixture;
fn write(root: &Path, path: &str, value: Value) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
}

#[test]
fn root_workspace_binds_members_and_uses_one_audit() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let prepared = prepare(&request).unwrap();
    assert_eq!(
        prepared
            .configurations
            .iter()
            .filter(|c| c.sensor.argv.get(1).is_some_and(|arg| arg == "audit"))
            .count(),
        1
    );
    assert!(prepared.configurations[0]
        .inputs
        .iter()
        .any(|f| f.path == "tools/openapi/package.json"));
}

fn edit(root: &Path, path: &str, f: impl FnOnce(&mut Value)) {
    let mut value = serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap();
    f(&mut value);
    write(root, path, value);
}

#[test]
fn workspace_rejects_manifest_inventory_and_lock_drift() {
    for case in 0..12 {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let request = fixture(root);
        match case {
            0 => edit(root, "package.json", |v| v["workspaces"] = json!([])),
            1 => edit(root, "package.json", |v| {
                v["workspaces"] = json!(["tools/*"])
            }),
            2 => write(root, "hidden/package.json", json!({})),
            3 => {
                fs::remove_file(root.join("tools/openapi/package.json")).unwrap();
            }
            4 => edit(root, "tools/openapi/package.json", |v| {
                v["devDependencies"]["typescript"] = json!("6.0.3")
            }),
            5 => edit(root, "package-lock.json", |v| {
                v["packages"]
                    .as_object_mut()
                    .unwrap()
                    .remove("tools/openapi");
            }),
            6 => edit(root, "package-lock.json", |v| {
                v["packages"]["extra"] = json!({})
            }),
            7 => edit(root, "package-lock.json", |v| {
                v["packages"]["node_modules/generator"]["resolved"] = json!("../external")
            }),
            8 => edit(root, "package-lock.json", |v| {
                v["packages"]
                    .as_object_mut()
                    .unwrap()
                    .remove("node_modules/generator");
            }),
            9 => edit(
                root,
                "tools/openapi/node_modules/typescript/package.json",
                |v| v["version"] = json!("6.0.3"),
            ),
            10 => {
                fs::write(root.join(".npmrc"), "workspaces=false\nomit=dev\n").unwrap();
            }
            11 => write(root, "tools/openapi/package-lock.json", json!({})),
            _ => unreachable!(),
        }
        assert!(prepare(&request).is_err(), "case {case}");
    }
}

#[test]
fn workspace_rejects_unsafe_paths_and_audit_scope_reduction() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let original = serde_json::to_value(&request.profile).unwrap();
    for path in [
        "../other/package.json",
        "/tmp/member/package.json",
        "tools\\openapi\\package.json",
        "tools/*/package.json",
        "tools/./openapi/package.json",
        "tools//openapi/package.json",
        "node_modules/member/package.json",
    ] {
        let mut profile = original.clone();
        profile["selection"]["graphs"][0]["workspace"]["member_manifests"] = json!([path]);
        assert!(parse_profile(&profile.to_string()).is_err(), "{path}");
    }
    for arg in [
        "--omit=dev",
        "--workspaces=false",
        "--workspace=generator",
        "--include-workspace-root=false",
        "--production",
    ] {
        let mut profile = original.clone();
        profile["sensors"][0]["argv"]
            .as_array_mut()
            .unwrap()
            .push(json!(arg));
        assert!(parse_profile(&profile.to_string()).is_err(), "{arg}");
    }
    for key in [
        "npm_config_omit",
        "NPM_CONFIG_WORKSPACES",
        "NPM_CONFIG_WORKSPACE",
        "NODE_OPTIONS",
    ] {
        let mut profile = original.clone();
        profile["sensors"][0]["environment_allowlist"] = json!([key]);
        assert!(parse_profile(&profile.to_string()).is_err(), "{key}");
    }
    let mut profile = original.clone();
    profile["selection"]["packages"][1]["installed_manifest"] =
        json!("node_modules/typescript/package.json");
    assert!(parse_profile(&profile.to_string()).is_err());
}

#[test]
fn workspace_rejects_external_dependencies_even_with_matching_lock_declarations() {
    for dep in [
        "file:../../outside",
        "../outside",
        "/tmp/outside",
        "https://example.invalid/package.tgz",
        "git+https://example.invalid/repo",
        "npm:typescript@6.0.3",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let request = fixture(temp.path());
        edit(temp.path(), "tools/openapi/package.json", |v| {
            v["dependencies"]["escape"] = json!(dep)
        });
        edit(temp.path(), "package-lock.json", |v| {
            v["packages"]["tools/openapi"]["dependencies"]["escape"] = json!(dep)
        });
        assert!(prepare(&request).is_err(), "{dep}");
    }
}

#[test]
fn workspace_rejects_member_dependency_hidden_from_audit_inventory() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    edit(temp.path(), "tools/openapi/package.json", |v| {
        v["dependencies"] = json!({"lodash":"4.17.20"})
    });
    edit(temp.path(), "package-lock.json", |v| {
        v["packages"]["tools/openapi"]["dependencies"] = json!({"lodash":"4.17.20"})
    });
    assert!(
        prepare(&request).is_err(),
        "member-only dependency omitted from audit package inventory"
    );
}

#[test]
fn workspace_rejects_reserved_configuration_and_member_aliases() {
    for name in [".policy-npm-userconfig", ".policy-npm-globalconfig"] {
        let temp = tempfile::tempdir().unwrap();
        let request = fixture(temp.path());
        fs::write(temp.path().join(name), "omit=dev\n").unwrap();
        assert!(prepare(&request).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let original = serde_json::to_value(request.profile).unwrap();
    for change in 0..5 {
        let mut profile = original.clone();
        match change {
            0 => {
                profile["selection"]["graphs"][0]["workspace"]["member_manifests"] =
                    json!(["tools/openapi/package.json", "tools/openapi/package.json"])
            }
            1 => profile["selection"]["graphs"][0]["workspace"]["minimum_pk_revision"] = json!(""),
            2 => profile["selection"]["graphs"][0]["workspace"]["member_manifests"] = json!([]),
            3 => {
                profile["sensors"][0]["inputs"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            4 => {
                let graph = profile["selection"]["graphs"][0].clone();
                profile["selection"]["graphs"]
                    .as_array_mut()
                    .unwrap()
                    .push(graph);
            }
            _ => unreachable!(),
        }
        assert!(
            parse_profile(&profile.to_string()).is_err(),
            "case {change}"
        );
    }
}

#[test]
fn member_installed_manifest_can_be_an_explicit_sensor_input() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = fixture(temp.path());
    request.profile.sensors[1]
        .inputs
        .push("tools/openapi/node_modules/typescript/package.json".into());
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    prepare(&request).unwrap();
}

#[test]
fn frontend_component_consumes_its_own_root_lock() {
    let temp = tempfile::tempdir().unwrap();
    let frontend = temp.path().join("frontend");
    fs::create_dir(&frontend).unwrap();
    let mut request = fixture(&frontend);
    request.root = temp.path().canonicalize().unwrap();
    request.profile.components[0].root = "frontend".into();
    request.source_files[0].path = "frontend/config.json".into();
    request.expected_source_digest = policy_core::canonical_sha256(&request.source_files).unwrap();
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    write(
        temp.path(),
        "package-lock.json",
        json!({"unrelated":"must not be consumed"}),
    );
    let prepared = prepare(&request).unwrap();
    assert!(prepared
        .configurations
        .iter()
        .all(|c| c.component_root == frontend.canonicalize().unwrap()));
    fs::remove_file(frontend.join("package-lock.json")).unwrap();
    assert!(prepare(&request).is_err());
}

#[test]
fn declared_workspaces_cannot_be_hidden_by_removing_the_graph_extension() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = fixture(temp.path());
    let selection = request.profile.selection.as_mut().unwrap();
    let graph = &mut selection.graphs[0];
    graph.workspace = None;
    request.profile.sensors[0].argv = graph.audit_argv(&selection.tools["npm"]);
    request.profile.sensors[0]
        .inputs
        .retain(|p| p != "tools/openapi/package.json");
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    assert!(prepare(&request).is_err());
}

#[test]
fn workspace_inputs_bind_all_sensors_and_detect_post_collection_drift() {
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let prepared = prepare(&request).unwrap();
    assert!(prepared.configurations.iter().all(|c| c
        .inputs
        .iter()
        .any(|i| i.path == "tools/openapi/package.json")));
    let config = prepared
        .configurations
        .into_iter()
        .find(|c| c.sensor.definition.id.as_str() == "frontend:unit")
        .unwrap();
    let report = include_bytes!("../../../tests/fixtures/project-reports/vitest.json");
    fs::create_dir(temp.path().join("reports")).unwrap();
    fs::write(temp.path().join("reports/unit.json"), report).unwrap();
    let collection = common::collection(&request, &config, report);
    write(temp.path(), "extra/package.json", json!({}));
    assert!(project_verification::run::normalize_collection(&request, config, collection).is_err());
}

#[cfg(unix)]
#[test]
fn workspace_rejects_symlink_members_and_wrong_installed_links() {
    for member in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let request = fixture(temp.path());
        if member {
            fs::rename(
                temp.path().join("tools/openapi"),
                temp.path().join("elsewhere"),
            )
            .unwrap();
            std::os::unix::fs::symlink("../elsewhere", temp.path().join("tools/openapi")).unwrap();
        } else {
            fs::remove_file(temp.path().join("node_modules/generator")).unwrap();
            std::os::unix::fs::symlink("../", temp.path().join("node_modules/generator")).unwrap();
        }
        assert!(prepare(&request).is_err());
    }
}
