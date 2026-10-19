# ADR: Kernel scope and compatibility preservation

Date: 2026-10-19. Decision: accepted product boundary and stage-1 sequence;
implementation planned. Baseline: `2ff9907bb35bd929e4ad94fd19f38e3041c075ae`.
The active landing is [Task 18](../../tasks/task-18/README.md); implementation
status belongs to [Current status](../../status.md).

## Context and accepted decisions

Native execution, environment compatibility and consumer-specific composition
have accumulated around the deterministic evaluator. Moving directories alone
would retain that maintenance burden. The approved scope-reduction plan narrows
the kernel product to deterministic evidence, scope and release decisions.

- Stop adding exact native tool combinations and bootstrap presets. Freeze
  ProjectProfile schemas 1–4, selection schemas 1–3 and bootstrap releases;
  do not delete them. Preserve recovery, security fixes and explicit consumer
  compatibility without promising broader native environment support.
- Adopt a small additive native-result façade:
  `ExpectedScopeV1` + `NativeRunV1` → `LocalScopeResultV1`, with conversion to
  existing evidence/core APIs. These are planned conceptual types, not available
  types or commands. Consumers should not have to assemble EvidenceBundles
  directly. Existing core contracts and policy-v3 review semantics remain intact.
- Preserve every existing required environment check in the first transition:
  npm/MSBuild and NuGet graphs, audits, actual tool versions, generated output,
  overrides and bundled-inventory exclusions. An optional adapter does not make
  a required policy check optional. Relaxation requires separate policy review.
- Proceed through stage 1, PRs 1–8, one at a time. This PR records decisions only;
  no code, schema, CLI, tests or TJERP execution changes are included.

## Ownership boundary

This table assigns target product responsibilities, not completed moves or named
maintainers. The present workspace and binaries stay usable during transition.

| Owner | Responsibilities retained or assigned | Boundary |
| --- | --- | --- |
| Kernel | `policy-contracts`, `policy-core`, advisory `policy-cli`, current `policy-ci`/`policy-controller` and `github-adapter`, GitHub action, schemas, xtask/mutation, trust/promotion documentation and Rust conformance tests | Keep authoritative observation, review, receipt and post-merge semantics; no controller redesign |
| Kernel, planned addition | Small native wire contract and typed verifier/assembler invoking existing core functions | No npm/MSBuild/framework model, raw report parsing, installation or reverse dependency on report adapters |
| Adapter product | Shared TRX/Vitest/Jest/Playwright readers and corpus; raw import frontend; `project-verification` compatibility runner; optional `evidence-collector`; environment validation and frozen bootstrap | Preserve parser versions/IDs, raw artifacts and failure behavior; collector use is not mandatory for native-result input |
| Adapter product | Generic native acceptance, example staging, bootstrap/process/lifecycle regression fixtures | Retain native CI coverage separately from kernel conformance; synthetic fixtures are not real consumer adoption evidence |
| TJERP | Catalog/AC/B/N approval, suites/profiles, exact selectors, source exclusions and split scope composition; `tjerp_adoption`, needed `tjerp_profile` tooling and consumer fault tests | Reuse parser/core libraries; never copy core decision logic |
| TJERP or consuming native CI | Native commands, dependency installation, servers/browser lifecycle, approved expected head/source inventory and artifact retention | Preserve existing safety obligations; no new central lifecycle engine or automatic adapter installation by kernel verification |

The generic negative-verdict helper may remain a small shared test utility.
TJERP-specific source/catalog rules move with the consumer, while generic hashing,
path validation and parsing belong to the shared importer/adapter. The
`native_tjerp` / `examples/tjerp-compatible` fixtures are synthetic: retain their
generic report/audit coverage as split-consumer conformance when separating them.

## Native façade guardrails

The façade verifies an already completed native run; it is not a command runner
or a trusted success boolean. Expected head, source-set digest, approved
requirements digest, required IDs/mappings, command/config/parser bindings and
freshness limits are independent consumer-approved inputs. They must not be
regenerated from the observed passing tests.

Observations bind run/execution/suite identity, actual head/source, argv/cwd,
times, exit/timeout/error, configuration, parser ID/version and raw artifact
byte hashes to normalized test identities and outcomes. The adapter checks local
artifact paths/bytes/hashes and reparses reports; the kernel verifies typed
consistency and calls existing core rules. A JSON-only verifier cannot claim to
have inspected raw files. Raw byte hashes and canonical typed hashes stay distinct.
Legacy approved digests are retained, never silently replaced by reserialization.

