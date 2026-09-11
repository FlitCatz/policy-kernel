# Task 13 Bootstrap Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan in this session. The user approved proceeding with the existing Task 13 design.

**Goal:** Add a separate Rust bootstrap installer with reviewable inspect/plan/apply, managed-file tracking, conflict protection and recovery.

**Architecture:** Embed a fixed local template release and reuse Task 12 profiles. Recompute a saved plan before applying it, then stage files and persist a rollback journal. Use descriptor-relative no-follow filesystem operations on Unix; unsupported platforms fail closed.

**Tech Stack:** Rust, serde/serde_json, clap, sha2, rustix, existing project-verification.

**Spec:** `docs/specs/2026-09-07-project-verification-bootstrap-design.md`, section 4; Task 13 in `docs/plans/2026-09-04-policy-kernel-implementation.md`.

## Global constraints

- Runtime logic, CLI and tests are Rust. No Python or generated application/test business logic.
- The existing user prohibition on test execution remains active. Write regression tests first, but record execution and compilation as `not_run`; run only formatting and static inspection.
- Keep the current feature checkout, consistent with Task 12's recorded workflow. Preserve unrelated `verification/` output.
- No credentials, provider login, reviewer selection, remote protection, policy adoption or new policy-cli authority.
- Use the existing fixed compatibility candidates. These are not a certified release; Task 12 and Task 14 runtime acceptance remain open.

## 13A — deterministic planning and templates

Files: `bootstrap/Cargo.toml`, `bootstrap/src/{lib,plan,templates}.rs`, `bootstrap/templates/1.0.0/`, `bootstrap/tests/installer.rs`, `tests/fixtures/bootstrap/`, workspace manifest and lockfile.

- [x] Write regression cases for empty React/.NET/fullstack destinations, existing-project preservation, ambiguous detection, unsupported release and component escapes. Assert rendered component roots and absence of application manifests.
- [x] Implement `plan(Request) -> Result<Plan>` and `inspect(Request) -> Result<Plan>` with deterministic JSON, input digests, directory observations, per-file additions/updates/removals/conflicts and missing consumer inputs.
- [x] Embed release 1.0.0; render project and unbound sensor profiles, declarative check settings, usage and CI templates. Reuse the existing native tool pins without downloading tools. Existing unowned files conflict even when identical; never silently adopt them.
- [x] Track release, contract compatibility, component roots and exact rendered file digests in `.policy/bootstrap-manifest.json`. Updates include previous/proposed text. Uninstall plans remove only files whose content still matches the known template.

## 13B — bounded transaction and CLI

Files: `bootstrap/src/{fs,transaction,main}.rs`, `bootstrap/tests/{installer,cli}.rs`.

- [x] Write stale-plan, edited-file, symlink, parent-file, tampered-plan, rollback, interrupted recovery and uninstall regression cases before implementing the write path.
- [x] Implement descriptor-relative bounded filesystem reads/writes. Reject lexical escapes, symlinks and special files; never follow a destination ancestor symlink. Use exclusive file creation and a transaction lock.
- [x] Implement `apply(&Plan) -> Result<()>`: reopen the same target identity, recompute the complete plan and compare it, reject conflicts, stage all bytes, persist original bytes and created-directory inventory, then publish. Restore original bytes on failure. An interrupted transaction blocks new application until explicit `recover(target)` restores it; refuse recovery if affected files were subsequently edited.
- [x] Expose `inspect` and `plan` with explicit target/profile/release/component inputs; `apply --plan FILE`; `recover --target DIR`; and `--uninstall` on planning. Print JSON to stdout; no target writes during inspect/plan.

## 13C — static review and handoff

Files: `docs/integrations/bootstrap.md`, `docs/reviews/2026-09-09-task-13-bootstrap.md`.

- [x] Document exact CLI invocations, file ownership, conflict repair, saved-plan freshness, recovery, uninstall, CI activation and remaining consuming-project inputs.
- [x] Resolve the lockfile offline without compiling or executing tests.
- [x] Run targeted rustfmt and `git diff --check`; review the template inventory and transaction paths. Request a bounded read-only code review while completing documentation.
- [x] Record actual checks and limitations. Do not claim runtime correctness, release readiness, Task 12 verification or Task 14 acceptance.

## Deferred execution commands

Once test execution is permitted: `rtk cargo test --locked -p policy-bootstrap`, followed by relevant project-profile/schema regressions and the existing final-head workspace CI. These commands are documentation, not authorization to run them in this task.
