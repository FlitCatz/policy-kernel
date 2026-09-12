use assert_cmd::cargo::cargo_bin;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use tempfile::TempDir;

fn run(args: &[&str]) -> Output {
    Command::new(cargo_bin!("xtask"))
        .args(args)
        .output()
        .unwrap()
}
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}
fn write(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
fn failure(output: &Output) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}
const FILES: [&str; 5] = [
    "canonical",
    "provenance",
    "reviews",
    "pre_merge",
    "post_merge",
];
fn results() -> Value {
    let mut outcomes = vec![json!({"scenario":"Baseline","summary":"Success"})];
    for file in FILES {
        outcomes.push(json!({"scenario":{"Mutant":{
            "name": format!("mutate {file}"), "package":"policy-core",
            "file":format!("crates/policy-core/src/{file}.rs"),
            "function":null,"span":{},"replacement":"false","genre":"FnValue"
        }},"summary":"CaughtMutant"}));
    }
    json!({"cargo_mutants_version":"27.1.0","end_time":"2026-09-08T00:00:00Z",
        "total_mutants":5,"caught":5,"missed":0,"timeout":0,"unviable":0,"success":0,"outcomes":outcomes})
}
fn mutation(payload: &Value, inventory: Option<Value>) -> Output {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("outcomes.json");
    write(&path, payload);
    if let Some(inventory) = inventory {
        write(&temp.path().join("mutants.json"), &inventory);
    }
    run(&["check-mutation-results", path.to_str().unwrap()])
}
fn inventory(payload: &Value) -> Value {
    Value::Array(
        payload["outcomes"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .map(|o| o["scenario"]["Mutant"].clone())
            .collect(),
    )
}

#[test]
fn mutation_accepts_complete_inventory() {
    let p = results();
    let out = mutation(&p, Some(inventory(&p)));
    success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("5 mutants across 5"));
}
#[test]
fn mutation_rejects_incomplete_or_bad_results() {
    let valid = results();
    for (pointer, value) in [
        ("/cargo_mutants_version", json!("27.0.0")),
        ("/end_time", json!(null)),
        ("/total_mutants", json!(0)),
        ("/total_mutants", json!(6)),
        ("/caught", json!(true)),
        ("/caught", json!(-1)),
        ("/outcomes/0/summary", json!("Failure")),
        ("/outcomes/1/summary", json!("Unknown")),
        (
            "/outcomes/1/scenario/Mutant/file",
            json!("crates/policy-core/src/lib.rs"),
        ),
        ("/outcomes/1/scenario/Mutant/package", json!("other")),
    ] {
        let mut p = valid.clone();
        *p.pointer_mut(pointer).unwrap() = value;
        failure(&mutation(&p, Some(inventory(&p))));
    }
    for (counter, summary) in [
        ("missed", "MissedMutant"),
        ("timeout", "Timeout"),
        ("success", "Success"),
    ] {
        let mut p = valid.clone();
        p[counter] = json!(1);
        p["caught"] = json!(4);
        p["outcomes"][1]["summary"] = json!(summary);
        failure(&mutation(&p, Some(inventory(&p))));
    }
    let mut missing = valid.clone();
    missing["outcomes"].as_array_mut().unwrap().pop();
    missing["total_mutants"] = json!(4);
    missing["caught"] = json!(4);
    failure(&mutation(&missing, Some(inventory(&missing))));
}
#[test]
fn mutation_requires_exact_unique_generated_inventory() {
    let p = results();
    failure(&mutation(&p, None));
    let mut inv = inventory(&p);
    let mut extra = inv[0].clone();
    extra["name"] = json!("extra");
    inv.as_array_mut().unwrap().push(extra);
    failure(&mutation(&p, Some(inv)));
    let mut duplicate = p.clone();
    let entry = duplicate["outcomes"][1].clone();
    duplicate["outcomes"].as_array_mut().unwrap().push(entry);
    duplicate["caught"] = json!(6);
    duplicate["total_mutants"] = json!(6);
    failure(&mutation(&duplicate, Some(inventory(&duplicate))));
    let mut inv = inventory(&p);
    inv[0].as_object_mut().unwrap().remove("span");
    failure(&mutation(&p, Some(inv)));
}
#[test]
fn mutation_rejects_unreadable_and_malformed_json() {
    failure(&run(&[
        "check-mutation-results",
        "/definitely/missing/outcomes.json",
    ]));
    let t = TempDir::new().unwrap();
    let p = t.path().join("outcomes.json");
    fs::write(&p, "{not-json").unwrap();
    failure(&run(&["check-mutation-results", p.to_str().unwrap()]));
}
fn rollback(change: impl FnOnce(&mut Vec<Value>)) -> Output {
    let t = TempDir::new().unwrap();
    let names = [
        "baseline",
        "promoted",
        "restored",
        "evidence-before",
        "evidence-after",
    ];
    let mut values: Vec<Value> = names
        .iter()
        .map(|n| {
            serde_json::from_slice(
                &fs::read(root().join(format!("tests/fixtures/rollback/{n}.json"))).unwrap(),
            )
            .unwrap()
        })
        .collect();
    change(&mut values);
    let mut args = vec!["rollback-drill".to_owned()];
    let mut before = Vec::new();
    for (name, value) in names.iter().zip(values) {
        let path = t.path().join(format!("{name}.json"));
        write(&path, &value);
        before.push((path.clone(), fs::read(&path).unwrap()));
        args.push(format!("--{name}"));
        args.push(path.to_str().unwrap().to_owned());
    }
    let out = run(&args.iter().map(String::as_str).collect::<Vec<_>>());
    for (path, bytes) in before {
        assert_eq!(bytes, fs::read(path).unwrap());
    }
    out
}
#[test]
fn rollback_preserves_evidence_and_reports_fixture_scope() {
    let out = rollback(|_| {});
    success(&out);
    let value: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["scope"], "supplied-snapshots-only");
    assert_eq!(value["live_protection_verified"], false);
}
#[test]
fn rollback_rejects_weakened_or_unbound_protection_and_modified_evidence() {
    for (index, pointer, value) in [
        (2, "/required_approving_review_count", json!(0)),
        (2, "/allow_force_pushes", json!(true)),
        (2, "/allow_deletions", json!(true)),
        (2, "/dismiss_stale_reviews", json!(false)),
        (2, "/required_checks", json!([])),
        (0, "/repository", json!("other/repo")),
        (4, "/artifacts/0/sha256", json!("b".repeat(64))),
        (0, "/schema_version", json!(true)),
        (0, "/required_checks/0/app_id", json!(0)),
    ] {
        let out = rollback(|v| *v[index].pointer_mut(pointer).unwrap() = value);
        assert_eq!(out.status.code(), Some(64));
        failure(&out);
    }
    failure(&rollback(|v| {
        v[3]["artifacts"] = json!([]);
        v[4]["artifacts"] = json!([]);
    }));
    failure(&rollback(|v| v[2] = v[1].clone()));
    failure(&rollback(|v| v[0]["unexpected"] = json!(true)));
}

