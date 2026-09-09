use project_verification::{parse_profile, run, ProjectProfile, RunRequest};
use std::{env, fs, process::ExitCode};

fn execute() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, file] if command == "validate-profile" => {
            let profile = parse_profile(&fs::read_to_string(file)?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "project_profile_digest":policy_core::canonical_sha256(&profile)?,
                    "sensor_profile":profile.sensor_profile()
                }))?
            );
            Ok(true)
        }
        [command] if command == "schema" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schemars::schema_for!(ProjectProfile))?
            );
            Ok(true)
        }
        [command, flag, file] if command == "run" && flag == "--input" => {
            let request: RunRequest = serde_json::from_str(&fs::read_to_string(file)?)?;
            let result = run(request)?;
            let passed = result
                .sensors
                .iter()
                .all(|s| s.evidence.status == policy_contracts::SensorStatus::Passed);
            println!("{}", serde_json::to_string_pretty(&result)?);
            Ok(passed)
        }
        [command, flag, file] if command == "prepare" && flag == "--input" => {
            let request: RunRequest = serde_json::from_str(&fs::read_to_string(file)?)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&project_verification::run::prepare(&request)?)?
            );
            Ok(true)
        }
        _ => Err(
            "usage: project-verification validate-profile FILE | schema | prepare --input FILE | run --input FILE".into(),
        ),
    }
}

fn main() -> ExitCode {
    match execute() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(2),
        Err(e) => {
            eprintln!("project-verification: {e}");
            ExitCode::from(64)
        }
    }
}
