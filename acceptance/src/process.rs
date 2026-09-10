use crate::Result;
use std::{
    fs,
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub(crate) fn command(cwd: &Path, program: &str, args: &[&str]) -> Command {
    let mut command = Command::new(program);
    command.current_dir(cwd).args(args).stdin(Stdio::null());
    command
}

pub(crate) fn checked(cwd: &Path, program: &str, args: &[&str], log: &Path) -> Result<()> {
    let output = fs::File::create(log)?;
    let mut child = command(cwd, program, args)
        .stdout(output.try_clone()?)
        .stderr(output)
        .spawn()?;
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!(
                    "{program} {args:?} failed ({status}); see {}",
                    log.display()
                )
                .into())
            };
        }
        if start.elapsed() > Duration::from_secs(600) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{program} timed out; see {}", log.display()).into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = command(
        root,
        "git",
        &[
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Policy acceptance",
            "-c",
            "user.email=acceptance@example.invalid",
        ],
    )
    .args(args)
    .env_remove("GIT_DIR")
    .env_remove("GIT_WORK_TREE")
    .output()?;
    if !output.status.success() {
        return Err(format!("git {args:?}: {}", String::from_utf8_lossy(&output.stderr)).into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

pub(crate) struct Servers(Vec<Child>);

impl Servers {
    pub(crate) fn start(root: &Path, logs: &Path) -> Result<Self> {
        let mut servers = Self(Vec::new());
        // Refuse occupied ports so tests cannot accidentally exercise another service.
        for port in [5186, 5187] {
            let _listener = std::net::TcpListener::bind(("127.0.0.1", port))?;
        }
        checked(
            &root.join("backend"),
            "dotnet",
            &[
                "build",
                "App.csproj",
                "--no-restore",
                "--configuration",
                "Release",
            ],
            &logs.join("server-build.log"),
        )?;
        for (cwd, program, args, name) in [
            (
                root.join("backend"),
                "dotnet",
                vec![
                    "bin/Release/net9.0/App.dll",
                    "--urls",
                    "http://127.0.0.1:5187",
                ],
                "backend.log",
            ),
            (
                root.join("frontend"),
                "node",
                vec![
                    "node_modules/vite/bin/vite.js",
                    "--host",
                    "127.0.0.1",
                    "--port",
                    "5186",
                    "--strictPort",
                ],
                "frontend.log",
            ),
        ] {
            let file = fs::File::create(logs.join(name))?;
            servers.0.push(
                command(&cwd, program, &args)
                    .stdout(file.try_clone()?)
                    .stderr(file)
                    .spawn()?,
            );
        }
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(60) {
            for child in &mut servers.0 {
                if child.try_wait()?.is_some() {
                    return Err("consumer server exited before readiness".into());
                }
            }
            if [5186, 5187]
                .iter()
                .all(|port| std::net::TcpStream::connect(("127.0.0.1", *port)).is_ok())
            {
                return Ok(servers);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err("consumer server readiness timed out".into())
    }
}

impl Drop for Servers {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
