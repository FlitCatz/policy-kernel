# Task 15 — Strict mutation policy and executable generator replacements

This follow-up distinguishes the former policy's observations from the new
all-selected-mutants-caught requirement. Investigation and implementation started
on 2026-09-16; execution resumed on 2026-09-21. It does not rewrite the
[hosted record](2026-09-16-task-15-hosted-verification.md).

## Candidate and evidence boundary

Work stayed in `.worktrees/task-15-compatibility`, branch
`feat/task-15-compatibility`. Actual HEAD at entry was
`470dcf61791d8f5acad66726e9c3fcafd023f64a`, clean and one commit ahead of the
remote-tracking branch. Its existing parenthesized input-limit comparison was
already committed, contrary to the earlier handoff's uncommitted description.
The original checkout was clean and was not modified. No new worktree, reset,
discard, force push or merge was used.

The new candidate is that HEAD **plus dirty verification/CI changes**, recorded
in `target/task15-strict/full-02/candidate.json` and `source-manifest.json`.
This is local macOS ARM64 evidence, not a newly published SHA or hosted CI result.
The manifest binds tested source, tests, fixtures, configuration, Cargo.lock and
the verifier. Subsequent Markdown edits record the observation separately.

- `source-manifest.json` SHA-256:
  `90b6af4aebd427e9333870d3e65a95e26660a4467b7328c366c1f9d668042e48`.
- Embedded contract SHA-256:
  `362b3167b99522a48a683e8ec21626f7ca24ebd410b80bf03a17ebf00b9fa8f7`.
- Mutation tool: cargo-mutants 27.1.0; compiler: Rust 1.95.0
  (`59807616e`, 2026-04-14); platform: macOS/aarch64.

The original downloaded artifact remains at
`target/task15-hosted/2026-09-16/751fa76/10430105941.zip`. Its SHA-256 was recomputed
as `98c272182a3c999b259926e1bdf9b3a20740c5c20477ec1b780c15b01e9e5b9e`, matching the
recorded artifact `10430105941` from run `35050598715`, candidate
`751fa760bb0fb85e8c7b739f1ef8f9dc782a1da0`.
The extracted `artifacts/10430105941/` directory preserves core `mutants.out/`,
`controller/mutants.out/`, and `workflow-source/mutants.out/` with their original
inventories, outcomes, compiler/test logs and diffs. The new validator rejects
the old core and controller results with exit 1, `unviable mutants found`.
The old workflow-source component still passes; it is not reused as evidence for
the new candidate.

## Investigation of all thirteen original failures

Every entry below was a build failure of the generated mutant, not a detected
test failure. The successful original baselines and the compiler diagnostics
identify generator/type incompatibilities, rather than an environmental failure
or a defect in the unmutated authorization policy. Exact names, packages,
functions, spans and replacement expressions are preserved in the
[versioned contract](../../../verification/mutation-contract.json), together with
the historical log/diff paths. Source hashes bind the affected files and all
policy-contracts source modules so the reasons cannot silently become stale.

| Original mutant location/function | Compiler evidence | Treatment |
| --- | --- | --- |
| `canonical.rs:16:5`, `canonical_sha256` | E0277, `Sha256Digest: Default` absent | Return an explicitly parsed all-zero digest inside `Ok` |
| `post_merge.rs:8:5`, `evaluate_post_merge` | E0277, `PostMergeDecision: Default` absent | Force the resulting status to `Shipped` |
| `post_merge.rs:304:5`, `post_merge_decision_digest` | E0277, `Sha256Digest: Default` absent | Return an explicitly parsed all-zero digest |
| `pre_merge.rs:11:5`, `evaluate_pre_merge` | E0277, `PreMergeDecision: Default` absent | Force the resulting status to `ReadyToMerge` |
| `provenance.rs:159:9`, `ValidationResult::reasons` | E0277, `Reason: Default` absent | Return one explicit invented reason |
| `provenance.rs:232:5`, `child_digest` | E0277, `Sha256Digest: Default` absent | Return `Some` of an explicitly parsed all-zero digest |
| `provenance.rs:246:5`, `digest` | E0277, `Sha256Digest: Default` absent | Return `Some` of an explicitly parsed all-zero digest |
| `reviews.rs:37:5`, `evaluate_reviews` | E0277, `ReviewEvaluation: Default` absent | Return `Approved` with empty reasons |
| `controller.rs:121:5`, `read_json` | E0277, generic `T: Default` unavailable | Reject even valid JSON with an explicit error |
| `controller.rs:145:5`, `open_input` | E0277, `File: Default` absent | Use ordinary `File::open`, bypassing NONBLOCK/NOFOLLOW protections |
| `provenance.rs:355:5`, `timestamps` | E0271, generated `Once<&mut _>` cannot yield `&UtcTimestamp` | Iterate only the immutable manifest timestamp, omitting child timestamps |
| `controller.rs:137:27`, `read_file`, `>` → `<` | Cast followed by `<` parsed as generic arguments | Retain HEAD's `(bytes.len() as u64)` parentheses; run the generated `<` mutant normally |
| `controller.rs:222:35`, `bounded_crypto`, `+` → `*` | E0369, no `Instant * Duration` | Set deadline to `Instant::now() + Duration::ZERO` |

