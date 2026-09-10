use crate::AdapterError;
use std::{process::Command, time::Duration};

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use super::*;
    use rustix::{
        fs::{fcntl_getfl, fcntl_setfl, OFlags},
        process::{kill_process_group, waitid, Pid, Signal, WaitId, WaitIdOptions},
    };
    use std::{
        io::{self, Read, Write},
        os::{fd::AsFd, unix::process::CommandExt},
        process::{Child, ExitStatus},
        thread,
        time::Instant,
    };

    // Retain the unreaped leader until group cleanup to prevent PID reuse.
    struct OwnedGroup {
        child: Child,
        pid: Pid,
        cleaned: bool,
    }

    impl OwnedGroup {
        fn exited(&self) -> io::Result<bool> {
            match waitid(
                WaitId::Pid(self.pid),
                WaitIdOptions::EXITED | WaitIdOptions::NOHANG | WaitIdOptions::NOWAIT,
            ) {
                Ok(status) => Ok(status.is_some()),
                Err(rustix::io::Errno::INTR) => Ok(false),
                Err(error) => Err(error.into()),
            }
        }

        fn finish(&mut self) -> io::Result<ExitStatus> {
            if let Err(error) = kill_process_group(self.pid, Signal::KILL) {
                // Darwin can report EPERM when only the zombie leader remains.
                let zombie = cfg!(target_os = "macos")
                    && error == rustix::io::Errno::PERM
                    && self.exited()?;
                if error != rustix::io::Errno::SRCH && !zombie {
                    return Err(error.into());
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

    fn read_chunk(
        pipe: &mut impl Read,
        bytes: &mut Vec<u8>,
        limit: usize,
        eof: &mut bool,
    ) -> Result<(), AdapterError> {
        if *eof {
            return Ok(());
        }
        let mut buffer = [0; 8192];
        match pipe.read(&mut buffer) {
            Ok(0) => *eof = true,
            Ok(count) => {
                if count > limit.saturating_sub(bytes.len()) {
                    return Err(AdapterError::OutputTooLarge { limit });
                }
                bytes.extend_from_slice(&buffer[..count]);
            }
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    pub(super) fn execute(
        mut command: Command,
        input: Option<&[u8]>,
        timeout: Duration,
        limit: usize,
    ) -> Result<Vec<u8>, AdapterError> {
        let started = Instant::now();
        let child = command.process_group(0).spawn()?;
        let mut group = OwnedGroup {
            pid: Pid::from_child(&child),
            child,
            cleaned: false,
        };
        let mut stdout = group.child.stdout.take().expect("piped stdout");
        let mut stderr = group.child.stderr.take().expect("piped stderr");
        let mut stdin = group.child.stdin.take();
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        if let Some(pipe) = &stdin {
            nonblocking(pipe)?;
        }
        let mut pending = input.unwrap_or_default();
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let (mut out_eof, mut err_eof) = (false, false);
        loop {
            if started.elapsed() >= timeout {
                return Err(AdapterError::Timeout(timeout));
            }
            if let Some(pipe) = &mut stdin {
                if !pending.is_empty() {
                    match pipe.write(&pending[..pending.len().min(8192)]) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::WriteZero,
                                "GitHub CLI stdin stopped accepting input",
                            )
                            .into())
                        }
                        Ok(count) => pending = &pending[count..],
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) => {}
                        Err(error) => return Err(error.into()),
                    }
                }
                if pending.is_empty() {
                    stdin.take();
                }
            }
            read_chunk(&mut stdout, &mut out, limit, &mut out_eof)?;
            read_chunk(&mut stderr, &mut err, limit, &mut err_eof)?;
            if group.exited()? && out_eof && err_eof && stdin.is_none() {
                break;
            }
            thread::sleep(Duration::from_millis(1));
        }
        let status = group.finish()?;
        if !status.success() {
            return Err(AdapterError::ProcessFailure {
                status: status.code(),
                stderr: String::from_utf8_lossy(&err).into_owned(),
            });
        }
        Ok(out)
    }
}

pub(crate) fn execute(
    command: Command,
    input: Option<&[u8]>,
    timeout: Duration,
    limit: usize,
) -> Result<Vec<u8>, AdapterError> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        unix::execute(command, input, timeout, limit)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (command, input, timeout, limit);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "GitHub CLI process cleanup requires Linux or macOS",
        )
        .into())
    }
}
