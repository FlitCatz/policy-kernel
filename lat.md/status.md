# Current status

The current-state authority is [docs/status.md](../docs/status.md); this page only summarizes it and must not claim more.

Third-party npm bundle inventory requires registry-tarball/sha512-pinned parents
and records `bundled, not individually audited`, including absent optional
platform parents. The native oxide fixture satisfies local scope; TJERP P8
adoption and re-verification remain pending. See [[kernel#Npm workspace graph]].

Reviewed npm root overrides bind exact versions into the profile and verify
affected lock resolutions. Native Node 26.12.0/npm 11.20.0 audits detect vulnerable
and fixed override versions. Local scope is satisfied; TJERP adoption and
promotion remain unestablished. See [[kernel#Npm workspace graph]].

P7 adds the [[tjerp]] local harness and synthetic FI-00–FI-09 protocol tests.
It binds approved catalogs, raw identities and split source/head/profile results
through existing APIs. LocalUntrusted remains; real P8 profiles and P9 execution
are pending, with no adoption, authoritative decision or promotion claim.

P2 adds explicit graph opt-in for bounded compiler-generated output, with XML,
command/environment and filesystem checks. The offline Mapperly fixture does not
establish TJERP adoption, authoritative CI or trust. See [[kernel#Compiler-generated output]].
The current-status authority records local scope satisfied for the cached-package
native proof, including the empty-output rejection.

P3 makes empty adequacy an explained non-pass, rejects empty schema 3/4 consumer
policies and requires negative cases to map to executed tests. Frozen parsing
is preserved; local tests do not establish adoption or promotion. See [[kernel#Test adequacy]].

Native fixture dependencies use Jest 30.5.2 with scoped `js-yaml` 4 and
parser 8.42.0 with scoped `typescript-estree` 8.48.0 overrides. Frozen bootstrap
pins remain unchanged; this refresh alone makes no hosted or promotion claim.

P1 adds explicit npm members under one root graph and audit. Native
Node 26.12.0/npm 11.20.0 fixtures satisfy the local scope; evidence remains
LocalUntrusted and does not establish TJERP adoption or promotion.
See [[kernel#Npm workspace graph]] and the current-status authority.

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
