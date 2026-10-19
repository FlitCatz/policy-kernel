//! Descriptor-relative, no-follow access. Never traverse a producer symlink.
use rustix::fs::{mkdirat, open, openat, Dir, Mode, OFlags};
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
};

const FLAGS: OFlags = OFlags::RDONLY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC)
    .union(OFlags::NONBLOCK);
pub(crate) const FILE_LIMIT: u64 = 16 * 1024 * 1024;
pub(crate) const TOTAL_LIMIT: u64 = 256 * 1024 * 1024;
pub(crate) const ENTRY_LIMIT: usize = 4096;

pub(crate) fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
pub(crate) fn absolute(path: &Path) -> io::Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut result = PathBuf::from("/");
    for c in path.components() {
        match c {
            Component::Normal(name) => result.push(name),
            Component::RootDir | Component::CurDir => (),
            _ => return Err(invalid("parent traversal in directory path")),
        }
    }
    Ok(result)
}
pub(crate) fn directory(path: &Path) -> io::Result<File> {
    let mut dir: File = open("/", FLAGS | OFlags::DIRECTORY, Mode::empty())?.into();
    for c in absolute(path)?.components() {
        if let Component::Normal(name) = c {
            dir = openat(&dir, name, FLAGS | OFlags::DIRECTORY, Mode::empty())?.into();
        }
    }
    Ok(dir)
}
pub(crate) fn fresh(path: &Path, input: Option<&File>) -> io::Result<File> {
    let parent = directory(
        path.parent()
            .ok_or_else(|| invalid("output needs parent"))?,
    )?;
    // Textual prefix checks alone miss case aliases on case-insensitive volumes.
    if let Some(input) = input {
        use std::os::unix::fs::MetadataExt;
        let identity = |file: &File| -> io::Result<_> {
            let meta = file.metadata()?;
            Ok((meta.dev(), meta.ino()))
        };
        let source = identity(input)?;
        let mut ancestor = parent.try_clone()?;
        loop {
            let current = identity(&ancestor)?;
            if current == source {
                return Err(invalid("output is inside input directory"));
            }
            let next: File =
                openat(&ancestor, "..", FLAGS | OFlags::DIRECTORY, Mode::empty())?.into();
            if identity(&next)? == current {
                break;
            }
            ancestor = next;
        }
    }
    let name = path
        .file_name()
        .ok_or_else(|| invalid("output needs name"))?;
    mkdirat(&parent, name, Mode::RWXU)?;
    parent.sync_all()?;
    Ok(openat(&parent, name, FLAGS | OFlags::DIRECTORY, Mode::empty())?.into())
}
pub(crate) fn write(dir: &File, name: &str, bytes: &[u8]) -> io::Result<()> {
    let mut file: File = openat(
        dir,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::RUSR | Mode::WUSR,
    )?
    .into();
    file.write_all(bytes)?;
    file.sync_all()?;
    dir.sync_all()
}
pub(crate) fn complete(dir: &File) -> io::Result<()> {
    write(
        dir,
        "complete.pending",
        b"import record complete; not a scope decision\n",
    )?;
    rustix::fs::linkat(
        dir,
        "complete.pending",
        dir,
        "complete",
        rustix::fs::AtFlags::empty(),
    )?;
    dir.sync_all()?;
    rustix::fs::unlinkat(dir, "complete.pending", rustix::fs::AtFlags::empty())?;
    dir.sync_all()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Entry {
    pub path: String,
    pub kind: rustix::fs::FileType,
    pub stat: (u64, u64, u64, i64, i64, i64, i64),
}
pub(crate) fn inventory(root: &File) -> io::Result<Vec<Entry>> {
    fn walk(dir: &File, prefix: &str, depth: usize, entries: &mut Vec<Entry>) -> io::Result<()> {
        if depth > 32 {
            return Err(invalid("directory depth exceeds 32"));
        }
        for item in Dir::read_from(dir)? {
            let item = item?;
            let name = item
                .file_name()
                .to_str()
                .map_err(|_| invalid("non-UTF8 path"))?;
            if name == "." || name == ".." {
                continue;
            }
            if entries.len() >= ENTRY_LIMIT {
                return Err(invalid("directory exceeds 4096 entries"));
            }
            let path = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            let stat = rustix::fs::statat(dir, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)?;
            let kind = rustix::fs::FileType::from_raw_mode(stat.st_mode);
            #[allow(clippy::unnecessary_cast)]
            let stamp = (
                stat.st_dev as u64,
                stat.st_ino as u64,
                stat.st_size as u64,
                stat.st_mtime as i64,
                stat.st_mtime_nsec as i64,
                stat.st_ctime as i64,
                stat.st_ctime_nsec as i64,
            );
            entries.push(Entry {
                path: path.clone(),
                kind,
                stat: stamp,
            });
            if kind == rustix::fs::FileType::Directory {
                let child: File =
                    openat(dir, name, FLAGS | OFlags::DIRECTORY, Mode::empty())?.into();
                walk(&child, &path, depth + 1, entries)?;
            }
        }
        Ok(())
    }
    let mut entries = Vec::new();
    walk(root, "", 0, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}
pub(crate) fn safe(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}
pub(crate) fn read(root: &File, entry: &Entry) -> io::Result<Vec<u8>> {
    if !safe(&entry.path) {
        return Err(invalid("unsafe artifact path"));
    }
    let mut dir = root.try_clone()?;
    let mut parts = entry.path.split('/').peekable();
    while let Some(name) = parts.next() {
        if parts.peek().is_some() {
            dir = openat(&dir, name, FLAGS | OFlags::DIRECTORY, Mode::empty())?.into();
        } else {
            let file: File = openat(&dir, name, FLAGS, Mode::empty())?.into();
            let meta = file.metadata()?;
            use std::os::unix::fs::MetadataExt;
            if !meta.is_file()
                || meta.dev() != entry.stat.0
                || meta.ino() != entry.stat.1
                || meta.len() != entry.stat.2
            {
                return Err(invalid("artifact changed or is not regular"));
            }
            if meta.len() > FILE_LIMIT {
                return Err(invalid("artifact exceeds 16 MiB"));
            }
            let mut bytes = Vec::new();
            file.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
            if bytes.len() as u64 != meta.len() {
                return Err(invalid("artifact size changed"));
            }
            return Ok(bytes);
        }
    }
    Err(invalid("empty artifact path"))
}
