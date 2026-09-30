# Bootstrap installer

For a frontend schema-4/selection-3 user profile, use custom-only release
`1.3.0` with `--profile-file PATH --release 1.3.0`. It installs fixed inactive
integration examples and a schema-4 receipt; the source profile, package files,
runtime files and CI remain user-owned. No package manager runs during install.
The profile may omit provisioning and `.node-version`; if it declares a runtime
file, project verification checks its exact value against the observed tool.
Uninstall uses the same profile path and release even if the source was edited or
removed. The [project-owned selection contract](tool-selection.md#project-owned-frontend-versions-schema-4)
describes migration and verification. The historical release instructions below
continue to apply to their respective schemas.

For schema-3 user profiles, use custom-only release 1.2.0 as documented in
[explicit consumer composition](consumer-schema3.md). The preset and schema-2
instructions below continue to use 1.1.0 and retain their historical semantics.

Task 13 adds the separate Rust `policy-bootstrap` workspace package under
`bootstrap/`. Task 14 has compiled and run the Rust regressions and exercises this
installer through consumer staging. Native and CI acceptance are tracked in the
[Task 14 execution record](../archive/reviews/2026-09-09-task-14-consuming-acceptance.md).
This is not a verified template release.

## Inputs and commands

Use an existing destination directory on Linux/macOS. Every ancestor must be a
real directory without symlinks: on macOS use the physical `/private/tmp/...`
path instead of the `/tmp` alias. Relative targets resolve from the current
directory; parent traversal and the filesystem root are rejected. Components
use portable relative paths; `.` explicitly denotes the target root.

The following examples select the current supported template release explicitly;
they are usage commands, not evidence that Task 13 executed them:

```console
rtk cargo run --locked -p policy-bootstrap -- inspect --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api > /absolute/bootstrap-plan.json
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
`--release` is required. Embedded releases `1.0.0`, `1.0.1`, `1.0.2` and `1.1.0`
are recognized; there is no implicit latest release. Use `1.1.0` for a new
installation unless an explicitly reviewed compatibility requirement selects a
historical release. No release lookup, download, or package restore occurs.

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

The receipt records selection, release, project-profile schema (1 for frozen old
releases, 2 for 1.1.0), sensor-profile schema 2, evidence schema 1, and SHA-256 of each managed file's exact UTF-8 bytes.
Its own bytes are bound by the saved plan, avoiding a circular self-digest.
Receipt claims must match the known template inventory; editing the receipt
cannot claim arbitrary files for replacement or uninstall.

Release resources, including profiles and Task 10 action copies, live under
`bootstrap/templates/1.0.0/`, `bootstrap/templates/1.0.1/` and
`bootstrap/templates/1.0.2/`. Historical bytes remain unchanged for receipt
validation and recovery. Version 1.0.2 retains the 1.0.1 pins (Vite 7.3.6,
Vitest 4.1.11, and Playwright 1.55.1) and adds the required
`shared:dependencies` sensor, so its fullstack profile has 15 sensors and audits
the shared npm graph as well as the frontend graph. Versions 1.0.0 and 1.0.1
retain 14 sensors and do not perform that shared audit. Select `--release 1.1.0`
explicitly after reviewing and updating the consumer manifests/lockfiles.
Cross-version tests cover upgrade, downgrade, repeated installation, uninstall
and preservation of local edits, including frozen receipts from the old runner.

The plan reports `add`, `unchanged`, `update`, `remove`, or `conflict` for each
file. `before_digest`/`after_digest` bind bytes; `previous`/`content` expose old
known template and proposed text for reviewing updates. Unowned local contents
are never printed, only digests. Even an identical unowned file conflicts: no
silent ownership adoption occurs. Unchanged managed files retain timestamps.
Missing managed files can be recreated; edited managed files conflict.

Compatible application manifests are read and preserved. In legacy preset mode, conflicting frameworks
or package managers, competing `.csproj` files, a nonstandard/conditional .NET
target, mismatched installed/lockfile pins, and existing unowned configurations
are explicit conflicts. Detection does not evaluate MSBuild imports or select a
different framework. Keep an existing equivalent toolchain and author/review an
adapted profile when these defaults do not fit; do not delete working settings
merely to silence the installer. There is no automatic merge or takeover flag.

## User profiles and release 1.1.0

Release `1.1.0` introduces schema 2 project profiles with explicit tool selections.
The built-in presets keep their reviewed runtime/package choices and add the
selected dependency graph contract. Releases `1.0.0`–`1.0.2` retain their original
profile and receipt semantics; upgrades and downgrades validate their exact assets.

For an existing project, supply a reviewed schema 2 profile stored inside the
target checkout, outside `.policy`, `.git`, and the transaction directory:

```console
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile-file config/project-profile.json --release 1.1.0 > /absolute/bootstrap-plan.json
rtk cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-plan.json
```

`--profile-file` replaces `--profile` and cannot be combined with `--component`.
Component roots and selected tools come from the source profile. Planning validates
the profile, checks its declared inputs, and records its exact SHA-256 digest in
`request.installation.user_profile` and the input snapshot. A source change after
planning prevents apply. Missing inputs remain prerequisites; bootstrap does not
run tools or claim that a declared combination has passed native verification.

Custom mode installs only `.policy/USAGE.md` and the three fixed CI integration
files under `.policy/ci/`. It leaves the source, native configurations and runtime
files user-owned. It does not copy a custom profile or sensor profile into `.policy`.
Use the source path and its current digest when preparing and adopting a RunRequest.
The complete managed inventory is reconstructed from frozen integration assets,
independently of mutable profile input. Even an identical unowned integration file
conflicts. A forged receipt cannot claim arbitrary source content or destinations.

The `1.1.0` receipt uses schema 2; older releases retain receipt schema 1. Source
changes require a new plan and fresh policy preparation, but do not prevent recovery
or removal of unchanged integration assets. Uninstall matches the receipt's original
profile path and selection even if the source is edited, invalid, or missing:

```console
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile-file config/project-profile.json --release 1.1.0 --uninstall > /absolute/bootstrap-uninstall.json
rtk cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-uninstall.json
```

The source remains user-owned through preset/custom transitions and uninstall.
Managed file edits still block the whole operation. Profile-file inputs are read
through the same descriptor-relative no-follow traversal as other plan inputs.

## Native checks and consumer work

Installation, inspection, transaction, CLI and test logic are Rust. Native JS/TS
files contain declarative tool configuration only; no application implementation,
test cases or check-running scripts are generated. YAML is limited to the existing
Rust CI adapter and a Rust verification invocation.

The releases use the direct profile pins below. Frontend defaults also include
`@typescript-eslint/parser` 8.42.0 and `jsdom` 26.1.0, binding installed package
manifests and package-lock entries to their sensors. These versions have upstream
release records. Actual runs are recorded separately; the table is a selection inventory:
[typescript-eslint 8.42.0](https://github.com/typescript-eslint/typescript-eslint/releases/tag/v8.42.0),
[jsdom 26.1.0](https://github.com/jsdom/jsdom/releases/tag/26.1.0).
The bootstrap profile selects `eslint.config.mjs` explicitly and includes these
dependencies.

| Tool | 1.0.0 | 1.0.1 | 1.0.2 |
| --- | --- | --- | --- |
| Node / npm | 22.18.0 / 10.9.3 | 22.18.0 / 10.9.3 | 22.18.0 / 10.9.3 |
| TypeScript / ESLint | 5.9.2 / 9.34.0 | 5.9.2 / 9.34.0 | 5.9.2 / 9.34.0 |
| Vite / Vitest | 7.1.3 / 3.2.4 | 7.3.6 / 4.1.11 | 7.3.6 / 4.1.11 |
| Playwright | 1.55.0 | 1.55.1 | 1.55.1 |
| typescript-eslint parser / jsdom | 8.42.0 / 26.1.0 | 8.42.0 / 26.1.0 | 8.42.0 / 26.1.0 |
| .NET SDK / VSTest Test SDK | 9.0.304 / 17.14.1 | 9.0.304 / 17.14.1 | 9.0.304 / 17.14.1 |

An installed profile's digest differs from the repository-root historical asset
when its release or component roots differ. Prepare and adopt the actual
`.policy/project-profile.json`; do not reuse a base-asset digest.

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

Uninstall uses the same saved-plan mechanism. Read the
`.policy/bootstrap-manifest.json` receipt and pass exactly its recorded profile,
release, and component mapping; the example matches the 1.1.0 installation above:

```console
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api --uninstall > /absolute/bootstrap-uninstall.json
rtk cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-uninstall.json
```

Only unchanged managed files are removed. Any edited file blocks the entire
operation before writes. Unowned files and application directories remain; no
recursive directory removal occurs. Interrupted uninstall uses the same journal
to restore original managed contents.

See the [execution record](../archive/reviews/2026-09-09-task-13-bootstrap.md) for actual
verification status and remaining release gates.
