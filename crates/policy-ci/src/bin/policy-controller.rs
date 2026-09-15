#![forbid(unsafe_code)]
use clap::Parser;
use policy_ci::controller::{
    produce, read_json, validate_isolation, ControllerConfig, SignedObservation,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Authenticate signed collection observations and issue current-run CI receipts")]
struct Cli {
    /// Owner-installed configuration and trust anchors outside the PR checkout.
    #[arg(long)]
    config: PathBuf,
    /// Evaluated contributor checkout; trust anchors and output must be outside it.
    #[arg(long)]
    checkout: PathBuf,
    #[arg(long)]
    signed_observation: PathBuf,
    /// New controller-owned output directory, inaccessible to contributor code.
    #[arg(long)]
    output: PathBuf,
}
fn main() {
    let args = Cli::parse();
    let result = (|| {
        let config: ControllerConfig = read_json(&args.config)?;
        validate_isolation(&args.config, &config, &args.checkout, &args.output)?;
        let signed: SignedObservation = read_json(&args.signed_observation)?;
        produce(&config, &signed, &args.output)
    })();
    if let Err(error) = result {
        eprintln!("policy-controller: {error}");
        std::process::exit(2);
    }
}
