# Policy Kernel Extraction Design

> Date: 2026-09-04
> Status: Final specification approved by the human owner; ready for implementation planning
> Source repository: `agent-dev-kit` at `e5f6ec8584cc91fdeca3aab40687c6cdcd303b43`

## 1. Context

`agent-dev-kit` began as a language-agnostic, multi-model development and
verification harness. Its current implementation mixes three different concerns:

1. development workflow and agent orchestration;
2. command, model, and tool execution;
3. release-policy and evidence evaluation.

Existing harnesses now provide the first two concerns more completely. Codex plus
Superpowers will own the normal Discuss, Plan, Execute, and local Verify workflow.
Oh My Pi (OMP) may provide an alternate-provider read-only reviewer and, after a
bounded number of failed repair attempts, an isolated debugger. The new repository
will retain only the third concern as a small, deterministic policy kernel.

This is an extraction into a new repository, not a refactor in place. The goal is
to preserve policy knowledge and adversarial test cases without carrying forward
orchestration, shell, session, container, and compatibility residue.

## 2. Decision drivers

- CI and the Git provider, not a local agent, must own authoritative release
  decisions.
- A producer must not approve its own work. Provider and execution independence
  are evaluated against every code-modifying execution contributing to the
  evaluated change set, not only the last modifier.
- Missing tools, authentication, quota, provenance, sensors, or reviewers must
  remain explicit non-success states.
- Local and CI evaluation must use the same deterministic policy implementation.
- Agent runtimes, models, skills, and code-navigation tools must remain replaceable.
- The kernel must be understandable and testable without running an LLM, GitHub,
  Docker, a shell command, or a network service.
- The migration must be reversible and must not require importing mutable legacy
  lifecycle state.

## 3. Selected approach

Use a **contract-first selective reimplementation** in a fresh repository.

The migration will extract normative obligations, adversarial cases, and portable
fixtures before implementation. New code will be written against those contracts.
Only small, already-pure functions may be selectively transplanted after review;
entire legacy modules will not be copied.

This approach is preferred over extracting and cleaning the existing `tools` crate,
because the current modules combine pure policy with filesystem access, process
execution, environment discovery, configuration fallback, and CLI presentation. It
is preferred over a strict clean-room rewrite because both repositories have the
same owner and the existing negative tests contain valuable security knowledge that
should not be discarded.

## 4. Scope

### 4.1 In scope

- versioned policy and evidence contracts;
- deterministic lifecycle and release-state validation;
- evidence completeness, integrity, freshness, and exact-SHA validation;
- producer/verifier provider and execution independence;
- severity consensus and conflict handling;
- human waiver validation;
- project sensor-profile parsing and applicability resolution;
- GitHub observation validation for `agent-ready`, checks, PR head, and protected
  merge;
- a local advisory CLI using the shared kernel;
- a CI-authoritative entry point using the shared kernel;
- content-addressed evidence bundles and machine-readable decisions.

### 4.2 Out of scope

- agent TUI, chat, session, memory, compaction, or remote-control features;
- model catalogs, OAuth, provider fallback, rate limiting, or model invocation;
- subagent scheduling and repair-loop orchestration;
- code editing, Hashline, LSP, browser, MCP, or CodeGraph implementations;
- plan generation, plan checking, or context-envelope orchestration;
- Docker Compose agent ecosystems and credential distribution;
- Discord, Telegram, or another approval channel;
- GitLab support in the first release;
- a daemon, database, dashboard, or hosted control plane.

## 5. Workflow ownership

### 5.1 Normal development

Codex plus Superpowers owns the non-authoritative development workflow:

```text
Discuss -> Plan -> Execute -> local Verify
```

Superpowers supplies brainstorming, planning, TDD, systematic debugging, review,
and verification procedures. Codex is normally the implementation producer. These
workflow states are not policy-kernel lifecycle states and are not synchronized
into a second orchestration database.

### 5.2 Independent review and repair

OMP is an execution surface, not an identity. A read-only OMP review is independent
only when its resolved provider differs from every provider that modified code in
the evaluated change set and its execution ID differs from every producing
execution. A small repair by another provider does not make an original producer
eligible to approve the combined result.

The two OMP roles must remain separate:

- **Verifier:** reads committed artifacts, does not edit, and emits structured
  review evidence.
