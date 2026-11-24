# Provenance

## Frozen legacy source

- Repository: `FlitCatz/agent-dev-kit`
- Commit: `e5f6ec8584cc91fdeca3aab40687c6cdcd303b43`
- Specification: `docs/archive/specs/2026-09-04-policy-kernel-extraction-design.md`
- Extraction mode: contract-first selective reimplementation
- Copied production code: none
- Distribution license: dual-licensed under MIT or Apache-2.0.

The legacy repository is evidence, not a source-tree template. The Rust files below
are behavioral sources only. Their filesystem access, subprocess execution,
configuration fallback, ambient clocks, provider bindings, and GSD lifecycle state
are explicitly rejected as implementation coupling.

| Legacy path | Git blob SHA | SHA-256 content digest | Classification |
|---|---|---|---|
| `tools/src/consensus.rs` | `9e253adc9bdd5028db7f03baf1428f9b14a27d0d` | `e53babb3131369e7952bf03d3ee018914e689c00838fd4d271d9c9d06335d361` | Behavioral source; do not copy |
| `tools/src/sensor_resolver.rs` | `aa16445b13001289b5c4753759c5bb8fb55283c8` | `a5e0b718bfe5eea294826e65ddc7ab1cd944ea88c0083450cd1e0de3b2fdf2f4` | Behavioral source; do not copy |
| `tools/src/git_provider.rs` | `3a5837521b966240ef778db29bc45072c5cab720` | `c96936675075dbe3cfa81fcb390b8759d2a06af4f0a890158ec7090779a1d93f` | Behavioral source; do not copy |
| `tools/src/lifecycle.rs` | `ba1c92358f08fc66265a935330d632570c92e17b` | `8aa6160467daaef1c014ccdade223472af8466a13079526e72c40ba9fe5e3a3b` | Behavioral source; do not copy |
| `tools/tests/integration_tests.rs` | `bb40dbdacdba4b09f5fcc92df2f360ae0b459499` | `04f1dffd4e8b45f0d2fd92a6d73c262687f62266c22b7841798c55f31202f1de` | Adversarial behavior source |
| `AGENTS_GUIDE.md` | `ac3282911ebf292e3c2d318222245d09c4e93560` | `cd70fc7b78b70f4738571f57ceccc0887daf9f2553dea8b2d26ea6822edb89f8` | Normative documentation source |
| `docs/ARCHITECTURE.md` | `babaf7ff47c0475da735170cdc6ea69d72382685` | `645623f55f650dae6e9beb55c1f7b2d09843efef7898ad9ff56d61acfb3d9c92` | Architecture documentation source |

## Rust dependency pins

The workspace declares only the approved technology stack. The version
requirements and feature selections live in `[workspace.dependencies]` of the
root `Cargo.toml`; `Cargo.lock` records the exact tested resolution, which must
build on the workspace minimum compiler (Rust 1.85).

`jsonschema` is used only by Task 8 tests to execute Draft 2020-12 instance
validation against the generated files. Its HTTP and file resolution features are
disabled; the tests resolve only each schema's in-document definitions and run
offline with timestamp format validation enabled.

### Timestamp library

`jiff` replaced `chrono` on 2026-11-18 after RustSec marked every `chrono`
release unmaintained (RUSTSEC-2026-0543, no patched version). Only
`policy-contracts` depends on it, and no Jiff type appears in a public API:
`UtcTimestamp` stores a UTC civil date-time and offers the clock, comparison,
duration and arithmetic operations the other crates need. The minimum compiler
stays Rust 1.85, and schemars no longer enables its `chrono04` feature.

Jiff's own parser and printer are not used for wire data. `UtcTimestamp` keeps the
previous RFC 3339 grammar and canonical output exactly: a four-digit year, `T`,
`t` or a space before the time, `Z`, `z` or a numeric offset of at most 23:59
(U+2212 is accepted as the minus sign), any number of fractional digits truncated
to nanoseconds, and a `Z`-suffixed result with 0, 3, 6 or 9 fractional digits.
Timestamps from `0000-01-01T00:00:00Z` through `9999-12-31T23:59:59.999999999Z`
serialize to the same bytes as before, so existing digests and signed bytes are
unchanged.

Three behaviors changed on purpose, all for input no real producer emits:

- A leap second (seconds field `60`) is rejected. It was formerly kept and
  serialized as `:60`; Jiff cannot represent it, and rewriting it to `:59` would
  silently change signed bytes.
- An offset that moves the UTC year outside 0000-9999 is rejected. It was formerly
  serialized with a signed year such as `-0001` or `+10000`, a form the parser
  never accepted back.
- A Vitest `startTime` outside years 0000-9999 is still rejected, now always as
  `invalid startTime` instead of sometimes `invalid timestamp`.

A local differential run against `chrono` 0.4.42 compared 2,000,065 parse inputs,
2,999,965 ordered pairs (ordering, durations and every freshness predicate) and
2,000,000 clock and Unix-millisecond conversions. It found no difference outside
the first two classes. The pinned expectations live in
`crates/policy-contracts/tests/utc_timestamp.rs`.

## Verification status

GitHub Actions run `33868276500` validated commit
`ae56817e05376e352ff3a56fcc3cb28ff611b8c1` with Rust 1.95.0. The locked workspace
check, frozen source-inventory test, and exact seven-member workspace metadata check
all completed successfully. Dependency pins not yet referenced by a crate will enter
the lockfile when their corresponding implementation task introduces them.

## Task 11 release verification tools

The original plan requested a mutation-tool pin but had not recorded a version.
Task 11 selects these exact tool versions; they are verification tools and are
not policy-core dependencies.

| Tool | Exact version | Primary release source |
| --- | --- | --- |
| cargo-mutants | 27.1.0 | https://github.com/sourcefrog/cargo-mutants/releases/tag/v27.1.0 |
| cargo-deny | 0.20.2 | https://github.com/EmbarkStudios/cargo-deny/blob/main/CHANGELOG.md |
| cargo-audit | 0.22.2 | https://github.com/rustsec/rustsec/releases |

Use `cargo install --locked --version <exact-version> <tool>`; record the
installed version in each release log. Do not treat an absent tool or incomplete
run as success. Dependency advisories remain tied to the actual fetched advisory
database state and must be recorded per candidate.

Task 8 and Task 9 were composed on the Task 11 branch. Task 9's `truncated`
execution status required regenerating `evidence-bundle.schema.json`; the existing
schema drift test detected this integration mismatch before Task 11 changes.

Task 11 workflow pins were verified using read-only `git ls-remote` against the
official repositories:

| Action | Version | Commit |
| --- | --- | --- |
| actions/checkout | v4.2.2 | 11bd71901bbe5b1630ceea73d27597364c9af683 |
| actions/upload-artifact | v4.6.2 | ea165f8d65b6e75b540449e92b4886f43607fa02 |

Workflow syntax/expressions were also checked locally using actionlint 1.7.7.
Every workspace crate inherits the dual MIT or Apache-2.0 license declaration.
`deny.toml` checks external dependency licenses and permits only private path
dependencies to omit versions; registry wildcard dependencies remain prohibited.
Duplicate transitive syn major versions are reported as a warning, not silently
removed from the graph.
