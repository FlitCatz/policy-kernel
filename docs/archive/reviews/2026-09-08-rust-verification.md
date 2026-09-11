# Rust verification tools

This change supersedes the script implementation described in the earlier main
integration record. The owner requires repository logic and tests to use Rust.

- Moved mutation-result validation, rollback snapshot validation, runtime residue
  scanning, and public API scanning into `xtask/src/verification.rs`.
- Ported adversarial cases to Rust integration tests: incomplete/duplicate mutation
  inventories, unsafe rollback changes, malformed snapshots, missing artifacts,
  hidden/ignored residues, dependency failures, and duplicate JSON fields.
- Removed the Python tools/tests and standalone Shell scanners. CI invokes Rust
  tools directly; workspace tests include their validation tests.
- Replaced the CI test's interpreter fixture and the adapter/Action test's Shell
  stubs with Rust process fixtures. The Action test still executes the actual
  minimal YAML Shell wrapper to verify its literal argument forwarding.
- Added a Rust regression that prevents interpreter scripts or interpreter calls
  from reappearing in crates, verification scripts, or CI workflows.

The initial Rust command tests failed against the missing commands, then passed
after implementation. The interpreter-dependency regression failed on the old
script before removal.

Verification: the workspace suite passed 204 tests with one explicitly opt-in
live GitHub test ignored before the final duplicate-field regression was added.
The final focused verification suite passes 10 Rust integration tests, with
multiple adversarial cases per test. Strict workspace Clippy and formatting pass.
The Rust runtime scanner, public API scanner, and supplied-snapshot rollback
drill pass. Schema generation remains unchanged. The final main verification is
reported with the merge result.

The implementation-language policy is recorded verbatim in `AGENTS.md`.

This changes the implementation language, not the live promotion prerequisites.
No external GitHub write or live mutation report is claimed.
