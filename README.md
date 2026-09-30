# policy-kernel

Deterministic Rust components for evaluating evidence against a release policy.
The kernel is independent of agent harnesses and model providers: it evaluates
typed input, produces typed decisions, and puts authentication and release
authority at explicit integration boundaries.

[한국어 문서](README_KO.md)

## What this repository provides

- Versioned policy, evidence, identity, and decision contracts with JSON Schemas.
- Deterministic pre-merge and post-merge policy evaluation in `policy-core`.
- A local advisory CLI and an authoritative GitHub Actions `policy-ci` boundary.
- Bounded evidence collection, project-profile validation, report normalization,
  and a transactional bootstrap installer.
- Rust-native consumer acceptance fixtures for React/TypeScript, .NET, and
  full-stack applications.

`ReadyToMerge` is a decision over supplied evidence; it does not grant shipping
permission. A release still needs authenticated CI observations, an exact-head
required check, protected-branch merge evidence, and owner approval. Read the
[trust model](docs/trust-model.md) before using results for release work.

## Status and scope

Tasks 1–14 provide implemented components in this workspace. The
[current status](docs/status.md) and [Task index](docs/tasks/README.md) identify the supporting evidence and
remaining implementation, integration, and operational conditions.
[Task 15](docs/tasks/task-15/README.md) tracks compatible tool/version selection,
runtime independence, dependency and knowledge evidence gaps, and authenticated
consumer integration. Follow its landing page and the current-status document
for completed corrections and remaining work.
Local implementation and verification through 15.10 are complete. Published
candidate `751fa76` passed hosted Release verification, the Rust 1.85/1.95
Ubuntu/macOS matrix and four native consumer jobs; see the
[hosted record](docs/archive/reviews/2026-09-16-task-15-hosted-verification.md).
Hosted mutation passed the former policy on that candidate (448 caught, 13
unviable, zero missed or timed out). Those results fail the strengthened policy:
every selected mutant must build and be caught. The
[mutation follow-up](docs/archive/reviews/2026-09-21-task-15-strict-mutation.md)
records the exact generation/replacement contract and subsequent local execution.
The new local rerun caught 449 native and 12 replacement mutants, with zero
unviable/missed/timeouts and five successful baselines. The twelve replaced
generator forms receive no caught credit. The measurements used a dirty candidate,
subsequently committed as `1d48a92` and integrated into remote main in `c79571e`.
No new hosted verification result is recorded here.
15.11 still requires operational evidence and owner approvals.
This does not claim a production promotion:
the live shadow window, protected-merge evidence, and independent-provider
reviews in the [promotion protocol](docs/promotion.md) remain separate
requirements.

A 2026-09-30 [codebase review remediation](docs/archive/reviews/2026-09-30-codebase-review-remediation.md)
tightens kernel semantics: a sensor not-applicable exemption must cover every
changed path, a fully waived blocked review no longer conflicts with an approval,
and a gate without a reason cannot leave readiness intact. Signed controller bytes
now come from explicit canonical JSON. That record contains local verification
only; no hosted result or promotion is claimed for it. A follow-up
[remaining area review](docs/archive/reviews/2026-09-30-remaining-area-review.md)
makes bootstrap-installed files follow the umask and extends the strict mutation
gate to sensor evaluation, also with local verification only.

The project is Rust-first. Repository execution logic, CLIs, verification tools,
report conversion, bootstrap installation, and tests are Rust. JavaScript,
TypeScript, and C# occur only in the approved native consumer fixtures and
declarative tool configuration under `examples/`.

## Requirements

