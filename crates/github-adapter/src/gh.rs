use crate::AdapterError;
use policy_contracts::WorkflowRunId;
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Duration,
};

pub trait GitHubApi {
    fn issue(&self, owner: &str, repository: &str, number: u64) -> Result<Vec<u8>, AdapterError>;
    fn label_events(
        &self,
        owner: &str,
        repository: &str,
        number: u64,
    ) -> Result<Vec<u8>, AdapterError>;
    fn pull_request(
        &self,
        owner: &str,
        repository: &str,
        number: u64,
    ) -> Result<Vec<u8>, AdapterError>;
    fn branch_protection(
        &self,
        owner: &str,
        repository: &str,
        branch: &str,
    ) -> Result<Vec<u8>, AdapterError>;
    fn check_run(
        &self,
        owner: &str,
        repository: &str,
        check_run_id: u64,
    ) -> Result<Vec<u8>, AdapterError>;
    fn workflow_run(
        &self,
        owner: &str,
        repository: &str,
        workflow_run_id: &WorkflowRunId,
    ) -> Result<Vec<u8>, AdapterError>;
    fn create_check_run(
        &self,
        owner: &str,
        repository: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, AdapterError>;
}

#[derive(Debug, Clone)]
pub struct GhCliApi {
    executable: PathBuf,
    timeout: Duration,
    output_limit: usize,
}

impl GhCliApi {
    pub fn new(executable: impl AsRef<Path>, timeout: Duration, output_limit: usize) -> Self {
        Self {
            executable: executable.as_ref().to_path_buf(),
            timeout,
            output_limit,
        }
    }

    fn get(&self, route: String) -> Result<Vec<u8>, AdapterError> {
        self.execute(
            [
                "api",
                "--hostname",
                "github.com",
                "--method",
                "GET",
                route.as_str(),
            ],
            None,
        )
    }

    fn execute<const N: usize>(
        &self,
        arguments: [&str; N],
        input: Option<&[u8]>,
    ) -> Result<Vec<u8>, AdapterError> {
        let mut command = Command::new(&self.executable);
        command
            .args(arguments)
            .env_remove("GH_HOST")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if input.is_some() {
            command.stdin(Stdio::piped());
        } else {
            command.stdin(Stdio::null());
        }

        crate::process::execute(command, input, self.timeout, self.output_limit)
    }
}

impl GitHubApi for GhCliApi {
    fn issue(&self, owner: &str, repository: &str, number: u64) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        validate_nonzero(number)?;
        self.get(format!("/repos/{owner}/{repository}/issues/{number}"))
    }

    fn label_events(
        &self,
        owner: &str,
        repository: &str,
        number: u64,
    ) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        validate_nonzero(number)?;
        self.get(format!(
            "/repos/{owner}/{repository}/issues/{number}/events"
        ))
    }

    fn pull_request(
        &self,
        owner: &str,
        repository: &str,
        number: u64,
    ) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        validate_nonzero(number)?;
        self.get(format!("/repos/{owner}/{repository}/pulls/{number}"))
    }

    fn branch_protection(
        &self,
        owner: &str,
        repository: &str,
        branch: &str,
    ) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        let branch = encode_route_segment(branch)?;
        self.get(format!(
            "/repos/{owner}/{repository}/branches/{branch}/protection"
        ))
    }

    fn check_run(
        &self,
        owner: &str,
        repository: &str,
        check_run_id: u64,
    ) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        validate_nonzero(check_run_id)?;
        self.get(format!(
            "/repos/{owner}/{repository}/check-runs/{check_run_id}"
        ))
    }

    fn workflow_run(
        &self,
        owner: &str,
        repository: &str,
        workflow_run_id: &WorkflowRunId,
    ) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        if !workflow_run_id
            .as_str()
            .parse::<u64>()
            .is_ok_and(|id| id > 0 && id.to_string() == workflow_run_id.as_str())
        {
            return Err(AdapterError::InvalidRouteSegment(
                workflow_run_id.as_str().to_owned(),
            ));
        }
        self.get(format!(
            "/repos/{owner}/{repository}/actions/runs/{workflow_run_id}"
        ))
    }

    fn create_check_run(
        &self,
        owner: &str,
        repository: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, AdapterError> {
        let (owner, repository) = validated_repository(owner, repository)?;
        if payload.len() > 16 * 1024 {
            return Err(AdapterError::PayloadTooLarge {
                actual: payload.len(),
                limit: 16 * 1024,
            });
        }
        let route = format!("/repos/{owner}/{repository}/check-runs");
        self.execute(
            [
                "api",
                "--hostname",
                "github.com",
                "--method",
                "POST",
                route.as_str(),
                "--input",
                "-",
            ],
            Some(payload),
        )
    }
}

fn validated_repository<'a>(
    owner: &'a str,
    repository: &'a str,
) -> Result<(&'a str, &'a str), AdapterError> {
    validate_simple_segment(owner)?;
    validate_simple_segment(repository)?;
    Ok((owner, repository))
}

fn validate_simple_segment(value: &str) -> Result<(), AdapterError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        Err(AdapterError::InvalidRouteSegment(value.to_owned()))
    } else {
        Ok(())
    }
}

fn encode_route_segment(value: &str) -> Result<String, AdapterError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.bytes().any(|byte| byte.is_ascii_control())
    {
        return Err(AdapterError::InvalidRouteSegment(value.to_owned()));
    }
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            write!(&mut encoded, "%{byte:02X}").expect("writing to String cannot fail");
        }
    }
    Ok(encoded)
}

fn validate_nonzero(value: u64) -> Result<(), AdapterError> {
    if value == 0 {
        Err(AdapterError::InvalidRouteSegment(value.to_string()))
    } else {
        Ok(())
    }
}
