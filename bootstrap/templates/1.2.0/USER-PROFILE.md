# Explicit consumer profile integration

Release 1.2.0 accepts a user-owned schema-3 project profile with selection schema 2.
It manages only this guide and fixed, inactive CI integration examples. Native
configuration, the source profile and test files remain user-owned.

Validate the source with `project-verification validate-profile PATH`. Prepare a
fresh RunRequest bound to the actual HEAD, source inventory and profile digest;
run `project-verification prepare --input REQUEST` followed by
`project-verification run --input REQUEST`. Schema 3 declares suite roles and
exact producer contracts explicitly. A development RC selection is not production
approval. All direct observations remain LocalUntrusted.

Use the same `--profile-file PATH --release 1.2.0 --uninstall` selection for
removal. Edited or missing source profiles do not authorize deleting native files.
Edited managed integration files conflict. The receipt grants no policy authority.

Provision selected tools before integrating the examples into a trusted CI job.
Bootstrap performs no package restore, authentication setup or branch protection.
