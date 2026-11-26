# Register a key and sign a local run

Use a FIDO authenticator that supports OpenSSH security keys and requires
physical presence for each signature. Use `ed25519-sk` (wire type
`sk-ssh-ed25519@openssh.com`) on a FIDO2 key that supports Ed25519, or
`ecdsa-sk` (wire type `sk-ecdsa-sha2-nistp256@openssh.com`) on a compatible
FIDO/U2F key. Install a FIDO-capable build of OpenSSH on the signing Mac or
Linux machine. The attester uses `/usr/bin/ssh-keygen` by default. Supply
`--ssh-keygen /absolute/path/to/ssh-keygen` if your FIDO-capable copy is elsewhere.

## Create and register the key

Connect the authenticator and generate a dedicated key handle. At key creation,
touch the authenticator when prompted. The handle uses an empty file passphrase
so the attester can inspect its touch requirement before signing. The signing
key itself stays on the authenticator.

```console
ssh-keygen -t ed25519-sk -f ~/.ssh/policy-kernel-runner -N '' -C policy-kernel-runner
```

Keep the handle `~/.ssh/policy-kernel-runner` on the owner's machine, outside the
checkout. Add the public key from the `.pub` file to the repository's
`.policy-kernel/allowed_signers`, on a single line with the exact runner ID
first.

```text
owner-runner namespaces="policy-kernel/gate-local-attestation/v1" sk-ssh-ed25519@openssh.com <public-key-base64>
```

Use the normal review and hosted CI process to approve the registry and gate
policy at the base branch. Independently compare the public key fingerprint
against `ssh-keygen -lf ~/.ssh/policy-kernel-runner.pub` on the owner's
machine. Put the same ID in `policy.local_runner_evidence.allowed_runner_ids`.
Supply the approved combined scope template hash in
`approved_scope_template_digests`. Configure the runner, check tooling, CI
policy and toolchain pins in `ci_required_when`, and configure the labels that
require hosted CI in `local_attestation.ci_required_labels`. See the
[gate configuration](ci-gate.md#signed-local-runs).

To rotate the key, generate a new handle and replace the old public key in
the base-branch registry in a reviewed change. Use one entry per runner ID.
To revoke a lost key, remove its entry and runner ID from the base policy.
Once the base advances, old attestations require a new run and signature against
the new base. The gate rejects the removed key on subsequent evaluations.

## Sign the current PR run

Install the [pinned release binaries](native-contracts-v1.md#pinned-binary-releases).
Fetch the target branch, check out the exact PR head in a clean worktree, and
prepare an independent `ExpectedScopeV1` for the approved checks with at least
one test suite. The runner
writes a fresh `NativeRunV1` manifest and raw reports. Keep the expected scope,
runner artifacts, and attestation directory outside the checkout.

```console
policy-local-attest \
  --repo /work/checkout --base origin/main --pr 42 --gate-policy .policy-kernel/gate.json \
  --expected /work/input/expected.json --run /work/run/run.json \
  --artifacts /work/run/reports --output /work/attested \
  --run-id run-42 --scope-id full-check --runner-id owner-runner \
  --signing-key ~/.ssh/policy-kernel-runner \
  -- /owner/bin/local-check /work/run/run.json /work/run/reports
```

The attester verifies the run and displays the repository, PR, head and tree,
base, policy and scope-template hashes, runner and scope, and the SHA-256 of
the statement it will sign. For initial template approval, compare the
displayed template digest to the reviewed expected scope and decline signing.
When signing, compare the preview to the intended PR and result, type `yes` in
the terminal, then touch the key when OpenSSH prompts. One confirmation and one
signature cover the combined run for that PR snapshot. If the head, base or
policy moves, run the checks and confirm again for the new snapshot.

Forward `gate-attestation.json`, `expected.json`, and `observations.json` to the
gate evaluator through your project's evidence channel. Keep the PR head
unchanged when transferring them. The [gate command](ci-gate.md#signed-local-runs)
reads the three files from one directory. Use the evaluator in the lightweight
hosted PR job for independently observed PR and base metadata. Keep that job
and its policy inputs under owner control. The advisory gate report does not
publish a check or establish shipping evidence.

The wire format uses a [canonical run statement](native-contracts-v1.md#local-runner-attestation-contracts)
inside the gate statement along with the PR number and SHA-256 of the exact
policy file bytes at the base. The [OpenSSH security-key protocol](https://github.com/openssh/openssh-portable/blob/master/PROTOCOL.u2f)
defines key flags and signature presence. Confirmation shows what will be signed.
Code running under the owner's account can still imitate that prompt, so use the
released attester and a reviewed runner.
