//! P7 local acceptance entry point. Real profiles and dry-run are P8/P9.
use clap::{Parser, Subcommand};
use policy_acceptance::tjerp::*;
use policy_contracts::*;
use project_verification::{bytes_digest, parse_profile};
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(about = "TJERP split-profile local advisory harness; no installation or trust import")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}
fn typed<T: serde::de::DeserializeOwned>(s: &str) -> Result<T, String> {
    serde_json::from_value(serde_json::json!(s)).map_err(|e| e.to_string())
}
#[derive(Subcommand)]
enum Action {
    Run(Box<Args>),
}
#[derive(clap::Args)]
struct Args {
    #[arg(long)]
    checkout: PathBuf,
    #[arg(long)]
    source_repository: PathBuf,
    #[arg(long, value_parser=typed::<CommitSha>)]
    source_revision: CommitSha,
    #[arg(long, value_parser=typed::<CommitSha>)]
    kernel_revision: CommitSha,
    #[arg(long)]
    profiles_directory: PathBuf,
    #[arg(long)]
    catalog: PathBuf,
    #[arg(long, value_parser=typed::<Sha256Digest>)]
    catalog_digest: Sha256Digest,
    /// Raw, fresh unscored discovery reports under backend/ and frontend/.
    #[arg(long)]
    discovery_directory: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long,default_value="local-advisory",value_parser=["local-advisory"])]
    mode: String,
    #[arg(long,default_value="baseline",value_parser=["baseline"])]
    scenario: String,
    #[arg(long, value_parser=typed::<ProviderId>)]
    provider: ProviderId,
    #[arg(long, value_parser=typed::<ModelId>)]
    model: ModelId,
    #[arg(long, value_parser=typed::<ExecutionId>)]
    execution_id: ExecutionId,
    #[arg(long, default_value_t = 3600)]
    max_age_seconds: u64,
}
fn now() -> UtcTimestamp {
    let time: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
    UtcTimestamp::parse(&time.to_rfc3339()).unwrap()
}
fn run(a: Args) -> policy_acceptance::Result<bool> {
    let root = a.checkout.canonicalize()?;
    let repo = a.source_repository.canonicalize()?;
    let profiles_dir = a.profiles_directory.canonicalize()?;
    let catalog_path = a.catalog.canonicalize()?;
    if profiles_dir != root.join("config/policy")
        || catalog_path != profiles_dir.join("requirements.json")
    {
        return Err("TJ-INPUT-PATH: use archive config/policy inputs".into());
    }
    let inventory = git_inventory(&repo, &root, &a.source_revision)?;
    let profiles = [
        parse_profile(&String::from_utf8(checked_read(
            &root,
            "config/policy/backend.json",
        )?)?)?,
        parse_profile(&String::from_utf8(checked_read(
            &root,
            "config/policy/frontend.json",
        )?)?)?,
    ];
    if profiles[0].schema_version != 3
        || profiles[1].schema_version != 4
        || profiles[0].id != "dotnet"
        || profiles[1].id != "react-typescript"
    {
        return Err("TJ-PROFILE-VERSION: backend 3, frontend 4 required".into());
    }
    let suites: Suites =
        serde_json::from_slice(&checked_read(&root, "config/policy/suites.json")?)?;
    let catalog = check_catalog(
        &checked_read(&root, "config/policy/requirements.json")?,
        &a.catalog_digest,
        &String::from_utf8(checked_read(&root, ".planning/REQUIREMENTS.md")?)?,
        &suites,
        &profiles,
    )?;
    let raw = discovery(&root, &a.discovery_directory.canonicalize()?, &suites)?;
    let identity = RunIdentity {
        repository: RepositoryId::parse("local/TJERP")?,
        head_sha: a.source_revision.clone(),
        provider: a.provider,
        model: a.model,
        execution_id: a.execution_id,
    };
    let requests = requests(
        &root, profiles, &inventory, &identity, &catalog, &suites, &raw,
    )?;
    let parent = a.output.parent().ok_or("TJ-OUTPUT-PATH")?.canonicalize()?;
    let output = parent.join(a.output.file_name().ok_or("TJ-OUTPUT-PATH")?);
    if output.starts_with(&root) || output.starts_with(&repo) {
        return Err("TJ-OUTPUT-PATH: output must be outside source trees".into());
    }
    fs::create_dir(&output)?;
    write_json(
        &output.join("context.json"),
        &serde_json::json!({"schema_version":1,"mode":a.mode,"scenario":a.scenario,"source_revision":a.source_revision,
        "kernel_revision_claim":a.kernel_revision,"harness_binary_sha256":bytes_digest(&fs::read(std::env::current_exe()?)?),"catalog_digest":a.catalog_digest,
        "source_inventory":inventory,"discovery":"unscored; not policy success","trust":"local_untrusted"}),
    )?;
    write_json(&output.join("discovery.json"), &raw)?;
    let discovery_root = a.discovery_directory.canonicalize()?;
    for (id, c, s) in suites.entries() {
        write_new(
            &output.join(format!("discovery-{}.raw", id.replace(':', "-"))),
            &checked_read(&discovery_root, &format!("{}/{}", c.cwd, s.report))?,
        )?;
    }
    let result = execute(
        &requests,
        &output,
        &catalog,
        &suites,
        now,
        a.max_age_seconds,
    );
    // Compare against the actual external commit again after all profile commands finish.
    let final_inventory = git_inventory(&repo, &root, &a.source_revision);
    if let Err(e) = final_inventory {
        write_json(
            &output.join("source-failure.json"),
            &serde_json::json!({"label":"local scope non-pass","error":e.to_string()}),
        )?;
        return Err(e);
    }
    let report = result?;
    write_json(&output.join("scope.json"), &report)?;
    println!("{} (LocalUntrusted)", report.label);
    Ok(report.reasons.is_empty())
}
fn main() {
    let Action::Run(args) = Cli::parse().command;
    match run(*args) {
        Ok(true) => {}
        Ok(false) => std::process::exit(2),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(64);
        }
    }
}
