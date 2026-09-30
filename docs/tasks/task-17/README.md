# Task 17 — Project-owned frontend tool versions

The [design](../../archive/specs/2026-09-29-project-owned-tool-versions.md)
and [implementation plan](../../archive/plans/2026-09-29-project-owned-tool-versions.md)
define this scope. The current code adds frontend profile schema 4 / selection
schema 3, adapter contract IDs, optional provisioning, explicit migration,
custom bootstrap 1.3.0 and an opt-in native conformance harness. The
[current status](../../status.md) and
[local execution record](../../archive/reviews/2026-09-29-project-owned-tool-versions-local.md)
and [follow-up](../../archive/reviews/2026-09-30-project-owned-tool-versions-follow-up.md)
identify which checks were actually run.
The [hosted candidate record](../../archive/reviews/2026-09-30-project-owned-tool-versions-hosted.md)
records exact-head PR #7 workflow observations separately.

The new path accepts React frontend npm graphs with Vitest, Jest and Playwright
JSON plus process and npm audit contracts. It keeps actual tool observations,
package/lock agreement, report failure handling and LocalUntrusted results.
.NET and fullstack remain on their existing schema 2/3 contracts. It does not
establish production promotion, protected-branch merge or an authoritative CI
decision. See [selection](../../integrations/tool-selection.md),
[bootstrap](../../integrations/bootstrap.md),
[report formats](../../integrations/report-formats.md) and
[acceptance](../../integrations/acceptance.md) for usage and limits.

Before a consumer version change is adopted, run the ignored Rust native test
against its supplied candidate fixture with a fresh output path. Retain the output's
request, result, original reports, audit observations, exact versions, lockfile,
head and harness source binding. The PR #7 candidate has a recorded hosted
run; later heads need their own observation. Separate operational approval
remains necessary.
