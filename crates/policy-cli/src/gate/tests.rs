use super::{
    evaluate, raw_digest, resolve_required_scope, scope_profile, Evidence, GatePolicy, Result,
    Verdict,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use policy_contracts::UtcTimestamp;
use policy_native_contracts::{canonical_digest, DigestRole};
use serde_json::{json, Value};
use std::{cell::Cell, collections::BTreeMap, io::Write, path::Path};
use yaml_rust2::{Yaml, YamlLoader};

const BASE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HEAD: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MERGE: &str = "cccccccccccccccccccccccccccccccccccccccc";
const PATH: &str = ".policy-kernel/gate.json";
static NOW: std::sync::LazyLock<UtcTimestamp> =
    std::sync::LazyLock::new(|| UtcTimestamp::parse("2026-11-26T00:16:00Z").unwrap());
const LIGHT: &str = "light";

fn policy() -> Value {
    json!({
        "policy": {
            "schema_version": 5,
            "policy_digest": "e".repeat(64),
            "required_check_name": "advisory/ci",
            "max_evidence_age_seconds": 3600,
            "authorized_actors": [],
            "required_sensor_ids": [],
            "required_reviews": {"minimum_reviewers": 1, "require_provider_independence": true, "allowed_providers": []},
            "scope_rules": {
                "default_required_sensor_ids": ["verify"],
                "rules": [
                    {"when": "paths", "path_globs": ["docs/**"], "required_sensor_ids": ["docs"]},
                    {"when": "label", "label": "run-ci", "required_sensor_ids": ["extra"]}
                ]
            }
        },
        "checks": [
            {"id": "verify", "workflow": ".github/workflows/ci.yml", "job": "verify", "source": {"kind": LIGHT}},
            {"id": "docs", "workflow": ".github/workflows/ci.yml", "job": "docs", "source": {"kind": LIGHT}},
            {"id": "extra", "workflow": ".github/workflows/ci.yml", "job": "extra", "source": {"kind": LIGHT}}
        ]
    })
}

// GitHub response shapes and attempt timestamps recorded from the public API,
// reduced to the consumed fields with generic repository and test identities.
struct Fixture {
    pull: Value,
    files: Value,
    reviews: Option<Value>,
    policy: Option<Value>,
    workflows: BTreeMap<String, Workflow>,
    archives: BTreeMap<u64, Vec<u8>>,
    commits: BTreeMap<String, Value>,
    final_pull: Option<Value>,
    pull_reads: Cell<u32>,
    run_reads: Cell<u32>,
}

struct Workflow {
    run: Value,
    listing: Vec<Value>,
    jobs: Vec<Value>,
    artifacts: Vec<Value>,
    final_run: Option<Value>,
}

fn recorded() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/gate-github.json")).unwrap()
}

impl Fixture {
    fn new() -> Self {
        let data = recorded();
        let mut skipped = data["run"].clone();
        skipped["id"] = json!(1002);
        skipped["run_number"] = json!(16);
        skipped["conclusion"] = json!("skipped");
        let mut older = data["run"].clone();
        older["id"] = json!(1000);
        older["run_number"] = json!(14);
        older["run_attempt"] = json!(1);
        let mut extra = data["job"].clone();
        extra["id"] = json!(2002);
        extra["name"] = json!("extra");
        Self {
            pull: data["pull"].clone(),
            files: data["files"].clone(),
            reviews: Some(data["reviews"].clone()),
            policy: Some(policy()),
            workflows: BTreeMap::from([(
                "ci.yml".into(),
                Workflow {
                    run: data["run"].clone(),
                    listing: vec![skipped, older],
                    jobs: vec![data["job"].clone(), extra],
                    artifacts: vec![],
                    final_run: None,
                },
            )]),
            archives: BTreeMap::new(),
            commits: BTreeMap::new(),
            final_pull: None,
            pull_reads: Cell::new(0),
            run_reads: Cell::new(0),
        }
    }

    fn ci(&mut self) -> &mut Workflow {
        self.workflows.get_mut("ci.yml").unwrap()
    }

