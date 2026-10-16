//! Explicit npm workspaces share one install root, lock inventory, and audit.
use crate::{
    invalid, paths,
    profile::{relative, ProjectProfile},
    selection::{DependencyAdapter, DependencyGraph, Selection},
    Error,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpmWorkspace {
    /// Component-relative paths, never npm workspace patterns.
    pub member_manifests: Vec<String>,
    /// Consumer-declared minimum source revision containing this extension.
    /// This is compatibility metadata, not authenticated provenance.
    pub minimum_pk_revision: String,
}

fn explicit(path: &str) -> bool {
    relative(path, false)
        && !path.contains(['*', '?', '[', ']', '{', '}', '!', '#'])
        && !path
            .split('/')
            .any(|part| matches!(part, "node_modules" | ".git"))
}

pub(crate) fn validate_contract(
    selection: &Selection,
    profile: &ProjectProfile,
    graph: &DependencyGraph,
) -> Result<(), Error> {
    let Some(workspace) = &graph.workspace else {
        return Ok(());
    };
    if selection.schema_version != 3
        || graph.adapter != DependencyAdapter::Npm
        || graph.manifest != "package.json"
        || graph.lockfile != "package-lock.json"
        || workspace.member_manifests.is_empty()
        || workspace.minimum_pk_revision.len() != 40
        || !workspace
            .minimum_pk_revision
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        || selection
            .graphs
            .iter()
            .filter(|g| g.component == graph.component)
            .count()
            != 1
    {
        return Err(invalid(
            "workspace needs selection 3, one component-root npm graph and a minimum PK revision",
        ));
    }
    let mut seen = BTreeSet::new();
    for member in &workspace.member_manifests {
        if !explicit(member) || !member.ends_with("/package.json") || !seen.insert(member) {
            return Err(invalid(
                "workspace members must be unique explicit descendant manifests",
            ));
        }
    }
    let audit = profile
        .sensors
        .iter()
        .find(|s| s.definition.id.as_str() == graph.audit_sensor)
        .ok_or_else(|| invalid("missing workspace audit"))?;
    if workspace
        .member_manifests
        .iter()
        .any(|m| !audit.inputs.contains(m))
    {
        return Err(invalid("workspace audit must bind every member manifest"));
    }
    for package in selection
        .packages
        .iter()
        .filter(|p| p.component == graph.component)
    {
        let path = package
            .installed_manifest
            .strip_suffix("/package.json")
            .ok_or_else(|| {
                invalid("workspace package selection must name its installed manifest")
            })?;
        let pointer = format!(
            "/packages/{}/version",
            path.replace('~', "~0").replace('/', "~1")
        );
        if package.lock_pointer != pointer || package.lockfile != graph.lockfile {
            return Err(invalid(
                "workspace installed manifest must bind its own lock location",
            ));
        }
    }
    Ok(())
}

fn declarations(manifest: &Value, locked: &Value) -> Result<(), Error> {
    if !manifest.is_object() || !locked.is_object() {
        return Err(invalid("workspace package must be an object"));
    }
    for field in [
        "version",
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
        "peerDependenciesMeta",
    ] {
        if manifest.get(field) != locked.get(field) {
            return Err(invalid(format!("workspace manifest/lock drift: {field}")));
        }
    }
    for field in [
        "dependencies",
        "devDependencies",
        "optionalDependencies",
        "peerDependencies",
    ] {
        if let Some(deps) = manifest.get(field) {
            for (name, requirement) in deps
                .as_object()
                .ok_or_else(|| invalid("workspace dependencies must be an object"))?
            {
                let requirement = requirement
                    .as_str()
                    .ok_or_else(|| invalid("invalid dependency requirement"))?;
                if !package_name(name)
                    || requirement.is_empty()
                    || requirement.contains([':', '/', '\\'])
                {
                    return Err(invalid("workspace dependencies must use registry versions; external/file dependencies are forbidden"));
                }
            }
        }
    }
    if manifest.get("bundledDependencies").is_some()
        || manifest.get("bundleDependencies").is_some()
        || manifest.get("overrides").is_some()
    {
        return Err(invalid("workspace bundles and overrides are unsupported"));
    }
    Ok(())
}

fn package_name(name: &str) -> bool {
    let scoped = name.starts_with('@');
    let name = name.strip_prefix('@').unwrap_or(name);
    relative(name, false)
        && name.split('/').count() == if scoped { 2 } else { 1 }
        && name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-._/".contains(&c))
}

pub(crate) fn validate_files(
    graph: &DependencyGraph,
    files: &BTreeMap<String, Vec<u8>>,
    allow_missing: bool,
) -> Result<(), Error> {
    let Some(workspace) = &graph.workspace else {
        return Ok(());
    };
    let read = |path: &str| -> Result<Option<Value>, Error> {
        match files.get(path) {
            Some(bytes) => Ok(Some(serde_json::from_slice(bytes)?)),
            None if allow_missing => Ok(None),
            None => Err(invalid(format!("missing workspace input: {path}"))),
        }
    };
    let root = read(&graph.manifest)?;
    let lock = read(&graph.lockfile)?;
    let dirs: BTreeSet<_> = workspace
        .member_manifests
        .iter()
        .map(|m| m.strip_suffix("/package.json").unwrap())
        .collect();
    let declared = |value: &Value| -> Result<BTreeSet<String>, Error> {
        let list = value
            .get("workspaces")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("workspace root needs an explicit workspaces array"))?;
        let mut set = BTreeSet::new();
        for item in list {
            let path = item
                .as_str()
                .ok_or_else(|| invalid("workspace path must be a string"))?;
            if !explicit(path) || !set.insert(path.to_owned()) {
                return Err(invalid(
                    "workspace paths must be unique and explicit; globs are unsupported",
                ));
            }
        }
        if set.iter().map(String::as_str).collect::<BTreeSet<_>>() != dirs {
            return Err(invalid("declared workspace members disagree with graph"));
        }
        Ok(set)
    };
    if let Some(root) = &root {
        declared(root)?;
    }
    let packages = lock
        .as_ref()
        .map(|l| {
            l.get("packages")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid("missing workspace lock packages"))
        })
        .transpose()?;
    if let Some(packages) = packages {
        let locked_root = packages
            .get("")
            .ok_or_else(|| invalid("missing workspace lock root"))?;
        declared(locked_root)?;
        if let Some(root) = &root {
            declarations(root, locked_root)?;
        }
        let actual: BTreeSet<_> = packages
            .keys()
            .filter(|k| !k.is_empty() && !k.split('/').any(|p| p == "node_modules"))
            .map(String::as_str)
            .collect();
        if actual != dirs {
            return Err(invalid("workspace lock member inventory disagrees"));
        }
        let mut expected_links = BTreeMap::new();
        for member in &workspace.member_manifests {
            let dir = member.strip_suffix("/package.json").unwrap();
            let locked = &packages[dir];
            if let Some(manifest) = read(member)? {
                declarations(&manifest, locked)?;
                if manifest.get("workspaces").is_some() {
                    return Err(invalid("nested workspaces are unsupported"));
                }
                let name = manifest
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|n| package_name(n))
                    .ok_or_else(|| invalid("workspace member needs a valid package name"))?;
                if locked.get("name").is_some_and(|n| n.as_str() != Some(name)) {
                    return Err(invalid("workspace lock member name drift"));
                }
                if expected_links
                    .insert(format!("node_modules/{name}"), dir)
                    .is_some()
                {
                    return Err(invalid("duplicate workspace package name"));
                }
            } else {
                return Ok(());
            }
        }
        for (path, package) in packages {
            if path.is_empty() {
                continue;
            }
            if !relative(path, false) {
                return Err(invalid("unsafe workspace lock path"));
            }
            if package.get("link").and_then(Value::as_bool) == Some(true) {
                if expected_links
                    .remove(path)
                    .is_none_or(|dir| package.get("resolved").and_then(Value::as_str) != Some(dir))
                {
                    return Err(invalid("unexpected or mismatched workspace link"));
                }
            } else if path.split('/').any(|p| p == "node_modules") {
                let prefix = path
                    .split("node_modules/")
                    .next()
                    .unwrap()
                    .trim_end_matches('/');
                if !prefix.is_empty() && !dirs.contains(prefix) {
                    return Err(invalid("undeclared workspace lock package location"));
                }
                if !package
                    .get("resolved")
                    .and_then(Value::as_str)
                    .is_some_and(|url| url.starts_with("https://registry.npmjs.org/"))
                {
                    return Err(invalid(
                        "workspace lock dependencies must resolve to the HTTPS npm registry",
                    ));
                }
            }
        }
        if !expected_links.is_empty() {
            return Err(invalid("missing workspace lock links"));
        }
        validate_dependency_inventory(packages)?;
    }
    Ok(())
}

