use crate::{
    digest,
    fs::{relative, Root},
    invalid, json,
    plan::{self, Manifest},
    templates::{self, MANIFEST, TRANSACTION},
    Action, Installation, Operation, Plan, Result,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

const JOURNAL: &str = ".policy-bootstrap-transaction/journal.json";
const COMMITTED: &str = ".policy-bootstrap-transaction/committed";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    path: String,
    before: Option<Vec<u8>>,
    after: Option<Vec<u8>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema_version: u32,
    target_identity: (u64, u64),
    installation: Installation,
    operation: Operation,
    old_manifest: Option<Vec<u8>>,
    entries: Vec<Entry>,
    directories: Vec<String>,
}

fn staging(index: usize) -> String {
    format!("{TRANSACTION}/stage-{index}")
}
fn restore_path(index: usize) -> String {
    format!("{TRANSACTION}/restore-{index}")
}

fn parents(path: &str) -> Vec<String> {
    let mut current = path;
    let mut result = Vec::new();
    while let Some((parent, _)) = current.rsplit_once('/') {
        result.push(parent.into());
        current = parent;
    }
    result
}

fn journal(root: &Root, plan: &Plan) -> Result<Journal> {
    let mut entries = Vec::new();
    let mut directories = BTreeSet::new();
    for change in plan.files.iter().filter(|c| c.action != Action::Unchanged) {
        let before = root.read(&change.path)?;
        if before.as_deref().map(digest) != change.before_digest {
            return Err(invalid("plan input changed"));
        }
        let after = change.content.as_ref().map(|c| c.as_bytes().to_vec());
        if after.is_some() {
            for directory in parents(&change.path) {
                if !root.exists(&directory)? {
                    directories.insert(directory);
                }
            }
        }
        entries.push(Entry {
            path: change.path.clone(),
            before,
            after,
        });
    }
    // Publish the receipt last, after all managed files are present.
    entries.sort_by_key(|e| (e.path == MANIFEST, e.path.clone()));
    let mut directories: Vec<_> = directories.into_iter().collect();
    directories.sort_by_key(|p| (p.matches('/').count(), p.clone()));
    Ok(Journal {
        schema_version: 1,
        target_identity: root.identity,
        installation: plan.request.installation.clone(),
        operation: plan.request.operation,
        old_manifest: root.read(MANIFEST)?,
        entries,
        directories,
    })
}

/// A journal is local input, not arbitrary permission to overwrite paths.
fn validate(root: &Root, journal: &Journal) -> Result<()> {
    if journal.schema_version != 1
        || journal.target_identity != root.identity
        || journal.entries.len() > 100
    {
        return Err(invalid("invalid recovery journal identity/version/size"));
    }
    let old = match &journal.old_manifest {
        Some(b) => plan::owned(b)?,
        None => BTreeMap::new(),
    };
    let mut new = match journal.operation {
        Operation::Install => templates::render(&journal.installation)?,
        Operation::Uninstall => {
            let receipt: Manifest = serde_json::from_slice(
                journal
                    .old_manifest
                    .as_deref()
                    .ok_or_else(|| invalid("uninstall journal missing receipt"))?,
            )?;
            if receipt.installation != journal.installation {
                return Err(invalid("uninstall journal selection mismatch"));
            }
            BTreeMap::new()
        }
    };
    if journal.operation == Operation::Install {
        new.insert(
            MANIFEST.into(),
            json(&plan::manifest(&journal.installation, &new))?,
        );
    }
    let mut seen = BTreeSet::new();
    let mut allowed_parents = BTreeSet::new();
    for entry in &journal.entries {
        if !relative(&entry.path, false)
            || !seen.insert(&entry.path)
            || (!old.contains_key(&entry.path) && !new.contains_key(&entry.path))
            || entry.after.as_deref() != new.get(&entry.path).map(|s| s.as_bytes())
            || entry
                .before
                .as_ref()
                .is_some_and(|b| old.get(&entry.path).map(|s| s.as_bytes()) != Some(b.as_slice()))
            || entry.before == entry.after
        {
            return Err(invalid(
                "journal contains unauthorized or inconsistent file changes",
            ));
        }
        if entry.after.is_some() {
            allowed_parents.extend(parents(&entry.path));
        }
    }
    let mut seen_dirs = BTreeSet::new();
    for directory in &journal.directories {
        if !allowed_parents.contains(directory) || !seen_dirs.insert(directory) {
            return Err(invalid("journal contains invalid directories"));
        }
    }
    Ok(())
}

