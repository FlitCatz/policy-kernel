use crate::{fs::relative, invalid, json, Installation, Result};
use project_verification::ProjectProfile;
use std::collections::BTreeMap;

pub(crate) const MANIFEST: &str = ".policy/bootstrap-manifest.json";
pub(crate) const TRANSACTION: &str = ".policy-bootstrap-transaction";

// Keep each release's bytes available for receipt validation and recovery.
macro_rules! release_asset {
    ($installation:expr, $path:literal) => {
        match $installation.release.as_str() {
            "1.0.0" => include_str!(concat!("../templates/1.0.0/", $path)),
            "1.0.1" => include_str!(concat!("../templates/1.0.1/", $path)),
            "1.0.2" => include_str!(concat!("../templates/1.0.2/", $path)),
            "1.1.0" => include_str!(concat!("../templates/1.1.0/", $path)),
            _ => return Err(invalid("unsupported template release")),
        }
    };
}

pub(crate) fn join(root: &str, file: &str) -> String {
    if root == "." {
        file.into()
    } else {
        format!("{root}/{file}")
    }
}

pub(crate) fn profile(installation: &Installation) -> Result<ProjectProfile> {
    if installation.user_profile.is_some() {
        return Err(invalid("user profile must be loaded from the plan input"));
    }
    if !matches!(
        installation.release.as_str(),
        "1.0.0" | "1.0.1" | "1.0.2" | "1.1.0"
    ) {
        return Err(invalid(
            "unsupported template release; expected 1.0.0, 1.0.1, 1.0.2 or 1.1.0",
        ));
    }
    let source = match installation.profile.as_str() {
        "react-typescript" => {
            release_asset!(installation, "profiles/react-typescript/profile.json")
        }
        "dotnet" => release_asset!(installation, "profiles/dotnet/profile.json"),
        "fullstack" => release_asset!(installation, "profiles/fullstack/profile.json"),
        _ => return Err(invalid("unsupported profile")),
    };
    let mut profile =
        project_verification::parse_profile(source).map_err(|e| invalid(e.to_string()))?;
    for (id, path) in &installation.components {
        if !relative(path, true)
            || path.split('/').any(|p| {
                matches!(
                    p.to_ascii_lowercase().as_str(),
                    ".git" | ".policy" | TRANSACTION
                )
            })
        {
            return Err(invalid(format!("unsafe component path: {path}")));
        }
        let component = profile
            .components
            .iter_mut()
            .find(|c| &c.id == id)
            .ok_or_else(|| invalid(format!("unknown component: {id}")))?;
        component.root = path.clone();
    }
    if installation.profile == "fullstack" {
        let frontend = &profile
            .components
            .iter()
            .find(|c| c.id == "frontend")
            .unwrap()
            .root;
        let backend = &profile
            .components
            .iter()
            .find(|c| c.id == "backend")
            .unwrap()
            .root;
        let frontend = frontend.to_ascii_lowercase();
        let backend = backend.to_ascii_lowercase();
        if frontend == "."
            || backend == "."
            || frontend == backend
            || frontend.starts_with(&format!("{backend}/"))
            || backend.starts_with(&format!("{frontend}/"))
        {
            return Err(invalid(
                "fullstack application component roots must be disjoint",
            ));
        }
        let shared = &profile
            .components
            .iter()
            .find(|c| c.id == "shared")
            .unwrap()
            .root;
        let shared = shared.to_ascii_lowercase();
        if shared != "."
            && (shared == frontend
                || shared == backend
                || shared.starts_with(&format!("{frontend}/"))
                || shared.starts_with(&format!("{backend}/"))
                || frontend.starts_with(&format!("{shared}/"))
                || backend.starts_with(&format!("{shared}/")))
        {
            return Err(invalid(
                "shared root must be the checkout root or a disjoint directory",
            ));
        }
    }
    profile.validate().map_err(|e| invalid(e.to_string()))?;
    Ok(profile)
}

pub(crate) fn normalized(installation: &Installation) -> Result<Installation> {
    if let Some(source) = &installation.user_profile {
        validate_user_path(&source.path)?;
        if !matches!(installation.release.as_str(), "1.1.0" | "1.2.0" | "1.3.0")
            || !matches!(
                installation.profile.as_str(),
                "react-typescript" | "dotnet" | "fullstack"
            )
            || (installation.release == "1.3.0" && installation.profile != "react-typescript")
            || !installation.components.is_empty()
            || source.digest.len() != 64
            || !source
                .digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid(
                "invalid user profile selection; use the matching custom release without component overrides",
            ));
        }
        return Ok(installation.clone());
    }
    let profile = profile(installation)?;
    Ok(Installation {
        profile: installation.profile.clone(),
        release: installation.release.clone(),
        user_profile: None,
        components: profile
            .components
            .into_iter()
            .map(|c| (c.id, c.root))
            .collect(),
    })
}

