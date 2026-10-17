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

const BUNDLE_PARENT: &str = "node_modules/@tailwindcss/oxide-wasm32-wasi";
const BUNDLE_NAMES: [&str; 6] = [
    "@napi-rs/wasm-runtime",
    "@emnapi/core",
    "@emnapi/runtime",
    "@tybys/wasm-util",
    "@emnapi/wasi-threads",
    "tslib",
];

fn bundle_fixture(root: &Path) -> project_verification::RunRequest {
    let request = fixture(root);
    edit(root, "package-lock.json", |v| {
        v["packages"][BUNDLE_PARENT] = json!({
            "version":"4.3.3", "optional":true, "cpu":["wasm32"],
            "resolved":"https://registry.npmjs.org/@tailwindcss/oxide-wasm32-wasi/-/oxide-wasm32-wasi-4.3.3.tgz",
            "integrity":format!("sha512-{}==", "A".repeat(86)),
            "bundleDependencies":BUNDLE_NAMES,
            "dependencies": BUNDLE_NAMES.into_iter().map(|n| (n, "1.0.0")).collect::<std::collections::BTreeMap<_, _>>()
        });
    });
    request
}

#[test]
fn bundled_dependencies_record_unaudited_platform_inventory_and_versions() {
    let temp = tempfile::tempdir().unwrap();
    let request = bundle_fixture(temp.path());
    for explicit in [false, true] {
        if explicit {
            edit(temp.path(), "package-lock.json", |v| {
                v["packages"][format!("{BUNDLE_PARENT}/node_modules/@emnapi/core")] =
                    json!({"version":"1.0.0","inBundle":true});
            });
        }
        let prepared = prepare(&request).unwrap();
        let config = &prepared.configurations[0];
        assert_eq!(config.bundled_dependencies.len(), 6);
        assert!(prepared.configurations[1].bundled_dependencies.is_empty());
        assert!(!temp.path().join(BUNDLE_PARENT).exists());
        for item in &config.bundled_dependencies {
            assert_eq!(item.coverage, "bundled, not individually audited");
            assert_eq!(item.parent_name, "@tailwindcss/oxide-wasm32-wasi");
            assert_eq!(item.parent_version, "4.3.3");
            assert_eq!(
                item.parent_integrity,
                format!("sha512-{}==", "A".repeat(86))
            );
            assert_eq!(
                item.version.as_deref(),
                (explicit && item.name == "@emnapi/core").then_some("1.0.0")
            );
        }
        let serialized = serde_json::to_value(config).unwrap();
        assert_eq!(
            serialized["bundled_dependencies"][0]["coverage"],
            "bundled, not individually audited"
        );
        let mut altered = config.clone();
        altered.bundled_dependencies.clear();
        assert_ne!(
            project_verification::run::configuration_digest(config).unwrap(),
            project_verification::run::configuration_digest(&altered).unwrap()
        );
    }
}