- **Debugger:** receives a bounded failure context in an isolated worktree and may
  create a repair commit after the normal Codex repair budget is exhausted.

A debugger cannot verify its own repair. After any repair, verification is repeated
against the new head SHA by a different provider and execution.

Repair budgets, failure-signature comparison, and the point at which OMP is selected
belong to the external Superpowers/controller workflow. They are not policy-kernel
rules or state. The kernel only rejects evidence when a repair is self-approved or
when required provenance is missing. Authentication, quota, infrastructure, and
context-truncation failures remain explicit non-success evidence.

### 5.3 Authoritative evaluation

CI collects or re-executes required evidence at the actual PR head, then calls the
policy kernel's pre-merge evaluator. It returns one of:

```text
BLOCKED
INCONCLUSIVE
READY_TO_MERGE
```

After merge, a separate deterministic post-merge evaluator returns `SHIPPED` or a
non-success decision. Neither an LLM nor an adapter may synthesize that
authorization.

The post-merge evaluator also requires a trusted GitHub required-check observation
showing that the exact successful pre-merge decision digest was published for the
evaluated PR head before merge. A caller-supplied decision object without that
observation cannot authorize `SHIPPED`.

“Exact evaluated head” means the merged PR observation's `headRefOid` equals the
head SHA bound to the successful pre-merge decision. It does not mean that a merge
or squash commit SHA equals the PR head SHA. The post-merge input records the PR
head OID, resulting merge commit OID, target branch, merge method, actor, merge
timestamp, and successful pre-merge decision digest. Merge-queue support requires
its own declared observation contract and is not inferred from ordinary PR data.

## 6. GSD Core disposition

GSD Core is removed as a runtime dependency. Its full Discuss, Plan, Execute,
Verify, and Ship orchestration is not reproduced.

The kernel retains only minimal release facts evaluated by two explicit functions:

```text
EvidenceCollected -> Evaluated -> Blocked | Inconclusive | ReadyToMerge
ReadyToMerge decision digest + matching protected merge observation -> Shipped
```

`.planning/STATE.md`, `.planning/events.jsonl`, GSD commands, GSD hooks, and GSD
compatibility contracts are historical inputs and are not migrated into production
state. If legacy interoperability is temporarily needed, it belongs in a disposable
migration adapter outside the production kernel.

## 7. Architecture

```text
Codex + Superpowers             OMP
(producer and local checks)     (alternate verifier or debugger)
             |                   |
             +------ evidence ---+
                         |
                         v
               Local advisory CLI
                         |
                    commit / PR
                         |
                         v
                CI evidence collector
                         |
                         v
              +---------------------+
              | policy-contracts    |
              | policy-core         |
              +---------------------+
                         |
          PreMergeDecision + digest
                         |
                         v
             GitHub required check
                         |
          authorized protected merge
                         |
                         v
             PostMergeDecision
                         |
                      Shipped
```

### 7.1 Repository layout

```text
policy-kernel/
  Cargo.toml
  crates/
    policy-contracts/
    policy-core/
    policy-cli/
    policy-ci/
    evidence-collector/
    github-adapter/
  schemas/
    policy.schema.json
    evidence-bundle.schema.json
    sensor-profile.schema.json
  tests/
    fixtures/
    contract/
    adversarial/
    differential/
  docs/
    architecture.md
    trust-model.md
    provenance.md
    adr/
  action/
    action.yml
```

### 7.2 `policy-contracts`

Defines serializable, versioned types without policy decisions:

- `PolicySnapshot` and its digest;
- `WorkItemObservation`;
- `ExecutionEvidence`;
- `TestAdequacyEvidence`;
- `ReviewEvidence` and `Finding`;
- `SensorProfile`, `SensorDefinition`, and `SensorEvidence`;
- `WaiverEvidence`;
- `GitHubPullRequestObservation`;
- `GitHubCheckObservation`;
- `GitHubMergeObservation`;
- `EvidenceManifest` and `EvidenceBundle`;
- `EvaluationMode`, `PreMergeDecision`, `PostMergeDecision`, and stable reason
  codes.

Unknown required fields, unsupported schema versions, invalid timestamps, malformed
digests, and ambiguous identities fail closed.

### 7.3 `policy-core`

Contains deterministic functions only. It receives complete values and returns
values; it does not discover ambient state.

