mod authenticated_controller {
    use super::*;
    use policy_ci::controller::*;
    use std::path::Path;

    struct SignedFixture {
        ci: Fixture,
        config: ControllerConfig,
        signed: SignedObservation,
    }

    impl SignedFixture {
        fn new() -> Self {
            let ci = Fixture::new("ready");
            let openssl = std::env::var_os("POLICY_TEST_OPENSSL").map(PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::split_paths(&std::env::var_os("PATH").unwrap())
                        .map(|dir| dir.join("openssl"))
                        .find(|path| path.is_file()).expect("OpenSSL is required for cryptographic controller tests")
                });
            let mut identity: Value = read_json(&ci.trusted.join("identity.json")).unwrap();
            for key in ["schema_version", "evidence_digest", "context_digest", "work_item_digest"] {
                identity.as_object_mut().unwrap().remove(key);
            }
            identity["workflow_ref"] = json!("FlitCatz/policy-kernel/.github/workflows/policy.yml@refs/heads/main");
            identity["workflow_source_sha"] = json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
            let binding: RunBinding = serde_json::from_value(identity).unwrap();
            let mut evidence: EvidenceBundle = read_json(&ci.root.join("evidence.json")).unwrap();
            for v in &mut evidence.modifications { v.trust = TrustClass::LocalUntrusted; }
            for v in &mut evidence.executions { v.trust = TrustClass::LocalUntrusted; }
            for v in &mut evidence.reviews { v.trust = TrustClass::LocalUntrusted; }
            for v in &mut evidence.sensors { v.trust = TrustClass::LocalUntrusted; }
            for v in &mut evidence.knowledge { v.trust = TrustClass::LocalUntrusted; }
            let mut context: PreMergeContext = read_json(&ci.trusted.join("context.json")).unwrap();
            context.agent_ready.as_mut().unwrap().trust = TrustClass::LocalUntrusted;
            let mut policy: PolicySnapshot = read_json(&ci.root.join("policy.json")).unwrap();
            policy.schema_version = 2;
            policy.required_knowledge = serde_json::from_value(json!([{"kind":"code_graph_index", "source":"ci/codegraph", "required_when":[]}])).unwrap();
            evidence.knowledge = serde_json::from_value(json!([{
                "kind":"code_graph_index", "source":"ci/codegraph", "version":"1.7.0",
                "configuration_digest":"1".repeat(64), "content_digest":"2".repeat(64),
                "source_set_digest":evidence.manifest.source_set_digest,
                "head_sha":evidence.manifest.head_sha, "trust":"local_untrusted"
            }])).unwrap();
            policy.sensor_profile_digest = Some(policy_core::canonical_sha256(&context.sensor_profile).unwrap());
            policy.policy_digest = policy_core::pre_merge_policy_digest(&policy).unwrap();
            evidence.manifest.policy_digest = policy.policy_digest.clone();
            let mut binding = binding;
            binding.policy_digest = policy.policy_digest.clone();
            let inventory = Inventory {
                complete: true,
                contributors: evidence.modifications.iter().map(|v| v.execution_id.clone()).collect(),
                executions: evidence.executions.iter().map(|v| v.execution_id.clone()).collect(),
                reviews: evidence.reviews.iter().map(|v| v.execution_id.clone()).collect(),
                sensors: evidence.sensors.iter().map(|v| v.sensor_id.clone()).collect(),
                knowledge: evidence.knowledge.iter().map(|v| format!("{:?}:{}", v.kind, v.source)).collect(),
            };
            let mut keys = Vec::new();
            for (provider, model) in evidence.modifications.iter().map(|v| (&v.provider, &v.model))
                .chain(evidence.executions.iter().map(|v| (&v.provider, &v.model)))
                .chain(evidence.reviews.iter().map(|v| (&v.provider, &v.model))) {
                if !keys.iter().any(|v: &ProducerKey| &v.provider == provider && &v.model == model) {
                    let id = format!("producer-{}", keys.len());
                    Self::key(&openssl, &ci.root, &id);
                    keys.push(ProducerKey { key_id: id.clone(), public_key: ci.root.join(format!("{id}.pub")), provider: provider.clone(), model: model.clone() });
                }
            }
            Self::key(&openssl, &ci.root, "observer");
            let config = ControllerConfig { schema_version: 1, expected: binding.clone(), observer_public_key: ci.root.join("observer.pub"), producer_keys: keys, openssl, minimum_reviewers: 2 };
            let observation = Observation { domain: "policy-kernel/controller-observation/v1".into(), binding, policy, context, work_item: read_json(&ci.trusted.join("work-item.json")).unwrap(), evidence, inventory };
            let mut fixture = Self { ci, config, signed: SignedObservation { observation, observer_signature: String::new(), producers: Vec::new() } };
            fixture.sign();
            fixture
        }

