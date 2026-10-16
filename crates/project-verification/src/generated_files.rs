//! Narrow, opt-in compiler output contract for reviewed NuGet projects.
use crate::{consumer::ConsumerContract, invalid, selection::*, Error, ProjectProfile};
use std::{collections::BTreeSet, path::Path};

pub(crate) fn is_property(name: &str) -> bool {
    [
        "EmitCompilerGeneratedFiles",
        "CompilerGeneratedFilesOutputPath",
    ]
    .iter()
    .any(|p| name.eq_ignore_ascii_case(p))
}

pub(crate) fn validate_xml(doc: &roxmltree::Document<'_>, enabled: bool) -> Result<(), Error> {
    let properties: Vec<_> = doc
        .descendants()
        .filter(|n| n.is_element() && is_property(n.tag_name().name()))
        .collect();
    if !enabled {
        return if properties.is_empty() {
            Ok(())
        } else {
            Err(invalid("generated output properties require a reviewed project opt-in; props inheritance is forbidden"))
        };
    }
    if properties.len() != 2
        || properties[0].parent() != properties[1].parent()
        || !properties[0].parent().is_some_and(|p| {
            p.has_tag_name("PropertyGroup") && p.parent() == Some(doc.root_element())
        })
        || properties.iter().any(|n| {
            n.attributes().len() != 0
                // MSBuild concatenates text separated by comments. Requiring a
                // single text node prevents a safe prefix hiding a path suffix.
                || n.children().count() != 1
                || !n.first_child().is_some_and(|c| c.is_text())
                || n.ancestors().any(|a| {
                    a.attributes()
                        .any(|a| a.name().eq_ignore_ascii_case("Condition"))
                })
        })
        || !properties.iter().any(|n| {
            n.tag_name().name() == "EmitCompilerGeneratedFiles" && n.text() == Some("true")
        })
        || !properties.iter().any(|n| {
            n.tag_name().name() == "CompilerGeneratedFilesOutputPath"
                && n.text() == Some("obj/generated")
        })
    {
        return Err(invalid("generated output requires one unconditional project PropertyGroup with exactly true and obj/generated"));
    }
    Ok(())
}

