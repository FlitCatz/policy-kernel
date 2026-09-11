# Current status

This page is the canonical summary of the repository's current documentation
state. It links to dated records for evidence and does not replace their
contemporaneous statements.

## Implemented scope

Tasks 1–14 are implemented in this workspace: typed policy contracts and core
evaluation; advisory and GitHub Actions boundaries; bounded collection; release
verification; project profiles; transactional bootstrap; and native consumer
acceptance. The [Task index](tasks/README.md) describes each scope and its
supporting records.

## Verified evidence

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
- [Trust model](trust-model.md)
- [Promotion and rollback protocol](promotion.md)
- [Project verification integration](integrations/project-verification.md)
- [Bootstrap integration](integrations/bootstrap.md)
- [Consumer acceptance integration](integrations/acceptance.md)