pub(crate) fn render(installation: &Installation) -> Result<BTreeMap<String, String>> {
    if installation.user_profile.is_some() {
        normalized(installation)?;
        if installation.release == "1.3.0" {
            return Ok(BTreeMap::from([
                (
                    ".policy/USAGE.md".into(),
                    include_str!("../templates/1.3.0/USER-PROFILE.md").into(),
                ),
                (
                    ".policy/ci/verification.yml".into(),
                    include_str!("../templates/1.3.0/verification.yml").into(),
                ),
                (
                    ".policy/ci/policy-action.yml".into(),
                    include_str!("../templates/1.3.0/action/action.yml").into(),
                ),
                (
                    ".policy/ci/policy-action-README.md".into(),
                    include_str!("../templates/1.3.0/action/README.md").into(),
                ),
            ]));
        }
        if installation.release == "1.2.0" {
            return Ok(BTreeMap::from([
                (
                    ".policy/USAGE.md".into(),
                    include_str!("../templates/1.2.0/USER-PROFILE.md").into(),
                ),
                (
                    ".policy/ci/verification.yml".into(),
                    include_str!("../templates/1.2.0/verification.yml").into(),
                ),
                (
                    ".policy/ci/policy-action.yml".into(),
                    include_str!("../templates/1.2.0/action/action.yml").into(),
                ),
                (
                    ".policy/ci/policy-action-README.md".into(),
                    include_str!("../templates/1.2.0/action/README.md").into(),
                ),
            ]));
        }
        // These exact static bytes and locations are the complete ownership
        // inventory. User input can never supply owned bytes or destinations.
        return Ok(BTreeMap::from([
            (
                ".policy/USAGE.md".into(),
                include_str!("../templates/1.1.0/USER-PROFILE.md").into(),
            ),
            (
                ".policy/ci/verification.yml".into(),
                release_asset!(installation, "verification.yml").into(),
            ),
            (
                ".policy/ci/policy-action.yml".into(),
                release_asset!(installation, "action/action.yml").into(),
            ),
            (
                ".policy/ci/policy-action-README.md".into(),
                release_asset!(installation, "action/README.md").into(),
            ),
        ]));
    }
    let profile = profile(installation)?;
    let mut files = BTreeMap::from([
        (".policy/project-profile.json".into(), json(&profile)?),
        (
            ".policy/sensor-profile.json".into(),
            json(&profile.sensor_profile())?,
        ),
        (
            ".policy/USAGE.md".into(),
            release_asset!(installation, "USAGE.md").into(),
        ),
        (
            ".policy/ci/verification.yml".into(),
            release_asset!(installation, "verification.yml").into(),
        ),
        (
            ".policy/ci/policy-action.yml".into(),
            release_asset!(installation, "action/action.yml").into(),
        ),
        (
            ".policy/ci/policy-action-README.md".into(),
            release_asset!(installation, "action/README.md").into(),
        ),
        (
            ".editorconfig".into(),
            release_asset!(installation, "editorconfig").into(),
        ),
    ]);
    for component in &profile.components {
        let templates: &[(&str, &str)] = match component.id.as_str() {
            "frontend" => &[
                (".node-version", "22.18.0\n"),
                (".npmrc", "engine-strict=true\naudit=true\n"),
                (
                    "tsconfig.json",
                    release_asset!(installation, "tsconfig.json"),
                ),
                (
                    "eslint.config.mjs",
                    release_asset!(installation, "eslint.config.mjs"),
                ),
                (
                    "vite.config.ts",
                    release_asset!(installation, "vite.config.ts"),
                ),
                (
                    "vitest.config.ts",
                    release_asset!(installation, "vitest.config.ts"),
                ),
            ],
            "backend" => &[
                ("global.json", release_asset!(installation, "global.json")),
                (
                    "Directory.Build.props",
                    release_asset!(installation, "Directory.Build.props"),
                ),
                ("NuGet.Config", release_asset!(installation, "NuGet.Config")),
            ],
            "shared" => &[
                (".node-version", "22.18.0\n"),
                (".npmrc", "engine-strict=true\naudit=true\n"),
                (
                    "vitest.contract.config.ts",
                    release_asset!(installation, "vitest.contract.config.ts"),
                ),
                (
                    "playwright.config.ts",
                    release_asset!(installation, "playwright.config.ts"),
                ),
            ],
            _ => return Err(invalid("unsupported component")),
        };
        for (path, content) in templates {
            let path = join(&component.root, path);
            if files.insert(path.clone(), (*content).into()).is_some() {
                return Err(invalid(format!("template path collision: {path}")));
            }
        }
    }
    // Also reject aliases on case-insensitive consumer filesystems and planned
    // files that another template would need to use as a directory.
    let mut folded = std::collections::BTreeSet::new();
    for path in files.keys() {
        if !folded.insert(path.to_ascii_lowercase()) {
            return Err(invalid("case-insensitive template path collision"));
        }
    }
    for path in &folded {
        let mut current = path.as_str();
        while let Some((parent, _)) = current.rsplit_once('/') {
            if folded.contains(parent) {
                return Err(invalid("template file/directory collision"));
            }
            current = parent;
        }
    }
    Ok(files)
}

pub(crate) fn validate_user_path(path: &str) -> Result<()> {
    if !relative(path, false)
        || path.split('/').any(|part| {
            matches!(
                part.to_ascii_lowercase().as_str(),
                ".git" | ".policy" | TRANSACTION
            )
        })
    {
        return Err(invalid(
            "user profile must be a checkout-relative file outside installer-owned directories",
        ));
    }
    Ok(())
}
