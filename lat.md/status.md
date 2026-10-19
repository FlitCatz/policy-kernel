# Current status

The current-state authority is [docs/status.md](../docs/status.md); this page only summarizes it and must not claim more.

[Task 18](../docs/tasks/task-18/README.md) records the accepted scope decision.
New exact native combinations/presets stop. Parser extraction merged in PR 23;
[[native-contracts]] supplies v1 types and the importer merged in PR 25.
The PR 5 candidate adds kernel-owned core evidence assembly and a local advisory
scope command (0/2/64). Trust remains local-untrusted, review unmet; no pre-merge
decision is produced. Stage-1 PRs 6–8 remain planned. Profiles, bootstrap and required environment checks are preserved;
TJERP is unchanged. Adapter support and compatibility end conditions remain open.
See [[kernel#Scope boundary]]. No adoption or promotion is established by this decision.

Policy v3 binds approving-review quorum and allowed providers. Legacy policy
digests remain valid, but readiness and shipping require explicit v3 reapproval.
Consumer profiles are unchanged; reapproval and promotion are not established.
See [[kernel#Review consensus and waivers]].

Third-party npm bundle inventory requires registry-tarball/sha512-pinned parents
and records `bundled, not individually audited`, including absent optional
platform parents. The native oxide fixture satisfies local scope; TJERP P8
adoption and re-verification completed at `4cf51da`. See [[kernel#Npm workspace graph]].

Reviewed npm root overrides bind exact versions into the profile and verify
affected lock resolutions. Native Node 26.12.0/npm 11.20.0 audits detect vulnerable
and fixed override versions. TJERP P8 adopted `4df10fa` and re-verified frontend
preparation; promotion remains unestablished. See [[kernel#Npm workspace graph]].

P7 adds the [[tjerp]] local harness and synthetic FI-00–FI-09 protocol tests.
It binds approved catalogs, raw identities and split source/head/profile results
through existing APIs. TJERP completed P8 profiles/run procedure (`1fd96a7`,
merged as `4cf51da`) and P9 execution (`3556ce9`, merged as `6e688e9`) on
2026-10-17. FI-00 and recovery on `f926418` satisfied local scope; FI-01–FI-09
were rejected as intended. Evidence remains LocalUntrusted and the review gate
is unmet: no independent review or pre-merge decision, Trusted evidence,
authoritative decision, protected merge or production promotion is established.
Full revision and catalog-digest citations are in the current-status authority.

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
[[tasks]] maps evidence to Tasks 1–18, and [[trust-boundaries]] explains why
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
