use crate::Result;
use policy_bootstrap::{apply, plan, Installation, Operation, Request};
use std::{collections::BTreeMap, fs, path::Path};

fn copy_tree(source: &Path, target: &Path) -> Result<()> {
    if !fs::symlink_metadata(source)?.is_dir() {
        return Err(format!("fixture is not a real directory: {}", source.display()).into());
    }
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        if matches!(
            name.to_str(),
            Some(
                "node_modules"
                    | "bin"
                    | "obj"
                    | "dist"
                    | ".git"
                    | ".policy-reports"
                    | "test-results"
            )
        ) {
            continue;
        }
        let kind = entry.file_type()?;
        let destination = target.join(name);
        if kind.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else if kind.is_file() {
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)?;
            std::io::copy(&mut fs::File::open(entry.path())?, &mut output)?;
        } else {
            return Err(format!(
                "fixture contains a symlink or special file: {}",
                entry.path().display()
            )
            .into());
        }
    }
    Ok(())
}

/// Creates a fresh consumer, composes fullstack sources, and applies the actual installer.
/// Existing destinations are refused, including symlinks. Partial failed staging is retained.
pub fn stage(examples: &Path, profile: &str, target: &Path) -> Result<()> {
    if !matches!(profile, "react-typescript" | "dotnet" | "fullstack") {
        return Err("unknown consuming profile".into());
    }
    outside_examples(examples, target)?;
    fs::create_dir(target)?;
    // Bootstrap validates all destination ancestors before any example content is copied.
    let request = Request {
        target: target.to_path_buf(),
        installation: Installation {
            profile: profile.into(),
            release: "1.0.1".into(),
            components: BTreeMap::new(),
        },
        operation: Operation::Install,
    };
    plan(request.clone())?;
    copy_tree(&examples.join(profile), target)?;
    if profile == "fullstack" {
        copy_tree(&examples.join("react-typescript"), &target.join("frontend"))?;
        copy_tree(&examples.join("dotnet"), &target.join("backend"))?;
    }
    let installation = plan(request)?;
    if !installation.conflicts.is_empty() {
        return Err(format!("bootstrap conflicts: {:?}", installation.conflicts).into());
    }
    apply(&installation)?;
    fs::write(
        target.join(".gitignore"),
        "node_modules/\nbin/\nobj/\ndist/\n.policy-reports/\ntest-results/\nplaywright-report/\n",
    )?;
    Ok(())
}

pub(crate) fn outside_examples(examples: &Path, target: &Path) -> Result<()> {
    let examples = examples.canonicalize()?;
    let parent = target
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if parent.canonicalize()?.starts_with(examples) {
        return Err("acceptance output must be outside the example source tree".into());
    }
    Ok(())
}
