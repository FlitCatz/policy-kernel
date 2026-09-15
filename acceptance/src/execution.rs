//! Native acceptance orchestration; observations here never grant kernel authority.

use crate::{
    failure::FailureObservation,
    negative_verified_set,
    process::{checked, command, git, Servers},
    stage, Result,
};
use policy_contracts::{
    ArtifactDigest, CommitSha, ExecutionId, ModelId, ProviderId, RepositoryId, SensorStatus,
    TrustClass,
};
use policy_core::canonical_sha256;
use project_verification::{
    bytes_digest, parse_profile,
    profile::Impact,
    run::{AdequacyMappings, RunRequest},
    RunResult,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Scenario {
    Baseline,
    TypeError,
    ComponentFailure,
    ApiAuthorization,
    EmptyReport,
    MissingReport,
    EmptyTrxReport,
    MissingTrxReport,
    EmptyE2eReport,
    MissingE2eReport,
    ContractMismatch,
}

impl Scenario {
    pub fn name(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::TypeError => "type-error",
            Self::ComponentFailure => "component-failure",
            Self::ApiAuthorization => "api-authorization",
            Self::EmptyReport => "empty-report",
            Self::MissingReport => "missing-report",
            Self::EmptyTrxReport => "empty-trx-report",
            Self::MissingTrxReport => "missing-trx-report",
            Self::EmptyE2eReport => "empty-e2e-report",
            Self::MissingE2eReport => "missing-e2e-report",
            Self::ContractMismatch => "contract-mismatch",
        }
    }
    pub fn sensor(self) -> Option<&'static str> {
        match self {
            Self::Baseline => None,
            Self::TypeError => Some("frontend:typecheck"),
            Self::ComponentFailure | Self::EmptyReport | Self::MissingReport => {
                Some("frontend:unit")
            }
            Self::EmptyTrxReport | Self::MissingTrxReport => Some("backend:unit"),
            Self::EmptyE2eReport | Self::MissingE2eReport => Some("shared:e2e"),
            Self::ApiAuthorization => Some("backend:integration"),
            Self::ContractMismatch => Some("shared:contract"),
        }
    }
    pub fn for_profile(profile: &str) -> Result<Vec<Self>> {
        let mut cases = vec![Self::Baseline];
        match profile {
            "react-typescript" => cases.extend([
                Self::TypeError,
                Self::ComponentFailure,
                Self::EmptyReport,
                Self::MissingReport,
            ]),
            "dotnet" => cases.extend([
                Self::ApiAuthorization,
                Self::EmptyTrxReport,
                Self::MissingTrxReport,
            ]),
            "fullstack" => cases.extend([
                Self::ContractMismatch,
                Self::EmptyE2eReport,
                Self::MissingE2eReport,
            ]),
            _ => return Err("unknown profile".into()),
        }
        Ok(cases)
    }

    pub fn rejecting_sensors(self) -> Vec<&'static str> {
        if self == Self::ContractMismatch {
            vec!["shared:contract", "backend:integration", "shared:e2e"]
        } else {
            self.sensor().into_iter().collect()
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Phase {
    pub head_sha: String,
    pub status: String,
    pub sensors: BTreeMap<String, bool>,
    pub failure_observations: BTreeMap<String, FailureObservation>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Case {
    pub scenario: Scenario,
    pub status: String,
    pub phases: BTreeMap<String, Phase>,
    pub error: Option<String>,
    pub previous_head_rejected: Option<bool>,
    pub failure_attribution: BTreeMap<String, bool>,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub schema_version: u32,
    pub profile: String,
    pub status: String,
    pub local_advisory: String,
    pub ci_authoritative: String,
    pub harness_head: String,
    pub harness: crate::harness::HarnessObservation,
    pub runtime_observations: BTreeMap<String, String>,
    pub cases: Vec<Case>,
}

fn json(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn runtimes(
    root: &Path,
    profile: &project_verification::ProjectProfile,
) -> (bool, BTreeMap<String, String>) {
    let mut observations = BTreeMap::new();
    let mut available = true;
    let Some(selection) = &profile.selection else {
        return (
            false,
            BTreeMap::from([("selection".into(), "missing explicit tool selection".into())]),
        );
    };
    let requirements: std::collections::BTreeSet<_> = selection
        .runtime_files
        .iter()
        .map(|r| &r.tool)
        .chain(selection.graphs.iter().map(|g| &g.tool))
        .collect();
    for id in requirements {
        let tool = &selection.tools[id];
        let args: Vec<_> = tool.version_args.iter().map(String::as_str).collect();
        let observed = match command(root, &tool.executable, &args).output() {
            Ok(output) if output.status.success() => match tool.observe(&output.stdout) {
                Ok(version) => version,
                Err(error) => {
                    available = false;
                    format!("incompatible: {error}")
                }
            },
            Ok(output) => {
                available = false;
                format!(
                    "unavailable: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )
            }
            Err(error) => {
                available = false;
                format!("unavailable: {error}")
            }
        };
        observations.insert(
            id.clone(),
            format!(
                "executable {}; argv {:?}; requested {}; provision {}; observed {observed}",
                tool.executable,
                tool.version_args,
                tool.version.requirement,
                tool.version.provision
            ),
        );
    }
    (available, observations)
}

fn provision(root: &Path, logs: &Path) -> Result<()> {
    let profile = parse_profile(&fs::read_to_string(
        root.join(".policy/project-profile.json"),
    )?)?;
    let selection = profile
        .selection
        .as_ref()
        .ok_or("provisioning requires explicit selection")?;
    for (index, graph) in selection.graphs.iter().enumerate() {
        let component = profile
            .components
            .iter()
            .find(|c| c.id == graph.component)
            .ok_or("missing component")?;
        let argv = graph.install_argv(&selection.tools[&graph.tool]);
        let args: Vec<_> = argv[1..].iter().map(String::as_str).collect();
        checked(
            &root.join(&component.root),
            &argv[0],
            &args,
            &logs.join(format!("restore-graph-{index}.log")),
        )?;
    }
    for sensor in &profile.sensors {
        if sensor.report.as_ref().is_some_and(|r| {
            r.format == project_verification::reports::ReportFormat::PlaywrightJson
        }) {
            let component = profile
                .components
                .iter()
                .find(|c| c.id == sensor.component)
                .ok_or("missing component")?;
            let cli = sensor
                .argv
                .get(1)
                .ok_or("missing selected Playwright CLI")?;
            checked(
                &root.join(&component.root),
                &sensor.argv[0],
                &[cli, "install", "chromium"],
                &logs.join("playwright-install.log"),
            )?;
        }
    }
    Ok(())
}

fn commit(root: &Path, message: &str) -> Result<String> {
    git(root, &["add", "--all"])?;
    git(root, &["commit", "--allow-empty", "-m", message])?;
    Ok(git(root, &["rev-parse", "HEAD"])?.trim().into())
}

fn request(root: &Path, head: &str, phase: &str) -> Result<RunRequest> {
    clean_head(root, head)?;
    let profile = parse_profile(&fs::read_to_string(
        root.join(".policy/project-profile.json"),
    )?)?;
    let mut source_files = Vec::new();
    for path in git(root, &["ls-files", "-z"])?
        .split('\0')
        .filter(|s| !s.is_empty())
    {
        if !fs::symlink_metadata(root.join(path))?.is_file() {
            return Err("source inventory contains a non-file".into());
        }
        source_files.push(ArtifactDigest {
            path: path.into(),
            digest: bytes_digest(&fs::read(root.join(path))?),
        });
    }
    source_files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(RunRequest {
        schema_version: 1,
        expected_profile_digest: canonical_sha256(&profile)?,
        profile,
        root: root.canonicalize()?,
        repository: RepositoryId::parse("acceptance/consumer")?,
        head_sha: CommitSha::parse(head)?,
        expected_source_digest: canonical_sha256(&source_files)?,
        source_files,
        provider: ProviderId::parse("local")?,
        model: ModelId::parse("native-acceptance")?,
        execution_id: ExecutionId::parse(format!("{phase}-{head}"))?,
        impact: Impact::Unknown,
        mappings: AdequacyMappings::default(),
    })
}

fn clean_head(root: &Path, head: &str) -> Result<()> {
    if !git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty() {
        return Err("consumer contains modified or untracked source".into());
    }
    if git(root, &["rev-parse", "HEAD"])?.trim() != head {
        return Err("consumer HEAD differs from requested head".into());
    }
    Ok(())
}

fn retain_reports(root: &Path, directory: &Path, result: &RunResult) -> Result<()> {
    for sensor in &result.sensors {
        if let Some(report) = &sensor.configuration.sensor.report {
            let component = sensor.configuration.component_root.strip_prefix(root)?;
            let path = root.join(component).join(&report.path);
            if path.is_file() {
                let bytes = fs::read(&path)?;
                let target = directory.join("reports").join(component).join(&report.path);
                fs::create_dir_all(target.parent().ok_or("report has no parent")?)?;
                fs::write(target, bytes)?;
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

fn phase(root: &Path, profile: &str, directory: &Path, label: &str) -> Result<Phase> {
    fs::create_dir(directory)?;
    let head = git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    let mut failure_observations = BTreeMap::new();
    let observed = (|| -> Result<BTreeMap<String, bool>> {
        let request = request(root, &head, label)?;
        json(&directory.join("request.json"), &request)?;
        let _servers = if profile == "fullstack" {
            Some(Servers::start(root, directory, &request.profile)?)
        } else {
            None
        };
        let result = project_verification::run(request)?;
        json(&directory.join("result.json"), &result)?;
        failure_observations = result
            .sensors
            .iter()
            .map(|sensor| {
                (
                    sensor.evidence.sensor_id.to_string(),
                    FailureObservation::from_sensor(sensor),
                )
            })
            .collect();
        retain_reports(root, directory, &result)?;
        clean_head(root, &head)?;
        if result.sensors.iter().any(|s| {
            s.evidence.trust != TrustClass::LocalUntrusted || s.evidence.head_sha.as_str() != head
        }) {
            return Err("native observations violated local/head binding".into());
        }
        Ok(result
            .sensors
            .iter()
            .map(|s| {
                (
                    s.evidence.sensor_id.to_string(),
                    s.evidence.status == SensorStatus::Passed,
                )
            })
            .collect())
    })();
    let phase = match observed {
        Ok(sensors) => Phase {
            head_sha: head,
            status: if !sensors.is_empty() && sensors.values().all(|s| *s) {
                "passed"
            } else {
                "failed"
            }
            .into(),
            sensors,
            failure_observations,
            error: None,
        },
        Err(error) => Phase {
            head_sha: head,
            status: "unverified".into(),
            sensors: BTreeMap::new(),
            failure_observations: BTreeMap::new(),
            error: Some(error.to_string()),
        },
    };
    json(&directory.join("observation.json"), &phase)?;
    Ok(phase)
}

fn replace(root: &Path, file: &str, from: &str, to: &str) -> Result<()> {
    let path = root.join(file);
    let old = fs::read_to_string(&path)?;
    if !old.contains(from) {
        return Err(format!("mutation marker absent in {file}: {from}").into());
    }
    fs::write(path, old.replace(from, to))?;
    Ok(())
}

fn mutate(root: &Path, scenario: Scenario) -> Result<()> {
    match scenario {
        Scenario::Baseline => {}
        Scenario::TypeError => fs::write(
            root.join("src/type-error.ts"),
            "export const broken: number = 'not a number';\n",
        )?,
        Scenario::ComponentFailure => {
            replace(root, "src/App.tsx", "Hello policy", "Broken component")?
        }
        Scenario::ApiAuthorization => replace(
            root,
            "Program.cs",
            "request.Headers.Authorization.ToString()",
            "\"Bearer rejected-token\"",
        )?,
        Scenario::ContractMismatch => {
            replace(root, "backend/Program.cs", "message =", "incompatible =")?
        }
        Scenario::EmptyReport => {
            replace(
                root,
                "vitest.config.ts",
                "passWithNoTests: false",
                "passWithNoTests: true",
            )?;
            replace(
                root,
                "vitest.config.ts",
                "src/**/*.{test,spec}.{ts,tsx}",
                "acceptance-empty/**/*.test.ts",
            )?;
        }
        Scenario::MissingReport
        | Scenario::MissingTrxReport
        | Scenario::MissingE2eReport
        | Scenario::EmptyTrxReport
        | Scenario::EmptyE2eReport => {
            let path = root.join(".policy/project-profile.json");
            let mut profile = parse_profile(&fs::read_to_string(&path)?)?;
            let sensor = profile
                .sensors
                .iter_mut()
                .find(|sensor| Some(sensor.definition.id.as_str()) == scenario.sensor())
                .ok_or("missing report scenario sensor")?;
            match scenario {
                Scenario::EmptyTrxReport => sensor.argv.extend([
                    "--filter".into(),
                    "FullyQualifiedName~Task15IntentionallyAbsentTest".into(),
                ]),
                Scenario::EmptyE2eReport => sensor.argv.extend([
                    "--grep".into(),
                    "Task15IntentionallyAbsentTest".into(),
                    "--pass-with-no-tests".into(),
                ]),
                _ => {
                    // Keep the real producer command, but request a report it does not emit.
                    sensor
                        .report
                        .as_mut()
                        .ok_or("missing report contract")?
                        .path = ".policy-reports/task15-intentionally-missing-report".into();
                }
            }
            profile.validate()?;
            fs::write(path, serde_json::to_vec_pretty(&profile)?)?;
        }
    }
    Ok(())
}

fn run_case(
    examples: &Path,
    profile: &str,
    scenario: Scenario,
    directory: &Path,
    restore: bool,
) -> Result<Case> {
    fs::create_dir(directory)?;
    let root = directory.join("consumer");
    let mut case = Case {
        scenario,
        status: "unverified".into(),
        phases: BTreeMap::new(),
        error: None,
        previous_head_rejected: None,
        failure_attribution: BTreeMap::new(),
    };
    let run = (|| -> Result<()> {
        stage(examples, profile, &root)?;
        if restore {
            provision(&root, directory)?;
        }
        git(&root, &["init", "--initial-branch=acceptance"])?;
        let baseline_head = commit(&root, "Baseline consumer")?;
        let baseline = phase(&root, profile, &directory.join("baseline"), "baseline")?;
        case.phases.insert("baseline".into(), baseline);
        if scenario == Scenario::Baseline {
            case.status = case.phases["baseline"].status.clone();
            return Ok(());
        }
        mutate(&root, scenario)?;
        commit(&root, "Introduce isolated acceptance failure")?;
        let negative = phase(&root, profile, &directory.join("negative"), "negative")?;
        case.failure_attribution = scenario
            .rejecting_sensors()
            .into_iter()
            .map(|id| {
                (
                    id.to_owned(),
                    negative
                        .failure_observations
                        .get(id)
                        .is_some_and(|observed| observed.proves_sensor(scenario, id)),
                )
            })
            .collect();
        case.phases.insert("negative".into(), negative);
        if matches!(
            scenario,
            Scenario::MissingReport | Scenario::MissingTrxReport | Scenario::MissingE2eReport
        ) {
            // Preserve the real producer's unexpected output for inspection, then
            // clear it before correction. This output never satisfies the missing contract.
            let baseline: RunResult =
                serde_json::from_slice(&fs::read(directory.join("baseline/result.json"))?)?;
            retain_reports(
                &root,
                &directory.join("negative/unrequested-output"),
                &baseline,
            )?;
        }
        git(
            &root,
            &[
                "restore",
                "--source",
                &baseline_head,
                "--staged",
                "--worktree",
                "--",
                ".",
            ],
        )?;
        commit(&root, "Correct acceptance failure")?;
        let corrected = phase(&root, profile, &directory.join("corrected"), "corrected")?;
        case.phases.insert("corrected".into(), corrected);
        let stale_result = directory.join("baseline/result.json");
        if stale_result.is_file() {
            let previous: RunResult = serde_json::from_slice(&fs::read(stale_result)?)?;
            let corrected_request =
                request(&root, &case.phases["corrected"].head_sha, "corrected")?;
            let rejections: Vec<_> = previous
                .sensors
                .into_iter()
                .map(|sensor| {
                    let sensor_id = sensor.evidence.sensor_id.to_string();
                    let error = project_verification::run::normalize_collection(
                        &corrected_request,
                        sensor.configuration,
                        sensor.collection,
                    )
                    .err()
                    .map(|e| e.to_string());
                    (sensor_id, error)
                })
                .collect();
            let rejected = !rejections.is_empty()
                && rejections.iter().all(|(_, error)| {
                    error.as_ref().is_some_and(|e| {
                        e.contains("collection does not match its checked configuration or digest")
                    })
                });
            json(&directory.join("previous-head-rejection.json"), &rejections)?;
            case.previous_head_rejected = Some(rejected);
        }
        let baseline = &case.phases["baseline"];
        let negative = &case.phases["negative"];
        let corrected = &case.phases["corrected"];
        let expected = scenario.rejecting_sensors();
        if negative_verified_set(&baseline.sensors, &negative.sensors, &expected)
            && baseline.status == "passed"
            && case.failure_attribution.values().all(|proved| *proved)
            && corrected.status == "passed"
            && corrected.head_sha != negative.head_sha
            && corrected.head_sha != baseline.head_sha
            && case.previous_head_rejected == Some(true)
        {
            case.status = "verified_rejection_and_correction".into();
        } else {
            case.status = "inconclusive".into();
        }
        Ok(())
    })();
    if let Err(error) = run {
        case.error = Some(error.to_string());
    }
    json(&directory.join("case.json"), &case)?;
    Ok(case)
}

/// Output must be new. Native execution is always advisory, including on a hosted CI runner.
pub fn execute(
    examples: &Path,
    profile: &str,
    output: &Path,
    selected: Option<Scenario>,
    restore: bool,
) -> Result<Summary> {
    let allowed = Scenario::for_profile(profile)?;
    if selected.is_some_and(|case| !allowed.contains(&case)) {
        return Err("scenario does not belong to this profile".into());
    }
    crate::staging::outside_examples(examples, output)?;
    fs::create_dir(output)?;
    let output: PathBuf = output.canonicalize()?;
    let examples = examples.canonicalize()?;
    let harness = examples.parent().ok_or("examples has no parent")?;
    let selected_profile = crate::staging::builtin_profile(profile)?;
    let ci_inputs = project_verification::selection::ci_inputs(&selected_profile)?;
    // Host preflight uses the selected SDK even when the CI image has newer SDKs.
    if let Some(version) = ci_inputs.get("dotnet_version") {
        json(
            &output.join("global.json"),
            &serde_json::json!({"sdk":{"version":version,"rollForward":"disable","allowPrerelease":false}}),
        )?;
    }
    json(&output.join("selection-inputs.json"), &ci_inputs)?;
    let (available, observations) = runtimes(&output, &selected_profile);
    let harness_observation = crate::harness::observe(harness)?;
    let mut summary = Summary { schema_version: 1, profile: profile.into(), status: "unverified".into(),
        local_advisory: "not_run".into(), ci_authoritative: "unverified: requires authenticated CI observations, adopted policy and independent approvals".into(),
        harness_head: harness_observation.head.clone(), harness: harness_observation, runtime_observations: observations, cases: Vec::new() };
    json(&output.join("summary.json"), &summary)?;
    if available {
        for scenario in selected.map_or(allowed, |case| vec![case]) {
            eprintln!("acceptance: {profile}/{}", scenario.name());
            summary.cases.push(run_case(
                &examples,
                profile,
                scenario,
                &output.join(scenario.name()),
                restore,
            )?);
            json(&output.join("summary.json"), &summary)?;
        }
        summary.local_advisory = "executed; inspect individual phases".into();
        summary.status = if summary.cases.iter().all(|case| {
            matches!(
                case.status.as_str(),
                "passed" | "verified_rejection_and_correction"
            )
        }) {
            "passed"
        } else {
            "incomplete"
        }
        .into();
    }
    json(&output.join("summary.json"), &summary)?;
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorization_mutation_changes_endpoint_wiring_only() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().canonicalize().unwrap().join("consumer");
        stage(
            Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples")),
            "dotnet",
            &root,
        )
        .unwrap();
        let before = fs::read_to_string(root.join("Program.cs")).unwrap();
        mutate(&root, Scenario::ApiAuthorization).unwrap();
        let after = fs::read_to_string(root.join("Program.cs")).unwrap();
        assert_eq!(
            before
                .split("public static class GreetingAuthorization")
                .nth(1),
            after
                .split("public static class GreetingAuthorization")
                .nth(1)
        );
        assert_ne!(before, after);
    }

    #[test]
    fn mutation_and_correction_have_distinct_git_heads_and_exact_source_bindings() {
        let tmp = tempfile::tempdir().unwrap();
        let tmp = tmp.path().canonicalize().unwrap();
        let root = tmp.join("consumer");
        let examples = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../examples"));
        stage(examples, "react-typescript", &root).unwrap();
        git(&root, &["init", "--initial-branch=acceptance"]).unwrap();
        let baseline_head = commit(&root, "baseline").unwrap();
        let baseline = request(&root, &baseline_head, "baseline").unwrap();
        mutate(&root, Scenario::TypeError).unwrap();
        assert!(
            request(&root, &baseline_head, "dirty").is_err(),
            "untracked source must invalidate a clean-head request"
        );
        let negative_head = commit(&root, "negative").unwrap();
        let negative = request(&root, &negative_head, "negative").unwrap();
        assert_ne!(baseline.head_sha, negative.head_sha);
        assert_ne!(
            baseline.expected_source_digest,
            negative.expected_source_digest
        );
        assert!(negative
            .source_files
            .iter()
            .any(|f| f.path == "src/type-error.ts"));
        git(
            &root,
            &[
                "restore",
                "--source",
                &baseline_head,
                "--staged",
                "--worktree",
                "--",
                ".",
            ],
        )
        .unwrap();
        let corrected_head = commit(&root, "corrected").unwrap();
        let corrected = request(&root, &corrected_head, "corrected").unwrap();
        assert_ne!(baseline.head_sha, corrected.head_sha);
        assert_ne!(negative.head_sha, corrected.head_sha);
        assert_eq!(
            baseline.expected_source_digest,
            corrected.expected_source_digest
        );
        assert!(!root.join("src/type-error.ts").exists());
    }
}
