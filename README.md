# policy-kernel

Deterministic evidence and release-policy evaluation, independent of any agent harness or model provider.

## Status

Core implementation is in progress. The original design and plan were reviewed;
the React/TypeScript and C#/.NET profile/bootstrap revision is proposed for review.

Initial consuming-project targets: React/TypeScript and C#/ASP.NET Core.
The kernel remains Rust and harness-independent; profiles and bootstrap are separate assets.

## Local CLI

Evaluate a pre-merge input locally:

```bash
cargo run -p policy-cli -- evaluate-pre tests/fixtures/pre-merge/ready.json
```

Local evaluation always uses advisory mode, even when the input requests
authoritative evaluation. It cannot authorize shipping. The command emits one
JSON decision to stdout and diagnostics to stderr. Exit codes are `0` for ready,
`2` for blocked, `3` for inconclusive, and `64` for invalid input.

Use `inspect <path>` to validate and display an existing pre-merge decision,
post-merge decision, or evidence bundle. Inspection preserves the supplied data;
it does not evaluate it or authenticate its provenance. Both commands accept `-`
instead of a path to read stdin. Local post-merge evaluation is not exposed.

Generate the checked-in contract schemas with:

```bash
cargo run -p xtask -- schema
```

## Documents

- [Architecture specification](docs/specs/2026-09-04-policy-kernel-extraction-design.md)
- [Implementation plan](docs/plans/2026-09-04-policy-kernel-implementation.md)
- [Design review record](docs/reviews/2026-09-04-policy-kernel-design-antigravity-review.md)
- [Plan review record](docs/reviews/2026-09-04-policy-kernel-plan-antigravity-review.md)

- [Project verification and bootstrap proposal](docs/specs/2026-09-07-project-verification-bootstrap-design.md)
- [Task 1–5 design impact and contract prerequisites](docs/reviews/2026-09-07-task-1-5-design-impact.md)
