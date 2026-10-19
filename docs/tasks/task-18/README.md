# Task 18 — Kernel scope reduction

The [accepted scope ADR](../../archive/specs/2026-10-19-scope-reduction-adr.md)
records the product boundary, target ownership, frozen compatibility paths and
ordered stage-1 PRs 1–8. PR 1 recorded the decision. The parser extraction merged
in [PR 23](https://github.com/FlitCatz/policy-kernel/pull/23). The additive
[native contract](../../integrations/native-contracts-v1.md) now provides v1 types,
structural validation, three crate-owned schemas and a conformance corpus.
The PR 4 candidate adds `policy-native-importer`: a Unix adapter library checking
raw paths/limits/hashes, independent subject/run/scope/time expectations and
recomputed report observations. Fresh output directories retain failure records
and bounded raw copies, with an atomic persistence marker. Scope decisions,
core evidence conversion and ownership transfers remain planned. The historical ADR retains its original planning context.

New exact native combinations and bootstrap presets stop. Existing profile
schemas 1–4, selection schemas 1–3, bootstrap releases and required environment
checks remain supported within their current limits. TJERP is unchanged.
Adapter maintenance/support and PR 14 compatibility end conditions remain open.

Follow [Current status](../../status.md) for implementation and evidence claims.
The ADR's stage-1 sequence and common completion rules govern later work; this
landing does not establish native adoption, an authoritative decision or promotion.
