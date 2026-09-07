#![forbid(unsafe_code)]

use evidence_collector::{collect, CollectionRequest};
use policy_contracts::ExecutionStatus;
use std::{
    fs::File,
    io::{self, Read, Write},
    process::ExitCode,
};

fn run() -> Result<u8, Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args
        .next()
        .ok_or("usage: evidence-collector <request.json|->")?;
    if args.next().is_some() {
        return Err("usage: evidence-collector <request.json|->".into());
    }
    let reader: Box<dyn Read> = if path == "-" {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(path)?)
    };
    const MAX_REQUEST_BYTES: u64 = 1024 * 1024;
    let mut input = String::new();
    reader
        .take(MAX_REQUEST_BYTES + 1)
        .read_to_string(&mut input)?;
    if input.len() as u64 > MAX_REQUEST_BYTES {
        return Err("request exceeds 1 MiB limit".into());
    }
    let result = collect(CollectionRequest::try_from_json(&input)?)?;
    let code = if result.evidence.status == ExecutionStatus::Passed {
        0
    } else {
        2
    };
    let stdout = io::stdout();
    let mut output = stdout.lock();
    serde_json::to_writer(&mut output, &result)?;
    writeln!(output)?;
    Ok(code)
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("evidence-collector: {error}");
            ExitCode::from(64)
        }
    }
}
