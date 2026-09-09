use crate::{invalid, profile::relative, Error};
use std::{fs::File, io::Read, path::Path};

pub(crate) fn directory(root: &Path, relative_path: &str) -> Result<std::path::PathBuf, Error> {
    if !relative(relative_path, true) {
        return Err(invalid("unsafe component path"));
    }
    let mut current = root.to_owned();
    if relative_path != "." {
        for part in relative_path.split('/') {
            current.push(part);
            let metadata = std::fs::symlink_metadata(&current)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(invalid("component must be a directory without symlinks"));
            }
        }
    }
    Ok(current)
}

#[cfg(unix)]
fn open(root: &Path, path: &str) -> Result<File, Error> {
    use rustix::fs::{open, openat, Mode, OFlags};
    if !relative(path, false) {
        return Err(invalid("unsafe input or report path"));
    }
    let flags = OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW | OFlags::NONBLOCK;
    let mut dir =
        open(root, flags | OFlags::DIRECTORY, Mode::empty()).map_err(std::io::Error::from)?;
    let mut parts = path.split('/').peekable();
    while let Some(part) = parts.next() {
        let last = parts.peek().is_none();
        let fd = openat(
            &dir,
            part,
            if last {
                flags
            } else {
                flags | OFlags::DIRECTORY
            },
            Mode::empty(),
        )
        .map_err(std::io::Error::from)?;
        if last {
            return Ok(fd.into());
        }
        dir = fd;
    }
    Err(invalid("empty file path"))
}

#[cfg(not(unix))]
fn open(_: &Path, _: &str) -> Result<File, Error> {
    Err(invalid("bounded file access requires Unix"))
}

pub(crate) fn read(root: &Path, path: &str) -> Result<Vec<u8>, Error> {
    let file = open(root, path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("input/report is not a regular file"));
    }
    let mut bytes = Vec::new();
    file.take((evidence_collector::MAX_OUTPUT_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > evidence_collector::MAX_OUTPUT_BYTES {
        return Err(invalid("input/report exceeds 16 MiB"));
    }
    Ok(bytes)
}

pub(crate) fn require_absent(root: &Path, path: &str) -> Result<(), Error> {
    match open(root, path) {
        Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(invalid(
            "report already exists; use a clean report destination",
        )),
    }
}