fn cleanup(root: &Root) -> Result<()> {
    let names = root
        .names(TRANSACTION)?
        .ok_or_else(|| invalid("transaction directory disappeared"))?;
    for name in &names {
        let numbered = |prefix: &str| {
            name.strip_prefix(prefix)
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        };
        if !matches!(
            name.as_str(),
            "journal.json" | "journal.tmp" | "committed" | "committed.tmp"
        ) && !numbered("stage-")
            && !numbered("restore-")
        {
            return Err(invalid(
                "unknown transaction file; preserve it for manual inspection",
            ));
        }
        // Refuse symlinks/special files before deleting anything in the transaction.
        root.read(&format!("{TRANSACTION}/{name}"))?;
    }
    // Journal/marker survive until all staging data have been cleaned up.
    for name in names
        .iter()
        .filter(|n| n.as_str() != "journal.json" && n.as_str() != "committed")
    {
        root.remove(&format!("{TRANSACTION}/{name}"), false)?;
    }
    if root.exists(JOURNAL)? {
        root.remove(JOURNAL, false)?;
    }
    if root.exists(COMMITTED)? {
        root.remove(COMMITTED, false)?;
    }
    root.remove(TRANSACTION, true)
}

fn rollback(root: &Root, journal: &Journal) -> Result<()> {
    validate(root, journal)?;
    // Preflight the entire rollback. A post-interruption user edit stops recovery
    // before it touches any other target file.
    for entry in &journal.entries {
        let current = root.read(&entry.path)?;
        if current != entry.before && current != entry.after {
            return Err(invalid(format!(
                "recovery conflict at {}; preserve the journal and resolve the edit",
                entry.path
            )));
        }
    }
    for (index, entry) in journal.entries.iter().enumerate().rev() {
        if root.read(&entry.path)? == entry.before {
            continue;
        }
        match &entry.before {
            Some(bytes) => {
                let staged = restore_path(index);
                if root.exists(&staged)? {
                    root.remove(&staged, false)?;
                }
                root.create(&staged, bytes)?;
                root.publish(&staged, &entry.path, entry.after.is_some())?;
            }
            None => root.remove(&entry.path, false)?,
        }
    }
    for directory in journal.directories.iter().rev() {
        if root.exists(directory)? {
            // Keep directories if another actor has added data since interruption.
            if root.names(directory)?.is_some_and(|n| n.is_empty()) {
                root.remove(directory, true)?;
            }
        }
    }
    cleanup(root)
}

fn transact(root: &Root, journal: &Journal) -> Result<()> {
    validate(root, journal)?;
    root.mkdir(TRANSACTION)?; // Exclusive lock for all installer transactions.
    let staged = (|| -> Result<()> {
        for (index, entry) in journal.entries.iter().enumerate() {
            if let Some(bytes) = &entry.after {
                root.create(&staging(index), bytes)?;
            }
        }
        root.create(
            &format!("{TRANSACTION}/journal.tmp"),
            json(journal)?.as_bytes(),
        )?;
        root.publish(&format!("{TRANSACTION}/journal.tmp"), JOURNAL, false)?;
        Ok(())
    })();
    if let Err(error) = staged {
        if let Err(cleanup_error) = cleanup(root) {
            return Err(invalid(format!(
                "staging failed: {error}; cleanup needs recovery: {cleanup_error}"
            )));
        }
        return Err(error);
    }
    let published = (|| -> Result<()> {
        for directory in &journal.directories {
            root.mkdir(directory)?;
        }
        for (index, entry) in journal.entries.iter().enumerate() {
            if root.read(&entry.path)? != entry.before {
                return Err(invalid(format!(
                    "file changed during apply: {}",
                    entry.path
                )));
            }
            if entry.after.is_some() {
                root.publish(&staging(index), &entry.path, entry.before.is_some())?;
            } else {
                root.remove(&entry.path, false)?;
            }
        }
        root.create(&format!("{TRANSACTION}/committed.tmp"), b"committed\n")?;
        root.publish(&format!("{TRANSACTION}/committed.tmp"), COMMITTED, false)?;
        Ok(())
    })();
    finish_publication(root, journal, published)
}

