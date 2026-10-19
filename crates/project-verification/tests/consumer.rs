use project_verification::{parse_profile, selection::ToolSelection};
use serde_json::{json, Value};
const SDK: &str = "11.0.100-rc.1.26425.128";

fn profile() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/dotnet/profile.json"
    ))
    .unwrap();
    value["schema_version"] = json!(3);
    value["adequacy"]["acceptance_criterion_ids"] = json!(["backend:unit-contract"]);
    value["selection"]["schema_version"] = json!(2);
    value["selection"]["consumer"] = json!({"development_only":true,
        "suites":[{"role":"unit","sensor":"backend:unit","graph":"backend-unit-dependencies"},
            {"role":"integration","sensor":"backend:integration","graph":"backend-integration-dependencies"}],
        "framework_props":{},"runtime_policies":{"backend:global.json":{"roll_forward":"latestPatch","allow_prerelease":true}}});
    for tool in value["selection"]["tools"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        tool["producer_contract"] = json!("dotnet11_rc1");
        tool["version"]["requirement"] = json!(format!("={SDK}"));
        tool["version"]["provision"] = json!(SDK);
    }
    value
}

#[test]
fn exact_development_sdk_is_accepted_without_widening_old_profiles() {
    let value = profile();
    let parsed = parse_profile(&value.to_string()).expect("explicit schema 3 RC consumer");
    let selection = parsed.selection.as_ref().unwrap();
    let tool = &selection.tools["backend:unit"];
    assert_eq!(tool.observe(format!("{SDK}\n").as_bytes()).unwrap(), SDK);
    for version in [
        "11.0.100",
        "11.0.100-rc.2.26425.128",
        "11.0.100-rc.1.26425.129",
        "11.0.100-rc.1.26425.128\nextra",
    ] {
        assert!(tool.observe(version.as_bytes()).is_err(), "{version}");
    }
    for (p, s) in [(2, 2), (3, 1), (2, 1)] {
        let mut invalid = value.clone();
        invalid["schema_version"] = json!(p);
        invalid["selection"]["schema_version"] = json!(s);
        assert!(parse_profile(&invalid.to_string()).is_err());
    }
    let bytes = serde_json::to_vec(
        &json!({"sdk":{"version":SDK,"rollForward":"latestPatch","allowPrerelease":true}}),
    )
    .unwrap();
    selection
        .validate_runtime_file(&selection.runtime_files[0], &bytes)
        .unwrap();
    let mut invalid = value;
    invalid["selection"]["consumer"]["development_only"] = json!(false);
    assert!(parse_profile(&invalid.to_string()).is_err());
}

#[test]
fn playwright_contract_accepts_only_the_selected_producer() {
    let tool: ToolSelection =
        serde_json::from_value(json!({"executable":"node","version_args":["--version"],
        "version":{"requirement":"=1.63.0","normalization":"plain","provision":"1.63.0"},
        "capabilities":["playwright_json"],"producer_contract":"playwright163"}))
        .unwrap();
    assert_eq!(tool.observe(b"1.63.0\n").unwrap(), "1.63.0");
    for v in ["1.62.0", "1.63.1", "1.64.0", "1.63.0-rc.1"] {
        assert!(tool.observe(v.as_bytes()).is_err());
    }
}

#[test]
fn legacy_profile_serialization_remains_unchanged() {
    let value: Value = serde_json::from_str(include_str!(
        "../../../bootstrap/templates/1.1.0/profiles/dotnet/profile.json"
    ))
    .unwrap();
    assert_eq!(
        serde_json::to_value(parse_profile(&value.to_string()).unwrap()).unwrap(),
        value
    );
}

fn combined() -> Value {
    let mut value = profile();
    let integration = value["selection"]["graphs"][2].clone();
    let unit = value["selection"]["graphs"][1].clone();
    value["selection"]["graphs"].as_array_mut().unwrap().pop();
    value["selection"]["tools"]
        .as_object_mut()
        .unwrap()
        .remove("backend:integration-dependencies");
    value["selection"]["bindings"]
        .as_object_mut()
        .unwrap()
        .remove("backend:integration-dependencies");
    value["sensors"]
        .as_array_mut()
        .unwrap()
        .retain(|s| s["definition"]["id"] != "backend:integration-dependencies");
    for sensor in value["sensors"].as_array_mut().unwrap() {
        for pin in sensor["pins"].as_array_mut().unwrap() {
            if pin["path"] == integration["manifest"] {
                pin["path"] = unit["manifest"].clone();
            }
            if pin["path"] == integration["lockfile"] {
                pin["path"] = unit["lockfile"].clone();
            }
        }
        for field in ["argv", "inputs"] {
            for part in sensor[field].as_array_mut().unwrap() {
                if *part == integration["manifest"] {
                    *part = unit["manifest"].clone();
                }
                if *part == integration["lockfile"] {
                    *part = unit["lockfile"].clone();
                }
            }
        }
    }
    value["selection"]["consumer"]["suites"][1]["graph"] = unit["id"].clone();
    value
}

