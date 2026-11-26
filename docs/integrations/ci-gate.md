# Advisory GitHub Actions gate

Run the distributed `policy-cli` on a machine with authenticated `gh`. Grant the
caller read access to the repository's contents, pull requests, and Actions.
The gate uses GitHub REST GET requests with those credentials.

```console
policy-cli gate --repo owner/repo --pr 42 --policy .policy-kernel/gate.json --output gate-42.json
```

The policy path is repository-relative. The gate reads it at the PR's observed
base SHA, including when the PR changes that file. The default output is
`gate-evidence.json`. Choose a new path on each run. The report includes the
policy bytes' SHA-256, initial and final PR observations, the resolved checks,
workflow and job identities, native import and scope diagnostics where applicable,
and advisory review states. Review state and review read errors never affect the
verdict. The gate publishes no check, status, or comment.

| CLI line | Exit | Meaning |
| --- | ---: | --- |
| `ready: …` | 0 | Every required check has passing evidence. |
| `not ready: …` | 2 | The evidence is complete and a required check did not pass. |
| `unknown: …` | 3 | The policy, scope, or evidence could not be established, or an observation became obsolete. |
| CLI diagnostic | 64 | Invalid command arguments or failure to write the report. |

When a required check's evidence is unknown, the report returns `unknown` and keeps
any separately observed failures. It re-reads the selected workflows and the PR.
Changes to the selected run or attempt, PR head or base, labels, or reported file
count invalidate the evaluated evidence.

## Base policy and check sources

Use a JSON document with a `policy` object and a `checks` array. The
[policy-kernel gate policy](../../.policy-kernel/gate.json), which this
repository uses for its own pull requests, uses light evidence. The
`policy` object follows the [policy v5 contract](policy-scope-v5.md). The gate
uses `required_sensor_ids`, `scope_rules`, and `max_evidence_age_seconds`. It
derives the v5 sensor profile from the check IDs. Omit the optional
`sensor_profile_digest` in a standalone gate policy. The review and authorization
fields of the v5 wire contract have no role in the advisory verdict.

Each check has a unique `id`, an exact workflow file path under
`.github/workflows/`, an exact `job` display name, and a `source`. IDs use ASCII
letters, digits, hyphens and underscores. A workflow and job pair appears once.
For matrix jobs, use the full display name. Path and label rules can require
different checks. A rename contributes both paths.

- `{"kind":"light"}` requires a completed successful job on the selected
  workflow attempt. The evidence record marks it as `light`.
- `{"kind":"native", …}` requires the named job and verified NativeRunV1
  evidence. Use this for jobs that upload native artifacts. Evidence never
  falls back to light mode when an artifact is missing or rejected.

The gate selects the newest non-skipped `pull_request` run for each required
workflow at the observed head. GitHub must link the run to this PR. A queued or
in-progress newer run withholds readiness. GitHub may carry forward jobs on a
partial rerun, so the gate checks that required jobs actually started during the
latest attempt. Use a full rerun when evidence for carried-forward jobs is
required. An unrelated job failure does not override the scoped checks.

## Native job artifacts

A native `source` provides the approved `requirements`
(`RequirementsV1`) and `bindings` (an array of `ObservationBindingV1`).
These use the [native v1 contract](native-contracts-v1.md). Every native
check requires its job's declared native scope.

- Upload `native-run-<key>` from the job. The optional `artifact_key` in the
  base policy defaults to the check ID. Keys must be unique within a workflow.
- The ZIP contains `run.json` at the root and raw files under `artifacts/`.
  `NativeRunV1.artifacts[].path` is relative to `artifacts/`.
- Use `<GITHUB_RUN_ID>-<key>` for `subject.run_id` and `<key>` for
  `subject.scope_id`. GitHub's run and artifact metadata must agree with the PR.
  The gate checks the upload and observation times against the selected job.
  Earlier attempts' artifacts are excluded using the attempt start time. An
  expired upload or multiple candidates for the same job withholds readiness.
- Record the actual checkout SHA. The gate accepts the PR head SHA or a
  two-parent checkout with that head as the second parent and the same Git tree.
  GitHub supplies the checkout identity. Set digests in the producer's subject
  describe its inputs. When a source inventory artifact is supplied, its
  verified raw digest must agree with the subject.
- An envelope that already contains test claims goes directly to the importer.
  For an empty envelope, optional `reports` entries name `observation_id` and
  `artifact_id`. Optional `mappings` entries name `requirement_id` and
  `suite_id`. Both arrays use the existing native claim preparation and
  importer with the parser and requirements supplied by the base policy.

The archive download is capped at 64 MiB and checked against the size and
SHA-256 returned by GitHub. Extraction accepts stored or deflated regular
files and caps each entry at 64 MiB and the archive's expanded files at 256
MiB. The importer checks raw hashes and recomputes report claims. The verifier
applies the base policy's command, configuration, result and test bindings.
Evidence from multiple jobs is evaluated together under distinct check IDs.
An observation named `unit` in two different jobs remains two observations.
Duplicate job, artifact, or observation identities within their namespace
withhold readiness.

GitHub Actions provenance identifies an advisory CI observation. Reviewers and
branch protection are outside the gate verdict. See the
[trust model](../trust-model.md) for authoritative evidence and protection.
