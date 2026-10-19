use project_verification::{parse_profile, selection::validate_available_component_files};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const PAIR: &str = "<EmitCompilerGeneratedFiles>true</EmitCompilerGeneratedFiles><CompilerGeneratedFilesOutputPath>obj/generated</CompilerGeneratedFilesOutputPath>";
const PROPS: &str =
    "<Project><PropertyGroup><TargetFramework>net11.0</TargetFramework></PropertyGroup></Project>";

fn profile() -> Value {
    let mut value: Value = serde_json::from_str(include_str!(
        "../../../examples/split-consumer/project-profile.json"
    ))
    .unwrap();
    value["selection"]["consumer"]["generated_files_graphs"] = json!(["modules"]);
    value
}

fn project(pair: &str) -> String {
    format!("<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup>{pair}</PropertyGroup></Project>")
}

fn validate(value: &Value, xml: &str, props: &str) -> Result<(), project_verification::Error> {
    let selection = parse_profile(&value.to_string())?.selection.unwrap();
    validate_available_component_files(
        &selection,
        "backend",
        &BTreeMap::from([
            ("src/Modules/Modules.csproj".into(), xml.as_bytes().to_vec()),
            ("Directory.Build.props".into(), props.as_bytes().to_vec()),
        ]),
    )
}

#[test]
fn only_reviewed_project_can_declare_the_exact_pair() {
    let value = profile();
    validate(&value, &project(PAIR), PROPS).unwrap();
    let mut unbound = value.clone();
    unbound["selection"]["consumer"]["generated_files_graphs"] = json!([]);
    assert!(validate(&unbound, &project(PAIR), PROPS).is_err());
    for id in ["absent", "web"] {
        let mut wrong = value.clone();
        wrong["selection"]["consumer"]["generated_files_graphs"] = json!([id]);
        assert!(parse_profile(&wrong.to_string()).is_err());
    }
    assert!(validate(&value, "<Project/>", PROPS).is_err());
}

#[test]
fn rejects_paths_expansions_duplicates_conditions_and_inheritance() {
    let value = profile();
    for path in [
        "/tmp/generated",
        "../generated",
        "obj/../../outside",
        "C:/generated",
        "C:\\generated",
        "\\\\host\\share",
        "obj\\generated",
        "$(OutputPath)",
        "$([System.IO.Path]::GetTempPath())",
        "$(HOME)",
        "%HOME%",
        "${HOME}",
        "obj/generated/",
        " obj/generated",
    ] {
        assert!(
            validate(
                &value,
                &project(&PAIR.replace("obj/generated", path)),
                PROPS
            )
            .is_err(),
            "{path}"
        );
    }
    for pair in [
        PAIR.replace(">true<", ">false<"),
        PAIR.replace(">true<", ">True<"),
        PAIR.replace("<EmitCompilerGeneratedFiles>", "<EmitCompilerGeneratedFiles Condition=\"true\">"),
        format!("{PAIR}<EmitCompilerGeneratedFiles>true</EmitCompilerGeneratedFiles>"),
        format!("{PAIR}<emitcompilergeneratedfiles>true</emitcompilergeneratedfiles>"),
        format!("{PAIR}<compilergeneratedfilesoutputpath>obj/generated</compilergeneratedfilesoutputpath>"),
        PAIR.replace("</EmitCompilerGeneratedFiles>", "</EmitCompilerGeneratedFiles></PropertyGroup><PropertyGroup>"),
    ] {
        assert!(validate(&value, &project(&pair), PROPS).is_err(), "{pair}");
    }
    for xml in [
        project(PAIR).replace("<PropertyGroup>", "<PropertyGroup Condition=\"true\">"),
        project(PAIR).replace("<Project ", "<Project Condition=\"true\" "),
        project(PAIR).replace("<Project ", "<Project TreatAsLocalProperty=\"CompilerGeneratedFilesOutputPath\" "),
        project(PAIR).replace("</Project>", "<Import Project=\"outside.props\"/></Project>"),
        project(PAIR).replace("</Project>", "<Target Name=\"BeforeBuild\"/></Project>"),
        project(PAIR).replace("</Project>", "<ItemGroup><ProjectReference Include=\"../Api/Api.csproj\" AdditionalProperties=\"CompilerGeneratedFilesOutputPath=/tmp\" /></ItemGroup></Project>"),
    ] {
        assert!(validate(&value, &xml, PROPS).is_err(), "{xml}");
    }
    for declaration in [
        PAIR,
        "<EmitCompilerGeneratedFiles>true</EmitCompilerGeneratedFiles>",
        "<CompilerGeneratedFilesOutputPath>obj/generated</CompilerGeneratedFilesOutputPath>",
    ] {
        let props = PROPS.replace(
            "</PropertyGroup>",
            &format!("{declaration}</PropertyGroup>"),
        );
        assert!(validate(&value, &project(PAIR), &props).is_err());
    }
}

