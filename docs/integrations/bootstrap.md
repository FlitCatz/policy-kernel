# Bootstrap installer

Task 13 adds the separate Rust `policy-bootstrap` workspace package under
`bootstrap/`. Implementation and regression tests are present; compilation,
tests and consuming-project execution are **not_run** under the existing user
instruction. This is not a verified template release. Task 12 verification and
Task 14 native positive/negative acceptance remain open.

## Inputs and commands

Use an existing destination directory on Linux/macOS. Every ancestor must be a
real directory without symlinks: on macOS use the physical `/private/tmp/...`
path instead of the `/tmp` alias. Relative targets resolve from the current
directory; parent traversal and the filesystem root are rejected. Components
use portable relative paths; `.` explicitly denotes the target root.

The following are future usage commands, not commands executed during Task 13:

```console
rtk cargo run --locked -p policy-bootstrap -- inspect --target /absolute/project --profile fullstack --release 1.0.0 --component frontend=apps/web --component backend=apps/api
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.0.0 --component frontend=apps/web --component backend=apps/api > /absolute/bootstrap-plan.json
rtk cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-plan.json
```

`inspect` and `plan` share one read-only snapshot operation and print JSON. Save
the plan **outside** the destination: new directory entries invalidate a saved
snapshot. No target files/directories are created during planning. Components
may be absent; the target directory itself must already exist.

Profiles are `react-typescript`, `dotnet`, and `fullstack`. Standalone defaults
use `.`; fullstack defaults are `frontend`, `backend`, and shared `.`. Repeat
`--component ID=PATH` to override roots. Unknown/duplicate components, overlapping
application roots, reserved paths and template file/directory collisions are
rejected. Fullstack shared may be the target root or a disjoint component.
`--release` is required; only the embedded `1.0.0` candidate is recognized.
No latest-release lookup, download or package restore occurs.

Exit codes: `0` means a conflict-free plan or successful apply/recovery; `2` means
a plan has conflicts; `64` means invalid input, unsafe filesystem access, stale
plan or transaction failure. None is a policy authorization decision.

## Installed files and ownership

| Scope | Managed assets |
| --- | --- |
| Target | `.editorconfig`, `.policy/project-profile.json`, `.policy/sensor-profile.json`, `.policy/USAGE.md`, `.policy/ci/` templates |
| Frontend | `.node-version`, `.npmrc`, `tsconfig.json`, `eslint.config.mjs`, `vite.config.ts`, `vitest.config.ts` |
| Backend | `global.json`, `Directory.Build.props`, `NuGet.Config` |
| Shared fullstack | `.node-version`, `.npmrc`, `vitest.contract.config.ts`, `playwright.config.ts` |
| Receipt | `.policy/bootstrap-manifest.json` |

The receipt records selection, release, project-profile schema 1, sensor-profile
schema 2, evidence schema 1, and SHA-256 of each managed file's exact UTF-8 bytes.
Its own bytes are bound by the saved plan, avoiding a circular self-digest.
Receipt claims must match the known template inventory; editing the receipt
cannot claim arbitrary files for replacement or uninstall.

Release resources, including profiles and Task 10 action copies, live under
`bootstrap/templates/1.0.0/`. After release, retain these bytes and introduce a
new directory for changes. Recovery needs the old inventory. Version 1.0.0 has
no prior released version to upgrade from; managed updates support changed
component selection with before/after text. Future releases must retain old
renderers and add cross-version regression cases.

The plan reports `add`, `unchanged`, `update`, `remove`, or `conflict` for each
file. `before_digest`/`after_digest` bind bytes; `previous`/`content` expose old
known template and proposed text for reviewing updates. Unowned local contents
are never printed, only digests. Even an identical unowned file conflicts: no
silent ownership adoption occurs. Unchanged managed files retain timestamps.
Missing managed files can be recreated; edited managed files conflict.

Compatible application manifests are read and preserved. Conflicting frameworks
or package managers, competing `.csproj` files, a nonstandard/conditional .NET
target, mismatched installed/lockfile pins, and existing unowned configurations
are explicit conflicts. Detection does not evaluate MSBuild imports or select a
different framework. Keep an existing equivalent toolchain and author/review an
adapted profile when these defaults do not fit; do not delete working settings
merely to silence the installer. There is no automatic merge or takeover flag.

