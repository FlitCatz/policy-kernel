# Promotion and native rollback

Task 11 installs verification gates; it does not promote the kernel. A successful
local test suite, engineering review or fixture drill cannot authorize a protected
merge or replace the recorded native CI baseline.

## Shadow protocol

Use advisory shadow evaluation for at least 14 consecutive days and 30 distinct
PR heads, including a failed native check, a corrected rerun at a new head, a squash
merge and an ordinary merge. If a required class has no observed sample, extend
the window. Record the full input and decision digests, repository, PR/head,
workflow identity, evaluation time, native result and policy version for every
sample. Keep evidence immutable; a rerun at a new head is a new sample.

Compare the native result with the kernel result for the same head. Every mismatch
must have an owner-reviewed explanation, a regression fixture, the stable reason
code and the approving specification section. Frozen legacy fixtures are a local
expectation check, not substitutes for these live samples. No unexplained mismatch
is accepted. These thresholds are proposed release criteria and require the owner
to approve them together with the candidate; no completed shadow window is claimed.

## Candidate evidence and approval

Freeze the exact candidate commit before release-quality checks and reviews.
Record the canonical policy digest produced by `pre_merge_policy_digest` for the
actual chosen policy, plus the sensor-profile digest. Do not use a fixture policy
digest as the deployed policy. A code or policy change invalidates earlier review
and test bindings and requires a new candidate.

The approval packet must contain:

- Candidate full commit SHA, actual policy/profile digests and command logs.
- Passing formatting, locked check/build, strict Clippy, workspace tests, schema
  regeneration with no tracked or untracked schema changes, deny/audit, residue
  check and a build using a fresh empty target directory.
- Complete pinned mutation output for canonical, provenance, reviews, pre_merge,
  post_merge, controller and workflow-source: successful baselines and every
  selected native or replacement mutant caught. Missed, timed-out, unviable,
  unclassified, missing or duplicate outcomes prevent passage. The unfiltered
  generated inventory must reconcile with native selection and every exact
  typed replacement; excluded generator forms never count as caught.
- Task 10 integration tests plus authenticated live required-check and protected
  merge observations for that exact candidate workflow/head.
- Two semantic review reports from providers independent of each other and of
  every code contributor, bound to the frozen candidate. Same-provider subagents
  are engineering reviews only. Missing review, unwaived Critical/High or confirmed
  Medium findings prevent promotion.
- Completed shadow sample inventory, approved parity exceptions and a captured
  native protection baseline with a rollback drill result.

Unavailable tools, failed authentication, missing Task 10 integration, missing
reviews or unperformed live sampling are `INCONCLUSIVE`, never a pass. Missed
mutations and confirmed defects prevent promotion until fixed and rerun. The
promotion workflow gathers mutation evidence; it does not independently satisfy
all of this packet or mutate branch protection.

### Mutation generation and execution contract

CI and local verification use the same Rust entry points (Unix, Git, the pinned
Rust toolchain, OpenSSL and cargo-mutants **27.1.0** required):

```console
cargo fetch --locked
cargo run --locked --offline -p xtask -- run-mutation-suite target/promotion-mutation
cargo run --locked --offline -p xtask -- check-mutation-suite target/promotion-mutation
```

Use a new output directory for every run; existing evidence is never overwritten.
Both commands exit 0 only on success and 1 on invalid/incomplete evidence or
execution failure. `run-mutation-repairs NEW_OUTPUT` probes only the replacements;
its output is explicitly ineligible for `check-mutation-suite`.
`check-mutation-results OUTCOMES [--scope core|controller|workflow-source]` remains
the per-inventory component check, including zero unviable. Passing that component
alone does not establish complete generation or authorize promotion.

The [versioned contract](../verification/mutation-contract.json) identifies twelve
individual ill-typed cargo-mutants expressions, their complete original identities,
historical compiler logs/diffs, source SHA-256 bindings, and explicit replacement
edits. It is executable review material, not a runtime allow-list supplied by the
caller. The verifier embeds it at build time, rejects a changed artifact copy,
and regenerates the unfiltered inventory with `--no-config`. Only exact identities
are filtered from native execution using anchored escaped expressions. All other
generated mutations must appear in both the selected inventory and native results.
Every removed generator expression has one separately executed typed replacement,
with a successful build, a failed test (not an infrastructure failure), and its own
baseline. Unknown omissions, stale source bindings, unverified replacements and
new unviable mutants fail closed. Broad function/file exclusions and blanket
`Default` exclusions are not permitted.

