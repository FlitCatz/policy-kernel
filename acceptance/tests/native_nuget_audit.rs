//! Reproduce the historical audit scope with an actual test-only vulnerable package.
use serde_json::json;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn observe(root: &Path, output: &Path, name: &str, args: &[&str]) -> serde_json::Value {
    let result = Command::new("dotnet")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    fs::write(output.join(format!("{name}.stdout.log")), &result.stdout).unwrap();
    fs::write(output.join(format!("{name}.stderr.log")), &result.stderr).unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    json!({"argv":std::iter::once("dotnet").chain(args.iter().copied()).collect::<Vec<_>>(),
        "exit_code":result.status.code(),"success":result.status.success(),
        "vulnerability_diagnostic":(["NU1901", "NU1902", "NU1903", "NU1904"].iter().any(|code| text.contains(code))),
        "service_diagnostic":text.contains("NU1900") || text.contains("NU1301")})
}

#[test]
#[ignore = "requires .NET9, NuGet network and POLICY_NUGET_OUTPUT"]
fn reproduce_test_only_nuget_vulnerability_with_restore_cache_and_warning_variants() {
    let output = PathBuf::from(
        std::env::var_os("POLICY_NUGET_OUTPUT").expect("set a fresh output directory"),
    );
    fs::create_dir(&output).unwrap();
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let root = output.join("consumer");
    policy_acceptance::stage(&workspace.join("examples"), "dotnet", &root).unwrap();
    let unit = "Tests/Unit/Unit.Tests.csproj";
    let original = fs::read_to_string(root.join(unit)).unwrap();
    fs::write(root.join(unit), original.replace("</Project>", "<ItemGroup><PackageReference Include=\"System.Text.Encodings.Web\" Version=\"4.7.1\" /></ItemGroup></Project>")).unwrap();
    let mut observations = serde_json::Map::new();
    // Deliberately prepare the changed lock graph without claiming this is an audit.
    for (name, project) in [
        ("app", "App.csproj"),
        ("unit", unit),
        ("integration", "Tests/Integration/Integration.Tests.csproj"),
    ] {
        let observation = observe(
            &root,
            &output,
            &format!("lock-preparation-{name}"),
            &[
                "restore",
                project,
                "--force-evaluate",
                "-p:NuGetAudit=false",
            ],
        );
        assert_eq!(observation["success"], true, "lock preparation {name}");
        observations.insert(format!("lock-preparation-{name}"), observation);
    }
    for (name, project) in [
        ("app", "App.csproj"),
        ("unit", unit),
        ("integration", "Tests/Integration/Integration.Tests.csproj"),
    ] {
        observations.insert(
            format!("prior-restore-{name}"),
            observe(
                &root,
                &output,
                &format!("prior-restore-{name}"),
                &["restore", project, "--locked-mode"],
            ),
        );
    }
    let fresh_packages = output.join("fresh-packages");
    observations.insert(
        "fresh-package-cache-unit".into(),
        observe(
            &root,
            &output,
            "fresh-package-cache-unit",
            &[
                "restore",
                unit,
                "--locked-mode",
                "--force",
                "--no-http-cache",
                "--packages",
                fresh_packages.to_str().unwrap(),
            ],
        ),
    );
    let legacy = project_verification::parse_profile(include_str!(
        "../../bootstrap/templates/1.0.2/profiles/dotnet/profile.json"
    ))
    .unwrap();
    for (phase, suppress) in [("plain", false), ("suppressed", true)] {
        if suppress {
            let text = fs::read_to_string(root.join(unit)).unwrap();
            fs::write(root.join(unit), text.replace("</PropertyGroup>", "<NuGetAudit>false</NuGetAudit><NoWarn>NU1900;NU1901;NU1902;NU1903;NU1904;NU1905</NoWarn></PropertyGroup>")).unwrap();
            observe(
                &root,
                &output,
                "suppressed-cache-restore",
                &["restore", unit, "--locked-mode"],
            );
        }
        for sensor in &legacy.sensors {
            if let Some(report) = &sensor.report {
                let file = root.join(&report.path);
                if file.exists() {
                    fs::remove_file(file).unwrap();
                }
            }
            let name = format!(
                "legacy-{phase}-{}",
                sensor.definition.id.as_str().replace(':', "-")
            );
            let args: Vec<_> = sensor.argv[1..].iter().map(String::as_str).collect();
            observations.insert(name.clone(), observe(&root, &output, &name, &args));
        }
        let selected = project_verification::selection::upgrade_profile(legacy.clone()).unwrap();
        let selection = selected.selection.as_ref().unwrap();
        for graph in &selection.graphs {
            let args = graph.audit_argv(&selection.tools[&graph.tool]);
            let args: Vec<_> = args[1..].iter().map(String::as_str).collect();
            let name = format!("selected-{phase}-{}", graph.id);
            observations.insert(name.clone(), observe(&root, &output, &name, &args));
        }
    }
    let version = Command::new("dotnet")
        .arg("--version")
        .current_dir(&root)
        .output()
        .unwrap();
    let summary = json!({"scope":"local-network-reproduction", "sdk":String::from_utf8_lossy(&version.stdout).trim(),
        "test_only_package":"System.Text.Encodings.Web@4.7.1", "observations":observations,
        "harness":policy_acceptance::harness::observe(workspace).unwrap()});
    fs::write(
        output.join("summary.json"),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
    assert_eq!(
        summary["observations"]["selected-plain-backend-unit-dependencies"]["success"],
        false
    );
    assert_eq!(
        summary["observations"]["selected-plain-backend-unit-dependencies"]
            ["vulnerability_diagnostic"],
        true
    );
    assert_eq!(
        summary["observations"]["selected-suppressed-backend-unit-dependencies"]["success"],
        false
    );
    assert_eq!(
        summary["observations"]["selected-suppressed-backend-unit-dependencies"]
            ["vulnerability_diagnostic"],
        true
    );
}
