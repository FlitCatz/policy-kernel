# Installed project verification configuration

This installation configures checks; it does not create an application, test
cases, policy approval or passing evidence. Template 1.0.0 and the Task 12 native
tool pins remain compatibility candidates pending Task 14 acceptance.

Install the pinned toolchains through your existing controlled setup. Author the
application, dependency manifests and lockfiles, application-specific configuration
and real acceptance/negative tests. Native lint/build/test defaults are installed;
their JS/TS files are declarative configuration, not installer or application logic.
Missing inputs listed by the
bootstrap plan must be supplied before running checks. No dependency restore or
package manager is run by bootstrap. Do not migrate an existing toolchain merely
to make the installer accept it; adapt and review a compatible profile separately.

Frontend defaults additionally require exact @typescript-eslint/parser 8.42.0 and
jsdom 26.1.0 in package.json, package-lock.json and installed dependencies. The
bootstrap profile binds those package/lock entries and uses eslint.config.mjs
explicitly. Keep JSX/application/test dependencies in your own manifest. Fullstack
contract tests live in tests/contracts; E2E journeys live in tests/e2e. Provision
the real servers before the shared E2E run; no example server or fake journey is
generated. No discovered tests remains a failure.

The project profile records component roots, explicit argv, package/runtime pins,
report destinations and required test counts. `sensor-profile.json` is unbound;
prepare a fresh bound profile before explicit policy adoption. Empty adequacy
lists require consumer policy authoring. Zero tests and missing reports are failures.

With already-built Rust tools on PATH:

```console
project-verification validate-profile .policy/project-profile.json
project-verification prepare --input .policy/run-request.json
project-verification run --input .policy/run-request.json
```

Create the RunRequest according to `docs/integrations/project-verification.md` in
the kernel release: actual checkout root/head, reviewed source digests, adopted
profile digest, impact and adequacy mappings are mandatory. Bootstrap cannot
invent these values. Ensure report destinations are fresh before each run.

`ci/verification.yml` contains steps for the consuming project's existing trusted
job. Keep its current native CI baseline. Review and activate these steps only
after provisioning the pinned tools and validating real producer reports.
`ci/policy-action.yml` is a copy of the existing Task 10 composite action: consult
`ci/policy-action-README.md` for runner-controlled receipts, exact-head identity
and policy-ci requirements. Bootstrap does not supply credentials or publish checks.

Commit the installed configuration and bootstrap manifest together. A later plan
shows changed managed files and conflicts. Never edit the receipt to claim that
local edits belong to a template. Preserve those edits and resolve ownership before
reinstalling. Uninstall also uses a saved plan and refuses locally edited files.