#[test]
fn combined_test_graph_preserves_independent_suite_obligations() {
    let value = combined();
    parse_profile(&value.to_string()).expect("combined test project with explicit suites");
    for mutation in ["missing", "same-sensor", "missing-graph", "optional"] {
        let mut invalid = value.clone();
        match mutation {
            "missing" => {
                invalid["selection"]["consumer"]["suites"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }
            "same-sensor" => {
                invalid["selection"]["consumer"]["suites"][1]["sensor"] = json!("backend:unit")
            }
            "missing-graph" => {
                invalid["selection"]["consumer"]["suites"][1]["graph"] = json!("absent")
            }
            _ => {
                for s in invalid["sensors"].as_array_mut().unwrap() {
                    if s["definition"]["id"] == "backend:integration" {
                        s["definition"]["required"] = json!(false);
                    }
                }
            }
        }
        assert!(parse_profile(&invalid.to_string()).is_err(), "{mutation}");
    }
}

#[test]
fn declared_framework_props_are_bound_and_conflicts_fail() {
    use project_verification::selection::validate_available_component_files;
    use std::collections::BTreeMap;
    let mut value = combined();
    for graph in value["selection"]["graphs"].as_array_mut().unwrap() {
        graph["target_framework"] = json!("net11.0");
    }
    value["selection"]["consumer"]["framework_props"] = json!({"backend-dependencies":"Directory.Build.props","backend-unit-dependencies":"Directory.Build.props"});
    let selection = parse_profile(&value.to_string())
        .unwrap()
        .selection
        .unwrap();
    assert!(selection
        .inputs("backend")
        .contains("Directory.Build.props"));
    let mut files=BTreeMap::from([
        ("Directory.Build.props".into(),b"<Project><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>".to_vec()),
        ("App.csproj".into(),b"<Project Sdk=\"Microsoft.NET.Sdk\"/>".to_vec())]);
    validate_available_component_files(&selection, "backend", &files).unwrap();
    for xml in ["<Project><PropertyGroup><TargetFramework>net9.0</TargetFramework></PropertyGroup></Project>","<Project><Import Project=\"other.props\"/></Project>","<Project><PropertyGroup Condition=\"true\"><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>"] {
        files.insert("App.csproj".into(),xml.as_bytes().to_vec());assert!(validate_available_component_files(&selection,"backend",&files).is_err());
    }
}

#[test]
fn suite_reports_reject_missing_or_reused_executions() {
    use project_verification::reports::{parse_report, ReportFormat};
    use std::collections::BTreeMap;
    let parsed = parse_profile(&combined().to_string()).unwrap();
    let consumer = parsed.selection.unwrap().consumer.unwrap();
    let bytes =
        include_bytes!("../../policy-report-adapters/tests/fixtures/project-reports/vstest.trx");
    let unit = parse_report(ReportFormat::Trx, "backend:unit", bytes).unwrap();
    let integration = parse_report(ReportFormat::Trx, "backend:integration", bytes).unwrap();
    let mut reports = BTreeMap::from([("backend:unit".into(), unit)]);
    assert!(consumer.validate_suite_reports(&reports).is_err());
    reports.insert("backend:integration".into(), integration);
    assert!(consumer.validate_suite_reports(&reports).is_err());
    let changed = String::from_utf8(bytes.to_vec())
        .unwrap()
        .replace("testId=\"", "testId=\"integration-");
    reports.insert(
        "backend:integration".into(),
        parse_report(ReportFormat::Trx, "backend:integration", changed.as_bytes()).unwrap(),
    );
    consumer.validate_suite_reports(&reports).unwrap();
}

#[test]
fn implicit_nested_framework_overrides_are_rejected() {
    let mut value = combined();
    value["selection"]["consumer"]["framework_props"] =
        json!({"backend-unit-dependencies":"Directory.Build.props"});
    let selection = parse_profile(&value.to_string())
        .unwrap()
        .selection
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("Tests/Unit")).unwrap();
    project_verification::consumer::validate_component_layout(&selection, "backend", root.path())
        .unwrap();
    for file in [
        "Tests/Directory.Build.props",
        "Tests/Unit/Directory.Build.targets",
        "Directory.Packages.props",
    ] {
        std::fs::write(root.path().join(file), "<Project/>").unwrap();
        assert!(project_verification::consumer::validate_component_layout(
            &selection,
            "backend",
            root.path()
        )
        .is_err());
        std::fs::remove_file(root.path().join(file)).unwrap();
    }
}

