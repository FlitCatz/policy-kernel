use project_verification::parse_profile;
use serde_json::{json, Value};
mod common;

fn request(root: &std::path::Path) -> project_verification::RunRequest {
    let mut request = common::request(root);
    std::fs::write(root.join("package.json"), "{}").unwrap();
    std::fs::write(
        root.join("package-lock.json"),
        r#"{"lockfileVersion":3,"packages":{"":{}}}"#,
    )
    .unwrap();
    let mut profile = serde_json::to_value(&request.profile).unwrap();
    profile["schema_version"] = json!(2);
    profile["version"] = json!("2.3.4");
    profile["sensors"][0]["version_stdout"] = json!("");
    let mut audit = profile["sensors"][0].clone();
    audit["definition"]["id"] = json!("frontend:audit-renamed");
    audit["definition"]["result_requirement"] = json!("process");
    audit["argv"] = json!([
        "npm",
        "audit",
        "--audit-level=low",
        "--include=dev",
        "--include=optional",
        "--include=peer"
    ]);
    audit["report"] = Value::Null;
    audit["inputs"] = json!(["package.json", "package-lock.json"]);
    profile["sensors"].as_array_mut().unwrap().push(audit);
    profile["selection"] = json!({"schema_version":1,
        "tools":{"tests":{"executable":"this-command-must-not-exist","version_args":["--version"],
            "version":{"requirement":">=4.1.0, <5.0.0","normalization":"plain","provision":"4.1.11"},"capabilities":["vitest_json"]},
            "packages":{"executable":"npm","version_args":["--version"],"version":{"requirement":"=10.9.3","normalization":"plain","provision":"10.9.3"},"capabilities":["process","npm_audit"]}},
        "bindings":{"frontend:unit":"tests","frontend:audit-renamed":"packages"},"packages":[],
        "graphs":[{"id":"app","component":"frontend","role":"application","manifest":"package.json","lockfile":"package-lock.json", "adapter":"npm","tool":"packages","audit_sensor":"frontend:audit-renamed","target_framework":null}],"runtime_files":[]});
    request.profile = parse_profile(&profile.to_string()).unwrap();
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    request
}

#[test]
fn runner_accepts_two_compatible_versions_and_rejects_malformed_or_out_of_range() {
    use policy_contracts::{SensorStatus, TrustClass};
    use project_verification::run::{normalize_collection, prepare};
    let root = tempfile::tempdir().unwrap();
    let request = request(root.path());
    let config = prepare(&request).unwrap().configurations.remove(0);
    let report =
        include_bytes!("../../policy-report-adapters/tests/fixtures/project-reports/vitest.json");
    std::fs::create_dir(root.path().join("reports")).unwrap();
    std::fs::write(root.path().join("reports/unit.json"), report).unwrap();
    for (version, passed) in [
        ("4.1.11\n", true),
        ("4.2.0", true),
        ("5.0.0", false),
        ("x4.1.11x", false),
        ("4.1.11\n5.0.0", false),
    ] {
        let mut collection = common::collection(&request, &config, report);
        collection.record.version_probe.stdout = version.as_bytes().to_vec();
        common::reseal(&mut collection);
        let normalized = normalize_collection(&request, config.clone(), collection).unwrap();
        assert_eq!(
            normalized.evidence.status == SensorStatus::Passed,
            passed,
            "{version}"
        );
        assert_eq!(
            normalized.collection.record.version_probe.stdout,
            version.as_bytes()
        );
        assert_eq!(normalized.evidence.trust, TrustClass::LocalUntrusted);
    }
}

#[test]
fn selected_graph_inputs_are_bound_even_when_test_does_not_list_them() {
    let root = tempfile::tempdir().unwrap();
    let request = request(root.path());
    let prepared = project_verification::run::prepare(&request).unwrap();
    assert!(prepared.configurations[0]
        .inputs
        .iter()
        .any(|f| f.path == "package-lock.json"));
    std::fs::write(
        root.path().join("package.json"),
        r#"{"dependencies":{"unlocked":"1.0.0"}}"#,
    )
    .unwrap();
    assert!(project_verification::run::prepare(&request).is_err());
}

#[test]
fn range_parser_rejects_ambiguous_versions_and_capabilities_cannot_be_omitted() {
    use project_verification::selection::version_matches;
    assert!(version_matches(">=22.18.0, <23.0.0", "22.19.2").unwrap());
    assert!(!version_matches(">=22.18.0, <23.0.0", "23.0.0").unwrap());
    for version in [
        "22.18",
        "022.18.0",
        "22.18.0-preview",
        "922337203685477580899.0.0",
        "22.18.0 extra",
    ] {
        assert!(version_matches("=22.18.0", version).is_err());
    }
    for range in [
        "",
        "22",
        "*",
        "^22.18.0",
        ">=22.18.0,",
        "=22.18.0 || =23.0.0",
    ] {
        assert!(version_matches(range, "22.18.0").is_err());
    }
    let mut value = selected_profile();
    value["selection"]["tools"]["frontend:unit"]["capabilities"] = json!(["process"]);
    assert!(parse_profile(&value.to_string()).is_err());
}

