use policy_contracts::{ArtifactDigest, Sha256Digest};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
    time::Instant,
};

#[cfg(unix)]
fn open_artifact(cwd: &Path, path: &Path) -> io::Result<File> {
    use rustix::fs::{open, openat, Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
    let mut directory = open(cwd, flags | OFlags::DIRECTORY, Mode::empty())?;
    let mut parts = path.components().peekable();
    while let Some(part) = parts.next() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(io::Error::other("artifact path contains traversal"));
        }
        let last = parts.peek().is_none();
        let opened = openat(
            &directory,
            part.as_os_str(),
            if last {
                flags
            } else {
                flags | OFlags::DIRECTORY
            },
            Mode::empty(),
        )?;
        if last {
            return Ok(opened.into());
        }
        directory = opened;
    }
    Err(io::Error::other("artifact path is empty"))
}

#[cfg(not(unix))]
fn open_artifact(_cwd: &Path, _path: &Path) -> io::Result<File> {
    Err(io::Error::other("unsupported platform"))
}

fn hash_file(cwd: &Path, path: &Path, deadline: Instant) -> io::Result<Sha256Digest> {
    let mut file = open_artifact(cwd, path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("artifact must be a regular file"));
    }
    let mut digest = Sha256::new();
    let mut total = 0;
    let mut buffer = [0; 8192];
    loop {
        if Instant::now() >= deadline {
            return Err(io::Error::other("artifact collection deadline exceeded"));
        }
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total += count;
        if total > crate::MAX_OUTPUT_BYTES {
            return Err(io::Error::other("artifact exceeds 16 MiB limit"));
        }
        digest.update(&buffer[..count]);
    }
    Ok(Sha256Digest::parse(hex::encode(digest.finalize())).expect("valid SHA-256"))
}

pub(crate) fn capture(
    cwd: &Path,
    paths: &[PathBuf],
    deadline: Instant,
) -> (Vec<ArtifactDigest>, Vec<String>) {
    let mut artifacts = vec![];
    let mut errors = vec![];
    for path in paths {
        match hash_file(cwd, path, deadline) {
            Ok(digest) => artifacts.push(ArtifactDigest {
                path: path.to_string_lossy().into_owned(),
                digest,
            }),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    (artifacts, errors)
}
