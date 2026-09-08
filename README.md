# policy-kernel

Deterministic evidence and release-policy evaluation, independent of any agent harness or model provider.

## Status

Tasks 1–11 are implemented and composed in this repository. Production promotion
remains pending the evidence described below. The React/TypeScript and C#/.NET
profile/bootstrap revision remains proposed for review.

Initial consuming-project targets: React/TypeScript and C#/ASP.NET Core.
The kernel remains Rust and harness-independent; profiles and bootstrap are separate assets.

## Local advisory CLI

`policy-cli` reads a path or `-` for stdin. Local pre-merge evaluation always
uses advisory mode, including when the supplied evaluation input requests
authoritative mode:

```console
cargo run -p policy-cli -- evaluate-pre tests/fixtures/pre-merge/ready.json
cargo run -p policy-cli -- inspect tests/fixtures/evidence/valid.json
cat decision.json | cargo run -p policy-cli -- inspect -
```

`evaluate-pre` exits 0 for ready, 2 for blocked, 3 for inconclusive and 64 for
invalid input. `inspect` accepts typed pre-merge decisions, post-merge decisions
and evidence bundles. It checks bundle integrity and emits the supplied JSON
object without evaluating it. A decision digest cannot be authenticated or
recomputed without its full evaluation input, so `inspect` makes no claim that
a supplied decision is authentic. The CLI has no local post-merge evaluation or
remote mutation command.

Generate the five checked-in contract schemas from the workspace with
`cargo run -p xtask -- schema`. The built `xtask schema` command resolves the
workspace and output directory independently of its caller's working directory;
callers outside the workspace can launch it with Cargo's `--manifest-path`.
The current `sensor-profile` schema describes the existing `SensorProfile`
contract only. It is not the complete project profile execution protocol;
working-directory, tool-version and report-binding requirements remain part of
Task 12.

## Documents

- [Architecture specification](docs/specs/2026-09-04-policy-kernel-extraction-design.md)
- [Implementation plan](docs/plans/2026-09-04-policy-kernel-implementation.md)
- [Design review record](docs/reviews/2026-09-04-policy-kernel-design-antigravity-review.md)
- [Plan review record](docs/reviews/2026-09-04-policy-kernel-plan-antigravity-review.md)

- [Project verification and bootstrap proposal](docs/specs/2026-09-07-project-verification-bootstrap-design.md)
- [Task 1–5 design impact and contract prerequisites](docs/reviews/2026-09-07-task-1-5-design-impact.md)

## Release verification

Repository verification tools and their tests are implemented in Rust:

```console
cargo run --locked -p xtask -- check-residue
cargo run --locked -p xtask -- check-public-api target/doc schemas
cargo run --locked -p xtask -- check-mutation-results verification/mutants.out/outcomes.json
cargo test --locked -p xtask
```

The `xtask rollback-drill` command validates supplied rollback snapshots; see the
promotion protocol below for its arguments and scope.

Task 11 provides adversarial and property tests, complete obligation-fixture
execution, fail-closed residue/mutation checks and CI workflows. See the
[trust model](docs/trust-model.md) and [promotion/rollback protocol](docs/promotion.md).
Promotion requires complete exact-head verification, live
shadow/rollback evidence and two contributor-independent provider reviews. Local
checks alone do not satisfy those requirements.

- [Task 8–11 integration record](docs/reviews/2026-09-08-main-integration.md)
