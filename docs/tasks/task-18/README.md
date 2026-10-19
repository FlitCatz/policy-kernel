# Task 18 — Kernel scope reduction

The [accepted scope ADR](../../archive/specs/2026-10-19-scope-reduction-adr.md)
records the product boundary, target ownership, frozen compatibility paths and
ordered stage-1 PRs 1–8. PR 1 is documentation only; the additive native façade,
parser extraction and ownership transfers are planned, not implemented here.

New exact native combinations and bootstrap presets stop. Existing profile
schemas 1–4, selection schemas 1–3, bootstrap releases and required environment
checks remain supported within their current limits. TJERP is unchanged.
Adapter maintenance/support and PR 14 compatibility end conditions remain open.

Follow [Current status](../../status.md) for implementation and evidence claims.
The ADR's stage-1 sequence and common completion rules govern later work; this
landing does not establish native adoption, an authoritative decision or promotion.