#[test]
fn bundles_reject_unpinned_parents_conflicts_and_undeclared_names() {
    for case in 0..24 {
        let temp = tempfile::tempdir().unwrap();
        let request = bundle_fixture(temp.path());
        edit(temp.path(), "package-lock.json", |v| {
            let parent = &mut v["packages"][BUNDLE_PARENT];
            match case {
                0 => {
                    parent.as_object_mut().unwrap().remove("integrity");
                }
                1 => parent["integrity"] = json!("sha256-AAAA"),
                2 => parent["integrity"] = json!("sha512-invalid"),
                3 => parent["resolved"] = json!("http://registry.npmjs.org/a/-/a.tgz"),
                4 => parent["resolved"] = json!("https://example.org/a.tgz"),
                5 => parent["resolved"] = json!("git+https://registry.npmjs.org/a.tgz"),
                6 => parent["resolved"] = json!("file:vendor/a.tgz"),
                7 => parent["resolved"] = json!("https://registry.npmjs.org/a"),
                8 => {
                    parent["resolved"] =
                        json!("https://registry.npmjs.org/@tailwindcss/oxide-wasm32-wasi/-/a.tgz?x")
                }
                9 => parent["bundleDependencies"] = json!(["tslib"]),
                10 => parent["bundleDependencies"] = json!(true),
                11 => parent["bundleDependencies"] = json!(["../escape"]),
                12 => parent["bundleDependencies"] = json!(["tslib", "tslib"]),
                13 => {
                    parent.as_object_mut().unwrap().remove("version");
                }
                14 => parent["name"] = json!("different"),
                15 => parent["integrity"] = json!(format!("sha512-{}==", "é".repeat(43))),
                _ => {
                    let mut child = json!({"version":"1.0.0","inBundle":true});
                    match case {
                        16 => child["inBundle"] = json!(false),
                        17 => child["version"] = json!("2.0.0"),
                        18 => {
                            child["resolved"] =
                                json!("https://registry.npmjs.org/@emnapi/core/-/core-1.0.0.tgz")
                        }
                        19 => child["link"] = json!(true),
                        20 => child["name"] = json!("other"),
                        21 => {
                            child.as_object_mut().unwrap().remove("version");
                        }
                        22 => parent["bundleDependencies"] = json!(["tslib"]),
                        23 => {
                            parent.as_object_mut().unwrap().remove("bundleDependencies");
                        }
                        _ => unreachable!(),
                    }
                    v["packages"][format!("{BUNDLE_PARENT}/node_modules/@emnapi/core")] = child;
                }
            }
        });
        assert!(prepare(&request).is_err(), "bundle rejection case {case}");
    }
    for manifest in ["package.json", "tools/openapi/package.json"] {
        for field in ["bundledDependencies", "bundleDependencies"] {
            let temp = tempfile::tempdir().unwrap();
            let request = bundle_fixture(temp.path());
            edit(temp.path(), manifest, |v| v[field] = json!(["tslib"]));
            assert!(prepare(&request).is_err(), "{manifest}:{field}");
        }
    }
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

fn reviewed_overrides(request: &mut project_verification::RunRequest, value: Value) {
    request.profile.selection.as_mut().unwrap().graphs[0]
        .workspace
        .as_mut()
        .unwrap()
        .overrides = serde_json::from_value(value).unwrap();
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
}

fn override_fixture(root: &Path) -> project_verification::RunRequest {
    let mut request = fixture(root);
    let overrides = json!({
        "svelte-toolbelt":"0.10.5",
        "fdir":{"picomatch":"4.0.7"},
        "@tailwindcss/cli":{"@parcel/watcher":"2.6.0"}
    });
    edit(root, "package.json", |v| {
        v["overrides"] = overrides.clone();
        v["dependencies"] =
            json!({"svelte-toolbelt":"0.10.5","fdir":"6.5.0","@tailwindcss/cli":"4.1.0"});
    });
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("package.json")).unwrap()).unwrap();
    edit(root, "package-lock.json", |v| {
        v["packages"][""]["dependencies"] = manifest["dependencies"].clone();
        for (name, version, deps) in [
            ("svelte-toolbelt", "0.10.5", json!({})),
            ("fdir", "6.5.0", json!({"picomatch":"4.0.2"})),
            ("picomatch", "4.0.7", json!({})),
            (
                "@tailwindcss/cli",
                "4.1.0",
                json!({"@parcel/watcher":"2.5.0"}),
            ),
            ("@parcel/watcher", "2.6.0", json!({})),
        ] {
            v["packages"][format!("node_modules/{name}")] = json!({
                "version":version, "dependencies":deps,
                "resolved":format!("https://registry.npmjs.org/{name}/-/fixture.tgz")
            });
        }
    });
    reviewed_overrides(&mut request, overrides);
    request
}

#[test]
fn reviewed_overrides_bind_profile_and_logical_hoisted_lock_edges() {
    let temp = tempfile::tempdir().unwrap();
    let request = override_fixture(temp.path());
    let prepared = prepare(&request).unwrap();
    assert_eq!(
        prepared
            .configurations
            .iter()
            .filter(|c| c.sensor.argv.get(1).is_some_and(|a| a == "audit"))
            .count(),
        1
    );
    let mut changed = request.clone();
    reviewed_overrides(&mut changed, json!({"svelte-toolbelt":"0.10.6"}));
    assert_ne!(
        request.expected_profile_digest,
        changed.expected_profile_digest
    );
    assert!(prepare(&changed).is_err(), "profile/manifest drift");
}

