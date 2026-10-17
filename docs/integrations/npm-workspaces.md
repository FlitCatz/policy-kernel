# P1 npm workspace shared lock

An npm workspace uses one component-root manifest, adjacent lock and audit.
[Current status](../status.md) records verification limits; this contract is not
an adoption or promotion record.

The optional graph `workspace` object contains `member_manifests`
(explicit component-relative `tools/openapi/package.json` paths) and
`minimum_pk_revision` (the consumer's declared 40-character lowercase source
revision containing this extension). That revision is compatibility metadata,
not authenticated provenance or an ancestry check. A consumer must record
a reviewed source revision containing this extension. Old
readers reject the unknown graph field. Existing profiles serialize unchanged.

For a component whose `root` is `frontend`, the graph uses `manifest:
"package.json"`, `lockfile: "package-lock.json"`, and a `workspace` object like:

```json
{
  "member_manifests": ["tools/openapi/package.json"],
  "minimum_pk_revision": "8ce6963ff0a2cdd5eb7239269f6e473372de7011"
}
```

`8ce6963ff0a2cdd5eb7239269f6e473372de7011` is the implementation revision rebased
onto the fixture refresh;
adopters should select a reviewed revision containing it. This object is a graph
fragment, not a complete profile. The audit sensor's explicit `inputs` must include
the root manifest, lock and every member manifest. No member lock is created.

Only profile schema 4 / selection schema 3 can opt in. One npm graph owns the
component-root `package.json` and adjacent `package-lock.json`; the component
can be rooted at `frontend`. Members have no independent lock or audit. Each
member manifest is added to every sensor's configuration binding. The graph,
root `workspaces` array, filesystem manifest inventory, lock member records,
lock links and installed workspace links must agree. Globs and unsafe paths,
undeclared packages, symlink members, external/file dependencies, nested locks,
root/member bundles and project `.npmrc` files fail closed. Registry lock URLs
are limited to `https://registry.npmjs.org/` in this initial contract.

Workspace audit arguments extend the existing root audit with `--workspaces`,
`--include-workspace-root`, `--userconfig=.policy-npm-userconfig` and
`--globalconfig=.policy-npm-globalconfig`. Both reserved config paths must remain
absent; npm rejects sharing one `/dev/null` path between the two sources.
The exact argument contract and environment allowlist
prevent omission and workspace-exclusion settings. Each installed package
selection must point to its own location in the root lock, preventing a member
TS5 selection from being rebound to the root TS6 manifest. Existing standalone
npm graphs and NuGet adjacent-lock rules are retained.

Each locked dependency must resolve through npm's ancestor package locations
within the audited inventory; exact numeric declarations must match the
resolved version. Missing optional peers are allowed. npm remains responsible
for general semver-range resolution during installation. Selected installed
package manifests must match their own locked versions; this is not a hash
attestation of every installed package file. Inventory traversal is bounded to
64 directory levels and 100,000 entries and excludes `.git` and `node_modules`.

All component configurations are rechecked before and after collection. A
member addition/removal or manifest/lock mutation cannot reuse the prepared
configuration. Source inventory completeness and actual Git identity retain
the caller responsibilities described in [project verification](project-verification.md).

`crates/project-verification/tests/npm_workspace.rs` covers twenty
local regression groups. The opt-in native test is:

```console
cargo test --locked --offline -p policy-acceptance --test native_npm_workspace -- --ignored --nocapture
```

It requires already installed Node 26.12.0 and npm 11.20.0. With explicit native
fixture approval, it installs TypeScript 6.0.3/5.9.3 and lodash 4.17.20 into a
temporary directory using `npm install --ignore-scripts --no-audit --no-fund`
and the graph's inclusion/configuration flags. Registry access is required for
installation and real npm audits. Cache and logs also remain inside that
temporary fixture; no global, repository or host-toolchain installation occurs.
Cargo itself remains locked and offline. The test clears inherited npm settings
and checks the exact installed tool versions before installation.

Set `POLICY_NPM_WORKSPACE_OUTPUT` to a fresh output directory to retain native
audit JSON, npm-generated locks and a summary. The fixture proves actual Node
resolution at the root/member, clean baseline/recovery audits, root-only and
member-only vulnerability detection, and rejection of an installed TS5→TS6
manifest swap. Each phase executes one root audit, never one audit per member.
The unit fixtures are synthetic; they are not substituted for these native
observations. See the [local execution record](../archive/reviews/2026-10-16-npm-workspace-local.md).

Successful direct evidence remains LocalUntrusted and can be described only as
“local scope satisfied”; it cannot authorize promotion or a protected merge.

## Reviewed root overrides

The optional `workspace.overrides` profile field must exactly match the root
manifest's `overrides` object. It is omitted when empty, preserving existing
profile serialization. A changed reviewed set changes the canonical profile
digest as well as the manifest input binding; updating only the manifest fails.
An older reader rejects the new field. Consumers must raise
`minimum_pk_revision` to a reviewed revision containing this extension; the
earlier P1 revision above does not contain override support.

The supported forms are global exact versions and a single package scope with
exact child versions. For TJERP's frontend root, put this same object in both
`package.json.overrides` and the graph's `workspace.overrides`:

```json
{
  "svelte-toolbelt": "0.10.5",
  "fdir": { "picomatch": "4.0.7" },
  "@tailwindcss/cli": { "@parcel/watcher": "2.6.0" }
}
```