        fn key(openssl: &Path, root: &Path, id: &str) {
            let output = std::process::Command::new(openssl).args(["genpkey", "-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048", "-out"]).arg(root.join(format!("{id}.key"))).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
            let output = std::process::Command::new(openssl).args(["pkey", "-in"]).arg(root.join(format!("{id}.key"))).args(["-pubout", "-out"]).arg(root.join(format!("{id}.pub"))).output().unwrap();
            assert!(output.status.success());
        }

        fn signature(&self, id: &str, bytes: &[u8]) -> String {
            fs::write(self.ci.root.join("sign-input"), bytes).unwrap();
            let output = std::process::Command::new(&self.config.openssl).args(["dgst", "-sha256", "-sign"]).arg(self.ci.root.join(format!("{id}.key"))).arg(self.ci.root.join("sign-input")).output().unwrap();
            assert!(output.status.success());
            output.stdout.iter().map(|byte| format!("{byte:02x}")).collect()
        }

        fn sign(&mut self) {
            let value = serde_json::to_value(&self.signed.observation.evidence).unwrap();
            for child in &mut self.signed.observation.evidence.manifest.child_digests {
                child.digest = policy_core::canonical_sha256(&value[&child.path]).unwrap();
            }
            self.signed.observer_signature = self.signature("observer", &canonical(&self.signed.observation).unwrap());
            let evidence = &self.signed.observation.evidence;
            self.signed.producers = evidence.modifications.iter().map(|v| (&v.execution_id, &v.provider, &v.model))
                .chain(evidence.executions.iter().map(|v| (&v.execution_id, &v.provider, &v.model)))
                .chain(evidence.reviews.iter().map(|v| (&v.execution_id, &v.provider, &v.model)))
                .map(|(execution_id, provider, model)| {
                    let key = self.config.producer_keys.iter().find(|v| &v.provider == provider && &v.model == model).unwrap();
                    // Construct the signing specification independently of the verifier helper.
                    let payload = serde_json::to_vec(&json!({
                        "domain":"policy-kernel/controller-producer/v1",
                        "binding":self.signed.observation.binding,
                        "evidence_digest":policy_core::canonical_sha256(&self.signed.observation.evidence).unwrap(),
                        "execution_id":execution_id,
                    })).unwrap();
                    ProducerSignature { key_id: key.key_id.clone(), execution_id: execution_id.clone(), signature: self.signature(&key.key_id, &payload) }
                }).collect();
        }

        fn produce(&self) -> Result<()> {
            produce(&self.config, &self.signed, &self.ci.root.join("authenticated"))
        }

