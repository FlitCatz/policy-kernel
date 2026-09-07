# Policy Kernel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Status:** Tasks 1–11 retain the original plan; the integration amendment below is a proposal not covered by the original Antigravity review.

**Goal:** Create a fresh, deterministic Rust `policy-kernel` repository that evaluates CI evidence and protected GitHub merges without carrying GSD Core or agent-harness runtime code.

**Architecture:** Build contract-first in a new sibling repository. Pure `policy-contracts` and `policy-core` crates own schemas and decisions; process, filesystem, GitHub, and CI concerns remain in `policy-cli`, `evidence-collector`, `github-adapter`, and `policy-ci`. Migration copies obligations and adversarial fixtures first, then selectively reimplements behavior without copying legacy modules.

**Tech Stack:** Rust 2021 workspace; `serde`, `serde_json`, `serde_yaml`, `chrono` with explicit injected timestamps, `sha2`, `hex`, `thiserror`, `clap`, `schemars`, `globset`; `assert_cmd`, `test-case`, and `proptest` for tests; pinned `cargo-mutants` for release mutation analysis; GitHub Actions and `gh` only outside `policy-core`.

**Spec:** [Kernel design](../specs/2026-09-04-policy-kernel-extraction-design.md) and [project verification/bootstrap design](../specs/2026-09-07-project-verification-bootstrap-design.md)

## Global Constraints

- Continue in the existing `FlitCatz/policy-kernel` repository on an isolated change branch; the original local repository creation step is historical. Do not publish packages.
- Pin `rustc` and Cargo to `1.95.0`; install and require the matching `rustfmt` and `clippy` components.
- Freeze legacy inputs at source commit `e5f6ec8584cc91fdeca3aab40687c6cdcd303b43` and record every used blob SHA and SHA-256 digest.
- The new repository has no GSD Core, `.planning`, agent runtime, prompt, model-routing, session, repair-loop, Docker, MCP, TUI, LSP, browser, Hashline, LAT, or CodeGraph production dependency.
- `policy-core` may not use `std::fs`, `std::process`, environment variables, ambient clocks, Git, GitHub, network clients, or provider-specific names.
- Local evaluation is advisory. Only CI-observed or accepted trusted-runner evidence can produce an authoritative pre-merge decision.
- Provider independence is checked against every code-modifying execution in the evaluated change set.
- `SHIPPED` requires a successful authoritative pre-merge decision, a matching trusted required-check observation, and a protected merge observation for the same PR head.
- `gemini-reviewer` remains an external project/skill-pack instruction. OMP and `agy` remain replaceable workflow executors and must not appear in kernel schemas or dependencies.
- Unknown schema versions, unknown required enum values, missing evidence, stale evidence, timeout, truncation, authentication failure, and unparsable results fail closed.
- Every task uses TDD, records the exact verification output, and ends in a small commit on an isolated branch/worktree.

## Target file map

```text
policy-kernel/
  Cargo.toml                         workspace members and pinned dependency policy
  Cargo.lock                        exact resolved dependency versions
  rust-toolchain.toml               pinned Rust channel and components
  deny.toml                         license/source/advisory policy
  .github/workflows/ci.yml          build, tests, clippy, audit, residue scan
  action/action.yml                 reusable authoritative gate action
  crates/policy-contracts/src/
    lib.rs                          public re-exports only
    identity.rs                     repository, SHA, digest, provider/execution IDs
    policy.rs                       PolicySnapshot and required gate policy
    evidence.rs                     evidence, review, waiver, sensor, adequacy types
    github.rs                       issue, PR, required-check, merge observations
    decision.rs                     modes, decisions, stable reason codes
  crates/policy-core/src/
    lib.rs                          evaluator public API only
    canonical.rs                    canonical JSON and bundle digest verification
    provenance.rs                   trust, freshness, contributor independence
    sensors.rs                      applicability and evidence completeness
    reviews.rs                      consensus, finding severity, waiver validation
    pre_merge.rs                    authoritative/advisory pre-merge evaluation
    post_merge.rs                   required-check and protected-merge evaluation
  crates/policy-core/tests/
    source_inventory.rs             frozen source and obligation coverage
    adversarial_release_gate.rs     cross-crate authorization regressions
    legacy_cases.rs                 frozen expectation matrix runner
  crates/policy-cli/src/main.rs      advisory evaluate and inspect commands
  crates/evidence-collector/src/
    lib.rs                          bounded explicit-argv execution contract
    main.rs                         collector CLI
  crates/github-adapter/src/
    lib.rs                          normalized provider-facing API
    parse.rs                        GitHub JSON normalization
    gh.rs                           bounded `gh api` invocation
  crates/policy-ci/src/main.rs       authoritative bundle assembly and check output
  xtask/src/main.rs                  deterministic JSON Schema generation
  schemas/                           generated JSON Schemas checked into source
  tests/fixtures/                    versioned valid and invalid JSON/YAML inputs
  docs/provenance.md                 exact origin and selective-reuse record
  docs/obligation-matrix.yaml        source rule to test/reason-code mapping
  docs/trust-model.md                authoritative trust boundary
  docs/integrations/agent-skills.md external skill/executor integration boundary
  docs/promotion.md                  shadow, promotion, native rollback procedure
```

---

### Task 1: Fresh repository, provenance, and obligation baseline

