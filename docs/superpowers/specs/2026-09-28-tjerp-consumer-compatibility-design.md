# TJERP consumer compatibility extension

Status: proposed design; implementation and native verification have not started.
Date: 2026-09-28
Baseline: policy-kernel `31cadf7d88276c0e00752b72c2b19ef6e2d199bf`.

## Intent and completion boundary

Enable a reviewed user profile to verify the existing TJERP React/.NET layout,
with exact development tool selections, auditable dependency graphs and real test
reports. Preserve historical profiles, frozen bootstrap assets and authority
boundaries. The user authorized starting this work after discussing compatibility
gaps; this document makes the proposed contract changes reviewable.

Completion means the extension passes existing regressions and positive/negative
native acceptance for the selected consumer combination. Local results remain
LocalUntrusted. Hosted execution, required-check activation, DB2 verification and
production promotion require their own evidence; they are not implied by local
acceptance. TJERP installation is a subsequent consumer task.

## Observed consumer and blockers

The TJERP files were inspected locally, without changing them.

| Consumer fact | Current kernel constraint |
| --- | --- |
| SDK 11.0.100-rc.1.26425.128 | Numeric stable versions only; TRX/NuGet SDK interval is >=9.0.100, <10.0.0 |
| global.json uses latestPatch and allowPrerelease=true | Runtime binding requires rollForward=disable and rejects prerelease |
| Playwright lockfile selects 1.63.0 | Producer interval is >=1.55.0, <1.56.0 |
| Vitest 4.1.11, TypeScript 5.9.3, Vite 6.4.3 in lockfile | Need explicit consumer selections, not preset replacement |
| net11.0 inherited from backend/Directory.Build.props | Each csproj must declare one unconditional TargetFramework |
| API references Modules; one TjArch.Tests project holds several test categories | All references need graph coverage; backend requires distinct Unit/Integration roles while duplicate graph manifests are rejected |
| Contract tests live in backend; E2E lives in frontend | fullstack requires a shared component, shared npm graph and shared:contract/shared:e2e sensors |

Relevant sources:
[selection](../../../crates/project-verification/src/selection.rs),
[profile](../../../crates/project-verification/src/profile.rs),
[bootstrap contract](../../integrations/bootstrap.md),
[tool selection contract](../../integrations/tool-selection.md).

A complete adoption also needs consumer lockfiles, an explicit NuGet.Config,
report output settings, adequate test mappings and reproducible server startup.
These are prerequisites, not automatically generated passing evidence.

## Alternatives and selected approach

1. Change TJERP to fit the preset: downgrade tools, split test projects and add a
   shared npm project. This changes the consumer architecture for installer
   convenience and does not meet the requested compatibility objective.
2. Widen existing ranges and remove structural checks. This changes the meaning
   of previously adopted profiles and weakens missing-evidence detection.
3. Add an explicitly versioned consumer contract, retaining old readers' meaning.
   Recommended: profile schema 3 and selection schema 2, accepted only together.
   Existing profile schemas 1/2 and selection schema 1 retain their behavior and
   canonical serialized content. No automatic migration or adoption occurs.

## Proposed contract

### Exact prerelease and producer selection

Keep the stable numeric parser and existing adapter intervals unchanged for old
contracts. New tool selections explicitly name their producer contract. Initially
add only the candidate combinations needed here: .NET SDK
11.0.100-rc.1.26425.128 with VSTest 17.14.1/TRX and NuGet audit, and Playwright
1.63.0 JSON. Native acceptance must establish these before documentation describes
them as verified.

Prerelease selection requires exact equality and a development-only designation.
Reject prerelease ranges, unrecognized suffixes, extra probe output, another RC,
and GA substituted for RC. Preserve the full version in provisioning, observed
evidence and profile digests. Do not infer support for all .NET 11 or intermediate
Playwright releases from one successful combination.

Runtime bindings explicitly declare permitted global.json settings. For TJERP,
latestPatch and allowPrerelease=true may be represented, but execution still
requires the exact selected observed SDK; roll-forward never authorizes a
different SDK. Stable legacy runtime rules remain unchanged. The development
designation is not release authority; policy and promotion gates still apply.

