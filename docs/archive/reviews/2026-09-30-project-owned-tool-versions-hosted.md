# Task 17 hosted candidate observation — 2026-09-30

Draft [PR #7](https://github.com/FlitCatz/policy-kernel/pull/7) was opened
against `main` for feature head
`f75796a6a2bf26ca781df949aac7be03788e3567`. The following GitHub Actions
results were observed for that head; they are historical facts for this exact
candidate, not an authorization to promote a later revision.

- [Native consumer compatibility acceptance, run 36654808707](https://github.com/FlitCatz/policy-kernel/actions/runs/36654808707): `success`. Its project-owned frontend versions, React SSR/Jest alternate Node/npm, native React, native .NET and native fullstack jobs all completed successfully. The project-owned job uploaded artifact `project-owned-versions-cdc675d5f97e2fa08372fbf4e644cc437aae6403` (artifact ID `11071729113`, ZIP digest `sha256:f7a99ec51a582d3925c26967359f12a46c8d358744d5d20d102ec4913d32276a`).
- [Release verification, run 36654808691](https://github.com/FlitCatz/policy-kernel/actions/runs/36654808691): `success` across Rust 1.85.0 and 1.95.0 on Ubuntu and macOS, and the verification job.
- [Task 4 sensors, run 36654808692](https://github.com/FlitCatz/policy-kernel/actions/runs/36654808692): `success`.
- Task 10 live API smoke, run 36654808688: `skipped` by its workflow conditions; no live smoke result is claimed.

These observations establish hosted execution of the Task 17 candidate job.
They do not by themselves establish an authoritative required-check decision,
independent review, a protected-branch merge, shadow operation, rollback
observation or production promotion. The PR remained a draft when recorded.
