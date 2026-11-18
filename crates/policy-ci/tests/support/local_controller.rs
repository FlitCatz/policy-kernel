mod local_runner {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use policy_native_verifier::{contracts as native, evaluate_scope, ImportedObservationsV1};

    mod local_signing {
        include!("local_signing.rs");
    }
    use local_signing::{string, NS, SK};

    include!("attester_e2e.rs");

    /// Software-only FIDO-format fixture, following OpenSSH PROTOCOL.sshsig and
    /// PROTOCOL.u2f. No authenticator, agent, private user key or device is used.
    struct LocalFixture {
        fixture: SignedFixture,
        key_blob: Vec<u8>,
    }

    impl LocalFixture {
        fn new() -> Self {
            Self::for_ci(Fixture::new("ready"))
        }

        fn for_ci(ci: Fixture) -> Self {
            let mut fixture = SignedFixture::for_ci(ci);
            let key = fixture.ci.root.join("local.key");
            assert!(std::process::Command::new(&fixture.config.openssl)
                .args(["genpkey", "-algorithm", "ED25519", "-out"])
                .arg(&key).output().unwrap().status.success());
            let out = std::process::Command::new(&fixture.config.openssl)
                .args(["pkey", "-in"]).arg(&key).args(["-pubout", "-outform", "DER"])
                .output().unwrap();
            assert!(out.status.success());
            assert_eq!(&out.stdout[..12], &[0x30, 0x2a, 0x30, 5, 6, 3, 0x2b, 0x65, 0x70, 3, 0x21, 0]);
            let mut key_blob = string(SK.as_bytes());
            key_blob.extend(string(&out.stdout[12..]));
            key_blob.extend(string(b"ssh:fixture"));
            let public = fixture.ci.root.join("local.pub");
            fs::write(&public, format!("runner-test namespaces=\"{NS}\" {SK} {}\n", STANDARD.encode(&key_blob))).unwrap();
            fixture.config.schema_version = 2;
            fixture.config.ssh_keygen = Some(PathBuf::from("/usr/bin/ssh-keygen"));
            fixture.config.local_runners = vec![LocalRunnerKey { runner_id: "runner-test".into(), public_key: public }];
            let corpus: Value = serde_json::from_str(include_str!("../../../policy-native-contracts/tests/corpus/v1.json")).unwrap();
            let mut expected: native::ExpectedScopeV1 = serde_json::from_value(corpus["valid"]["expected"].clone()).unwrap();
            let mut run: native::NativeRunV1 = serde_json::from_value(corpus["valid"]["run"].clone()).unwrap();
            let raw = &mut fixture.signed.observation;
            let now = raw.evidence.manifest.created_at.clone();
            let start = now.checked_sub(std::time::Duration::from_secs(10)).unwrap();
            expected.subject.repository = raw.binding.repository.clone();
            expected.subject.head = raw.binding.head_sha.clone();
            expected.subject.source_set_digest.sha256 = raw.evidence.manifest.source_set_digest.clone();
            expected.bindings.evaluated_at = now.clone();
            let mut bindings = expected.bindings.observations.as_slice().to_vec();
            bindings[0].command_digest = native::canonical_digest(native::DigestRole::Command, &run.observations[0].command).unwrap();
            expected.bindings.observations = native::RequiredSet::new(bindings).unwrap();
            run.subject.source = expected.subject.clone();
            run.observations[0].subject = run.subject.clone();
            run.observations[0].started_at = start.clone();
            run.observations[0].completed_at = now.clone();
            run.observations[0].observed_at = now.clone();
            let observations = ImportedObservationsV1 {
                schema_version: native::VersionV1::default(), expected_scope_digest: expected.digest().unwrap(),
                expected_run_id: run.subject.run_id.clone(), expected_scope_id: run.subject.scope_id.clone(),
                adapter_version: native::RecordId::parse("fixture/1").unwrap(), trust: native::LocalTrust::LocalUntrusted,
                review: native::ReviewGate::Unmet, unsuccessful_reports: vec![], run,
            };
            let evaluation = evaluate_scope(&expected, &observations).unwrap();
            assert_eq!(evaluation.result.status, native::ScopeStatus::Satisfied);
            let local = evaluation.evidence.unwrap();
            raw.evidence.executions.extend(local.executions);
            raw.evidence.sensors.extend(local.sensors);
            raw.inventory.executions = raw.evidence.executions.iter().map(|v| v.execution_id.clone()).collect();
            raw.inventory.sensors = raw.evidence.sensors.iter().map(|v| v.sensor_id.clone()).collect();
            raw.github_head_tree = Some(CommitSha::parse("2".repeat(40)).unwrap());
            let statement: native::LocalAttestationV1 = serde_json::from_value(json!({
                "schema_version":1, "domain":NS, "subject":expected.subject,
                "head_tree":raw.github_head_tree, "base":raw.evidence.manifest.base_sha,
                "run_id":observations.expected_run_id, "scope_id":observations.expected_scope_id,
                "expected_scope_digest":expected.digest().unwrap(), "scope_template_digest":expected.template_digest().unwrap(),
                "native_run_digest":observations.run.digest().unwrap(),
                "observations_digest":native::canonical_local_digest(native::LocalDigestRole::ImportedObservations, &observations).unwrap(),
                "result_digest":evaluation.result.digest().unwrap(),
                "runner":{"runner_id":"runner-test","os":"fixture","arch":"test","attestor_version":"fixture/1"},
                "started_at":start,"completed_at":now,"attested_at":now,
            })).unwrap();
            raw.local_attestations = vec![LocalAttestationInput {
                attestation: native::SignedLocalAttestationV1 { statement, signature: native::LocalSignatureV1 {
                    format: native::LocalSignatureFormat::Sshsig, runner_id: native::RecordId::parse("runner-test").unwrap(), value: native::RecordId::parse("pending").unwrap(),
                } }, expected_scope: expected, observations,
            }];
            raw.policy.schema_version = 4;
            raw.policy.local_runner_evidence = Some(serde_json::from_value(json!({
                "allowed_runner_ids":["runner-test"],
                "approved_scope_template_digests":[raw.local_attestations[0].expected_scope.template_digest().unwrap().sha256],
                "maximum_age_seconds":3600,"ci_required_when":[],
            })).unwrap());
            let mut local = Self { fixture, key_blob };
            local.sign(1, NS);
            local
        }

        fn sign(&mut self, flags: u8, namespace: &str) {
            let input = &self.fixture.signed.observation.local_attestations[0];
            let payload = canonical(&input.attestation.statement).unwrap();
            let key = local_signing::SoftwareKey {
                openssl: &self.fixture.config.openssl,
                root: &self.fixture.ci.root,
                blob: &self.key_blob,
            };
            let signature = key.sign(&payload, flags, namespace);
            self.fixture.signed.observation.local_attestations[0].attestation.signature.value =
                native::RecordId::parse(signature).unwrap();
            self.sign_observation();
        }

        fn sign_observation(&mut self) {
            let raw = &mut self.fixture.signed.observation;
            raw.policy.policy_digest = policy_core::pre_merge_policy_digest(&raw.policy).unwrap();
            raw.evidence.manifest.policy_digest = raw.policy.policy_digest.clone();
            raw.binding.policy_digest = raw.policy.policy_digest.clone();
            self.fixture.config.expected.policy_digest = raw.policy.policy_digest.clone();
            self.fixture.sign();
        }

        fn rehash(&mut self) {
            let input = &mut self.fixture.signed.observation.local_attestations[0];
            input.observations.expected_scope_digest = input.expected_scope.digest().unwrap();
            let s = &mut input.attestation.statement;
            s.expected_scope_digest = input.expected_scope.digest().unwrap();
            s.scope_template_digest = input.expected_scope.template_digest().unwrap();
            s.native_run_digest = input.observations.run.digest().unwrap();
            s.observations_digest = native::canonical_local_digest(native::LocalDigestRole::ImportedObservations, &input.observations).unwrap();
            s.result_digest = evaluate_scope(&input.expected_scope, &input.observations).unwrap().result.digest().unwrap();
            self.sign(1, NS);
        }
    }

    #[test]
    fn local_scope_is_reconstructed_and_only_covered_records_receive_the_weaker_class() {
        let mut local = LocalFixture::new();
        // Signed submitted assertions cannot replace the recomputed records.
        let raw = &mut local.fixture.signed.observation.evidence;
        raw.executions.last_mut().unwrap().output_digest = Sha256Digest::parse("0".repeat(64)).unwrap();
        raw.sensors.last_mut().unwrap().passed = Some(999);
        raw.test_adequacy.as_mut().unwrap().passed = 999;
        local.fixture.sign();
        local.fixture.produce().unwrap();
        let evidence: EvidenceBundle = read_json(&local.fixture.ci.root.join("authenticated/evidence.json")).unwrap();
        assert!(policy_core::validate_bundle(&evidence).is_valid());
        let input = &local.fixture.signed.observation.local_attestations[0];
        let recomputed = evaluate_scope(&input.expected_scope, &input.observations).unwrap().evidence.unwrap();
        assert_eq!(evidence.test_adequacy, recomputed.test_adequacy);
        assert_eq!(evidence.executions.last().unwrap().output_digest, recomputed.executions[0].output_digest);
        assert_eq!(evidence.sensors.last().unwrap().passed, Some(1));
        assert_eq!(evidence.executions.last().unwrap().trust, TrustClass::LocalRunnerAttested);
        assert_eq!(evidence.sensors.last().unwrap().trust, TrustClass::LocalRunnerAttested);
        assert!(evidence.executions[..evidence.executions.len()-1].iter().all(|e| e.trust == TrustClass::TrustedRunnerAttested));
        assert!(evidence.sensors[..evidence.sensors.len()-1].iter().all(|e| e.trust == TrustClass::TrustedRunnerAttested));
        assert!(evidence.modifications.iter().all(|e| e.trust == TrustClass::TrustedRunnerAttested));
        assert!(evidence.reviews.iter().all(|e| e.trust == TrustClass::TrustedRunnerAttested));
        assert!(evidence.knowledge.iter().all(|e| e.trust == TrustClass::TrustedRunnerAttested));
        fs::write(local.fixture.ci.root.join("local-config.json"), canonical(&local.fixture.config).unwrap()).unwrap();
        fs::write(local.fixture.ci.root.join("local-signed.json"), canonical(&local.fixture.signed).unwrap()).unwrap();
        let output = Command::cargo_bin("policy-controller").unwrap()
            .arg("--config").arg(local.fixture.ci.root.join("local-config.json"))
            .arg("--checkout").arg(&local.fixture.ci.checkout)
            .arg("--signed-observation").arg(local.fixture.ci.root.join("local-signed.json"))
            .arg("--output").arg(local.fixture.ci.root.join("local-cli")).output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(fs::read(local.fixture.ci.root.join("local-cli/evidence.json")).unwrap(), canonical(&evidence).unwrap());
    }

    #[test]
    fn signed_local_payload_domain_key_and_user_presence_are_verified() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.clone();
        local.fixture.signed.observation.local_attestations[0].attestation.statement.runner.os = native::RecordId::parse("tampered").unwrap();
        local.fixture.sign();
        local.fixture.reject("signature authentication failed");
        local.fixture.signed = original.clone();
        local.sign(1, "other-domain");
        local.fixture.reject("local signature domain mismatch");
        local.fixture.signed = original.clone();
        local.sign(0, NS);
        let input = &local.fixture.signed.observation.local_attestations[0];
        fs::write(local.fixture.ci.root.join("presence-payload"), canonical(&input.attestation.statement).unwrap()).unwrap();
        fs::write(local.fixture.ci.root.join("presence-signature"), input.attestation.signature.value.as_str()).unwrap();
        // OpenSSH alone accepts this cryptographically valid absent-presence
        // signature. K6 must reject it explicitly, without relying on key labels.
        let verified = std::process::Command::new("/usr/bin/ssh-keygen")
            .args(["-Y", "verify", "-I", "runner-test", "-n", NS, "-f"])
            .arg(&local.fixture.config.local_runners[0].public_key)
            .arg("-s").arg(local.fixture.ci.root.join("presence-signature"))
            .stdin(fs::File::open(local.fixture.ci.root.join("presence-payload")).unwrap())
            .output().unwrap();
        assert!(verified.status.success(), "{}", String::from_utf8_lossy(&verified.stderr));
        local.fixture.reject("local signature requires user presence");
        local.fixture.signed = original.clone();
        local.key_blob[40] ^= 1;
        local.sign(1, NS);
        local.fixture.reject("local signature key mismatch");
        local.fixture.signed = original;
        // The observer signature covers local inputs too.
        local.fixture.signed.observation.local_attestations[0].observations.adapter_version = native::RecordId::parse("tampered").unwrap();
        local.fixture.reject("signature authentication failed");
    }

    #[test]
    fn local_registry_and_policy_runner_allow_list_are_independent() {
        let mut local = LocalFixture::new();
        local.fixture.config.local_runners.clear();
        local.fixture.reject("local runner is not registered");
        let mut local = LocalFixture::new();
        local.fixture.signed.observation.policy.local_runner_evidence.as_mut().unwrap().allowed_runner_ids = vec!["other".into()];
        local.sign(1, NS);
        local.fixture.reject("local runner is not allowed by policy");
        let mut local = LocalFixture::new();
        local.fixture.signed.observation.policy.local_runner_evidence = None;
        local.sign(1, NS);
        local.fixture.reject("local runner requires policy v4 opt-in");
        let mut local = LocalFixture::new();
        local.fixture.config.schema_version = 1;
        local.fixture.reject("configuration/current-run binding mismatch");
    }

    #[test]
    fn local_configuration_fields_are_individually_versioned_and_verifier_paths_are_absolute() {
        let local = LocalFixture::new();
        let runner = local.fixture.config.local_runners[0].clone();
        let attestation = local.fixture.signed.observation.local_attestations[0].clone();
        for field in ["runner", "verifier", "tree", "attestation"] {
            let mut fixture = SignedFixture::new();
            match field {
                "runner" => fixture.config.local_runners.push(runner.clone()),
                "verifier" => fixture.config.ssh_keygen = Some(PathBuf::from("/usr/bin/ssh-keygen")),
                "tree" => fixture.signed.observation.github_head_tree = local.fixture.signed.observation.github_head_tree.clone(),
                _ => fixture.signed.observation.local_attestations.push(attestation.clone()),
            }
            fixture.sign();
            fixture.reject("controller configuration/current-run binding mismatch");
        }
        for verifier in [None, Some(PathBuf::from("ssh-keygen"))] {
            let mut fixture = SignedFixture::new();
            fixture.config.schema_version = 2;
            fixture.config.ssh_keygen = verifier;
            fixture.reject("controller configuration/current-run binding mismatch");
        }
    }

    #[test]
    fn local_scope_subject_must_match_the_signed_statement_even_when_native_inputs_agree() {
        let mut local = LocalFixture::new();
        let input = &mut local.fixture.signed.observation.local_attestations[0];
        input.expected_scope.subject.head = CommitSha::parse("9".repeat(40)).unwrap();
        input.observations.run.subject.source = input.expected_scope.subject.clone();
        input.observations.run.observations[0].subject = input.observations.run.subject.clone();
        local.rehash();
        local.fixture.reject("local subject/GitHub binding mismatch");
    }

    #[test]
    fn local_interval_checks_cover_interior_times_and_each_boundary_independently() {
        let mut local = LocalFixture::new();
        let now = local.fixture.signed.observation.evidence.manifest.created_at.clone();
        let time = |seconds| now.checked_sub(std::time::Duration::from_secs(seconds)).unwrap();
        let input = &mut local.fixture.signed.observation.local_attestations[0];
        input.attestation.statement.started_at = time(30);
        input.observations.run.observations[0].started_at = time(25);
        input.observations.run.observations[0].completed_at = time(20);
        input.observations.run.observations[0].observed_at = time(15);
        input.attestation.statement.completed_at = time(10);
        input.expected_scope.bindings.evaluated_at = time(5);
        input.attestation.statement.attested_at = time(0);
        local.rehash();
        let original = local.fixture.signed.clone();
        local.fixture.produce().unwrap();
        fs::remove_dir_all(local.fixture.ci.root.join("authenticated")).unwrap();
        for field in ["evaluation-before-completion", "evaluation-after-attestation", "start", "completion", "observation"] {
            local.fixture.signed = original.clone();
            let input = &mut local.fixture.signed.observation.local_attestations[0];
            match field {
                "evaluation-before-completion" => input.expected_scope.bindings.evaluated_at = time(11),
                "evaluation-after-attestation" => input.attestation.statement.attested_at = time(6),
                "start" => input.attestation.statement.started_at = time(24),
                "completion" => input.attestation.statement.completed_at = time(21),
                _ => input.observations.run.observations[0].observed_at = time(9),
            }
            local.rehash();
            local.fixture.reject("local observation is outside attested run interval");
        }
    }

    #[test]
    fn local_placeholders_use_neither_ci_freshness_nor_submitted_adequacy_bindings() {
        let mut local = LocalFixture::new();
        let raw = &mut local.fixture.signed.observation;
        let older = raw.evidence.manifest.created_at.checked_sub(std::time::Duration::from_secs(600)).unwrap();
        raw.policy.max_evidence_age_seconds = 60;
        let execution = raw.evidence.executions.last_mut().unwrap();
        execution.started_at = older.clone();
        execution.completed_at = older.clone();
        raw.evidence.sensors.last_mut().unwrap().observed_at = older;
        raw.evidence.test_adequacy.as_mut().unwrap().head_sha = None;
        local.sign(1, NS);
        local.fixture.produce().unwrap();
    }

    #[test]
    fn local_hardware_key_and_no_touch_rejections_have_distinct_reasons() {
        let mut local = LocalFixture::new();
        let path = local.fixture.config.local_runners[0].public_key.clone();
        let original = fs::read_to_string(&path).unwrap();
        fs::write(&path, original.replace("namespaces=", "no-touch-required,namespaces=")).unwrap();
        local.fixture.reject("local runner no-touch-required option is forbidden");
        fs::write(&path, original.replace(SK, "ssh-ed25519")).unwrap();
        local.fixture.reject("local runner key type is not hardware backed");
        for options in ["valid-before=\"20000101000000Z\"", "valid-after=\"20990101000000Z\"", "namespaces=\"other\""] {
            fs::write(&path, format!("runner-test {options} {SK} {}\n", STANDARD.encode(&local.key_blob))).unwrap();
            local.fixture.reject("signature authentication failed");
        }
        fs::write(&path, original).unwrap();
        local.fixture.config.local_runners.push(local.fixture.config.local_runners[0].clone());
        local.fixture.reject("local runner keys must be distinct and unambiguous");
    }

    #[test]
    fn local_attestations_cannot_replay_repository_head_tree_base_or_source() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.clone();
        for field in ["repository", "head", "tree", "base", "source", "run", "scope", "github-tree"] {
            local.fixture.signed = original.clone();
            let raw = &mut local.fixture.signed.observation;
            let s = &mut raw.local_attestations[0].attestation.statement;
            match field {
                "repository" => s.subject.repository = RepositoryId::parse("other/project").unwrap(),
                "head" => s.subject.head = CommitSha::parse("3".repeat(40)).unwrap(),
                "tree" => s.head_tree = CommitSha::parse("3".repeat(40)).unwrap(),
                "base" => s.base = CommitSha::parse("3".repeat(40)).unwrap(),
                "source" => s.subject.source_set_digest.sha256 = Sha256Digest::parse("3".repeat(64)).unwrap(),
                "run" => s.run_id = ExecutionId::parse("other").unwrap(),
                "scope" => s.scope_id = native::RecordId::parse("other").unwrap(),
                _ => raw.github_head_tree = None,
            }
            if matches!(field, "repository" | "head" | "source") {
                // Keep the entire signed native run internally consistent so
                // only the independent GitHub/source observation rejects it.
                let input = &mut raw.local_attestations[0];
                let subject = input.attestation.statement.subject.clone();
                input.expected_scope.subject = subject.clone();
                input.observations.run.subject.source = subject.clone();
                for observation in &mut input.observations.run.observations {
                    observation.subject.source = subject.clone();
                }
                local.rehash();
            } else {
                local.sign(1, NS);
            }
            local.fixture.reject("local subject/GitHub binding mismatch");
        }
    }

    #[test]
    fn local_templates_all_digests_and_scope_success_are_recomputed() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.clone();
        for field in ["expected", "template", "native", "observations", "result"] {
            local.fixture.signed = original.clone();
            let s = &mut local.fixture.signed.observation.local_attestations[0].attestation.statement;
            let digest = match field {
                "expected" => &mut s.expected_scope_digest.sha256,
                "template" => &mut s.scope_template_digest.sha256,
                "native" => &mut s.native_run_digest.sha256,
                "observations" => &mut s.observations_digest.sha256,
                _ => &mut s.result_digest.sha256,
            };
            *digest = Sha256Digest::parse("0".repeat(64)).unwrap();
            local.sign(1, NS);
            local.fixture.reject("local digest mismatch");
        }
        local.fixture.signed = original.clone();
        let input = &mut local.fixture.signed.observation.local_attestations[0];
        input.expected_scope.bindings.maximum_age_seconds += 1;
        local.rehash();
        local.fixture.reject("local scope template is not approved");
        for case in ["nonpass", "error"] {
            local.fixture.signed = original.clone();
            let input = &mut local.fixture.signed.observation.local_attestations[0];
            if case == "nonpass" { input.observations.run.mappings.clear(); }
            else { input.observations.run.observations[0].command.cwd = "other".into(); }
            local.rehash();
            local.fixture.reject("local scope is not satisfied");
        }
    }

    #[test]
    fn local_freshness_uses_controller_time_and_checks_run_interval() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.clone();
        local.fixture.signed.observation.policy.local_runner_evidence.as_mut().unwrap().maximum_age_seconds = 1;
        local.sign(1, NS);
        local.fixture.reject("local evidence is stale or future dated");
        for field in ["start", "completed", "attested", "evaluated"] {
            local.fixture.signed = original.clone();
            let input = &mut local.fixture.signed.observation.local_attestations[0];
            let future = input.attestation.statement.attested_at.checked_add(std::time::Duration::from_secs(86_400)).unwrap();
            match field {
                "start" => input.attestation.statement.started_at = future,
                "completed" => input.attestation.statement.completed_at = future,
                "attested" => input.attestation.statement.attested_at = future,
                _ => input.expected_scope.bindings.evaluated_at = future,
            }
            local.sign(1, NS);
            assert!(local.fixture.produce().is_err(), "{field}");
        }
        local.fixture.signed = original;
        let input = &mut local.fixture.signed.observation.local_attestations[0];
        input.attestation.statement.started_at = input.attestation.statement.completed_at.clone();
        local.sign(1, NS);
        local.fixture.reject("local observation is outside attested run interval");
    }

    #[test]
    fn local_coverage_is_complete_disjoint_and_cannot_authenticate_reviews_or_modifiers() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.clone();
        local.fixture.signed.observation.local_attestations.push(original.observation.local_attestations[0].clone());
        local.fixture.sign();
        local.fixture.reject("local coverage must be complete and disjoint");
        for field in ["execution", "sensor", "review", "modifier", "raised-trust", "extra-signature"] {
            local.fixture.signed = original.clone();
            let raw = &mut local.fixture.signed.observation;
            match field {
                "execution" => { raw.evidence.executions.pop(); raw.inventory.executions.pop(); },
                "sensor" => { raw.evidence.sensors.pop(); raw.inventory.sensors.pop(); },
                "review" | "modifier" => {
                    let id = if field == "review" { raw.evidence.reviews[0].execution_id.clone() } else { raw.evidence.modifications[0].execution_id.clone() };
                    raw.local_attestations[0].observations.run.observations[0].execution_id = id.clone();
                    raw.evidence.executions.last_mut().unwrap().execution_id = id;
                    raw.inventory.executions = raw.evidence.executions.iter().map(|e| e.execution_id.clone()).collect();
                    local.rehash();
                },
                "raised-trust" => raw.evidence.sensors.last_mut().unwrap().trust = TrustClass::LocalRunnerAttested,
                _ => {},
            }
            local.fixture.sign();
            if field == "extra-signature" {
                let mut sig = local.fixture.signed.producers[0].clone();
                sig.execution_id = ExecutionId::parse("exec-1").unwrap();
                local.fixture.signed.producers.push(sig);
            }
            assert!(local.fixture.produce().is_err(), "{field}");
        }
    }

    #[test]
    fn disjoint_local_scopes_combine_only_recomputed_adequacy_and_reject_requirement_overlap() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.observation.local_attestations[0].clone();
        let second = serde_json::to_string(&original).unwrap()
            .replace("\"unit\"", "\"unit-2\"")
            .replace("\"exec-1\"", "\"exec-2\"")
            .replace("\"run-1\"", "\"run-2\"")
            .replace("\"scope-1\"", "\"scope-2\"")
            .replace("\"AC-1\"", "\"AC-2\"")
            .replace("\"B-1\"", "\"B-2\"")
            .replace("\"N-1\"", "\"N-2\"");
        local.fixture.signed.observation.local_attestations[0] = serde_json::from_str(&second).unwrap();
        local.rehash();
        let raw = &mut local.fixture.signed.observation;
        let input = &raw.local_attestations[0];
        let second_evidence = evaluate_scope(&input.expected_scope, &input.observations).unwrap().evidence.unwrap();
        raw.policy.local_runner_evidence.as_mut().unwrap().approved_scope_template_digests.push(input.expected_scope.template_digest().unwrap().sha256);
        raw.evidence.executions.extend(second_evidence.executions);
        raw.evidence.sensors.extend(second_evidence.sensors);
        raw.inventory.executions = raw.evidence.executions.iter().map(|e| e.execution_id.clone()).collect();
        raw.inventory.sensors = raw.evidence.sensors.iter().map(|e| e.sensor_id.clone()).collect();
        raw.local_attestations.push(original);
        local.sign(1, NS);
        local.fixture.produce().unwrap();
        let evidence: EvidenceBundle = read_json(&local.fixture.ci.root.join("authenticated/evidence.json")).unwrap();
        let adequacy = evidence.test_adequacy.unwrap();
        assert_eq!((adequacy.discovered, adequacy.executed, adequacy.passed), (2, 2, 2));
        assert_eq!(adequacy.acceptance_criteria.len(), 2);
        assert_eq!(adequacy.changed_behaviors.len(), 4);
        assert_eq!(adequacy.negative_case_ids.len(), 2);
        assert_eq!(adequacy.executed_test_ids.len(), 2);
        let raw = &mut local.fixture.signed.observation;
        let input = &mut raw.local_attestations[0];
        let expected = serde_json::to_string(&input.expected_scope).unwrap().replace("\"AC-2\"", "\"AC-1\"");
        input.expected_scope = serde_json::from_str(&expected).unwrap();
        input.observations.run.mappings[0].requirement_id = native::RecordId::parse("AC-1").unwrap();
        raw.policy.local_runner_evidence.as_mut().unwrap().approved_scope_template_digests.push(input.expected_scope.template_digest().unwrap().sha256);
        local.rehash();
        let error = produce(&local.fixture.config, &local.fixture.signed, &local.fixture.ci.root.join("overlap")).unwrap_err();
        assert!(error.to_string().contains("local adequacy requirements overlap"));
    }

    #[test]
    fn malformed_local_signature_encodings_fail_before_receipt_creation() {
        let mut local = LocalFixture::new();
        let original = local.fixture.signed.clone();
        for signature in ["opaque".to_owned(), "x".repeat(16385),
            "-----BEGIN SSH SIGNATURE-----\nAAAA\n-----END SSH SIGNATURE-----".into(),
            "-----BEGIN SSH SIGNATURE-----\n!!!!\n-----END SSH SIGNATURE-----".into()] {
            local.fixture.signed = original.clone();
            local.fixture.signed.observation.local_attestations[0].attestation.signature.value = native::RecordId::parse(signature).unwrap();
            local.fixture.sign();
            assert!(local.fixture.produce().is_err());
            assert!(!local.fixture.ci.root.join("authenticated/identity.json").exists());
        }
    }
}
