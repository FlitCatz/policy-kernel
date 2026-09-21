use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
type Result<T> = std::result::Result<T, String>;
static INTERRUPTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
const CONTRACT: &str = include_str!("../../verification/mutation-contract.json");

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}
fn load(path: &Path) -> Result<Value> {
    serde_json::from_slice(&read(path)?).map_err(|e| e.to_string())
}
fn write(path: &Path, value: &Value) -> Result<()> {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn set(values: &[Value]) -> Result<BTreeSet<String>> {
    let keys: BTreeSet<_> = values
        .iter()
        .map(crate::verification::identity)
        .collect::<Result<_>>()?;
    if keys.len() != values.len() {
        return Err("duplicate inventory identity".into());
    }
    Ok(keys)
}
fn array(value: &Value) -> Result<&Vec<Value>> {
    value.as_array().ok_or("expected inventory array".into())
}
fn select(root: &Path, generated: &[Value], scope: &str) -> Result<(Vec<Value>, Vec<Value>)> {
    let contract: Value = serde_json::from_str(CONTRACT).map_err(|e| e.to_string())?;
    for (file, expected) in contract["sources"]
        .as_object()
        .ok_or("missing source contracts")?
    {
        if expected != &hash(&read(&root.join(file))?) {
            return Err(format!(
                "stale mutation contract for {file}; review the source and repair"
            ));
        }
    }
    let mut remaining = set(generated)?;
    let mut repaired = Vec::new();
    for repair in array(&contract["repairs"])? {
        let original = &repair["original"];
        let belongs = match scope {
            "core" => original["package"] == "policy-core",
            "controller" => original["file"] == "crates/policy-ci/src/controller.rs",
            "workflow-source" => false,
            _ => return Err("unsupported mutation scope".into()),
        };
        if !belongs {
            continue;
        }
        if !remaining.remove(&crate::verification::identity(original)?) {
            return Err(format!(
                "contract original absent or changed: {}",
                original["name"]
            ));
        }
        repaired.push(repair.clone());
    }
    let mut selected = Vec::new();
    for mutant in generated {
        if remaining.contains(&crate::verification::identity(mutant)?) {
            selected.push(mutant.clone());
        }
    }
    Ok((selected, repaired))
}
fn args(scope: &str) -> Result<Vec<String>> {
    let (package, files, filter): (&str, &[&str], Option<&str>) = match scope {
        "core" => (
            "policy-core",
            &[
                "canonical",
                "provenance",
                "reviews",
                "pre_merge",
                "post_merge",
            ],
            None,
        ),
        "controller" => ("policy-ci", &["controller"], None),
        "workflow-source" => ("policy-ci", &["main"], Some("workflow_source_matches")),
        _ => return Err("unsupported mutation scope".into()),
    };
    let mut args = vec![
        "mutants".into(),
        "--no-config".into(),
        "-p".into(),
        package.into(),
    ];
    for file in files {
        args.extend(["--file".into(), format!("crates/{package}/src/{file}.rs")]);
    }
    if let Some(filter) = filter {
        args.extend(["--re".into(), filter.into()]);
    }
    Ok(args)
}
fn generated(root: &Path, scope: &str) -> Result<Value> {
    let output = Command::new("cargo")
        .args(args(scope)?)
        .args(["--list", "--json"])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}
fn escaped(name: &str) -> String {
    let mut result = String::from("^");
    for c in name.chars() {
        if "\\.^$|?*+()[]{}".contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result.push('$');
    result
}
fn offset(source: &str, point: &Value) -> Result<usize> {
    let line = point["line"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("invalid line")? as usize;
    let col = point["column"]
        .as_u64()
        .filter(|n| *n > 0)
        .ok_or("invalid column")? as usize;
    let text = source
        .split_inclusive('\n')
        .nth(line - 1)
        .ok_or("line out of bounds")?;
    let byte = text
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(text.len()))
        .nth(col - 1)
        .ok_or("column out of bounds")?;
    Ok(source
        .split_inclusive('\n')
        .take(line - 1)
        .map(str::len)
        .sum::<usize>()
        + byte)
}
fn repaired_source(source: &str, repair: &Value) -> Result<String> {
    let start = offset(source, &repair["span"]["start"])?;
    let end = offset(source, &repair["span"]["end"])?;
    if start >= end {
        return Err("invalid repair span".into());
    }
    let replacement = repair["replacement"]
        .as_str()
        .ok_or("missing repair replacement")?;
    if &source[start..end] == replacement {
        return Err("repair is a no-op".into());
    }
    Ok(format!(
        "{}{replacement}{}",
        &source[..start],
        &source[end..]
    ))
}
fn repaired_identity(repair: &Value) -> Value {
    let mut mutant = repair["original"].clone();
    mutant["name"] = json!(format!(
        "{} [typed replacement v1]",
        mutant["name"].as_str().unwrap()
    ));
    mutant["span"] = repair["span"].clone();
    mutant["replacement"] = repair["replacement"].clone();
    mutant
}
fn now() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string()
}

// Each child gets its own process group. A timeout kills the complete cargo/test
// group, never leaving a test running while the next source mutation is applied.
#[cfg(unix)]
fn execute(command: &mut Command, log: &Path, timeout: u64) -> Result<Value> {
    use std::os::unix::process::CommandExt;
    let output = fs::File::create(log).map_err(|e| e.to_string())?;
    command
        .stdout(output.try_clone().map_err(|e| e.to_string())?)
        .stderr(output)
        .stdin(Stdio::null())
        .process_group(0);
    let argv: Vec<_> = std::iter::once(command.get_program())
        .chain(command.get_args())
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break if status.success() {
                json!("Success")
            } else {
                json!({"Failure":status.code()})
            };
        }
        if start.elapsed() > Duration::from_secs(timeout)
            || INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst)
        {
            if let Some(pid) = rustix::process::Pid::from_raw(child.id() as i32) {
                // cargo-mutants owns separate worker groups; let its signal
                // handler terminate them before waiting for the outer process.
                let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::TERM);
                let grace = Instant::now();
                while child.try_wait().map_err(|e| e.to_string())?.is_none()
                    && grace.elapsed() < Duration::from_secs(10)
                {
                    std::thread::sleep(Duration::from_millis(25));
                }
                let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            }
            child.wait().map_err(|e| e.to_string())?;
            if INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("mutation execution interrupted".into());
            }
            break json!("Timeout");
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    Ok(json!({"argv":argv,"process_status":status,"duration":start.elapsed().as_secs_f64()}))
}
#[cfg(not(unix))]
fn execute(_: &mut Command, _: &Path, _: u64) -> Result<Value> {
    Err("mutation suite requires Unix process-group isolation".into())
}

