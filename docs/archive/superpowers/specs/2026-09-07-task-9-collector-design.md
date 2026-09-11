# Task 9: bounded evidence collection

Implements task 9 of `docs/plans/2026-09-04-policy-kernel-implementation.md`
and section 7.6 of the extraction design. The user approved parallel work with
task 8; this branch owns the collector and the shared `Truncated` enum addition.

## Boundary and contract

`collect(CollectionRequest) -> Result<CollectedExecution, CollectionError>` runs
one selected argv and an explicit version-argument probe of the same resolved
executable. Requests deny unknown fields and validate positive bounded limits,
an absolute existing cwd, explicit environment names, repository/head and runner
identities, input digest, and relative artifact paths. Shell command strings and
known shell executables are rejected. This is a trusted-runner process launcher,
not a sandbox for arbitrary hostile executables or a policy selector.

`CollectedExecution` contains the existing `ExecutionEvidence` and a detailed
record. `command_digest` hashes the canonical command record, including exact
repository/head, identities, input digest, resolved executable, argv, cwd,
environment names, limits, version arguments and artifact paths. `output_digest`
hashes the canonical complete execution record, including byte-exact logs,
version-probe outcome, timestamps, exit code, status and artifact digests.
Environment values are never copied into metadata; child output is recorded
verbatim and may itself contain secrets. Input digests are supplied by the runner,
which remains responsible for authenticating the checked-out inputs.

Local library and CLI results always carry `LocalUntrusted`. JSON cannot request
CI or attested trust. Task 10 must authenticate runner context and bind records
into a bundle before treating an observation as authoritative.

## Execution and failure handling

Unix process groups and nonblocking stdout/stderr reads bound execution without
reader threads that can hang on inherited pipes. Clear the environment before
copying only explicitly allowlisted names; resolve PATH only from that allowlist,
or accept an explicit executable path. Close stdin. Each stream retains at most
its byte limit; observing one additional byte marks `Truncated`. Main execution
and version probe share one monotonic timeout budget. Timeout, truncation,
nonzero exit, signal, spawn failure, probe failure and artifact read failure can
never yield `Passed`. Kill the owned process group on completion and error, reap
the direct child, and close pipes. Process groups do not contain descendants that
deliberately create another session; deployment isolation belongs to the runner.

Artifact capture accepts at most 64 relative paths beneath cwd, regular files
only, and streams at most 16 MiB per file. Unix is supported initially; unsupported
platforms fail closed without executing. No new unsafe code is introduced.

## Shared changes and alternatives

Add only `ExecutionStatus::Truncated` to shared evidence contracts and map it to
`PK-EXEC-TRUNCATED` / inconclusive in pre-merge evaluation. Keeping detail in the
collector avoids changing every existing `ExecutionEvidence` literal and digest.
Embedding all fields in that shared struct would couple task 8 schema work and
existing fixtures unnecessarily. Mapping truncation to `Failed` would lose the
explicit state required by task 9. Task 8 schemas must be regenerated after this
enum addition is integrated.

## Verification

Exercise real executable fixtures for success, nonzero exit, spawn failure,
timeouts, output boundaries, both-stream pressure, inherited pipes, version
failure, environment isolation, cwd, shell rejection, binary logs, artifact
digests and CLI stdin/exit behavior. Check canonical bindings change when head,
argv or output changes. Run workspace tests, formatting, strict Clippy and diff
checks. No remote operations or main-branch integration are part of this task.
