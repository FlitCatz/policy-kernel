//! Offline native P2 proof. This is not a NuGet audit or trusted evidence.
use serde_json::{json, Value};
use std::{fs, path::Path, process::Command};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &to.join(entry.file_name()));
        } else {
            fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
#[ignore = "requires preinstalled .NET RC1 and complete local NuGet cache; never downloads"]
fn mapperly_files_are_generated_and_read_by_architecture_test() {
    let sdk =
        std::env::var("POLICY_GENERATED_DOTNET").expect("absolute path to installed dotnet RC1");
    let packages = std::env::var("POLICY_GENERATED_PACKAGES").expect("existing NuGet cache path");
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let source = workspace.join("examples/tjerp-compatible/backend");
    // Fail before launching .NET if any selected package is absent. No fallback feed.
    for lock in [
        "src/Api/packages.lock.json",
        "src/Modules/packages.lock.json",
        "tests/Tests/packages.lock.json",
    ] {
        let value: Value = serde_json::from_slice(&fs::read(source.join(lock)).unwrap()).unwrap();
        for (name, info) in value["dependencies"]["net11.0"].as_object().unwrap() {
            if let Some(version) = info["resolved"].as_str() {
                let path = Path::new(&packages).join(name.to_lowercase()).join(version);
                assert!(
                    path.join(".nupkg.metadata").is_file(),
                    "needs-decision: missing cached package {}",
                    path.display()
                );
            }
        }
    }
    assert!(
        Path::new(&packages)
            .join("riok.mapperly/4.3.1/.nupkg.metadata")
            .is_file(),
        "needs-decision: Mapperly 4.3.1 missing"
    );
    let temp = tempfile::tempdir().unwrap();
    let root = if let Some(output) = std::env::var_os("POLICY_GENERATED_OUTPUT") {
        let output = std::path::PathBuf::from(output);
        assert!(output.is_absolute(), "native output must be absolute");
        fs::create_dir(&output).expect("fresh native output directory");
        output.join("backend")
    } else {
        temp.path().join("backend")
    };
    copy_tree(&source, &root);
    fs::write(root.join("NuGet.Config"), "<configuration><packageSources><clear /></packageSources><auditSources><clear /></auditSources></configuration>").unwrap();
    fs::write(root.join("src/Modules/Modules.csproj"), r#"<Project Sdk="Microsoft.NET.Sdk">
<PropertyGroup><EmitCompilerGeneratedFiles>true</EmitCompilerGeneratedFiles><CompilerGeneratedFilesOutputPath>obj/generated</CompilerGeneratedFilesOutputPath></PropertyGroup>
<ItemGroup><PackageReference Include="Riok.Mapperly" Version="4.3.1" PrivateAssets="all" /></ItemGroup>
</Project>"#).unwrap();
    fs::write(
        root.join("src/Modules/Mapping.cs"),
        r#"using Riok.Mapperly.Abstractions;
namespace Modules;
public record Source(string Name);
public record Destination(string Name);
[Mapper] public static partial class GeneratedMapper {
    public static partial Destination Map(Source source);
}
"#,
    )
    .unwrap();
    let lock = root.join("src/Modules/packages.lock.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&lock).unwrap()).unwrap();
    value["dependencies"]["net11.0"]["Riok.Mapperly"] = json!({"type":"Direct","requested":"[4.3.1, )","resolved":"4.3.1","contentHash":"oSE+OVLHyiojrws0QVbQaAT2YKsYbyHqJXPdV7EhGseR/dYgeWS2x1T5qY5I4E+RJkJqMSG+FyZOq4SljWgJWQ=="});
    fs::write(lock, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    fs::write(root.join("tests/Tests/ArchitectureTests.cs"), r#"using Xunit;
public class ArchitectureTests {
    [Fact, Trait("Suite", "GeneratedArchitecture")]
    public void Reads_actual_mapperly_generated_source() {
        var dir = new System.IO.DirectoryInfo(System.AppContext.BaseDirectory);
        while (dir != null && !System.IO.File.Exists(System.IO.Path.Combine(dir.FullName, "Directory.Build.props"))) dir = dir.Parent;
        Assert.NotNull(dir);
        var output = System.IO.Path.Combine(dir.FullName, "src", "Modules", "obj", "generated");
        var files = System.IO.Directory.GetFiles(output, "*.cs", System.IO.SearchOption.AllDirectories);
        Assert.NotEmpty(files);
        var sources = string.Join("\n", files.Select(System.IO.File.ReadAllText));
        Assert.Contains("GeneratedMapper", sources);
        Assert.Contains("source.Name", sources);
        Assert.Equal("native", Modules.GeneratedMapper.Map(new Modules.Source("native")).Name);
    }
}
"#).unwrap();
    let mut profile: Value = serde_json::from_slice(
        &fs::read(workspace.join("examples/tjerp-compatible/project-profile.json")).unwrap(),
    )
    .unwrap();
    profile["selection"]["consumer"]["generated_files_graphs"] = json!(["modules"]);
    let parsed = project_verification::parse_profile(&profile.to_string()).unwrap();
    let selection = parsed.selection.unwrap();
    let files = selection
        .inputs("backend")
        .into_iter()
        .map(|path| {
            let bytes = fs::read(root.join(&path)).unwrap();
            (path, bytes)
        })
        .collect();
    project_verification::selection::validate_component_files(&selection, "backend", &files)
        .unwrap();
    project_verification::consumer::validate_component_layout(&selection, "backend", &root)
        .unwrap();
    let invocation = std::cell::Cell::new(0);
    let invoke = |args: &[&str]| {
        let index = invocation.get();
        invocation.set(index + 1);
        let log = root.join(format!("native-{index}"));
        fs::write(
            log.with_extension("argv.json"),
            serde_json::to_vec(args).unwrap(),
        )
        .unwrap();
        let mut child = Command::new(&sdk)
            .args(args)
            .current_dir(&root)
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("HOME", temp.path())
            .env("DOTNET_CLI_HOME", temp.path())
            .env("NUGET_PACKAGES", &packages)
            .env("DOTNET_SKIP_FIRST_TIME_EXPERIENCE", "1")
            .env("DOTNET_GENERATE_ASPNET_CERTIFICATE", "false")
            .env("DOTNET_CLI_USE_MSBUILD_SERVER", "0")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .stdout(fs::File::create(log.with_extension("stdout")).unwrap())
            .stderr(fs::File::create(log.with_extension("stderr")).unwrap())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(120);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if std::time::Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!(
                    "native command timed out: {args:?}; logs at {}",
                    log.display()
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        };
        fs::write(log.with_extension("status"), status.to_string()).unwrap();
        std::process::Output {
            status,
            stdout: fs::read(log.with_extension("stdout")).unwrap(),
            stderr: fs::read(log.with_extension("stderr")).unwrap(),
        }
    };
    let version = invoke(&["--version"]);
    assert_eq!(
        String::from_utf8_lossy(&version.stdout).trim(),
        "11.0.100-rc.1.26425.128"
    );
    for args in [
        vec![
            "restore",
            "tests/Tests/Tests.csproj",
            "--locked-mode",
            "--configfile",
            "NuGet.Config",
            "-p:NuGetAudit=false",
        ],
        vec![
            "test",
            "tests/Tests/Tests.csproj",
            "--no-restore",
            "--filter",
            "Suite=GeneratedArchitecture",
            "--logger",
            "trx;LogFileName=generated.trx",
            "--results-directory",
            ".policy-reports",
        ],
    ] {
        let result = invoke(&args);
        assert!(
            result.status.success(),
            "{args:?}\n{}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    let report = project_verification::reports::parse_report(
        project_verification::reports::ReportFormat::Trx,
        "backend:architecture",
        &fs::read(root.join(".policy-reports/generated.trx")).unwrap(),
    )
    .unwrap();
    assert!(report.successful);
    assert_eq!(report.executed, 1);
    project_verification::consumer::validate_component_layout(&selection, "backend", &root)
        .unwrap();
    // An empty directory must fail the very same compiled ArchitectureTests test.
    fs::remove_dir_all(root.join("src/Modules/obj/generated")).unwrap();
    fs::create_dir(root.join("src/Modules/obj/generated")).unwrap();
    let empty = invoke(&[
        "test",
        "tests/Tests/Tests.csproj",
        "--no-restore",
        "--no-build",
        "--filter",
        "Suite=GeneratedArchitecture",
    ]);
    assert!(
        !empty.status.success(),
        "empty output was incorrectly accepted"
    );
    assert!(String::from_utf8_lossy(&empty.stdout).contains("Assert.NotEmpty"));
}