    fn response(&self, route: &str) -> Result<Vec<u8>> {
        let route = route
            .strip_prefix("/repos/owner/repo")
            .expect("repository route");
        let value = match route {
            "/pulls/42" => {
                let read = self.pull_reads.replace(self.pull_reads.get() + 1);
                if read == 0 {
                    &self.pull
                } else {
                    self.final_pull.as_ref().unwrap_or(&self.pull)
                }
                .clone()
            }
            "/pulls/42/files?per_page=100&page=1" => self.files.clone(),
            "/pulls/42/reviews?per_page=100&page=1" => {
                self.reviews.clone().ok_or("reviews inaccessible")?
            }
            path if path == format!("/contents/{PATH}?ref={BASE}") => {
                let policy = self.policy.as_ref().ok_or("not found")?;
                json!({"type":"file","encoding":"base64","path":PATH,
                    "content":STANDARD.encode(serde_json::to_vec(policy).unwrap())})
            }
            path if path.starts_with("/actions/workflows/") => {
                let (file, query) = path
                    .strip_prefix("/actions/workflows/")
                    .unwrap()
                    .split_once("/runs?")
                    .unwrap();
                assert_eq!(
                    query,
                    format!("event=pull_request&head_sha={HEAD}&per_page=100&page=1")
                );
                let workflow = &self.workflows[file];
                let mut runs = workflow.listing.clone();
                let run = if self.run_reads.get() == 0 {
                    &workflow.run
                } else {
                    workflow.final_run.as_ref().unwrap_or(&workflow.run)
                };
                runs.push(run.clone());
                self.run_reads.set(self.run_reads.get() + 1);
                json!({"total_count":runs.len(),"workflow_runs":runs})
            }
            path if path.starts_with("/git/commits/") => {
                let sha = path.strip_prefix("/git/commits/").unwrap();
                self.commits.get(sha).cloned().ok_or("commit unavailable")?
            }
            path if path.ends_with("/zip") => {
                let id: u64 = path
                    .strip_prefix("/actions/artifacts/")
                    .unwrap()
                    .strip_suffix("/zip")
                    .unwrap()
                    .parse()
                    .unwrap();
                return self
                    .archives
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| "zip unavailable".into());
            }
            path if path.starts_with("/actions/runs/") => {
                let suffix = path.strip_prefix("/actions/runs/").unwrap();
                let id: u64 = suffix.split('/').next().unwrap().parse().unwrap();
                let workflow = self
                    .workflows
                    .values()
                    .find(|workflow| workflow.run["id"] == id)
                    .expect("only the latest non-skipped run is read");
                let attempt = workflow.run["run_attempt"].as_u64().unwrap();
                if suffix == id.to_string() || suffix == format!("{id}/attempts/{attempt}") {
                    workflow.run.clone()
                } else if suffix == format!("{id}/attempts/{attempt}/jobs?per_page=100&page=1") {
                    json!({"total_count":workflow.jobs.len(),"jobs":workflow.jobs})
                } else if suffix == format!("{id}/artifacts?per_page=100&page=1") {
                    json!({"total_count":workflow.artifacts.len(),"artifacts":workflow.artifacts})
                } else {
                    panic!("unexpected attempt route {route}");
                }
            }
            _ => panic!("unexpected GET or non-base policy read: {route}"),
        };
        Ok(serde_json::to_vec(&value)?)
    }

    fn evaluate(&self) -> Evidence<'static> {
        evaluate(&|route| self.response(route), "owner/repo", 42, PATH, &NOW)
    }

    fn native_job(&mut self, id: &str, bytes: Vec<u8>, job: NativeJob) {
        let check = self.policy.as_mut().unwrap()["checks"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|check| check["id"] == id)
            .unwrap();
        check["source"] = native_policy();
        check["workflow"] = json!(format!(".github/workflows/{}", job.workflow));
        let mut artifact = recorded()["artifact"].clone();
        artifact["id"] = json!(job.artifact);
        artifact["workflow_run"]["id"] = json!(job.run);
        artifact["name"] = json!(format!("native-run-{id}"));
        artifact["digest"] = json!(format!("sha256:{}", raw_digest(&bytes).sha256));
        artifact["size_in_bytes"] = json!(bytes.len());
        self.workflows
            .get_mut(job.workflow)
            .unwrap()
            .artifacts
            .push(artifact);
        self.archives.insert(job.artifact, bytes);
    }
}

