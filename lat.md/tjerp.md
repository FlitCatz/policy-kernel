# TJERP local acceptance

P7 connects split local profiles to approved catalogs and raw reports. Its synthetic fixtures exercise protocol rejection; actual TJERP profiles and the first native dry-run remain P8/P9.

The contract is [TJERP adoption](../docs/integrations/tjerp-adoption.md).
Current status belongs to [[status]], and core rules remain in [[kernel]].

## Catalog and identities

Catalog-derived AC/B/N lists must be nonempty and exactly match both catalog summaries and profiles. Fresh report parsers supply normalized IDs; historical snapshot hashes cannot substitute for them.

[[acceptance/src/tjerp/catalog.rs#check_catalog]] checks parent IDs, uniqueness,
negative membership, suites and the externally approved catalog file digest.
[[acceptance/src/tjerp/reports.rs#link_report]] preserves raw identities in parser
order. [[acceptance/src/tjerp/reports.rs#resolve]] rejects missing, ambiguous and
non-passing selectors while retaining flat TRX theory rows.
TRX raw names are read only from the parser's exact XML namespace; foreign
elements cannot attach a different name to a normalized ID.

## Source and split binding

Both requests bind one actual Git archive and sorted source inventory. Both results retain LocalUntrusted, and core sensor, bundle and advisory provenance checks remain mandatory.

[[acceptance/src/tjerp/source.rs#git_inventory]] compares scoped Git blobs with
archive bytes and rejects extra source. [[acceptance/src/tjerp/mod.rs#requests]]
builds typed request digests. [[acceptance/src/tjerp/mod.rs#evaluate_pair]]
checks bindings and calls the core; it creates no review or pre-merge decision.
[[acceptance/src/tjerp/source.rs#execute]] preserves raw reports and requirement
outcomes even if collection returns no normalized result.

## Synthetic fault matrix

FI-00–FI-09 verify positive composition and specific non-pass reasons using synthetic Rust reporters, temporary Git commits and protocol mutations. They do not establish actual TJERP source-fault detection or adoption.

The matrix covers contract/unit/FSD failures, missing reports, stale heads,
provider reuse, empty policy, missing/unexecuted mappings and claimed trust.
Additional cases reject list shrinkage, skipped/zero/retried/malformed reports,
renamed/ambiguous selectors, wrong checkout paths, extra source and symlinks.
