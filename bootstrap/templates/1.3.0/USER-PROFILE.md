# Project-owned frontend profile integration

Release 1.3.0 accepts a user-owned frontend profile schema 4 with selection
schema 3. The profile, package manifests, lockfile, runtime files, and CI job
remain user-owned. Bootstrap installs only this guide and fixed inactive CI
examples; it does not install Node, npm, or project packages.

Validate the profile with `project-verification validate-profile PATH`. A
`migrate-profile PATH --target-schema 4` command prints a candidate to stdout;
review and adopt its new digest before preparing a fresh, head-bound RunRequest.
Run `project-verification prepare --input REQUEST` and then
`project-verification run --input REQUEST`. Direct results remain LocalUntrusted.

Project tool versions use explicit record, exact, or numeric_range acceptance.
`selection.provisioning` is optional. If supplied, the selected version must
match the observed tool. A declared `.node-version` must also match the
observation. CI prepares tools externally when provisioning is absent. Report
and npm audit contracts remain required even when version acceptance is record.

For removal, select the same profile path and `--release 1.3.0 --uninstall`.
Edited or missing source profiles cannot authorize deletion of user-owned files;
edited managed integration files conflict. The receipt grants no policy authority.
