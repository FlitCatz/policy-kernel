# Explicit consumer composition (schema 3)

Project profile schema 3 pairs only with selection schema 2 and an explicit
`selection.consumer`. Schema 1/2 profiles keep their existing behavior and frozen
template assets. There is no implicit migration or policy adoption.

The [synthetic split-consumer fixture](../../examples/split-consumer/project-profile.json)
shows the full contract. It contains no product application code or data.

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

Schema 3 and schema 4 consumer profiles must declare at least one adequacy
requirement. Every negative ID must also occur in `changed_behavior_ids`, with
an ID-to-executed-test mapping. See [adequacy](adequacy.md) for validation,
legacy readiness changes and consumer-owned catalog boundaries.

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

## Compiler-generated files (P2)

Optional `consumer.generated_files_graphs: ["modules"]` binds the extension to
reviewed NuGet graph IDs and their existing manifest paths. An omitted or empty
list preserves the previous XML contract. Every graph in an opted-in component
must bind reviewed `framework_props`, including projects reached by references.

The opted-in project must declare exactly this pair, once, in the same direct,
unconditional `PropertyGroup`:

```xml
<PropertyGroup>
  <EmitCompilerGeneratedFiles>true</EmitCompilerGeneratedFiles>
  <CompilerGeneratedFilesOutputPath>obj/generated</CompilerGeneratedFilesOutputPath>
</PropertyGroup>
```

Names and values above are exact; neither property joins the generic safe-property
list. `false`, absolute paths, traversal, drive/UNC/backslash paths, expressions,
environment expansion, duplicate declarations (including case variants), property
or ancestor conditions, inherited declarations, `TreatAsLocalProperty`, imports
and targets fail closed. Project-reference property metadata is also rejected.
NuGet advisory suppression, alternate lock paths and audit-warning suppression
remain rejected. Nested `Directory.Build.*` and implicit MSBuild response files
cannot supply external overrides.

The whole component's actual command and environment allowlist are checked:
selected tools must be `dotnet` (or an explicit path ending in `dotnet`) with a
`--version` probe. Audit commands must exactly match the fixed NuGet adapter.
Other commands are `dotnet build PROJECT --no-restore` or
`dotnet test PROJECT --no-restore`, with only optional `--no-build`, `--filter`,
`--logger` and `--results-directory` test options, each at most once. CLI property
overrides, response files and other arguments are rejected. Environment keys are
limited to `PATH`, `HOME`, `DOTNET_ROOT`, `DOTNET_CLI_HOME`, `NUGET_PACKAGES`; the
collector clears ambient environment before forwarding allowed keys.

Output is relative to the reviewed project, not the component root. Preflight and
collection configuration checks reject symlinks in the project path, `obj`,
`obj/generated`, and existing output descendants. Missing output directories are
allowed before a build; an empty directory is not proof of generation. These
checks do not sandbox contributor code or protect against concurrent filesystem
mutation. Use an isolated checkout and trusted selected tool installation.

The offline native proof stages the synthetic backend, uses cached Mapperly 4.3.1
and the pinned RC1 SDK/VSTest 17.14.1, builds actual generated source, and runs a
C# ArchitectureTests-style test that reads and asserts its content. Replacing that
output with an empty directory must fail the same compiled test. The harness is
Rust; the C# source is fixture data. All NuGet feeds are cleared for this proof;
its locked cache-only restore disables audit and is **not** an audit result.

```console
POLICY_GENERATED_DOTNET=/absolute/installed/dotnet POLICY_GENERATED_PACKAGES=/absolute/existing/nuget/packages cargo test --locked --offline -p policy-acceptance --test native_generated_files -- --ignored --nocapture
```

Set `POLICY_GENERATED_OUTPUT` to a fresh absolute output directory to retain the
staged backend, command argv/status/stdout/stderr and baseline TRX. Commands time
out after 120 seconds. A restricted process/socket sandbox may prevent VSTest from
starting; the local proof needs permission to launch its native test processes.

Do not install or fetch missing SDKs/packages as part of this command. Missing
cached dependencies require a separate decision. A successful proof is labelled
**local scope satisfied**, remains LocalUntrusted, and proves neither actual
TJERP adoption nor authoritative CI or production promotion.

## Bootstrap

The compiler-generated-file extension above requires a reader containing
P2. Older readers reject its new field; frozen bootstrap assets are not rewritten.

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
POLICY_SPLIT_CONSUMER_OUTPUT=/private/tmp/split-consumer-native-new rtk cargo test --locked -p policy-acceptance --test native_split_consumer native_split_consumer_reports -- --ignored --nocapture
```

Use a fresh output directory. The Rust harness stages the fixture, installs locked
dependencies, runs actual audits and reports, rejects report tampering and stale
bindings, executes real failing Vitest/TRX/Playwright cases and verifies a corrected
new HEAD. It saves requests, results, reports, negative checks and harness identity.
Local success remains LocalUntrusted; hosted CI and production promotion remain
separate. See [Task 16](../tasks/task-16/README.md) for current evidence and limits.
