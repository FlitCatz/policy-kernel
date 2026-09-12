#![cfg(unix)]
#[path = "../../../tests/support/isolated_runtime.rs"]
mod isolated;
use isolated::{success, Runtime};
use std::{fs, path::Path};

#[test]
fn runtime_independence_evaluate_and_inspect_without_frameworks_or_rust() {
    let runtime = Runtime::new(Path::new(env!("CARGO_BIN_EXE_policy-cli")));
    let input = runtime.project.join("input.json");
    fs::write(
        &input,
        include_bytes!("../../../tests/fixtures/pre-merge/ready.json"),
    )
    .unwrap();
    let decision = success(
        runtime
            .command()
            .arg("evaluate-pre")
            .arg(&input)
            .output()
            .unwrap(),
    );
    assert_eq!(decision["status"], "ready_to_merge");
    assert_eq!(decision["mode"], "advisory");
    assert_eq!(decision["advisory"], true);
    fs::write(&input, serde_json::to_vec(&decision).unwrap()).unwrap();
    assert_eq!(
        success(
            runtime
                .command()
                .arg("inspect")
                .arg(&input)
                .output()
                .unwrap()
        ),
        decision
    );

    fs::write(
        &input,
        include_bytes!("../../../tests/fixtures/evidence/valid.json"),
    )
    .unwrap();
    success(
        runtime
            .command()
            .arg("inspect")
            .arg(&input)
            .output()
            .unwrap(),
    );
    fs::write(
        &input,
        include_bytes!("../../../tests/fixtures/evidence/tampered.json"),
    )
    .unwrap();
    let rejected = runtime
        .command()
        .arg("inspect")
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(rejected.status.code(), Some(64));
    assert!(rejected.stdout.is_empty());
    runtime.assert_no_agent_state();
}
