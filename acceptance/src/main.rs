use clap::{Parser, Subcommand};
use policy_acceptance::{
    execution::{execute, Scenario},
    stage, stage_profile,
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
        #[arg(
            long,
            required_unless_present = "profile_file",
            conflicts_with = "profile_file"
        )]
        profile: Option<String>,
        /// Source-relative schema2 profile in a complete fixture tree supplied by --examples.
        #[arg(long)]
        profile_file: Option<String>,
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
            profile_file,
            output,
        } => match profile_file {
            Some(path) => stage_profile(&examples, &path, &output),
            None => stage(
                &examples,
                profile.as_deref().expect("clap requires profile"),
                &output,
            ),
        }
        .map(|()| true),
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