pub(crate) fn validate_contract(
    consumer: &ConsumerContract,
    selection: &Selection,
    profile: &ProjectProfile,
) -> Result<(), Error> {
    let mut components = BTreeSet::new();
    for id in &consumer.generated_files_graphs {
        let graph = selection
            .graphs
            .iter()
            .find(|g| &g.id == id)
            .ok_or_else(|| invalid("generated output references an absent graph"))?;
        if graph.adapter != DependencyAdapter::Nuget || !consumer.framework_props.contains_key(id) {
            return Err(invalid(
                "generated output requires a NuGet graph with reviewed framework props",
            ));
        }
        components.insert(&graph.component);
    }
    for component in components {
        // Project references can build any graph in this component. Every one must
        // use the checked XML subset, and every command must preserve its values.
        if selection
            .graphs
            .iter()
            .filter(|g| &g.component == component)
            .any(|g| {
                g.adapter != DependencyAdapter::Nuget
                    || !consumer.framework_props.contains_key(&g.id)
            })
        {
            return Err(invalid(
                "generated output requires reviewed framework props for every component graph",
            ));
        }
        for sensor in profile.sensors.iter().filter(|s| &s.component == component) {
            let tool = selection.tool_for(sensor)?;
            if Path::new(&tool.executable)
                .file_name()
                .and_then(|n| n.to_str())
                != Some("dotnet")
                || tool.version_args != ["--version"]
                || sensor.environment_allowlist.iter().any(|key| {
                    !matches!(
                        key.as_str(),
                        "PATH" | "HOME" | "DOTNET_ROOT" | "DOTNET_CLI_HOME" | "NUGET_PACKAGES"
                    )
                })
            {
                return Err(invalid("generated output rejects external environment properties or version command overrides"));
            }
            if let Some(graph) = selection
                .graphs
                .iter()
                .find(|g| g.audit_sensor == sensor.definition.id.as_str())
            {
                if sensor.argv != graph.audit_argv(tool) {
                    return Err(invalid(
                        "generated output audit command differs from the fixed adapter",
                    ));
                }
                continue;
            }
            let args = &sensor.argv;
            if args.len() < 4
                || args[0] != tool.executable
                || !selection
                    .graphs
                    .iter()
                    .any(|g| &g.component == component && g.manifest == args[2])
                || args[3] != "--no-restore"
            {
                return Err(invalid("generated output requires a direct reviewed dotnet build/test command without restore"));
            }
            match args[1].as_str() {
                "build" if args.len() == 4 => {}
                "test" => {
                    let mut seen = BTreeSet::new();
                    let mut tail = args[4..].iter();
                    while let Some(flag) = tail.next() {
                        if !seen.insert(flag) {
                            return Err(invalid("duplicate generated output test option"));
                        }
                        match flag.as_str() {
                            "--no-build" => {},
                            "--filter" | "--logger" | "--results-directory" => {
                                let value = tail.next().ok_or_else(|| invalid("missing test option value"))?;
                                if value.is_empty() || value.starts_with(['-', '/', '@']) || value.contains(['\r', '\n']) {
                                    return Err(invalid("unsafe generated output test option value"));
                                }
                            },
                            _ => return Err(invalid("generated output rejects CLI properties, response files and external overrides")),
                        }
                    }
                }
                _ => return Err(invalid("unsupported generated output command")),
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_reference_overrides(doc: &roxmltree::Document<'_>) -> Result<(), Error> {
    for node in doc.descendants().filter(|n| n.is_element()) {
        if node
            .tag_name()
            .name()
            .eq_ignore_ascii_case("ProjectReference")
            && (node.attributes().any(|a| a.name() != "Include")
                || node.children().any(|c| c.is_element()))
        {
            return Err(invalid(
                "generated output rejects project reference property overrides",
            ));
        }
        if [
            "NuGetAuditSuppress",
            "NuGetLockFilePath",
            "NoWarn",
            "WarningsNotAsErrors",
        ]
        .iter()
        .any(|p| node.tag_name().name().eq_ignore_ascii_case(p))
        {
            return Err(invalid(
                "generated output rejects audit suppression and lock overrides",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_layout(
    consumer: &ConsumerContract,
    selection: &Selection,
    component: &str,
    root: &Path,
) -> Result<(), Error> {
    for graph in selection
        .graphs
        .iter()
        .filter(|g| g.component == component && consumer.generated_files_graphs.contains(&g.id))
    {
        let parent = Path::new(&graph.manifest)
            .parent()
            .ok_or_else(|| invalid("project has no directory"))?;
        let project_dir = crate::paths::directory(
            root,
            if parent.as_os_str().is_empty() {
                "."
            } else {
                parent
                    .to_str()
                    .ok_or_else(|| invalid("invalid project directory"))?
            },
        )?;
        let mut path = project_dir.clone();
        for part in ["obj", "generated"] {
            path.push(part);
            match std::fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
                Ok(_) => {
                    return Err(invalid(
                        "generated output and its parents must be directories without symlinks",
                    ))
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => break,
                Err(e) => return Err(e.into()),
            }
        }
        if path == project_dir.join("obj/generated") && path.exists() {
            check_output_tree(&path)?;
        }
    }
    if selection
        .graphs
        .iter()
        .any(|g| g.component == component && consumer.generated_files_graphs.contains(&g.id))
    {
        for graph in selection.graphs.iter().filter(|g| g.component == component) {
            let dir = root.join(Path::new(&graph.manifest).parent().unwrap());
            for ancestor in dir.ancestors() {
                for name in ["MSBuild.rsp", "Directory.Build.rsp"] {
                    match std::fs::symlink_metadata(ancestor.join(name)) {
                        Ok(_) => {
                            return Err(invalid(
                                "generated output rejects implicit MSBuild response files",
                            ))
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
    }
    Ok(())
}

fn check_output_tree(path: &Path) -> Result<(), Error> {
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() || !(kind.is_dir() || kind.is_file()) {
            return Err(invalid(
                "generated output contains a symlink or special file",
            ));
        }
        if kind.is_dir() {
            check_output_tree(&entry.path())?;
        }
    }
    Ok(())
}
