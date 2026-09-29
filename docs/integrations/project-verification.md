# Project verification

[Schema-3 consumers](consumer-schema3.md) can explicitly select the new exact
producer contracts and declare suite roles independently of preset directory
layout. Existing protocol/authority boundaries and legacy schemas remain intact.

Task 12 implementation includes the Rust `project-verification` crate, three
versioned profile assets and report adapters. Task 14 has run the Rust regression
suites; native producer and consuming-project certification is tracked separately
in the [acceptance execution record](../archive/reviews/2026-09-09-task-14-consuming-acceptance.md).
These assets are not a verified release. The separate [bootstrap installer](bootstrap.md)
installs versioned configuration.

## Required result contract

`SensorProfile` v2 declares a `result_requirement` for every sensor:

| Value | Required result |
| --- | --- |
| `process` | Existing typed execution provenance and accepted exit status; test counts are optional. |
| `test_counts` | Existing typed provenance plus all three counts: `discovered`, `executed`, `passed`. |

A required test-count sensor cannot pass by omitting counts even when another
suite provides positive aggregate adequacy. Existing validation still rejects
partial counts, zero discovery, count disagreement and non-success sensor status.
Counts alone do not establish acceptance-criterion coverage: the separate adequacy
mapping and all other kernel gates remain required when declared by policy.

These are policy requirements, not values inferred from a submitted report.
The canonical profile digest in `PolicySnapshot.sensor_profile_digest` binds them.
Changing a sensor from `test_counts` to `process` changes that digest and requires
an explicitly adopted policy update. A report normalizer cannot authorize the change.

## Compatibility

The profile version is independent of the existing version-1 evidence and decision
envelopes. Version-1 profiles retain their historical field representation and
legacy result-count behavior. They are not complete Task 12 project profiles.
Version-2 profiles require explicit result requirements. A version-1 profile with
v2 requirements, a v2 profile missing requirements, or an unsupported profile
version is invalid. Old readers reject v2 rather than interpreting it as v1.

Do not upgrade stored bundles by inserting fields or silently recomputing their
digests. Adopt a new profile/policy and collect fresh evidence for that policy.
Local normalization remains advisory; authenticated CI observations and the
existing exact-head pre-merge checks are still necessary for authorization.

## Versioned execution protocol

`ProjectProfile` schema 1 / assets 1.0.0–1.0.2 and schema 2 / asset 1.1.0 wrap
the v2 sensor definitions. Schema 2 adds the [selection contract](tool-selection.md)
and supports user asset versions; sensor-profile versioning is independent.
Strict JSON parsing and the generated schema reject unknown fields; semantic
validation rejects duplicate IDs, unsafe relative paths, missing application
components, report/result mismatches and missing fullstack contract/E2E suites.
Raw profiles must not contain result bindings: `prepare` derives them from checked
input files. Empty adequacy requirements in the distributed assets are placeholders
for consumer policy authoring, not evidence of application behavior coverage.

`Impact` is explicitly `{"kind":"unknown"}` or
`{"kind":"known","paths":["frontend/src/App.tsx"],"risk":"high"}`.
Unknown impact selects all sensors and makes them required in the prepared sensor
profile. Known impact uses the core's declared conditions. An empty known path
list does not mean unknown. The supplied assets deliberately use a conservative
always-required baseline, including integration/contract/E2E; consumers can adopt
reviewed conditional policies where the project contract permits them.

The three repository-root assets are retained historical 1.0.0 profiles:
[React/TypeScript](../../profiles/react-typescript/profile.json),
[.NET](../../profiles/dotnet/profile.json), and
[fullstack](../../profiles/fullstack/profile.json). The historical fullstack
profile has 14 sensors. Do not treat these stable compatibility assets as an
implicit latest release; see the [profile asset guide](../../profiles/README.md).

For current direct validation, select the explicit 1.1.0 asset under
`bootstrap/templates/1.1.0/profiles/`, or the installed/user-owned profile from a
reviewed bootstrap plan. Fullstack 1.1.0 has 17 required sensors: shared npm plus
app, Unit and Integration NuGet audits. Frozen 1.0.2 has 15 including shared npm. Standalone component
roots are `.`; fullstack uses `frontend/`, `backend/`, and a `shared` component
at the root. Each application component has a required test suite with
namespaced sensor IDs.

