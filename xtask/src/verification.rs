use policy_contracts::{RepositoryId, Sha256Digest};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

type Result<T> = std::result::Result<T, String>;
fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}
fn load<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read(path)?)
        .map_err(|e| format!("invalid JSON at {}: {e}", path.display()))
}
const FILES: [&str; 5] = [
    "crates/policy-core/src/canonical.rs",
    "crates/policy-core/src/provenance.rs",
    "crates/policy-core/src/reviews.rs",
    "crates/policy-core/src/pre_merge.rs",
    "crates/policy-core/src/post_merge.rs",
];
fn count(value: &Value, name: &str) -> Result<u64> {
    value
        .get(name)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{name} must be a non-negative integer"))
}
pub(crate) fn identity(value: &Value) -> Result<String> {
    if value
        .get("name")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("mutant name must be nonempty".into());
    }
    let mut fields = BTreeMap::new();
    for key in [
        "name",
        "package",
        "file",
        "function",
        "span",
        "replacement",
        "genre",
    ] {
        fields.insert(
            key,
            value
                .get(key)
                .ok_or_else(|| format!("incomplete mutant identity: {key}"))?,
        );
    }
    serde_json::to_string(&fields).map_err(|e| e.to_string())
}
pub fn mutation(path: &Path) -> Result<()> {
    mutation_scope(path, "core")
}

