# Project verification

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

`ProjectProfile` schema 1 / asset version 1.0.0 wraps the v2 sensor definitions.
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

The three assets are [React/TypeScript](../../profiles/react-typescript/profile.json),
[.NET](../../profiles/dotnet/profile.json) and
[fullstack](../../profiles/fullstack/profile.json). Standalone component roots are
`.`; fullstack uses `frontend/`, `backend/`, and a `shared` component at the root.
Each application component has a required test suite with namespaced sensor IDs.

The assets describe an existing Vite/npm application and an existing .NET 9
application with `App.csproj`, `Tests/Unit/Unit.Tests.csproj` and
`Tests/Integration/Integration.Tests.csproj`. They do not generate those projects
or migrate Jest, other frontend frameworks, .NET targets, or test runners.
The per-sensor `inputs` list is the exact required file list. JSON pointers in
`pins` verify installed JS package versions, matching package-lock entries,
global.json SDK version, and Microsoft.NET.Test.Sdk in each test project's lock.
Changing paths or tool versions requires a new adopted profile digest.

| Tool | Asset pin |
| --- | --- |
| Node / npm | 22.18.0 / 10.9.3 |
| TypeScript / ESLint / Vite | 5.9.2 / 9.34.0 / 7.1.3 |
| Vitest / Playwright | 3.2.4 / 1.55.0 |
| .NET SDK / VSTest Test SDK | 9.0.304 / 17.14.1 |

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

The commands below are usage documentation, not commands executed during this task:

```console
rtk cargo run --locked -p project-verification -- validate-profile profiles/fullstack/profile.json
rtk cargo run --locked -p project-verification -- schema
rtk cargo run --locked -p project-verification -- prepare --input request.json
rtk cargo run --locked -p project-verification -- run --input request.json
```

`validate-profile` returns the typed profile digest and an unbound sensor profile.
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
sources, flat TRX support, retry handling and hand-authored fixture provenance.
Additional pin references: [Node/npm](https://raw.githubusercontent.com/nodejs/node/v22.18.0/deps/npm/package.json),
[Vite](https://raw.githubusercontent.com/vitejs/vite/v7.1.3/packages/vite/package.json),
[TypeScript](https://raw.githubusercontent.com/microsoft/TypeScript/v5.9.2/package.json),
[.NET SDK releases](https://builds.dotnet.microsoft.com/dotnet/release-metadata/9.0/releases.json),
[NuGet audit behavior](https://learn.microsoft.com/en-us/nuget/concepts/auditing-packages).

See [the execution plan](../archive/plans/2026-09-09-task-12-project-profiles.md) and
[implementation status](../archive/reviews/2026-09-09-task-12-project-profiles.md).
