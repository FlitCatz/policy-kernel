#![forbid(unsafe_code)]

#[cfg(unix)]
mod command {
    use clap::{error::ErrorKind, Parser};
    use policy_contracts::{ExecutionId, SensorId};
    use policy_native_contracts::{ExpectedScopeV1, NativeRunV1, RecordId};
    use policy_native_importer::{
        import_run, populate_claims, ExpectedIdentity, MappingSelection, ReportSelection,
    };
    use std::{fs, io, path::PathBuf, process::ExitCode};

    #[derive(Parser)]
    #[command(version, about = "Verify and preserve an already-executed native run")]
    struct Args {
        #[arg(long)]
        run_directory: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        expected: PathBuf,
        #[arg(long)]
        run: PathBuf,
        /// Independently expected run ID; do not derive it from the run document.
        #[arg(long)]
        run_id: String,
        /// Independently expected scope ID; do not derive it from the run document.
        #[arg(long)]
        scope_id: String,
        /// Populate test claims from a declared raw artifact before importing; repeat per suite.
        #[arg(long, value_name = "OBSERVATION=ARTIFACT")]
        report: Vec<String>,
        /// Claim that all executed tests in a suite cover a requirement; repeat to combine suites.
        #[arg(long, value_name = "REQUIREMENT=SUITE", requires = "report")]
        map: Vec<String>,
    }

    pub fn main() -> ExitCode {
        let args = match Args::try_parse() {
            Ok(args) => args,
            Err(error) => {
                let code = if matches!(
                    error.kind(),
                    ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
                ) {
                    0
                } else {
                    64
                };
                let _ = error.print();
                return ExitCode::from(code);
            }
        };
        match run(args) {
            Ok(code) => ExitCode::from(code),
            Err(error) => {
                eprintln!("policy-native-importer: {error}");
                ExitCode::from(64)
            }
        }
    }

    fn run(args: Args) -> Result<u8, Box<dyn std::error::Error>> {
        let expected: ExpectedScopeV1 = serde_json::from_slice(&fs::read(&args.expected)?)?;
        let run: NativeRunV1 = serde_json::from_slice(&fs::read(&args.run)?)?;
        let run = if args.report.is_empty() {
            run
        } else {
            prepare_claims(&args, &expected, &run)?
        };
        let identity = ExpectedIdentity {
            run_id: ExecutionId::parse(args.run_id)?,
            scope_id: RecordId::parse(args.scope_id)?,
        };
        let record = import_run(
            &args.run_directory,
            &args.output,
            &expected,
            &identity,
            &run,
        )?;
        Ok(if record.verified_observations().is_some() {
            0
        } else {
            2
        })
    }

    fn prepare_claims(
        args: &Args,
        expected: &ExpectedScopeV1,
        run: &NativeRunV1,
    ) -> Result<NativeRunV1, Box<dyn std::error::Error>> {
        let mut reports = Vec::new();
        for value in &args.report {
            let (observation, artifact) = selection(value)?;
            reports.push(ReportSelection {
                observation_id: SensorId::parse(observation)?,
                artifact_id: RecordId::parse(artifact)?,
            });
        }
        let mut mappings = Vec::new();
        for value in &args.map {
            let (requirement, suite) = selection(value)?;
            mappings.push(MappingSelection {
                requirement_id: RecordId::parse(requirement)?,
                suite_id: SensorId::parse(suite)?,
            });
        }
        Ok(populate_claims(
            &args.run_directory,
            expected,
            run,
            &reports,
            &mappings,
        )?)
    }

    fn selection(value: &str) -> io::Result<(&str, &str)> {
        value
            .split_once('=')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "selection must be ID=ID"))
    }
}

fn main() -> std::process::ExitCode {
    #[cfg(unix)]
    {
        command::main()
    }
    #[cfg(not(unix))]
    {
        eprintln!("policy-native-importer: Unix is required for safe artifact access");
        std::process::ExitCode::from(64)
    }
}