pub fn mutation_scope(path: &Path, scope: &str) -> Result<()> {
    let (package, required_files, function): (&str, &[&str], Option<&str>) = match scope {
        "core" => ("policy-core", &FILES, None),
        "controller" => ("policy-ci", &["crates/policy-ci/src/controller.rs"], None),
        "workflow-source" => (
            "policy-ci",
            &["crates/policy-ci/src/main.rs"],
            Some("workflow_source_matches"),
        ),
        _ => return Err("unsupported mutation scope".into()),
    };
    let p: Value = load(path)?;
    if p["cargo_mutants_version"] != "27.1.0" {
        return Err("unexpected cargo-mutants version; expected 27.1.0".into());
    }
    if p["end_time"].as_str().is_none_or(str::is_empty) {
        return Err("incomplete mutation outcomes: missing end_time".into());
    }
    let total = count(&p, "total_mutants")?;
    if total == 0 {
        return Err("no mutants were exercised".into());
    }
    let names = ["missed", "caught", "timeout", "unviable", "success"];
    let declared: Vec<u64> = names.iter().map(|n| count(&p, n)).collect::<Result<_>>()?;
    let outcomes = p["outcomes"]
        .as_array()
        .ok_or("outcomes must be an array")?;
    let mut baseline = Vec::new();
    let mut observed_counts = [0_u64; 5];
    let mut observed = BTreeSet::new();
    let mut files = BTreeSet::new();
    for outcome in outcomes {
        let summary = outcome["summary"]
            .as_str()
            .ok_or("outcome summary must be a string")?;
        let scenario = &outcome["scenario"];
        if scenario == "Baseline" {
            baseline.push(summary);
            continue;
        }
        let scenario = scenario
            .as_object()
            .ok_or("scenario must be Baseline or Mutant")?;
        if scenario.len() != 1 || !scenario.contains_key("Mutant") {
            return Err("invalid mutation scenario".into());
        }
        let mutant = &scenario["Mutant"];
        if mutant["package"] != package {
            return Err(format!("every mutation must belong to {package}"));
        }
        if function.is_some_and(|name| mutant["function"]["function_name"] != name) {
            return Err("unexpected mutated function for selected scope".into());
        }
        let file = mutant["file"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("missing mutant source file")?
            .replace('\\', "/");
        files.insert(file.strip_prefix("./").unwrap_or(&file).to_owned());
        if !observed.insert(identity(mutant)?) {
            return Err("duplicate mutation identity".into());
        }
        let index = match summary {
            "MissedMutant" => 0,
            "CaughtMutant" => 1,
            "Timeout" => 2,
            "Unviable" => 3,
            "Success" => 4,
            _ => return Err("unclassified mutation outcome".into()),
        };
        observed_counts[index] += 1;
    }
    if baseline != ["Success"] {
        return Err("baseline did not succeed exactly once".into());
    }
    if observed.len() as u64 != total
        || declared != observed_counts
        || declared.iter().try_fold(0_u64, |a, b| a.checked_add(*b)) != Some(total)
    {
        return Err("counter mismatch in mutation outcomes".into());
    }
    if declared[0] != 0 {
        return Err("missed mutants found".into());
    }
    if declared[2] != 0 {
        return Err("timed-out mutants found".into());
    }
    if declared[3] != 0 {
        return Err("unviable mutants found".into());
    }
    if declared[4] != 0 || declared[1] == 0 {
        return Err("no caught viable mutations or unclassified successes".into());
    }
    let required: BTreeSet<String> = required_files
        .iter()
        .map(|file| (*file).to_owned())
        .collect();
    if !files.is_subset(&required) {
        return Err("unexpected mutated files".into());
    }
    if files != required {
        return Err("missing mutation coverage".into());
    }
    let inventory: Vec<Value> = load(&path.with_file_name("mutants.json"))?;
    let mut expected = BTreeSet::new();
    for mutant in inventory {
        if !expected.insert(identity(&mutant)?) {
            return Err("duplicate mutation inventory identity".into());
        }
    }
    if expected != observed {
        return Err("incomplete mutation coverage: inventory differs from outcomes".into());
    }
    println!("mutation promotion gate passed: {total} mutants across {} authorization-critical files; {} caught, {} unviable",files.len(),declared[1],declared[3]);
    Ok(())
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Check {
    context: String,
    app_id: u64,
}
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Protection {
    schema_version: u32,
    repository: RepositoryId,
    branch: String,
    required_checks: Vec<Check>,
    required_approving_review_count: u64,
    dismiss_stale_reviews: bool,
    require_code_owner_reviews: bool,
    enforce_admins: bool,
    allow_force_pushes: bool,
    allow_deletions: bool,
}
fn protection(path: &Path) -> Result<Protection> {
    let mut p: Protection = load(path)?;
    if p.schema_version != 1
        || p.branch.trim().is_empty()
        || p.required_approving_review_count == 0
        || p.allow_force_pushes
        || p.allow_deletions
        || p.required_checks.is_empty()
    {
        return Err("invalid or weakened native protection snapshot".into());
    }
    p.required_checks.sort();
    if p.required_checks
        .iter()
        .any(|c| c.context.trim().is_empty() || c.app_id == 0)
        || p.required_checks.windows(2).any(|w| w[0] == w[1])
    {
        return Err("invalid or duplicate required-check identity".into());
    }
    Ok(p)
}
#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
struct Artifact {
    path: String,
    sha256: Sha256Digest,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence {
    schema_version: u32,
    artifacts: Vec<Artifact>,
}
fn evidence(path: &Path) -> Result<Vec<Artifact>> {
    let mut p: Evidence = load(path)?;
    if p.schema_version != 1 || p.artifacts.is_empty() {
        return Err("invalid or empty evidence manifest".into());
    }
    p.artifacts.sort();
    if p.artifacts.iter().any(|a| a.path.is_empty())
        || p.artifacts.windows(2).any(|w| w[0].path == w[1].path)
    {
        return Err("empty or duplicate artifact paths".into());
    }
    Ok(p.artifacts)
}
pub fn rollback(args: &[String]) -> Result<()> {
    let required = [
        "--baseline",
        "--promoted",
        "--restored",
        "--evidence-before",
        "--evidence-after",
    ];
    let mut paths = BTreeMap::new();
    for pair in args.chunks(2) {
        if pair.len() != 2
            || !required.contains(&pair[0].as_str())
            || paths
                .insert(pair[0].as_str(), Path::new(&pair[1]))
                .is_some()
        {
            return Err("invalid or duplicate rollback option".into());
        }
    }
    if paths.len() != 5 {
        return Err("all five rollback snapshot paths are required".into());
    }
    let baseline = protection(paths["--baseline"])?;
    let mut promoted = protection(paths["--promoted"])?;
    let restored = protection(paths["--restored"])?;
    const KERNEL: &str = "policy-kernel/pre-merge";
    if baseline.required_checks.iter().any(|c| c.context == KERNEL)
        || promoted
            .required_checks
            .iter()
            .filter(|c| c.context == KERNEL)
            .count()
            != 1
    {
        return Err("native baseline or promoted kernel check is invalid".into());
    }
    promoted.required_checks.retain(|c| c.context != KERNEL);
    if promoted != baseline || restored != baseline {
        return Err("native protection baseline was not preserved and restored exactly".into());
    }
    if evidence(paths["--evidence-before"])? != evidence(paths["--evidence-after"])? {
        return Err("rollback changed evidence inventory".into());
    }
    println!(
        "{}",
        json!({"schema_version":1,"status":"passed","scope":"supplied-snapshots-only","live_protection_verified":false})
    );
    Ok(())
}

fn nonempty(path: &Path) -> Result<()> {
    if !fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0) {
        return Err(format!(
            "required file is missing or empty: {}",
            path.display()
        ));
    }
    Ok(())
}
fn schemas(path: &Path) -> Result<()> {
    let entries = fs::read_dir(path).map_err(|e| format!("cannot read schema directory: {e}"))?;
    for entry in entries {
        let p = entry.map_err(|e| e.to_string())?.path();
        if p.extension().is_some_and(|e| e == "json") && nonempty(&p).is_ok() {
            return Ok(());
        }
    }
    Err("required schema files are missing or empty".into())
}
fn forbidden(text: &str) -> bool {
    let lower = text.to_lowercase().replace('_', "-");
    [
        "superpowers",
        "agent-skills",
        "gsd-core",
        ".planning",
        "gemini-reviewer",
        "oh-my-pi",
    ]
    .iter()
    .any(|s| lower.contains(s))
        || lower
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|word| ["agy", "omp", "mcp", "hashline"].contains(&word))
}
fn scan(path: &Path) -> Result<()> {
    scan_runtime(path, false)
}
fn scan_runtime(path: &Path, skip_docs: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|e| format!("cannot inspect {}: {e}", path.display()))?;
    if metadata.file_type().is_symlink() {
        return Err(format!("scan refuses symlink: {}", path.display()));
    }
    if metadata.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<_>>()
            .map_err(|e| e.to_string())?;
        entries.sort();
        for entry in entries {
            scan_runtime(&entry, skip_docs)?;
        }
    } else if metadata.is_file() {
        if skip_docs && path.extension().is_some_and(|e| e == "md") {
            return Ok(());
        }
        if forbidden(&String::from_utf8_lossy(&read(path)?)) {
            return Err(format!(
                "forbidden runtime/public API residue found: {}",
                path.display()
            ));
        }
    } else {
        return Err(format!("unsupported scan entry: {}", path.display()));
    }
    Ok(())
}
fn check_dependencies(text: &str) -> Result<()> {
    if text
        .split_whitespace()
        .any(|s| ["reqwest", "octocrab", "clap", "tokio"].contains(&s))
    {
        return Err("forbidden policy-core dependency found".into());
    }
    Ok(())
}
pub fn residue(root: &Path) -> Result<()> {
    nonempty(&root.join("Cargo.toml"))?;
    nonempty(&root.join("Cargo.lock"))?;
    schemas(&root.join("schemas"))?;
    if nonempty(&root.join("action/action.yml")).is_err() {
        nonempty(&root.join("action/action.yaml"))?;
    }
    let mut sources = Vec::new();
    for entry in fs::read_dir(root.join("crates")).map_err(|e| e.to_string())? {
        let package = entry.map_err(|e| e.to_string())?.path();
        let path = package.join("src");
        if path.is_dir() {
            sources.push(path);
            sources.push(package.join("Cargo.toml"));
            scan_optional(&package.join("build.rs"), false)?;
        }
    }
    if sources.is_empty() {
        return Err("required crate source directories are missing".into());
    }
    sources.extend([
        root.join("schemas"),
        root.join("action"),
        root.join("Cargo.toml"),
        root.join("Cargo.lock"),
    ]);
    for path in sources {
        scan(&path)?;
    }
    for package in ["bootstrap", "acceptance", "xtask"] {
        let path = root.join(package);
        scan_optional(&path.join("Cargo.toml"), false)?;
        scan_optional(&path.join("build.rs"), false)?;
        if package != "xtask" {
            scan_optional(&path.join("src"), false)?;
        } else if path.join("src").is_dir() {
            // This scanner defines the deny-list itself; other verification
            // implementations are product code and must still be checked.
            for entry in fs::read_dir(path.join("src")).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?.path();
                if entry.file_name().is_some_and(|n| n != "verification.rs") {
                    scan(&entry)?;
                }
            }
        }
    }
    for path in [".github/workflows", ".cargo", "scripts", "build.rs"] {
        scan_optional(&root.join(path), true)?;
    }
    // Templates can contain executable/configuration assets, but their prose
    // and historical examples do not constitute a runtime dependency.
    scan_optional(&root.join("bootstrap/templates"), true)?;
    scan_cargo_targets(root)?;
    let output = Command::new("cargo")
        .args(["tree", "--locked", "-p", "policy-core"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo tree: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    check_dependencies(&String::from_utf8_lossy(&output.stdout))?;
    println!("runtime residue scan passed");
    Ok(())
}

fn scan_optional(path: &Path, skip_docs: bool) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => scan_runtime(path, skip_docs),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("cannot inspect {}: {e}", path.display())),
    }
}

