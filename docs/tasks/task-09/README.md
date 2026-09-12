# Task 09 — Bounded evidence collection

## Purpose

Collect explicit-argv execution evidence with bounded output and artifact digests.

## Current status

Implemented.

## Evidence and limits

Collector output remains LocalUntrusted until the trusted CI adapter validates its provenance.

The collector accepts a generic explicit-argv executable contract. It does not
load or require global agent plugins, skill directories, orchestration state, or
agent configuration. A caller-provided executable and any credentials or report
artifacts named by that integration must still exist; this independence is not a
filesystem or process sandbox.

The version probe runs after a successful main command and after an ordinary
nonzero exit with an exit code and no collection error. It is skipped for
incomplete or damaged execution such as spawn failure, timeout, truncation, or
signal exit. Probe success never upgrades a failed main command.

## Related implementation and references

- [2026-09-07-task-9-collector-design.md](../../archive/superpowers/specs/2026-09-07-task-9-collector-design.md)
- [2026-09-07-task-9-collector.md](../../archive/superpowers/plans/2026-09-07-task-9-collector.md)
- [2026-09-07-task-9-collector-review.md](../../archive/reviews/2026-09-07-task-9-collector-review.md)
- [External reviewer integration (historical `agent-skills.md` path)](../../integrations/agent-skills.md)