## Native checks and consumer work

Installation, inspection, transaction, CLI and test logic are Rust. Native JS/TS
files contain declarative tool configuration only; no application implementation,
test cases or check-running scripts are generated. YAML is limited to the existing
Rust CI adapter and a Rust verification invocation.

The release preserves Task 12's native candidate pins. Frontend defaults add
`@typescript-eslint/parser` 8.42.0 and `jsdom` 26.1.0, binding installed package
manifests and package-lock entries to their sensors. These versions have upstream
release records, but the combined matrix has not been executed here:
[typescript-eslint 8.42.0](https://github.com/typescript-eslint/typescript-eslint/releases/tag/v8.42.0),
[jsdom 26.1.0](https://github.com/jsdom/jsdom/releases/tag/26.1.0).
The bootstrap profile selects `eslint.config.mjs` explicitly and includes these
dependencies. Its digest differs from Task 12's base asset: prepare/adopt the
actual installed profile instead of reusing a base digest.

`prerequisites` lists absent native input files not supplied by the template:
package/application manifests, lockfiles, installed dependencies and real .NET
test projects. Frontend tests use jsdom; contract tests use Node; Playwright
writes `.policy-reports/e2e.json`. The consumer authors actual assertions and
provisions real servers. No example tests can be mistaken for adequacy evidence.

Bootstrap does not infer acceptance mappings, create a PolicySnapshot, adopt the
unbound SensorProfile or invent a head-bound RunRequest. Follow
[project verification](project-verification.md) for those inputs. Missing reports,
zero tests and missing tools remain non-success when checks execute. Tool absence
does not block configuration-only installation or make it ready.

CI assets are installed under `.policy/ci/`, not activated in `.github/workflows`.
Integrate the steps into the existing trusted job after pinned provisioning and
exact-head checkout. The copied Task 10 action requires already-built `policy-ci`
and runner-controlled observations; its README is included. Keep existing native
CI until Task 14 and normal promotion requirements are met. No credentials,
provider login, reviewers, branch protection or remote operations are generated.

## Apply, recovery and uninstall

Apply takes an advisory target-directory lock, recomputes the entire plan, and
requires equality with the saved JSON, including target device/inode, input
digests and directory inventories. Conflicts/stale snapshots stop before writes.
Additions use no-clobber publication; updates replace validated managed content.
Descriptor-relative no-follow traversal rejects symlinks and special files,
including dangling symlinks. Non-Unix CLI execution fails closed.

`.policy-bootstrap-transaction/` holds staged bytes and a durable journal of
original contents and directories to create. The receipt is published last.
Each publication is atomic; the complete set is not one atomic filesystem
operation. A fully written commit marker is published atomically before cleanup.
Publication failure attempts rollback; rollback failure preserves the journal.

```console
rtk cargo run --locked -p policy-bootstrap -- recover --target /absolute/project
```

Recovery restores pre-apply content for an uncommitted transaction; after commit
it finishes cleanup. Before rollback it checks every affected file. Edits after
interruption cause a conflict while preserving the journal and other files.
Resolve the competing edit against original/proposed bytes, then retry. Never
delete the journal to bypass this check. Unknown staging files stop cleanup.

Use an exclusively controlled checkout during apply/recovery. The advisory lock
serializes this installer, not arbitrary editors or other processes. Concurrent
hostile directory relocation, network-filesystem semantics and hardware failures
are outside the guarantee. Rollback restores content, not timestamps, inode
identity, ownership or extended attributes. New files/directories use owner-only
access.

Uninstall uses the same saved-plan mechanism and requires the recorded selection:

```console
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.0.0 --component frontend=apps/web --component backend=apps/api --uninstall > /absolute/bootstrap-uninstall.json
rtk cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-uninstall.json
```

Only unchanged managed files are removed. Any edited file blocks the entire
operation before writes. Unowned files and application directories remain; no
recursive directory removal occurs. Interrupted uninstall uses the same journal
to restore original managed contents.

See the [execution record](../reviews/2026-09-09-task-13-bootstrap.md) for actual
verification status and remaining release gates.
