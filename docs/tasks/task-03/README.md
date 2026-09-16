# Task 03 — Canonical bundle integrity and trust validation

## Purpose

Bind evidence bundles through canonical digests and fail-closed validation.

## Current status

Core implemented; mandatory knowledge/index enforcement remains pending in
[Task 15](../task-15/README.md).

## Evidence and limits

Integrity validation proves content consistency, not producer identity or release authority.
Present knowledge evidence is validated, but the current policy contract cannot
require its presence.

## Related implementation and references

- [2026-09-04-policy-kernel-implementation.md](../../archive/plans/2026-09-04-policy-kernel-implementation.md)
- [trust-model.md](../../trust-model.md)
