//! Process-environment isolation for built-binary tests, not a filesystem sandbox.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub struct Runtime {
    _temporary: tempfile::TempDir,
    pub root: PathBuf,
    pub project: PathBuf,
    binary: PathBuf,
}

impl Runtime {
    pub fn new(binary: &Path) -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        for name in [
            "home",
            "config",
            "data",
            "cache",
            "tmp",
            "empty-path",
            "project",
        ] {
            fs::create_dir(root.join(name)).unwrap();
        }
        let copied = root.join(binary.file_name().unwrap());
        fs::copy(binary, &copied).unwrap();
        let runtime = Self {
            project: root.join("project"),
            _temporary: temporary,
            root,
            binary: copied,
        };
        // Assert that the test environment really cannot discover build tools or
        // optional executors, rather than silently using the host's PATH.
        for name in [
            "rustc",
            "cargo",
            "rustup",
            "superpowers",
            "agent-skills",
            "gsd-core",
        ] {
            assert_eq!(
                runtime
                    .environment(Command::new(name))
                    .output()
                    .unwrap_err()
                    .kind(),
                std::io::ErrorKind::NotFound,
                "{name} unexpectedly available"
            );
        }
        runtime.assert_no_agent_state();
        runtime
    }

    fn environment(&self, mut command: Command) -> Command {
        command
            .env_clear()
            .current_dir(&self.project)
            .env("PATH", self.root.join("empty-path"))
            .env("HOME", self.root.join("home"))
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("XDG_CACHE_HOME", self.root.join("cache"))
            .env("TMPDIR", self.root.join("tmp"));
        command
    }

    pub fn command(&self) -> Command {
        self.environment(Command::new(&self.binary))
    }

    pub fn assert_no_agent_state(&self) {
        for base in [
            &self.project,
            &self.root.join("home"),
            &self.root.join("config"),
        ] {
            for name in [
                ".codex",
                ".agents",
                ".claude",
                ".planning",
                ".superpowers",
                "superpowers",
                "agent-skills",
                "gsd-core",
            ] {
                assert!(
                    !base.join(name).exists(),
                    "unexpected state: {}",
                    base.join(name).display()
                );
            }
        }
    }
}

pub fn success(output: Output) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}
