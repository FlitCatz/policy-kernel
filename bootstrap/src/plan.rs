use crate::{
    digest,
    fs::Root,
    invalid, json,
    templates::{self, join, MANIFEST, TRANSACTION},
    Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installation {
    pub profile: String,
    pub release: String,
    #[serde(default)]
    pub components: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_profile: Option<UserProfile>,
}

/// An input binding, never an installer ownership claim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserProfile {
    pub path: String,
    /// Empty for a new request; planning binds the exact source bytes.
    pub digest: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    #[default]
    Install,
    Uninstall,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub target: PathBuf,
    pub installation: Installation,
    #[serde(default)]
    pub operation: Operation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Add,
    Unchanged,
    Update,
    Remove,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub path: String,
    pub action: Action,
    pub before_digest: Option<String>,
    pub after_digest: Option<String>,
    /// Proposed template text; never contains bytes from an unowned local file.
    pub content: Option<String>,
    /// Old known template text for a managed update/removal, allowing a full diff.
    pub previous: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub schema_version: u32,
    pub request: Request,
    pub target_identity: (u64, u64),
    /// Digests of all inspected inputs, including absent files (null).
    pub inputs: BTreeMap<String, Option<String>>,
    /// Sorted directory inventories bind detection of newly added manifests.
    pub directories: BTreeMap<String, Option<Vec<String>>>,
    pub files: Vec<Change>,
    pub conflicts: Vec<String>,
    pub prerequisites: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub schema_version: u32,
    pub installation: Installation,
    pub project_profile_schema: u32,
    pub sensor_profile_schema: u32,
    pub evidence_schema: u32,
    pub files: BTreeMap<String, String>,
}

pub(crate) fn manifest(installation: &Installation, files: &BTreeMap<String, String>) -> Manifest {
    Manifest {
        schema_version: if installation.release == "1.1.0" {
            2
        } else {
            1
        },
        installation: installation.clone(),
        project_profile_schema: if installation.release == "1.1.0" {
            2
        } else {
            1
        },
        sensor_profile_schema: 2,
        evidence_schema: 1,
        files: files
            .iter()
            .map(|(p, c)| (p.clone(), digest(c.as_bytes())))
            .collect(),
    }
}

pub(crate) fn owned(bytes: &[u8]) -> Result<BTreeMap<String, String>> {
    let receipt: Manifest = serde_json::from_slice(bytes)?;
    let mut files = templates::render(&receipt.installation)?;
    let expected = manifest(&templates::normalized(&receipt.installation)?, &files);
    // Receipts may only own exact known template bytes, never arbitrary local files.
    if json(&receipt)? != json(&expected)? {
        return Err(invalid("invalid or altered bootstrap manifest"));
    }
    files.insert(
        MANIFEST.into(),
        String::from_utf8(bytes.to_vec()).map_err(|_| invalid("invalid manifest text"))?,
    );
    Ok(files)
}

fn observe(root: &Root, plan: &mut Plan, path: &str) -> Result<Option<Vec<u8>>> {
    let bytes = root.read(path)?;
    plan.inputs
        .insert(path.into(), bytes.as_deref().map(digest));
    Ok(bytes)
}

fn directory(root: &Root, plan: &mut Plan, path: &str) -> Result<Vec<String>> {
    let names = root.names(path)?;
    plan.directories.insert(path.into(), names.clone());
    Ok(names.unwrap_or_default())
}

