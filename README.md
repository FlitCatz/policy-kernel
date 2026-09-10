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

Tasks 1–14 are implemented in this workspace. The kernel, bootstrap assets, and
consumer-acceptance harness have Rust regression coverage. This does not claim a
production promotion: the live shadow window, protected-merge evidence, and
independent-provider reviews in the [promotion protocol](docs/promotion.md)
remain separate requirements.

The project is Rust-first. Repository execution logic, CLIs, verification tools,
report conversion, bootstrap installation, and tests are Rust. JavaScript,
TypeScript, and C# occur only in the approved native consumer fixtures and
declarative tool configuration under `examples/`.

## Requirements

- Rust **1.95.0** with `rustfmt` and `clippy`, as pinned by
  [rust-toolchain.toml](rust-toolchain.toml).
- Git for the CI adapter and consumer-acceptance harness.
- Node 22.18.0 / npm 10.9.3 and .NET SDK 9.0.304 only to run native consumer
  profiles. See [consumer acceptance](docs/integrations/acceptance.md).

## Get started

Clone the repository, then build and test the locked workspace:

```console
cargo build --locked --workspace
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

The workspace uses exact dependency pins and a checked-in `Cargo.lock`. Use
`--locked` for reproducible builds and checks.

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
cargo run --locked -p xtask -- check-mutation-results verification/mutants.out/outcomes.json
cargo test --locked -p xtask
```

`xtask rollback-drill` compares caller-supplied normalized snapshots only. It
does not query or change GitHub. Its arguments and the live rollback procedure
are documented in [promotion and rollback](docs/promotion.md).

## Project verification and bootstrap

`project-verification` turns an adopted project profile and checked source
inventory into local observations. It supports React/TypeScript, .NET, and
full-stack assets and normalizes Vitest JSON, Playwright JSON, and VSTest TRX
reports. The result remains local and advisory until a consumer binds it into the
trusted CI path.

```console
cargo run --locked -p project-verification -- validate-profile profiles/fullstack/profile.json
cargo run --locked -p project-verification -- schema
cargo run --locked -p project-verification -- prepare --input request.json
cargo run --locked -p project-verification -- run --input request.json
```

`policy-bootstrap` installs reviewed, pinned configuration into an existing
consumer directory. Inspect and save a plan before applying it; `apply`
revalidates the plan before it changes files.

```console
cargo run --locked -p policy-bootstrap -- inspect --target /absolute/project --profile fullstack --release 1.0.2 --component frontend=apps/web --component backend=apps/api
cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.0.2 --component frontend=apps/web --component backend=apps/api > /absolute/bootstrap-plan.json
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

## CI integration

The checked-in composite [GitHub Action](action/README.md) invokes an
already-built, trusted `policy-ci` binary on `PATH`. It verifies exact GitHub
workflow and head identity in a runner-controlled environment, then publishes
the `policy-kernel/pre-merge` check. Do not run it from a contributor-controlled
checkout or treat local `policy-cli` output as a required check.

`policy-ci` has `pre-merge` and `post-merge` commands and accepts only
authoritative mode. Its runner-controlled observations and CLI arguments are
documented with the action.

## Further reading

- [Architecture specification](docs/specs/2026-09-04-policy-kernel-extraction-design.md)
- [Trust model](docs/trust-model.md)
- [Promotion and rollback protocol](docs/promotion.md)
- [Project verification and bootstrap design](docs/specs/2026-09-07-project-verification-bootstrap-design.md)
- [Task 14 consumer-acceptance execution record](docs/reviews/2026-09-09-task-14-consuming-acceptance.md)
- [Repository CI workflows](.github/workflows/)