fn phases(root: &Path, output: &Path, package: &str, label: &str) -> Result<(String, Vec<Value>)> {
    let mut results = Vec::new();
    for (phase, timeout) in [("Build", 600), ("Test", 120)] {
        let mut cmd = Command::new("cargo");
        cmd.current_dir(root)
            .args(["test", "--locked", "--offline", "-p", package]);
        if phase == "Build" {
            cmd.arg("--no-run");
        }
        // Match cargo-mutants: warnings in intentionally damaged source are not errors.
        cmd.env("RUSTFLAGS", "--cap-lints=allow");
        let log = format!("{label}-{phase}.log");
        let mut result = execute(&mut cmd, &output.join(&log), timeout)?;
        result["phase"] = json!(phase);
        result["log_path"] = json!(log);
        result["log_sha256"] = json!(hash(&read(&output.join(&log))?));
        let status = result["process_status"].clone();
        results.push(result);
        if status == "Timeout" {
            return Ok(("Timeout".into(), results));
        }
        if status != "Success" {
            let failed_tests =
                String::from_utf8_lossy(&read(&output.join(format!("{label}-{phase}.log")))?)
                    .contains("test result: FAILED");
            return Ok((
                if phase == "Test" && status["Failure"] == 101 && failed_tests {
                    "CaughtMutant"
                } else {
                    "Unviable"
                }
                .into(),
                results,
            ));
        }
    }
    Ok(("Success".into(), results))
}
fn replacements(root: &Path, output: &Path, scope: &str, repairs: &[Value]) -> Result<()> {
    if repairs.is_empty() {
        return Ok(());
    }
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let package = if scope == "core" {
        "policy-core"
    } else {
        "policy-ci"
    };
    let (baseline, phases) = phases(root, output, package, "baseline")?;
    let mut outcomes =
        vec![json!({"scenario":"Baseline","summary":baseline,"phase_results":phases})];
    if baseline != "Success" {
        return Err("replacement baseline failed".into());
    }
    let inventory: Vec<_> = repairs.iter().map(repaired_identity).collect();
    write(&output.join("mutants.json"), &json!(inventory))?;
    for (index, repair) in repairs.iter().enumerate() {
        let mutant = &inventory[index];
        let path = root.join(mutant["file"].as_str().ok_or("missing mutant file")?);
        let source = String::from_utf8(read(&path)?).map_err(|e| e.to_string())?;
        let changed = repaired_source(&source, repair)?;
        fs::write(output.join(format!("repair-{index}.source.rs")), &changed)
            .map_err(|e| e.to_string())?;
        fs::write(&path, changed).map_err(|e| e.to_string())?;
        let result = self::phases(root, output, package, &format!("repair-{index}"));
        fs::write(&path, source).map_err(|e| e.to_string())?;
        let (summary, phases) = result?;
        let summary = if summary == "Success" {
            "MissedMutant".to_owned()
        } else {
            summary
        };
        println!("{summary}: {}", mutant["name"]);
        outcomes
            .push(json!({"scenario":{"Mutant":mutant},"summary":summary,"phase_results":phases}));
    }
    let count = |s| {
        outcomes
            .iter()
            .skip(1)
            .filter(|o| o["summary"] == s)
            .count()
    };
    write(
        &output.join("outcomes.json"),
        &json!({"runner":"policy-kernel typed replacements v1","cargo_mutants_version":"27.1.0","end_time":now(),"total_mutants":inventory.len(),"caught":count("CaughtMutant"),"missed":count("MissedMutant"),"unviable":count("Unviable"),"timeout":count("Timeout"),"success":0,"outcomes":outcomes}),
    )?;
    check_replacements(root, output, scope, repairs)
}

