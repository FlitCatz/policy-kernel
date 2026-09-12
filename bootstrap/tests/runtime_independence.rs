#![cfg(unix)]
#[path = "../../tests/support/isolated_runtime.rs"]
mod isolated;
use isolated::{success, Runtime};
use std::{fs, path::Path};

#[test]
fn runtime_independence_installs_and_uninstalls_without_frameworks_or_rust() {
    let runtime = Runtime::new(Path::new(env!("CARGO_BIN_EXE_policy-bootstrap")));
    for uninstall in [false, true] {
        let mut command = runtime.command();
        command
            .args([
                "plan",
                "--profile",
                "fullstack",
                "--release",
                "1.0.2",
                "--target",
            ])
            .arg(&runtime.project);
        if uninstall {
            command.arg("--uninstall");
        }
        let plan = success(command.output().unwrap());
        assert_eq!(plan["conflicts"], serde_json::json!([]));
        let input = runtime.root.join("plan.json");
        fs::write(&input, serde_json::to_vec(&plan).unwrap()).unwrap();
        success(
            runtime
                .command()
                .args(["apply", "--plan"])
                .arg(&input)
                .output()
                .unwrap(),
        );
        assert_eq!(
            runtime
                .project
                .join(".policy/bootstrap-manifest.json")
                .is_file(),
            !uninstall
        );
        if !uninstall {
            let profile: serde_json::Value = serde_json::from_slice(
                &fs::read(runtime.project.join(".policy/project-profile.json")).unwrap(),
            )
            .unwrap();
            assert_eq!(profile["sensors"].as_array().unwrap().len(), 15);
        }
        runtime.assert_no_agent_state();
    }
}