```rust
evaluate_pre_merge(input: &PreMergeEvaluationInput) -> PreMergeDecision
evaluate_post_merge(input: &PostMergeEvaluationInput) -> PostMergeDecision
```

`PreMergeEvaluationInput` explicitly contains the policy snapshot, work-item and
pull-request observations, evidence bundle, evaluation time, and evaluated head.
`PostMergeEvaluationInput` contains the successful pre-merge decision and digest,
the evaluated PR head, the trusted required-check observation that published that
decision digest, the protected merge observation, policy snapshot, and evaluation
time. The post-merge function verifies that the successful required check bound the
same repository, PR, head, policy digest, and decision digest and completed before
the protected merge. The post-merge function cannot return a pre-merge decision,
and the pre-merge function cannot return `SHIPPED`.

`PreMergeEvaluationInput` also carries an explicit `EvaluationMode::Advisory` or
`EvaluationMode::Authoritative`. The shared core applies the corresponding trust
threshold and records the mode plus `advisory: true|false` in the decision. An
advisory decision is structurally unable to satisfy a post-merge input.

The core must not read files, environment variables, clocks, Git state, networks,
or subprocesses. Evaluation time, current head, provider identity, and all external
observations are explicit inputs bound into the output digest.

### 7.4 `policy-cli`

Provides local advisory evaluation and evidence inspection. Every result is marked
`advisory: true`. It cannot publish required checks, authorize waivers, record
`SHIPPED`, or mutate remote lifecycle state.

### 7.5 `policy-ci`

Captures required sensors and provider CLI results under the CI identity, loads
trusted GitHub observations, verifies bundle integrity, invokes `policy-core`, and
emits the authoritative machine result. It does not implement a second copy of
policy rules or choose providers, credentials, fallbacks, or retry budgets.

A required alternate-provider review runs in CI or an explicitly trusted runner.
Locally produced OMP evidence remains advisory even if it is signed or names an
alternate provider.

### 7.6 `evidence-collector`

Runs a policy-declared explicit argument vector in a bounded CI step and records
resolved executable, version, environment allowlist, provider/model identity,
execution ID, input/output digests, timestamps, timeout, exit status, and log
reference. It may wrap a provider CLI such as OMP but does not select models,
manage authentication, retry failures, interpret prose, or make policy decisions.

### 7.7 `github-adapter`

Fetches GitHub Issue, label-event, PR, check, actor, and merge observations. It
normalizes those observations into `policy-contracts`. Authentication and API calls
stay in the adapter; exact-SHA and authorization decisions stay in `policy-core`.

`GitHubCheckObservation` records repository, PR, head SHA, check name, workflow and
run identity, decision and policy digests, conclusion, start/completion times, and
the trusted observation provenance. The core requires the project-declared check
name and a successful conclusion for post-merge authorization.

`work_item_id` and `pull_request_number` are distinct types to prevent accidental
substitution.

## 8. Evidence model

Each evaluation produces a content-addressed bundle:

```text
evidence/<run-id>/
  manifest.json
  commands/*.json
  reviews/*.json
  sensors/*.json
  artifacts/*
  decision.json
```

The manifest binds repository, work item, base/head SHA, policy digest, runner,
timestamps, and the digest of every child artifact. Canonical serialization and
SHA-256 provide integrity, not truth. Trust is recorded separately:

```text
LocalUntrusted
CiObserved
TrustedRunnerAttested
```

For the first release, authoritative evidence is limited to observations captured
directly by `policy-ci`/`evidence-collector` in the current GitHub workflow run, or
an explicitly accepted trusted-runner attestation. Required deterministic sensors
normally run again in CI. Local AI reviews are always advisory. A local signature
proves origin only; it does not convert a claim into execution evidence.

The first release requires canonical JSON, SHA-256, exact repository/head binding,
and GitHub workflow identity. A trusted-runner attestation additionally binds the
runner identity and invocation to those values. Artifact attestation or Sigstore
integration may be added later, but cannot weaken these bindings.

## 9. Sensor policy

Projects own a versioned sensor profile. The kernel parses the profile, evaluates
applicability, and verifies evidence completeness. It does not execute commands.

Required, missing, timed-out, skipped, or unparsable sensors cannot pass. A sensor
may be `NOT_APPLICABLE` only when its declared condition matches and the reason is
recorded. Each sensor has a typed result schema containing the resolved command,
source and configuration digests, result counts where applicable, and parser
version. Exit code zero proves only that the recorded command succeeded against the
recorded inputs.

