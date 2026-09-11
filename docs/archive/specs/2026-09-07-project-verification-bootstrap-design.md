# Project verification and bootstrap design

> Date: 2026-09-07 (Asia/Seoul)
> Status: Proposed revision for review; implementation not performed
> Confirmed targets: React (TypeScript), C# (.NET / ASP.NET Core)
> Basis: Current kernel design and the user's clarified targets. Earlier chat's exact recommendation was unavailable.

## 1. Responsibility and placement

Use an existing development harness. Keep policy evaluation in the Rust kernel,
and distribute common project setup alongside it without coupling their runtimes.

| Component | Owns | Does not own |
| --- | --- | --- |
| External harness | Planning, code edits, model selection, review and repair scheduling | Authoritative release decisions |
| Bootstrap | Install versioned project profiles, check configuration and CI templates | Agent orchestration, application generation, remote protection |
| Project verification | Compiler, linter, tests, report normalization | Waivers or merge authorization |
| policy-core | Evidence validity, completeness, independence and release-policy decisions | Tool installation, command execution, project discovery |
| Collector / CI adapters | Bounded execution, trusted observations, evidence collection | New policy rules or model routing |

Initially keep `profiles/` and `bootstrap/` in this repository. Give each
profile/template release its own version and supported contract-version range.
This avoids a separate tool repository at first while keeping later extraction
possible. Neither Node nor .NET is required to build or test the pure Rust core.
The installer is separate from the advisory `policy-cli`.

Rejected alternatives: embedding JS/.NET tools in the core couples policy to
toolchains; immediately splitting repositories adds release coordination before
the integration contracts are proven.

## 2. Recommended verification defaults

These are proposed defaults for new consuming projects. Preserve an existing
equivalent toolchain when it emits the required evidence. Exact compatible versions
must be pinned in runnable profiles during implementation, not selected as
unbounded latest releases during bootstrap.

| Layer | React / TypeScript | C# / ASP.NET Core |
| --- | --- | --- |
| Compile and static checks | TypeScript type check, production build, ESLint | dotnet build, SDK/Roslyn analyzers, formatting verification |
| Unit / component checks | Vitest + React Testing Library | xUnit through dotnet test |
| API integration | Frontend request/response expectations | WebApplicationFactory-based API integration tests |
| End-to-end | Playwright against the real frontend and API | Same shared fullstack run; avoid duplicate suites |
| Dependencies | Lockfile-aware vulnerability check | NuGet dependency vulnerability check |

For React without Vite, adapt build/test commands to its actual framework.
Do not force Vite or replace working Jest tests merely to install a profile.
For .NET, pin the SDK and the test runner explicitly; choose report parsers for
that runner rather than assuming all dotnet test modes emit identical reports.
Use the project's supported .NET target; this proposal does not migrate runtimes.

The `fullstack` profile composes both profiles with namespaced sensor IDs and
adds API contract consistency and shared E2E checks. Generate or validate an
OpenAPI snapshot and frontend types where the project uses OpenAPI. A schema/type
check supplements runtime API tests; it cannot establish authorization or behavior.
Projects without OpenAPI must declare an equivalent contract check before adopting
the fullstack profile.

## 3. Required checks and cost control

Every applicable code change requires build/type checks, lint/static checks and
the declared unit/component tests. API changes require API integration tests;
cross-layer changes require contract and E2E checks. Authentication, permissions,
shared configuration and dependency changes trigger the affected integration suites.
If impact cannot be resolved reliably, use the complete applicable profile.

PR E2E starts with the project's critical journeys: login, role-based access and
a representative business transaction. Broader browser, performance and mutation
suites are scheduled or explicitly required by project policy. No universal
coverage percentage or performance budget is imposed by bootstrap.
Test adequacy still maps changed behavior and acceptance criteria to real tests.

Dependency findings follow the declared severity/waiver policy. Missing or stale
advisory data cannot be recorded as a successful vulnerability scan.
Missing tools, reports, parser failures or runner failures remain non-success.
A required test suite discovering zero tests is not a pass. Required skipped
tests cannot be hidden inside a green process result.

## 4. Bootstrap behavior

Inputs: destination, react-typescript/dotnet/fullstack profile, component paths,
pinned template release, and explicit toolchain choices when detection is ambiguous.

1. Inspect project manifests and existing configurations without writing.
2. Produce a file-level plan showing additions, unchanged managed files and conflicts.
3. Apply that plan only if its input files still match the inspected digests.
4. Record template version, contract compatibility and managed-file digests.

Install common check scripts, formatting/lint/test configuration, policy/sensor
profile files, CI templates and usage instructions. Existing business rules and
test cases remain project-owned. Bootstrap can install a test example, but an
example alone cannot satisfy the consuming project's test-adequacy requirement.

Repeated application is a no-op. Never overwrite a locally edited file automatically.
Conflicts stop application before any writes. Stage writes and restore original
files if application fails. Reject destination escapes and symlink escapes.
Upgrades show a diff and preserve local edits. Uninstall removes only unchanged
managed files. Do not install credentials, choose reviewers, alter branch protection
or merge pull requests.

## 5. Evidence flow and trust

Bootstrap installs configuration. The selected external workflow runs checks
locally for advisory feedback. CI reruns required checks at the evaluated head,
then external normalizers convert tool reports to the existing evidence contracts.
policy-core evaluates the resulting bundle; the required-check and protected-merge
authorization chain is unchanged.

Bind profile content/version, configuration and lockfile digests, tool and parser
versions, explicit argv, component paths, actual head, timestamps and report
digests through the policy snapshot and evidence fields. Reuse existing fields
when possible. If the current schema cannot express a required binding, version
the contract explicitly before implementing that profile; do not add silently
ignored fields to strict contracts.

Normalizers retain report counts and skip/failure detail plus original report
references. Their output does not create trust: authoritative provenance comes
from the current trusted CI execution. Local AI or test reports remain advisory.

## 6. Delivery and acceptance

Finish the existing core tasks, then deliver profiles/report normalization,
bootstrap and consuming-project acceptance as separately reviewable additions.
Profile design can proceed alongside core completion; authoritative integration
depends on the core contracts and CI adapter.

Acceptance covers all three profiles, clean and existing projects, repeat install,
edited-file conflicts, interrupted apply recovery, source-head mismatch, malformed
and empty reports, and real positive/negative CI runs. A missing runtime is reported
as unverified, never as a pass.

The previous Antigravity review covers the original kernel revision only.
This proposal needs its own review before implementation. It makes no claim that
the recalled harness-strategy conversation approved these exact tool choices.

## 7. Tool references

- [TypeScript noEmit](https://www.typescriptlang.org/tsconfig/noEmit.html)
- [ESLint setup](https://eslint.org/docs/latest/use/getting-started)
- [Vitest guide](https://vitest.dev/guide/)
- [React Testing Library](https://testing-library.com/docs/react-testing-library/intro/)
- [Playwright assertions](https://playwright.dev/docs/test-assertions)
- [.NET testing](https://learn.microsoft.com/en-us/dotnet/core/testing/)
- [.NET code analysis](https://learn.microsoft.com/en-us/dotnet/fundamentals/code-analysis/overview)
- [ASP.NET Core integration tests](https://learn.microsoft.com/en-us/aspnet/core/test/integration-tests)
