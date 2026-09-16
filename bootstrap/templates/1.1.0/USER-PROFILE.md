# User profile integration

The source profile recorded in bootstrap-manifest.json remains user-owned.
This installation manages only this guide and the fixed CI integration examples.
It does not generate a project profile, sensor profile, native configuration,
application, tests, approval, or passing evidence.

Validate your source with project-verification validate-profile PATH, replacing
PATH with the receipt's user_profile.path. Prepare a fresh RunRequest using that
profile, its current digest, reviewed inputs and the actual checkout head.
Run project-verification prepare --input .policy/run-request.json, then
project-verification run --input .policy/run-request.json. Changes to the source
require fresh preparation and policy adoption; the install receipt is not approval.

The CI examples are inactive. Integrate them into an existing trusted job after
provisioning the selected tools and reviewing the action's trust requirements.
Bootstrap runs no package manager and grants no CI or merge authority.

To uninstall, pass the same --profile-file PATH and --release 1.1.0 with --uninstall.
The source may have been edited or removed; uninstall preserves all user inputs.
Locally edited managed integration files still conflict. Never edit the receipt
to claim ownership of local files.