#[test]
fn base_policy_scopes_light_jobs_and_reviews_remain_advisory() {
    let fixture = Fixture::new();
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Ready, "{report:?}");
    assert_eq!(report.verdict.exit_code(), 0);
    assert_eq!(fixture.run_reads.get(), 2);
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["required_checks"], json!(["extra", "verify"]));
    assert_eq!(
        value["policy_sha256"],
        raw_digest(&serde_json::to_vec(&policy()).unwrap())
            .sha256
            .as_str()
    );
    assert_eq!(
        value["workflows"][".github/workflows/ci.yml"]["run_attempt"],
        2
    );
    assert_eq!(value["workflows"][".github/workflows/ci.yml"]["id"], 1001);
    assert_eq!(value["checks"][0]["kind"], "light");
    assert!(value["checks"][0]["native"].is_null());
    assert_eq!(value["advisory_reviews"][0]["state"], "CHANGES_REQUESTED");
}

#[test]
fn failed_light_job_returns_not_ready_even_if_reviews_are_unavailable() {
    let mut fixture = Fixture::new();
    fixture.reviews = None;
    fixture.ci().jobs[0]["conclusion"] = json!("failure");
    fixture.ci().run["conclusion"] = json!("failure");
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::NotReady, "{report:?}");
    assert_eq!(report.verdict.exit_code(), 2);
    assert!(report.reason.contains("failure"));
    assert!(report.advisory_review_error.is_some());
}

#[test]
fn a_move_during_reading_overrides_an_otherwise_ready_verdict() {
    for branch in ["head", "base"] {
        let mut fixture = Fixture::new();
        let mut changed = fixture.pull.clone();
        changed[branch]["sha"] = json!("c".repeat(40));
        fixture.final_pull = Some(changed);
        let report = fixture.evaluate();
        assert_eq!(report.verdict, Verdict::Unknown, "{report:?}");
        assert_eq!(report.verdict.exit_code(), 3);
        assert!(report.reason.contains("obsolete evidence"));
    }
}

#[test]
fn missing_policy_or_unreadable_scope_never_returns_ready() {
    let mut fixture = Fixture::new();
    fixture.policy = None;
    assert!(fixture
        .evaluate()
        .reason
        .contains("policy unavailable at PR base"));
    let mut fixture = Fixture::new();
    fixture.pull["labels"] = Value::Null;
    assert_eq!(fixture.evaluate().verdict, Verdict::Unknown);
    let mut fixture = Fixture::new();
    fixture.files = json!([]);
    assert!(fixture
        .evaluate()
        .reason
        .contains("incomplete GitHub collection"));
}

#[test]
fn a_job_carried_forward_in_rerun_is_not_latest_attempt_evidence() {
    let mut fixture = Fixture::new();
    fixture.ci().jobs[0]["started_at"] = json!("2026-11-25T14:54:42Z");
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown, "{report:?}");
    assert!(report.reason.contains("older attempt"));
}

#[test]
fn a_new_unfinished_run_prevents_fallback_to_a_prior_success() {
    let mut fixture = Fixture::new();
    let mut queued = fixture.ci().run.clone();
    queued["run_attempt"] = json!(1);
    queued["status"] = json!("queued");
    queued["conclusion"] = Value::Null;
    fixture.ci().run = queued;
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown, "{report:?}");
    assert!(report.reason.contains("no completed conclusion"));
}

#[test]
fn a_new_attempt_during_evaluation_invalidates_completed_evidence() {
    let mut fixture = Fixture::new();
    let mut rerun = fixture.ci().run.clone();
    rerun["run_attempt"] = json!(3);
    rerun["status"] = json!("queued");
    rerun["conclusion"] = Value::Null;
    fixture.ci().final_run = Some(rerun);
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown, "{report:?}");
    assert!(report.reason.contains("latest run or attempt changed"));
}

