//! Native npm 11 workspace conformance. Installations are registry-backed and temporary.
use serde_json::{json, Value};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
#[path = "../../crates/project-verification/tests/common/npm_workspace.rs"]
mod workspace;

fn command(root: &Path, program: &str, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(root)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap_or_default())
        .env("npm_config_registry", "https://registry.npmjs.org/")
        .env("npm_config_logs_dir", root.join("logs"))
        .env("npm_config_cache", root.join(".npm-cache"))
        .env("npm_config_fetch_retries", "0")
        .env("npm_config_fetch_timeout", "30000")
        .output()
        .unwrap()
}

fn checked(root: &Path, program: &str, args: &[&str]) -> Output {
    let result = command(root, program, args);
    assert!(
        result.status.success(),
        "{program} {args:?}: {}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    result
}

#[test]
#[ignore = "requires installed Node 26.12.0/npm 11.20.0, registry access for TS 6.0.3/5.9.3/lodash 4.17.20 and live npm audit access"]
fn native_npm_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    assert_eq!(
        String::from_utf8(checked(root, "node", &["--version"]).stdout)
            .unwrap()
            .trim(),
        "v26.12.0"
    );
    assert_eq!(
        String::from_utf8(checked(root, "npm", &["--version"]).stdout)
            .unwrap()
            .trim(),
        "11.20.0"
    );
    let request = workspace::fixture(root);
    // Remove synthetic unit-test inputs before the native installation; npm generates
    // the real shared lock, installed packages, and workspace link from npm registry data.
    fs::remove_dir_all(root.join("node_modules")).unwrap();
    fs::remove_dir_all(root.join("tools/openapi/node_modules")).unwrap();
    fs::remove_file(root.join("package-lock.json")).unwrap();
    let install = checked(
        root,
        "npm",
        &[
            "install",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--include=dev",
            "--include=optional",
            "--include=peer",
            "--workspaces",
            "--include-workspace-root",
            "--userconfig=.policy-npm-userconfig",
            "--globalconfig=.policy-npm-globalconfig",
        ],
    );
    let lock: Value =
        serde_json::from_slice(&fs::read(root.join("package-lock.json")).unwrap()).unwrap();
    assert_eq!(
        lock["packages"]["node_modules/typescript"]["version"],
        "6.0.3"
    );
    assert_eq!(
        lock["packages"]["tools/openapi/node_modules/typescript"]["version"],
        "5.9.3"
    );
    project_verification::run::prepare(&request).unwrap();
    for (directory, version) in [
        (root.to_path_buf(), "6.0.3"),
        (root.join("tools/openapi"), "5.9.3"),
    ] {
        let resolved = checked(
            &directory,
            "node",
            &["-p", "require('typescript/package.json').version"],
        );
        assert_eq!(String::from_utf8(resolved.stdout).unwrap().trim(), version);
    }
    let audit_args = [
        "audit",
        "--audit-level=low",
        "--include=dev",
        "--include=optional",
        "--include=peer",
        "--workspaces",
        "--include-workspace-root",
        "--userconfig=.policy-npm-userconfig",
        "--globalconfig=.policy-npm-globalconfig",
        "--json",
    ];
    let baseline = checked(root, "npm", &audit_args);
    let baseline_json: Value = serde_json::from_slice(&baseline.stdout).unwrap();
    assert_eq!(baseline_json["metadata"]["vulnerabilities"]["total"], 0);
    let baseline_lock = fs::read(root.join("package-lock.json")).unwrap();
    // Only the member gets the vulnerable dependency. A root audit must catch it.
    let member_path = root.join("tools/openapi/package.json");
    let mut member: Value = serde_json::from_slice(&fs::read(&member_path).unwrap()).unwrap();
    member["dependencies"] = json!({"lodash":"4.17.20"});
    fs::write(&member_path, serde_json::to_vec_pretty(&member).unwrap()).unwrap();
    checked(
        root,
        "npm",
        &[
            "install",
            "--ignore-scripts",
            "--no-audit",
            "--no-fund",
            "--include=dev",
            "--include=optional",
            "--include=peer",
            "--workspaces",
            "--include-workspace-root",
            "--userconfig=.policy-npm-userconfig",
            "--globalconfig=.policy-npm-globalconfig",
        ],
    );
    project_verification::run::prepare(&request).unwrap();
    let vulnerable = command(root, "npm", &audit_args);
    assert_eq!(vulnerable.status.code(), Some(1));
    let vulnerable_json: Value = serde_json::from_slice(&vulnerable.stdout).unwrap();
    assert!(
        vulnerable_json["vulnerabilities"]["lodash"].is_object(),
        "{}",
        String::from_utf8_lossy(&vulnerable.stdout)
    );
    let ts5 = root.join("tools/openapi/node_modules/typescript/package.json");
    let original = fs::read(&ts5).unwrap();
    fs::copy(root.join("node_modules/typescript/package.json"), &ts5).unwrap();
    assert!(project_verification::run::prepare(&request).is_err());
    fs::write(ts5, original).unwrap();
    let member_lock = fs::read(root.join("package-lock.json")).unwrap();
    // A separate root-only vulnerable declaration proves include-workspace-root.
    member.as_object_mut().unwrap().remove("dependencies");
    fs::write(&member_path, serde_json::to_vec_pretty(&member).unwrap()).unwrap();
    let root_path = root.join("package.json");
    let mut manifest: Value = serde_json::from_slice(&fs::read(&root_path).unwrap()).unwrap();
    manifest["dependencies"] = json!({"lodash":"4.17.20"});
    fs::write(&root_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let install_args = [
        "install",
        "--ignore-scripts",
        "--no-audit",
        "--no-fund",
        "--include=dev",
        "--include=optional",
        "--include=peer",
        "--workspaces",
        "--include-workspace-root",
        "--userconfig=.policy-npm-userconfig",
        "--globalconfig=.policy-npm-globalconfig",
    ];
    checked(root, "npm", &install_args);
    project_verification::run::prepare(&request).unwrap();
    let root_vulnerable = command(root, "npm", &audit_args);
    assert_eq!(root_vulnerable.status.code(), Some(1));
    let root_json: Value = serde_json::from_slice(&root_vulnerable.stdout).unwrap();
    assert!(root_json["vulnerabilities"]["lodash"].is_object());
    let root_lock = fs::read(root.join("package-lock.json")).unwrap();
    manifest.as_object_mut().unwrap().remove("dependencies");
    fs::write(&root_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    checked(root, "npm", &install_args);
    project_verification::run::prepare(&request).unwrap();
    let recovery = checked(root, "npm", &audit_args);
    let recovered: Value = serde_json::from_slice(&recovery.stdout).unwrap();
    assert_eq!(recovered["metadata"]["vulnerabilities"]["total"], 0);
    if let Some(out) = std::env::var_os("POLICY_NPM_WORKSPACE_OUTPUT") {
        let out = std::path::PathBuf::from(out);
        fs::create_dir(&out).unwrap();
        for (name, bytes) in [
            ("baseline-audit.json", baseline.stdout),
            ("member-vulnerability-audit.json", vulnerable.stdout),
            ("root-vulnerability-audit.json", root_vulnerable.stdout),
            ("recovery-audit.json", recovery.stdout),
            ("baseline-package-lock.json", baseline_lock),
            ("member-vulnerability-package-lock.json", member_lock),
            ("root-vulnerability-package-lock.json", root_lock),
            ("install.log", install.stdout),
        ] {
            fs::write(out.join(name), bytes).unwrap();
        }
        fs::write(
            out.join("package-lock.json"),
            fs::read(root.join("package-lock.json")).unwrap(),
        )
        .unwrap();
        fs::write(out.join("summary.json"),serde_json::to_vec_pretty(&json!({"status":"local scope satisfied","trust":"LocalUntrusted","node":"26.12.0","npm":"11.20.0","install":"temporary fixture only","audits_per_graph":1,"phases":["baseline","member-only vulnerability","root-only vulnerability","recovery"],"native_resolution":{"root_typescript":"6.0.3","member_typescript":"5.9.3"},"installed_manifest_swap":"rejected"})).unwrap()).unwrap();
    }
}

#[test]
#[ignore = "requires Node 26.12.0/npm 11.20.0 and temporary registry install/audit access"]
fn native_npm_overrides_audit_resolved_versions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    assert_eq!(
        String::from_utf8(checked(root, "node", &["--version"]).stdout)
            .unwrap()
            .trim(),
        "v26.12.0"
    );
    assert_eq!(
        String::from_utf8(checked(root, "npm", &["--version"]).stdout)
            .unwrap()
            .trim(),
        "11.20.0"
    );
    let mut request = workspace::fixture(root);
    fs::remove_dir_all(root.join("node_modules")).unwrap();
    fs::remove_dir_all(root.join("tools/openapi/node_modules")).unwrap();
    fs::remove_file(root.join("package-lock.json")).unwrap();
    let member_path = root.join("tools/openapi/package.json");
    let mut member: Value = serde_json::from_slice(&fs::read(&member_path).unwrap()).unwrap();
    member["dependencies"] = json!({"mkdirp":"0.5.0"});
    fs::write(&member_path, serde_json::to_vec_pretty(&member).unwrap()).unwrap();
    let install_args = [
        "install",
        "--ignore-scripts",
        "--no-audit",
        "--no-fund",
        "--include=dev",
        "--include=optional",
        "--include=peer",
        "--workspaces",
        "--include-workspace-root",
        "--userconfig=.policy-npm-userconfig",
        "--globalconfig=.policy-npm-globalconfig",
    ];
    let audit_args = [
        "audit",
        "--audit-level=low",
        "--include=dev",
        "--include=optional",
        "--include=peer",
        "--workspaces",
        "--include-workspace-root",
        "--userconfig=.policy-npm-userconfig",
        "--globalconfig=.policy-npm-globalconfig",
        "--json",
    ];
    let output_dir = std::env::var_os("POLICY_NPM_OVERRIDES_OUTPUT").map(std::path::PathBuf::from);
    if let Some(out) = &output_dir {
        fs::create_dir(out).unwrap();
    }
    for (phase, overrides, version, expected_exit) in [
        ("vulnerable", json!({"minimist":"0.0.8"}), "0.0.8", 1),
        ("fixed", json!({"mkdirp":{"minimist":"1.2.8"}}), "1.2.8", 0),
    ] {
        let path = root.join("package.json");
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        manifest["overrides"] = overrides.clone();
        fs::write(path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
        request.profile.selection.as_mut().unwrap().graphs[0]
            .workspace
            .as_mut()
            .unwrap()
            .overrides = serde_json::from_value(overrides).unwrap();
        request.expected_profile_digest = policy_core::canonical_sha256(&request.profile).unwrap();
        // Regenerate every phase so npm must actually apply the changed override.
        if root.join("package-lock.json").exists() {
            fs::remove_file(root.join("package-lock.json")).unwrap();
        }
        if root.join("node_modules").exists() {
            fs::remove_dir_all(root.join("node_modules")).unwrap();
        }
        if root.join("tools/openapi/node_modules").exists() {
            fs::remove_dir_all(root.join("tools/openapi/node_modules")).unwrap();
        }
        checked(root, "npm", &install_args);
        let lock_bytes = fs::read(root.join("package-lock.json")).unwrap();
        let lock: Value = serde_json::from_slice(&lock_bytes).unwrap();
        let copies: Vec<_> = lock["packages"]
            .as_object()
            .unwrap()
            .iter()
            .filter(|(p, _)| p.ends_with("node_modules/minimist"))
            .collect();
        assert!(!copies.is_empty());
        assert!(copies.iter().all(|(_, v)| v["version"] == version));
        project_verification::run::prepare(&request).unwrap();
        let native = checked(root, "node", &["-p", "require(require.resolve('minimist/package.json', {paths: [require.resolve('mkdirp/package.json')]})).version"]);
        assert_eq!(String::from_utf8(native.stdout).unwrap().trim(), version);
        let audit = command(root, "npm", &audit_args);
        assert_eq!(
            audit.status.code(),
            Some(expected_exit),
            "{}\n{}",
            String::from_utf8_lossy(&audit.stdout),
            String::from_utf8_lossy(&audit.stderr)
        );
        let report: Value = serde_json::from_slice(&audit.stdout).unwrap();
        if expected_exit == 1 {
            let finding = &report["vulnerabilities"]["minimist"];
            assert!(finding.is_object());
            assert!(copies.iter().all(|(p, _)| finding["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n.as_str() == Some(p))));
        } else {
            assert_eq!(report["metadata"]["vulnerabilities"]["total"], 0);
        }
        if let Some(out) = &output_dir {
            fs::write(out.join(format!("{phase}-audit.json")), &audit.stdout).unwrap();
            fs::write(out.join(format!("{phase}-package-lock.json")), lock_bytes).unwrap();
            fs::write(
                out.join(format!("{phase}-package.json")),
                fs::read(root.join("package.json")).unwrap(),
            )
            .unwrap();
        }
    }
    if let Some(out) = output_dir {
        fs::write(out.join("summary.json"), serde_json::to_vec_pretty(&json!({"status":"local scope satisfied","trust":"LocalUntrusted","node":"26.12.0","npm":"11.20.0","audits_per_phase":1,"vulnerable_override":"minimist 0.0.8: audit exit 1","fixed_scoped_override":"mkdirp > minimist 1.2.8: audit exit 0","installation":"temporary fixture only"})).unwrap()).unwrap();
    }
}
