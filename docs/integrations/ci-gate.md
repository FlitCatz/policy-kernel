# Advisory pull request gate

Run the distributed `policy-cli` on a machine with authenticated `gh`. Grant the
caller read access to the repository's contents, pull requests, and Actions.
The gate uses GitHub REST GET requests with those credentials.

```console
policy-cli gate --repo owner/repo --pr 42 --policy .policy-kernel/gate.json --output gate-42.json
```

The policy path is repository-relative. For an open PR, the gate resolves the
current tip of its target branch and reads the policy at that commit. The default
output is `gate-evidence.json`. Choose a new path on each run. The report includes
the policy bytes' SHA-256, the PR's reported base SHA within each PR observation,
the independently resolved initial and final base tips, the resolved checks,
workflow and job identities, native import and scope diagnostics where applicable,
and advisory review states. Review state and review read errors never affect the
verdict. The gate publishes no check, status, or comment.

| CLI line | Exit | Meaning |
| --- | ---: | --- |
| `ready: …` | 0 | Every required check has passing evidence. The line names the evidence classes used. |
| `not ready: …` | 2 | The evidence is complete and a required check did not pass. |
| `unknown: …` | 3 | The policy, scope, or evidence could not be established, or an observation became obsolete. |
| CLI diagnostic | 64 | Invalid command arguments or failure to write the report. |

When a required check's evidence is unknown, the report returns `unknown` and keeps
any separately observed failures. It re-reads the selected workflows and the PR.
Changes to the selected run or attempt, PR state, head or base, independently
resolved base tip, labels, or reported file count invalidate the evidence.
Closed and merged PRs yield `unknown`.

## Base policy and check sources

Use a JSON document with a `policy` object and a `checks` array. The
[policy-kernel gate policy](../../.policy-kernel/gate.json), which this
repository uses for its own pull requests, uses light evidence. The
`policy` object follows the [policy v5 contract](policy-scope-v5.md) or the
[v4 contract](../status.md#policy-v4-local-runner-contract). V5 resolves path
and label scope rules and derives a sensor profile from the check IDs. V4 uses
the fixed `required_sensor_ids` list. Omit the optional `sensor_profile_digest`
in a standalone gate policy. The gate uses `max_evidence_age_seconds` and
reports review observations for reference.

Each check has a unique `id`, an exact workflow file path under
`.github/workflows/`, an exact `job` display name, and a `source`. IDs use ASCII
letters, digits, hyphens and underscores. A workflow and job pair appears once.
For matrix jobs, use the full display name. Path and label rules can require
different checks. A rename contributes both paths.

- `{"kind":"light"}` requires a completed successful job on the selected
  workflow attempt. The evidence record marks it as `light`.
- `{"kind":"native", …}` requires the named job and verified NativeRunV1
  evidence. Use this for jobs that upload native artifacts. Evidence never
  falls back to light mode when an artifact is missing or rejected. Only a
  successful job proceeds to artifact intake.

Both forms map a required job's conclusion the same way. A job that concluded
`failure` or `timed_out` yields `not ready`. A cancelled, skipped, neutral, or
other unsuccessful job yields `unknown`.

The gate selects the newest non-skipped `pull_request` run for each required
workflow at the observed head. GitHub must link the run to this PR. GitHub does
not link runs for a PR opened from a fork, so such a PR yields `unknown`. A head
branch with more than one open PR in the repository also yields `unknown`, as
does a candidate run that links multiple PRs. A queued or in-progress newer run
withholds readiness. GitHub may carry forward jobs on a partial rerun, so the
gate checks that required jobs actually started during the latest attempt. Use a
full rerun when evidence for carried-forward jobs is required. An unrelated job
failure does not override the scoped checks.

Changes under `.github/workflows/` or `.github/actions/` yield `unknown`,
including a rename into or out of either directory. The base-branch policy
controls which checks are required. The workflow-change rule withholds
readiness when the PR edits any CI workflow or action, whether or not a
required check uses it, because the evidence from that PR cannot be trusted.

## Signed local runs

To use a registered local runner, the base policy must opt in with
`policy.local_runner_evidence` and the gate document must declare
`local_attestation`. Name the required native checks that the approved combined
local scope covers. Other required checks keep their CI source. A single
signed run can cover all selected checks from that list. The base policy's
approved templates must each cover the whole listed set.

```json
{
  "allowed_signers": ".policy-kernel/allowed_signers",
  "checks": ["verify"],
  "ci_required_labels": ["run-ci", "security", "dependency-update", "hosted-review"]
}
```

Pass the attester's output directory to the gate.

```console
policy-cli gate --repo owner/repo --pr 42 --policy .policy-kernel/gate.json \
  --local-attestation /work/attested --output gate-42.json
```

The directory supplies `gate-attestation.json`, `expected.json`, and
`observations.json`. The gate reads the public key registry at the same base
branch tip as the policy. It checks the signature and user-presence flag,
enforces the approved runner and template, compares the exact PR number, head,
tree, base and policy-file hash, and recomputes the scope result. A missing,
stale, unregistered, mismatched or unverifiable local run withholds readiness
for checks routed to local evidence.

The gate uses CI for every required check when a configured CI-only label or
`ci_required_when` condition matches, or when the PR changes the gate policy or
key registry itself. Labels match case-insensitively. Risk-specific conditions
are retained when they could match because the gate has no independent risk
observation. The workflow/action edit rule above still applies. Configure the
repository's checking tools, runner and toolchain pins as CI-only paths, and
route security and dependency updates, static-analysis policy transitions,
hosted-review exceptions and scheduled main checks to hosted CI.

The one-line verdict and the JSON report identify `local_runner_attested` and
`ci_observed` when a verdict uses each class. The report includes the verified
statement and the class for each check. See [key registration and signing](local-runner-signing.md).

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
  The gate binds the artifact to the run attempt and the selected job's time
  window. GitHub's artifact metadata has no uploader job identity. Earlier
  attempts' artifacts are excluded using the attempt start time. An
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

The gate produces an advisory verdict. Reviews and branch protection remain
independent controls, and shipping requires CI-grade evidence. See the
[trust model](../trust-model.md) for authoritative evidence and protection.