Missing or duplicate requirements/suites, unknown versions/IDs, stale or mixed-head
runs, unexecuted mappings, missing/malformed reports, inconsistent summaries and
unsupported skip/retry results fail closed. Failure logs and raw artifacts remain
available; incomplete output cannot become an empty success. Environment details
remain explicit required sensors or opaque artifacts, outside the minimal wire
schema. Strict versioning and conformance vectors must precede use.

Local results remain `LocalUntrusted`, with review unmet; scope satisfaction does
not imply `ReadyToMerge`. No synthetic provider/model/review may substitute for
the authoritative observer/controller path. Hashes do not authenticate producers,
and requirement-to-test mappings do not prove business adequacy. The planned
scope result is separate from pre-merge decisions; its exact CLI is future work.

## Supported compatibility paths

These paths remain supported within their existing validation limits. Freeze
does not broaden accepted versions or bypass policy-v3 reapproval or adequacy
requirements. ProjectProfile is distinct from core SensorProfile v2.

| Existing surface | Preservation requirement |
| --- | --- |
| ProjectProfile 1 / no selection | Existing 1.0.0–1.0.2 profiles and frozen assets retain parsing/serialization |
| ProjectProfile 2 / selection 1 | Existing selected tool/path contracts and custom bootstrap 1.1.0 remain |
| ProjectProfile 3 / selection 2 | Existing explicit .NET/frontend/fullstack consumer graphs and custom bootstrap 1.2.0 remain |
| ProjectProfile 4 / selection 3 | Existing project-owned frontend versions/workspace contracts and custom bootstrap 1.3.0 remain; this is not a new .NET/fullstack contract |
| Bootstrap presets | Existing `react-typescript`, `dotnet`, `fullstack` preset assets at 1.0.0, 1.0.1, 1.0.2 and 1.1.0 remain byte-preserved |
| Bootstrap custom profiles | Release/schema pairs 1.1.0/2, 1.2.0/3 and 1.3.0/4 remain separate from presets; receipt/journal recovery depends on exact release bytes |

The [profile](../../../crates/project-verification/src/profile.rs),
[selection](../../../crates/project-verification/src/selection.rs) and
[bootstrap templates](../../../bootstrap/src/templates.rs) implementations define
the accepted combinations. Existing detailed usage remains in
[tool selection](../../integrations/tool-selection.md),
[consumer schema 3](../../integrations/consumer-schema3.md),
[bootstrap](../../integrations/bootstrap.md) and
[report formats](../../integrations/report-formats.md).

| Binary / entry point | Existing CLI paths to preserve |
| --- | --- |
| `project-verification` (`crates/project-verification`) | `validate-profile FILE`, `schema`, `prepare --input FILE`, `run --input FILE`, `migrate-profile FILE [--target-schema 4]`, `ci-inputs FILE [--github-output FILE]`, `provision-npm FILE --prefix DIR` |
| `policy-bootstrap` (`bootstrap`) | `inspect`, `plan`, `apply --plan FILE`, `recover --target DIR`; uninstall remains `--uninstall` on selection/plan followed by apply, not a new subcommand |
| `policy-acceptance` (`acceptance`) | Generic `stage` and `run` with their existing options |
| `tjerp_adoption` / `tjerp_profile` (`acceptance/examples`) | Existing example build and invocation paths until reviewed consumer adoption; the former's `run` is distinct from `policy-acceptance run` |
| `evidence-collector` (`crates/evidence-collector`) | Existing request-file/stdin collection CLI and bounded execution behavior |
| `policy-cli` (`crates/policy-cli`) | `evaluate-pre INPUT`, `inspect INPUT`; advisory evaluation remains 0 ready, 2 blocked, 3 inconclusive, 64 invalid input |
| `policy-ci`, `policy-controller` (`crates/policy-ci`) | Existing pre/post-merge and authenticated controller interfaces, plus owner-installed binary use by `action/action.yml` |
| `xtask` (`xtask`) | Schema generation, mutation suite/repairs/checks, residue/public-API checks and rollback drill |

`project-verification` retains 0 success, 2 sensor non-pass and 64 input/execution
error. Bootstrap remains Unix-only (64 otherwise), with existing apply/recover/
uninstall protections against symlinks, concurrent changes and user modifications.
The façade's future exit contract must not change existing CLI meanings.

