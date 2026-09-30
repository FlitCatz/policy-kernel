use project_verification::{
    parse_profile,
    tool_versions::{ProjectVersionRule, VersionAcceptance},
};
use serde_json::{json, Value};

fn legacy() -> Value {
    serde_json::from_str(include_str!(
        "../../../examples/react-jest/project-profile.json"
    ))
    .unwrap()
}

fn project() -> Value {
    let mut value = legacy();
    value["schema_version"] = json!(4);
    value["selection"]["schema_version"] = json!(3);
    value["selection"]["runtime_files"] = json!([]);
    for tool in value["selection"]["tools"]
        .as_object_mut()
        .unwrap()
        .values_mut()
    {
        let normalization = tool["version"]["normalization"].clone();
        tool["version"] = json!({"normalization":normalization,"acceptance":{"kind":"record"}});
    }
    value["selection"]["packages"][0]
        .as_object_mut()
        .unwrap()
        .remove("requirement");
    value["selection"]["packages"][0]["acceptance"] = json!({"kind":"record"});
    value["selection"]["contracts"] = json!({
        "node":["process/v1"], "npm":["process/v1","npm-ci-audit/v1"],
        "jest":["jest-json/v1"]
    });
    value
}

#[test]
fn project_observation_accepts_record_and_exact_tokens() {
    let record = ProjectVersionRule {
        normalization: project_verification::selection::VersionNormalization::Plain,
        acceptance: VersionAcceptance::Record {},
    };
    assert_eq!(record.observe(b" 99.0.0\n").unwrap(), "99.0.0");
    let exact = ProjectVersionRule {
        normalization: project_verification::selection::VersionNormalization::Plain,
        acceptance: VersionAcceptance::Exact {
            value: "11.0.100-rc.1.26425.128".into(),
        },
    };
    assert_eq!(
        exact.observe(b"11.0.100-rc.1.26425.128").unwrap(),
        "11.0.100-rc.1.26425.128"
    );
    assert!(exact.observe(b"11.0.100").is_err());
}

#[test]
fn project_observation_rejects_malformed_output_and_ranges() {
    let record = ProjectVersionRule {
        normalization: project_verification::selection::VersionNormalization::Plain,
        acceptance: VersionAcceptance::Record {},
    };
    for output in [
        "",
        "\n",
        "1.0.0\n2.0.0",
        "v1.0.0",
        "1.0.0\0",
        "1".repeat(129).as_str(),
    ] {
        assert!(record.observe(output.as_bytes()).is_err(), "{output:?}");
    }
    let range = ProjectVersionRule {
        normalization: project_verification::selection::VersionNormalization::Plain,
        acceptance: VersionAcceptance::NumericRange {
            requirement: ">=10.0.0, <11.0.0".into(),
        },
    };
    assert!(range.observe(b"11.0.0").is_err());
}

#[test]
fn schema_four_accepts_project_versions_and_legacy_rejects_new_fields() {
    let value = project();
    parse_profile(&value.to_string()).unwrap();
    let mut old = legacy();
    let old_digest =
        policy_core::canonical_sha256(&parse_profile(&old.to_string()).unwrap()).unwrap();
    assert_eq!(
        old_digest,
        policy_core::canonical_sha256(&parse_profile(&legacy().to_string()).unwrap()).unwrap()
    );
    old["selection"]["provisioning"] = json!({});
    assert!(parse_profile(&old.to_string()).is_err());
    assert!(serde_json::from_value::<project_verification::ProjectProfile>(old).is_err());
    for bad in [
        json!({"kind":"record","value":"1.0.0"}),
        json!({"kind":"unknown"}),
        json!(null),
    ] {
        let mut invalid = value.clone();
        invalid["selection"]["tools"]["node"]["version"]["acceptance"] = bad;
        assert!(parse_profile(&invalid.to_string()).is_err());
    }
    let mut invalid = value.clone();
    invalid["id"] = json!("fullstack");
    assert!(parse_profile(&invalid.to_string()).is_err());
    let mut invalid = value.clone();
    invalid["selection"]["tools"]["node"]["version"]["provision"] = json!("22.19.0");
    assert!(parse_profile(&invalid.to_string()).is_err());
}