#[test]
fn override_versions_names_depth_and_scope_fail_closed() {
    let base = json!({"svelte-toolbelt":"0.10.5"});
    for invalid in [
        json!({"svelte-toolbelt":"^0.10.5"}),
        json!({"svelte-toolbelt":"~0.10.5"}),
        json!({"svelte-toolbelt":"*"}),
        json!({"svelte-toolbelt":"latest"}),
        json!({"svelte-toolbelt":"$svelte-toolbelt"}),
        json!({"svelte-toolbelt":"npm:other@0.10.5"}),
        json!({"svelte-toolbelt":"git+https://example.invalid/repo"}),
        json!({"svelte-toolbelt":"file:../other"}),
        json!({"svelte-toolbelt":"link:../other"}),
        json!({"svelte-toolbelt":"https://example.invalid/a.tgz"}),
        json!({"svelte-toolbelt":"./a.tgz"}),
        json!({"svelte-toolbelt":"workspace:*"}),
        json!({"svelte-toolbelt":"0.10.05"}),
        json!({"svelte-toolbelt":"0.10.5-beta.1"}),
        json!({"svelte-toolbelt":"0.10.5+foo"}),
        json!({"Svelte-toolbelt":"0.10.5"}),
        json!({"svеlte-toolbelt":"0.10.5"}),
        json!({"@Scope/pkg":"0.10.5"}),
        json!({"@scope/Ｐkg":"0.10.5"}),
        json!({"../svelte-toolbelt":"0.10.5"}),
        json!({".hidden":"0.10.5"}),
        json!({"node_modules":"0.10.5"}),
        json!({"favicon.ico":"0.10.5"}),
        json!({"svelte-toolbelt@1":"0.10.5"}),
        json!({"":"0.10.5"}),
        json!({"@scope":"0.10.5"}),
        json!({"@scope/":"0.10.5"}),
        json!({"svelte-toolbelt":{".":"0.10.5"}}),
        json!({"fdir":{"picomatch":{"other":"1.0.0"}}}),
        json!({"fdir":{}}),
        json!({"fdir":{"fdir":"1.0.0"}}),
        json!({"fdir":{"picomatch":"4.0.7"},"picomatch":"4.0.7"}),
        json!({"svelte-toolbelt":null}),
        json!({"svelte-toolbelt":123}),
        json!({"svelte-toolbelt":[]}),
        json!([]),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let request = fixture(temp.path());
        let mut profile = serde_json::to_value(&request.profile).unwrap();
        profile["selection"]["graphs"][0]["workspace"]["overrides"] = invalid.clone();
        assert!(
            parse_profile(&profile.to_string()).is_err(),
            "accepted {invalid}"
        );
        // An unreviewed root override also cannot bypass the profile.
        edit(temp.path(), "package.json", |v| {
            v["overrides"] = invalid.clone()
        });
        assert!(prepare(&request).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    let request = fixture(temp.path());
    let mut profile = serde_json::to_value(&request.profile).unwrap();
    let many: serde_json::Map<String, Value> = (0..33)
        .map(|i| (format!("package-{i}"), json!("1.0.0")))
        .collect();
    profile["selection"]["graphs"][0]["workspace"]["overrides"] = json!(many);
    assert!(parse_profile(&profile.to_string()).is_err());
    for root_too in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let mut request = fixture(temp.path());
        edit(temp.path(), "tools/openapi/package.json", |v| {
            v["overrides"] = base.clone()
        });
        if root_too {
            edit(temp.path(), "package.json", |v| {
                v["overrides"] = base.clone()
            });
            reviewed_overrides(&mut request, base.clone());
        }
        assert!(
            prepare(&request).is_err(),
            "member overrides: root={root_too}"
        );
    }
}

#[test]
fn overrides_reject_contract_targets_dead_rules_and_lock_drift() {
    for target in [
        "typescript",
        "vite",
        "eslint",
        "vitest",
        "@playwright/test",
        "@typescript-eslint/parser",
        "jsdom",
        "app",
        "generator",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let mut request = fixture(temp.path());
        let value = json!({target:"1.0.0"});
        edit(temp.path(), "package.json", |v| {
            v["overrides"] = value.clone()
        });
        reviewed_overrides(&mut request, value);
        assert!(
            prepare(&request).is_err(),
            "contract/root/member target {target}"
        );
    }
    for case in 0..9 {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let mut request = override_fixture(root);
        match case {
            0 => edit(root, "package-lock.json", |v| {
                v["packages"]["node_modules/picomatch"]["version"] = json!("4.0.2")
            }),
            1 => edit(root, "package-lock.json", |v| {
                v["packages"]
                    .as_object_mut()
                    .unwrap()
                    .remove("node_modules/picomatch");
            }),
            2 => edit(root, "package-lock.json", |v| {
                v["packages"]["node_modules/svelte-toolbelt"]["version"] = json!("0.10.4")
            }),
            3 => {
                edit(root, "package.json", |v| {
                    v["overrides"]["missing"] = json!("1.0.0")
                });
                let mut overrides = serde_json::to_value(
                    &request.profile.selection.as_ref().unwrap().graphs[0]
                        .workspace
                        .as_ref()
                        .unwrap()
                        .overrides,
                )
                .unwrap();
                overrides["missing"] = json!("1.0.0");
                reviewed_overrides(&mut request, overrides);
            }
            4 => edit(root, "package-lock.json", |v| {
                v["packages"]["node_modules/fdir"]["dependencies"] = json!({});
            }),
            5 => edit(root, "package-lock.json", |v| {
                v["packages"]["tools/openapi/node_modules/svelte-toolbelt"] = json!({"version":"0.10.4","resolved":"https://registry.npmjs.org/svelte-toolbelt/-/old.tgz"})
            }),
            6 => edit(root, "package-lock.json", |v| {
                v["packages"]["node_modules/picomatch"]["name"] = json!("malicious")
            }),
            7 => edit(root, "package.json", |v| {
                v["overrides"]["svelte-toolbelt"] = json!("0.10.6")
            }),
            8 => edit(root, "package-lock.json", |v| {
                v["packages"]["tools/openapi"]["dependencies"] = json!({"fdir":"6.5.0"});
                v["packages"]["tools/openapi/node_modules/fdir"] = json!({"version":"6.5.0","resolved":"https://registry.npmjs.org/fdir/-/fdir.tgz","dependencies":{"picomatch":"4.0.2"}});
                v["packages"]["tools/openapi/node_modules/picomatch"] = json!({"version":"4.0.2","resolved":"https://registry.npmjs.org/picomatch/-/picomatch.tgz"});
            }),
            _ => unreachable!(),
        }
        if case == 8 {
            edit(root, "tools/openapi/package.json", |v| {
                v["dependencies"] = json!({"fdir":"6.5.0"})
            });
        }
        assert!(prepare(&request).is_err(), "case {case}");
    }
    for field in ["bundledDependencies", "bundleDependencies"] {
        let temp = tempfile::tempdir().unwrap();
        let request = override_fixture(temp.path());
        edit(temp.path(), "package.json", |v| v[field] = json!([]));
        assert!(prepare(&request).is_err(), "bundles remain forbidden");
    }
}

#[test]
fn overrides_cannot_change_selected_packages_or_direct_root_specs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let mut request = override_fixture(root);
    let package = serde_json::from_value(json!({
        "component":"frontend","installed_manifest":"node_modules/picomatch/package.json",
        "lockfile":"package-lock.json","lock_pointer":"/packages/node_modules~1picomatch/version",
        "acceptance":{"kind":"exact","value":"4.0.7"}
    }))
    .unwrap();
    request
        .profile
        .selection
        .as_mut()
        .unwrap()
        .packages
        .push(package);
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    assert!(
        request.profile.validate().is_err(),
        "selected custom contract target"
    );
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let request = override_fixture(root);
    edit(root, "package.json", |v| {
        v["dependencies"]["svelte-toolbelt"] = json!("^0.10.0")
    });
    edit(root, "package-lock.json", |v| {
        v["packages"][""]["dependencies"]["svelte-toolbelt"] = json!("^0.10.0")
    });
    assert!(
        prepare(&request).is_err(),
        "npm EOVERRIDE direct spec mismatch"
    );
}

