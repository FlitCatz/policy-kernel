use clap::{Parser, Subcommand};
use policy_acceptance::{
    execution::{execute, Scenario},
    stage,
};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(about = "Run native consumer acceptance; never grants policy authority")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Stage {
        #[arg(long, default_value = "examples")]
        examples: PathBuf,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        output: PathBuf,
    },
    Run {
        #[arg(long, default_value = "examples")]
        examples: PathBuf,
        #[arg(long)]
        profile: String,
        #[arg(long)]
        output: PathBuf,
        #[arg(long, value_enum)]
        scenario: Option<Scenario>,
        /// Restore locked native dependencies and provision the pinned browser.
        #[arg(long)]
        provision: bool,
    },
}

fn main() -> ExitCode {
    let result = match Cli::parse().command {
        Commands::Stage {
            examples,
            profile,
            output,
        } => stage(&examples, &profile, &output).map(|()| true),
        Commands::Run {
            examples,
            profile,
            output,
            scenario,
            provision,
        } => execute(&examples, &profile, &output, scenario, provision).map(|summary| {
            println!(
                "{}",
                serde_json::to_string_pretty(&summary).expect("summary serialization")
            );
            summary.status == "passed"
        }),
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(error) => {
            eprintln!("policy-acceptance: {error}");
            ExitCode::from(64)
        }
    }
}
