use super::*;
use std::process::Command;

fn git(repository: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .env("GIT_NO_LAZY_FETCH", "1")
        .arg("-C")
        .arg(repository)
        .args(args)
        .output()?;
    ensure(
        output.status.success(),
        &format!("TJ-GIT: {}", String::from_utf8_lossy(&output.stderr)),
    )?;
    Ok(output.stdout)
}

/// Compare archive bytes with every regular Git blob except the explicit Flutter exclusion.
/// This is a local consistency check, not authentication of Git or inventory ownership.
pub fn git_inventory(
    repository: &Path,
    checkout: &Path,
    revision: &CommitSha,
) -> Result<Vec<ArtifactDigest>> {
    ensure(
        repository.canonicalize()? != checkout.canonicalize()? && !checkout.join(".git").exists(),
        "TJ-ARCHIVE: checkout must be a separate archive without .git",
    )?;
    let resolved = git(
        repository,
        &["rev-parse", "--verify", &format!("{}^{{commit}}", revision)],
    )?;
    ensure(
        String::from_utf8(resolved)?.trim() == revision.as_str(),
        "TJ-GIT-REVISION",
    )?;
    ensure(
        String::from_utf8(git(repository, &["rev-parse", "HEAD"])?)?.trim() == revision.as_str(),
        "TJ-GIT-HEAD: requested revision is not the current source repository HEAD",
    )?;
    let tree = git(
        repository,
        &["ls-tree", "-rz", "--full-tree", revision.as_str()],
    )?;
    let mut inventory = vec![];
    for entry in tree.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let (metadata, path) = std::str::from_utf8(entry)?
            .split_once('\t')
            .ok_or("TJ-GIT-TREE")?;
        relative(path)?;
        if path.starts_with("authenticator/") {
            continue;
        }
        let parts: Vec<_> = metadata.split(' ').collect();
        ensure(
            parts.len() == 3 && matches!(parts[0], "100644" | "100755") && parts[1] == "blob",
            "TJ-GIT-TREE: symlink/submodule unsupported in scope",
        )?;
        let expected = git(repository, &["cat-file", "blob", parts[2]])?;
        let bytes = checked_read(checkout, path)?;
        ensure(bytes == expected, &format!("TJ-SOURCE-CHANGED: {path}"))?;
        inventory.push(ArtifactDigest {
            path: path.into(),
            digest: bytes_digest(&bytes),
        });
    }
    inventory.sort_by(|a, b| a.path.cmp(&b.path));
    ensure(!inventory.is_empty(), "TJ-GIT-INVENTORY-EMPTY")?;
    let tracked: BTreeSet<_> = inventory.iter().map(|a| a.path.as_str()).collect();
    check_extra_files(checkout, checkout, &tracked)?;
    Ok(inventory)
}

fn check_extra_files(root: &Path, directory: &Path, tracked: &BTreeSet<&str>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(root)?
            .to_str()
            .ok_or("TJ-SOURCE-PATH")?
            .to_owned();
        relative(&relative_path)?;
        if relative_path == "authenticator" {
            continue;
        }
        // These runtime outputs/dependencies are not source. Their configuration and
        // required generated output remain checked by project-verification.
        let runtime_directory = matches!(
            entry.file_name().to_str(),
            Some(
                "node_modules"
                    | "bin"
                    | "obj"
                    | ".policy-reports"
                    | ".svelte-kit"
                    | "test-results"
                    | "playwright-report"
                    | "build"
                    | "dist"
            )
        );
        if entry.file_type()?.is_dir() && runtime_directory {
            continue;
        }
        ensure(
            !entry.file_type()?.is_symlink(),
            "TJ-PATH: symlink outside runtime outputs",
        )?;
        if entry.file_type()?.is_dir() {
            check_extra_files(root, &path, tracked)?;
        } else {
            ensure(
                tracked.contains(relative_path.as_str()),
                &format!("TJ-SOURCE-UNTRACKED: {relative_path}"),
            )?;
        }
    }
    Ok(())
}

/// Save with create_new, including errors and raw reports; no prior artifact is overwritten.
pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    write_new(path, &serde_json::to_vec_pretty(value)?)
}

