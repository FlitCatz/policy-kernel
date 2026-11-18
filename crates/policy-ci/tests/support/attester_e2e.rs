use std::ffi::OsString;

mod native_expected {
    include!("../../../policy-local-attest/tests/support/native_expected.rs");
}

fn compiled_attester() -> PathBuf {
    // Cargo exposes binary paths to tests only within a package. Build the
    // sibling CLI without adding a dependency from the policy boundary to it.
    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "build",
            "--locked",
            "--offline",
            "--jobs",
            "1",
            "--message-format=json",
            "-p",
            "policy-local-attest",
            "--bin",
            "policy-local-attest",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find_map(|event| {
            (event["target"]["name"] == "policy-local-attest")
                .then(|| event["executable"].as_str().map(PathBuf::from))
                .flatten()
        })
        .expect("Cargo must report the compiled attester executable")
}

fn run_with_owner_confirmation(attester: &Path, args: &[OsString], signing_input: &Path) -> String {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::process::{Command as Process, Stdio};

    let mut terminal = Process::new("/usr/bin/script");
    if cfg!(target_os = "macos") {
        terminal.args(["-q", "/dev/null"]).arg(attester).args(args);
    } else {
        // util-linux script takes a shell command, so quote each fixture argument.
        let command = std::iter::once(attester.as_os_str())
            .chain(args.iter().map(OsString::as_os_str))
            .map(|arg| format!("'{}'", arg.to_str().unwrap().replace('\'', "'\"'\"'")))
            .collect::<Vec<_>>()
            .join(" ");
        terminal.args(["-q", "-e", "-c", &command, "/dev/null"]);
    }
    let mut terminal = terminal
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = terminal.stdin.take().unwrap();
    let mut output = BufReader::new(terminal.stdout.take().unwrap());
    let mut transcript = String::new();
    loop {
        let mut line = String::new();
        let read = output.read_line(&mut line).unwrap();
        transcript.push_str(&line);
        assert!(
            read > 0,
            "attester exited before confirmation:\n{transcript}"
        );
        if line.contains("Type yes to authorize signing:") {
            break;
        }
    }
    assert!(
        !signing_input.exists(),
        "the signer ran before owner confirmation"
    );
    writeln!(input, "yes").unwrap();
    output.read_to_string(&mut transcript).unwrap();
    assert!(terminal.wait().unwrap().success(), "{transcript}");
    transcript
}

