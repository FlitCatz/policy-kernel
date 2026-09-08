use crate::AdapterError;
use policy_contracts::WorkflowRunId;
use std::{
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
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

        let started = Instant::now();
        let mut child = command.spawn()?;

        let (stdin_sender, stdin_receiver) = mpsc::channel();
        if let Some(input) = input {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| crate::binding_error("GitHub CLI stdin was unavailable"))?;
            let owned_input = input.to_vec();
            thread::spawn(move || {
                let result = stdin.write_all(&owned_input);
                drop(stdin);
                let _ = stdin_sender.send(result);
            });
        } else {
            drop(stdin_sender);
        }

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| crate::binding_error("GitHub CLI stdout was unavailable"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| crate::binding_error("GitHub CLI stderr was unavailable"))?;
        let limit = self.output_limit;
        let (stdout_sender, stdout_receiver) = mpsc::channel();
        thread::spawn(move || {
            let _ = stdout_sender.send(read_bounded(stdout, limit));
        });
        let (stderr_sender, stderr_receiver) = mpsc::channel();
        thread::spawn(move || {
            let _ = stderr_sender.send(read_bounded(stderr, limit));
        });

        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if started.elapsed() >= self.timeout {
                return timeout_child(&mut child, self.timeout);
            }
            thread::sleep(Duration::from_millis(5));
        };

        if input.is_some() {
            receive_before(&stdin_receiver, started, self.timeout)?.map_err(AdapterError::Io)?;
        }
        let (stdout, stdout_exceeded) =
            receive_before(&stdout_receiver, started, self.timeout)?.map_err(AdapterError::Io)?;
        let (stderr, stderr_exceeded) =
            receive_before(&stderr_receiver, started, self.timeout)?.map_err(AdapterError::Io)?;
        if stdout_exceeded || stderr_exceeded {
            return Err(AdapterError::OutputTooLarge { limit });
        }
        if !status.success() {
            return Err(AdapterError::ProcessFailure {
                status: status.code(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
            });
        }
        Ok(stdout)
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

fn read_bounded(mut reader: impl Read, limit: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut kept = Vec::with_capacity(limit.min(8 * 1024));
    let mut exceeded = false;
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let remaining = limit.saturating_sub(kept.len());
        kept.extend_from_slice(&buffer[..count.min(remaining)]);
        exceeded |= count > remaining;
    }
    Ok((kept, exceeded))
}

fn receive_before<T>(
    receiver: &mpsc::Receiver<T>,
    started: Instant,
    timeout: Duration,
) -> Result<T, AdapterError> {
    let remaining = timeout
        .checked_sub(started.elapsed())
        .ok_or(AdapterError::Timeout(timeout))?;
    receiver
        .recv_timeout(remaining)
        .map_err(|error| match error {
            mpsc::RecvTimeoutError::Timeout => AdapterError::Timeout(timeout),
            mpsc::RecvTimeoutError::Disconnected => {
                crate::binding_error("GitHub CLI I/O worker stopped unexpectedly")
            }
        })
}

fn timeout_child<T>(child: &mut std::process::Child, timeout: Duration) -> Result<T, AdapterError> {
    let _ = child.kill();
    let _ = child.wait();
    Err(AdapterError::Timeout(timeout))
}
