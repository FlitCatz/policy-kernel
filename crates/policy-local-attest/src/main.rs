#![forbid(unsafe_code)]

#[cfg(unix)]
mod attest;

#[cfg(unix)]
fn main() -> std::process::ExitCode {
    attest::main()
}

#[cfg(not(unix))]
fn main() -> std::process::ExitCode {
    eprintln!("policy-local-attest requires Unix");
    std::process::ExitCode::FAILURE
}