#[test]
fn project_provisioning_is_optional_but_when_set_must_match_observation() {
    let value = project();
    let profile = parse_profile(&value.to_string()).unwrap();
    let selection = profile.selection.unwrap();
    assert_eq!(
        selection.observe_tool("node", b"v26.10.0\n").unwrap(),
        "26.10.0"
    );
    let mut value = value;
    value["selection"]["provisioning"] = json!({"node":"26.10.0"});
    let profile = parse_profile(&value.to_string()).unwrap();
    let selection = profile.selection.unwrap();
    assert!(selection.observe_tool("node", b"v26.10.1\n").is_err());
    assert!(selection.observe_tool("node", b"v26.10.0\n").is_ok());
}

#[test]
fn declared_runtime_file_is_checked_against_observed_version() {
    let mut value = project();
    value["selection"]["runtime_files"] = json!([{"component":"frontend","path":".node-version","tool":"node","format":"node_version"}]);
    let profile = parse_profile(&value.to_string()).unwrap();
    let selection = profile.selection.unwrap();
    assert!(selection
        .validate_runtime_file(&selection.runtime_files[0], b"26.10.0\n")
        .is_ok());
    assert!(selection
        .validate_observed_runtime(
            "node",
            "26.10.1",
            &std::collections::BTreeMap::from([(
                ("frontend".into(), ".node-version".into()),
                b"26.10.0\n".to_vec()
            )])
        )
        .is_err());
}

#[test]
fn project_runtime_declaration_requires_node_version_and_node_tool() {
    let mut value = project();
    value["selection"]["runtime_files"] = json!([{"component":"frontend","path":"global.json","tool":"node","format":"dotnet_global"}]);
    assert!(parse_profile(&value.to_string()).is_err());
    value["selection"]["runtime_files"] = json!([{"component":"frontend","path":".node-version","tool":"npm","format":"node_version"}]);
    assert!(parse_profile(&value.to_string()).is_err());
}

#[test]
fn project_runtime_tool_must_be_observed_by_a_required_sensor() {
    let mut value = project();
    value["selection"]["runtime_files"] = json!([{"component":"frontend","path":".node-version","tool":"node","format":"node_version"}]);
    value["sensors"].as_array_mut().unwrap().retain(|sensor| {
        sensor["definition"]["id"] != "frontend:node"
            && sensor["definition"]["id"] != "frontend:syntax"
    });
    value["selection"]["bindings"]
        .as_object_mut()
        .unwrap()
        .remove("frontend:node");
    value["selection"]["bindings"]
        .as_object_mut()
        .unwrap()
        .remove("frontend:syntax");
    assert!(parse_profile(&value.to_string()).is_err());
}

#[test]
fn exported_schema_rejects_cross_version_fields() {
    let schema =
        serde_json::to_value(schemars::schema_for!(project_verification::ProjectProfile)).unwrap();
    let validator = jsonschema::draft202012::options().build(&schema).unwrap();
    let mut old = legacy();
    old["selection"]["contracts"] = json!({});
    assert!(!validator.is_valid(&old));
    let mut old_package = legacy();
    old_package["selection"]["packages"][0]
        .as_object_mut()
        .unwrap()
        .remove("requirement");
    old_package["selection"]["packages"][0]["acceptance"] = json!({"kind":"record"});
    assert!(!validator.is_valid(&old_package));
    let mut wrong_tool = project();
    wrong_tool["selection"]["tools"]["node"]["version"] =
        legacy()["selection"]["tools"]["node"]["version"].clone();
    assert!(!validator.is_valid(&wrong_tool));
    let mut wrong_package = project();
    wrong_package["selection"]["packages"][0]
        .as_object_mut()
        .unwrap()
        .remove("acceptance");
    wrong_package["selection"]["packages"][0]["requirement"] = json!(">=1.0.0");
    assert!(!validator.is_valid(&wrong_package));
}

#[test]
fn project_npm_graph_requires_lock_v3_and_full_dependency_declarations() {
    let mut value = project();
    value["selection"]["packages"] = json!([]);
    let selection = parse_profile(&value.to_string())
        .unwrap()
        .selection
        .unwrap();
    let files = |manifest: &str, lock: &str| {
        std::collections::BTreeMap::from([
            ("package.json".into(), manifest.as_bytes().to_vec()),
            ("package-lock.json".into(), lock.as_bytes().to_vec()),
        ])
    };
    assert!(project_verification::selection::validate_component_files(
        &selection,
        "frontend",
        &files("{}", r#"{"lockfileVersion":2,"packages":{"":{}}}"#)
    )
    .is_err());
    assert!(project_verification::selection::validate_component_files(
        &selection,
        "frontend",
        &files(
            r#"{"peerDependencies":{"peer":"1"}}"#,
            r#"{"lockfileVersion":3,"packages":{"":{}}}"#
        )
    )
    .is_err());
}