#[test]
fn out_of_range_probe_prevents_the_selected_command_from_executing() {
    let root = tempfile::tempdir().unwrap();
    let fixture = root.path().join("selected-fixture");
    assert!(std::process::Command::new(
        std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into())
    )
    .args([
        "--edition=2021",
        "../../tests/support/selected_runtime_fixture.rs",
        "-o"
    ])
    .arg(&fixture)
    .status()
    .unwrap()
    .success());
    let mut request = request(root.path());
    request.profile.sensors[0].argv = vec![fixture.to_str().unwrap().into(), "touch".into()];
    request
        .profile
        .selection
        .as_mut()
        .unwrap()
        .tools
        .get_mut("tests")
        .unwrap()
        .executable = fixture.to_str().unwrap().into();
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    assert!(project_verification::run(request).is_err());
    assert!(!root.path().join("executed-marker").exists());
}

fn selected_profile() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../bootstrap/templates/1.0.2/profiles/react-typescript/profile.json"
    ))
    .unwrap();
    value["schema_version"] = json!(2);
    value["version"] = json!("1.1.0");
    let mut tools = serde_json::Map::new();
    let mut bindings = serde_json::Map::new();
    for sensor in value["sensors"].as_array_mut().unwrap() {
        if sensor["argv"][0] == "npm" && sensor["argv"][1] == "audit" {
            sensor["argv"] = json!([
                "npm",
                "audit",
                "--audit-level=low",
                "--include=dev",
                "--include=optional",
                "--include=peer"
            ]);
        }
        let id = sensor["definition"]["id"].as_str().unwrap().to_owned();
        let output = sensor["version_stdout"].as_str().unwrap();
        let (normalization, version) = match output.strip_prefix('v') {
            Some(v) => ("node", v),
            None => ("plain", output),
        };
        let mut capabilities = vec!["process"];
        if sensor["report"]["format"] == "vitest_json" {
            capabilities.push("vitest_json");
        }
        if id.ends_with(":dependencies") {
            capabilities.push("npm_audit");
        }
        tools.insert(
            id.clone(),
            json!({"executable":sensor["argv"][0],
            "version_args":sensor["version_args"],"version":{"requirement":format!("={version}"),
            "normalization":normalization,"provision":version},"capabilities":capabilities}),
        );
        bindings.insert(id.clone(), json!(id));
        sensor["version_stdout"] = json!("");
    }
    value["selection"] = json!({"schema_version":1,"tools":tools,"bindings":bindings,
        "packages":[],"graphs":[{"id":"frontend-app","component":"frontend",
            "role":"application","manifest":"package.json","lockfile":"package-lock.json",
            "adapter":"npm","tool":"frontend:dependencies","audit_sensor":"frontend:dependencies",
            "target_framework":null}],"runtime_files":[{"component":"frontend","path":".node-version",
                "tool":"frontend:node","format":"node_version"}]});
    value
}

#[test]
fn schema_two_accepts_new_asset_release_and_preserves_selection_digest() {
    let value = selected_profile();
    let profile = parse_profile(&value.to_string()).unwrap();
    assert_eq!(
        serde_json::to_value(&profile).unwrap()["selection"],
        value["selection"]
    );
    let before = policy_core::canonical_sha256(&profile).unwrap();
    let mut changed = value;
    changed["selection"]["tools"]["frontend:node"]["version"]["requirement"] =
        json!(">=22.18.0, <23.0.0");
    assert_ne!(
        before,
        policy_core::canonical_sha256(&parse_profile(&changed.to_string()).unwrap()).unwrap()
    );
}

#[test]
fn legacy_profile_cannot_silently_adopt_new_selection_semantics() {
    let mut value = selected_profile();
    value["schema_version"] = json!(1);
    value["version"] = json!("1.0.2");
    assert!(parse_profile(&value.to_string()).is_err());
}

fn dotnet_request(root: &std::path::Path) -> project_verification::RunRequest {
    let mut request = common::request(root);
    request.profile = parse_profile(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/dotnet/profile.json"
    ))
    .unwrap();
    let selection = request.profile.selection.as_ref().unwrap();
    let mut inputs = selection.inputs("backend");
    inputs.extend(
        request
            .profile
            .sensors
            .iter()
            .flat_map(|s| s.inputs.iter().cloned()),
    );
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for input in inputs {
        let example = repo.join("examples/dotnet").join(&input);
        let source = if example.is_file() {
            example
        } else {
            repo.join("bootstrap/templates/1.1.0").join(&input)
        };
        let target = root.join(&input);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(source, target).unwrap();
    }
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    request
}