/// Check that npm's ancestor lookup can find each declared dependency in the
/// audited lock inventory. Optional peers may legitimately be absent.
fn validate_dependency_inventory(packages: &serde_json::Map<String, Value>) -> Result<(), Error> {
    for (path, package) in packages {
        if package.get("link").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        for field in [
            "dependencies",
            "devDependencies",
            "optionalDependencies",
            "peerDependencies",
        ] {
            let Some(dependencies) = package.get(field) else {
                continue;
            };
            for (name, requirement) in dependencies
                .as_object()
                .ok_or_else(|| invalid("invalid locked dependency declarations"))?
            {
                if !package_name(name) {
                    return Err(invalid("invalid locked dependency name"));
                }
                let mut directory = path.as_str();
                let mut resolved = None;
                loop {
                    let candidate = if directory.is_empty() {
                        format!("node_modules/{name}")
                    } else {
                        format!("{directory}/node_modules/{name}")
                    };
                    if let Some(value) = packages.get(&candidate) {
                        resolved = Some(value);
                        break;
                    }
                    if directory.is_empty() {
                        break;
                    }
                    directory = directory.rsplit_once('/').map_or("", |(parent, _)| parent);
                }
                let optional_peer = field == "peerDependencies"
                    && package
                        .get("peerDependenciesMeta")
                        .and_then(|v| v.get(name))
                        .and_then(|v| v.get("optional"))
                        .and_then(Value::as_bool)
                        == Some(true);
                let Some(resolved) = resolved else {
                    if optional_peer {
                        continue;
                    }
                    return Err(invalid(format!(
                        "dependency omitted from workspace audit inventory: {path}:{name}"
                    )));
                };
                // Exact numeric requirements are common in pinned consumer profiles.
                // Range evaluation remains npm's responsibility; presence never proves
                // that a hand-edited lock is a reproducible installation.
                if let Some(exact) = requirement
                    .as_str()
                    .filter(|v| crate::selection::numeric_version(v).is_ok())
                {
                    let resolved = if resolved.get("link").and_then(Value::as_bool) == Some(true) {
                        resolved
                            .get("resolved")
                            .and_then(Value::as_str)
                            .and_then(|p| packages.get(p))
                            .ok_or_else(|| invalid("unresolved workspace dependency link"))?
                    } else {
                        resolved
                    };
                    if resolved.get("version").and_then(Value::as_str) != Some(exact) {
                        return Err(invalid(
                            "exact workspace dependency differs from lock resolution",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_layout(
    selection: &Selection,
    component: &str,
    root: &Path,
) -> Result<(), Error> {
    for graph in selection.graphs.iter().filter(|g| g.component == component) {
        let Some(workspace) = &graph.workspace else {
            continue;
        };
        // Distinct nonexistent files isolate npm's user/global configuration.
        // npm rejects using the same file for both configuration sources.
        paths::require_absent(root, ".policy-npm-userconfig")?;
        paths::require_absent(root, ".policy-npm-globalconfig")?;
        let mut found = BTreeSet::new();
        let mut remaining = 100_000;
        inventory(root, root, &mut found, &mut remaining, 0)?;
        let expected: BTreeSet<_> = workspace
            .member_manifests
            .iter()
            .cloned()
            .chain(["package.json".into()])
            .collect();
        if found != expected {
            return Err(invalid(
                "actual workspace manifest inventory disagrees with graph",
            ));
        }
        for member in &workspace.member_manifests {
            let bytes = paths::read(root, member)?;
            let manifest: Value = serde_json::from_slice(&bytes)?;
            let name = manifest
                .get("name")
                .and_then(Value::as_str)
                .filter(|n| package_name(n))
                .ok_or_else(|| invalid("workspace member needs a valid package name"))?;
            let parent = if let Some((scope, _)) = name.rsplit_once('/') {
                format!("node_modules/{scope}")
            } else {
                "node_modules".into()
            };
            paths::directory(root, &parent)?;
            let link = root.join(format!("node_modules/{name}"));
            let metadata = std::fs::symlink_metadata(&link)?;
            let target = root.join(member.strip_suffix("/package.json").unwrap());
            if !metadata.file_type().is_symlink()
                || link.canonicalize()? != target.canonicalize()?
            {
                return Err(invalid("installed workspace link disagrees with member"));
            }
        }
    }
    Ok(())
}

fn inventory(
    root: &Path,
    dir: &Path,
    found: &mut BTreeSet<String>,
    remaining: &mut usize,
    depth: usize,
) -> Result<(), Error> {
    if depth > 64 {
        return Err(invalid("workspace inventory depth exceeded"));
    }
    for entry in std::fs::read_dir(dir)? {
        if *remaining == 0 {
            return Err(invalid("workspace inventory limit exceeded"));
        }
        *remaining -= 1;
        let entry = entry?;
        let name = entry.file_name();
        if name == "node_modules" || name == ".git" {
            continue;
        }
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(invalid(
                "symlinks outside node_modules are forbidden in a workspace",
            ));
        }
        if name == ".npmrc"
            || name == "npm-shrinkwrap.json"
            || (name == "package-lock.json" && dir != root)
        {
            return Err(invalid(
                "workspace npm configuration or nested lock is unsupported",
            ));
        }
        if kind.is_dir() {
            inventory(root, &entry.path(), found, remaining, depth + 1)?;
        } else if name == "package.json" {
            if !kind.is_file() {
                return Err(invalid("workspace manifest is not a regular file"));
            }
            found.insert(
                entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .ok_or_else(|| invalid("non-UTF8 workspace path"))?
                    .replace(std::path::MAIN_SEPARATOR, "/"),
            );
        }
    }
    Ok(())
}