The runner snapshots current files into a temporary directory without modifying
the worktree. It records HEAD, dirty state, tool/platform versions, source hashes,
raw generated/selected inventories, the contract, native logs/diffs/outcomes and
replacement source/build/test logs/outcomes. Rechecking compares every non-Markdown
candidate input, including tests, fixtures, configuration and the verifier itself.
Later prose may record the observation without changing the tested inputs.
Build/test limits are 600/120 seconds. Baselines run for each native scope and each
nonempty replacement scope. Original generator exclusions and caught replacement
counts must be reported separately; no compiler failure is detection credit.

Contract changes require code review of the exact source binding, compiler
diagnostic, semantic replacement and successful full rerun. The current contract
is a proposed repository change until its PR review is accepted; local execution
does not grant owner approval or independent-provider review. A caller cannot
approve additional exclusions through an output file or command-line flag.

Only the human owner authorizes promotion. After reviewing the packet, the owner
may require `policy-kernel/pre-merge` from the intended GitHub App identity in
addition to the native checks. Preserve protected branches, required human review,
stale-review dismissal, code-owner requirements, admin enforcement and restrictions
on force push/deletion. Never enable a check based solely on its display name or
allow a contributor-controlled workflow to forge a trusted check.

## Capture the native baseline

Before any protection change, capture the authenticated current branch protection
and all applicable repository/organization rulesets, check application identities,
human review requirements and the deterministic CI workflow at its immutable
commit. Record the actor and capture time. For classic branch protection the
read-only starting point is:

```console
gh api repos/FlitCatz/policy-kernel/branches/main/protection > native-protection.raw.json
```

Rulesets can add requirements beyond that endpoint and must also be captured and
reviewed. Keep raw observations intact. Normalize the subset used by the fixture
drill into the schema shown in `tests/fixtures/rollback/baseline.json`; preserve
additional raw settings separately and compare them during the live drill.

Task 11's recorded local attempt returned HTTP 401 (`Requires authentication`).
Later observations, including restored user authentication and the private-repository
feature restriction, are in the [current status](status.md) and
[Task 15 hosted record](archive/reviews/2026-09-16-task-15-hosted-verification.md).
No live baseline or protection configuration is claimed. The committed `native-ci`
check and app ID in the fixtures are synthetic examples, not this repository's settings.

## Rollback procedure

1. Pause kernel promotion and preserve all decisions, original reports, immutable
   artifact digests, required-check observations and review evidence.
2. Obtain owner authorization for the exact repository/branch and the previously
   captured native baseline. If that baseline is missing, stop; do not guess which
   checks or rules to remove.
3. In GitHub protection/ruleset settings, remove only the kernel required check and
   restore any changed native settings to the recorded baseline. Retain native
   deterministic CI, human/code-owner reviews and existing protection strength.
4. Read back both classic protection and applicable rulesets. Compare all settings
   and check application identities with the baseline; verify a deliberately
   failing native check still prevents merging in a separately authorized test PR.
5. Confirm that archived evidence content and digests are unchanged. Record actor,
   time, observed configuration and the failing-check test result. Resume the
   native workflow only after the owner verifies the comparison.

Rollback never rewrites old evidence or reactivates `.planning` or GSD lifecycle.
This procedure was not executed against live GitHub in Task 11.

## Reproducible local drill

The drill compares supplied normalized snapshots and artifact inventories. It has
no network client and performs no writes. It rejects missing native checks,
weakened reviews, wrong repository/branch binding, retained kernel checks and
changed evidence inventories. It does not hash archived artifact content or prove
the snapshots came from GitHub; those checks belong to the live procedure above.

```console
cargo test --locked -p xtask --test verification rollback
cargo run --locked -p xtask -- rollback-drill --baseline tests/fixtures/rollback/baseline.json --promoted tests/fixtures/rollback/promoted.json --restored tests/fixtures/rollback/restored.json --evidence-before tests/fixtures/rollback/evidence-before.json --evidence-after tests/fixtures/rollback/evidence-after.json
```

A successful result explicitly reports `scope: supplied-snapshots-only` and
`live_protection_verified: false`. Do not relabel it as a live rollback result.