#[test]
fn issued_attestations_reach_ci_and_respect_required_paths() {
    let attester = compiled_attester();
    let helper = compile_process_fixture("attester_fixture", &["serde_json", "chrono", "base64"]);
    let ci_required = [
        ("dependencies", "Cargo.lock"),
        ("analysis-policy", "analysis/baseline.json"),
        ("hosted-review", "audit/review-bound-comments.md"),
        ("scheduled-check", ".github/workflows/weekly-check.yml"),
    ];
    let conditions: Vec<_> = ci_required
        .iter()
        .map(|(id, path)| json!({"id":id, "risks":[], "path_globs":[path]}))
        .collect();
    for path in std::iter::once("src/lib.rs").chain(ci_required.map(|(_, path)| path)) {
        let mut local = LocalFixture::for_ci(Fixture::with_change("ready", Some(path)));
        let workspace = local.fixture.ci.root.canonicalize().unwrap();
        fs::write(workspace.join("local-key-blob"), &local.key_blob).unwrap();
        let raw = &mut local.fixture.signed.observation;
        let input = &raw.local_attestations[0];
        let mut expected = serde_json::to_value(&input.expected_scope).unwrap();
        let mut run = serde_json::to_value(&input.observations.run).unwrap();
        native_expected::prepare(&mut expected, &mut run);
        let expected_scope: native::ExpectedScopeV1 =
            serde_json::from_value(expected.clone()).unwrap();
        fs::write(
            workspace.join("local-expected.json"),
            serde_json::to_vec(&expected).unwrap(),
        )
        .unwrap();
        fs::write(
            workspace.join("run-template.json"),
            serde_json::to_vec(&run).unwrap(),
        )
        .unwrap();
        let base = raw.evidence.manifest.base_sha.to_string();
        let openssl = &local.fixture.config.openssl;
        let arg = |flag: &str, value: &Path| [OsString::from(flag), value.as_os_str().to_owned()];
        let signer_arg = |value: &Path| {
            let mut flag = OsString::from("--signer-arg=");
            flag.push(value);
            flag
        };
        let output = workspace.join("attested");
        let mut args: Vec<OsString> = [
            "--base",
            &base,
            "--runner-id",
            "runner-test",
            "--run-id",
            "run-1",
            "--scope-id",
            "scope-1",
            "--signer-arg=sign",
        ]
        .into_iter()
        .map(OsString::from)
        .collect();
        args.extend(arg("--repo", &local.fixture.ci.checkout));
        args.extend(arg("--expected", &workspace.join("local-expected.json")));
        args.extend(arg("--run", &workspace.join("run.json")));
        args.extend(arg("--artifacts", &workspace.join("raw")));
        args.extend(arg("--output", &output));
        args.extend(arg("--signer", &helper));
        args.extend([signer_arg(&workspace), signer_arg(openssl)]);
        args.extend([
            "--".into(),
            helper.as_os_str().to_owned(),
            "run".into(),
            workspace.as_os_str().to_owned(),
            openssl.as_os_str().to_owned(),
            "literal spaces; and $metacharacters".into(),
        ]);
        let signing_input = workspace.join("attester-signing-input");
        let transcript = run_with_owner_confirmation(&attester, &args, &signing_input);
        let signing_input = fs::read(signing_input).unwrap();
        let sha256 = local_signing::openssl(
            openssl,
            &workspace,
            &["dgst", "-sha256", "-r"],
            &signing_input,
        );
        let sha256 = std::str::from_utf8(&sha256)
            .unwrap()
            .split_whitespace()
            .next()
            .unwrap();
        assert!(transcript.contains("Local scope Satisfied"), "{transcript}");
        assert!(
            transcript.contains(&format!("Statement SHA-256 (signing bytes) {sha256}")),
            "{transcript}"
        );

        let input = LocalAttestationInput {
            attestation: read_json(&output.join("attestation.json")).unwrap(),
            expected_scope: read_json(&output.join("expected.json")).unwrap(),
            observations: read_json(&output.join("observations.json")).unwrap(),
        };
        let evaluation = evaluate_scope(&input.expected_scope, &input.observations).unwrap();
        assert_eq!(evaluation.result.status, native::ScopeStatus::Satisfied);
        let evidence = evaluation.evidence.unwrap();
        *raw.evidence.executions.last_mut().unwrap() = evidence.executions[0].clone();
        *raw.evidence.sensors.last_mut().unwrap() = evidence.sensors[0].clone();
        raw.context.sensor_profile = evaluation.profile;
        raw.policy.required_sensor_ids = vec![SensorId::parse("unit").unwrap()];
        raw.policy.sensor_profile_digest =
            Some(policy_core::canonical_sha256(&raw.context.sensor_profile).unwrap());
        let tree = std::process::Command::new("git")
            .args(["rev-parse", "HEAD^{tree}"])
            .current_dir(&local.fixture.ci.checkout)
            .output()
            .unwrap();
        assert!(tree.status.success());
        raw.github_head_tree =
            Some(CommitSha::parse(std::str::from_utf8(&tree.stdout).unwrap().trim()).unwrap());
        raw.local_attestations = vec![input];
        let policy = raw.policy.local_runner_evidence.as_mut().unwrap();
        policy.approved_scope_template_digests =
            vec![expected_scope.template_digest().unwrap().sha256];
        policy.ci_required_when = serde_json::from_value(json!(conditions)).unwrap();
        local.sign_observation();

        let fixture = &mut local.fixture;
        let config = workspace.join("config.json");
        let signed = workspace.join("signed.json");
        fs::write(&config, canonical(&fixture.config).unwrap()).unwrap();
        fs::write(&signed, canonical(&fixture.signed).unwrap()).unwrap();
        fixture.ci.trusted = workspace.join("authenticated");
        let controller = Command::cargo_bin("policy-controller")
            .unwrap()
            .arg("--config")
            .arg(config)
            .arg("--checkout")
            .arg(&fixture.ci.checkout)
            .arg("--signed-observation")
            .arg(signed)
            .arg("--output")
            .arg(&fixture.ci.trusted)
            .output()
            .unwrap();
        assert!(
            controller.status.success(),
            "{path}: {}",
            String::from_utf8_lossy(&controller.stderr)
        );
        for name in ["policy.json", "evidence.json"] {
            fs::copy(fixture.ci.trusted.join(name), workspace.join(name)).unwrap();
        }

        let ordinary = path == "src/lib.rs";
        let evaluated = fixture.ci.command().output().unwrap();
        assert_eq!(
            evaluated.status.code(),
            Some(if ordinary { 0 } else { 1 }),
            "{path}: {}",
            String::from_utf8_lossy(&evaluated.stderr)
        );
        let published: Value = read_json(&workspace.join("published.json")).unwrap();
        let decision: Value = read_json(&fixture.ci.artifacts.join("decision.json")).unwrap();
        let reasons: Vec<_> = decision["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|reason| reason["code"].as_str().unwrap())
            .collect();
        if ordinary {
            assert_eq!(published["output"]["title"], "READY_TO_MERGE");
            assert!(reasons.contains(&"PK-PROV-LOCAL-ATTESTED"), "{decision}");
        } else {
            assert_ne!(published["output"]["title"], "READY_TO_MERGE", "{path}");
            assert!(
                reasons.contains(&"PK-PROV-CI-REQUIRED"),
                "{path}: {decision}"
            );
        }
    }
    fs::remove_file(helper).unwrap();
}