The assets describe an existing Vite/npm application and an existing .NET 9
application with `App.csproj`, `Tests/Unit/Unit.Tests.csproj` and
`Tests/Integration/Integration.Tests.csproj`. They do not generate those projects
or migrate existing applications. A schema 2 user profile can select the supported
Jest adapter, runtime/package versions and other safe app/test paths. The per-sensor
`inputs` list plus all selected component inputs is the required file list. JSON pointers in
`pins` verify installed JS package versions, matching package-lock entries,
global.json SDK version, and Microsoft.NET.Test.Sdk in each test project's lock.
Changing paths or tool versions requires a new adopted profile digest.

| Tool | Root historical 1.0.0 | Bootstrap 1.0.2 |
| --- | --- | --- |
| Node / npm | 22.18.0 / 10.9.3 | 22.18.0 / 10.9.3 |
| TypeScript / ESLint | 5.9.2 / 9.34.0 | 5.9.2 / 9.34.0 |
| Vite / Vitest | 7.1.3 / 3.2.4 | 7.3.6 / 4.1.11 |
| Playwright | 1.55.0 | 1.55.1 |
| typescript-eslint parser / jsdom | Not pinned by these root assets | 8.42.0 / 26.1.0 |
| .NET SDK / VSTest Test SDK | 9.0.304 / 17.14.1 | 9.0.304 / 17.14.1 |

Embedded bootstrap 1.0.0 is not byte-identical to the root 1.0.0 profiles: it
already adds the parser/jsdom bindings. See the
[bootstrap release table](bootstrap.md) for the embedded assets.

These are fixed compatibility candidates, not a claim of current recommended
versions or a tested toolchain matrix. Restore dependencies and browser binaries
in the consumer's existing controlled setup. Required config and lockfiles must
already exist. Vitest retries are disabled on argv. The consumer's Playwright
configuration must select its JSON reporter with outputFile
`.policy-reports/e2e.json`; it must also configure the real application/API web
servers. Missing output fails instead of assuming that a green process wrote it.

NuGet audit uses explicit audit configuration, disabled HTTP cache and escalated
NU1900–NU1905 warnings. Its `NuGet.Config` must declare HTTPS auditSources; declared
XML inputs containing NuGetAuditSuppress are rejected by the default adapter.
All imported MSBuild/configuration files belong in the reviewed source inventory
and relevant input lists. Native tool policy and its source completeness remain
the consumer CI's responsibility. Native audit failures are never converted to a
successful sensor merely because another test passed.

## Commands and bindings

Commands for the current profile contract:

```console
rtk cargo run --locked -p project-verification -- validate-profile bootstrap/templates/1.1.0/profiles/fullstack/profile.json
rtk cargo run --locked -p project-verification -- schema
rtk cargo run --locked -p project-verification -- prepare --input request.json
rtk cargo run --locked -p project-verification -- run --input request.json
```

`validate-profile` returns the typed profile digest and an unbound sensor profile.
To validate a bootstrapped consumer instead, replace the path with
`/absolute/project/.policy/project-profile.json`.
`schema` emits the project schema from the same Rust types used by validation;
it does not overwrite the existing five kernel schemas. `prepare` only reads and
validates inputs, returning checked configurations and a candidate bound sensor
profile. `run` repeats preflight, executes explicit argv through the existing
bounded collector, and prints observations. CLI exit 0 means selected executions
and reports passed, 2 means non-success observations, and 64 means invalid input
or collection preparation failure. None is a kernel authorization decision.

The strict `RunRequest` object contains:

- `schema_version: 1`, inline `profile`, and `expected_profile_digest`.
- Canonical absolute checkout `root`, `repository`, and `head_sha`.
- `source_files`: unique checkout-relative `{path,digest}` entries, and
  `expected_source_digest` for the sorted inventory.