fn scan_cargo_targets(root: &Path) -> Result<()> {
    // Cargo metadata is read-only: it discovers custom build/bin/lib paths and
    // additional workspace members without executing their build scripts.
    let output = Command::new("cargo")
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let members = metadata["workspace_members"]
        .as_array()
        .ok_or("missing workspace members")?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("missing workspace packages")?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    for package in packages.iter().filter(|p| members.contains(&p["id"])) {
        let manifest = Path::new(
            package["manifest_path"]
                .as_str()
                .ok_or("missing manifest path")?,
        );
        scan_workspace_path(&root, manifest)?;
        let directory = manifest.parent().ok_or("missing package directory")?;
        for target in package["targets"]
            .as_array()
            .ok_or("missing Cargo targets")?
        {
            let kinds = target["kind"].as_array().ok_or("missing target kind")?;
            if kinds
                .iter()
                .any(|k| ["test", "example", "bench"].contains(&k.as_str().unwrap_or("")))
            {
                continue;
            }
            let source = Path::new(target["src_path"].as_str().ok_or("missing target source")?);
            let parent = source.parent().ok_or("missing target directory")?;
            // Root-level targets can import sibling Rust modules too. Inspect
            // package Rust sources while excluding conventional test/artifact
            // directories; don't treat historical prose as a build dependency.
            if parent == directory {
                scan_package_rust(&root, directory)?;
            } else if parent == root.join("xtask/src") {
                scan_workspace_path(&root, source)?;
            } else {
                scan_workspace_path(&root, parent)?;
            }
        }
    }
    Ok(())
}