fn detect(root: &Root, plan: &mut Plan, installation: &Installation) -> Result<()> {
    let profile = templates::profile(installation)?;
    for component in &profile.components {
        let names = directory(root, plan, &component.root)?;
        let package_path = join(&component.root, "package.json");
        if let Some(bytes) = observe(root, plan, &package_path)? {
            let package: serde_json::Value = match serde_json::from_slice(&bytes) {
                Ok(value @ serde_json::Value::Object(_)) => value,
                _ => {
                    plan.conflicts
                        .push(format!("malformed manifest: {package_path}"));
                    continue;
                }
            };
            if component.id == "backend" {
                plan.conflicts.push(format!(
                    "mixed project detection at {}: package.json in backend root",
                    component.root
                ));
            } else {
                let dependency = |name: &str| {
                    package
                        .get("devDependencies")
                        .and_then(|v| v.get(name))
                        .or_else(|| package.get("dependencies").and_then(|v| v.get(name)))
                };
                if component.id == "frontend"
                    && (dependency("vite").is_none()
                        || ["next", "react-scripts", "@angular/core", "jest"]
                            .iter()
                            .any(|name| dependency(name).is_some()))
                {
                    plan.conflicts.push(format!("ambiguous or unsupported frontend tools: {package_path}; pinned Vite/Vitest profile requires explicit compatible tooling"));
                }
                for (tool, pin) in [
                    (
                        "vite",
                        if installation.release == "1.0.0" {
                            "7.1.3"
                        } else {
                            "7.3.6"
                        },
                    ),
                    ("typescript", "5.9.2"),
                    ("eslint", "9.34.0"),
                    (
                        "vitest",
                        if installation.release == "1.0.0" {
                            "3.2.4"
                        } else {
                            "4.1.11"
                        },
                    ),
                    (
                        "@playwright/test",
                        if installation.release == "1.0.0" {
                            "1.55.0"
                        } else {
                            "1.55.1"
                        },
                    ),
                    ("@typescript-eslint/parser", "8.42.0"),
                    ("jsdom", "26.1.0"),
                ] {
                    if let Some(actual) = dependency(tool) {
                        if actual.as_str() != Some(pin) {
                            plan.conflicts.push(format!("{package_path}: {tool} must explicitly match template pin {pin}; adapt existing tooling separately"));
                        }
                    }
                }
            }
        }
        let projects: Vec<_> = names.iter().filter(|n| n.ends_with(".csproj")).collect();
        if !projects.is_empty()
            && (component.id != "backend"
                || projects.len() != 1
                || projects[0].as_str() != "App.csproj")
        {
            plan.conflicts.push(format!(
                "ambiguous project detection at {}; expected one App.csproj for backend",
                component.root
            ));
        }
        for name in names.iter().filter(|n| {
            n.ends_with(".csproj")
                || n.ends_with(".sln")
                || n.ends_with(".slnx")
                || matches!(
                    n.as_str(),
                    "yarn.lock" | "pnpm-lock.yaml" | "bun.lock" | "bun.lockb"
                )
        }) {
            let path = join(&component.root, name);
            let bytes = observe(root, plan, &path)?;
            if name.ends_with(".csproj") && component.id == "backend" {
                // The release's profile is explicitly net9.0. Never migrate a consumer runtime.
                let text = bytes
                    .as_deref()
                    .and_then(|b| std::str::from_utf8(b).ok())
                    .unwrap_or("");
                let compatible = roxmltree::Document::parse(text)
                    .ok()
                    .is_some_and(|document| {
                        let targets: Vec<_> = document
                            .descendants()
                            .filter(|node| {
                                node.has_tag_name("TargetFramework")
                                    || node.has_tag_name("TargetFrameworks")
                            })
                            .collect();
                        document.root_element().has_tag_name("Project")
                            && targets.len() == 1
                            && targets[0].has_tag_name("TargetFramework")
                            && targets[0].text().map(str::trim) == Some("net9.0")
                            && !targets[0]
                                .ancestors()
                                .any(|node| node.attribute("Condition").is_some())
                    });
                if !compatible {
                    plan.conflicts
                        .push(format!("unsupported or ambiguous target framework: {path}"));
                }
            }
            if matches!(
                name.as_str(),
                "yarn.lock" | "pnpm-lock.yaml" | "bun.lock" | "bun.lockb"
            ) && component.id != "backend"
            {
                plan.conflicts.push(format!(
                    "ambiguous package manager: {path}; template uses npm"
                ));
            }
        }
    }
    Ok(())
}