Only `read_file` changes product source, and that change was already in HEAD.
All twelve other edits exist only in isolated mutant copies. No `Default`
implementation, generic bound, product policy, trust check or input check was
weakened to accommodate the generator. These replacements exercise specific
wrong behavior, rather than replacing every function with a panic.

Representative failed assertions from the new replacement test logs are listed
below. Each corresponding Build log succeeded; these are test detections, not
compiler diagnostics. Log names are relative to the scope's `replacements/`.

| Scope / log | Replacement | Representative detecting test |
| --- | --- | --- |
| core / `repair-0-Test.log` | `canonical_sha256` | `merge_and_squash_commit_oids_may_differ_from_the_evaluated_pr_head` |
| core / `repair-1-Test.log` | `evaluate_post_merge` | `synthetic_or_misbinding_required_checks_cannot_authorize_shipped` |
| core / `repair-2-Test.log` | `post_merge_decision_digest` | `decision_digest_binds_every_input_and_the_result_deterministically` |
| core / `repair-3-Test.log` | `evaluate_pre_merge` | `altered_evidence_and_replayed_agent_ready_fail_closed` |
| core / `repair-4-Test.log` | `ValidationResult::reasons` | `altered_evidence_and_replayed_agent_ready_fail_closed` |
| core / `repair-5-Test.log` | `child_digest` | `required_knowledge_accepts_ci_and_attested_bindings` |
| core / `repair-6-Test.log` | `digest` | `required_knowledge_accepts_ci_and_attested_bindings` |
| core / `repair-7-Test.log` | `timestamps` | `waiver_issuance_must_be_fresh` |
| core / `repair-8-Test.log` | `evaluate_reviews` | `missing_or_non_independent_review_evidence_cannot_authorize` |
| controller / `repair-0-Test.log` | `read_json` | `input_bound_rejects_oversized_valid_json_and_keeps_boundary_content` |
| controller / `repair-1-Test.log` | `open_input` | `named_pipe_input_is_rejected_without_waiting_for_a_writer` |
| controller / `repair-2-Test.log` | `bounded_crypto` | `private_signature_scratch_is_removed_and_invalid_encodings_never_spawn_crypto` |

The ordinary `read_file` `<` mutant is now identified at `controller.rs:137:29`.
In the complete rerun its Build phase succeeded and its Test phase exited 101,
classified `CaughtMutant`. Its native log/diff stem is
`crates__policy-ci__src__controller.rs_line_137_col_29_001` under
`controller/mutants.out/log/` and `diff/`. It is not a contract exclusion or a
separately counted replacement.

## Selection and judgment changes

`xtask check-mutation-results` now rejects any nonzero unviable counter while
retaining exact inventory, counters, scope, baseline, duplicate, unknown-outcome,
missed and timeout checks. A regression fixture that previously passed with one
unviable result was observed failing before the fix and passing afterwards;
the complete positive fixture remains accepted.

`run-mutation-suite` snapshots the candidate and generates all three inventories
with cargo-mutants 27.1.0 and `--no-config`. Twelve exact ill-typed generator forms
are removed from native execution and mapped one-to-one to typed replacements.
The generated forms themselves receive **no caught credit**. There is no waived
verification obligation: every native selection and every replacement must build
and be caught after a successful baseline. New invalid forms fail closed.

