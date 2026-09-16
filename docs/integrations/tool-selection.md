# Consumer tool selection

Profile schema 2 separates an asset's stable numeric `version` from the reader
contract. Bootstrap release 1.1.0 supplies three reviewed presets; a user profile
may use a different release identifier without extending a parser allowlist.
Schema 1 retains releases 1.0.0–1.0.2 and their exact probe semantics. Old readers
reject schema 2. Migration is explicit and changes the profile digest, so it
requires fresh preparation, policy adoption and evidence.

```console
rtk cargo run --locked -p project-verification -- migrate-profile bootstrap/templates/1.0.2/profiles/fullstack/profile.json
rtk cargo run --locked -p project-verification -- validate-profile examples/react-jest/project-profile.json
rtk cargo run --locked -p project-verification -- ci-inputs examples/react-jest/project-profile.json
rtk cargo run --locked -p project-verification -- schema
```

`migrate-profile` prints schema 2 JSON for a valid 1.0.2 input. It does not edit
the source or grant adoption. `ci-inputs FILE [--github-output FILE]` emits the
exact provisioning choices and profile digest, optionally appending them in
GitHub's output format. Conflicting runtime choices in one job fail; separate
those consumers into jobs. These commands return 0 on success and 64 on error.

## Request, provision, observation

The `selection` object has its own `schema_version: 1`. `tools` maps identifiers
to an executable, explicit version-probe arguments, required capabilities and
a `version` rule. `bindings` binds every sensor ID to a tool. Each sensor retains
its complete execution argv; conflicting executable/probe choices are rejected.
Its legacy `version_stdout` must be empty in schema 2.

The version rule separates `requirement`, `normalization` and `provision`:

```json
{
  "requirement": ">=22.18.0, <23.0.0",
  "normalization": "node",
  "provision": "22.19.0"
}
```

Requirements are comma-separated AND comparisons using `=`, `>=`, `>`, `<=`, or
`<`. Each bound and observation has exactly three unsigned numeric components.
Leading zeroes, prereleases, overflow, wildcards, caret/tilde ranges, OR syntax,
partial versions and trailing messages are rejected. `plain` trims surrounding
whitespace; `node` additionally requires and removes exactly one leading `v`.
This is a deliberately limited numeric contract, not an npm semver expression.

Provision must satisfy the request. A bounded version preflight runs before the
selected command. Invalid output, an out-of-range version or an unsupported
producer capability stops execution. The actual command's recorded executable
and normalized probe must match that preflight. Raw probes, normalized exact
versions, executable paths, argv, parser IDs, configuration/lockfile/package
digests, profile digest and source/head bindings remain in the result. A compatible
range does not prove who ran a tool, and all direct collection stays LocalUntrusted.

## Supported adapters and observed combinations

| Role | Adapter / capability | Accepted producer interval | Native combinations exercised locally |
| --- | --- | --- | --- |
| Runtime | Node numeric probe | Adopted request | 22.18.0 and 22.19.0 |
| Install / lock / audit | npm / `npm_audit` | >=10.9.3, <11.0.0 | npm 10.9.3 and 10.9.4, lockfile v3 |
| Build / static analysis | Explicit process argv | Adopted request | TypeScript 5.9.2, ESLint 9.34.0, Vite 7.3.6; Node syntax check in the alternate consumer |
| Unit / contract | `vitest_json` | >=3.2.4, <5.0.0 | Vitest 4.1.11; historical 3.2.4 report contract retained |
| Unit | `jest_json` | >=30.2.0, <31.0.0 | React SSR with Jest 30.2.0 under `web/` |
| E2E | `playwright_json` | >=1.55.0, <1.56.0 | Playwright 1.55.1 with Chromium |
| SDK / restore / audit | NuGet / `nuget_audit` | SDK >=9.0.100, <10.0.0 | SDK 9.0.304, explicit net9.0 graphs |
| Unit / integration report | `trx` | SDK >=9.0.100, <10.0.0 | VSTest 17.14.1 flat TRX on SDK 9.0.304 |