fn detect_selected(
    root: &Root,
    plan: &mut Plan,
    profile: &project_verification::ProjectProfile,
    proposed: &BTreeMap<String, String>,
) -> Result<()> {
    let selection = profile
        .selection
        .as_ref()
        .ok_or_else(|| invalid("missing selection contract"))?;
    for component in &profile.components {
        directory(root, plan, &component.root)?;
        let mut files = BTreeMap::new();
        for input in selection.inputs(&component.id) {
            let path = join(&component.root, &input);
            match observe(root, plan, &path)?
                .or_else(|| proposed.get(&path).map(|s| s.as_bytes().to_vec()))
            {
                Some(bytes) => {
                    files.insert(input, bytes);
                }
                None => plan.prerequisites.push(path),
            }
        }
        for runtime in selection
            .runtime_files
            .iter()
            .filter(|r| r.component == component.id)
        {
            if let Some(bytes) = files.get(&runtime.path) {
                if let Err(error) = selection.validate_runtime_file(runtime, bytes) {
                    plan.conflicts
                        .push(format!("{}: {error}", join(&component.root, &runtime.path)));
                }
            }
        }
        // Check each available graph/package independently, so absent installed
        // packages cannot mask a conflicting runtime or application lockfile.
        if let Err(error) = project_verification::selection::validate_available_component_files(
            selection,
            &component.id,
            &files,
        ) {
            plan.conflicts.push(format!("{}: {error}", component.root));
        }
    }
    Ok(())
}

/// Inspection and planning share the same read-only snapshot to avoid a detection/plan gap.
pub fn inspect(request: Request) -> Result<Plan> {
    plan(request)
}

