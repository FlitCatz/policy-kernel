# Current status

The current-state authority is [docs/status.md](../docs/status.md); this page only summarizes it and must not claim more.

It identifies the implemented scope, dated verification evidence, and conditions
not demonstrated in production. [[overview]] explains the repository's role,
[[tasks]] maps evidence to Tasks 1–17, and [[trust-boundaries]] explains why
local evidence cannot by itself authorize a release.

[Task 17](../docs/tasks/task-17/README.md) adds project-owned frontend tool
versions (profile schema 4 / selection schema 3). Its hosted observations are
bound to a single feature head, not to a production promotion.

On 2026-09-30 a codebase review closed several fail-open paths, bootstrap files
began to follow the umask, sensor evaluation joined the strict mutation gate, and
four design decisions were applied: location-independent configuration bindings,
head-bound execution and adequacy evidence, GitHub cross-checked merge protection,
and segment-local `*` in path globs. [[kernel]] anchors these rules to code.
The dated records are the
[review remediation](../docs/archive/reviews/2026-09-30-codebase-review-remediation.md),
[remaining area review](../docs/archive/reviews/2026-09-30-remaining-area-review.md)
and [design decision changes](../docs/archive/reviews/2026-09-30-design-decisions.md).
They are local evidence; hosted CI results for the resulting main commits are
reported in the current-state authority. Live GitHub Check publication and
operational promotion remain unverified.