### Composition independent of preset directory layout

Schema 3 declares required suite roles explicitly: unit, integration, contract and
E2E, mapped to required sensor IDs in their actual components. A fullstack consumer
must cover frontend and backend tests plus contract and E2E obligations, but need
not create an otherwise nonexistent shared package graph.

Inventory each real dependency graph once. Permit a test graph to serve multiple
declared suite roles through explicit sensor mappings. Each mapped test sensor
must reference the graph and its lockfile and produce a distinct, nonempty report.
For separately filtered unit/integration runs, reject overlapping executed test
identities that would count the same tests as both suites. Coverage adequacy is
still explicit; labels and filenames do not automatically prove a business rule.

Keep graph audit requirements, transitive ProjectReference coverage and report
failure/empty/skip checks. Additional library projects, including Modules, must
have audited graphs. No component-name trick or omitted graph bypasses validation.

### Bounded inherited framework support

Support a declared component-local Directory.Build.props containing exactly one
unconditional TargetFramework, with no competing project-level framework.
Include it in every affected graph's bound inputs and digest. Resolve and bind the
applicable implicit props path; reject conflicting nested props, conditional
framework definitions, TargetFrameworks, custom imports or target redefinitions
outside this supported subset. This is a deliberately bounded contract, not a
general MSBuild evaluator. Cover framework/lock agreement and input changes.

### Bootstrap and evidence integration

Accept the new contract only through explicit user-profile installation.
Preserve frozen 1.0.0-1.1.0 preset bytes and historical receipt semantics.
If receipt validation depends on supported schema inventories, introduce a new
release/receipt discriminator rather than reinterpreting an old receipt.

Bootstrap continues to manage integration assets only in custom mode. Profile,
native configs, test code and dependency provisioning remain consumer-owned.
Preparation binds profile, producer contract, runtime/props/manifests/locks,
source inventory and HEAD. Evidence normalization, controller authentication and
promotion boundaries remain mandatory.

## Acceptance design

Use a small synthetic consumer fixture under examples/, mirroring relevant TJERP
structure without copying ERP source or data. Rust owns orchestration and
verification; C#/TypeScript are fixture application/tests only.

Required checks:

- Existing profile parse/run/bootstrap/receipt regression tests stay green.
- Old canonical profile digests and frozen template bytes remain unchanged.
- New exact SDK and Playwright selections pass only for their declared contracts.
- Another RC, implicit SDK drift, unsupported producer and malformed version fail.
- Inherited props succeed; conflicts, escaped paths, unbound imports, changed props
  and mismatched framework lock inventories fail.
- A combined test project can produce distinct unit/integration evidence.
- A missing role, duplicate counted suite, missing graph/reference/audit or shared
  graph fabricated instead of actual dependencies cannot pass.
- Actual Vitest, VSTest TRX and Playwright reports normalize with nonzero tests.
- Missing/empty/truncated reports, failing/skipped-only suites, retries where
  prohibited, tool/service failures and stale HEAD/profile results fail closed.
- Dependency audit examines every selected application/library/test graph.
- Custom bootstrap plan/apply/uninstall preserves native files and profile ownership.

Record command, source commit (or explicit dirty state), tool versions, report
digests and result for each native run. Synthetic parser tests are not native
acceptance. A staged TJERP checkout can be assessed after the fixture passes;
consumer prerequisites discovered there must be reported separately.

## Implementation sequence for the subsequent plan

1. Add failing compatibility/legacy-preservation tests and new versioned types.
2. Implement exact producer/runtime selection with rejection-path coverage.
3. Implement explicit suite-role/graph bindings and bounded props inheritance.
4. Integrate preparation, normalization, schema generation and custom bootstrap.
5. Add and execute the native consumer fixture and failure scenarios.
6. Update README.md and README_KO.md together, integration docs, task/status records
   and lat.md navigation; record remaining external gates accurately.

Each step must retain existing fail-closed behavior. Do not replace a failed
native run with an expanded allowlist or a fabricated report.

## Review scope

Approve or adjust the new versioned-contract approach, exact development SDK
support and composition model before implementation planning. This proposal does
not claim that the adapter extensions or TJERP integration have passed.
