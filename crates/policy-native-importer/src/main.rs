#![forbid(unsafe_code)]

#[cfg(unix)]
mod command {
    use clap::{error::ErrorKind, Parser};
    use policy_contracts::ExecutionId;
    use policy_native_contracts::{ExpectedScopeV1, NativeRunV1, RecordId};
    use policy_native_importer::{import_run, ExpectedIdentity};
    use std::{fs, path::PathBuf, process::ExitCode};

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
        let expected: ExpectedScopeV1 = serde_json::from_slice(&fs::read(args.expected)?)?;
        let run: NativeRunV1 = serde_json::from_slice(&fs::read(args.run)?)?;
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
