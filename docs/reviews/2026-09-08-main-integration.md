# Task 8–11 main integration

User authorization: integrate all Task 8–11 implementation branches into local main.
Integration base: main `7eb7c88`. Remote push and live promotion are not part of this operation.

## Branch reconciliation

- Task 8 baseline: `feat/task-8-advisory-cli-schemas` (`22e585d`), already in Task 11 ancestry.
- Alternate Task 8: `feat/task-8-advisory-cli` (`5ddb0db`). Retained the baseline CLI (including evidence integrity validation and duplicate-key rejection), Unicode whitespace schema semantics, and provenance documentation. Integrated the additional regression tests and standard integer bounds in the schema generator. Both schema test suites are retained.
- Task 9: `feat/task-9-bounded-evidence-collector` (`7e295c9`) merged into the Task 11 line. Its collector implementation matches the existing `0f59a66` collector; the latter also inherits Task 8 schema annotations. Both histories are retained without duplicating implementation.
- Task 10: `task10-github-ci` (`dbda015`) merged, including GitHub normalization, trusted CI entry point, Action, integration tests, and historical live-smoke record.
- Task 11: `feat/task-11-release-gates` (`24e3715`) is the integration foundation. Reimplemented the uncommitted scanner hardening described in the handoff: hidden/ignored runtime files, empty Action/schema directories, and missing rustdoc indexes fail closed. Six new regression cases failed before the fix and pass afterward.
- Cargo.lock was reconciled from the full Task 11 dependency set and updated offline for the combined manifests. Duplicate workspace jsonschema declarations were removed. Schemas were regenerated from the combined contracts, retaining the collector truncated status.

## Verification

- `cargo test --workspace --locked --offline`: 193 passed, 1 ignored, 33 suites. The ignored test is the explicitly opt-in live GitHub publication smoke; it is not local integration evidence.
- `python3 -m unittest discover -s scripts/tests -v`: 37 passed.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: passed.
- `cargo fmt --all --check`: passed.
- `bash scripts/check-residue.sh`: passed.
- Clean build with an initially empty target directory: passed.
- Generated rustdoc public API and schema residue scan: passed.
- Schema regeneration against the staged integration result: checked immediately before the integration commit.

## Limits and preserved work

This integrates implementation, not production promotion. cargo-deny, cargo-audit, and cargo-mutants are unavailable on PATH; their current-head results remain INCONCLUSIVE. No completed final historical mutation run was recovered. The inherited CI references cargo-deny but has no checked-in deny.toml; dependency/license policy still needs owner review before release acceptance. No production license was inferred from the unrelated untracked LICENSE file. Live shadow/protected merge evidence and independent-provider release reviews remain outstanding as documented in docs/promotion.md.

Original main local changes (Cargo.lock, CHANGELOG.md, LICENSE, and the historical handoff) were backed up with SHA-256 hashes under `/private/tmp/policy-kernel-main-integration-backup` before integration and are preserved separately from implementation commits. The handoff describes an earlier state; this record supersedes its implementation status.
