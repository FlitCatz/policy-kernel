# Task 13 execution record

Date: 2026-09-09. Base: `8c91818`; checkout: `feat/task-12-project-profiles`.
Status: implementation present; runtime verification `not_run`.

The user requested Task 13 and explicitly approved proceeding. Work reused the
existing design and feature checkout. Untracked `verification/` output belonged
to the starting workspace and was left untouched. No remote writes, branch
changes, commits, merges or external installations were performed. A read-only
reviewer agent inspected the implementation.

## Implementation

- Added separate Rust `policy-bootstrap` with inspect/plan/apply/recover.
  Uninstall is a planning operation using the same transaction. Kernel
  authorization and `policy-cli` are unchanged.
- Added release-directory copies of three profiles and the Task 10 action,
  native check settings, CI templates and usage. Frontend profiles bind the
  TypeScript ESLint parser and jsdom; Task 12 base assets remain unchanged.
- Added strict plan/receipt JSON, component selection, conservative manifest
  detection, input/package pins, target identity and directory snapshots.
  Unowned/edited files are never adopted or overwritten.
- Added descriptor-relative no-follow access, target locking, durable original
  content journal, receipt-last publication, rollback and interrupted recovery.
  Commit markers are staged and atomically published. Recovery preflights edits.
- Added Rust regressions for empty/existing projects, ambiguity, stale/tampered
  plans, custom roots, symlinks/escapes/aliases, config bindings, repeat no-op,
  missing-file repair, uninstall, interrupted install/uninstall, partial marker
  writes and installer locking. Fixtures are manifest data, not runnable apps.

## Review

The initial read-only review found three Important issues and no Critical issue:

1. Partial writes to the commit marker could block recovery. Switched to atomic
   publication of a synced temporary marker and added a recovery regression.
2. Release resources referenced mutable profile/action paths. Copied them into
   the versioned directory so future upstream edits cannot change this inventory.
3. Native lint/build/test defaults were missing. Added declarative
   ESLint/Vite/Vitest/Playwright templates and profile bindings. Real business
   tests and servers remain consumer-owned.

This is static same-provider review, not independent-provider approval or
runtime correctness evidence.

The focused follow-up confirmed those three changes and found a further commit
boundary: marker publication can succeed before unlink/fsync returns an error.
Starting rollback in that state would conflict with committed recovery semantics.
The error path now preserves a published/uncertain marker and journal instead of
rolling back; added an error-after-marker-publication regression. No runtime
validation is claimed for this correction.

## Verification

- Targeted rustfmt ran successfully during implementation.
- Tracked `git diff --check` had no diagnostics. All 33 new files were also
  checked using no-index whitespace inspection with no diagnostics (exit 1
  denotes differing files in no-index mode); a trailing-whitespace scan had no
  matches. These checks do not execute the implementation.
- Offline Cargo metadata recognized the workspace package.
- Offline lockfile resolution ran. Incidental getrandom/r-efi changes were
  removed with precise offline resolution; only the new workspace package is
  added, retaining all registry dependency versions.
- Tests, compilation, installer execution, native execution and schema validation
  executables: `not_run`, honoring the existing user prohibition.
- Static inspection and formatting cannot establish type correctness,
  transaction behavior, producer compatibility or passing acceptance tests.

When execution is permitted: `rtk cargo test --locked -p policy-bootstrap`,
affected profile/schema regressions and final-head workspace CI. Exercise real
failure/recovery semantics on Linux/macOS. Task 12 runtime verification and Task
14 positive/negative consumer evidence remain release prerequisites.

Only bundled release 1.0.0 is implemented. Future releases must retain old
templates/rendering semantics and add cross-version tests. Recovery restores
content, not metadata/ACLs; use an exclusive checkout. CI assets remain templates
and installation success never produces authoritative ready evidence.
