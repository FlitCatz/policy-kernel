#![forbid(unsafe_code)]

use clap::{error::ErrorKind, Parser, Subcommand};
use policy_contracts::{
    EvaluationMode, EvidenceBundle, PostMergeDecision, PreMergeDecision, PreMergeEvaluationInput,
    PreMergeStatus,
};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

const EXIT_OK: u8 = 0;
const EXIT_BLOCKED: u8 = 2;
const EXIT_INCONCLUSIVE: u8 = 3;
const EXIT_INVALID_INPUT: u8 = 64;

#[derive(Debug, Parser)]
#[command(name = "policy-cli")]
#[command(about = "Local advisory policy evaluation and contract inspection")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate a pre-merge input locally in advisory mode.
    EvaluatePre { path: PathBuf },
    /// Validate and display a decision or evidence bundle without evaluating it.
    Inspect { path: PathBuf },
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let informational = matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            );
            let _ = error.print();
            return ExitCode::from(if informational {
                EXIT_OK
            } else {
                EXIT_INVALID_INPUT
            });
        }
    };

    ExitCode::from(match run(cli) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("invalid input: {message}");
            EXIT_INVALID_INPUT
        }
    })
}

fn run(cli: Cli) -> Result<u8, String> {
    match cli.command {
        Command::EvaluatePre { path } => evaluate_pre(&path),
        Command::Inspect { path } => inspect(&path),
    }
}

fn evaluate_pre(path: &Path) -> Result<u8, String> {
    let bytes = read_input(path)?;
    let mut input: PreMergeEvaluationInput =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    input.mode = EvaluationMode::Advisory;

    let decision = policy_core::evaluate_pre_merge(&input);
    write_json(&serde_json::to_value(&decision).map_err(|error| error.to_string())?)?;
    Ok(match decision.status {
        PreMergeStatus::ReadyToMerge => EXIT_OK,
        PreMergeStatus::Blocked => EXIT_BLOCKED,
        PreMergeStatus::Inconclusive => EXIT_INCONCLUSIVE,
    })
}

fn inspect(path: &Path) -> Result<u8, String> {
    let bytes = read_input(path)?;
    let valid = serde_json::from_slice::<PreMergeDecision>(&bytes).is_ok()
        || serde_json::from_slice::<PostMergeDecision>(&bytes).is_ok()
        || serde_json::from_slice::<EvidenceBundle>(&bytes).is_ok();
    if !valid {
        return Err(
            "document is not a valid pre-merge decision, post-merge decision, or evidence bundle"
                .to_owned(),
        );
    }

    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    write_json(&value)?;
    Ok(EXIT_OK)
}

fn read_input(path: &Path) -> Result<Vec<u8>, String> {
    if path.as_os_str() == "-" {
        let mut bytes = Vec::new();
        io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("cannot read stdin: {error}"))?;
        Ok(bytes)
    } else {
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

fn write_json(value: &serde_json::Value) -> Result<(), String> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, value).map_err(|error| error.to_string())?;
    output
        .write_all(b"\n")
        .map_err(|error| format!("cannot write stdout: {error}"))
}
