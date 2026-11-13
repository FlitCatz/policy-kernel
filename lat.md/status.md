# Current status

The current-state authority is [docs/status.md](../docs/status.md); this page only summarizes it and must not claim more.

A 2026-11-12 [decision record](../docs/decisions/2026-11-12-local-runner-attestation-adr.md)
plans signed local-runner attestation as a weaker trust class. The class and
authenticated acceptance remain unimplemented; sibling structural contracts are
described below. See [[trust-boundaries#Planned local-runner attestation]].

[Task 18](../docs/tasks/task-18/README.md) records the accepted scope decision.
New exact native combinations/presets stop. Parser extraction merged in PR 23;
[[native-contracts]] supplies v1 types and the importer merged in PR 25.
Plan PR 5 added kernel-owned core evidence assembly and a local advisory
scope command (0/2/64). Trust remains local-untrusted, review unmet; no pre-merge
decision is produced. Plan PRs 6–7 transferred and adopted the TJERP harness;
PR 8 completes stage 1
in implementation, removing product code here and retaining generic split-consumer
conformance. TJERP main `3c62b9e` owns `tools/policy-adoption`, pinned to `4df10fa`.
This does not assert that the PR has merged.
Profiles, bootstrap and required environment checks are preserved;
TJERP is unchanged. Adapter support and compatibility end conditions remain open.
See [[kernel#Scope boundary]]. No adoption or promotion is established by this decision.

[[native-contracts#Local-runner attestations]] adds structural statement/envelope
contracts and scope-template digests. It does not establish signature verification,
authenticated local-runner acceptance or promotion; existing scope v1 stays unchanged.

Native distribution adds a Unix importer command and a version-tag workflow for
both commands on macOS arm64 and Linux x64, with SHA-256 and source-commit assets.
The owner must cut the first release after merge; no published release or consumer
adoption is established. See [[native-contracts#Native command distribution]].

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

The former P7 [[tjerp]] harness and its product-specific assertions now belong
to TJERP; the pinned dependency is unaffected.
TJERP completed P8 profiles/run procedure (`1fd96a7`,
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

Native presets select bootstrap 1.1.1 with Vite 7.3.7 / Vitest 4.1.12;
1.1.0 bytes remain frozen. Fixture-scoped minimatch/glob overrides remain;
the historical Jest deepmerge failures preceded managed execution retirement. See [[kernel#Bootstrap installer]]
and the current-status authority; no promotion is established.

The split-consumer frontend separately selects Vite 6.4.5 / Vitest 4.1.12
and locks source-map-js 1.2.2. Its selected npm 10.9.3 audit reports zero
vulnerabilities; historical Jest audit failures are preserved in their record.
See the current-status authority and [[kernel#Split-consumer conformance]].

[[kernel#Alternate frontend conformance]] now uses Vitest; managed Jest runner/CI
guarantees have ended. Reader/enums, importer, profile-only corpus, frozen templates
and receipt recovery/uninstall remain compatible. Retry zero does not recover
Vitest retry history. The project-owned native job also executes a schema-two
Vitest profile migrated to four with record acceptance and no provisioning.
See the current-status authority.

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
