#![forbid(unsafe_code)]

use clap::{error::ErrorKind, Parser, Subcommand};
use policy_contracts::{
    EvaluationMode, EvidenceBundle, PostMergeDecision, PreMergeDecision, PreMergeEvaluationInput,
    PreMergeStatus,
};
use policy_core::{evaluate_pre_merge, validate_bundle};
use serde_json::Value;
use std::{
    fs,
    io::{self, Read, Write},
    process::ExitCode,
};

const EXIT_BLOCKED: u8 = 2;
const EXIT_INCONCLUSIVE: u8 = 3;
const EXIT_INVALID_INPUT: u8 = 64;

#[derive(Debug, Parser)]
#[command(
    name = "policy-cli",
    version,
    about = "Local advisory policy inspection"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Evaluate native local scope (typed JSON only; no raw artifact verification).
    EvaluateScope {
        #[arg(long)]
        expected: String,
        #[arg(long)]
        observations: String,
    },
    /// Evaluate complete pre-merge input in advisory mode.
    EvaluatePre { input: String },
    /// Validate and display a decision or evidence bundle without evaluating it.
    Inspect { input: String },
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let success = matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            );
            let _ = error.print();
            return if success {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(EXIT_INVALID_INPUT)
            };
        }
    };

    match run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("policy-cli: {error}");
            ExitCode::from(EXIT_INVALID_INPUT)
        }
    }
}

fn run(cli: Cli) -> Result<u8, String> {
    match cli.command {
        Command::EvaluateScope {
            expected,
            observations,
        } => {
            if expected == "-" && observations == "-" {
                return Err("expected scope and observations cannot both use stdin".into());
            }
            let (result, exit) = policy_cli::evaluate_scope_json(
                &read_input(&expected)?,
                &read_input(&observations)?,
            )?;
            write_json(&result)?;
            Ok(exit)
        }
        Command::EvaluatePre { input } => evaluate_pre(&input),
        Command::Inspect { input } => inspect(&input),
    }
}

fn evaluate_pre(input: &str) -> Result<u8, String> {
    let bytes = read_input(input)?;
    let mut evaluation: PreMergeEvaluationInput = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid pre-merge evaluation input: {error}"))?;
    evaluation.mode = EvaluationMode::Advisory;
    let decision = evaluate_pre_merge(&evaluation);
    write_json(&decision)?;
    Ok(match decision.status {
        PreMergeStatus::ReadyToMerge => 0,
        PreMergeStatus::Blocked => EXIT_BLOCKED,
        PreMergeStatus::Inconclusive => EXIT_INCONCLUSIVE,
    })
}

fn inspect(input: &str) -> Result<u8, String> {
    let bytes = read_input(input)?;
    let is_decision = serde_json::from_slice::<PreMergeDecision>(&bytes).is_ok()
        || serde_json::from_slice::<PostMergeDecision>(&bytes).is_ok();

    if !is_decision {
        let bundle: EvidenceBundle = serde_json::from_slice(&bytes).map_err(|_| {
            "input is not a valid pre-merge decision, post-merge decision, or evidence bundle"
                .to_owned()
        })?;
        let integrity = validate_bundle(&bundle);
        if !integrity.is_valid() {
            let codes = integrity.reason_codes().join(", ");
            return Err(format!(
                "evidence bundle integrity validation failed: {codes}"
            ));
        }
    }

    let original: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON after contract validation: {error}"))?;
    write_json(&original)?;
    Ok(0)
}

fn read_input(input: &str) -> Result<Vec<u8>, String> {
    if input == "-" {
        let mut bytes = Vec::new();
        io::stdin()
            .lock()
            .read_to_end(&mut bytes)
            .map_err(|error| format!("failed to read stdin: {error}"))?;
        Ok(bytes)
    } else {
        fs::read(input).map_err(|error| format!("failed to read {input}: {error}"))
    }
}

fn write_json<T: serde::Serialize>(value: &T) -> Result<(), String> {
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, value)
        .map_err(|error| format!("failed to write JSON output: {error}"))?;
    output
        .write_all(b"\n")
        .map_err(|error| format!("failed to finish JSON output: {error}"))
}