Test adequacy is separate `TestAdequacyEvidence`: it maps acceptance criteria and
changed behavior to executed tests and negative cases. A successful no-op command,
an empty test discovery result, or execution evidence without the required adequacy
mapping cannot satisfy the test-adequacy gate.

## 10. Knowledge routing: LAT and CodeGraph

LAT and CodeGraph are optional knowledge tools used by the workflow layer, not
policy-kernel dependencies.

### 10.1 LAT

LAT stores approved semantic knowledge, observed legacy behavior, domain vocabulary,
architecture decisions, and unresolved uncertainty. Approved and observed claims
must remain distinguishable. LAT cannot assert that tests passed or that a provider
executed.

Codex or OMP may consult LAT during Discuss, Plan, or semantic Verify. Newly observed
claims remain proposed until the declared human or project process approves them.
The kernel may validate a referenced LAT commit and content digest but never treats
LAT prose alone as approval evidence.

### 10.2 CodeGraph

CodeGraph is used conditionally for current-source structure and impact analysis.
Its declared indexable source set must exist in the exact current worktree, and the
source-set digest, tool version, index configuration, database digest, and evaluated
head must be recorded. The database file alone is never proof that the index is
current. A parent-worktree or stale index is invalid.

When project policy makes CodeGraph required, CI must rebuild the index from the
checked-out evaluated head or consume a trusted-runner attestation binding all of
those values. Otherwise CodeGraph remains supplemental and its absence cannot block
or approve a core gate.

Compiler, LSP, and test results outrank CodeGraph. A CodeGraph result is supplemental
evidence and cannot replace execution evidence, test adequacy, or semantic review.
CodeGraph and another structural-analysis tool are not activated for the same
question by default.

### 10.3 Routing ownership

Project bootstrap and research-routing skills decide when LAT or CodeGraph is
applicable. Codex or OMP performs the read/query. The policy kernel only validates
submitted provenance and enforces a project policy if that policy explicitly made
the tool required.

Default routing policy:

```yaml
knowledge:
  lat:
    enabled: auto
    required: false
    triggers: [discuss, plan, semantic-review]
  codegraph:
    enabled: conditional
    required: false
    require_database_at_current_head: true
    triggers: [impact-analysis, public-api-change, cross-module-change]
```

## 11. Agent Skills

Agent Skills remain external evidence producers. They live in consuming projects or
a separately versioned plugin/skill pack, not in the policy kernel's production
dependency graph. In particular, the current repository's `gemini-reviewer` is a
project-local review-instruction skill, not a Codex command and not a kernel
component. A consuming workflow may pair that skill with an explicitly configured
OMP or `agy` execution adapter, and may replace either independently.

A skill may guide a reviewer and produce findings, but it cannot transition release
state, approve itself, waive a finding, or establish model provenance by naming a
provider in its output. The first-release kernel schemas contain no built-in skill,
CLI, provider, or model names.

## 12. Failure semantics

- `BLOCKED`: reproducible policy violation or required execution failure.
- `INCONCLUSIVE`: required evidence, identity, tool, authentication, quota, context,
  or trusted observation is missing or invalid.
- `READY_TO_MERGE`: every required policy obligation is satisfied for the exact
  evaluated PR head; this is a pre-merge decision only.
- `SHIPPED`: a protected post-merge observation matches the exact evaluated PR head
  and successful pre-merge decision digest.

Retries are owned by the workflow or CI adapter and are classified before execution.
Fallback always re-evaluates provider and execution independence. Unsupported schema
versions and unknown required reason codes fail closed.

## 13. Migration and provenance

The old repository is evidence, not a source tree template.

1. Freeze the source commit and record relevant file blob SHAs and content digests.
2. Build an obligation matrix mapping each normative rule to its source, input
   fixture, expected decision, and non-goal.
3. Harvest adversarial and golden fixtures without importing mutable legacy state.
4. Write failing contract tests in the new repository.
5. Implement the smallest vertical slices against those tests.
6. Selectively transplant only reviewed pure logic and record every transplant in
   `docs/provenance.md`.
7. Run old and new evaluators in shadow mode against identical frozen inputs.
8. Promote the new CI job only after the declared parity and adversarial criteria
   pass; never run two authoritative state owners concurrently.