Intervals define accepted adapter contracts; this table does not claim that every
intermediate release was executed. Presets retain their exact reviewed package
selections. The additional fixture was chosen to exercise an existing JSON test
report contract with React server rendering, a different test runner, Node/npm
versions and component path. It does not need Vite, Vitest or ESLint. Report
details and native limitations are in [report formats](report-formats.md) and
[acceptance](acceptance.md). The execution record distinguishes actual runs from
synthetic parser/version tests. Hosted Linux/macOS execution remains separately
observable; local runs here are macOS ARM64. Windows, alternative hosting,
pnpm/yarn/bun, arbitrary reporters and multi-target .NET projects are not certified.

## Package, graph and path contracts

`packages` binds installed package manifests to a declared lockfile JSON pointer
and compatible version request. The installed exact version must equal the lock
entry. npm graph validation compares manifest dependency declarations with the
lockfile root, and installation uses `npm ci`; audit uses `npm audit --audit-level=low`.
Both commands explicitly include dev, optional and peer dependencies with
`--include=dev --include=optional --include=peer`, overriding ordinary npm omission
settings. A native nested fixture reproduces the old `omit=dev` audit passing with
a vulnerable development dependency, then verifies the selected audit rejects it.
Nested npm graphs add `--prefix DIRECTORY` to both commands, so the declared graph
is actually installed and audited. Manifests must be named `package.json` and
bind the adjacent `package-lock.json`; NuGet projects bind their adjacent
`packages.lock.json`. Unsupported lockfile redirection is rejected.
An unrelated alternative-manager lockfile is not itself a schema 2 conflict.
Selecting its unsupported adapter is rejected.

`graphs` records component, application/unit/integration/shared role, manifest,
lockfile, adapter, tool and required audit sensor. Backend composition requires
separate app, Unit and Integration graphs and corresponding tests. Fullstack also
requires frontend and shared graphs. All declared manifests/locks are bound to
each component's execution configuration, including sensors that did not repeat
those inputs. Omitting a graph, audit, required flag, declared project reference
or corresponding test fails validation. Component roots and app/test paths are
data; safe relative paths and receipt ownership constraints still apply.

NuGet requires one explicit unconditional target framework per project, matching
its lock inventory. SDK selection is separate. Each graph runs locked forced
restore with all-severity transitive auditing, an explicit `NuGet.Config`, cleared
warning suppressions and audit warning/error codes promoted to errors. Audit XML
validation follows the declared adapter, independent of sensor names. Explicit
advisory suppression and local overrides of audit command properties are rejected.
Custom MSBuild execution is still contributor code: this is an audit input
contract, not a general MSBuild sandbox. Service/database errors, missing results
and timeouts cannot produce passing audit evidence.

`runtime_files` binds `.node-version` or `global.json` to the exact provision
choice. `global.json` must disable roll-forward and prereleases; a conflicting
existing configuration is reported instead of silently changed. Bootstrap checks
available inputs independently and compares pairs when both exist, while reporting
missing prerequisites; runtime validation requires
them all. Bootstrap invokes no package manager.

For a controlled CI job that already has Node and npm available, the optional
Rust provisioning command installs the exact selected npm into a fresh prefix:

```console
rtk cargo run --locked -p project-verification -- provision-npm examples/react-jest/project-profile.json --prefix /private/tmp/selected-npm
```

It verifies the installed version and prints its `node_modules/.bin` directory.
The caller adds that path to its job environment. Network/install/version errors
return 64. No global npm installation or user runtime configuration is changed.

## User profiles and ownership

Use `policy-bootstrap plan --target /absolute/project --profile-file config/profile.json
--release 1.1.0` with a target-relative existing schema 2 profile. Inspect and apply
the saved plan normally. The source and native configuration remain user-owned;
bootstrap manages only fixed integration guides/examples. The receipt records
the source path and digest, and application rechecks them. Removing or changing
the source does not prevent receipt-based uninstall from preserving it.
See [bootstrap](bootstrap.md) for migration, receipt and recovery procedures.

New release assets and schema 2 receipts do not rewrite frozen 1.0.0–1.0.2 bytes
or validate arbitrary receipt-owned files. Existing receipt inventories, exact
rendered content, conflicts, transaction recovery and removal checks remain.
