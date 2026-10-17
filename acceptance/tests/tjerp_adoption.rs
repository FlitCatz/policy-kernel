//! Synthetic TJERP-shaped protocol acceptance only. No TJERP C#/Svelte execution.
use policy_acceptance::tjerp::*;
use policy_contracts::*;
use policy_core::{canonical_sha256, evaluate_sensor_evidence, validate_provenance};
use project_verification::{bytes_digest, parse_profile, ProjectProfile, RunRequest, RunResult};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn now() -> UtcTimestamp {
    let t: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    UtcTimestamp::parse(&t.to_rfc3339()).unwrap()
}
fn save(path: &Path, bytes: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}
fn report(root: &Path, component: &str, role: &str, fault: &str) -> Vec<u8> {
    let failed = (fault == "FI-01" && component == "backend" && role == "contract")
        || (fault == "FI-02" && component == "backend" && role == "unit");
    let passed = u64::from(!failed);
    if component == "backend" {
        format!(r#"<TestRun xmlns="http://microsoft.com/schemas/VisualStudio/TeamTest/2010"><Times start="{}"/><Results><UnitTestResult testId="synthetic-{role}" testName="TjArch.Tests.{role}Tests.Check" outcome="{}"/></Results><ResultSummary outcome="{}"><Counters total="1" executed="1" passed="{passed}" failed="{}" error="0" timeout="0" aborted="0" inconclusive="0" passedButRunAborted="0" notRunnable="0" notExecuted="0" disconnected="0" warning="0" completed="0" inProgress="0" pending="0"/></ResultSummary></TestRun>"#,now().as_datetime().to_rfc3339(),if failed{"Failed"}else{"Passed"},if failed{"Failed"}else{"Completed"},u64::from(failed)).into_bytes()
    } else if role == "unit" {
        serde_json::to_vec(&json!({"startTime":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64,
            "success":true,"numTotalTests":1,"numPassedTests":1,"numFailedTests":0,"numTodoTests":0,"numPendingTests":0,
            "numTotalTestSuites":1,"numPassedTestSuites":1,"numFailedTestSuites":0,"numPendingTestSuites":0,
            "testResults":[{"name":root.join("frontend/src/Statement.test.ts"),"status":"passed","message":"","assertionResults":[{"fullName":"synthetic preserves draft","status":"passed","failureMessages":[]}]}]})).unwrap()
    } else {
        serde_json::to_vec(&json!({"config":{"rootDir":root.join("frontend/e2e")},"errors":[],"stats":{"startTime":now(),"expected":1,"unexpected":0,"flaky":0,"skipped":0},
            "suites":[{"specs":[{"id":format!("synthetic-{role}"),"file":"sales.spec.ts","title":format!("synthetic {role}"),"tests":[{"projectId":"","status":"expected","expectedStatus":"passed","annotations":[],"results":[{"startTime":now(),"status":"passed","retry":0,"errors":[]}]}]}]}]})).unwrap()
    }
}
fn produce(role: &str) {
    let cwd = std::env::current_dir().unwrap();
    let component = cwd.file_name().unwrap().to_str().unwrap();
    let fault = fs::read_to_string("producer.json").unwrap();
    if role == "fsd" {
        if fault == "FI-03" {
            eprintln!("synthetic layer-direction");
            std::process::exit(1);
        }
        return;
    }
    if fault == "FI-04" && component == "backend" && role == "contract" {
        return;
    }
    let extension = if component == "backend" {
        "trx"
    } else {
        "json"
    };
    save(
        &cwd.join(format!(".policy-reports/{role}.{extension}")),
        report(cwd.parent().unwrap(), component, role, &fault),
    );
}
macro_rules! producer {
    ($name:ident,$role:literal) => {
        #[test]
        #[ignore = "synthetic subprocess entry point, invoked by acceptance tests"]
        fn $name() {
            produce($role)
        }
    };
}
producer!(producer_unit, "unit");
producer!(producer_contract, "contract");
producer!(producer_integration, "integration");
producer!(producer_e2e, "e2e");
producer!(producer_csp, "csp");
producer!(producer_fsd, "fsd");

struct Fixture {
    _temp: tempfile::TempDir,
    root: PathBuf,
    profiles: [ProjectProfile; 2],
    catalog: Catalog,
    bytes: Vec<u8>,
    suites: Suites,
    approved: Sha256Digest,
}
impl Fixture {
    fn new(fault: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let exe = std::env::current_exe().unwrap();
        let version = String::from_utf8(Command::new(&exe).arg("--list").output().unwrap().stdout)
            .unwrap()
            .trim()
            .to_string();
        let mut suites = json!({"version":1,"backend":{"cwd":"backend","suites":{}},"frontend":{"cwd":"frontend","suites":{}}});
        let mut profiles = vec![];
        let mut reqs = vec![];
        let mut components = json!({});
        save(
            &root.join(".planning/REQUIREMENTS.md"),
            "- [ ] **AUTH-01** synthetic parent\n- [ ] **STMT-04** synthetic parent\n",
        );
        for (component, parent) in [("backend", "AUTH-01"), ("frontend", "STMT-04")] {
            save(&root.join(format!("{component}/producer.json")), fault);
            for file in [
                "src/Statement.test.ts",
                "e2e/sales.spec.ts",
                "tests/TjArch.Tests/contractTests.cs",
                "tests/TjArch.Tests/unitTests.cs",
                "tests/TjArch.Tests/integrationTests.cs",
            ] {
                save(
                    &root.join(format!("{component}/{file}")),
                    "// SYNTHETIC; not actual TJERP source\n",
                );
            }
            let roles = if component == "backend" {
                vec!["unit", "integration", "contract"]
            } else {
                vec!["unit", "e2e", "csp", "fsd"]
            };
            let mut sensors = vec![];
            for role in roles {
                let id = format!("{component}:{role}");
                let argv = json!([
                    exe,
                    "--ignored",
                    "--exact",
                    format!("producer_{role}"),
                    "--nocapture"
                ]);
                let (format, extension) = if component == "backend" {
                    ("trx", "trx")
                } else if role == "unit" {
                    ("vitest_json", "json")
                } else {
                    ("playwright_json", "json")
                };
                let path = format!(".policy-reports/{role}.{extension}");
                let spec = if role == "fsd" {
                    Value::Null
                } else {
                    json!({"format":format,"path":path})
                };
                sensors.push(json!({"definition":{"id":id,"required":true,"required_when":[],"not_applicable_when":[],"accepted_exit_codes":[0],"result_requirement":if role=="fsd"{"process"}else{"test_counts"}},
                    "component":component,"argv":argv,"version_args":["--list"],"version_stdout":version,"environment_allowlist":[],"report":spec,"inputs":["producer.json"],"timeout_ms":20000}));
                if role != "fsd" {
                    suites[component]["suites"][role] = json!({"report":path,"format":format,"retry":0,"max_skipped":0,"argv":argv,"classes":if component=="backend" {vec![format!("TjArch.Tests.{role}Tests")]}else{vec![]}});
                }
            }
            for role in if component == "backend" {
                vec!["build", "api-audit", "modules-audit", "tests-audit"]
            } else {
                vec!["build", "specs", "api", "audit"]
            } {
                sensors.push(json!({"definition":{"id":format!("{component}:{role}"),"required":true,"required_when":[],"not_applicable_when":[],"accepted_exit_codes":[0],"result_requirement":"process"},
                    "component":component,"argv":[exe,"--list"],"version_args":["--list"],"version_stdout":version,"environment_allowlist":[],"report":null,"inputs":["producer.json"],"timeout_ms":20000}));
            }
            let ac = format!("{component}:{parent}-AC01");
            let b = format!("{component}:{parent}-B01");
            let n = format!("{component}:{parent}-N01");
            let lists =
                json!({"acceptance_criteria":[ac],"changed_behaviors":[b,n],"negative_cases":[n]});
            components[component] = json!({"root":component,"adequacy":lists});
            for (id, kind, lists) in [
                (ac.clone(), "AC", vec!["acceptance_criteria"]),
                (b.clone(), "B", vec!["changed_behaviors"]),
                (n.clone(), "N", vec!["changed_behaviors", "negative_cases"]),
            ] {
                let selector = if component == "backend" {
                    json!({"path":"backend/tests/TjArch.Tests/contractTests.cs","selector":"TjArch.Tests.contractTests.Check","selector_kind":"fully_qualified_name","suite":"backend:contract"})
                } else {
                    json!({"path":"frontend/src/Statement.test.ts","selector":"synthetic preserves draft","selector_kind":"fullName","suite":"frontend:unit"})
                };
                reqs.push(json!({"id":id,"parent_id":parent,"component":component,"kind":kind,"adequacy_lists":lists,"tests":[selector]}));
            }
            profiles.push(parse_profile(&json!({"schema_version":1,"id":if component=="backend"{"dotnet"}else{"react-typescript"},"version":"1.0.0","components":[{"id":component,"root":component}],"sensors":sensors,
                "adequacy":{"acceptance_criterion_ids":[ac],"changed_behavior_ids":[b,n],"negative_case_ids":[n]}}).to_string()).unwrap());
        }
        let profiles: [ProjectProfile; 2] = profiles.try_into().unwrap();
        let suites: Suites = serde_json::from_value(suites).unwrap();
        let bytes=serde_json::to_vec(&json!({"catalog_version":1,"components":components,"requirements":reqs,"required_regression_tests":[{"path":"frontend/e2e/sales.spec.ts","selector":"synthetic e2e","selector_kind":"title","suite":"frontend:e2e"}]})).unwrap();
        let approved = bytes_digest(&bytes);
        let catalog =
            check_catalog(&bytes, &approved, "AUTH-01 STMT-04", &suites, &profiles).unwrap();
        Self {
            _temp: temp,
            root,
            profiles,
            catalog,
            bytes,
            suites,
            approved,
        }
    }
    fn requests(&self) -> [RunRequest; 2] {
        let mut raw = vec![];
        for (id, c, s) in self.suites.entries() {
            raw.extend(
                link_report(
                    &self.root,
                    &id,
                    s,
                    &report(
                        &self.root,
                        &c.cwd,
                        id.split(':').nth(1).unwrap(),
                        "baseline",
                    ),
                )
                .unwrap(),
            );
        }
        let files: Vec<_> = [
            "backend/producer.json",
            "frontend/producer.json",
            ".planning/REQUIREMENTS.md",
        ]
        .into_iter()
        .map(|p| ArtifactDigest {
            path: p.into(),
            digest: bytes_digest(&checked_read(&self.root, p).unwrap()),
        })
        .collect();
        let identity:RunIdentity=serde_json::from_value(json!({"repository":"local/TJERP-synthetic","head_sha":"a".repeat(40),"provider":"synthetic","model":"rust-fixture","execution_id":"synthetic-P7"})).unwrap();
        requests(
            &self.root,
            self.profiles.clone(),
            &files,
            &identity,
            &self.catalog,
            &self.suites,
            &raw,
        )
        .unwrap()
    }
    fn results(&self, requests: &[RunRequest; 2]) -> [RunResult; 2] {
        [
            project_verification::run(requests[0].clone()).unwrap(),
            project_verification::run(requests[1].clone()).unwrap(),
        ]
    }
    fn evaluate(&self, r: &[RunRequest; 2], v: &[RunResult; 2]) -> ScopeReport {
        evaluate_pair(r, v, &now(), 3600, &self.catalog, &self.suites).unwrap()
    }
}

// @lat: [[tjerp#Synthetic fault matrix]]
#[test]
fn fi_00_baseline_is_local_scope_satisfied_without_a_review_claim() {
    let f = Fixture::new("baseline");
    let r = f.requests();
    let v = f.results(&r);
    let report = f.evaluate(&r, &v);
    assert_eq!(report.label, "local scope satisfied");
    assert_eq!(report.trust, TrustClass::LocalUntrusted);
    assert!(report.review_gate.starts_with("unfulfilled"));
    assert_eq!(report.requirements.len(), 6);
    assert!(v
        .iter()
        .flat_map(|v| &v.sensors)
        .all(|s| s.evidence.trust == TrustClass::LocalUntrusted
            && s.evidence.status == SensorStatus::Passed));
}
#[test]
fn fi_01_synthetic_failed_contract_marks_ac_and_negative_unproven() {
    let f = Fixture::new("FI-01");
    let r = f.requests();
    let v = f.results(&r);
    let report = f.evaluate(&r, &v);
    assert_eq!(report.label, "local scope non-pass");
    assert_eq!(
        report.requirements["backend:AUTH-01-AC01"],
        "failed/unproven"
    );
    assert_eq!(
        report.requirements["backend:AUTH-01-N01"],
        "failed/unproven"
    );
    assert!(report
        .reasons
        .iter()
        .any(|r| r.code == "TJ-REQUIREMENT-UNPROVEN"));
}
#[test]
fn fi_02_synthetic_architecture_unit_failure_is_not_hidden_by_other_suites() {
    let f = Fixture::new("FI-02");
    let r = f.requests();
    let v = f.results(&r);
    assert_eq!(
        v[0].sensors
            .iter()
            .find(|s| s.evidence.sensor_id.as_str() == "backend:unit")
            .unwrap()
            .evidence
            .status,
        SensorStatus::Failed
    );
    assert_eq!(f.evaluate(&r, &v).label, "local scope non-pass");
}
#[test]
fn fi_03_synthetic_fsd_process_failure_preserves_diagnostic() {
    let f = Fixture::new("FI-03");
    let r = f.requests();
    let v = f.results(&r);
    let fsd = v[1]
        .sensors
        .iter()
        .find(|s| s.evidence.sensor_id.as_str() == "frontend:fsd")
        .unwrap();
    assert_eq!(fsd.evidence.exit_code, Some(1));
    assert!(
        String::from_utf8_lossy(&fsd.collection.record.execution.stderr)
            .contains("layer-direction")
    );
    assert_eq!(f.evaluate(&r, &v).label, "local scope non-pass");
}
#[test]
fn fi_04_missing_report_exit_zero_is_nonpass_and_raw_artifacts_survive() {
    let f = Fixture::new("FI-04");
    let r = f.requests();
    let out = f.root.join("output");
    fs::create_dir(&out).unwrap();
    let err = execute(&r, &out, &f.catalog, &f.suites, now, 3600)
        .unwrap_err()
        .to_string();
    assert!(err.contains("TJ-COLLECTION-NONPASS"));
    assert!(out.join("failure.json").exists());
    assert!(out.join("frontend-unit.raw").exists());
    assert!(!out.join("scope.json").exists());
}
#[test]
fn fi_05_old_head_and_internally_consistent_replay_are_rejected_externally() {
    let f = Fixture::new("baseline");
    let mut r = f.requests();
    let v = f.results(&r);
    r[1].head_sha = CommitSha::parse("b".repeat(40)).unwrap();
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-SPLIT-BINDING"));
    r[0].head_sha = r[1].head_sha.clone();
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-RESULT-BINDING"));
    let bundle = bundle(&r[0], &v[0], &now()).unwrap();
    assert!(
        validate_provenance(EvaluationMode::Advisory, &bundle, &now(), 3600)
            .reason_codes()
            .contains(&"PK-PROV-BINDING-MISMATCH")
    );
}
fn seal(bundle: &mut EvidenceBundle) {
    let value = serde_json::to_value(&*bundle).unwrap();
    bundle.manifest.child_digests = [
        "modifications",
        "executions",
        "reviews",
        "findings",
        "waivers",
        "sensors",
        "test_adequacy",
        "knowledge",
    ]
    .into_iter()
    .map(|p| ArtifactDigest {
        path: p.into(),
        digest: canonical_sha256(&value[p]).unwrap(),
    })
    .collect();
}
#[test]
fn fi_06_distinct_agents_models_and_executions_do_not_make_provider_independent() {
    let f = Fixture::new("baseline");
    let r = f.requests();
    let v = f.results(&r);
    let mut bundle = bundle(&r[0], &v[0], &now()).unwrap();
    for (execution, model) in [("review-A", "model-A"), ("review-B", "model-B")] {
        bundle.reviews.push(serde_json::from_value(json!({"provider":"same-provider","model":model,"execution_id":execution,"status":"approved","head_sha":r[0].head_sha,"report_digest":bytes_digest(b"synthetic review"),"observed_at":now(),"trust":"local_untrusted"})).unwrap());
    }
    bundle.modifications.push(serde_json::from_value(json!({"provider":"same-provider","model":"modifier","execution_id":"modifier-A","head_sha":r[0].head_sha,"observed_at":now(),"trust":"local_untrusted"})).unwrap());
    seal(&mut bundle);
    assert!(
        validate_provenance(EvaluationMode::Advisory, &bundle, &now(), 3600)
            .reason_codes()
            .contains(&"PK-PROV-DUPLICATE-PROVIDER")
    );
    assert!(
        validate_provenance(EvaluationMode::Advisory, &bundle, &now(), 3600)
            .reason_codes()
            .contains(&"PK-SELF-PROVIDER")
    );
}
#[test]
fn fi_07_empty_policy_with_absent_or_positive_count_evidence_has_an_explained_nonpass() {
    let f = Fixture::new("baseline");
    let r = f.requests();
    let v = f.results(&r);
    let mut profile = v[0].bound_sensor_profile.clone();
    profile.adequacy.acceptance_criterion_ids.clear();
    profile.adequacy.changed_behavior_ids.clear();
    profile.adequacy.negative_case_ids.clear();
    for evidence in [None, Some(&v[0].adequacy)] {
        let result = evaluate_sensor_evidence(&profile, &[], &[], evidence);
        assert!(result.has_reason("PK-ADEQ-POLICY-EMPTY"));
    }
    let mut profiles = f.profiles.clone();
    profiles[0].adequacy = profile.adequacy;
    assert!(check_catalog(
        &f.bytes,
        &f.approved,
        "AUTH-01 STMT-04",
        &f.suites,
        &profiles
    )
    .unwrap_err()
    .to_string()
    .contains("TJ-CATALOG-COMPLETENESS"));
}
#[test]
fn fi_08_missing_ac_negative_and_unexecuted_mapping_have_precise_core_reasons() {
    let f = Fixture::new("baseline");
    let r = f.requests();
    let v = f.results(&r);
    for (case, expected) in [
        (0, "PK-ADEQ-CRITERION-UNMAPPED"),
        (1, "PK-ADEQ-BEHAVIOR-UNMAPPED"),
        (2, "PK-ADEQ-UNEXECUTED-MAPPING"),
    ] {
        let mut adequacy = v[0].adequacy.clone();
        match case {
            0 => adequacy.acceptance_criteria.clear(),
            1 => adequacy
                .changed_behaviors
                .retain(|m| !m.requirement_id.ends_with("N01")),
            _ => {
                adequacy.acceptance_criteria[0].test_ids =
                    vec!["backend:contract:not-from-this-run".into()]
            }
        }
        let result =
            evaluate_sensor_evidence(&v[0].bound_sensor_profile, &[], &[], Some(&adequacy));
        assert!(result.has_reason(expected), "{case}: {:?}", result.reasons);
    }
}
#[test]
fn fi_09_caller_claimed_trust_is_not_a_harness_import_path() {
    let f = Fixture::new("baseline");
    let r = f.requests();
    let mut v = f.results(&r);
    let sensor = &v[0].sensors[0];
    let mut forged = sensor.collection.clone();
    forged.evidence.trust = TrustClass::TrustedRunnerAttested;
    let normalized = project_verification::run::normalize_collection(
        &r[0],
        sensor.configuration.clone(),
        forged,
    )
    .unwrap();
    assert_eq!(normalized.evidence.trust, TrustClass::LocalUntrusted);
    assert_eq!(
        normalized.collection.evidence.trust,
        TrustClass::LocalUntrusted
    );
    let bundle = bundle(&r[0], &v[0], &now()).unwrap();
    assert!(
        validate_provenance(EvaluationMode::Authoritative, &bundle, &now(), 3600)
            .reason_codes()
            .contains(&"PK-PROV-UNTRUSTED")
    );
    v[0].sensors[0].evidence.trust = TrustClass::TrustedRunnerAttested;
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-TRUST"));
}

#[test]
fn catalog_rejects_missing_parent_duplicate_id_list_removal_and_joint_shrink() {
    let f = Fixture::new("baseline");
    assert!(
        check_catalog(&f.bytes, &f.approved, "STMT-04", &f.suites, &f.profiles)
            .unwrap_err()
            .to_string()
            .contains("TJ-CATALOG-PARENT")
    );
    let mut value: Value = serde_json::from_slice(&f.bytes).unwrap();
    value["requirements"][1] = value["requirements"][0].clone();
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(check_catalog(
        &bytes,
        &bytes_digest(&bytes),
        "AUTH-01 STMT-04",
        &f.suites,
        &f.profiles
    )
    .unwrap_err()
    .to_string()
    .contains("TJ-CATALOG-ID"));
    let mut profiles = f.profiles.clone();
    profiles[0].adequacy.acceptance_criterion_ids.clear();
    assert!(check_catalog(
        &f.bytes,
        &f.approved,
        "AUTH-01 STMT-04",
        &f.suites,
        &profiles
    )
    .is_err());
    value["requirements"].as_array_mut().unwrap().remove(0);
    assert!(check_catalog(
        &serde_json::to_vec(&value).unwrap(),
        &f.approved,
        "AUTH-01 STMT-04",
        &f.suites,
        &profiles
    )
    .unwrap_err()
    .to_string()
    .contains("TJ-CATALOG-DIGEST"));
}
#[test]
fn discovery_rejects_renamed_ambiguous_skipped_retry_zero_and_malformed_reports() {
    let f = Fixture::new("baseline");
    let suite = &f.suites.frontend.suites["unit"];
    let original = report(&f.root, "frontend", "unit", "baseline");
    let raw = link_report(&f.root, "frontend:unit", suite, &original).unwrap();
    let selector = &f
        .catalog
        .requirements
        .iter()
        .find(|r| r.component == "frontend")
        .unwrap()
        .tests[0];
    let mut renamed = selector.clone();
    renamed.selector = "renamed test".into();
    assert!(resolve(&renamed, &raw, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-SELECTOR-MISSING"));
    let mut duplicate = raw.clone();
    duplicate.extend(raw.clone());
    assert!(resolve(selector, &duplicate, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-SELECTOR-AMBIGUOUS"));
    let mut skipped = raw.clone();
    skipped[0].outcome = project_verification::reports::TestOutcome::Skipped;
    assert!(resolve(selector, &skipped, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-REQUIREMENT-UNPROVEN"));
    assert!(link_report(&f.root, "frontend:unit", suite, b"{").is_err());
    let dir = f.root.join("discovery");
    fs::create_dir(&dir).unwrap();
    for (id, c, s) in f.suites.entries() {
        save(
            &dir.join(format!("{}/{}", c.cwd, s.report)),
            report(&f.root, &c.cwd, id.split(':').nth(1).unwrap(), "baseline"),
        );
    }
    assert!(!discovery(&f.root, &dir, &f.suites).unwrap().is_empty());
    let p = dir.join("frontend/.policy-reports/e2e.json");
    let original = fs::read(&p).unwrap();
    let mut retry: Value = serde_json::from_slice(&original).unwrap();
    retry["suites"][0]["specs"][0]["tests"][0]["results"][0]["retry"] = json!(1);
    save(&p, serde_json::to_vec(&retry).unwrap());
    assert!(discovery(&f.root, &dir, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-DISCOVERY-NONPASS"));
    let mut zero: Value = serde_json::from_slice(&original).unwrap();
    zero["suites"] = json!([]);
    zero["stats"]["expected"] = json!(0);
    save(&p, serde_json::to_vec(&zero).unwrap());
    assert!(discovery(&f.root, &dir, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-DISCOVERY-NONPASS"));
}

#[test]
fn git_archive_inventory_rejects_old_commit_changed_and_extra_source_and_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let repo = base.join("repository");
    let archive = base.join("archive");
    fs::create_dir(&repo).unwrap();
    fs::create_dir(&archive).unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_AUTHOR_NAME", "synthetic")
            .env("GIT_AUTHOR_EMAIL", "synthetic@example.invalid")
            .env("GIT_COMMITTER_NAME", "synthetic")
            .env("GIT_COMMITTER_EMAIL", "synthetic@example.invalid")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    };
    git(&["init", "--quiet"]);
    save(&repo.join("backend/Source.cs"), "synthetic old source");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "synthetic H0"]);
    let h0 = CommitSha::parse(git(&["rev-parse", "HEAD"])).unwrap();
    save(&archive.join("backend/Source.cs"), "synthetic old source");
    assert!(git_inventory(&repo, &archive, &h0).is_ok());
    save(&repo.join("backend/Source.cs"), "synthetic current source");
    git(&["add", "."]);
    git(&["commit", "--quiet", "-m", "synthetic H"]);
    let head = CommitSha::parse(git(&["rev-parse", "HEAD"])).unwrap();
    save(&archive.join("backend/Source.cs"), "synthetic old source");
    assert!(git_inventory(&repo, &archive, &h0)
        .unwrap_err()
        .to_string()
        .contains("TJ-GIT-HEAD"));
    assert!(git_inventory(&repo, &archive, &head)
        .unwrap_err()
        .to_string()
        .contains("TJ-SOURCE-CHANGED"));
    save(
        &archive.join("backend/Source.cs"),
        "synthetic current source",
    );
    assert!(git_inventory(&repo, &archive, &head).is_ok());
    save(
        &archive.join("backend/Rogue.cs"),
        "synthetic omitted source",
    );
    assert!(git_inventory(&repo, &archive, &head)
        .unwrap_err()
        .to_string()
        .contains("TJ-SOURCE-UNTRACKED"));
    fs::remove_file(archive.join("backend/Rogue.cs")).unwrap();
    #[cfg(unix)]
    {
        fs::remove_file(archive.join("backend/Source.cs")).unwrap();
        std::os::unix::fs::symlink(
            repo.join("backend/Source.cs"),
            archive.join("backend/Source.cs"),
        )
        .unwrap();
        assert!(git_inventory(&repo, &archive, &head)
            .unwrap_err()
            .to_string()
            .contains("TJ-PATH: symlink"));
    }
}

#[test]
fn theory_rows_keep_each_raw_identity_and_foreign_checkout_paths_are_rejected() {
    let f = Fixture::new("baseline");
    let suite = &f.suites.backend.suites["contract"];
    let original = String::from_utf8(report(&f.root, "backend", "contract", "baseline")).unwrap();
    let first = original.replace(
        "TjArch.Tests.contractTests.Check\"",
        "TjArch.Tests.contractTests.Check(value: 1)\"",
    );
    let row = first
        .split("<Results>")
        .nth(1)
        .unwrap()
        .split("</Results>")
        .next()
        .unwrap();
    let second = row
        .replace("synthetic-contract", "synthetic-contract-row2")
        .replace("value: 1", "value: 2");
    let two = first
        .replace("</Results>", &format!("{second}</Results>"))
        .replace(
            "total=\"1\" executed=\"1\" passed=\"1\"",
            "total=\"2\" executed=\"2\" passed=\"2\"",
        );
    let raw = link_report(&f.root, "backend:contract", suite, two.as_bytes()).unwrap();
    let selector = &f.catalog.requirements[0].tests[0];
    let ids = resolve(selector, &raw, &f.suites).unwrap();
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
    assert_ne!(raw[0].raw_identity, raw[1].raw_identity);
    let mut foreign: Value =
        serde_json::from_slice(&report(&f.root, "frontend", "unit", "baseline")).unwrap();
    foreign["testResults"][0]["name"] = json!("/different-checkout/frontend/src/Statement.test.ts");
    assert!(link_report(
        &f.root,
        "frontend:unit",
        &f.suites.frontend.suites["unit"],
        &serde_json::to_vec(&foreign).unwrap()
    )
    .unwrap_err()
    .to_string()
    .contains("TJ-RAW-SOURCE"));
}

#[test]
fn split_inventory_profile_final_identity_and_freshness_must_all_match() {
    let f = Fixture::new("baseline");
    let mut r = f.requests();
    let mut v = f.results(&r);
    let original = r.clone();
    r[1].expected_source_digest = bytes_digest(b"different inventory");
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-SPLIT-BINDING"));
    r = original.clone();
    r[1].source_files.pop();
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-SPLIT-BINDING"));
    r = original.clone();
    v[0].project_profile_digest = bytes_digest(b"different profile");
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-RESULT-BINDING"));
    v[0].project_profile_digest = r[0].expected_profile_digest.clone();
    let future = UtcTimestamp::parse("2099-01-01T00:00:00Z").unwrap();
    let report = evaluate_pair(&r, &v, &future, 3600, &f.catalog, &f.suites).unwrap();
    assert!(report.reasons.iter().any(|r| r.code == "PK-PROV-STALE"));
    r[0].mappings.acceptance_criteria[0].test_ids = vec!["backend:contract:invented".into()];
    assert!(evaluate_pair(&r, &v, &now(), 3600, &f.catalog, &f.suites)
        .unwrap_err()
        .to_string()
        .contains("TJ-FINAL-MAPPING"));
}

#[test]
fn missing_required_process_and_wrong_suite_argv_or_classes_cannot_pass_preflight() {
    let f = Fixture::new("baseline");
    let mut p = f.profiles.clone();
    p[1].sensors
        .retain(|s| s.definition.id.as_str() != "frontend:fsd");
    assert!(
        check_catalog(&f.bytes, &f.approved, "AUTH-01 STMT-04", &f.suites, &p)
            .unwrap_err()
            .to_string()
            .contains("TJ-PROCESS-MISSING")
    );
    let mut suites = f.suites.clone();
    suites
        .frontend
        .suites
        .get_mut("unit")
        .unwrap()
        .argv
        .push("changed-command".into());
    assert!(check_catalog(
        &f.bytes,
        &f.approved,
        "AUTH-01 STMT-04",
        &suites,
        &f.profiles
    )
    .unwrap_err()
    .to_string()
    .contains("TJ-SUITE-PROFILE"));
    let report = String::from_utf8(report(&f.root, "backend", "contract", "baseline"))
        .unwrap()
        .replace("contractTests", "UnclassifiedTests");
    assert!(link_report(
        &f.root,
        "backend:contract",
        &f.suites.backend.suites["contract"],
        report.as_bytes()
    )
    .unwrap_err()
    .to_string()
    .contains("TJ-SUITE-CLASS"));
}