The new repository begins private unless the human owner explicitly selects a
license. Before public distribution, copied code and fixtures require an explicit
license and provenance review. Adapted external skills are not migrated into the
kernel.

## 14. Residue-free acceptance criteria

The production dependency graph and public API contain no agent-runtime or workflow
surface. The following are forbidden in `policy-core`:

- `std::process`, `std::fs`, ambient environment or clock access;
- `gh`, Docker, MCP, TUI, LSP, browser, Hashline, plan checker, rescue scripts, or
  token-bucket dependencies;
- `.planning` paths and GSD commands or generated-state formats;
- provider login, model routing, prompt, session, or repair-loop code;
- GitHub network calls and tokens.

CI enforces a dependency allowlist, forbidden-symbol scan, public-API review, schema
compatibility tests, and a clean build from an empty cache. Removed features must
not appear in CLI help or production schemas. Historical names may appear only in
provenance and migration documentation.

## 15. Verification strategy

### 15.1 Contract and adversarial tests

- every valid and invalid release-state transition;
- missing, malformed, stale, reordered, and digest-mismatched evidence;
- same-provider and same-execution self-approval;
- verifier provider matching any earlier code contributor despite a different last
  modifier;
- fallback that accidentally resolves to the producer provider;
- unwaived Critical/High and conflicting/confirmed Medium findings;
- waiver actor, finding, reason, head, digest, and expiry binding;
- sensor `required_when`, `NOT_APPLICABLE`, timeout, and accepted-exit-code edges;
- missing or replayed `agent-ready` observations;
- PR head mismatch, unmerged PR, and stale approved evidence;
- merge/squash commit OID differing from the evaluated PR head while `headRefOid`
  still matches;
- merged observation whose `headRefOid` does not match the evaluated PR head;
- post-repair verification by the repairing provider;
- local advisory evidence presented as authoritative.
- synthetic pre-merge decision without a matching successful required-check
  observation;
- advisory decision presented to the post-merge evaluator.

### 15.2 Property and mutation testing

- lifecycle transition closure and event continuity;
- consensus result invariance under report ordering;
- monotonic blocking for unwaived Critical/High findings;
- waiver invalidation across head or evaluation-time changes;
- deterministic sensor selection for normalized paths;
- fail-closed behavior for unknown required values.

Mutation thresholds are defined before release promotion. A surviving mutation in a
security- or authorization-relevant branch blocks promotion.

### 15.3 Integration validation

The new job first runs as advisory beside the existing required gate. Frozen fixture
parity, live GitHub observation parsing for merge and squash methods,
evidence-bundle reproducibility, CI-observed alternate-provider provenance, and the
native rollback path are reported separately. Unit success is not reported as live
end-to-end validation.

## 16. Promotion and rollback

Promotion changes only the GitHub required-check configuration after:

- all required contract, adversarial, property, and integration checks pass at the
  candidate head;
- no unresolved semantic divergence remains between the approved obligation matrix
  and the new evaluator;
- an independent provider critiques the candidate design and implementation;
- the human owner approves the exact candidate and policy digest.

Before promotion, the repository records and tests a native GitHub safety baseline:
branch protection, required deterministic CI checks, review requirements, and human
merge authorization that do not depend on the legacy `.planning` state. Rollback
restores that declared baseline and returns the new kernel to advisory mode; it does
not reactivate GSD Core. Evidence bundles remain readable and are never rewritten.
The old and new systems must not both record authoritative release state.

## 17. Considered alternatives

### Extract and clean the existing crate

Rejected because deletion-driven migration is likely to retain ambient-state,
execution, and compatibility coupling. It is faster initially but conflicts with
the residue-free objective.

### Strict clean-room implementation

Rejected because legal isolation is not required and excluding existing adversarial
fixtures would discard valuable security knowledge. It also increases semantic
regression risk.

### Contract-first selective reimplementation

Selected because it preserves policy knowledge and negative cases while forcing a
new architecture, explicit provenance, and a minimal dependency graph.

## 18. Validation obligations and decision status

This final specification is approved by the human owner, and its independent
Antigravity critique passed with no unresolved Critical, High, or Medium design
finding. Repository creation and implementation follow only through the reviewed
Superpowers `writing-plans` execution plan; this specification alone is not an
instruction to merge, publish, or change GitHub protection.