pub fn plan(mut request: Request) -> Result<Plan> {
    let root = Root::open(&request.target)?;
    request.target = root.path.clone();
    // Source contents are needed only to plan installation. Receipt validation,
    // recovery and removal deliberately do not load this mutable user input.
    let mut user_source = None;
    if let Some(source) = &mut request.installation.user_profile {
        templates::validate_user_path(&source.path)?;
        if request.operation == Operation::Uninstall {
            let bytes = root
                .read(MANIFEST)?
                .ok_or_else(|| invalid("no managed installation to uninstall"))?;
            owned(&bytes)?;
            let receipt: Manifest = serde_json::from_slice(&bytes)?;
            let recorded = receipt
                .installation
                .user_profile
                .as_ref()
                .ok_or_else(|| invalid("recorded installation does not use a user profile"))?;
            if source.path != recorded.path
                || (!source.digest.is_empty() && source.digest != recorded.digest)
            {
                return Err(invalid("uninstall user profile selection mismatch"));
            }
            source.digest = recorded.digest.clone();
            if request.installation.profile.is_empty() {
                request.installation.profile = receipt.installation.profile;
            }
        } else {
            let bytes = root
                .read(&source.path)?
                .ok_or_else(|| invalid("user profile is missing"))?;
            let actual = digest(&bytes);
            if !source.digest.is_empty() && source.digest != actual {
                return Err(invalid("user profile changed since planning"));
            }
            let profile = project_verification::parse_profile(
                std::str::from_utf8(&bytes).map_err(|_| invalid("user profile must be UTF-8"))?,
            )
            .map_err(|e| invalid(e.to_string()))?;
            if profile.schema_version != 2 {
                return Err(invalid(
                    "user profile installation requires project schema 2",
                ));
            }
            if !request.installation.profile.is_empty()
                && request.installation.profile != profile.id
            {
                return Err(invalid("user profile family does not match selection"));
            }
            request.installation.profile = profile.id.clone();
            source.digest = actual;
            user_source = Some(profile);
        }
    }
    request.installation = templates::normalized(&request.installation)?;
    if root.exists(TRANSACTION)? {
        return Err(invalid("unfinished transaction; recover before planning"));
    }
    let mut result = Plan {
        schema_version: 1,
        request,
        target_identity: root.identity,
        inputs: BTreeMap::new(),
        directories: BTreeMap::new(),
        files: vec![],
        conflicts: vec![],
        prerequisites: vec![],
    };
    let old_bytes = observe(&root, &mut result, MANIFEST)?;
    let old = match old_bytes.as_deref() {
        Some(bytes) => owned(bytes)?,
        None => BTreeMap::new(),
    };
    let installation = result.request.installation.clone();
    if result.request.operation == Operation::Install {
        if let Some(source) = &installation.user_profile {
            observe(&root, &mut result, &source.path)?;
        }
    }
    let mut new = match result.request.operation {
        Operation::Install => templates::render(&installation)?,
        Operation::Uninstall => {
            if old.is_empty() {
                return Err(invalid("no managed installation to uninstall"));
            }
            let receipt: Manifest = serde_json::from_slice(old_bytes.as_deref().unwrap())?;
            if receipt.installation != installation {
                return Err(invalid(
                    "uninstall selection must match the recorded installation",
                ));
            }
            BTreeMap::new()
        }
    };
    if result.request.operation == Operation::Install {
        let receipt = json(&manifest(&installation, &new))?;
        new.insert(MANIFEST.into(), receipt);
        let profile = match user_source {
            Some(profile) => profile,
            None => {
                if installation.release != "1.1.0" {
                    detect(&root, &mut result, &installation)?;
                }
                templates::profile(&installation)?
            }
        };
        if profile.selection.is_some() {
            detect_selected(&root, &mut result, &profile, &new)?;
        }
        for sensor in &profile.sensors {
            let component = profile
                .components
                .iter()
                .find(|c| c.id == sensor.component)
                .unwrap();
            for input in &sensor.inputs {
                let path = join(&component.root, input);
                if observe(&root, &mut result, &path)?.is_none() && !new.contains_key(&path) {
                    result.prerequisites.push(path);
                }
            }
            for pin in &sensor.pins {
                let path = join(&component.root, &pin.path);
                if let Some(bytes) = observe(&root, &mut result, &path)? {
                    let value = serde_json::from_slice::<serde_json::Value>(&bytes).ok();
                    if value
                        .as_ref()
                        .and_then(|v| v.pointer(&pin.pointer))
                        .and_then(|v| v.as_str())
                        != Some(pin.expected.as_str())
                    {
                        result.conflicts.push(format!(
                            "incompatible package/runtime pin: {path} {} requires {}",
                            pin.pointer, pin.expected
                        ));
                    }
                }
            }
        }
    }
    let paths: BTreeSet<_> = old.keys().chain(new.keys()).cloned().collect();
    for path in paths {
        // Inspect all parent components even if the final file does not exist.
        let mut parent = path.as_str();
        while let Some((prefix, _)) = parent.rsplit_once('/') {
            directory(&root, &mut result, prefix)?;
            parent = prefix;
        }
        let current = observe(&root, &mut result, &path)?;
        let before = current.as_deref().map(digest);
        let expected = old.get(&path).map(|c| digest(c.as_bytes()));
        let after = new.get(&path).map(|c| digest(c.as_bytes()));
        let action = if current.is_some() && (expected.is_none() || before != expected) {
            result
                .conflicts
                .push(format!("unowned or locally edited file: {path}"));
            Action::Conflict
        } else if before == after {
            Action::Unchanged
        } else if after.is_none() {
            Action::Remove
        } else if before.is_none() {
            Action::Add
        } else {
            Action::Update
        };
        result.files.push(Change {
            path: path.clone(),
            action,
            before_digest: before,
            after_digest: after,
            content: new.get(&path).cloned(),
            previous: if matches!(action, Action::Update | Action::Remove) {
                old.get(&path).cloned()
            } else {
                None
            },
        });
    }
    directory(&root, &mut result, ".")?;
    result.conflicts.sort();
    result.conflicts.dedup();
    result.prerequisites.sort();
    result.prerequisites.dedup();
    Ok(result)
}
