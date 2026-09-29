//! Generate the reviewed synthetic consumer profile; no tool execution or adoption.
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn main() {
    let mut sensors = Vec::new();
    let mut tools = BTreeMap::<String, Value>::new();
    let mut bindings = BTreeMap::new();
    let mut graphs = Vec::new();
    let mut suites = Vec::new();
    let mut props = BTreeMap::new();
    let mut add = |id: &str,
                   component: &str,
                   argv: Vec<String>,
                   probe: Vec<String>,
                   version: &str,
                   capability: &str,
                   producer: Option<&str>,
                   report: Option<Value>,
                   inputs: Vec<String>| {
        let requirement = format!("={version}");
        let capabilities = if capability == "process" {
            vec![capability]
        } else {
            vec!["process", capability]
        };
        let mut tool = json!({"executable":argv[0],"version_args":probe,"version":{"requirement":requirement,"normalization":if id=="frontend:node" {"node"}else{"plain"},"provision":version},"capabilities":capabilities});
        if let Some(p) = producer {
            tool["producer_contract"] = json!(p);
        }
        tools.insert(id.into(), tool);
        bindings.insert(id.to_owned(), id.to_owned());
        sensors.push(json!({"definition":{"id":id,"required":true,"required_when":[],"not_applicable_when":[],"accepted_exit_codes":[0],"result_requirement":if report.is_some(){"test_counts"}else{"process"}},
            "component":component,"argv":argv,"version_args":probe,"version_stdout":"","environment_allowlist":["HOME","PATH","DOTNET_ROOT"],
            "report":report,"inputs":inputs,"pins":[],"timeout_ms":300000}));
    };
    let sdk = "11.0.100-rc.1.26425.128";
    for (id, manifest, role) in [
        ("app", "src/Api/Api.csproj", "application"),
        ("modules", "src/Modules/Modules.csproj", "application"),
        ("tests", "tests/Tests/Tests.csproj", "unit"),
    ] {
        let sensor = format!("backend:{id}-audit");
        let lock = format!(
            "{}/packages.lock.json",
            manifest.rsplit_once('/').unwrap().0
        );
        let mut argv: Vec<String> = [
            "dotnet",
            "restore",
            manifest,
            "--locked-mode",
            "--force",
            "--no-http-cache",
            "-p:NuGetAudit=true",
            "-p:NuGetAuditMode=all",
            "-p:NuGetAuditLevel=low",
            "-p:WarningsAsErrors=NU1900%3BNU1901%3BNU1902%3BNU1903%3BNU1904%3BNU1905",
            "--configfile",
            "NuGet.Config",
            "-p:NoWarn=",
            "-p:WarningsNotAsErrors=",
        ]
        .map(str::to_owned)
        .into();
        add(
            &sensor,
            "backend",
            std::mem::take(&mut argv),
            vec!["--version".into()],
            sdk,
            "nuget_audit",
            Some("dotnet11_rc1"),
            None,
            vec![
                manifest.into(),
                lock.clone(),
                "NuGet.Config".into(),
                "Directory.Build.props".into(),
                "global.json".into(),
            ],
        );
        graphs.push(json!({"id":id,"component":"backend","role":role,"manifest":manifest,"lockfile":lock,"adapter":"nuget","tool":sensor,"audit_sensor":sensor,"target_framework":"net11.0"}));
        props.insert(id, "Directory.Build.props");
    }
    for role in ["unit", "integration", "contract"] {
        let sensor = format!("backend:{role}");
        let output = format!(".policy-reports/{role}.trx");
        let filter = format!(
            "Suite={}",
            match role {
                "unit" => "Unit",
                "integration" => "Integration",
                _ => "Contract",
            }
        );
        add(
            &sensor,
            "backend",
            vec![
                "dotnet".into(),
                "test".into(),
                "tests/Tests/Tests.csproj".into(),
                "--no-restore".into(),
                "--filter".into(),
                filter,
                "--logger".into(),
                format!("trx;LogFileName={role}.trx"),
                "--results-directory".into(),
                ".policy-reports".into(),
            ],
            vec!["--version".into()],
            sdk,
            "trx",
            Some("dotnet11_rc1"),
            Some(json!({"format":"trx","path":output})),
            vec![
                "tests/Tests/Tests.csproj".into(),
                "tests/Tests/packages.lock.json".into(),
                "global.json".into(),
            ],
        );
        suites.push(json!({"role":role,"sensor":sensor,"graph":"tests"}));
    }
    add(
        "frontend:node",
        "frontend",
        vec!["node".into(), "--version".into()],
        vec!["--version".into()],
        "26.10.0",
        "process",
        None,
        None,
        vec![
            ".node-version".into(),
            "package.json".into(),
            "package-lock.json".into(),
        ],
    );
    add(
        "frontend:audit",
        "frontend",
        [
            "npm",
            "audit",
            "--audit-level=low",
            "--include=dev",
            "--include=optional",
            "--include=peer",
        ]
        .map(str::to_owned)
        .into(),
        vec!["--version".into()],
        "10.9.3",
        "npm_audit",
        None,
        None,
        vec!["package.json".into(), "package-lock.json".into()],
    );
    graphs.push(json!({"id":"web","component":"frontend","role":"application","manifest":"package.json","lockfile":"package-lock.json","adapter":"npm","tool":"frontend:audit","audit_sensor":"frontend:audit","target_framework":null}));
    add(
        "frontend:unit",
        "frontend",
        [
            "node",
            "node_modules/vitest/vitest.mjs",
            "run",
            "--retry=0",
            "--reporter=json",
            "--outputFile=.policy-reports/unit.json",
        ]
        .map(str::to_owned)
        .into(),
        vec![
            "-p".into(),
            "require('./node_modules/vitest/package.json').version".into(),
        ],
        "4.1.11",
        "vitest_json",
        None,
        Some(json!({"format":"vitest_json","path":".policy-reports/unit.json"})),
        [
            "package.json",
            "package-lock.json",
            "vitest.config.ts",
            "node_modules/vitest/package.json",
        ]
        .map(str::to_owned)
        .into(),
    );
    add(
        "frontend:e2e",
        "frontend",
        [
            "node",
            "node_modules/@playwright/test/cli.js",
            "test",
            "--config=playwright.config.ts",
        ]
        .map(str::to_owned)
        .into(),
        vec![
            "-p".into(),
            "require('./node_modules/@playwright/test/package.json').version".into(),
        ],
        "1.63.0",
        "playwright_json",
        Some("playwright163"),
        Some(json!({"format":"playwright_json","path":".policy-reports/e2e.json"})),
        [
            "package.json",
            "package-lock.json",
            "playwright.config.ts",
            "node_modules/@playwright/test/package.json",
        ]
        .map(str::to_owned)
        .into(),
    );
    suites.extend([
        json!({"role":"unit","sensor":"frontend:unit","graph":"web"}),
        json!({"role":"e2e","sensor":"frontend:e2e","graph":"web"}),
    ]);
    let packages:Vec<_>=[("vitest","4.1.11"),("@playwright/test","1.63.0"),("vite","6.4.3"),("react","19.0.0"),("react-dom","19.0.0")].into_iter().map(|(name,v)|json!({"component":"frontend","installed_manifest":format!("node_modules/{name}/package.json"),"lockfile":"package-lock.json","lock_pointer":format!("/packages/node_modules~1{}/version",name.replace('/',"~1")),"requirement":format!("={v}")})).collect();
    let profile = json!({"schema_version":3,"id":"fullstack","version":"1.2.0","components":[{"id":"frontend","root":"frontend"},{"id":"backend","root":"backend"}],"sensors":sensors,
        "adequacy":{"acceptance_criterion_ids":[],"changed_behavior_ids":[],"negative_case_ids":[]},
        "selection":{"schema_version":2,"tools":tools,"bindings":bindings,"packages":packages,"graphs":graphs,
        "runtime_files":[{"component":"frontend","path":".node-version","tool":"frontend:node","format":"node_version"},{"component":"backend","path":"global.json","tool":"backend:app-audit","format":"dotnet_global"}],
        "consumer":{"development_only":true,"suites":suites,"framework_props":props,"runtime_policies":{"backend:global.json":{"roll_forward":"latestPatch","allow_prerelease":true}}}}});
    project_verification::parse_profile(&profile.to_string()).unwrap();
    println!("{}", serde_json::to_string_pretty(&profile).unwrap());
}
