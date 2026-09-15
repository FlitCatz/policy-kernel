# Policy kernel Action

This composite Action invokes an **already-built, owner-installed `policy-ci` at
an absolute path** (`executable-path`, default `/opt/policy-kernel/bin/policy-ci`).
The runner also needs `git` and authenticated `gh`. Build the binary from an
approved kernel revision with `cargo build --locked -p policy-ci`; install it on
the runner before invoking the Action. Run from `GITHUB_WORKSPACE`, checked out at
the full evaluated **PR head** in both modes. The PR head can differ from
`GITHUB_SHA` (the workflow trigger's synthetic merge commit).

The calling workflow declares:

```yaml
permissions:
  checks: write
  contents: read
  pull-requests: read
  issues: read
  actions: read
```

`actions: read` is necessary for independently fetching workflow identity,
including in private repositories. See [GitHub's workflow-run API](https://docs.github.com/en/rest/actions/workflow-runs).
The Action inherits authentication; supply `GH_TOKEN` from the calling workflow.
It does not acquire credentials or download a binary.

After trusted observation/evidence collection and installing the binary:

```yaml
- uses: FlitCatz/policy-kernel/action@<full-reviewed-commit-sha>
  id: policy
  env:
    GH_TOKEN: ${{ github.token }}
  with:
    executable-path: /opt/policy-kernel/bin/policy-ci
    mode: pre-merge
    policy-path: /trusted/policy.json
    evidence-path: /trusted/evidence.json
    expected-head: ${{ github.event.pull_request.head.sha }}
    artifact-directory: ${{ runner.temp }}/policy-result-${{ github.run_attempt }}
    trusted-observation-directory: ${{ runner.temp }}/policy-observations
```

Replace `<full-reviewed-commit-sha>` with the immutable commit of the approved
kernel release. Load the Action definition from that revision, not from the
PR checkout: an edited composite Action could bypass the installed binary.

Use `post-merge` for the second phase. The observation directory and artifact
paths above are examples; the caller must populate them as described below.
Choose a new artifact directory per invocation: existing output files are never
overwritten. Fork PR workflows must have a separately designed trusted collection
and publication path with the required permissions before they can use this gate.

## Trusted observation contract

This is a trusted-runner integration boundary, not a verifier for arbitrary
uploaded JSON. A trusted workflow/controller must create the following files
**outside the evaluated checkout**, after collecting or authenticating the
observations. PR code, downloaded PR artifacts, and contributor-controlled steps
must not be able to replace these files, the policy, the Action definition, or the installed binary.
Putting a local report outside the checkout does not make it trusted. The CLI
rejects a trusted directory inside the checkout and symlinked observation files;
filesystem location alone is not proof of origin.

Legacy externally authenticated runner integrations use `identity.json` version 1
with exactly these fields:

```json
{
  "schema_version": 1,
  "repository": "owner/repository",
  "pull_request_number": 42,
  "head_sha": "cccccccccccccccccccccccccccccccccccccccc",
  "workflow_run_id": "9001",
  "workflow_run_attempt": 2,
  "workflow_trigger_head_sha": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "workflow_head_sha": "cccccccccccccccccccccccccccccccccccccccc",
  "workflow_id": 77,
  "workflow_path": ".github/workflows/policy.yml",
  "check_app_id": 15368,
  "check_app_slug": "github-actions",
  "policy_digest": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
  "evidence_digest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
  "context_digest": "1111111111111111111111111111111111111111111111111111111111111111",
  "work_item_digest": "2222222222222222222222222222222222222222222222222222222222222222"
}
```

`workflow_trigger_head_sha` binds `GITHUB_SHA`; `workflow_head_sha` independently
binds the REST workflow-run `head_sha`. For `pull_request` runs the former may be
the synthetic merge commit while the latter is the PR head. Do not substitute one
for the other.

The values are illustrative. Configure the allowed workflow and publishing App
from trusted project settings, rather than copying claims from a Check Run.
`policy_digest` is `policy_core::pre_merge_policy_digest(policy)`;
`evidence_digest` is `policy_core::canonical_sha256(full_typed_evidence_bundle)`,
including the manifest. `context_digest` and `work_item_digest` are canonical
SHA-256 digests of the typed `PreMergeContext` and `WorkItemObservation` files;
pre-merge verifies all four digests against the receipt. These two fields and
`evidence_digest` are retained but not consumed for the current post-merge run.
The trusted collector issues this receipt only for
current-run evidence it observed or independently authenticated. Local evidence
retains its untrusted classification. The CLI checks receipt bindings against
Actions environment fields and a fetched workflow-run response. It separately
compares the actual Git checkout and fetched PR head to `head_sha`.

The [authenticated collection reference](controller.md) produces version 2
receipts, adding required `workflow_ref` and `workflow_source_sha` fields. The
consumer compares these with `GITHUB_WORKFLOW_REF` and `GITHUB_WORKFLOW_SHA`.
The latter identifies the workflow definition commit, independently of both the
evaluated PR head and the trigger commit. Version 1 retains its existing trusted
runner contract; it cannot carry version 2 fields. Neither receipt version is a
public verifier for arbitrary JSON. Protect the consuming process and receipt
directory from contributor code, including processes running under the same OS
identity. See the reference's key ownership and service isolation requirements.

For **pre-merge**, also provide:

- `context.json`: `PreMergeContext`, including the policy-bound sensor profile,
  complete changed paths/risk, current run, and an exact-head `agent_ready`
  observation (or null when absent).
- `work-item.json`: `WorkItemObservation`, including separately verified work
  authorization. Work item IDs and PR numbers are distinct.
- The `policy-path` points to a `PolicySnapshot` whose required check is
  `policy-kernel/pre-merge`; the `evidence-path` points to an `EvidenceBundle`.

For **post-merge**, provide a current-run `identity.json` and retain these files
from the authenticated pre-merge publication:

- `pre-merge-input.json`, emitted by the pre-merge command. The CLI re-evaluates
  it to recover the decision, rather than accepting arbitrary decision JSON.
- `publication-receipt.json`, emitted after the Check Run creation response.
  Its `published_at` records trusted runner time after the API response; the
  core-facing check completion uses this conservative upper bound, so a check
  posted after a merge cannot claim the earlier evaluation time. Trusted runners
  must have synchronized clocks. Preserve this via trusted runner storage; copying contributor artifacts cannot
  establish this receipt's provenance.
- `merge-receipt.json`, a `GitHubMergeObservation` captured by the trusted merge
  integration, containing merge method, branch protection **at merge time**,
  actor type and explicit queue use, as well as repository, PR head, merge SHA,
  branch, actor and timestamp. See `tests/fixtures/github/merge-receipt.json`.

The CLI re-fetches the PR, dedicated Check Run, and publication's workflow run.
The merge receipt is compared with API-observable facts; missing supplemental
facts fail closed. The current REST PR response does not establish historical
protection or an unambiguous actual merge method. A Check Run's JSON and
`external_id` are caller-controlled, so they must agree with the independent
publication receipt. The custom check suite and workflow suite may differ.
A later workflow rerun invalidates the older attempt under the current strict
workflow-run check; recollect/re-publish for the accepted attempt.

## Results

Pre-merge emits canonical `pre-merge-input.json`, `decision.json`,
`publication-receipt.json` and `check.json`. Post-merge emits
`post-merge-input.json` and `decision.json`. Check binding JSON is capped at 4 KiB.

`READY_TO_MERGE` publishes success; `BLOCKED` publishes failure; `INCONCLUSIVE`
publishes neutral. Both non-ready decisions return exit code 1 **after** publishing
and emitting step outputs. Invalid inputs, authentication/API errors and binding
failures return 2, without reporting authorization. Post-merge returns 0 only for
`SHIPPED`.

When promoting this gate, require both the dedicated Check Run and the Actions
job that invokes this Action, and do not use `continue-on-error` or skip that job.
The task's prescribed inconclusive conclusion is `neutral`, which [GitHub accepts
as passing for branch protection](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks).
The failing Actions job is therefore necessary to block merging an inconclusive
result. Requiring only the dedicated Check Run is insufficient. The core still
requires an actual `success` conclusion before it can emit `SHIPPED`.

The Action exposes status, head, decision/policy digests, check ID and
current workflow run/attempt, and writes a concise step summary.

The Action performs evaluation and Check Run publication only. Merge operations,
waivers and protection changes remain outside this interface.

## Task10 live transport smoke

`.github/workflows/task-10-live-smoke.yml` runs only for a same-repository PR
whose head branch is `task10-github-ci`. It invokes the ignored
`policy-ci/tests/live_github.rs` integration test with explicit opt-in and a
GitHub Actions token. The test calls the real `policy-ci` binary, fetches the PR
and current workflow, publishes a Check Run, then reads it back from GitHub.

The input deliberately contains **no execution/review evidence** and an
**unauthorized work item**. The expected result is `BLOCKED`, process exit code 1,
and a **failure** Check Run named `policy-kernel/pre-merge`. The smoke test job is
successful only when that rejection and the head/run/digest bindings are observed.
It does not establish release readiness, authenticate contributor evidence,
exercise a protected merge, or configure branch protection. Do not use this
smoke workflow as a production collector or release gate.

Regular `cargo test --workspace` ignores this network test. Explicit execution
requires `POLICY_CI_LIVE_SMOKE=1`, Actions identity and the dedicated PR context.
The local simulated suite continues to test ready and post-merge decisions.