- `provider`, `model`, unique `execution_id`, explicit `impact`, and
  `mappings` containing acceptance_criteria, changed_behaviors and negative_case_ids.

Digest construction uses `policy_core::canonical_sha256` for typed JSON and raw
SHA-256 for files. The profile digest hashes the deserialized ProjectProfile,
including defaults; it is not the digest of formatted profile.json bytes. Source
inventory is sorted by path before hashing. Every named file is read and compared
with its expected digest before and after execution. The adapter does not infer
inventory completeness or authenticate Git HEAD; trusted CI must establish both.

Each `ConfigurationBinding` hashes schema version, project-profile digest, the
complete ProjectSensor, absolute component cwd, and sorted actual input file
digests. The execution input hashes schema version, configuration digest,
repository, head and source-set digest. The collector's command/record digests
retain actual argv, cwd, process status, version probe and original artifact hashes.
The normalized result digest hashes the collection digest, original report
reference, parsed counts/test details and diagnostics. Raw floating-point reporter
metadata is not canonicalized into kernel evidence.

Prepared v2 definitions bind configuration_digest, parser_version and
resolved_command. The core compares passed evidence with those policy-bound
values (`PK-SENSOR-RESULT-BINDING` on mismatch). The prepared profile is a candidate
for explicit policy adoption; it does not update policy or approvals. Absolute cwd
and changed input bytes affect its digest, so moving a checkout or updating inputs
requires new bindings. Never reuse an old policy/evidence digest after doing so.

## Failure handling and trust

Execution is local advisory on supported collector platforms (Linux/macOS).
Environment names are explicitly allowlisted; values are not copied into the
protocol. Launched native tools retain their normal filesystem privileges: this
adapter is not an operating-system sandbox. Component/input symlinks and lexical
escapes are rejected; file reads walk descriptors with no-follow semantics.
An attacker changing a component directory concurrently with native process
startup is outside the path-check guarantee and requires an isolated trusted CI
checkout. Do not run untrusted project commands on an unprotected host.

Pre-existing reports are refused without deletion or overwrite. Use a clean
consumer checkout/report destination for each execution. Missing, malformed,
oversized, digest-mismatched, or old reports remain non-success. Parsed report
start must fall within the corresponding process interval (1 ms allowance for
millisecond timestamp truncation). Process failure, timeout, truncation, version
mismatch and artifact errors cannot be upgraded by a successful report.

Outputs retain each original collection record and each normalized sensor. Both
execution and sensor evidence remain `LocalUntrusted`, including imported records
claiming CI trust. The API validates transport hashes and configuration before
normalization; hashes alone do not authenticate a producer. Trusted collection
and existing CI observations are still required for authoritative kernel use.

Adequacy test IDs are namespaced by component and sensor. Mappings must reference
executed tests in the same component; no coverage mapping is inferred from names.
The native report's per-suite counts and the core's required-count gate prevent
one component from hiding another component's empty or skipped suite. Preserve
both sensor and execution evidence when constructing the bundle. Real CI binding,
independent review and consuming-project positive/negative runs remain Task 14
acceptance work.

See [report formats and limitations](report-formats.md) for the pinned producer
sources, flat TRX support, retry handling and fixture provenance. See the
[selection contract](tool-selection.md) for compatible ranges, actual observations,
CI inputs, user profiles and package/graph checks.
Additional pin references: [Node/npm](https://raw.githubusercontent.com/nodejs/node/v22.18.0/deps/npm/package.json),
[Vite](https://raw.githubusercontent.com/vitejs/vite/v7.1.3/packages/vite/package.json),
[TypeScript](https://raw.githubusercontent.com/microsoft/TypeScript/v5.9.2/package.json),
[.NET SDK releases](https://builds.dotnet.microsoft.com/dotnet/release-metadata/9.0/releases.json),
[NuGet audit behavior](https://learn.microsoft.com/en-us/nuget/concepts/auditing-packages).

See [the execution plan](../archive/plans/2026-09-09-task-12-project-profiles.md) and
[implementation status](../archive/reviews/2026-09-09-task-12-project-profiles.md).