#[test]
fn renames_require_checks_for_the_old_and_new_paths() {
    let mut fixture = Fixture::new();
    fixture.files =
        json!([{"filename":"src/lib.rs","status":"renamed","previous_filename":"docs/lib.md"}]);
    let mut docs = fixture.ci().jobs[0].clone();
    docs["id"] = json!(2003);
    docs["name"] = json!("docs");
    fixture.ci().jobs.push(docs);
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Ready, "{report:?}");
    assert_eq!(
        serde_json::to_value(report).unwrap()["required_checks"],
        json!(["docs", "extra", "verify"])
    );
}

fn native_policy() -> Value {
    let argv = json!({"argv":["verify"],"cwd":"."});
    json!({
        "kind":"native",
        "requirements":{
            "approved_file_digest": raw_digest(b"verify"),
            "observations":[{"id":"unit","accepted_exit_codes":[0],"tests":null}],
            "requirements":[{"id":"process","process_checks":["unit"]}],
            "exemptions":[]
        },
        "bindings":[{"id":"unit","command_digest":canonical_digest(DigestRole::Command,&argv).unwrap(),
            "configuration_digest":canonical_digest(DigestRole::Configuration,&"v1").unwrap(),"parser":null}]
    })
}

#[derive(Clone, Copy)]
struct NativeJob {
    workflow: &'static str,
    run: u64,
    artifact: u64,
}

const VERIFY: NativeJob = NativeJob {
    workflow: "ci.yml",
    run: 1001,
    artifact: 3001,
};
const EXTRA: NativeJob = NativeJob {
    workflow: "extra.yml",
    run: 2001,
    artifact: 3002,
};

fn native_zip(id: &str, job: NativeJob, log: &[u8], code: i32) -> Vec<u8> {
    native_zip_at(id, job, log, code, HEAD)
}

fn native_zip_at(id: &str, job: NativeJob, log: &[u8], code: i32, head: &str) -> Vec<u8> {
    let policy = native_policy();
    let checkout = ("owner/repo", head);
    let subject = json!({
        "source": {
            "repository":"owner/repo","head":head,
            "source_set_digest":canonical_digest(DigestRole::SourceSet,&checkout).unwrap(),
            "source_inventory_digest":raw_digest(format!(r#"["owner/repo","{head}"]"#).as_bytes()),
            "scope_definition_digest":canonical_digest(DigestRole::ScopeDefinition,
                &(id, &policy["requirements"], &policy["bindings"])).unwrap()
        },
        "run_id":format!("{}-{id}",job.run),
        "scope_id":id
    });
    let run = json!({
        "schema_version":1,
        "subject":subject,
        "observations":[{
            "id":"unit","execution_id":"unit-execution","subject":subject,
            "command":{"argv":["verify"],"cwd":"."},
            "configuration_digest":policy["bindings"][0]["configuration_digest"],
            "started_at":"2026-11-26T00:08:00Z",
            "completed_at":"2026-11-26T00:08:01Z",
            "observed_at":"2026-11-26T00:08:02Z",
            "completion":{"kind":"exited","code":code}
        }],
        "artifacts":[{"id":"log","path":"command.log","role":"command_log","digest":raw_digest(b"ran verify")}],
        "test_observations":[],"mappings":[],"exclusions":[]
    });
    let mut archive = rawzip::ZipArchiveWriter::new(Vec::new());
    let manifest = serde_json::to_vec(&run).unwrap();
    for (name, bytes) in [
        ("run.json", manifest.as_slice()),
        ("artifacts/command.log", log),
    ] {
        let (mut entry, config) = archive
            .new_file(name)
            .compression_method(rawzip::CompressionMethod::DEFLATE)
            .start()
            .unwrap();
        let encoder =
            flate2::write::DeflateEncoder::new(&mut entry, flate2::Compression::default());
        let mut writer = config.wrap(encoder);
        writer.write_all(bytes).unwrap();
        let (encoder, descriptor) = writer.finish().unwrap();
        encoder.finish().unwrap();
        entry.finish(descriptor).unwrap();
    }
    archive.finish().unwrap()
}

#[test]
fn merge_preserves_same_named_sensors_across_jobs_and_runs() {
    let mut fixture = Fixture::new();
    let mut run = fixture.ci().run.clone();
    run["id"] = json!(EXTRA.run);
    run["workflow_id"] = json!(78);
    run["path"] = json!(".github/workflows/extra.yml");
    let mut job = fixture.ci().jobs.pop().unwrap();
    job["run_id"] = json!(EXTRA.run);
    fixture.workflows.insert(
        EXTRA.workflow.into(),
        Workflow {
            run,
            listing: vec![],
            jobs: vec![job],
            artifacts: vec![],
            final_run: None,
        },
    );
    fixture.native_job(
        "verify",
        native_zip("verify", VERIFY, b"ran verify", 0),
        VERIFY,
    );
    fixture.native_job("extra", native_zip("extra", EXTRA, b"ran verify", 0), EXTRA);
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Ready, "{report:?}");
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["workflows"].as_object().unwrap().len(), 2);
    for check in value["checks"].as_array().unwrap() {
        assert_eq!(check["kind"], "native_run_v1");
        assert_eq!(check["native"]["observations"], json!(["unit"]));
        assert_eq!(check["native"]["evaluation"]["status"], "satisfied");
        assert_eq!(check["native"]["import"]["issues"], json!([]));
    }
    assert_ne!(
        value["checks"][0]["native"]["evaluation"]["subject"]["run_id"],
        value["checks"][1]["native"]["evaluation"]["subject"]["run_id"]
    );
}