**Files:**
- Create: all workspace manifests listed below
- Create: `docs/provenance.md`
- Create: `docs/obligation-matrix.yaml`
- Create: `crates/policy-core/tests/source_inventory.rs`

**Interfaces:**
- Consumes: frozen source commit and the approved specification.
- Produces: buildable empty workspace and `Obligation { id, source, expected, reason_code, fixture }` inventory used by every later task.

- [ ] **Step 1: Create an isolated sibling repository and branch**

Run:

```bash
mkdir /Users/dev-r/Repository/playground/policy-kernel
cd /Users/dev-r/Repository/playground/policy-kernel
git init
git switch -c feat/initial-policy-kernel
```

Expected: an empty repository on `feat/initial-policy-kernel`; no remote configured.

- [ ] **Step 2: Write the workspace manifests and empty crate entry points**

Create `Cargo.toml` with members for the six production crates plus `xtask` and workspace package values `edition = "2021"` and `publish = false`. Do not declare a distribution license until the human owner selects one. Add only the dependencies listed in the Tech Stack, generate `Cargo.lock`, and record the resolved versions in `docs/provenance.md`.

Each library entry point contains only `#![forbid(unsafe_code)]`; each binary `main` returns `Result<(), Box<dyn std::error::Error>>` and prints no success claim.

- [ ] **Step 3: Write the source inventory test first**

```rust
#[test]
fn frozen_source_and_required_obligations_are_recorded() {
    let matrix_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/obligation-matrix.yaml");
    let matrix = std::fs::read_to_string(matrix_path).unwrap();
    assert!(matrix.contains("e5f6ec8584cc91fdeca3aab40687c6cdcd303b43"));
    for id in [
        "PK-PROV-001", "PK-SELF-001", "PK-SEV-001", "PK-WAIVE-001",
        "PK-SENSOR-001", "PK-ADEQ-001", "PK-PRE-001", "PK-POST-001",
    ] {
        assert!(matrix.contains(id), "missing obligation {id}");
    }
}
```

- [ ] **Step 4: Run the inventory test and observe failure**

Run: `cargo test -p policy-core --test source_inventory`

Expected: FAIL because `docs/obligation-matrix.yaml` does not yet contain the frozen source and obligation IDs.

- [ ] **Step 5: Populate provenance and the obligation matrix**

For each obligation, inspect the source repository with `git -C /Users/dev-r/Repository/playground/agent-dev-kit rev-parse e5f6ec8584cc91fdeca3aab40687c6cdcd303b43:<path>`. Record the source commit, exact legacy path, returned blob SHA, SHA-256 content digest, preserved semantic rule, rejected implementation coupling, fixture path, and expected stable reason code. Never run the source-object lookup against the newly initialized repository. The initial source set is:

```text
tools/src/consensus.rs
tools/src/sensor_resolver.rs
tools/src/git_provider.rs
tools/src/lifecycle.rs
tools/tests/integration_tests.rs
AGENTS_GUIDE.md
docs/ARCHITECTURE.md
```

Mark `consensus.rs`, `sensor_resolver.rs`, `git_provider.rs`, and `lifecycle.rs` as behavioral sources, not files approved for copying. Explicitly reject their filesystem, process, config fallback, ambient clock, and GSD lifecycle code.

- [ ] **Step 6: Verify and commit**

Run: `cargo test -p policy-core --test source_inventory && cargo metadata --no-deps --format-version 1`

Expected: test PASS and workspace metadata lists exactly seven members: six production crates and `xtask`.

```bash
git add Cargo.toml Cargo.lock rust-toolchain.toml crates docs
git commit -m "chore: establish policy kernel provenance baseline"
```

---

### Task 2: Versioned contracts and strict deserialization

**Files:**
- Create: `crates/policy-contracts/src/{lib,identity,policy,evidence,github,decision}.rs`
- Create: `crates/policy-contracts/tests/contracts.rs`
- Create: `tests/fixtures/contracts/{valid-pre-merge,advisory-decision,unknown-version,invalid-sha}.json`

**Interfaces:**
- Consumes: obligation IDs from Task 1.
- Produces: `PreMergeEvaluationInput`, `PostMergeEvaluationInput`, `PreMergeDecision`, and `PostMergeDecision` used by all other crates.

- [ ] **Step 1: Write strict contract tests**

```rust
#[test]
fn unknown_schema_version_and_short_sha_are_rejected() {
    assert!(serde_json::from_str::<PreMergeEvaluationInput>(
        include_str!("../../../tests/fixtures/contracts/unknown-version.json")
    ).is_err());
    assert!(CommitSha::parse("deadbeef").is_err());
}

#[test]
fn advisory_decision_cannot_be_post_merge_authorization() {
let decision: PreMergeDecision = serde_json::from_str(
    include_str!("../../../tests/fixtures/contracts/advisory-decision.json")
).unwrap();
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_err());
}
```

- [ ] **Step 2: Run the tests and observe missing-type failures**

Run: `cargo test -p policy-contracts`

Expected: FAIL because the contract types are not defined.

- [ ] **Step 3: Implement exact identity and decision types**