fn scan_workspace_path(root: &Path, path: &Path) -> Result<()> {
    if !path
        .canonicalize()
        .map_err(|e| e.to_string())?
        .starts_with(root)
    {
        return Err(format!(
            "runtime source is outside scanned workspace: {}",
            path.display()
        ));
    }
    scan_runtime(path, true)
}

fn scan_package_rust(root: &Path, path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if metadata.file_type().is_symlink() {
        return Err(format!("scan refuses symlink: {}", path.display()));
    }
    if metadata.is_dir() {
        let mut entries = fs::read_dir(path)
            .map_err(|e| e.to_string())?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        entries.sort();
        for entry in entries {
            let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if [
                "tests",
                "benches",
                "examples",
                "docs",
                "target",
                "node_modules",
                ".git",
                ".worktrees",
                ".codegraph",
                ".superpowers",
            ]
            .contains(&name)
            {
                continue;
            }
            scan_package_rust(root, &entry)?;
        }
    } else if path.extension().is_some_and(|e| e == "rs") {
        scan_workspace_path(root, path)?;
    }
    Ok(())
}
pub fn public_api(docs: &Path, schema_dir: &Path) -> Result<()> {
    for name in ["policy_core", "policy_contracts"] {
        let path = docs.join(name);
        nonempty(&path.join("index.html"))?;
        scan(&path)?;
    }
    schemas(schema_dir)?;
    scan(schema_dir)?;
    println!("public API and schema residue scan passed");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forbidden_dependencies_are_rejected() {
        for name in ["reqwest", "octocrab", "clap", "tokio"] {
            assert!(check_dependencies(&format!("policy-core v0.1.0\n└── {name} v1.0.0")).is_err());
        }
        assert!(check_dependencies("policy-core v0.1.0\n└── serde v1.0.0").is_ok());
    }
}