pub fn discovery(root: &Path, reports_root: &Path, suites: &Suites) -> Result<Vec<RawTest>> {
    let mut tests = vec![];
    for (id, c, s) in suites.entries() {
        let bytes = checked_read(reports_root, &format!("{}/{}", c.cwd, s.report))?;
        let parsed = project_verification::reports::parse_report(s.format, &id, &bytes)?;
        ensure(
            parsed.successful
                && parsed.discovered > 0
                && parsed.discovered == parsed.executed
                && parsed.executed == parsed.passed,
            &format!("TJ-DISCOVERY-NONPASS: {id}"),
        )?;
        tests.extend(link_report(root, &id, s, &bytes)?);
    }
    Ok(tests)
}

/// Execute only reviewed profile commands; the caller provisions tools and API lifecycle.
/// Failure archives are non-pass even when the project API returns no normalized result.
pub fn execute(
    requests: &[RunRequest; 2],
    output: &Path,
    catalog: &Catalog,
    suites: &Suites,
    time: impl Fn() -> UtcTimestamp,
    max_age_seconds: u64,
) -> Result<ScopeReport> {
    for request in requests {
        let component = &request.profile.components[0].id;
        write_json(&output.join(format!("{component}.request.json")), request)?;
        let prepared = project_verification::run::prepare(request)?;
        write_json(
            &output.join(format!("{component}.prepared.json")),
            &prepared,
        )?;
    }
    let mut results = vec![];
    let mut errors = BTreeMap::new();
    let mut raw_tests = vec![];
    for request in requests {
        let component = &request.profile.components[0].id;
        match project_verification::run(request.clone()) {
            Ok(result) => {
                write_json(&output.join(format!("{component}.result.json")), &result)?;
                results.push(result);
            }
            Err(e) => {
                errors.insert(component.clone(), e.to_string());
            }
        }
        // Preserve report bytes even when suite validation prevented a RunResult.
        for (id, c, s) in suites
            .entries()
            .filter(|(id, _, _)| id.starts_with(&format!("{component}:")))
        {
            let path = format!("{}/{}", c.cwd, s.report);
            match checked_read(&request.root, &path) {
                Ok(bytes) => {
                    write_new(
                        &output.join(format!("{}.raw", id.replace(':', "-"))),
                        &bytes,
                    )?;
                    match link_report(&request.root, &id, s, &bytes) {
                        Ok(tests) => {
                            write_json(
                                &output.join(format!("{}.identities.json", id.replace(':', "-"))),
                                &tests,
                            )?;
                            raw_tests.extend(tests);
                        }
                        Err(e) => {
                            errors.insert(format!("{id}:parse"), e.to_string());
                        }
                    }
                }
                Err(e) => {
                    errors.insert(format!("{id}:report"), e.to_string());
                }
            }
        }
    }
    let requirement_status: BTreeMap<_,_> = catalog.requirements.iter().map(|r| {
        let failures: Vec<_> = r.tests.iter().filter_map(|s| resolve(s,&raw_tests,suites).err().map(|e|e.to_string())).collect();
        let mapped_ids: BTreeSet<_> = requests.iter().flat_map(|q|q.mappings.acceptance_criteria.iter().chain(&q.mappings.changed_behaviors))
            .filter(|m|m.requirement_id==r.id).flat_map(|m|m.test_ids.iter()).collect();
        let observations: Vec<_> = raw_tests.iter().filter(|t|mapped_ids.contains(&t.normalized_id)).collect();
        (&r.id,serde_json::json!({"status":if failures.is_empty(){"satisfied"}else{"failed/unproven"},"selector_errors":failures,"observations":observations}))
    }).collect();
    write_json(&output.join("requirements.json"), &requirement_status)?;
    if !errors.is_empty() {
        write_json(
            &output.join("failure.json"),
            &serde_json::json!({"label":"local scope non-pass","stage":"collection","errors":errors,"trust":"local_untrusted","normalized_result_missing_is_pass":false}),
        )?;
        return Err("TJ-COLLECTION-NONPASS: see failure.json and raw reports".into());
    }
    let results: [RunResult; 2] = results.try_into().map_err(|_| "TJ-SPLIT-RESULT-MISSING")?;
    let report = evaluate_pair(
        requests,
        &results,
        &time(),
        max_age_seconds,
        catalog,
        suites,
    )?;
    Ok(report)
}