        fn reject(&self, message: &str) {
            let error = self.produce().expect_err(message).to_string();
            assert!(error.contains(message), "expected {message}, got {error}");
            assert!(!self.ci.root.join("authenticated/identity.json").exists());
        }
    }

    #[test]
    fn authenticated_collection_reaches_authoritative_ci_and_preserves_local_source() {
        let mut fixture = SignedFixture::new();
        let source = canonical(&fixture.signed).unwrap();
        fs::write(fixture.ci.root.join("controller.json"), canonical(&fixture.config).unwrap()).unwrap();
        fs::write(fixture.ci.root.join("signed.json"), &source).unwrap();
        let output = Command::cargo_bin("policy-controller").unwrap()
            .arg("--config").arg(fixture.ci.root.join("controller.json"))
            .arg("--checkout").arg(&fixture.ci.checkout)
            .arg("--signed-observation").arg(fixture.ci.root.join("signed.json"))
            .arg("--output").arg(fixture.ci.root.join("authenticated"))
            .output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(source, fs::read(fixture.ci.root.join("signed.json")).unwrap());
        fixture.ci.trusted = fixture.ci.root.join("authenticated");
        for name in ["policy.json", "evidence.json"] {
            fs::copy(fixture.ci.trusted.join(name), fixture.ci.root.join(name)).unwrap();
        }
        for (name, value) in [("GITHUB_WORKFLOW_SHA", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), ("GITHUB_WORKFLOW_REF", "FlitCatz/policy-kernel/.github/workflows/policy.yml@refs/heads/attacker")] {
            let rejected = fixture.ci.command().env(name, value).output().unwrap();
            assert_eq!(rejected.status.code(), Some(2));
            assert!(!fixture.ci.root.join("published.json").exists());
        }
        let output = fixture.ci.command().output().unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        let decision: Value = read_json(&fixture.ci.artifacts.join("decision.json")).unwrap();
        assert_eq!(decision["status"], "ready_to_merge");
        assert_eq!(decision["advisory"], false);
        assert!(fixture.produce().is_err(), "output cannot be overwritten");
    }

    #[test]
    fn forged_json_local_artifacts_and_producer_signature_tampering_are_rejected() {
        let mut fixture = SignedFixture::new();
        let original = fixture.signed.clone();
        fixture.signed.observation.work_item.authorized = false;
        fixture.reject("signature authentication failed");
        fixture.signed = original.clone();
        fixture.signed.observer_signature.clear();
        fixture.reject("signature encoding");
        fixture.signed = original.clone();
        fixture.signed.producers[0].signature.replace_range(..2, "00");
        fixture.reject("signature authentication failed");
        fixture.signed = original;
        fixture.signed.producers.pop();
        fixture.reject("exactly one authenticated signature");
    }

    #[test]
    fn authenticated_receipts_cannot_replay_other_heads_runs_attempts_workflows_apps_or_policies() {
        let fixture = SignedFixture::new();
        let config = serde_json::to_value(&fixture.config).unwrap();
        for (field, value) in [
            ("head_sha", json!("dddddddddddddddddddddddddddddddddddddddd")),
            ("workflow_run_id", json!("9002")), ("workflow_run_attempt", json!(3)),
            ("workflow_trigger_head_sha", json!(fixture.ci.head)),
            ("workflow_head_sha", json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
            ("workflow_id", json!(78)), ("workflow_path", json!(".github/workflows/other.yml")),
            ("workflow_ref", json!("FlitCatz/policy-kernel/.github/workflows/policy.yml@refs/heads/untrusted")),
            ("workflow_source_sha", json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")),
            ("check_app_id", json!(123)), ("check_app_slug", json!("other")),
            ("policy_digest", json!("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")),
            ("pull_request_number", json!(7)), ("repository", json!("other/repository")),
        ] {
            let mut changed = config.clone();
            changed["expected"][field] = value;
            let changed: ControllerConfig = serde_json::from_value(changed).unwrap();
            assert!(produce(&changed, &fixture.signed, &fixture.ci.root.join("authenticated")).is_err(), "{field}");
        }
    }

    #[test]
    fn even_signed_incomplete_inventories_and_missing_reviews_or_contributors_fail() {
        let mut fixture = SignedFixture::new();
        let original = fixture.signed.clone();
        fixture.signed.observation.inventory.complete = false;
        fixture.sign(); fixture.reject("inventory is incomplete");
        fixture.signed = original.clone();
        fixture.signed.observation.evidence.modifications.pop();
        fixture.sign(); fixture.reject("inventory is incomplete");
        fixture.signed = original.clone();
        fixture.signed.observation.evidence.reviews.clear();
        fixture.signed.observation.inventory.reviews.clear();
        fixture.sign(); fixture.reject("inventory is incomplete");
        fixture.signed = original.clone();
        fixture.signed.observation.evidence.sensors.clear();
        fixture.signed.observation.inventory.sensors.clear();
        fixture.sign(); fixture.reject("inventory is incomplete");
        fixture.signed = original;
        fixture.signed.observation.evidence.modifications[0].trust = TrustClass::CiObserved;
        fixture.sign(); fixture.reject("each local producer record");
    }

    #[test]
    fn policy_and_provider_claims_cannot_be_changed_by_a_valid_observer() {
        let mut fixture = SignedFixture::new();
        let original = fixture.signed.clone();
        fixture.signed.observation.policy.required_sensor_ids.clear();
        fixture.sign(); fixture.reject("policy/profile/head/content binding mismatch");
        fixture.signed = original;
        let reviewer = fixture.signed.producers.last_mut().unwrap();
        reviewer.key_id = fixture.config.producer_keys[0].key_id.clone();
        fixture.reject("producer key does not authenticate");
    }

    #[test]
    fn signed_but_invalid_controller_configuration_cannot_issue_a_receipt() {
        let mut fixture = SignedFixture::new();
        let original = fixture.config.clone();
        let signed = fixture.signed.clone();
        for (pointer, value) in [
            ("/schema_version", json!(2)), ("/expected/workflow_run_attempt", json!(0)),
            ("/expected/workflow_id", json!(0)), ("/expected/check_app_id", json!(0)),
            ("/expected/check_app_slug", json!(" ")), ("/expected/workflow_path", json!("other.yml")),
            ("/expected/workflow_ref", json!("other/repo/.github/workflows/a.yml@main")),
            ("/minimum_reviewers", json!(1)), ("/openssl", json!("openssl")),
            ("/observer_public_key", json!("observer.pub")), ("/producer_keys/0/public_key", json!("producer.pub")),
        ] {
            let mut config = serde_json::to_value(&original).unwrap();
            *config.pointer_mut(pointer).unwrap() = value;
            fixture.config = serde_json::from_value(config).unwrap();
            fixture.signed = signed.clone();
            fixture.signed.observation.binding = fixture.config.expected.clone();
            fixture.reject("configuration/current-run binding mismatch");
        }
        fixture.config = original.clone(); fixture.signed = signed.clone();
        fixture.config.minimum_reviewers = 3;
        fixture.reject("inventory is incomplete");
        for variant in 0..3 {
            fixture.config = original.clone(); fixture.signed = signed.clone();
            match variant {
                0 => fixture.config.producer_keys[0].key_id.clear(),
                1 => fixture.config.producer_keys[1].key_id = fixture.config.producer_keys[0].key_id.clone(),
                _ => fixture.config.producer_keys[1].public_key = fixture.config.producer_keys[0].public_key.clone(),
            }
            fixture.reject("keys must be distinct");
        }
    }

    #[test]
    fn individually_signed_observation_bindings_and_inventory_entries_are_all_checked() {
        let mut fixture = SignedFixture::new();
        let original = fixture.signed.clone();
        let original_config = fixture.config.clone();
        for (pointer, value) in [
            ("/policy/policy_digest", json!("1".repeat(64))),
            ("/evidence/manifest/policy_digest", json!("1".repeat(64))),
            ("/evidence/manifest/repository", json!("other/repo")),
            ("/evidence/manifest/head_sha", json!("1".repeat(40))),
            ("/evidence/manifest/workflow_run_id", json!("8888")),
            ("/context/workflow_run_id", json!("8888")),
            ("/work_item/repository", json!("other/repo")),
            ("/work_item/work_item_id", json!("other-work-item")),
            ("/policy/sensor_profile_digest", json!("1".repeat(64))),
        ] {
            fixture.signed = original.clone(); fixture.config = original_config.clone();
            let mut value_json = serde_json::to_value(&fixture.signed.observation).unwrap();
            *value_json.pointer_mut(pointer).unwrap() = value;
            fixture.signed.observation = serde_json::from_value(value_json).unwrap();
            fixture.sign(); fixture.reject("policy/profile/head/content binding mismatch");
        }
        for field in ["contributors", "executions", "reviews", "sensors", "knowledge"] {
            fixture.signed = original.clone(); fixture.config = original_config.clone();
            let mut value = serde_json::to_value(&fixture.signed.observation).unwrap();
            let entries = value["inventory"][field].as_array_mut().unwrap();
            if let Some(first) = entries.first().cloned() { entries.push(first); }
            else { entries.push(json!("unobserved")); }
            fixture.signed.observation = serde_json::from_value(value).unwrap();
            fixture.sign(); fixture.reject("inventory is incomplete");
        }
        for field in ["modifications", "executions", "reviews", "sensors", "knowledge"] {
            fixture.signed = original.clone(); fixture.config = original_config.clone();
            let mut value = serde_json::to_value(&fixture.signed.observation).unwrap();
            if value["evidence"][field].as_array().unwrap().is_empty() { continue; }
            value["evidence"][field][0]["trust"] = json!("ci_observed");
            fixture.signed.observation = serde_json::from_value(value).unwrap();
            fixture.sign(); fixture.reject("each local producer record");
        }
        for (field, value) in [("provider", json!("other-provider")), ("model", json!("other-model"))] {
            fixture.signed = original.clone(); fixture.config = original_config.clone();
            let mut config = serde_json::to_value(&fixture.config).unwrap();
            config["producer_keys"][0][field] = value;
            fixture.config = serde_json::from_value(config).unwrap();
            fixture.reject("producer key does not authenticate");
        }
        for (field, value) in [("trust", json!("ci_observed")), ("repository", json!("other/repo")),
            ("head_sha", json!("1".repeat(40))), ("pull_request_number", json!(99)), ("work_item_id", json!("other-work-item"))] {
            fixture.signed = original.clone(); fixture.config = original_config.clone();
            let mut value_json = serde_json::to_value(&fixture.signed.observation).unwrap();
            value_json["context"]["agent_ready"][field] = value;
            fixture.signed.observation = serde_json::from_value(value_json).unwrap();
            fixture.sign(); fixture.reject("agent-ready observation does not match");
        }
    }

    #[test]
    fn controller_isolation_rejects_checkout_owned_configuration_keys_binary_and_output() {
        let fixture = SignedFixture::new();
        let config_path = fixture.ci.root.join("controller.json");
        fs::write(&config_path, canonical(&fixture.config).unwrap()).unwrap();
        let output = fixture.ci.root.join("new-output");
        validate_isolation(&config_path, &fixture.config, &fixture.ci.checkout, &output).unwrap();
        assert!(validate_isolation(&config_path, &fixture.config, &fixture.ci.checkout, &fixture.ci.checkout.join("output")).is_err());
        let untrusted = fixture.ci.checkout.join("config.json");
        fs::copy(&config_path, &untrusted).unwrap();
        assert!(validate_isolation(&untrusted, &fixture.config, &fixture.ci.checkout, &output).is_err());
        for field in ["openssl", "observer_public_key"] {
            let mut value = serde_json::to_value(&fixture.config).unwrap(); value[field] = json!(untrusted);
            let changed: ControllerConfig = serde_json::from_value(value).unwrap();
            assert!(validate_isolation(&config_path, &changed, &fixture.ci.checkout, &output).is_err());
        }
        let mut config = fixture.config.clone(); config.producer_keys[0].public_key = untrusted;
        assert!(validate_isolation(&config_path, &config, &fixture.ci.checkout, &output).is_err());
    }

    #[test]
    fn authenticated_inventory_must_include_requirements_from_both_policy_and_profile() {
        let mut fixture = SignedFixture::new();
        let original = fixture.signed.clone();
        for source in ["policy", "profile"] {
            fixture.signed = original.clone();
            if source == "policy" {
                fixture.signed.observation.policy.required_sensor_ids.push(policy_contracts::SensorId::parse("backend:additional").unwrap());
            } else {
                let profile = &mut fixture.signed.observation.context.sensor_profile;
                let mut sensor = profile.sensors[0].clone();
                sensor.id = policy_contracts::SensorId::parse("backend:additional").unwrap(); sensor.required = true;
                profile.sensors.push(sensor);
                fixture.signed.observation.policy.sensor_profile_digest = Some(policy_core::canonical_sha256(profile).unwrap());
            }
            let raw = &mut fixture.signed.observation;
            raw.policy.policy_digest = policy_core::pre_merge_policy_digest(&raw.policy).unwrap();
            raw.evidence.manifest.policy_digest = raw.policy.policy_digest.clone();
            raw.binding.policy_digest = raw.policy.policy_digest.clone();
            fixture.config.expected.policy_digest = raw.policy.policy_digest.clone();
            fixture.sign(); fixture.reject("inventory is incomplete");
        }
        fixture.signed = original;
        fixture.config.expected = fixture.signed.observation.binding.clone();
        fixture.signed.observation.evidence.manifest.child_digests[0].digest = policy_contracts::Sha256Digest::parse("1".repeat(64)).unwrap();
        fixture.signed.observer_signature = fixture.signature("observer", &canonical(&fixture.signed.observation).unwrap());
        fixture.reject("policy/profile/head/content binding mismatch");
    }
}