#[test]
fn schema_rejects_old_contract_extension_fields_and_wrong_pairings() {
    let schema = schemars::schema_for!(project_verification::ProjectProfile);
    let schema = serde_json::to_value(schema).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut value = profile();
    assert!(validator.is_valid(&value));
    value["selection"]["schema_version"] = json!(1);
    assert!(!validator.is_valid(&value));
    value["schema_version"] = json!(2);
    assert!(!validator.is_valid(&value));
}

#[test]
fn msbuild_case_and_custom_import_properties_cannot_override_bound_framework() {
    use project_verification::consumer::validate_framework_xml;
    let props="<Project><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>";
    for project in [
        "<Project><PropertyGroup><targetframework>net9.0</targetframework></PropertyGroup></Project>",
        "<Project><PropertyGroup><CustomAfterMicrosoftCommonTargets>extra.targets</CustomAfterMicrosoftCommonTargets></PropertyGroup></Project>",
        "<Project><PropertyGroup><MSBuildExtensionsPath>elsewhere</MSBuildExtensionsPath></PropertyGroup></Project>",
        "<Project Sdk=\"Custom.Sdk\"/>",
    ] { assert!(validate_framework_xml(project,props,"net11.0").is_err(),"{project}"); }
    let referenced = props.replace(
        "</Project>",
        "<ItemGroup><ProjectReference Include=\"hidden.csproj\"/></ItemGroup></Project>",
    );
    assert!(validate_framework_xml("<Project/>", &referenced, "net11.0").is_err());
}

#[test]
fn ancestor_targets_outside_component_cannot_change_the_framework() {
    let temp = tempfile::tempdir().unwrap();
    let backend = temp.path().join("backend");
    std::fs::create_dir_all(backend.join("Tests/Unit")).unwrap();
    let mut value = combined();
    value["selection"]["consumer"]["framework_props"] =
        json!({"backend-unit-dependencies":"Directory.Build.props"});
    let selection = parse_profile(&value.to_string())
        .unwrap()
        .selection
        .unwrap();
    std::fs::write(temp.path().join("Directory.Build.targets"), "<Project/>").unwrap();
    assert!(project_verification::consumer::validate_component_layout(
        &selection, "backend", &backend
    )
    .is_err());
}

#[test]
fn rc_trx_contract_binds_the_real_vstest_package_and_lock() {
    use project_verification::selection::validate_available_component_files;
    use std::collections::BTreeMap;
    let selection = parse_profile(&combined().to_string())
        .unwrap()
        .selection
        .unwrap();
    let xml="<Project><PropertyGroup><TargetFramework>net9.0</TargetFramework></PropertyGroup><ItemGroup><PackageReference Include=\"Microsoft.NET.Test.Sdk\" Version=\"17.14.1\"/></ItemGroup></Project>";
    let lock = json!({"dependencies":{"net9.0":{
        "Microsoft.NET.Test.Sdk":{"type":"Direct","resolved":"17.14.1"},
        "Microsoft.TestPlatform.TestHost":{"type":"Transitive","resolved":"17.14.1"},
        "Microsoft.TestPlatform.ObjectModel":{"type":"Transitive","resolved":"17.14.1"}
    }}});
    let mut files = BTreeMap::from([
        (
            "Tests/Unit/Unit.Tests.csproj".into(),
            xml.as_bytes().to_vec(),
        ),
        (
            "Tests/Unit/packages.lock.json".into(),
            serde_json::to_vec(&lock).unwrap(),
        ),
    ]);
    validate_available_component_files(&selection, "backend", &files).unwrap();
    files.insert(
        "Tests/Unit/Unit.Tests.csproj".into(),
        xml.replace("17.14.1", "18.0.0").into_bytes(),
    );
    assert!(validate_available_component_files(&selection, "backend", &files).is_err());
    files.insert(
        "Tests/Unit/Unit.Tests.csproj".into(),
        xml.as_bytes().to_vec(),
    );
    let mut changed = lock.clone();
    changed["dependencies"]["net9.0"]["Microsoft.NET.Test.Sdk"]["resolved"] = json!("18.0.0");
    files.insert(
        "Tests/Unit/packages.lock.json".into(),
        serde_json::to_vec(&changed).unwrap(),
    );
    assert!(validate_available_component_files(&selection, "backend", &files).is_err());
    for package in [
        "Microsoft.TestPlatform.TestHost",
        "Microsoft.TestPlatform.ObjectModel",
    ] {
        let mut changed = lock.clone();
        changed["dependencies"]["net9.0"][package]["resolved"] = json!("18.0.0");
        files.insert(
            "Tests/Unit/packages.lock.json".into(),
            serde_json::to_vec(&changed).unwrap(),
        );
        assert!(
            validate_available_component_files(&selection, "backend", &files).is_err(),
            "{package}"
        );
    }
}