#[test]
fn synthetic_checkout_must_match_the_pr_head_tree() {
    let mut fixture = Fixture::new();
    fixture.commits = BTreeMap::from([
        (
            HEAD.into(),
            json!({"sha":HEAD,"tree":{"sha":"d".repeat(40)},"parents":[]}),
        ),
        (
            MERGE.into(),
            json!({"sha":MERGE,"tree":{"sha":"d".repeat(40)},"parents":[{"sha":BASE},{"sha":HEAD}]}),
        ),
    ]);
    fixture.native_job(
        "verify",
        native_zip_at("verify", VERIFY, b"ran verify", 0, MERGE),
        VERIFY,
    );
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Ready, "{report:?}");
    fixture.commits.get_mut(MERGE).unwrap()["tree"]["sha"] = json!("e".repeat(40));
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown, "{report:?}");
    assert!(report.reason.contains("does not match the PR head"));
}

#[test]
fn missing_expired_and_conflicting_artifacts_fail_closed() {
    let with_native = || {
        let mut fixture = Fixture::new();
        fixture.native_job(
            "verify",
            native_zip("verify", VERIFY, b"ran verify", 0),
            VERIFY,
        );
        fixture
    };
    let mut fixture = with_native();
    fixture.ci().artifacts[0]["created_at"] = json!("2026-11-25T14:57:00Z");
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown);
    assert!(report.reason.contains("missing artifact native-run-verify"));

    let mut fixture = with_native();
    fixture.ci().artifacts[0]["expired"] = json!(true);
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown);
    assert!(report.reason.contains("expired"));

    let mut fixture = with_native();
    let mut duplicate = fixture.ci().artifacts[0].clone();
    duplicate["id"] = json!(3003);
    fixture.ci().artifacts.push(duplicate);
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown);
    assert!(report.reason.contains("conflicting duplicate"));
}