Use validated newtypes for `RepositoryId`, `WorkItemId`, `PullRequestNumber`, `CommitSha`, `Sha256Digest`, `ProviderId`, `ModelId`, `ExecutionId`, `WorkflowRunId`, `SensorId`, and `UtcTimestamp`. Define `Risk`, `TrustClass`, `ExecutionStatus`, `ReviewStatus`, `FindingSeverity`, `CheckConclusion`, and `MergeMethod` as closed enums. Define:

```rust
pub enum EvaluationMode { Advisory, Authoritative }
pub enum PreMergeStatus { Blocked, Inconclusive, ReadyToMerge }
pub enum PostMergeStatus { Blocked, Inconclusive, Shipped }
pub struct PreMergeDecision {
    pub schema_version: u32,
    pub mode: EvaluationMode,
    pub advisory: bool,
    pub status: PreMergeStatus,
    pub binding: DecisionBinding,
    pub reasons: Vec<Reason>,
    pub decision_digest: Sha256Digest,
}
pub struct PostMergeDecision {
    pub schema_version: u32,
    pub status: PostMergeStatus,
    pub binding: DecisionBinding,
    pub reasons: Vec<Reason>,
    pub decision_digest: Sha256Digest,
}
```

All wire structs use `#[serde(deny_unknown_fields)]`. `AuthoritativePreMergeDecision` is a validated wrapper constructible only when `mode == Authoritative`, `advisory == false`, and status is `ReadyToMerge`.

- [ ] **Step 4: Implement evidence and GitHub observation contracts**

Define `ModificationEvidence`, `ExecutionEvidence`, `ReviewEvidence`, `Finding`, `WaiverEvidence`, `SensorEvidence`, `TestAdequacyEvidence`, optional `KnowledgeEvidence`, `WorkItemObservation`, `GitHubPullRequestObservation`, `GitHubCheckObservation`, `GitHubMergeObservation`, `EvidenceManifest`, and `EvidenceBundle`. `KnowledgeEvidence` distinguishes LAT claims from CodeGraph indexes and binds source/version/configuration/content/source-set/head digests without treating either as approval. The post-merge input contains an `AuthoritativePreMergeDecision`, not a raw decision.

- [ ] **Step 5: Verify serialization stability and commit**

Run: `cargo test -p policy-contracts`

Expected: all contract tests PASS and the valid fixture round-trips byte-for-byte after canonical field ordering.

```bash
git add crates/policy-contracts tests/fixtures/contracts
git commit -m "feat: define strict policy and evidence contracts"
```

---

### Task 3: Canonical bundle integrity and trust validation

**Files:**
- Create: `crates/policy-core/src/{lib,canonical,provenance}.rs`
- Create: `crates/policy-core/tests/provenance.rs`
- Create: `tests/fixtures/evidence/{valid,tampered,local-authoritative,stale}.json`

**Interfaces:**
- Consumes: Task 2 contracts.
- Produces: `canonical_sha256<T: Serialize>()`, `validate_bundle()`, and `validate_provenance()`.

- [ ] **Step 1: Write failing tamper, freshness, and trust tests**

```rust
#[test]
fn tampered_child_digest_is_inconclusive() {
    let result = validate_bundle(&fixture("tampered.json"));
    assert_eq!(result.reason_codes(), ["PK-PROV-DIGEST-MISMATCH"]);
}

#[test]
fn local_evidence_cannot_satisfy_authoritative_mode() {
    let result = validate_provenance(
        EvaluationMode::Authoritative,
        &fixture("local-authoritative.json"),
        fixed_evaluation_time(),
    );
    assert_eq!(result.reason_codes(), ["PK-PROV-UNTRUSTED"]);
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p policy-core --test provenance`

Expected: FAIL because canonical and provenance validators are missing.

- [ ] **Step 3: Implement deterministic integrity validation**

Canonicalize JSON by recursively sorting object keys, preserving array order, rejecting floating-point numbers, and hashing UTF-8 bytes with SHA-256. Verify every manifest child digest, repository/base/head binding, policy digest, source-set digest, workflow/run identity, optional required knowledge-evidence binding, and explicit evaluation time. A required CodeGraph record is valid only when CI-built at the evaluated head or trusted-runner-attested; a database digest alone is insufficient. Return stable `Reason` values rather than prose-only errors.

- [ ] **Step 4: Implement contributor-wide independence**

```rust
pub fn verifier_is_independent(
    verifier: &ReviewEvidence,
    modifiers: &[ModificationEvidence],
) -> bool {
    modifiers.iter().all(|modifier| {
        modifier.provider != verifier.provider
            && modifier.execution_id != verifier.execution_id
    })
}
```

Also reject duplicate reviewer providers, duplicate execution IDs, missing resolved provider/model, and evidence outside the policy freshness window.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p policy-core --test provenance`

Expected: valid evidence passes; tampered, stale, local-authoritative, and any-contributor self-review fixtures return their exact reason codes.

```bash
git add crates/policy-core tests/fixtures/evidence
git commit -m "feat: validate bundle integrity and provenance"
```

---

### Task 4: Sensor applicability and test adequacy

**Files:**
- Create: `crates/policy-core/src/sensors.rs`
- Create: `crates/policy-core/tests/sensors.rs`
- Create: `tests/fixtures/sensors/{profile,missing,timeout,not-applicable,no-tests}.json`

**Interfaces:**
- Consumes: `SensorProfile`, `SensorEvidence`, `TestAdequacyEvidence`.
- Produces: `resolve_required_sensors()` and `evaluate_sensor_evidence()`.

- [ ] **Step 1: Write failing selection and no-op tests**

```rust
#[test]
fn required_when_requires_all_declared_dimensions() {
    let ids = resolve_required_sensors(&profile(), &["src/api.rs"], Risk::High);
    assert_eq!(ids, [SensorId::from("compile"), SensorId::from("api-e2e")]);
}