#[test]
fn rollback_rejects_duplicate_json_fields_and_duplicate_identities() {
    let temp = TempDir::new().unwrap();
    let names = [
        "baseline",
        "promoted",
        "restored",
        "evidence-before",
        "evidence-after",
    ];
    let mut args = vec!["rollback-drill".to_owned()];
    for name in names {
        let mut raw =
            fs::read_to_string(root().join(format!("tests/fixtures/rollback/{name}.json")))
                .unwrap();
        if name == "baseline" {
            raw = raw.replacen(
                "\"schema_version\": 1",
                "\"schema_version\": 99, \"schema_version\": 1",
                1,
            );
        }
        let path = temp.path().join(format!("{name}.json"));
        fs::write(&path, raw).unwrap();
        args.extend([format!("--{name}"), path.to_str().unwrap().to_owned()]);
    }
    let out = run(&args.iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(out.status.code(), Some(64));
    failure(&out);
    failure(&rollback(|v| {
        let check = v[0]["required_checks"][0].clone();
        v[0]["required_checks"].as_array_mut().unwrap().push(check);
    }));
    failure(&rollback(|v| {
        for index in [3, 4] {
            let artifact = v[index]["artifacts"][0].clone();
            v[index]["artifacts"].as_array_mut().unwrap().push(artifact);
        }
    }));
}
fn scan_repo() -> TempDir {
    let t = TempDir::new().unwrap();
    let p = t.path();
    for dir in [
        "crates/policy-core/src",
        "schemas",
        "action",
        "doc/policy_core",
        "doc/policy_contracts",
    ] {
        fs::create_dir_all(p.join(dir)).unwrap();
    }
    for (name, text) in [
        (
            "Cargo.toml",
            "[workspace]\nmembers=[\"crates/policy-core\"]\nresolver=\"2\"\n",
        ),
        (
            "Cargo.lock",
            "version=4\n[[package]]\nname=\"policy-core\"\nversion=\"0.1.0\"\n",
        ),
        (
            "crates/policy-core/Cargo.toml",
            "[package]\nname=\"policy-core\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        ),
        (
            "crates/policy-core/src/lib.rs",
            "pub enum Knowledge { LatClaim, CodeGraphIndex }\n",
        ),
        ("schemas/policy.json", "{}\n"),
        ("action/action.yml", "name: gate\n"),
        ("doc/policy_core/index.html", "kernel API"),
        ("doc/policy_contracts/index.html", "LatClaim"),
    ] {
        fs::write(p.join(name), text).unwrap();
    }
    t
}
fn residue(path: &Path) -> Output {
    run(&["check-residue", path.to_str().unwrap()])
}
fn public_api(path: &Path) -> Output {
    run(&[
        "check-public-api",
        path.join("doc").to_str().unwrap(),
        path.join("schemas").to_str().unwrap(),
    ])
}
#[test]
fn clean_runtime_and_public_api_pass() {
    let t = scan_repo();
    success(&residue(t.path()));
    success(&public_api(t.path()));
}
#[test]
fn scans_include_hidden_and_ignored_files() {
    for path in [
        "crates/policy-core/src/.hidden.rs",
        "crates/policy-core/src/legacy.rs",
        "action/legacy.txt",
        "schemas/.hidden.json",
    ] {
        let t = scan_repo();
        fs::write(
            t.path().join("crates/policy-core/src/.ignore"),
            "legacy.rs\n",
        )
        .unwrap();
        fs::write(t.path().join(path), "gsd-core").unwrap();
        failure(&residue(t.path()));
    }
    for path in ["doc/policy_core/.hidden.html", "schemas/policy.json"] {
        let t = scan_repo();
        fs::write(t.path().join(path), "hashline runtime").unwrap();
        failure(&public_api(t.path()));
    }
}

#[test]
fn scans_cover_installer_acceptance_build_hooks_manifests_and_ci() {
    for (path, text) in [
        ("bootstrap/src/main.rs", "superpowers::initialize();"),
        ("acceptance/src/process.rs", "agent_skills::run();"),
        (
            "bootstrap/templates/next/verification.yml",
            "run: gsd-core verify",
        ),
        (
            "bootstrap/Cargo.toml",
            "[dependencies]\nsuperpowers = \"1\"",
        ),
        (
            "acceptance/build.rs",
            "fn main() { agent_skills::install(); }",
        ),
        (
            "crates/policy-core/Cargo.toml",
            "[dependencies]\nagent-skills = \"1\"",
        ),
        (
            "crates/policy-core/build.rs",
            "fn main() { gsd_core::install(); }",
        ),
        (
            "Cargo.lock",
            "version=4\n[[package]]\nname=\"superpowers\"\nversion=\"1.0.0\"",
        ),
        (".github/workflows/ci.yml", "run: superpowers verify"),
        ("xtask/src/helper.rs", "gsd_core::run();"),
    ] {
        let t = scan_repo();
        let path = t.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
        let out = residue(t.path());
        failure(&out);
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("forbidden"),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

#[test]
fn historical_docs_and_negative_test_fixtures_are_not_runtime_dependencies() {
    let t = scan_repo();
    for path in [
        "docs/archive/old-plan.md",
        "bootstrap/tests/negative.rs",
        "acceptance/tests/negative.rs",
        "xtask/tests/negative.rs",
        "bootstrap/templates/next/README.md",
    ] {
        let path = t.path().join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "superpowers agent-skills gsd-core").unwrap();
    }
    success(&residue(t.path()));
}

#[test]
fn declared_custom_cargo_build_hooks_are_runtime_inputs() {
    let t = scan_repo();
    fs::write(
        t.path().join("crates/policy-core/Cargo.toml"),
        "[package]\nname=\"policy-core\"\nversion=\"0.1.0\"\nedition=\"2021\"\nbuild=\"hooks/install.rs\"\n",
    ).unwrap();
    let hooks = t.path().join("crates/policy-core/hooks");
    fs::create_dir(&hooks).unwrap();
    fs::write(
        hooks.join("install.rs"),
        "fn main() { superpowers::install(); }",
    )
    .unwrap();
    let out = residue(t.path());
    failure(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("forbidden"));
}

#[test]
fn cargo_workspace_members_outside_conventional_directories_are_scanned() {
    let t = scan_repo();
    fs::write(
        t.path().join("Cargo.toml"),
        "[workspace]\nmembers=[\"crates/policy-core\",\"tools/helper\"]\nresolver=\"2\"\n",
    )
    .unwrap();
    fs::write(
        t.path().join("Cargo.lock"),
        "version=4\n[[package]]\nname=\"policy-core\"\nversion=\"0.1.0\"\n[[package]]\nname=\"helper\"\nversion=\"0.1.0\"\n",
    ).unwrap();
    let helper = t.path().join("tools/helper");
    fs::create_dir_all(helper.join("src")).unwrap();
    fs::write(
        helper.join("Cargo.toml"),
        "[package]\nname=\"helper\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    )
    .unwrap();
    fs::write(helper.join("src/main.rs"), "fn main() {}").unwrap();
    success(&residue(t.path()));
    fs::write(
        helper.join("src/main.rs"),
        "fn main() { agent_skills::run(); }",
    )
    .unwrap();
    let out = residue(t.path());
    failure(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("forbidden"));
}

#[test]
fn root_level_custom_targets_include_sibling_rust_modules_not_tests() {
    let t = scan_repo();
    let package = t.path().join("crates/policy-core");
    fs::write(package.join("Cargo.toml"), "[package]\nname=\"policy-core\"\nversion=\"0.1.0\"\nedition=\"2021\"\nbuild=\"custom_build.rs\"\n").unwrap();
    fs::write(
        package.join("custom_build.rs"),
        "mod helper; fn main() { helper::install(); }",
    )
    .unwrap();
    fs::create_dir(package.join("tests")).unwrap();
    fs::write(
        package.join("tests/negative.rs"),
        "// superpowers agent-skills gsd-core",
    )
    .unwrap();
    fs::write(package.join("helper.rs"), "pub fn install() {}").unwrap();
    success(&residue(t.path()));
    fs::write(
        package.join("helper.rs"),
        "pub fn install() { gsd_core::install(); }",
    )
    .unwrap();
    let out = residue(t.path());
    failure(&out);
    assert!(String::from_utf8_lossy(&out.stderr).contains("forbidden"));
}
#[test]
fn scans_fail_closed_on_missing_empty_or_invalid_inputs() {
    for path in ["action/action.yml", "schemas/policy.json", "Cargo.toml"] {
        let t = scan_repo();
        fs::remove_file(t.path().join(path)).unwrap();
        failure(&residue(t.path()));
    }
    for path in [
        "doc/policy_core/index.html",
        "doc/policy_contracts/index.html",
        "schemas/policy.json",
    ] {
        let t = scan_repo();
        fs::write(t.path().join(path), "").unwrap();
        failure(&public_api(t.path()));
    }
    let t = scan_repo();
    fs::write(t.path().join("Cargo.toml"), "not toml").unwrap();
    failure(&residue(t.path()));
}