#[test]
fn failed_native_job_without_artifact_is_not_ready() {
    for (conclusion, verdict, reason) in [
        (
            "failure",
            Verdict::NotReady,
            "required job concluded failure",
        ),
        (
            "timed_out",
            Verdict::NotReady,
            "required job concluded timed_out",
        ),
        (
            "cancelled",
            Verdict::Unknown,
            "required native job concluded cancelled",
        ),
    ] {
        let mut fixture = Fixture::new();
        fixture.native_job(
            "verify",
            native_zip("verify", VERIFY, b"ran verify", 0),
            VERIFY,
        );
        fixture.ci().artifacts.clear();
        let job = fixture
            .ci()
            .jobs
            .iter_mut()
            .find(|job| job["name"] == "verify")
            .unwrap();
        job["conclusion"] = json!(conclusion);
        fixture.ci().run["conclusion"] = json!("failure");
        let get = |route: &str| {
            assert!(!route.contains("/artifacts"), "unexpected read: {route}");
            fixture.response(route)
        };
        let report = evaluate(&get, "owner/repo", 42, PATH, &NOW);
        assert_eq!(report.verdict, verdict, "{conclusion}: {report:?}");
        assert!(report.reason.contains(reason), "{report:?}");
    }
}

#[test]
fn unsuccessful_native_job_with_valid_artifact_is_unknown() {
    for conclusion in ["cancelled", "skipped", "neutral"] {
        let mut fixture = Fixture::new();
        fixture.native_job(
            "verify",
            native_zip("verify", VERIFY, b"ran verify", 0),
            VERIFY,
        );
        fixture.ci().jobs[0]["conclusion"] = json!(conclusion);

        let get = |route: &str| {
            assert!(!route.contains("/artifacts"), "unexpected read: {route}");
            fixture.response(route)
        };
        let report = evaluate(&get, "owner/repo", 42, PATH, &NOW);
        assert_eq!(report.verdict, Verdict::Unknown, "{conclusion}: {report:?}");
        assert!(report.reason.contains(conclusion), "{report:?}");
    }
}

#[test]
fn native_importer_rejects_inconsistent_raw_bytes_and_verifier_reports_nonpass() {
    let mut fixture = Fixture::new();
    fixture.native_job(
        "verify",
        native_zip("verify", VERIFY, b"other bytes", 0),
        VERIFY,
    );
    let report = fixture.evaluate();
    assert_eq!(report.verdict, Verdict::Unknown, "{report:?}");
    let json = serde_json::to_string(&report).unwrap();
    assert!(json.contains("raw SHA-256 mismatch"));

    fixture.ci().artifacts.clear();
    fixture.native_job(
        "verify",
        native_zip("verify", VERIFY, b"ran verify", 9),
        VERIFY,
    );
    assert_eq!(fixture.evaluate().verdict, Verdict::NotReady);
}

// Policy-kernel's own gate policy must stay valid for the gate parser and the v5
// scope resolver, and every job it names must exist in a PR-triggered workflow.
#[test]
fn own_gate_policy_scopes_jobs_present_in_pull_request_workflows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let settings: GatePolicy =
        serde_json::from_slice(&std::fs::read(root.join(PATH)).unwrap()).unwrap();
    settings.validate().unwrap();
    let profile = scope_profile(&settings.checks);
    let scope = |labels: &[&str]| {
        let labels: Vec<_> = labels.iter().map(ToString::to_string).collect();
        resolve_required_scope(&settings.policy, &profile, &["README.md".into()], &labels)
            .unwrap()
            .iter()
            .map(|id| id.as_str().to_owned())
            .collect::<Vec<_>>()
    };
    assert_eq!(scope(&[]), ["verify"]);
    assert_eq!(scope(&["run-ci"]), ["knowledge-graph", "verify"]);

    for check in &settings.checks {
        let source = std::fs::read_to_string(root.join(&check.workflow)).unwrap();
        let workflow = YamlLoader::load_from_str(&source).unwrap().remove(0);
        let on_pull_request = workflow["on"]
            .as_hash()
            .is_some_and(|events| events.contains_key(&Yaml::String("pull_request".into())));
        assert!(
            on_pull_request,
            "{} lacks a pull_request trigger",
            check.workflow
        );
        let jobs = workflow["jobs"].as_hash().unwrap();
        let names: Vec<_> = jobs
            .iter()
            .map(|(key, job)| job["name"].as_str().or(key.as_str()).unwrap())
            .collect();
        assert!(
            names.contains(&check.job.as_str()),
            "{} has no job named {:?}: {names:?}",
            check.workflow,
            check.job
        );
    }
}