fn finish_publication(root: &Root, journal: &Journal, published: Result<()>) -> Result<()> {
    if let Err(error) = published {
        // Publication can succeed before unlink/fsync reports an error. Once a
        // marker is visible we must not roll back: recovery would interpret it
        // as committed and discard the rollback journal. If marker inspection
        // itself fails, also preserve all state for explicit recovery.
        match root.exists(COMMITTED) {
            Ok(false) => {},
            Ok(true) => return Err(invalid(format!(
                "commit marker published; preserve installation and run recover to finish cleanup: {error}"
            ))),
            Err(inspect_error) => return Err(invalid(format!(
                "commit state uncertain; preserve journal and run recover: {error}; {inspect_error}"
            ))),
        }
        if let Err(rollback_error) = rollback(root, journal) {
            return Err(invalid(format!(
                "apply failed: {error}; recovery required: {rollback_error}"
            )));
        }
        return Err(error);
    }
    cleanup(root).map_err(|e| {
        invalid(format!(
            "installation committed; run recover to finish transaction cleanup: {e}"
        ))
    })
}

pub fn apply(saved: &Plan) -> Result<()> {
    if !saved.conflicts.is_empty() {
        return Err(invalid("plan contains conflicts; no files written"));
    }
    let root = Root::open(&saved.request.target)?;
    root.lock()?;
    if root.identity != saved.target_identity {
        return Err(invalid("installation target changed"));
    }
    let current = plan::plan(saved.request.clone())?;
    if saved != &current {
        return Err(invalid("stale or altered plan; inspect and plan again"));
    }
    if saved.files.iter().all(|f| f.action == Action::Unchanged) {
        return Ok(());
    }
    transact(&root, &journal(&root, saved)?)
}

