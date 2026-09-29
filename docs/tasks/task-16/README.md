# Task 16 — TJERP-shaped consumer compatibility

Profile schema 3/selection schema 2 adds exact .NET 11 RC1 and Playwright 1.63
producer contracts, explicit suite-to-graph composition, bounded inherited .NET
frameworks and custom-only bootstrap 1.2.0. Older contracts and frozen assets keep
their meaning. Direct evidence remains LocalUntrusted.

- [Contract and usage](../../integrations/consumer-schema3.md)
- [Approved design](../../superpowers/specs/2026-09-28-tjerp-consumer-compatibility-design.md)
- [Implementation plan and ledger](../../superpowers/plans/2026-09-28-tjerp-consumer-compatibility.md)
- [Local verification record](../../archive/reviews/2026-09-29-tjerp-consumer-compatibility.md)

Native synthetic consumer verification is distinct from installation into TJERP.
TJERP still needs a reviewed profile, dependency lockfiles, explicit NuGet audit
configuration and test/report mappings. Hosted evidence and production promotion
are not established by this task's local runs.