#[test]
fn every_graph_audit_and_test_remains_required_after_sensor_renaming() {
    let original: Value = serde_json::from_str(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/fullstack/profile.json"
    ))
    .unwrap();
    for graph in original["selection"]["graphs"].as_array().unwrap() {
        let audit = graph["audit_sensor"].as_str().unwrap();
        for omission in ["graph", "sensor", "optional", "argv", "input", "tool"] {
            let mut changed = original.clone();
            let sensor = changed["sensors"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|s| s["definition"]["id"] == audit)
                .unwrap();
            match omission {
                "optional" => sensor["definition"]["required"] = json!(false),
                "argv" => sensor["argv"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("--ignore-failure")),
                "input" => sensor["inputs"] = json!([]),
                "sensor" => changed["sensors"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|s| s["definition"]["id"] != audit),
                "graph" => changed["selection"]["graphs"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|g| g["audit_sensor"] != audit),
                "tool" => {
                    changed["selection"]["tools"][graph["tool"].as_str().unwrap()]["capabilities"] =
                        json!(["process"])
                }
                _ => unreachable!(),
            }
            assert!(
                parse_profile(&changed.to_string()).is_err(),
                "{audit}: {omission}"
            );
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut request = dotnet_request(root.path());
    let text = serde_json::to_string(&request.profile)
        .unwrap()
        .replace("backend:dependencies", "backend:renamed-audit");
    request.profile = parse_profile(&text).unwrap();
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    assert!(project_verification::run::prepare(&request).is_ok());
    let original =
        std::fs::read_to_string(root.path().join("Tests/Unit/Unit.Tests.csproj")).unwrap();
    for suppressed in [original.replace("</Project>", "<ItemGroup><NuGetAuditSuppress Include=\"https://example.invalid/advisory\" /></ItemGroup></Project>"),
        original.replace("<Project ", "<Project TreatAsLocalProperty=\"NuGetAudit;WarningsAsErrors\" "),
        original.replace("<Project ", "<Project TreatAsLocalProperty=\"RestoreLockedMode\" "),
        original.replace("</Project>", "<PropertyGroup><NuGetLockFilePath>other.lock.json</NuGetLockFilePath></PropertyGroup></Project>")] {
        std::fs::write(root.path().join("Tests/Unit/Unit.Tests.csproj"), suppressed).unwrap();
        assert!(project_verification::run::prepare(&request).is_err(), "renaming cannot bypass audit XML checks");
    }
}

#[test]
fn selected_paths_frameworks_and_runtime_files_are_checked_without_fixed_app_names() {
    let root = tempfile::tempdir().unwrap();
    let mut request = dotnet_request(root.path());
    let text = serde_json::to_string(&request.profile)
        .unwrap()
        .replace("App.csproj", "Service.csproj")
        .replace("Tests/", "Quality/");
    request.profile = parse_profile(&text).unwrap();
    std::fs::rename(
        root.path().join("App.csproj"),
        root.path().join("Service.csproj"),
    )
    .unwrap();
    std::fs::rename(root.path().join("Tests"), root.path().join("Quality")).unwrap();
    for file in [
        "Quality/Unit/Unit.Tests.csproj",
        "Quality/Integration/Integration.Tests.csproj",
    ] {
        let path = root.path().join(file);
        let content = std::fs::read_to_string(&path)
            .unwrap()
            .replace("App.csproj", "Service.csproj");
        std::fs::write(path, content).unwrap();
    }
    request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
    assert!(project_verification::run::prepare(&request).is_ok());
    let original = std::fs::read_to_string(root.path().join("Service.csproj")).unwrap();
    std::fs::write(
        root.path().join("Service.csproj"),
        original.replace("net9.0", "net8.0"),
    )
    .unwrap();
    assert!(project_verification::run::prepare(&request).is_err());
    std::fs::write(root.path().join("Service.csproj"), original).unwrap();
    std::fs::write(
        root.path().join("global.json"),
        r#"{"sdk":{"version":"9.0.100","rollForward":"disable"}}"#,
    )
    .unwrap();
    assert!(project_verification::run::prepare(&request).is_err());
}

#[test]
fn ci_inputs_preserve_exact_provisions_and_reject_conflicting_job_selections() {
    let profile = parse_profile(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    let inputs = project_verification::selection::ci_inputs(&profile).unwrap();
    assert_eq!(inputs["node_version"], "22.19.0");
    assert_eq!(inputs["npm_version"], "10.9.4");
    assert_eq!(
        inputs["project_profile_digest"],
        policy_core::canonical_sha256(&profile).unwrap().to_string()
    );
    let mut fullstack = parse_profile(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/fullstack/profile.json"
    ))
    .unwrap();
    fullstack
        .selection
        .as_mut()
        .unwrap()
        .tools
        .get_mut("shared:dependencies")
        .unwrap()
        .version
        .legacy_mut()
        .unwrap()
        .provision = "10.9.4".into();
    assert!(project_verification::selection::ci_inputs(&fullstack).is_err());
}

#[test]
fn nested_npm_graph_commands_target_the_declared_manifest_directory() {
    let mut profile = parse_profile(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap();
    let selection = profile.selection.as_mut().unwrap();
    let graph = &mut selection.graphs[0];
    graph.manifest = "packages/extra/package.json".into();
    graph.lockfile = "packages/extra/package-lock.json".into();
    let tool = &selection.tools[&graph.tool];
    assert_eq!(
        graph.audit_argv(tool),
        [
            "npm",
            "--prefix",
            "packages/extra",
            "audit",
            "--audit-level=low",
            "--include=dev",
            "--include=optional",
            "--include=peer"
        ]
    );
    assert_eq!(
        graph.install_argv(tool),
        [
            "npm",
            "--prefix",
            "packages/extra",
            "ci",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--include=dev",
            "--include=optional",
            "--include=peer"
        ]
    );
    let mut invalid = selected_profile();
    for field in ["manifest", "lockfile"] {
        invalid["selection"]["graphs"][0][field] = json!("unused.json");
        assert!(parse_profile(&invalid.to_string()).is_err());
    }
    let mut dotnet: Value = serde_json::from_str(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/dotnet/profile.json"
    ))
    .unwrap();
    dotnet["selection"]["graphs"][1]["lockfile"] = json!("packages.lock.json");
    assert!(parse_profile(&dotnet.to_string()).is_err());
}

#[test]
fn available_inputs_are_validated_even_when_their_pair_is_not_installed() {
    use project_verification::selection::validate_available_component_files;
    use std::collections::BTreeMap;
    let profile = parse_profile(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/dotnet/profile.json"
    ))
    .unwrap();
    let selection = profile.selection.as_ref().unwrap();
    let xml = |target: &str| {
        format!("<Project><PropertyGroup><TargetFramework>{target}</TargetFramework></PropertyGroup></Project>").into_bytes()
    };
    let mut files = BTreeMap::from([("App.csproj".into(), xml("net8.0"))]);
    assert!(validate_available_component_files(selection, "backend", &files).is_err());
    files.insert("App.csproj".into(), xml("net9.0"));
    validate_available_component_files(selection, "backend", &files).unwrap();
    files.clear();
    files.insert(
        "packages.lock.json".into(),
        br#"{"dependencies":{"net8.0":{}}}"#.to_vec(),
    );
    assert!(validate_available_component_files(selection, "backend", &files).is_err());
    let profile = parse_profile(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/react-typescript/profile.json"
    ))
    .unwrap();
    let selection = profile.selection.as_ref().unwrap();
    let package = &selection.packages[0];
    files = BTreeMap::from([(
        package.installed_manifest.clone(),
        br#"{"version":"1.0.0"}"#.to_vec(),
    )]);
    assert!(validate_available_component_files(selection, "frontend", &files).is_err());
    let key = package
        .installed_manifest
        .strip_suffix("/package.json")
        .unwrap();
    let locked = json!({"lockfileVersion":3,"packages":{"":{},key:{"version":"1.0.0"}}});
    files = BTreeMap::from([(
        package.lockfile.clone(),
        serde_json::to_vec(&locked).unwrap(),
    )]);
    assert!(validate_available_component_files(selection, "frontend", &files).is_err());
}

#[test]
fn schema_and_reader_agree_on_profile_contract_versions_and_null_selection() {
    let schema = jsonschema::validator_for(
        &serde_json::to_value(schemars::schema_for!(project_verification::ProjectProfile)).unwrap(),
    )
    .unwrap();
    let base = selected_profile();
    for (version, release, valid) in [
        (2, "73.2.1", true),
        (2, "1.1", false),
        (2, "01.1.0", false),
        (3, "1.1.0", false),
        (1, "1.1.0", false),
    ] {
        let mut value = base.clone();
        value["schema_version"] = json!(version);
        value["version"] = json!(release);
        assert_eq!(schema.is_valid(&value), valid);
        assert_eq!(
            serde_json::from_value::<project_verification::ProjectProfile>(value).is_ok(),
            valid
        );
    }
    for missing in [true, false] {
        let mut value = base.clone();
        if missing {
            value.as_object_mut().unwrap().remove("selection");
        } else {
            value["selection"] = Value::Null;
        }
        assert!(!schema.is_valid(&value));
        assert!(serde_json::from_value::<project_verification::ProjectProfile>(value).is_err());
    }
}
