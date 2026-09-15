use assert_cmd::Command;

#[test]
fn advisory_mode_is_rejected_by_authoritative_entry_point() {
    let output = Command::cargo_bin("policy-ci")
        .unwrap()
        .args(["pre-merge", "--mode", "advisory"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("authoritative"));
}

#[test]
fn expected_head_must_equal_independently_observed_checkout_head() {
    let output = Command::cargo_bin("policy-ci")
        .unwrap()
        .args([
            "pre-merge",
            "--mode",
            "authoritative",
            "--expected-head",
            "cccccccccccccccccccccccccccccccccccccccc",
            "--checked-out-head",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("checked-out head"));
}

#[test]
fn local_invocation_cannot_claim_actions_identity() {
    let output = Command::cargo_bin("policy-ci")
        .unwrap()
        .env_remove("GITHUB_ACTIONS")
        .args([
            "pre-merge",
            "--expected-head",
            "cccccccccccccccccccccccccccccccccccccccc",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("GitHub Actions"));
}

#[cfg(unix)]
mod integration {
    use super::*;
    use policy_contracts::*;
    use serde_json::{json, Value};
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::{
            atomic::{AtomicU64, Ordering},
            OnceLock,
        },
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);

    include!("support/controller.rs");

    #[test]
    fn legacy_receipts_cannot_partially_adopt_authenticated_workflow_fields() {
        for (field, value) in [
            (
                "workflow_ref",
                "FlitCatz/policy-kernel/.github/workflows/policy.yml@refs/heads/main",
            ),
            (
                "workflow_source_sha",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
        ] {
            let fixture = Fixture::new("ready");
            let path = fixture.trusted.join("identity.json");
            let mut receipt: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
            assert_eq!(receipt["schema_version"], 1);
            receipt[field] = json!(value);
            fs::write(path, serde_json::to_vec(&receipt).unwrap()).unwrap();
            let output = fixture.command().output().unwrap();
            assert_eq!(output.status.code(), Some(2), "{field}");
            assert!(!fixture.root.join("published.json").exists());
        }
    }

    // Compile the Rust-only process fixture once, using the dependencies Cargo
    // built for this integration test. It is not a production binary target.
    fn fake_gh() -> &'static PathBuf {
        static EXE: OnceLock<PathBuf> = OnceLock::new();
        EXE.get_or_init(|| {
            let deps = std::env::current_exe()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf();
            let artifact = |name: &str| {
                let mut paths: Vec<_> = fs::read_dir(&deps)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .filter(|p| {
                        p.file_name()
                            .unwrap()
                            .to_string_lossy()
                            .starts_with(&format!("lib{name}-"))
                            && p.extension().is_some_and(|e| e == "rlib")
                    })
                    .collect();
                paths.sort_by_key(|p| fs::metadata(p).unwrap().modified().unwrap());
                paths.pop().expect("Cargo must build fixture dependencies")
            };
            let output =
                std::env::temp_dir().join(format!("policy-ci-fake-gh-{}", std::process::id()));
            let result = std::process::Command::new(
                std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()),
            )
            .args(["--edition=2021", "--crate-name", "policy_ci_fake_gh"])
            .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/support/fake_gh.rs"))
            .arg("-L")
            .arg(format!("dependency={}", deps.display()))
            .arg("--extern")
            .arg(format!("serde_json={}", artifact("serde_json").display()))
            .arg("--extern")
            .arg(format!("chrono={}", artifact("chrono").display()))
            .arg("-o")
            .arg(&output)
            .output()
            .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            output
        })
    }

    struct Fixture {
        root: PathBuf,
        checkout: PathBuf,
        trusted: PathBuf,
        artifacts: PathBuf,
        head: String,
    }
    impl Fixture {
        fn new(status: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "policy-ci-test-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            let checkout = root.join("checkout");
            let trusted = root.join("trusted");
            let artifacts = root.join("artifacts");
            fs::create_dir_all(&checkout).unwrap();
            fs::create_dir_all(&trusted).unwrap();
            for args in [
                vec!["init", "--quiet"],
                vec![
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "commit",
                    "--quiet",
                    "--allow-empty",
                    "-m",
                    "fixture",
                ],
            ] {
                assert!(std::process::Command::new("git")
                    .args(args)
                    .current_dir(&checkout)
                    .status()
                    .unwrap()
                    .success());
            }
            let head = String::from_utf8(
                std::process::Command::new("git")
                    .args(["rev-parse", "HEAD"])
                    .current_dir(&checkout)
                    .output()
                    .unwrap()
                    .stdout,
            )
            .unwrap()
            .trim()
            .to_string();
            let now: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
            let observed = (now - chrono::Duration::seconds(20)).to_rfc3339();
            let raw = include_str!("../../../tests/fixtures/pre-merge/ready.json")
                .replace(
                    "cccccccccccccccccccccccccccccccccccccccc\"",
                    &format!("{head}\""),
                )
                .replace("2026-09-07T08:58:00Z", &observed)
                .replace("2026-09-07T08:57:00Z", &observed)
                .replace("2026-09-07T09:00:00Z", &now.to_rfc3339())
                .replace("run-6", "9001");
            let mut input: PreMergeEvaluationInput = serde_json::from_str(&raw).unwrap();
            input.policy.required_check_name = "policy-kernel/pre-merge".into();
            input.policy.protected_target_branches = vec!["main".into()];
            input.policy.allowed_merge_methods = vec![MergeMethod::Merge, MergeMethod::Squash];
            input.policy.policy_digest =
                policy_core::pre_merge_policy_digest(&input.policy).unwrap();
            input.evidence.manifest.policy_digest = input.policy.policy_digest.clone();
            if status == "blocked" {
                input.work_item.authorized = false;
            }
            if status == "inconclusive" {
                input.context.as_mut().unwrap().agent_ready = None;
            }
            let evidence_value = serde_json::to_value(&input.evidence).unwrap();
            for child in &mut input.evidence.manifest.child_digests {
                child.digest = policy_core::canonical_sha256(&evidence_value[&child.path]).unwrap();
            }
            let write = |path: PathBuf, value: Value| {
                fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap()
            };
            write(
                root.join("policy.json"),
                serde_json::to_value(&input.policy).unwrap(),
            );
            write(
                root.join("evidence.json"),
                serde_json::to_value(&input.evidence).unwrap(),
            );
            write(
                trusted.join("context.json"),
                serde_json::to_value(&input.context).unwrap(),
            );
            write(
                trusted.join("work-item.json"),
                serde_json::to_value(&input.work_item).unwrap(),
            );
            write(
                trusted.join("identity.json"),
                json!({
                    "schema_version": 1, "repository": "FlitCatz/policy-kernel", "pull_request_number": 6,
                    "head_sha": head, "workflow_run_id": "9001", "workflow_run_attempt": 2,
                    "workflow_trigger_head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "workflow_head_sha": head, "workflow_id": 77,
                    "workflow_path": ".github/workflows/policy.yml", "check_app_id": 15368, "check_app_slug": "github-actions",
                    "policy_digest": input.policy.policy_digest, "evidence_digest": policy_core::canonical_sha256(&input.evidence).unwrap(),
                    "context_digest": policy_core::canonical_sha256(input.context.as_ref().unwrap()).unwrap(),
                    "work_item_digest": policy_core::canonical_sha256(&input.work_item).unwrap()
                }),
            );
            let mut pr: Value =
                serde_json::from_str(include_str!("../../../tests/fixtures/github/pr-open.json"))
                    .unwrap();
            pr["number"] = json!(6);
            pr["head"]["sha"] = json!(head);
            pr["updated_at"] = json!(observed);
            write(root.join("pr.json"), pr);
            let mut workflow: Value = serde_json::from_str(include_str!(
                "../../../tests/fixtures/github/workflow-run.json"
            ))
            .unwrap();
            workflow["head_sha"] = json!(head);
            workflow["created_at"] = json!(observed);
            workflow["updated_at"] = json!(observed);
            write(root.join("workflow.json"), workflow);
            fs::create_dir_all(root.join("bin")).unwrap();
            let gh = root.join("bin/gh");
            fs::copy(fake_gh(), &gh).unwrap();
            fs::set_permissions(gh, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(root.join("output"), "").unwrap();
            fs::write(root.join("summary"), "").unwrap();
            Self {
                root,
                checkout,
                trusted,
                artifacts,
                head,
            }
        }
        fn command(&self) -> Command {
            self.command_for("pre-merge")
        }
        fn command_for(&self, mode: &str) -> Command {
            let mut cmd = Command::cargo_bin("policy-ci").unwrap();
            cmd.current_dir(&self.checkout)
                .env(
                    "PATH",
                    format!(
                        "{}:{}",
                        self.root.join("bin").display(),
                        std::env::var("PATH").unwrap()
                    ),
                )
                .env("FAKE_GH_ROOT", &self.root)
                .env("GITHUB_ACTIONS", "true")
                .env("GITHUB_WORKSPACE", &self.checkout)
                .env("GITHUB_REPOSITORY", "FlitCatz/policy-kernel")
                .env("GITHUB_RUN_ID", "9001")
                .env("GITHUB_RUN_ATTEMPT", "2")
                .env("GITHUB_SHA", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .env(
                    "GITHUB_WORKFLOW_SHA",
                    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                )
                .env(
                    "GITHUB_WORKFLOW_REF",
                    "FlitCatz/policy-kernel/.github/workflows/policy.yml@refs/heads/main",
                )
                .env("GITHUB_OUTPUT", self.root.join("output"))
                .env("GITHUB_STEP_SUMMARY", self.root.join("summary"))
                .args([mode, "--expected-head", &self.head, "--policy"])
                .arg(self.root.join("policy.json"))
                .arg("--evidence")
                .arg(self.root.join("evidence.json"))
                .arg("--trusted-observations")
                .arg(&self.trusted)
                .arg("--artifact-directory")
                .arg(&self.artifacts);
            cmd
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn ready_blocked_and_inconclusive_publish_bound_checks_before_exit() {
        for (status, conclusion, success) in [
            ("ready", "success", true),
            ("blocked", "failure", false),
            ("inconclusive", "neutral", false),
        ] {
            let fixture = Fixture::new(status);
            let output = fixture.command().output().unwrap();
            assert_eq!(
                output.status.success(),
                success,
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let published: Value = serde_json::from_slice(
                &fs::read(fixture.root.join("published.json")).expect("must publish check"),
            )
            .unwrap();
            assert_eq!(published["conclusion"], conclusion);
            assert_eq!(published["head_sha"], fixture.head);
            let decision: Value =
                serde_json::from_slice(&fs::read(fixture.artifacts.join("decision.json")).unwrap())
                    .unwrap();
            assert_eq!(decision["advisory"], false);
            let binding: Value =
                serde_json::from_str(published["output"]["text"].as_str().unwrap()).unwrap();
            assert_eq!(binding["decision_digest"], decision["decision_digest"]);
            assert_eq!(binding["workflow_run_attempt"], 2);
            assert!(fs::read_to_string(fixture.root.join("output"))
                .unwrap()
                .contains("check_run_id=6001"));
            assert!(fixture.artifacts.join("publication-receipt.json").is_file());
        }
    }

    #[test]
    fn altered_evidence_cannot_borrow_a_trusted_receipt() {
        let fixture = Fixture::new("ready");
        let path = fixture.root.join("evidence.json");
        let mut value: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        value["manifest"]["workflow_run_id"] = json!("12345");
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
        let output = fixture.command().output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("evidence"));
        assert!(!fixture.root.join("published.json").exists());
    }
    #[test]
    fn post_merge_verifies_live_check_and_preserves_squash_head() {
        let mut fixture = Fixture::new("ready");
        let output = fixture.command().output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for name in ["publication-receipt.json", "pre-merge-input.json"] {
            fs::copy(fixture.artifacts.join(name), fixture.trusted.join(name)).unwrap();
        }
        let merged_at: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
        let merge_sha = "dddddddddddddddddddddddddddddddddddddddd";
        let mut pr: Value =
            serde_json::from_slice(&fs::read(fixture.root.join("pr.json")).unwrap()).unwrap();
        pr["state"] = json!("closed");
        pr["merged"] = json!(true);
        pr["merged_at"] = json!(merged_at.to_rfc3339());
        pr["merge_commit_sha"] = json!(merge_sha);
        pr["merged_by"] = json!({"login":"release-owner", "type":"User"});
        fs::write(
            fixture.root.join("pr.json"),
            serde_json::to_vec(&pr).unwrap(),
        )
        .unwrap();
        fs::write(fixture.trusted.join("merge-receipt.json"), serde_json::to_vec(&json!({
            "schema_version":1, "repository":"FlitCatz/policy-kernel", "pull_request_number":6,
            "head_sha":fixture.head, "merge_commit_sha":merge_sha, "target_branch":"main",
            "method":"squash", "protected":true, "merged_by":"release-owner", "actor_type":"human",
            "via_merge_queue":false, "merged_at":merged_at.to_rfc3339()
        })).unwrap()).unwrap();
        fixture.artifacts = fixture.root.join("post-artifacts");
        let output = fixture.command_for("post-merge").output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let decision: Value =
            serde_json::from_slice(&fs::read(fixture.artifacts.join("decision.json")).unwrap())
                .unwrap();
        assert_eq!(decision["status"], "shipped");
        assert_eq!(decision["binding"]["head_sha"], fixture.head);
        assert_eq!(decision["merge_commit_sha"], merge_sha);

        let mut published: Value =
            serde_json::from_slice(&fs::read(fixture.root.join("published.json")).unwrap())
                .unwrap();
        published["external_id"] = json!("policy-kernel:v1:forged:forged");
        fs::write(
            fixture.root.join("published.json"),
            serde_json::to_vec(&published).unwrap(),
        )
        .unwrap();
        fixture.artifacts = fixture.root.join("forged-post-artifacts");
        let output = fixture.command_for("post-merge").output().unwrap();
        assert!(!output.status.success());
        assert!(!fixture.artifacts.join("decision.json").exists());
    }
    #[test]
    fn merge_during_check_publication_cannot_ship() {
        let mut fixture = Fixture::new("ready");
        let output = fixture
            .command()
            .env("MERGE_DURING_POST", "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for name in ["publication-receipt.json", "pre-merge-input.json"] {
            fs::copy(fixture.artifacts.join(name), fixture.trusted.join(name)).unwrap();
        }
        let pr: Value =
            serde_json::from_slice(&fs::read(fixture.root.join("pr.json")).unwrap()).unwrap();
        fs::write(fixture.trusted.join("merge-receipt.json"), serde_json::to_vec(&json!({
            "schema_version":1, "repository":"FlitCatz/policy-kernel", "pull_request_number":6,
            "head_sha":fixture.head, "merge_commit_sha":pr["merge_commit_sha"], "target_branch":"main",
            "method":"squash", "protected":true, "merged_by":"release-owner", "actor_type":"human",
            "via_merge_queue":false, "merged_at":pr["merged_at"]
        })).unwrap()).unwrap();
        fixture.artifacts = fixture.root.join("late-post-artifacts");
        let output = fixture.command_for("post-merge").output().unwrap();
        assert!(
            !output.status.success(),
            "late publication must not authorize shipping"
        );
        let decision: Value =
            serde_json::from_slice(&fs::read(fixture.artifacts.join("decision.json")).unwrap())
                .unwrap();
        assert_eq!(decision["status"], "inconclusive");
        assert!(decision["reasons"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["code"] == "PK-POST-CHECK-AFTER-MERGE"));
    }
    #[test]
    fn observations_cannot_be_swapped_under_another_receipt() {
        let fixture = Fixture::new("blocked");
        let path = fixture.trusted.join("work-item.json");
        let mut work: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        work["authorized"] = json!(true);
        fs::write(path, serde_json::to_vec(&work).unwrap()).unwrap();
        let output = fixture.command().output().unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("observation digest"));
        assert!(!fixture.root.join("published.json").exists());
    }
    #[test]
    fn workflow_api_head_and_actions_trigger_sha_are_independently_bound() {
        let fixture = Fixture::new("ready");
        let output = fixture
            .command()
            .env("GITHUB_SHA", &fixture.head)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!fixture.root.join("published.json").exists());
        let path = fixture.root.join("workflow.json");
        let mut workflow: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        workflow["head_sha"] = json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        fs::write(path, serde_json::to_vec(&workflow).unwrap()).unwrap();
        let output = fixture.command().output().unwrap();
        assert!(!output.status.success());
        assert!(!fixture.root.join("published.json").exists());
    }
}
