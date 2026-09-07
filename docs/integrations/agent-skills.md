# Collecting evidence from replaceable reviewer executors

A consuming workflow may combine an external review skill with OMP, `agy`, or
another executor. The workflow selects the policy-declared argv and supplies the
provider, model and execution IDs. `evidence-collector` runs that selection once
and records observations. It never embeds `gemini-reviewer`, selects a provider
or model, manages login, retries, compares failure signatures, owns repair budgets,
chooses debugger escalation, or treats model prose as provenance.

An isolated debugger's repair becomes new modification evidence. It must be
reviewed by a provider and execution independent of every contributor, including
the debugger. A collector result is execution evidence, not a semantic review or
a gate decision. Report normalization belongs to the consuming integration.

## Invocation

Run `evidence-collector request.json` or pipe JSON to `evidence-collector -`.
The CLI writes one JSON object to stdout and diagnostics to stderr. Exit codes
are 0 for passed collection, 2 for a non-success observation, and 64 for invalid
input or inability to construct an evidence result. Input JSON is limited to 1 MiB.

An example request (replace paths, bindings and identities with runner values):

```json
{
  "argv": ["/opt/review/bin/reviewer", "review", "--head", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"],
  "cwd": "/workspace/project",
  "environment_allowlist": ["REVIEW_API_KEY"],
  "timeout_ms": 120000,
  "stdout_limit_bytes": 1048576,
  "stderr_limit_bytes": 65536,
  "version_args": ["--version"],
  "repository": "owner/project",
  "head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "provider": "workflow-selected-provider",
  "model": "workflow-selected-model",
  "execution_id": "unique-execution-identifier",
  "input_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "artifact_paths": ["review.json"]
}
```

The Rust API is `collect(CollectionRequest::try_from_json(json)?)`. JSON
deserialization also validates the request. Unknown fields, including `trust`,
are rejected. Shell strings and known shell executables are rejected; argv
arguments remain literal. No shell expands variables, globs or command substitutions.

The executable is resolved once. A bare executable name requires `PATH` in the
explicit environment allowlist; otherwise use an absolute path or a path relative
to cwd. Only allowlisted environment variables present in the collector process
are copied to the child. Metadata records the sorted allowlist names, never their
values. Logs are byte arrays and preserve non-UTF-8 output; programs can print
credentials, so workflows must control log access and configure safe output.

## Limits and incomplete outcomes

Linux and macOS are supported. Other platforms fail before execution. The main
command and version probe share one monotonic timeout, at most one hour. A version
probe uses the same resolved executable and its supplied `version_args`, cwd and
environment; it runs only after the main command passes. A failed probe makes
the whole collection non-success. A skipped probe is recorded explicitly.

Each stdout/stderr limit is 1 byte to 16 MiB. Exactly reaching a limit is allowed;
observing any additional byte produces `truncated` and terminates the process
group. Timeouts produce `timed_out`. Nonzero exits, signals, spawn failures and
artifact failures cannot pass. Timeout/truncation remains explicit even if cleanup
also reports an error. The result preserves any captured prefix and diagnostic.

The collector kills the owned process group and reaps its direct child. It uses
nonblocking pipe reads so descendants holding stdout/stderr cannot hang reader
threads. It retains the group leader until the last signal to avoid PID reuse.
Process groups are not a security sandbox: a program can deliberately create a
new session or change credentials. The trusted runner must restrict executables
and use OS/container isolation for untrusted code. Filesystem operations on a
stalled remote filesystem and uninterruptible kernel work are not hard realtime
bounded by the user-space deadline.

Artifacts are at most 64 distinct relative paths under cwd, regular files only,
at most 16 MiB each. Every path component is opened relative to its parent
directory descriptor without following symlinks. Contents are streamed into SHA-256; collection errors remain
in the result. The workflow must create artifacts in an isolated checkout and
prevent concurrent modification while they are captured.

## Verification and trust

The output has `evidence` (`ExecutionEvidence`) and `record` (detailed collection).
Verify `canonical_sha256(record.command) == evidence.command_digest` and
`canonical_sha256(record) == evidence.output_digest` using policy-core canonical
JSON. The command record binds repository/head, input digest, identities, argv,
resolved executable, cwd, environment names, limits, version arguments and
artifact paths. The complete record additionally binds byte logs, process status,
exit codes, timestamps, version output, artifact digests and capture errors.
Store the record alongside its evidence; discarding it loses the data needed to
audit these bindings.

All direct library and CLI results are `local_untrusted`. Provider/model names,
repository/head and input digests are supplied claims; the collector does not
infer them from prose or authenticate the checkout. The task 10 trusted adapter
must authenticate workflow/runner identity, verify repository/head and input
bindings against the actual checkout, verify record digests and outcomes, and
build the evidence manifest before granting CI-observed or attested provenance.
Neither JSON fields nor environment variables upgrade collector trust.

## Task 8 integration

Task 9 adds `ExecutionStatus::Truncated` (`"truncated"`) and maps it to
`PK-EXEC-TRUNCATED` / inconclusive in pre-merge evaluation. Existing
`ExecutionEvidence` fields and old fixture serialization stay unchanged.
After integrating task 9, regenerate task 8's schemas with `cargo run -p xtask --
schema`, commit the enum change in generated schemas, and rerun CLI/schema tests.
Readers predating this enum addition reject the new status; this is not a claim
of old-reader compatibility or a frozen distribution protocol.

Collector fixtures use `@fixture` and `@workspace` tokens that integration tests
replace with a freshly compiled test executable and temporary working directory.
