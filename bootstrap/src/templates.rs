use crate::{fs::relative, invalid, json, Installation, Result};
use project_verification::ProjectProfile;
use std::collections::BTreeMap;

pub(crate) const MANIFEST: &str = ".policy/bootstrap-manifest.json";
pub(crate) const TRANSACTION: &str = ".policy-bootstrap-transaction";

pub(crate) fn join(root: &str, file: &str) -> String {
    if root == "." {
        file.into()
    } else {
        format!("{root}/{file}")
    }
}

pub(crate) fn profile(installation: &Installation) -> Result<ProjectProfile> {
    if installation.release != "1.0.0" {
        return Err(invalid(
            "unsupported template release; expected pinned 1.0.0",
        ));
    }
    let source = match installation.profile.as_str() {
        "react-typescript" => {
            include_str!("../templates/1.0.0/profiles/react-typescript/profile.json")
        }
        "dotnet" => include_str!("../templates/1.0.0/profiles/dotnet/profile.json"),
        "fullstack" => include_str!("../templates/1.0.0/profiles/fullstack/profile.json"),
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
    let profile = profile(installation)?;
    Ok(Installation {
        profile: installation.profile.clone(),
        release: installation.release.clone(),
        components: profile
            .components
            .into_iter()
            .map(|c| (c.id, c.root))
            .collect(),
    })
}

pub(crate) fn render(installation: &Installation) -> Result<BTreeMap<String, String>> {
    let profile = profile(installation)?;
    let mut files = BTreeMap::from([
        (".policy/project-profile.json".into(), json(&profile)?),
        (
            ".policy/sensor-profile.json".into(),
            json(&profile.sensor_profile())?,
        ),
        (
            ".policy/USAGE.md".into(),
            include_str!("../templates/1.0.0/USAGE.md").into(),
        ),
        (
            ".policy/ci/verification.yml".into(),
            include_str!("../templates/1.0.0/verification.yml").into(),
        ),
        (
            ".policy/ci/policy-action.yml".into(),
            include_str!("../templates/1.0.0/action/action.yml").into(),
        ),
        (
            ".policy/ci/policy-action-README.md".into(),
            include_str!("../templates/1.0.0/action/README.md").into(),
        ),
        (
            ".editorconfig".into(),
            include_str!("../templates/1.0.0/editorconfig").into(),
        ),
    ]);
    for component in &profile.components {
        let templates: &[(&str, &str)] = match component.id.as_str() {
            "frontend" => &[
                (".node-version", "22.18.0\n"),
                (".npmrc", "engine-strict=true\naudit=true\n"),
                (
                    "tsconfig.json",
                    include_str!("../templates/1.0.0/tsconfig.json"),
                ),
                (
                    "eslint.config.mjs",
                    include_str!("../templates/1.0.0/eslint.config.mjs"),
                ),
                (
                    "vite.config.ts",
                    include_str!("../templates/1.0.0/vite.config.ts"),
                ),
                (
                    "vitest.config.ts",
                    include_str!("../templates/1.0.0/vitest.config.ts"),
                ),
            ],
            "backend" => &[
                (
                    "global.json",
                    include_str!("../templates/1.0.0/global.json"),
                ),
                (
                    "Directory.Build.props",
                    include_str!("../templates/1.0.0/Directory.Build.props"),
                ),
                (
                    "NuGet.Config",
                    include_str!("../templates/1.0.0/NuGet.Config"),
                ),
            ],
            "shared" => &[
                (".node-version", "22.18.0\n"),
                (".npmrc", "engine-strict=true\naudit=true\n"),
                (
                    "vitest.contract.config.ts",
                    include_str!("../templates/1.0.0/vitest.contract.config.ts"),
                ),
                (
                    "playwright.config.ts",
                    include_str!("../templates/1.0.0/playwright.config.ts"),
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