#[test]
fn zero_exit_with_zero_discovered_tests_fails_adequacy() {
    let result = evaluate_sensor_evidence(&profile(), &no_tests_bundle());
    assert!(result.has_reason("PK-ADEQ-NO-TESTS"));
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p policy-core --test sensors`

Expected: FAIL because sensor resolution is missing.

- [ ] **Step 3: Implement pure applicability and completeness**

Normalize paths to forward slashes; reject `..`; use deterministic glob matching. Required sensors are the union of unconditional sensors and conditions whose declared risk and changed-path predicates both match. `NOT_APPLICABLE` succeeds only with a recorded matching condition and reason. Missing, skipped, timed-out, and unparsable required results return `INCONCLUSIVE`; executed non-accepted exit codes return `BLOCKED`.

- [ ] **Step 4: Implement separate adequacy evaluation**

Require every acceptance-criterion ID and changed-behavior ID to map to at least one executed test ID, require declared negative-case IDs, and require discovered/executed/passed counts to agree. Never infer adequacy from sensor exit status.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p policy-core --test sensors`

Expected: all selection, timeout, applicability, parser, empty-discovery, and adequacy tests PASS.

```bash
git add crates/policy-core tests/fixtures/sensors
git commit -m "feat: evaluate sensors and test adequacy"
```

---

### Task 5: Review consensus and human waivers

**Files:**
- Create: `crates/policy-core/src/reviews.rs`
- Create: `crates/policy-core/tests/reviews.rs`
- Create: `tests/fixtures/reviews/{approved,self-review,critical,medium-confirmed,medium-conflict,waiver-expired}.json`

**Interfaces:**
- Consumes: review, modification, finding, waiver, policy, and evaluation-time contracts.
- Produces: `evaluate_reviews(input: ReviewEvaluationInput) -> ReviewEvaluation`.

- [ ] **Step 1: Write the decision-table tests**

```rust
#[test_case("critical.json", ReviewStatus::Blocked, "PK-REVIEW-HIGH")]
#[test_case("medium-confirmed.json", ReviewStatus::Blocked, "PK-REVIEW-MEDIUM-CONFIRMED")]
#[test_case("medium-conflict.json", ReviewStatus::Inconclusive, "PK-REVIEW-MEDIUM-CONFLICT")]
#[test_case("self-review.json", ReviewStatus::Inconclusive, "PK-SELF-PROVIDER")]
fn review_policy_is_fail_closed(name: &str, expected: ReviewStatus, reason: &str) {
    let actual = evaluate_reviews(load(name));
    assert_eq!(actual.status, expected);
    assert!(actual.has_reason(reason));
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p policy-core --test reviews`

Expected: FAIL because review consensus is missing.

- [ ] **Step 3: Implement severity and consensus rules**

Block on any unwaived Critical/High finding. Block when the same stable finding ID is confirmed by two independent providers. Return `INCONCLUSIVE` for an unconfirmed Medium, conflicting report, missing reviewer, duplicate provider/execution ID, or invalid provenance. Record Low/Info without blocking.

- [ ] **Step 4: Implement waiver validation with injected time**

A waiver is valid only when its authorized human actor is in `PolicySnapshot.authorized_waiver_actors`, and its finding ID, repository, head SHA, evidence digest, non-empty reason, issued timestamp, and expiry all match. Compare expiry to `ReviewEvaluationInput.evaluation_time`; never call `Utc::now()`.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p policy-core --test reviews`

Expected: every decision-table case passes, including a test where an original producer attempts review after another provider's one-line repair.

```bash
git add crates/policy-core tests/fixtures/reviews
git commit -m "feat: enforce independent review consensus"
```

---

### Task 6: Pre-merge evaluator

**Files:**
- Create: `crates/policy-core/src/pre_merge.rs`
- Create: `crates/policy-core/tests/pre_merge.rs`
- Create: `tests/fixtures/pre-merge/{ready,missing-agent-ready,head-mismatch,advisory}.json`

**Interfaces:**
- Consumes: validators from Tasks 3–5 and `PreMergeEvaluationInput`.
- Produces: `pub fn evaluate_pre_merge(&PreMergeEvaluationInput) -> PreMergeDecision`.

- [ ] **Step 1: Write aggregate gate tests**

```rust
#[test]
fn authoritative_ready_requires_all_evidence_classes() {
    let decision = evaluate_pre_merge(&load("ready.json"));
    assert_eq!(decision.status, PreMergeStatus::ReadyToMerge);
    assert!(!decision.advisory);
}

#[test]
fn advisory_mode_never_emits_authoritative_decision() {
    let decision = evaluate_pre_merge(&load("advisory.json"));
    assert_eq!(decision.status, PreMergeStatus::ReadyToMerge);
    assert!(decision.advisory);
    assert!(AuthoritativePreMergeDecision::try_from(decision).is_err());
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p policy-core --test pre_merge`

Expected: FAIL because the aggregate evaluator is missing.

- [ ] **Step 3: Implement deterministic aggregation**

Validate, in order: schema/policy, repository and exact PR head, authorized `agent-ready` observation, bundle integrity/freshness, execution evidence, test adequacy, review independence/consensus, and waivers. Preserve all reasons but apply precedence `BLOCKED` over `INCONCLUSIVE` over `READY_TO_MERGE`. Compute the decision digest only after normalizing reason order by stable code and evidence digest.

- [ ] **Step 4: Verify deterministic output**

Run the same ready fixture 100 times and assert identical canonical decision bytes and digest. Permute review and sensor input order and assert the same result.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p policy-core --test pre_merge`

Expected: all aggregate and determinism tests PASS.

```bash
git add crates/policy-core tests/fixtures/pre-merge
git commit -m "feat: evaluate pre-merge release readiness"
```

---

### Task 7: Post-merge evaluator and required-check anchor

**Files:**
- Create: `crates/policy-core/src/post_merge.rs`
- Create: `crates/policy-core/tests/post_merge.rs`
- Create: `tests/fixtures/post-merge/{merge,squash,wrong-head,missing-check,late-check,advisory}.json`

**Interfaces:**
- Consumes: `PostMergeEvaluationInput` containing validated authoritative pre-merge decision, required check, and merge observation.
- Produces: `pub fn evaluate_post_merge(&PostMergeEvaluationInput) -> PostMergeDecision`.

- [ ] **Step 1: Write merge semantics tests**

```rust
#[test_case("merge.json")]
#[test_case("squash.json")]
fn shipped_allows_distinct_merge_commit_oid(name: &str) {
    let input = load(name);
    assert_ne!(input.merge.merge_commit_oid, input.evaluated_pr_head);
    assert_eq!(evaluate_post_merge(&input).status, PostMergeStatus::Shipped);
}

#[test_case("wrong-head.json", "PK-POST-HEAD-MISMATCH")]
#[test_case("missing-check.json", "PK-POST-CHECK-MISSING")]
#[test_case("late-check.json", "PK-POST-CHECK-AFTER-MERGE")]
fn invalid_authorization_chain_fails(name: &str, reason: &str) {
    assert!(evaluate_post_merge(&load(name)).has_reason(reason));
}

#[test]
fn advisory_decision_is_rejected_before_post_merge_evaluation() {
    let raw = std::fs::read_to_string(fixture_path("advisory.json")).unwrap();
    assert!(serde_json::from_str::<PostMergeEvaluationInput>(&raw).is_err());
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p policy-core --test post_merge`

Expected: FAIL because the post-merge evaluator is missing.

- [ ] **Step 3: Implement the authorization chain**

Require equality across repository, PR number, `headRefOid`, policy digest, pre-merge decision digest, and project-declared required-check name. Require check conclusion `Success`, check completion before merge time, protected target branch, authorized human merge actor, and declared merge method. Record merge commit OID separately and reject undeclared merge-queue observations with `PK-POST-MERGE-QUEUE-UNSUPPORTED`.

- [ ] **Step 4: Verify and commit**

Run: `cargo test -p policy-core --test post_merge`

Expected: merge and squash fixtures ship; synthetic, stale, mismatched, unprotected, and late-check inputs do not. The advisory fixture fails strict `PostMergeEvaluationInput` deserialization and never reaches `evaluate_post_merge`.

```bash
git add crates/policy-core tests/fixtures/post-merge
git commit -m "feat: authorize shipped state from protected merge evidence"
```

---

### Task 8: Advisory CLI and schema generation

**Files:**
- Create: `crates/policy-cli/src/main.rs`
- Create: `crates/policy-cli/tests/cli.rs`
- Create: `schemas/{policy,evidence-bundle,sensor-profile,pre-merge-decision,post-merge-decision}.schema.json`
- Create: `xtask/src/main.rs`

**Interfaces:**
- Consumes: contract parsers and both evaluators.
- Produces: `policy-cli evaluate-pre`, `inspect`, and deterministic checked-in schemas.

- [ ] **Step 1: Write CLI failure and advisory tests**

```rust
#[test]
fn local_pre_merge_is_forced_advisory() {
    let output = Command::cargo_bin("policy-cli").unwrap()
        .args(["evaluate-pre", "tests/fixtures/pre-merge/ready.json"])
        .output().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["advisory"], true);
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p policy-cli`

Expected: FAIL because commands are missing.

- [ ] **Step 3: Implement CLI without remote mutation**

Force `EvaluationMode::Advisory` in `evaluate-pre` regardless of file content. Do not expose a local post-merge evaluation command. `inspect` validates and displays a supplied decision or bundle but never evaluates or rewrites it. Parse stdin when path is `-`, write one JSON object to stdout, diagnostics to stderr, and use exit codes 0 for advisory ready, 2 for blocked, 3 for inconclusive, and 64 for invalid input.

- [ ] **Step 4: Generate and verify schemas**

Add `xtask schema` using `schemars`; sort generated JSON keys before writing. Add a test that regeneration yields no diff and rejects fixtures with unknown fields or schema versions.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p policy-cli && cargo run -p xtask -- schema && git diff --exit-code -- schemas`

Expected: CLI tests pass and schema regeneration creates no diff.

```bash
git add crates/policy-cli schemas xtask Cargo.toml Cargo.lock
git commit -m "feat: add advisory CLI and versioned schemas"
```

---

### Task 9: Bounded evidence collector

**Files:**
- Create: `crates/evidence-collector/src/{lib,main}.rs`
- Create: `crates/evidence-collector/tests/collector.rs`
- Create: `tests/fixtures/collector/{success,timeout,truncated}.json`

**Interfaces:**
- Consumes: policy-declared explicit argv, allowlisted environment, timeout, output byte limit, repository/head binding.
- Produces: `ExecutionEvidence`; it never produces a gate decision.

- [ ] **Step 1: Write process-boundary tests**

```rust
#[test]
fn timeout_and_truncation_are_explicit_non_success() {
    assert_eq!(collect(timeout_request()).unwrap().status, ExecutionStatus::TimedOut);
    assert_eq!(collect(truncation_request()).unwrap().status, ExecutionStatus::Truncated);
}

#[test]
fn shell_strings_are_not_accepted() {
    assert!(CollectionRequest::try_from_json(r#"{"command":"sh -c echo ok"}"#).is_err());
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p evidence-collector`

Expected: FAIL because the collector is missing.

- [ ] **Step 3: Implement explicit-argv execution**

Accept a non-empty `Vec<String>` only; never invoke a shell. Apply the requested working directory, explicit environment allowlist, timeout, stdout/stderr byte limits, and process-group termination. Record resolved executable and version probe output, full argv, cwd, environment-name list without secret values, start/end timestamps, exit code, status, logs, artifact digests, provider/model/execution IDs supplied by the trusted runner, and input/output digests.

- [ ] **Step 4: Document replaceable reviewer execution**

In `docs/integrations/agent-skills.md`, state that a consuming workflow may combine an external review skill with OMP, `agy`, or another executor. The collector receives only the already selected argv and identities. It never embeds `gemini-reviewer`, chooses a provider/model, manages login, retries, compares failure signatures, owns repair budgets, chooses debugger escalation, or treats model prose as provenance. An isolated debugger's repair becomes new modification evidence and must be reviewed by a provider and execution independent of every contributor.

- [ ] **Step 5: Verify and commit**

Run: `cargo test -p evidence-collector`

Expected: success, non-zero exit, spawn failure, timeout, truncation, and forbidden-shell-form tests PASS.

```bash
git add crates/evidence-collector tests/fixtures/collector docs/integrations
git commit -m "feat: collect bounded execution evidence"
```

---

### Task 10: GitHub adapter, authoritative CI entry point, and Action

**Files:**
- Create: `crates/github-adapter/src/{lib,parse,gh}.rs`
- Create: `crates/github-adapter/tests/github.rs`
- Create: `crates/policy-ci/src/main.rs`
- Create: `crates/policy-ci/tests/ci.rs`
- Create: `action/action.yml`
- Create: `tests/fixtures/github/{issue,pr-open,pr-merged,check-success,check-forged}.json`

**Interfaces:**
- Consumes: trusted GitHub JSON and collected evidence.
- Produces: normalized observations, a dedicated GitHub Check Run carrying exact decision bindings, and authoritative output bound to the workflow run.

- [ ] **Step 1: Write GitHub normalization tests**

```rust
#[test]
fn squash_merge_preserves_pr_head_and_merge_commit_separately() {
    let observation = parse_merge(include_str!("../../../tests/fixtures/github/pr-merged.json")).unwrap();
    assert_eq!(observation.head_ref_oid, sha(PR_HEAD));
    assert_eq!(observation.merge_commit_oid, sha(SQUASH_COMMIT));
    assert_ne!(observation.head_ref_oid, observation.merge_commit_oid);
}

#[test]
fn arbitrary_check_json_cannot_claim_trusted_provenance() {
    assert!(parse_trusted_check(forged_json(), untrusted_context()).is_err());
}

#[test]
fn check_run_binding_payload_stays_bounded() {
    let payload = serialize_check_binding(maximum_valid_binding()).unwrap();
    assert!(payload.len() <= 4_096, "binding payload exceeded 4 KiB safety cap");
}
```

- [ ] **Step 2: Run and observe failure**

Run: `cargo test -p github-adapter -p policy-ci`

Expected: FAIL because parser and CI entry point are missing.

- [ ] **Step 3: Implement the adapter boundary**

Define a `GitHubApi` trait returning raw JSON bytes and a `GhCliApi` implementation that executes only fixed `gh api` routes with separately supplied owner, repository, issue, PR, check-run, and workflow-run identifiers. Reject repository/name injection before argv construction. Normalize label events, PR `headRefOid`, merge commit OID, branch protection, actor, method, timestamps, required-check conclusion, workflow/run identity, and decision/policy digests.

Add `publish_pre_merge_check(decision, workflow_identity) -> GitHubCheckObservation`. It creates a dedicated Check Run through `POST /repos/{owner}/{repo}/check-runs` with name `policy-kernel/pre-merge`, the evaluated PR `head_sha`, `status: completed`, and `conclusion: success|failure|neutral` mapped from ready/blocked/inconclusive. Set `external_id` to `policy-kernel:v1:<decision_digest>:<policy_digest>` and set `output.text` to canonical JSON containing `schema_version`, `repository`, `pull_request_number`, `head_sha`, `policy_digest`, `decision_digest`, `workflow_run_id`, `workflow_run_attempt`, and `completed_at`. `parse.rs` extracts exactly those fields from `external_id` and `output.text`, rejects disagreement, and binds the API-observed check-run ID and timestamps. The workflow declares `permissions: checks: write, contents: read, pull-requests: read, issues: read`.

- [ ] **Step 4: Implement `policy-ci` authoritative evaluation**

Require GitHub Actions identity fields, checked-out full head SHA, policy/evidence paths, and a trusted observation directory. Assemble `PreMergeEvaluationInput` with `EvaluationMode::Authoritative`; emit canonical `decision.json`, publish it using `publish_pre_merge_check`, then emit GitHub output fields and a concise step summary. Return non-zero for `BLOCKED` and `INCONCLUSIVE` after publishing the non-success Check Run. A separate `post-merge` command fetches the dedicated Check Run plus merge observations and calls `evaluate_post_merge`.

- [ ] **Step 5: Define the composite Action**

`action/action.yml` pins inputs for mode, policy path, evidence path, expected head, and artifact directory. It invokes the already-built `policy-ci` and documents the required minimal GitHub permissions. It does not download moving binaries, call a model, merge, waive, or change branch protection.

- [ ] **Step 6: Verify and commit**

Run: `cargo test -p github-adapter -p policy-ci`

Expected: parsing, injection rejection, exact-head, required-check digest, 4 KiB Check Run payload cap, advisory rejection, merge, and squash tests PASS.

```bash
git add crates/github-adapter crates/policy-ci action tests/fixtures/github
git commit -m "feat: add GitHub CI policy gate adapter"
```

---

### Task 11: Adversarial gate, residue scan, shadow comparison, and rollback

**Files:**
- Create: `crates/policy-core/tests/adversarial_release_gate.rs`
- Create: `crates/policy-core/tests/legacy_cases.rs`
- Create: `scripts/check-residue.sh`
- Create: `.github/workflows/ci.yml`
- Create: `deny.toml`
- Create: `docs/{trust-model,promotion}.md`

**Interfaces:**
- Consumes: complete workspace and Task 1 obligation matrix.
- Produces: release evidence for the new repository and a tested native rollback procedure.

- [ ] **Step 1: Write the cross-crate adversarial suite**

Cover every obligation-matrix row plus digest tampering, replayed `agent-ready`, missing reviewer, contributor/provider overlap, duplicate execution, expired waiver, sensor timeout, empty test discovery, stale CodeGraph attestation when explicitly required, local evidence in authoritative mode, synthetic required check, wrong PR head, late check, direct/unprotected merge, merge/squash OID difference, and unsupported merge queue.

- [ ] **Step 2: Write the differential expectation runner**

Encode frozen legacy cases as input/expected-decision data, not imports from the old crate. For intentional semantic changes—removal of GSD lifecycle, contributor-wide independence, explicit evaluation mode, required-check anchoring—record `changed_by_design: true`, the new reason code, and the approving spec section. Assert every matrix row is executed by at least one contract or adversarial test.

- [ ] **Step 3: Add a residue scan that fails before exclusions are correct**

```bash
#!/usr/bin/env bash
set -euo pipefail
if rg -n 'gsd-core|\.planning|gemini-reviewer|\bagy\b|oh-my-pi|\bomp\b|mcp|hashline' \
  crates/*/src schemas action Cargo.toml; then
  echo 'forbidden runtime residue found' >&2
  exit 1
fi
cargo tree -p policy-core | rg 'reqwest|octocrab|clap|tokio' && exit 1 || true
```

The scan intentionally excludes tests, provenance, and integration documentation, where negative cases and historical/external names are allowed. A separate public-API test asserts that forbidden runtime names do not occur in exported items or generated schemas.

- [ ] **Step 4: Add property and mutation gates**

Use `proptest` to generate review-report permutations, evidence orderings, head/time changes, malformed digests, and waiver expiry boundaries. Assert consensus order invariance, blocking monotonicity for unwaived Critical/High findings, waiver invalidation after head or evaluation-time changes, and deterministic sensor selection.

Install the exact `cargo-mutants` version recorded in `docs/provenance.md`, then run mutation analysis over `canonical.rs`, `provenance.rs`, `reviews.rs`, `pre_merge.rs`, and `post_merge.rs`. Promotion requires zero missed mutations and zero timeouts in these authorization-critical files; unviable mutants are recorded separately and do not count as killed.

- [ ] **Step 5: Configure complete CI verification**

Pin third-party Actions by immutable commit SHA. Run `cargo fmt --check`, `cargo check --workspace --locked`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, schema regeneration diff, `cargo deny check`, `cargo audit`, residue scan, and a clean build with an empty target directory. A separate promotion workflow runs the pinned `cargo-mutants` command and rejects any missed mutation or timeout in the five authorization-critical files. Upload decisions and logs without secrets.

- [ ] **Step 6: Document promotion and native rollback drill**

Define advisory shadow duration and sample size, required parity exceptions, exact policy/head digest approval, required-check name, protected branch settings, human merge authorization, and rollback steps. The rollback removes the kernel check from required status, restores the recorded deterministic CI/review baseline, leaves evidence immutable, and never reactivates `.planning` or GSD Core.

- [ ] **Step 7: Run the full release-quality evidence set**

Run:

```bash
cargo fmt --check
cargo check --workspace --locked
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo run -p xtask -- schema
git diff --exit-code -- schemas
cargo deny check
cargo audit
bash scripts/check-residue.sh
cargo mutants -p policy-core --file crates/policy-core/src/canonical.rs --file crates/policy-core/src/provenance.rs --file crates/policy-core/src/reviews.rs --file crates/policy-core/src/pre_merge.rs --file crates/policy-core/src/post_merge.rs
```

Expected: every command exits 0; test output reports no ignored required test, schema/residue checks produce no diff or finding, and mutation output reports zero missed mutations and zero timeouts. If `cargo-deny`, `cargo-audit`, or the pinned `cargo-mutants` is unavailable, report `INCONCLUSIVE`; do not record a pass.

- [ ] **Step 8: Independent review and final commit**

Freeze the candidate head and collect read-only semantic reviews from at least two providers independent of every code contributor. Run the release-quality review against that exact head. Any unwaived Critical/High, confirmed Medium, missing trusted reviewer, or changed head prevents promotion.

```bash
git add .github deny.toml docs scripts tests
git commit -m "test: enforce policy kernel release gates"
```

Do not create a remote, change GitHub protection, or merge. Present the exact final head, policy digest, command evidence, review evidence, and rollback baseline to the human owner for separate authorization.

## Integration amendment — 2026-09-07

This amendment is a design-level work breakdown, not an implementation-ready
replacement for Tasks 1–11. Read the linked profile/bootstrap design before
implementing these additions. Refine each new task into an executable plan when
its prerequisite contracts are available. Do not renumber existing tasks or infer
their completion from the unchecked historical plan.

### Existing task impact

| Task | Required integration |
| --- | --- |
| 2: contracts | Preserve language-neutral contracts; inspect schema fit before any field addition and version breaking changes explicitly. |
| 4: sensor validation | Cover combined profiles, missing reports, zero required tests, and policy-declared applicability. |
| 8: CLI/schemas | Keep local evaluation advisory; validate generated profile instances against the actual schemas. |
| 9: collector | Accept selected argv and collect execution provenance; report normalization is outside the pure core. |
| 10: CI/Action | Document consuming-project inputs and authoritative reruns of required checks at the evaluated head. |
| 11: acceptance | Keep Rust release gates; additionally validate profile assets and bootstrap independently before distributing those assets. |

Tasks 3–7 retain their authorization and independence semantics. Profile support
does not replace the policy-kernel repository's own Rust checks with JS/.NET checks.

### Task 12: Project profiles and report normalization

**Prerequisites:** Tasks 2, 4, 8 and 9.
**Files:** `profiles/{react-typescript,dotnet,fullstack}/`,
`tests/fixtures/project-reports/`, `docs/integrations/project-verification.md`.
**Deliverable:** Three versioned profiles plus external report normalizers producing
existing SensorEvidence/ExecutionEvidence and adequacy references.

- [ ] Bind explicit argv, working directories, toolchain locks, parser version and applicability to each profile.
- [ ] Validate profile instances against Task 8 schemas before execution.
- [ ] Normalize Vitest/Playwright machine reports and the pinned .NET runner's reports; retain counts, failures, skips and original artifact digests.
- [ ] Verify valid, empty, malformed, stale, skipped, timeout and nonzero-exit fixtures. A zero exit with an absent or empty required report must never satisfy the gate.
- [ ] Demonstrate fullstack composition requires both projects and the declared API/E2E checks without conflicting sensor IDs.
- [ ] Document exact local/CI commands and pin compatible versions in each runnable fixture before release.

### Task 13: Bootstrap installer and templates

**Prerequisite:** Task 12; Task 10 for CI templates.
**Files:** `bootstrap/`, `tests/fixtures/bootstrap/`,
`docs/integrations/bootstrap.md`.
**Deliverable:** Separate installer supporting inspect/plan/apply with the behavior
defined in the bootstrap specification; no new policy-cli authority.

- [ ] Accept target path, selected profile, component paths and a pinned template release.
- [ ] Preview additions/conflicts, install only missing or unchanged managed files, and record template version and content digests.
- [ ] Check empty React, empty .NET, combined, and existing-project fixtures.
- [ ] Verify repeat application is a no-op; edited files and ambiguous project detection produce an explicit conflict without writes.
- [ ] Reject paths escaping the target, including symlink escapes.
- [ ] Verify no credentials, provider login, application business logic, or remote protection changes are generated.

### Task 14: Consuming-project acceptance

**Prerequisites:** Tasks 10–13.
**Files:** `examples/{react-typescript,dotnet,fullstack}/`,
`docs/integrations/acceptance.md`.
**Deliverable:** Runnable fixtures and evidence demonstrating the installed checks,
not merely successful template rendering.

- [ ] Run each profile with its pinned Node/.NET versions in CI.
- [ ] Introduce a type error, failing component test, rejected API authorization, empty required report, and frontend/backend contract mismatch in separate fixtures; verify each cannot yield authoritative ready.
- [ ] Verify correction and rerun bind all evidence to the new head; previous-head evidence remains invalid.
- [ ] Record local advisory and CI-authoritative outcomes separately.
- [ ] Report unavailable tools/runners as unverified, and retain the native CI baseline until normal promotion requirements are met.
