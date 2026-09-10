//! Local source observations are not trusted or reproducible-build proof.

use crate::{process::git, Result};
use policy_contracts::{ArtifactDigest, Sha256Digest};
use policy_core::canonical_sha256;
use project_verification::bytes_digest;
use serde::Serialize;
use std::{collections::BTreeSet, fs, path::Path};

#[derive(Debug, Serialize)]
pub struct HarnessObservation {
    pub head: String,
    pub dirty: bool,
    pub source_files: Vec<ArtifactDigest>,
    pub source_digest: Sha256Digest,
    pub executable_digest: Sha256Digest,
}

/// Record the current checkout and executable independently; this does not prove
/// that the executable was built from these sources or authenticate either one.
pub fn observe(root: &Path) -> Result<HarnessObservation> {
    let head = git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    let mut dirty = !git(root, &["status", "--porcelain", "--untracked-files=all"])?.is_empty();
    let inventory = git(
        root,
        &[
            "ls-files",
            "--cached",
            "--others",
            "--exclude-standard",
            "-z",
        ],
    )?;
    let paths: BTreeSet<_> = inventory
        .split('\0')
        .filter(|path| !path.is_empty())
        .collect();
    let mut source_files = Vec::new();
    for path in paths {
        let mut full = root.to_path_buf();
        let mut absent = false;
        for component in Path::new(path).components() {
            if !matches!(component, std::path::Component::Normal(_)) {
                return Err("harness inventory contains an unsafe path".into());
            }
            full.push(component);
            match fs::symlink_metadata(&full) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err("harness inventory contains a symlink".into());
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    absent = true;
                    break;
                }
                Err(error) => return Err(error.into()),
            }
        }
        if absent {
            dirty = true;
            continue;
        }
        if !fs::symlink_metadata(&full)?.is_file() {
            return Err("harness inventory contains a non-file".into());
        }
        source_files.push(ArtifactDigest {
            path: path.to_owned(),
            digest: bytes_digest(&fs::read(&full)?),
        });
    }
    Ok(HarnessObservation {
        head,
        dirty,
        source_digest: canonical_sha256(&source_files)?,
        source_files,
        executable_digest: bytes_digest(&fs::read(std::env::current_exe()?)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repository() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        git(temp.path(), &["init", "--quiet"]).unwrap();
        fs::write(temp.path().join("source.rs"), "original").unwrap();
        fs::write(temp.path().join(".gitignore"), "ignored/\n").unwrap();
        git(temp.path(), &["add", "."]).unwrap();
        git(temp.path(), &["commit", "--quiet", "-m", "baseline"]).unwrap();
        temp
    }

    #[test]
    fn modified_untracked_and_deleted_sources_change_observation_at_same_head() {
        let repo = repository();
        let baseline = observe(repo.path()).unwrap();
        assert!(!baseline.dirty);
        fs::write(repo.path().join("source.rs"), "modified").unwrap();
        let modified = observe(repo.path()).unwrap();
        assert!(modified.dirty);
        assert_eq!(baseline.head, modified.head);
        assert_ne!(baseline.source_digest, modified.source_digest);
        fs::write(repo.path().join("new.rs"), "new").unwrap();
        let untracked = observe(repo.path()).unwrap();
        assert!(untracked.dirty);
        assert_eq!(baseline.head, untracked.head);
        assert_ne!(modified.source_digest, untracked.source_digest);
        assert!(untracked
            .source_files
            .iter()
            .any(|file| file.path == "new.rs"));
        fs::remove_file(repo.path().join("source.rs")).unwrap();
        let deleted = observe(repo.path()).unwrap();
        assert!(deleted.dirty);
        assert_eq!(baseline.head, deleted.head);
        assert_ne!(untracked.source_digest, deleted.source_digest);
        assert!(!deleted
            .source_files
            .iter()
            .any(|file| file.path == "source.rs"));
        assert_eq!(baseline.executable_digest, deleted.executable_digest);
    }

    #[test]
    fn ignored_files_do_not_change_inventory_and_order_is_stable() {
        let repo = repository();
        let baseline = observe(repo.path()).unwrap();
        fs::create_dir(repo.path().join("ignored")).unwrap();
        fs::write(repo.path().join("ignored/output"), "artifact").unwrap();
        let current = observe(repo.path()).unwrap();
        assert!(!current.dirty);
        assert_eq!(baseline.source_digest, current.source_digest);
        assert!(current
            .source_files
            .windows(2)
            .all(|files| files[0].path < files[1].path));
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_source_is_rejected() {
        let repo = repository();
        std::os::unix::fs::symlink("source.rs", repo.path().join("link.rs")).unwrap();
        assert!(observe(repo.path()).is_err());
    }
}
