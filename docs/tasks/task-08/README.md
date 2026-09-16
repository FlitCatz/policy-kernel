# Task 08 — Advisory CLI and generated schemas

## Purpose

Provide the local policy CLI and deterministic JSON Schema generation.

## Current status

Implemented.

## Evidence and limits

The CLI is local and advisory; its output is not a required GitHub check.

## Related implementation and references

Policy schema 2 adds optional required-knowledge declarations; schema/Rust parity
and legacy reader/digest behavior are covered by [Task 15](../task-15/README.md).

- [2026-09-08-task-8-cli-schema.md](../../archive/superpowers/plans/2026-09-08-task-8-cli-schema.md)
- [README.md](../../../README.md)
