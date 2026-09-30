use project_verification::{parse_profile, run, ProjectProfile, RunRequest};
use std::{env, fs, process::ExitCode};

fn execute() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    match args.as_slice() {
        [command, file, flag, prefix] if command == "provision-npm" && flag == "--prefix" => {
            let profile = parse_profile(&fs::read_to_string(file)?)?;
            let inputs = project_verification::selection::ci_inputs(&profile)?;
            let version = inputs.get("npm_version").ok_or("profile has no npm selection")?;
            let prefix = std::path::Path::new(prefix);
            if !prefix.is_absolute() || prefix.exists() { return Err("npm prefix must be an absent absolute directory".into()); }
            let status = std::process::Command::new("npm").args(["install", "--prefix"]).arg(prefix)
                .args([&format!("npm@{version}"), "--no-audit", "--no-fund"]).status()?;
            if !status.success() { return Err("selected npm provisioning failed".into()); }
            let bin = prefix.join("node_modules/.bin");
            let probe = std::process::Command::new(bin.join("npm")).arg("--version").output()?;
            if !probe.status.success() || std::str::from_utf8(&probe.stdout)?.trim() != version {
                return Err("provisioned npm does not match exact selection".into());
            }
            println!("{}", bin.display());
            Ok(true)
        }
        [command, file] if command == "migrate-profile" => {
            let profile = parse_profile(&fs::read_to_string(file)?)?;
            println!("{}", serde_json::to_string_pretty(&project_verification::selection::upgrade_profile(profile)?)?);
            Ok(true)
        }
        [command, file, flag, schema] if command == "migrate-profile" && flag == "--target-schema" && schema == "4" => {
            let profile = parse_profile(&fs::read_to_string(file)?)?;
            let candidate = project_verification::selection::upgrade_frontend_profile(profile)?;
            eprintln!("schema 4 changes the profile digest and stops applying legacy producer version intervals; adopt and verify this candidate explicitly");
            println!("{}", serde_json::to_string_pretty(&candidate)?);
            Ok(true)
        }
        [command, file] if command == "ci-inputs" => {
            let profile = parse_profile(&fs::read_to_string(file)?)?;
            println!("{}", serde_json::to_string_pretty(&project_verification::selection::ci_inputs(&profile)?)?);
            Ok(true)
        }
        [command, file, flag, output] if command == "ci-inputs" && flag == "--github-output" => {
            use std::io::Write;
            let profile = parse_profile(&fs::read_to_string(file)?)?;
            let inputs = project_verification::selection::ci_inputs(&profile)?;
            let mut output = fs::OpenOptions::new().create(true).append(true).open(output)?;
            for (name, value) in &inputs { writeln!(output, "{name}={value}")?; }
            println!("{}", serde_json::to_string_pretty(&inputs)?);
            Ok(true)
        }
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
            "usage: project-verification validate-profile FILE | migrate-profile FILE [--target-schema 4] | ci-inputs FILE [--github-output FILE] | provision-npm FILE --prefix DIR | schema | prepare --input FILE | run --input FILE".into(),
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