Default debug build paths remain `target/debug/<binary>` and
`target/debug/examples/{tjerp_adoption,tjerp_profile}` (under the selected target
directory when overridden). TJERP's approved path remains the pinned
`4df10fa44f3bce79c0613bf8e918cd2d75222d68` build of `project-verification` and
`tjerp_adoption`, with approved binary hashes and catalog inputs. Updating kernel
main is not consumer reapproval. See the [adoption procedure](../../integrations/tjerp-adoption.md)
and [P8/P9 record citations](../../status.md#tjerp-p8-adoption-and-p9-first-actual-dry-run).

## Frozen and eventual retirement candidates

New presets and exact native combinations stop now. Legacy profiles/selections,
bootstrap releases, provisioning and `ci-inputs` remain compatibility paths.
Their eventual retirement, unused validators and unneeded consumer examples are
candidates, not removals authorized by this ADR. Preserve existing security and
recovery behavior and rollback artifacts. No support deadline is set here.

## Stage 1 sequence

Only PR 1 is delivered by this change. Later rows are ordered work units, each
reviewed and validated independently. TJERP work units are local consumer changes
because its observed repository has no remote; these are not kernel GitHub PRs.

| Unit | Planned change | Compatibility / exit condition |
| --- | --- | --- |
| PR 1 | Record boundary, ownership and preservation lines | Documentation only; no TJERP changes |
| PR 2 | Extract report readers/tests/corpus into a small Rust adapter crate | Preserve old module re-exports, parser versions, normalized IDs, serialization, limits and failure verdicts; existing TJERP APIs still build |
| PR 3 | Add minimal native wire types and canonical corpus | No executor/importer/environment enums; strict versions and required sets; separate schema, unchanged existing five schemas and approved core digests |
| PR 4 | Add adapter raw/subject importer | Validate path/size/hash/head/source/run/time and reparse; retain failures; no execution/tool selection or scope-success API yet |
| PR 5 | Add kernel typed verifier/assembler and advisory scope command | Reuse core, preserve pre-merge CLI, separate expected/observed data, downgrade local trust; compare legacy/new non-pass reasons; no kernel dependency on raw parsers |
| PR 6 | Copy/review TJERP harness as consumer-owned Rust tooling | Pin existing `4df10fa` library/parser API; keep kernel implementation and old operational command until new hashes approved; synthetic tests are not a new native run |
| PR 7 | Adopt and pin TJERP-owned harness | Fresh baseline/negative/recovery with preserved scope/requirements, approved revision/binary hashes; retain old artifact/command rollback; PV still executes native commands |
| PR 8 | Remove kernel TJERP product code after PR 7 evidence; generalize fixtures | Preserve generic report/audit coverage, pinned consumer dependency and historical records; integration stub/LAT anchors, crate graph, workspace and mutation candidate baseline checked |

PRs 9–11 (backend/frontend native adoption and environment boundary), PRs 12–13
(workspace/repository separation) and PR 14 (compatibility closure) are later
stages, outside this change and the approved stage-1 delivery scope. Stage 1 is
an ownership milestone; it does not itself prove native consumer simplification.

## Common completion and compatibility rules

Every unit must preserve the approved execution path or supply adoption evidence
and a reproducible rollback artifact. New binaries/configuration/catalog digests
require explicit consumer approval and fresh records; do not repackage P9 evidence
as a new run. Preserve backend/frontend source/head agreement, discovery-to-final
ID binding, required audit coverage and `bundled, not individually audited` labels.

Run tests appropriate to changed Rust/API/corpus, CLI/serialization compatibility
and workspace/native matrices. Mechanical moves must not hide semantic changes
or leave a native CI coverage gap. Review mutation source pins/repairs when pinned
core/controller/contracts change. A final promotion candidate needs new exact-SHA
source-manifest strict mutation evidence; prior results cannot authorize it.

Keep README languages together, current status in `docs/status.md`, Task landings
and LAT links current, and historical records unchanged. Check relative Markdown
links, `lat check` and `git diff --check`; after source changes sync/query CodeGraph
where an index exists. This documentation-only PR additionally runs workspace
tests, formatting and strict Clippy, and observes hosted CI before delivery.
Those checks do not establish authoritative CI decisions, protected merges or
production promotion.

## Open decisions

Adapter repository maintainer, release location and support level remain open for
stage 3. A separate adapter workspace/repository is a target boundary, not a
promise of an operated release service; current packages are `publish=false`.
Immutable revisions or checksummed artifacts and cross-product conformance are
needed before external distribution, without permanent local absolute paths.

Compatibility end conditions remain open for PR 14: known-consumer migration,
receipt/recovery support window, security patch scope and rollback retention need
an explicit decision. Evidence-based closure is the recommendation, not an
approved deletion date. No installer asset, reader or rollback artifact is
removed by the present decision.