There may be at most 32 target rules, each with a distinct target name. Names
are lowercase ASCII registry names, at most 214 bytes, with each name/scope
starting with a letter or digit; npm's reserved `node_modules` and `favicon.ico`
names are rejected. Only stable numeric `major.minor.patch`
versions are accepted. Ranges, prereleases, build metadata, version-qualified
selectors, `.` self selectors, `$ref`, aliases, protocols, paths, URLs,
tarballs, wildcards, empty scopes and deeper nesting are rejected. The root and
workspace members cannot be override targets or scopes. Member manifests must
not declare overrides, even if the root also declares them. A global override
of a direct root dependency must exactly equal its declared spec, following
npm's direct-dependency restriction.

All selected package names are protected, including selections using `record`
acceptance. The frozen bootstrap contract names `vite`, `typescript`, `eslint`,
`vitest`, `@playwright/test`, `@typescript-eslint/parser` and `jsdom` are also
forbidden targets regardless of their presence in the consumer selection. This
deliberately rejects both upgrades and downgrades of those packages. Scoped
overrides of their transitive dependencies remain possible. Frozen bootstrap
assets and profiles are unchanged.

Validation follows logical dependency edges from the root and all members,
using ancestor resolution and workspace links. A scoped rule remains active
under its parent, including hoisted and transitive descendants; physical
directory nesting alone does not establish scope. Every affected edge must
resolve to the exact override version, and every rule must match an edge.
Global rules additionally check every locked copy of the target. Unaffected
exact dependency requirements retain their original checks. npm installation
must use separate package instances if a target declaration would be shared
both inside and outside a scoped override; ambiguous shared declarations fail.
npm still owns general semver compatibility; this validates lock inventory rather
than attesting all installed bytes or proving installation reproducibility.

The native override proof uses the same pinned toolchain and temporary-only
network boundary as P1:

```console
cargo test --locked --offline -p policy-acceptance --test native_npm_workspace native_npm_overrides_audit_resolved_versions -- --ignored --nocapture
```

It installs a member dependency on `mkdirp` 0.5.0. A global `minimist` 0.0.8
override produces a vulnerable resolved lock and root audit exit 1; a scoped
`mkdirp` → `minimist` 1.2.8 override produces root audit exit 0 with zero
findings. Each phase verifies actual Node resolution, every locked target copy
and successful preparation against the reviewed profile, then runs one root
audit. Set `POLICY_NPM_OVERRIDES_OUTPUT` to a fresh **absolute** output directory
to retain manifests, locks, audit JSON and summary. Audit results depend on
the live advisory service; a network/service failure is not a passing audit.
See the [override execution record](../archive/reviews/2026-10-17-npm-overrides-local.md).

## Third-party bundled dependencies

Only third-party lock entries may declare `bundleDependencies`, as a unique
array of lowercase registry package names. Root and workspace-member manifests
still reject both `bundleDependencies` and `bundledDependencies`, even empty
arrays. The lock alias `bundledDependencies` and boolean bundle declarations
are unsupported. Consumers must raise `minimum_pk_revision` to a reviewed
revision containing this extension before relying on it.

The parent must have a nonempty version, a canonical single sha512 SRI value
(64 bytes encoded in padded base64), and a `resolved` HTTPS npm registry tarball
under `https://registry.npmjs.org/<parent-name>/-/<filename>.tgz`. The filename
contains only ASCII letters, digits, dots, underscores and hyphens. Other
registries, git/file sources, query strings, fragments, workspace links and
parents themselves marked `inBundle` fail closed. This binds the inventory to
the lock's tarball identity; it does not independently download or hash it.

Every accepted name must appear in that parent's `bundleDependencies`; it may
lack a separate lock entry. A lock entry at
`<parent-path>/node_modules/<bundled-name>` must instead have `inBundle: true`,
a nonempty version, no `link`, `resolved` or `integrity` fields, and a matching
name if present. Exact numeric declarations on the parent must match that
version; npm remains responsible for general semver ranges. Conflicting entries
fail even if a hoisted package could resolve the name. Every explicit `inBundle`
entry needs such a parent declaration; undeclared or nested indirect bundle
entries are unsupported. Other dependency-presence checks remain in force,
including dependencies declared by explicit bundled entries.

The audit sensor's `ConfigurationBinding.bundled_dependencies` is a deterministic
lock-derived array. Each record contains `name`, `version` (null without an
explicit entry), `parent_path`, `parent_name`, `parent_version`,
`parent_integrity`, and `coverage` with the exact value
**`bundled, not individually audited`**. No version is inferred from a range.
The array is included in prepared output and the collected sensor configuration,
and in its configuration digest. It is omitted when empty, preserving existing
configuration serialization. A successful root audit does not establish
individual audit coverage for these records.

Inventory is independent of host installation: optional platform-specific
parents absent from `node_modules` remain represented with all their bundle
names and parent identities. Absence does not remove or turn them into audited
dependencies. Selected installed-package checks are unchanged.

The positive Rust fixture mirrors oxide-wasm32-wasi 4.3.3 with six missing child
entries; negative fixtures cover unpinned/non-registry parents, undeclared names,
conflicts, malformed declarations and root/member bundles. The native proof uses
the same temporary-only network boundary and pinned toolchain as P1:

```console
cargo test --locked --offline -p policy-acceptance --test native_npm_workspace native_npm_bundled_platform_inventory -- --ignored --nocapture
```

It installs `@tailwindcss/oxide` 4.3.3, checks npm's absent wasm32 parent and
six missing bundled child entries, prepares the configuration and performs one
root audit. Set `POLICY_NPM_BUNDLES_OUTPUT` to a fresh absolute directory to
retain the lock, prepared inventory, audit JSON and summary. The
[local execution record](../archive/reviews/2026-10-17-npm-bundles-local.md)
establishes fixture observations only; TJERP P8 must adopt the merged reviewed
revision and re-verify its own frontend `prepare`.