Frontend projects can opt into [project-owned tool versions](docs/integrations/tool-selection.md#project-owned-frontend-versions-schema-4): profile schema 4 / selection schema 3 separates version acceptance from the Vitest, Jest, Playwright, process, and npm audit contracts. Provisioning is optional; tool observation and declared runtime/lock checks remain mandatory. `migrate-profile FILE --target-schema 4` prints a candidate without changing the source. Custom bootstrap 1.3.0 accepts this frontend profile and leaves native files user-owned. The [native conformance guide](docs/integrations/acceptance.md#project-owned-frontend-conformance) covers supplied consumer candidates and raw report retention. See [Task 17](docs/tasks/task-17/README.md) for validation scope and the exact-head hosted observation; production promotion remains separate.

Explicit [schema-3 consumer profiles](docs/integrations/consumer-schema3.md) now
support the selected .NET 11 RC1/VSTest 17.14.1 and Playwright 1.63.0 combination,
inherited framework configuration and combined test projects. Use custom bootstrap
release 1.2.0 for schema 3; existing 1.1.0 presets remain unchanged. See
[Task 16](docs/tasks/task-16/README.md) for local evidence and remaining adoption gates.

- For source builds, Rust **1.85.0** is the declared native workspace minimum. The checked-in
  [rust-toolchain.toml](rust-toolchain.toml) keeps **1.95.0** with `rustfmt` and
  `clippy` as the reproducible development and release-verification default.
  See [Rust and dependency compatibility](docs/integrations/rust-compatibility.md).
- The exercised `policy-cli`, collector, project-verification, and bootstrap
  binary paths do not require Rust or an agent framework at runtime. A compatible
  OS/system runtime and caller-selected external tools are still required.
  Repository verification commands such as `xtask check-residue` require Cargo
  and its selected Rust tooling.
- Git for the CI adapter and consumer-acceptance harness.
- Native tools follow the adopted [selection contract](docs/integrations/tool-selection.md).
  Built-in provisioning chooses Node 22.18.0 / npm 10.9.3 and .NET SDK 9.0.304;
  the additional React/Jest consumer chooses Node 22.19.0 / npm 10.9.4.
  Requests, exact provisioning choices and actual observations remain distinct.
- OpenSSL for the authenticated controller and its cryptographic tests; see the
  [controller reference](action/controller.md) for owner-controlled trust anchors.

## Get started

Clone the repository, then build and test the locked workspace:

```console
cargo build --locked --workspace
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

The manifests use reviewed compatible dependency ranges and the checked-in
`Cargo.lock` records the exact tested resolution. Use `--locked` for reproducible
builds and checks; a newly resolvable compatible version is not automatically a
tested version.

### Try local advisory evaluation

`policy-cli` reads a JSON file or `-` for standard input. It always evaluates
pre-merge input in advisory mode, even when the input requests authoritative mode.

```console
cargo run --locked -p policy-cli -- evaluate-pre tests/fixtures/pre-merge/ready.json
cargo run --locked -p policy-cli -- inspect tests/fixtures/evidence/valid.json
cat decision.json | cargo run --locked -p policy-cli -- inspect -
```

`evaluate-pre` exits 0 for `ReadyToMerge`, 2 for `Blocked`, 3 for
`Inconclusive`, and 64 for invalid input. `inspect` validates and displays a
typed decision or evidence bundle; it does not recompute or authenticate a
supplied decision.

## Workspace map

| Package | Purpose |
| --- | --- |
| `policy-contracts` | Strict, versioned wire types and schema source types. |
| `policy-core` | Deterministic policy, provenance, review, sensor, and merge evaluation. |
| `policy-cli` | Local advisory pre-merge evaluation and typed-object inspection. |
| `policy-ci` | Authoritative GitHub Actions evaluation and dedicated check publication. |
| `evidence-collector` | Bounded explicit-argv execution with artifact digests. |
| `github-adapter` | GitHub observation and process adapter used by CI integration. |
| `project-verification` | Profile validation, preparation, execution, and report normalization. |
| `policy-bootstrap` | Transactional profile installation, recovery, and uninstall. |
| `policy-acceptance` | Rust harness that stages and exercises native consumer fixtures. |
| `xtask` | Schema generation and repository/release verification commands. |

## Schemas and repository checks

Generate the five checked-in contract schemas in [`schemas/`](schemas/):

```console
cargo run --locked -p xtask -- schema
```

The command resolves its workspace output directory itself, so it can be launched
outside the repository with Cargo's `--manifest-path`. Verify generated schemas,
the documentation surface, source-language residue, and a supplied mutation
result with:

```console
cargo run --locked -p xtask -- check-residue
cargo run --locked -p xtask -- check-public-api target/doc schemas
cargo fetch --locked
cargo run --locked --offline -p xtask -- run-mutation-suite target/promotion-mutation
cargo run --locked --offline -p xtask -- check-mutation-suite target/promotion-mutation
cargo test --locked -p xtask
```

The suite covers all five core authorization modules, controller and
workflow-source. It rejects unviable, missed, timed-out or incomplete results and
checks the unfiltered inventory against every native and typed replacement
execution. Use a new output directory each time. The per-inventory
`check-mutation-results OUTCOMES [--scope core|controller|workflow-source]` command
defaults to core; it cannot establish full-suite completion on its own. See the
[mutation contract and commands](docs/promotion.md#mutation-generation-and-execution-contract).

`xtask rollback-drill` compares caller-supplied normalized snapshots only. It
does not query or change GitHub. Its arguments and the live rollback procedure
are documented in [promotion and rollback](docs/promotion.md).

## Project verification and bootstrap

`project-verification` turns an adopted project profile and checked source
inventory into local observations. It supports React/TypeScript, .NET, and
full-stack assets and user profiles, and normalizes Vitest JSON, Jest JSON,
Playwright JSON, and VSTest TRX
reports. The result remains local and advisory until a consumer binds it into the
trusted CI path.

Schema 2 dependency audits cover each selected graph. npm installation and audit
explicitly include dev, optional and peer dependencies, including when npm
configuration would otherwise omit them.
Native acceptance also checks empty and missing Vitest, TRX and Playwright reports
with successful producer exits, corrections and rejection of earlier-head results.

```console
cargo run --locked -p project-verification -- validate-profile bootstrap/templates/1.1.0/profiles/fullstack/profile.json
cargo run --locked -p project-verification -- schema
cargo run --locked -p project-verification -- prepare --input request.json
cargo run --locked -p project-verification -- run --input request.json
```

The repository-root `profiles/fullstack/profile.json` is a retained 1.0.0 asset
with 14 sensors. Frozen release 1.0.2 has 15 required sensors including
`shared:dependencies`; current release 1.1.0 uses profile schema 2 and has 17,
adding separate Unit and Integration NuGet audits. After bootstrap installation,
validate the selected copy at `.policy/project-profile.json`. See the
[profile asset guide](profiles/README.md) for the historical/current distinction.

`policy-bootstrap` installs reviewed, selected configuration into an existing
consumer directory. Inspect and save a plan before applying it; `apply`
revalidates the plan before it changes files.

```console
cargo run --locked -p policy-bootstrap -- inspect --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api
cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api > /absolute/bootstrap-plan.json
cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-plan.json
```

Use a new output directory for `policy-acceptance`; `--provision` permits locked
dependency restoration and browser provisioning in staged fixtures.

```console
cargo run --locked -p policy-acceptance -- run --profile fullstack --output /private/tmp/policy-acceptance-fullstack --provision
```

Read [project verification](docs/integrations/project-verification.md),
[bootstrap](docs/integrations/bootstrap.md), [report formats](docs/integrations/report-formats.md),
and [consumer acceptance](docs/integrations/acceptance.md) before using these
tools in a project.

## Local code graph

This repository uses [CodeGraph](https://github.com/colbymchenry/codegraph) as a
local development aid for Rust source structure and impact analysis. It is not a
CI input, policy evidence, or release-approval mechanism. The generated
`.codegraph/` SQLite index is intentionally ignored by Git.

Install the CLI once, connect it to Codex, then create or refresh this
repository's index:

```console
npm install -g @colbymchenry/codegraph
codegraph install --target=codex --location=global --yes
codegraph init
codegraph status
```

Restart Codex after the MCP installation. With the index available, use the
`codegraph_explore` MCP tool for structural questions, or use CLI commands such
as `codegraph explore "How does pre-merge evaluation flow?"` and
`codegraph impact PolicySnapshot`.

## CI integration

The checked-in composite [GitHub Action](action/README.md) invokes an
already-built, owner-installed `policy-ci` binary through the absolute
`executable-path` input (default `/opt/policy-kernel/bin/policy-ci`). It verifies exact GitHub
workflow and head identity in a runner-controlled environment, then publishes
the `policy-kernel/pre-merge` check. Do not run it from a contributor-controlled
checkout or treat local `policy-cli` output as a required check.

`policy-ci` has `pre-merge` and `post-merge` commands and accepts only
authoritative mode. Its runner-controlled observations and CLI arguments are
documented with the action. The [authenticated controller reference](action/controller.md)
verifies independent observer and producer signatures before issuing a receipt;
its local cryptographic tests do not establish deployed provider services.
Policy schema 2 can require explicit knowledge evidence. Policies with no such
requirement continue to allow an empty knowledge collection.

## Further reading

- [Architecture specification](docs/archive/specs/2026-09-04-policy-kernel-extraction-design.md)
- [Trust model](docs/trust-model.md)
- [Promotion and rollback protocol](docs/promotion.md)
- [Project verification and bootstrap design](docs/archive/specs/2026-09-07-project-verification-bootstrap-design.md)
- [Current status](docs/status.md)
- [Task index](docs/tasks/README.md)
- [Repository CI workflows](.github/workflows/)