/// Roll back an interrupted publication, or finish cleaning a committed transaction.
pub fn recover(target: &Path) -> Result<()> {
    let root = Root::open(target)?;
    root.lock()?;
    if !root.exists(TRANSACTION)? {
        return Err(invalid("no interrupted transaction"));
    }
    let Some(bytes) = root.read(JOURNAL)? else {
        // Without a published journal no target mutation may have begun (or cleanup
        // already finished). Only recognized private staging files can be removed.
        return cleanup(&root);
    };
    let journal: Journal = serde_json::from_slice(&bytes)?;
    validate(&root, &journal)?;
    if let Some(marker) = root.read(COMMITTED)? {
        if marker != b"committed\n" {
            return Err(invalid("invalid commit marker"));
        }
        return cleanup(&root);
    }
    rollback(&root, &journal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Operation, Request};

    fn fixture() -> (tempfile::TempDir, Root, Plan) {
        let temp = tempfile::tempdir().unwrap();
        let root = Root::open(&temp.path().canonicalize().unwrap()).unwrap();
        let plan = plan::plan(Request {
            target: root.path.clone(),
            installation: Installation {
                profile: "dotnet".into(),
                release: "1.0.0".into(),
                components: BTreeMap::new(),
            },
            operation: Operation::Install,
        })
        .unwrap();
        (temp, root, plan)
    }

    // Catches continuing after a failed publish and losing earlier writes.
    #[test]
    fn publication_failure_restores_all_original_files() {
        let (_temp, root, plan) = fixture();
        let journal = journal(&root, &plan).unwrap();
        // A conflicting directory is introduced after preflight. The first file is
        // published before the later directory collision makes publication fail.
        let blocked = journal.entries[1].path.clone();
        for parent in parents(&blocked).iter().rev() {
            if !root.exists(parent).unwrap() {
                root.mkdir(parent).unwrap();
            }
        }
        root.mkdir(&blocked).unwrap();
        // Directory creations from the original snapshot must not mask publication.
        let mut journal = journal;
        journal.directories.retain(|d| !root.exists(d).unwrap());
        assert!(transact(&root, &journal).is_err());
        // Recovery deliberately preserves the conflicting external directory and
        // retains its journal, rather than hiding this unresolved conflict.
        assert!(root.exists(TRANSACTION).unwrap());
        root.remove(&blocked, true).unwrap();
        recover(&root.path).unwrap();
        assert!(!root.exists(&journal.entries[0].path).unwrap());
        assert!(!root.exists(MANIFEST).unwrap());
    }

    fn interrupt(root: &Root, journal: &Journal) {
        root.mkdir(TRANSACTION).unwrap();
        root.create(JOURNAL, json(journal).unwrap().as_bytes())
            .unwrap();
        for directory in &journal.directories {
            root.mkdir(directory).unwrap();
        }
        for (index, entry) in journal.entries.iter().enumerate().take(2) {
            root.create(&staging(index), entry.after.as_ref().unwrap())
                .unwrap();
            root.publish(&staging(index), &entry.path, false).unwrap();
        }
    }

    // Catches forgetting durable rollback state after partial publication.
    #[test]
    fn interrupted_apply_is_recoverable_and_blocks_new_plans() {
        let (_temp, root, plan) = fixture();
        let journal = journal(&root, &plan).unwrap();
        interrupt(&root, &journal);
        assert!(crate::plan(plan.request).is_err());
        recover(&root.path).unwrap();
        assert_eq!(root.names(".").unwrap().unwrap(), Vec::<String>::new());
    }

    // Catches recovery overwriting a user edit or restoring arbitrary journal paths.
    #[test]
    fn recovery_edits_and_forged_paths_are_rejected_without_partial_rollback() {
        let (_temp, root, plan) = fixture();
        let mut journal = journal(&root, &plan).unwrap();
        interrupt(&root, &journal);
        let first = root.read(&journal.entries[0].path).unwrap();
        std::fs::write(root.path.join(&journal.entries[1].path), "user edit").unwrap();
        assert!(recover(&root.path).is_err());
        assert_eq!(root.read(&journal.entries[0].path).unwrap(), first);
        journal.entries[0].path = "../escape".into();
        assert!(validate(&root, &journal).is_err());
    }

    // Catches an interrupted marker write being mistaken for a commit or blocking rollback.
    #[test]
    fn partial_temporary_commit_marker_does_not_block_recovery() {
        let (_temp, root, plan) = fixture();
        let journal = journal(&root, &plan).unwrap();
        interrupt(&root, &journal);
        root.create(&format!("{TRANSACTION}/committed.tmp"), b"comm")
            .unwrap();
        recover(&root.path).unwrap();
        assert!(root.names(".").unwrap().unwrap().is_empty());
    }

    #[test]
    fn recovery_cannot_run_while_another_installer_holds_the_target_lock() {
        let (_temp, root, plan) = fixture();
        let journal = journal(&root, &plan).unwrap();
        interrupt(&root, &journal);
        root.lock().unwrap();
        assert!(recover(&root.path).is_err());
        assert!(root.exists(JOURNAL).unwrap());
    }

    // Catches interrupted removal losing original content and the ownership receipt.
    #[test]
    fn interrupted_uninstall_restores_original_managed_files() {
        let (_temp, root, mut saved) = fixture();
        apply(&saved).unwrap();
        let original = root.read(MANIFEST).unwrap();
        saved.request.operation = Operation::Uninstall;
        let uninstall = plan::plan(saved.request).unwrap();
        let journal = journal(&root, &uninstall).unwrap();
        root.mkdir(TRANSACTION).unwrap();
        root.create(JOURNAL, json(&journal).unwrap().as_bytes())
            .unwrap();
        for entry in &journal.entries {
            root.remove(&entry.path, false).unwrap();
        }
        recover(&root.path).unwrap();
        assert_eq!(root.read(MANIFEST).unwrap(), original);
        assert!(root.exists("global.json").unwrap());
    }

    // Catches rolling back after link(COMMITTED) succeeded but its unlink/fsync failed.
    #[test]
    fn error_after_commit_marker_publication_never_starts_rollback() {
        let (_temp, root, saved) = fixture();
        let journal = journal(&root, &saved).unwrap();
        root.mkdir(TRANSACTION).unwrap();
        root.create(JOURNAL, json(&journal).unwrap().as_bytes())
            .unwrap();
        for directory in &journal.directories {
            root.mkdir(directory).unwrap();
        }
        for (index, entry) in journal.entries.iter().enumerate() {
            root.create(&staging(index), entry.after.as_ref().unwrap())
                .unwrap();
            root.publish(&staging(index), &entry.path, false).unwrap();
        }
        root.create(COMMITTED, b"committed\n").unwrap();
        let receipt = root.read(MANIFEST).unwrap();
        let sync_error = Err(std::io::Error::other("commit directory sync failed").into());
        assert!(finish_publication(&root, &journal, sync_error).is_err());
        assert_eq!(root.read(MANIFEST).unwrap(), receipt);
        assert!(root.exists(JOURNAL).unwrap());
        recover(&root.path).unwrap();
        assert_eq!(root.read(MANIFEST).unwrap(), receipt);
        assert!(root.exists("global.json").unwrap());
        assert!(!root.exists(TRANSACTION).unwrap());
    }
}