#[test]
fn scoped_overrides_follow_descendants_and_check_every_nested_copy() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let request = override_fixture(root);
    edit(root, "package-lock.json", |v| {
        v["packages"]["node_modules/fdir"]["dependencies"] =
            json!({"helper":"1.0.0", "picomatch":"4.0.2"});
        v["packages"]["node_modules/helper"] = json!({"version":"1.0.0", "resolved":"https://registry.npmjs.org/helper/-/helper.tgz", "dependencies":{"picomatch":"4.0.2"}});
        v["packages"]["node_modules/helper/node_modules/picomatch"] = json!({"version":"4.0.7", "resolved":"https://registry.npmjs.org/picomatch/-/picomatch.tgz"});
    });
    prepare(&request).unwrap();
    edit(root, "package-lock.json", |v| {
        v["packages"]["node_modules/helper/node_modules/picomatch"]["version"] = json!("4.0.2")
    });
    assert!(prepare(&request).is_err(), "transitive nested copy drift");
}

#[test]
fn override_resolution_skips_spurious_node_modules_ancestors() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let request = override_fixture(root);
    edit(root, "package-lock.json", |v| {
        let target = v["packages"]
            .as_object_mut()
            .unwrap()
            .remove("node_modules/picomatch")
            .unwrap();
        v["packages"]["node_modules/node_modules/picomatch"] = target;
    });
    assert!(prepare(&request).is_err(), "not a Node resolution location");
}

#[test]
fn scoped_override_cannot_leak_through_a_shared_dependency_declaration() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let request = override_fixture(root);
    edit(root, "package.json", |v| {
        v["dependencies"]["helper"] = json!("1.0.0")
    });
    edit(root, "package-lock.json", |v| {
        v["packages"][""]["dependencies"]["helper"] = json!("1.0.0");
        v["packages"]["node_modules/fdir"]["dependencies"] = json!({"helper":"1.0.0"});
        v["packages"]["node_modules/helper"] = json!({"version":"1.0.0", "resolved":"https://registry.npmjs.org/helper/-/helper.tgz", "dependencies":{"picomatch":"4.0.2"}});
    });
    let error = prepare(&request).unwrap_err().to_string();
    assert!(error.contains("outside its scope"), "{error}");
}
