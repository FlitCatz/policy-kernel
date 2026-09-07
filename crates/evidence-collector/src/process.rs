use crate::{CommandRecord, ProcessRecord};
use policy_contracts::ExecutionStatus;
use std::{collections::BTreeMap, ffi::OsString, time::Instant};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use super::*;
    use rustix::{
        fs::{fcntl_getfl, fcntl_setfl, OFlags},
        process::{kill_process_group, waitid, Pid, Signal, WaitIdOptions},
    };
    use std::{
        io::{self, Read},
        os::{fd::AsFd, unix::process::CommandExt},
        process::{Child, Command, Stdio},
        thread,
        time::Duration,
    };

    // Keep the leader unreaped until the last group signal. WNOWAIT prevents
    // its PID from being reused for an unrelated group between exit and cleanup.
    struct OwnedGroup {
        child: Child,
        pid: Pid,
        cleaned: bool,
    }
    impl OwnedGroup {
        fn finish(&mut self) -> io::Result<std::process::ExitStatus> {
            let signal = kill_process_group(self.pid, Signal::KILL);
            if let Err(error) = signal {
                // Darwin reports EPERM for a group containing only an unreaped
                // zombie. Accept it only after observing our child's exit;
                // permission failures for a running leader remain failures.
                let exited_zombie = cfg!(target_os = "macos")
                    && error == rustix::io::Errno::PERM
                    && waitid(
                        rustix::process::WaitId::Pid(self.pid),
                        WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
                    )?
                    .is_some();
                if error != rustix::io::Errno::SRCH && !exited_zombie {
                    return Err(io::Error::other(format!("kill process group: {error}")));
                }
            }
            let status = self.child.wait()?;
            self.cleaned = true;
            Ok(status)
        }
    }
    impl Drop for OwnedGroup {
        fn drop(&mut self) {
            if !self.cleaned {
                let _ = kill_process_group(self.pid, Signal::KILL);
                let _ = self.child.kill();
                let _ = self.child.wait();
            }
        }
    }

    fn nonblocking(pipe: &impl AsFd) -> io::Result<()> {
        fcntl_setfl(pipe, fcntl_getfl(pipe)? | OFlags::NONBLOCK)?;
        Ok(())
    }

    // One bounded read per iteration guarantees fairness and deadline checks
    // even when a child continuously floods just one of the streams.
    fn read_chunk(
        pipe: &mut impl Read,
        bytes: &mut Vec<u8>,
        limit: usize,
        eof: &mut bool,
    ) -> io::Result<bool> {
        if *eof {
            return Ok(false);
        }
        let mut buffer = [0; 8192];
        match pipe.read(&mut buffer) {
            Ok(0) => {
                *eof = true;
                Ok(false)
            }
            Ok(count) => {
                let keep = count.min(limit - bytes.len());
                bytes.extend_from_slice(&buffer[..keep]);
                Ok(keep < count)
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }

    pub(super) fn execute(
        command: &CommandRecord,
        args: &[String],
        environment: &BTreeMap<String, OsString>,
        deadline: Instant,
        record: &mut ProcessRecord,
    ) -> io::Result<()> {
        let Some(executable) = &command.resolved_executable else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "executable could not be resolved from explicit path or allowlisted PATH",
            ));
        };
        let child = Command::new(executable)
            .arg0(&command.argv[0])
            .args(args)
            .current_dir(&command.cwd)
            .env_clear()
            .envs(environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?;
        let pid = Pid::from_child(&child);
        let mut group = OwnedGroup {
            child,
            pid,
            cleaned: false,
        };
        let mut stdout = group.child.stdout.take().expect("piped stdout");
        let mut stderr = group.child.stderr.take().expect("piped stderr");
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        let (mut out_eof, mut err_eof) = (false, false);
        loop {
            let out_truncated = read_chunk(
                &mut stdout,
                &mut record.stdout,
                command.stdout_limit_bytes,
                &mut out_eof,
            )?;
            let err_truncated = read_chunk(
                &mut stderr,
                &mut record.stderr,
                command.stderr_limit_bytes,
                &mut err_eof,
            )?;
            if out_truncated || err_truncated {
                record.status = ExecutionStatus::Truncated;
                break;
            }
            if Instant::now() >= deadline {
                record.status = ExecutionStatus::TimedOut;
                break;
            }
            let exited = match waitid(
                rustix::process::WaitId::Pid(pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ) {
                Ok(status) => status.is_some(),
                Err(rustix::io::Errno::INTR) => false,
                Err(error) => return Err(io::Error::other(format!("observe child exit: {error}"))),
            };
            if exited && out_eof && err_eof {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let exit = group.finish()?;
        record.exit_code = exit.code();
        if record.status == ExecutionStatus::Passed && !exit.success() {
            record.status = ExecutionStatus::Failed;
        }
        Ok(())
    }
}

pub(crate) fn run(
    command: &CommandRecord,
    args: &[String],
    environment: &BTreeMap<String, OsString>,
    deadline: Instant,
) -> ProcessRecord {
    let mut record = ProcessRecord::empty(ExecutionStatus::Passed, None);
    if Instant::now() >= deadline {
        record.status = ExecutionStatus::TimedOut;
        return record;
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    if let Err(error) = unix::execute(command, args, environment, deadline, &mut record) {
        if record.status == ExecutionStatus::Passed {
            record.status = ExecutionStatus::Failed;
        }
        record.error = Some(error.to_string());
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (command, args, environment);
        record.status = ExecutionStatus::Failed;
        record.error = Some("unsupported platform".into());
    }
    record.completed_at = crate::now();
    record
}
