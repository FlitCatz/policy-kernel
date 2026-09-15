# Task 03 — Canonical bundle integrity and trust validation

## Purpose

Bind evidence bundles through canonical digests and fail-closed validation.

## Current status

Implemented, including policy schema 2 required knowledge in
[Task 15](../task-15/README.md).

## Evidence and limits

Integrity validation proves content consistency, not producer identity or release authority.
Policies can declare required knowledge kind/source and applicability. Undeclared
legacy policies still allow empty knowledge. Required evidence must match head,
source set and trusted provenance; local indexes alone do not satisfy it.

## Related implementation and references

- [2026-09-04-policy-kernel-implementation.md](../../archive/plans/2026-09-04-policy-kernel-implementation.md)
- [trust-model.md](../../trust-model.md)
