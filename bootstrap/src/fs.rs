use crate::{invalid, Result};
use rustix::fs::{mkdirat, open, openat, statat, unlinkat, AtFlags, Dir, Mode, OFlags};
use std::{
    fs::File,
    io::{Read, Write},
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

pub(crate) const LIMIT: u64 = 16 * 1024 * 1024;
const FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::CLOEXEC)
    .union(OFlags::NOFOLLOW)
    .union(OFlags::NONBLOCK);

pub(crate) fn relative(path: &str, root: bool) -> bool {
    (root && path == ".")
        || (!path.is_empty()
            && !path.contains(['\\', '\0', ':'])
            && path
                .split('/')
                .all(|p| !p.is_empty() && p != "." && p != ".."))
}

pub(crate) struct Root {
    pub file: File,
    pub path: PathBuf,
    pub identity: (u64, u64),
}

impl Root {
    pub fn lock(&self) -> Result<()> {
        rustix::fs::flock(
            &self.file,
            rustix::fs::FlockOperation::NonBlockingLockExclusive,
        )
        .map_err(std::io::Error::from)?;
        Ok(())
    }

    pub fn open(path: &Path) -> Result<Self> {
        let absolute = if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()?.join(path)
        };
        let mut file: File = open("/", FLAGS | OFlags::DIRECTORY, Mode::empty())
            .map_err(std::io::Error::from)?
            .into();
        let mut normalized = PathBuf::from("/");
        for component in absolute.components() {
            match component {
                Component::RootDir | Component::CurDir => {}
                Component::Normal(name) => {
                    file = openat(&file, name, FLAGS | OFlags::DIRECTORY, Mode::empty())
                        .map_err(std::io::Error::from)?
                        .into();
                    normalized.push(name);
                }
                _ => return Err(invalid("target must not contain parent traversal")),
            }
        }
        if normalized == Path::new("/") {
            return Err(invalid("filesystem root is not an installation target"));
        }
        let meta = file.metadata()?;
        Ok(Self {
            file,
            path: normalized,
            identity: (meta.dev(), meta.ino()),
        })
    }

    pub fn dir(&self, path: &str) -> Result<File> {
        if !relative(path, true) {
            return Err(invalid(format!("unsafe directory: {path}")));
        }
        let mut file = self.file.try_clone()?;
        if path != "." {
            for part in path.split('/') {
                file = openat(&file, part, FLAGS | OFlags::DIRECTORY, Mode::empty())
                    .map_err(std::io::Error::from)?
                    .into();
            }
        }
        Ok(file)
    }

    pub fn parent(&self, path: &str) -> Result<(File, String)> {
        if !relative(path, false) {
            return Err(invalid(format!("unsafe file: {path}")));
        }
        let (parent, name) = path.rsplit_once('/').unwrap_or((".", path));
        Ok((self.dir(parent)?, name.into()))
    }

    pub fn read(&self, path: &str) -> Result<Option<Vec<u8>>> {
        let (parent, name) = match self.parent(path) {
            Err(crate::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None)
            }
            other => other?,
        };
        let fd = match openat(&parent, name.as_str(), FLAGS, Mode::empty()) {
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            other => other.map_err(std::io::Error::from)?,
        };
        let file: File = fd.into();
        if !file.metadata()?.is_file() {
            return Err(invalid(format!("not a regular file: {path}")));
        }
        let mut bytes = Vec::new();
        file.take(LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMIT {
            return Err(invalid(format!("file exceeds 16 MiB: {path}")));
        }
        Ok(Some(bytes))
    }

    pub fn names(&self, path: &str) -> Result<Option<Vec<String>>> {
        let dir = match self.dir(path) {
            Err(crate::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None)
            }
            other => other?,
        };
        let mut names = Vec::new();
        for entry in Dir::read_from(&dir).map_err(std::io::Error::from)? {
            let entry = entry.map_err(std::io::Error::from)?;
            let name = entry
                .file_name()
                .to_str()
                .map_err(|_| invalid("non-UTF8 directory entry"))?;
            if name != "." && name != ".." {
                names.push(name.into());
            }
        }
        names.sort();
        Ok(Some(names))
    }

    pub fn exists(&self, path: &str) -> Result<bool> {
        let (parent, name) = match self.parent(path) {
            Err(crate::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(false)
            }
            other => other?,
        };
        match statat(&parent, name.as_str(), AtFlags::SYMLINK_NOFOLLOW) {
            Ok(_) => Ok(true),
            Err(rustix::io::Errno::NOENT) => Ok(false),
            Err(e) => Err(std::io::Error::from(e).into()),
        }
    }

    pub fn mkdir(&self, path: &str) -> Result<()> {
        let (parent, name) = self.parent(path)?;
        mkdirat(&parent, name.as_str(), Mode::RWXU).map_err(std::io::Error::from)?;
        parent.sync_all()?;
        Ok(())
    }

    pub fn create(&self, path: &str, bytes: &[u8]) -> Result<()> {
        let (parent, name) = self.parent(path)?;
        let mut file: File = openat(
            &parent,
            name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        )
        .map_err(std::io::Error::from)?
        .into();
        file.write_all(bytes)?;
        file.sync_all()?;
        parent.sync_all()?;
        Ok(())
    }

    pub fn remove(&self, path: &str, directory: bool) -> Result<()> {
        let (parent, name) = self.parent(path)?;
        unlinkat(
            &parent,
            name.as_str(),
            if directory {
                AtFlags::REMOVEDIR
            } else {
                AtFlags::empty()
            },
        )
        .map_err(std::io::Error::from)?;
        parent.sync_all()?;
        Ok(())
    }

    pub fn publish(&self, source: &str, destination: &str, replace: bool) -> Result<()> {
        let (source_parent, source_name) = self.parent(source)?;
        let (dest_parent, dest_name) = self.parent(destination)?;
        if replace {
            rustix::fs::renameat(
                &source_parent,
                source_name.as_str(),
                &dest_parent,
                dest_name.as_str(),
            )
            .map_err(std::io::Error::from)?;
        } else {
            // A hard link is an atomic no-clobber publication on Linux and macOS.
            rustix::fs::linkat(
                &source_parent,
                source_name.as_str(),
                &dest_parent,
                dest_name.as_str(),
                AtFlags::empty(),
            )
            .map_err(std::io::Error::from)?;
            self.remove(source, false)?;
        }
        dest_parent.sync_all()?;
        source_parent.sync_all()?;
        Ok(())
    }
}
