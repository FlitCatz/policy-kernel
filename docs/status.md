# Current status

This page is the canonical summary of the repository's current documentation
state. It links to dated records for evidence and does not replace their
contemporaneous statements.

## Implemented scope

Tasks 1–14 provide implemented components in this workspace: typed policy contracts
and core evaluation; advisory and GitHub Actions boundaries; bounded collection; release
verification; project profiles; transactional bootstrap; and native consumer
acceptance. The [Task index](tasks/README.md) describes each scope and its
supporting records.

## Follow-up in progress and remaining implementation

[Task 15](tasks/task-15/README.md) consolidates compatibility and runtime
independence, current profile/documentation corrections, complete dependency
auditing, mandatory knowledge evidence, and authenticated collection-to-CI
integration. Its first batch (15.01–15.03) is implemented and locally verified:
current examples use bootstrap 1.0.2, isolated built-binary tests exercise operation
without agent configuration or Rust on PATH, and the locked workspace builds/tests
with Rust 1.85.0 and 1.95.0 on macOS ARM64. The added Linux/macOS compiler CI matrix
is configured, not yet observed for this candidate.

Tool/profile selection, complete dependency auditing, required knowledge, and
authenticated consumer integration remain pending. First-batch regression checks
do not complete Task 15's broader native acceptance and promotion requirements.

The policy contract still cannot declare mandatory knowledge/index presence.
The production observation controller remains an external integration requirement;
native acceptance is advisory. These are implementation/integration gaps as well
as the separate live promotion conditions below. Local CodeGraph tooling does not
satisfy mandatory knowledge evidence or authenticate CI observations.

The three defects from the final codebase review were remediated in `5841d8b`,
including the fullstack shared audit in bootstrap 1.0.2. Task 15 preserves those
regressions; its first batch corrects current entry points without rewriting the
historical root profile assets or earlier execution records.

## Verified evidence

- The [Task 15 foundation verification record](archive/reviews/2026-09-12-task-15-foundation-verification.md)
  records the first implementation batch, framework-reference classification,
  two-compiler local checks, and the boundaries of that evidence.

- The [final codebase verification record](archive/reviews/2026-09-10-final-codebase-verification.md)
  records local automated checks and CI observations for its reviewed candidate,
  while also recording defects and unfinished work.
- The [Task 14 execution record](archive/reviews/2026-09-09-task-14-consuming-acceptance.md)
  records local native acceptance evidence for React/TypeScript, .NET, and
  full-stack fixtures.
- The [Task 14 remediation record](archive/reviews/2026-09-10-task14-remediation.md)
  records follow-up changes and validation context.

These records are evidence for their stated revisions and environments. They do
not authenticate a later candidate automatically.

## Not demonstrated in production

The repository does not claim a completed production promotion. The following
remain governed by the release protocol: an owner-approved candidate, authenticated
exact-head required-check observations, protected-merge evidence, the required
shadow window, independent-provider reviews, and a live rollback observation.

## Canonical references

- [Task index](tasks/README.md)
- [Task 15 consolidated follow-up](tasks/task-15/README.md)
- [Trust model](trust-model.md)
- [Promotion and rollback protocol](promotion.md)
- [Project verification integration](integrations/project-verification.md)
- [Bootstrap integration](integrations/bootstrap.md)
- [Consumer acceptance integration](integrations/acceptance.md)
