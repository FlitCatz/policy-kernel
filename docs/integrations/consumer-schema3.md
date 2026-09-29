# Explicit consumer composition (schema 3)

Project profile schema 3 pairs only with selection schema 2 and an explicit
`selection.consumer`. Schema 1/2 profiles keep their existing behavior and frozen
template assets. There is no implicit migration or policy adoption.

The [synthetic TJERP-shaped fixture](../../examples/tjerp-compatible/project-profile.json)
shows the full contract. It does not contain TJERP application code or data.

## Tool and runtime contracts

An optional tool `producer_contract` opts into one exact additional combination:

| Contract | Provision and observation | Capabilities |
| --- | --- | --- |
| `dotnet11_rc1` | `11.0.100-rc.1.26425.128` | Process, NuGet audit, TRX with Microsoft.NET.Test.Sdk 17.14.1 |
| `playwright163` | `1.63.0` | Process, Playwright JSON |

These contracts require `normalization: plain` and `requirement: =VERSION`.
Another RC, patch, GA, malformed or multiline probe is rejected. Ordinary numeric
selection and the legacy adapter intervals remain available without a new producer
contract. An RC producer requires `consumer.development_only: true`; it does not
grant production approval. VSTest's test project declaration and lock resolution
must both select 17.14.1 for RC/TRX suites. The lock must also resolve TestHost and
ObjectModel to 17.14.1 so a direct dependency cannot silently replace the producer.

`consumer.runtime_policies` maps `component:path` to `roll_forward` (`disable` or
`latestPatch`) and `allow_prerelease`. Only the explicitly selected development
SDK may permit prereleases. Runtime JSON must match these declarations exactly.
Even with latestPatch, the observed SDK must equal the exact provision; implicit
SDK migration does not widen the policy.

## Required suites and dependency graphs

`consumer.suites` is an array of `{role, sensor, graph}` bindings. Roles are
`unit`, `integration`, `contract`, `e2e`. Frontend requires unit; backend requires
unit and integration; fullstack also requires contract and E2E. Each role maps
to a distinct unconditional required test sensor with a report, manifest and
lockfile inputs. NuGet suites execute their declared project and report TRX;
E2E reports Playwright JSON.

A graph is inventoried/audited once and can serve multiple suite roles. For
example, separate filtered runs of one test project can provide unit, integration
and contract reports. Reusing executed identities from that graph across roles
is rejected. Missing, failed, empty and overlapping suite evidence rejects the
composed run (CLI exit 64); other failed process sensors retain exit 2 semantics.
The collector does not infer business-rule adequacy from suite labels.

Fullstack schema 3 needs frontend/backend components and their real application
graphs. It does not require an artificial shared npm project. Contract and E2E
may live in their actual backend/frontend components. Every referenced library
project still needs an audited graph. Other declared components also need an
application graph; unused synthetic components do not satisfy suite obligations.

## Bounded framework inheritance

`consumer.framework_props` maps NuGet graph IDs to component-root
`Directory.Build.props`. The declared file is bound to each component's input
digest and must provide one unconditional matching framework. Conflicting
project-level declarations, multiple frameworks, custom SDKs, imports, targets,
props items/references and unknown MSBuild properties are rejected.

Supported scalar properties are TargetFramework, Nullable, ImplicitUsings,
TreatWarningsAsErrors, RestorePackagesWithLockFile, IsTestProject, IsPackable,
OutputType, LangVersion and GenerateProgramFile. MSBuild property names and
framework-condition checks use case-insensitive comparison. Import hooks such as
CustomAfterMicrosoftCommonTargets are outside this subset.

Before preparation and each collection, the runner rejects intervening
Directory.Build.props, Directory.Build.targets and Directory.Packages.props.
Ancestor targets/packages files above the component are also rejected. These
checks supplement digest binding and reject unsupported layouts; they do not
turn contributor MSBuild execution into a sandbox.

## Bootstrap

Release 1.2.0 is **custom-profile only** and requires schema 3. Release 1.1.0 keeps
schema 2 user profiles and its existing presets. Receipt schema 3 identifies the
new release and its fixed ownership inventory. Native files remain user-owned.

```console
rtk cargo run --locked -p project-verification -- validate-profile /absolute/project/config/profile.json
rtk cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile-file config/profile.json --release 1.2.0 > /private/tmp/consumer-plan.json
rtk cargo run --locked -p policy-bootstrap -- apply --plan /private/tmp/consumer-plan.json
```

Prepare and adopt the actual source profile digest, then collect fresh HEAD-bound
evidence. Installation creates no tests, approvals, credentials or active workflow.
Uninstall uses the same profile path/release with `--uninstall` and preserves
edited/missing user source inputs. Edited managed files still conflict.

## Native verification

Provision the fixture's selected Node 26.10.0, npm 10.9.3, .NET SDK and Playwright
Chromium, with their executables on PATH and DOTNET_ROOT pointing to the selected
SDK installation. These are fixture selections, not requirements imposed on all
consumers. The initial npm lock was generated with npm 12.1.0; native execution
uses the selected npm 10.9.3 `ci` and `audit` with all dependency classes included.

```console
POLICY_TJERP_OUTPUT=/private/tmp/tjerp-native-new rtk cargo test --locked -p policy-acceptance --test native_tjerp native_tjerp_reports -- --ignored --nocapture
```

Use a fresh output directory. The Rust harness stages the fixture, installs locked
dependencies, runs actual audits and reports, rejects report tampering and stale
bindings, executes real failing Vitest/TRX/Playwright cases and verifies a corrected
new HEAD. It saves requests, results, reports, negative checks and harness identity.
Local success remains LocalUntrusted; hosted CI and production promotion remain
separate. See [Task 16](../tasks/task-16/README.md) for current evidence and limits.