#[test]
fn audit_suppression_and_lock_overrides_stay_rejected() {
    for extra in [
        "<NuGetLockFilePath>other.json</NuGetLockFilePath>",
        "<NoWarn>NU1901</NoWarn>",
        "<WarningsNotAsErrors>NU1901</WarningsNotAsErrors>",
        "<NuGetAudit>false</NuGetAudit>",
    ] {
        assert!(validate(&profile(), &project(&format!("{PAIR}{extra}")), PROPS).is_err());
    }
    for spelling in ["NuGetAuditSuppress", "nugetauditsuppress"] {
        let xml = project(PAIR).replace("</Project>", &format!("<ItemGroup><{spelling} Include=\"https://example.invalid/advisory\" /></ItemGroup></Project>"));
        assert!(validate(&profile(), &xml, PROPS).is_err());
    }
}

#[test]
fn command_and_environment_overrides_cannot_replace_reviewed_values() {
    for arg in [
        "-p:CompilerGeneratedFilesOutputPath=/tmp",
        "/p:EmitCompilerGeneratedFiles=false",
        "--property:EmitCompilerGeneratedFiles=false",
        "@outside.rsp",
        "--",
        "--artifacts-path",
        "-p:DirectoryBuildPropsPath=outside.props",
    ] {
        let mut value = profile();
        let sensor = value["sensors"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["definition"]["id"] == "backend:unit")
            .unwrap();
        sensor["argv"].as_array_mut().unwrap().push(json!(arg));
        assert!(parse_profile(&value.to_string()).is_err(), "{arg}");
    }
    for key in [
        "EmitCompilerGeneratedFiles",
        "CompilerGeneratedFilesOutputPath",
        "MSBuildExtensionsPath",
        "DirectoryBuildPropsPath",
    ] {
        let mut value = profile();
        value["sensors"][0]["environment_allowlist"]
            .as_array_mut()
            .unwrap()
            .push(json!(key));
        assert!(parse_profile(&value.to_string()).is_err(), "{key}");
    }
    // A version probe must not run a second MSBuild command.
    let mut value = profile();
    value["selection"]["tools"]["backend:unit"]["version_args"] = json!(["build"]);
    let sensor = value["sensors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["definition"]["id"] == "backend:unit")
        .unwrap();
    sensor["version_args"] = json!(["build"]);
    assert!(parse_profile(&value.to_string()).is_err());
}

#[test]
fn nested_props_targets_and_response_files_are_rejected() {
    let selection = parse_profile(&profile().to_string())
        .unwrap()
        .selection
        .unwrap();
    let temp = tempfile::tempdir().unwrap();
    for dir in ["src/Modules", "src/Api", "tests/Tests"] {
        std::fs::create_dir_all(temp.path().join(dir)).unwrap();
    }
    project_verification::consumer::validate_component_layout(&selection, "backend", temp.path())
        .unwrap();
    for path in [
        "src/Directory.Build.props",
        "src/Modules/Directory.Build.props",
        "src/Modules/Directory.Build.targets",
        "src/Directory.Packages.props",
        "src/Modules/Directory.Build.rsp",
        "MSBuild.rsp",
    ] {
        let file = temp.path().join(path);
        std::fs::write(&file, "<Project/>").unwrap();
        assert!(
            project_verification::consumer::validate_component_layout(
                &selection,
                "backend",
                temp.path()
            )
            .is_err(),
            "{path}"
        );
        std::fs::remove_file(file).unwrap();
    }
}

#[cfg(unix)]
#[test]
fn actual_output_parent_and_descendant_symlinks_are_rejected() {
    use std::{fs, os::unix::fs::symlink};
    let selection = parse_profile(&profile().to_string())
        .unwrap()
        .selection
        .unwrap();
    for path in [
        "obj",
        "obj/generated",
        "obj/generated/mapper",
        "obj/generated/result.cs",
    ] {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("src/Modules");
        let link = project.join(path);
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        symlink(outside.path(), &link).unwrap();
        assert!(
            project_verification::consumer::validate_component_layout(
                &selection,
                "backend",
                temp.path()
            )
            .is_err(),
            "{path}"
        );
        fs::remove_file(link).unwrap();
        project_verification::consumer::validate_component_layout(
            &selection,
            "backend",
            temp.path(),
        )
        .unwrap();
    }
}

#[test]
fn generated_output_does_not_allow_a_shell_as_the_selected_dotnet_tool() {
    let mut value = profile();
    value["selection"]["tools"]["backend:unit"]["executable"] = json!("sh");
    let sensor = value["sensors"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["definition"]["id"] == "backend:unit")
        .unwrap();
    sensor["argv"][0] = json!("sh");
    assert!(parse_profile(&value.to_string()).is_err());
}

#[test]
fn xml_comments_cannot_hide_a_property_value_suffix() {
    for pair in [
        PAIR.replace("obj/generated", "obj/generated<!-- split -->/../../outside"),
        PAIR.replace(">true<", ">true<!-- split -->false<"),
    ] {
        assert!(validate(&profile(), &project(&pair), PROPS).is_err());
    }
}
