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
bundles, overrides and project `.npmrc` files fail closed. Registry lock URLs
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

`crates/project-verification/tests/npm_workspace.rs` covers eleven
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