fn snapshot(root: &Path, destination: Option<&Path>) -> Result<Value> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("cannot enumerate candidate files".into());
    }
    let mut manifest = serde_json::Map::new();
    for name in output.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
        let name = std::str::from_utf8(name).map_err(|e| e.to_string())?;
        let path = Path::new(name);
        if path.is_absolute()
            || path
                .components()
                .any(|p| !matches!(p, std::path::Component::Normal(_)))
        {
            return Err("unsafe candidate path".into());
        }
        let source = root.join(path);
        if fs::symlink_metadata(&source)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err(format!("candidate symlink unsupported: {name}"));
        }
        let bytes = read(&source)?;
        if let Some(destination) = destination {
            let target = destination.join(path);
            fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(&target, &bytes).map_err(|e| e.to_string())?;
            fs::set_permissions(
                &target,
                fs::metadata(&source)
                    .map_err(|e| e.to_string())?
                    .permissions(),
            )
            .map_err(|e| e.to_string())?;
        }
        manifest.insert(name.to_owned(), json!(hash(&bytes)));
    }
    Ok(Value::Object(manifest))
}
pub fn run(root: &Path, output: &Path, only_repairs: bool) -> Result<()> {
    ctrlc::set_handler(|| INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst))
        .map_err(|e| e.to_string())?;
    if output.exists() {
        return Err("output already exists; evidence must not be overwritten".into());
    }
    let version = Command::new("cargo")
        .args(["mutants", "--version"])
        .output()
        .map_err(|e| e.to_string())?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "cargo-mutants 27.1.0"
    {
        return Err("cargo-mutants 27.1.0 required".into());
    }
    let scratch = tempfile::tempdir().map_err(|e| e.to_string())?;
    let manifest = snapshot(root, Some(scratch.path()))?;
    fs::create_dir_all(output).map_err(|e| e.to_string())?;
    let output = output.canonicalize().map_err(|e| e.to_string())?;
    write(&output.join("source-manifest.json"), &manifest)?;
    let git = |args: &[&str]| -> Result<String> {
        let result = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| e.to_string())?;
        if !result.status.success() {
            return Err("cannot record candidate git state".into());
        }
        Ok(String::from_utf8_lossy(&result.stdout).trim().to_owned())
    };
    write(
        &output.join("candidate.json"),
        &json!({"head":git(&["rev-parse","HEAD"] )?,"status":git(&["status","--porcelain"] )?,"scope":if only_repairs {"repairs-only"} else {"full-suite"},"started_at":now(),"contract_sha256":hash(CONTRACT.as_bytes()),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"generator":String::from_utf8_lossy(&version.stdout).trim(),"rustc":String::from_utf8_lossy(&Command::new("rustc").arg("--version").output().map_err(|e| e.to_string())?.stdout).trim()}),
    )?;
    write(
        &output.join("contract.json"),
        &serde_json::from_str(CONTRACT).map_err(|e| e.to_string())?,
    )?;
    for scope in ["core", "controller", "workflow-source"] {
        let directory = output.join(scope);
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let inventory = generated(scratch.path(), scope)?;
        let (selected, repairs) = select(scratch.path(), array(&inventory)?, scope)?;
        write(&directory.join("generated.json"), &inventory)?;
        write(&directory.join("selected.json"), &json!(selected))?;
        write(&directory.join("repairs.json"), &json!(repairs))?;
        replacements(
            scratch.path(),
            &directory.join("replacements"),
            scope,
            &repairs,
        )?;
        if only_repairs {
            continue;
        }
        let mut command = Command::new("cargo");
        command.current_dir(scratch.path()).args(args(scope)?);
        for repair in &repairs {
            command.args([
                "--exclude-re",
                &escaped(repair["original"]["name"].as_str().unwrap()),
            ]);
        }
        command
            .args([
                "--test-workspace",
                "false",
                "--jobs",
                "2",
                "--timeout",
                "120",
                "--build-timeout",
                "600",
                "-C",
                "--locked",
                "-C",
                "--offline",
                "--output",
            ])
            .arg(&directory);
        let result = execute(&mut command, &directory.join("cargo-mutants.log"), 7200)?;
        write(&directory.join("execution.json"), &result)?;
        if result["process_status"] != "Success" {
            return Err(format!(
                "{scope} cargo-mutants execution failed; see {}",
                directory.display()
            ));
        }
        check_scope(root, &directory, scope)?;
    }
    if !only_repairs {
        check(root, &output)?;
        write(
            &output.join("completed.json"),
            &json!({"ended_at":now(),"scope":"full-suite","status":"passed","contract_sha256":hash(CONTRACT.as_bytes())}),
        )?;
    }
    Ok(())
}
fn check_scope(root: &Path, directory: &Path, scope: &str) -> Result<()> {
    let fresh = generated(root, scope)?;
    let stored = load(&directory.join("generated.json"))?;
    if set(array(&fresh)?)? != set(array(&stored)?)? {
        return Err("unfiltered generator inventory changed or omitted".into());
    }
    let (selected, repairs) = select(root, array(&fresh)?, scope)?;
    if load(&directory.join("selected.json"))? != json!(selected)
        || load(&directory.join("repairs.json"))? != json!(repairs)
    {
        return Err("unauthorized selection or replacement contract".into());
    }
    let native = directory.join("mutants.out");
    if set(array(&load(&native.join("mutants.json"))?)?)? != set(&selected)? {
        return Err("native execution inventory differs from selected inventory".into());
    }
    if load(&directory.join("execution.json"))?["process_status"] != "Success" {
        return Err("native runner did not succeed".into());
    }
    crate::verification::mutation_scope(&native.join("outcomes.json"), scope)?;
    check_replacements(root, &directory.join("replacements"), scope, &repairs)?;
    println!(
        "{scope}: {} generated = {} native + {} typed replacements; zero unmapped",
        array(&fresh)?.len(),
        selected.len(),
        repairs.len()
    );
    Ok(())
}
fn check_replacements(root: &Path, directory: &Path, scope: &str, repairs: &[Value]) -> Result<()> {
    if !repairs.is_empty() {
        for (index, repair) in repairs.iter().enumerate() {
            let source = String::from_utf8(read(
                &root.join(
                    repair["original"]["file"]
                        .as_str()
                        .ok_or("missing repair source")?,
                ),
            )?)
            .map_err(|e| e.to_string())?;
            if read(&directory.join(format!("repair-{index}.source.rs")))?
                != repaired_source(&source, repair)?.as_bytes()
            {
                return Err("replacement source differs from reviewed edit".into());
            }
        }
        let identities: Vec<_> = repairs.iter().map(repaired_identity).collect();
        if set(array(&load(&directory.join("mutants.json"))?)?)? != set(&identities)? {
            return Err("replacement execution inventory differs from contract".into());
        }
        let outcomes = load(&directory.join("outcomes.json"))?;
        if outcomes["runner"] != "policy-kernel typed replacements v1" {
            return Err("unexpected replacement runner".into());
        }
        for outcome in array(&outcomes["outcomes"])? {
            let label = if outcome["scenario"] == "Baseline" {
                "baseline".to_owned()
            } else {
                let identity = crate::verification::identity(&outcome["scenario"]["Mutant"])?;
                let index = identities
                    .iter()
                    .position(|m| crate::verification::identity(m).is_ok_and(|id| id == identity))
                    .ok_or("unknown replacement outcome")?;
                format!("repair-{index}")
            };
            let phases = array(&outcome["phase_results"])?;
            if phases.len() != 2
                || phases[0]["phase"] != "Build"
                || phases[0]["process_status"] != "Success"
                || phases[1]["phase"] != "Test"
            {
                return Err("replacement must build and test".into());
            }
            let expected = if outcome["scenario"] == "Baseline" {
                json!("Success")
            } else {
                json!({"Failure":101})
            };
            if phases[1]["process_status"] != expected {
                return Err("replacement test status not verified".into());
            }
            for phase in phases {
                let log = phase["log_path"].as_str().ok_or("missing phase log")?;
                if log
                    != format!(
                        "{label}-{}.log",
                        phase["phase"].as_str().ok_or("missing phase")?
                    )
                {
                    return Err("log does not bind this replacement and phase".into());
                }
                let bytes = read(&directory.join(log))?;
                if bytes.is_empty() || phase["log_sha256"] != hash(&bytes) {
                    return Err("empty or changed execution log".into());
                }
                if phase["phase"] == "Test"
                    && expected != "Success"
                    && !String::from_utf8_lossy(&bytes).contains("test result: FAILED")
                {
                    return Err("replacement failure was not a test failure".into());
                }
            }
        }
        crate::verification::mutation_scope(&directory.join("outcomes.json"), scope)?;
    }
    Ok(())
}
pub fn check(root: &Path, output: &Path) -> Result<()> {
    let candidate = load(&output.join("candidate.json"))?;
    if candidate["scope"] != "full-suite"
        || candidate["contract_sha256"] != hash(CONTRACT.as_bytes())
        || load(&output.join("contract.json"))?
            != serde_json::from_str::<Value>(CONTRACT).map_err(|e| e.to_string())?
    {
        return Err("incomplete suite or unapproved contract".into());
    }
    check_source_bindings(
        &load(&output.join("source-manifest.json"))?,
        &snapshot(root, None)?,
    )?;
    for scope in ["core", "controller", "workflow-source"] {
        check_scope(root, &output.join(scope), scope)?;
    }
    println!("complete mutation suite passed; all selected native and replacement mutants caught");
    Ok(())
}
fn check_source_bindings(recorded: &Value, current: &Value) -> Result<()> {
    // Prose may record the result afterwards. Every build/test/config/fixture
    // input, including this verifier and contract, remains candidate-bound.
    let inputs = |manifest: &Value| -> Result<serde_json::Map<String, Value>> {
        Ok(manifest
            .as_object()
            .ok_or("invalid source manifest")?
            .iter()
            .filter(|(path, _)| !path.ends_with(".md"))
            .map(|(path, digest)| (path.clone(), digest.clone()))
            .collect())
    };
    let recorded = inputs(recorded)?;
    if recorded.is_empty() || recorded != inputs(current)? {
        return Err("candidate source inventory or content changed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
    }
    fn originals(scope: &str) -> Vec<Value> {
        let c: Value = serde_json::from_str(CONTRACT).unwrap();
        c["repairs"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| (r["original"]["package"] == "policy-core") == (scope == "core"))
            .map(|r| r["original"].clone())
            .collect()
    }
    #[test]
    fn every_generator_repair_is_exact_and_preserves_ordinary_mutants() {
        for scope in ["core", "controller"] {
            let mut generated = originals(scope);
            let mut ordinary = generated[0].clone();
            ordinary["name"] = Value::from("ordinary viable mutation");
            ordinary["replacement"] = Value::from("false");
            generated.push(ordinary.clone());
            let (selected, repaired) = select(root(), &generated, scope).unwrap();
            assert_eq!(selected, vec![ordinary]);
            assert_eq!(repaired.len(), generated.len() - 1);
        }
    }
    #[test]
    fn missing_duplicate_or_changed_original_cannot_be_excluded() {
        let generated = originals("core");
        let mut missing = generated.clone();
        missing.pop();
        assert!(select(root(), &missing, "core").is_err());
        let mut duplicate = generated.clone();
        duplicate.push(generated[0].clone());
        assert!(select(root(), &duplicate, "core").is_err());
        let mut changed = generated;
        changed[0]["span"]["start"]["line"] = Value::from(999);
        assert!(select(root(), &changed, "core").is_err());
    }
    #[test]
    fn stale_source_contract_is_rejected() {
        let root = tempfile::tempdir().unwrap();
        assert!(select(root.path(), &originals("core"), "core").is_err());
    }
    fn replacement_fixture() -> (tempfile::TempDir, Vec<Value>) {
        let output = tempfile::tempdir().unwrap();
        let (_, repairs) = select(root(), &originals("controller"), "controller").unwrap();
        let inventory: Vec<_> = repairs.iter().map(repaired_identity).collect();
        let mut outcomes = Vec::new();
        for index in 0..=repairs.len() {
            let baseline = index == 0;
            let label = if baseline {
                "baseline".to_owned()
            } else {
                format!("repair-{}", index - 1)
            };
            let mut phases = Vec::new();
            for phase in ["Build", "Test"] {
                let failed = !baseline && phase == "Test";
                let log = format!("{label}-{phase}.log");
                let bytes = if failed {
                    "test result: FAILED"
                } else {
                    "test result: ok"
                };
                fs::write(output.path().join(&log), bytes).unwrap();
                phases.push(json!({"phase":phase,"process_status":if failed {json!({"Failure":101})} else {json!("Success")},"log_path":log,"log_sha256":hash(bytes.as_bytes())}));
            }
            outcomes.push(json!({"scenario":if baseline {json!("Baseline")} else {json!({"Mutant":inventory[index-1]})},"summary":if baseline {"Success"} else {"CaughtMutant"},"phase_results":phases}));
        }
        for (index, repair) in repairs.iter().enumerate() {
            let source =
                fs::read_to_string(root().join(repair["original"]["file"].as_str().unwrap()))
                    .unwrap();
            fs::write(
                output.path().join(format!("repair-{index}.source.rs")),
                repaired_source(&source, repair).unwrap(),
            )
            .unwrap();
        }
        write(&output.path().join("mutants.json"), &json!(inventory)).unwrap();
        write(&output.path().join("outcomes.json"), &json!({"runner":"policy-kernel typed replacements v1","cargo_mutants_version":"27.1.0","end_time":"finished","total_mutants":repairs.len(),"caught":repairs.len(),"missed":0,"timeout":0,"unviable":0,"success":0,"outcomes":outcomes})).unwrap();
        (output, repairs)
    }
    #[test]
    fn replacements_accept_complete_evidence_and_reject_incomplete_execution() {
        let (output, repairs) = replacement_fixture();
        check_replacements(root(), output.path(), "controller", &repairs).unwrap();
        for pointer in [
            "/outcomes/0/phase_results/0/process_status",
            "/outcomes/1/phase_results/0/process_status",
            "/outcomes/1/phase_results/1/process_status",
        ] {
            let (output, repairs) = replacement_fixture();
            let path = output.path().join("outcomes.json");
            let mut value = load(&path).unwrap();
            *value.pointer_mut(pointer).unwrap() = json!("Timeout");
            write(&path, &value).unwrap();
            assert!(check_replacements(root(), output.path(), "controller", &repairs).is_err());
        }
    }
    #[test]
    fn replacement_logs_cannot_be_changed_after_execution() {
        let (output, repairs) = replacement_fixture();
        fs::write(
            output.path().join("repair-0-Test.log"),
            "different test result: FAILED",
        )
        .unwrap();
        assert!(check_replacements(root(), output.path(), "controller", &repairs).is_err());
    }
    #[test]
    fn replacement_source_cannot_differ_from_reviewed_edit() {
        let (output, repairs) = replacement_fixture();
        fs::write(
            output.path().join("repair-0.source.rs"),
            "// a different mutant",
        )
        .unwrap();
        assert!(check_replacements(root(), output.path(), "controller", &repairs).is_err());
    }
    #[test]
    fn candidate_binding_rejects_omissions_additions_and_changed_tests() {
        let original = json!({"src/lib.rs":"a","tests/fixture.json":"b","README.md":"c"});
        check_source_bindings(&original, &original).unwrap();
        for changed in [
            json!({"src/lib.rs":"a"}),
            json!({"src/lib.rs":"a","tests/fixture.json":"changed"}),
            json!({"src/lib.rs":"a","tests/fixture.json":"b","extra.rs":"new"}),
        ] {
            assert!(check_source_bindings(&original, &changed).is_err());
        }
        let mut prose = original.clone();
        prose["README.md"] = json!("later result record");
        check_source_bindings(&original, &prose).unwrap();
        assert!(check_source_bindings(&json!({}), &json!({})).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn timeout_allows_a_supervisor_to_clean_its_separate_worker_group() {
        let output = tempfile::tempdir().unwrap();
        let heartbeat = output.path().join("heartbeat");
        let mut command = Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "mutation_suite::tests::process_fixture",
                "--nocapture",
            ])
            .env("POLICY_MUTATION_PROCESS_FIXTURE", "supervisor")
            .env("POLICY_MUTATION_HEARTBEAT", &heartbeat);
        let result = execute(&mut command, &output.path().join("child.log"), 1).unwrap();
        assert_eq!(result["process_status"], "Timeout");
        let stopped = fs::read(&heartbeat).expect("worker ran before timeout");
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(stopped, fs::read(&heartbeat).unwrap());
    }
    #[cfg(unix)]
    #[test]
    fn process_fixture() {
        use std::os::unix::process::CommandExt;
        let Ok(mode) = std::env::var("POLICY_MUTATION_PROCESS_FIXTURE") else {
            return;
        };
        if mode == "worker" {
            for index in 0..1000 {
                fs::write(
                    std::env::var("POLICY_MUTATION_HEARTBEAT").unwrap(),
                    index.to_string(),
                )
                .unwrap();
                std::thread::sleep(Duration::from_millis(20));
            }
        } else {
            ctrlc::set_handler(|| INTERRUPTED.store(true, std::sync::atomic::Ordering::SeqCst))
                .unwrap();
            let mut child = Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "mutation_suite::tests::process_fixture",
                    "--nocapture",
                ])
                .env("POLICY_MUTATION_PROCESS_FIXTURE", "worker")
                .process_group(0)
                .spawn()
                .unwrap();
            while !INTERRUPTED.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }
            rustix::process::kill_process_group(
                rustix::process::Pid::from_raw(child.id() as i32).unwrap(),
                rustix::process::Signal::TERM,
            )
            .unwrap();
            child.wait().unwrap();
        }
    }
}
