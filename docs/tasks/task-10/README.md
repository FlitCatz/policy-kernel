# Task 10 — GitHub Actions authoritative boundary

## Purpose

Authenticate GitHub workflow observations and publish the dedicated policy check.

## Current status

Implemented; production evidence remains incomplete.

## Evidence and limits

The implementation and smoke validation are recorded, but live required-check and protected-merge evidence must be collected for the chosen candidate.
[Task 15](../task-15/README.md) adds the
[authenticated controller reference](../../../action/controller.md), cryptographic
fixture tests and version 2 workflow-source binding. Actual isolated controller
and independently observing producer services remain deployment prerequisites.
The [Task 15 hosted record](../../archive/reviews/2026-09-16-task-15-hosted-verification.md)
includes current GitHub Actions observations and protection capability reads.
Its Task 10 live smoke was skipped because the existing branch gate did not apply;
the successful Release/native workflows do not supply live policy publication evidence.

## Related implementation and references

- [2026-09-08-task-10-validation.md](../../archive/reviews/2026-09-08-task-10-validation.md)
- [2026-09-08-live-promotion-readiness.md](../../archive/reviews/2026-09-08-live-promotion-readiness.md)
- [README.md](../../../action/README.md)
- [promotion.md](../../promotion.md)