The contract is embedded in the verifier, not loaded as an arbitrary caller
allow-list. Native selection uses anchored, escaped complete names. The aggregate
checker regenerates the full inventory, checks its exact partition, binds the
replacement inventory/source/logs, and rejects source drift, missing/duplicate
entries, unverified test phases and changed logs. Source and test/configuration
inputs are bound to the candidate manifest. A repairs-only probe is never eligible
as full-suite evidence. Both CI and local paths invoke this same Rust runner.

An engineering code review inspected selection and process isolation. Its snapshot
race and worker-cleanup findings were corrected: copied bytes equal hashed bytes,
and graceful interruption gives cargo-mutants time to clean separate worker
groups. A Rust regression exercises a supervisor with a separate worker group.
This review is not an independent-provider promotion review. The contract and CI
changes remain uncommitted PR-review material; no owner promotion approval is
inferred from this local execution.

## Execution observations

The 2026-09-16 targeted probe in `target/task15-strict/repairs-02/` observed all
9 core and 3 controller replacements built and caught, with successful baselines.
`full-01/` later recorded 301/301 caught native core mutants but did not produce
the runner's completion record or the other two scopes. It is incomplete and is
not accepted by `check-mutation-suite`.

The complete rerun uses:

```console
rtk cargo run --locked --offline -p xtask -- run-mutation-suite target/task15-strict/full-02
rtk cargo run --locked --offline -p xtask -- check-mutation-suite target/task15-strict/full-02
```

The complete local rerun and a separate `check-mutation-suite` invocation both
exited **0**. It ran on 2026-09-21 from 01:10:32 to 01:23:23 UTC. The measured
partition and outcomes are:

| Scope | Generated | Excluded ill-typed generator forms | Native caught | Replacement caught | Unviable / missed / timeout | Successful baselines |
| --- | ---: | ---: | ---: | ---: | --- | ---: |
| core | 310 | 9 | 301 | 9 | 0 / 0 / 0 | 2 |
| controller | 143 | 3 | 140 | 3 | 0 / 0 / 0 | 2 |
| workflow-source | 8 | 0 | 8 | 0 | 0 / 0 / 0 | 1 |
| Total | **461** | **12** | **449** | **12** | **0 / 0 / 0** | **5** |

Thus 461 executable mutations were caught: 449 unchanged generator selections
plus 12 explicitly typed replacements. The twelve excluded generator expressions
are not reported as caught; each has a separately verified replacement. Unmapped,
missing, duplicate and unclassified entries were zero under the aggregate gate.
There are no coverage waivers. The complete generated/selected/result inventories
were compared against fresh generation from the same candidate inputs.

Raw output is under `target/task15-strict/full-02/`; the parent console log is
`target/task15-strict/full-02.log`. Each scope contains `generated.json`,
`selected.json`, `repairs.json`, and native `mutants.out/`. Core/controller also
contain `replacements/` with explicit mutated source and separate build/test logs.
The completed output and parent log are preserved in
`target/task15-strict/full-02.tar.gz`, SHA-256
`fbacb604f59eb4a5ed730fa7faec56be3002896a0b2c0d29722c0b53ac0e977b`.
This archive is local and has not been uploaded as a GitHub Actions artifact.

Rust 1.95 and 1.85 workspace tests each passed **349 / 3 ignored**; strict workspace
Clippy, formatting, runtime residue and `git diff --check` passed. The local Rust
Markdown scanner checked 272 relative links with no missing targets. Rust 1.85
used explicit matching cargo/rustc/rustdoc binaries under
`/private/tmp/task15-rustup-home/toolchains/1.85.0-aarch64-apple-darwin/bin` and
`CARGO_TARGET_DIR=target/msrv-1.85`; its log is
`target/task15-strict/rust-1.85-workspace.log`. The ignored tests remain
`native_jest`, `native_nuget_audit`, and
`live_github`. This change does not relabel them as passed. Their earlier native
execution observations and the unperformed Task 15 live GitHub smoke remain
separate.

## Remaining external conditions

No new hosted mutation success, actual GitHub Check publication/query, protected
merge or operational promotion is established here. The owner still needs the
production attesters and complete ledger, isolated publication deployment and
credentials, adopted candidate/policy/profile, authenticated exact-head checks,
independent-provider reviews, shadow samples, protection baseline and live rollback
evidence. See [current status](../../status.md) and the
[promotion protocol](../../promotion.md).
