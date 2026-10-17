# Third-party npm bundles local execution — 2026-10-17

This record describes a dirty candidate based on
`c4d4e37fc4806b5b4ea6461f10e959b99cafcd7c` on isolated branch
`fm/pk-npm-bundled-deps`. It establishes local fixture behavior, not a merged
revision, TJERP adoption, authoritative CI decision or production promotion.

## Execution

The installed Node 26.12.0 and npm 11.20.0 were checked by the Rust fixture.
Registry installation and audit access were explicitly authorized for temporary
fixture directories only; Cargo was locked and offline. No repository/global
npm installation or frozen bootstrap asset change was made.

```console
POLICY_NPM_BUNDLES_OUTPUT=/private/tmp/pk-npm-bundles-20261017 cargo test --locked --offline -p policy-acceptance --test native_npm_workspace native_npm_bundled_platform_inventory -- --ignored --nocapture
```

The fixture passed. npm installed `@tailwindcss/oxide` 4.3.3 and generated the
root workspace lock with an optional `@tailwindcss/oxide-wasm32-wasi` 4.3.3
parent (`cpu: ["wasm32"]`). That parent was not installed on this host. Its
six `bundleDependencies` had no individual entries beneath the parent:
`@napi-rs/wasm-runtime`, `@emnapi/core`, `@emnapi/runtime`, `@tybys/wasm-util`,
`@emnapi/wasi-threads` and `tslib`.

Preparation succeeded and retained all six with null versions, the locked
parent version and sha512 integrity, and coverage exactly
`bundled, not individually audited`. The one root audit exited 0. This audit
result does not imply individual audit coverage for any bundled name.

## Retained local artifacts

Artifacts are local to `/private/tmp/pk-npm-bundles-20261017` and are not shipped
as policy evidence. SHA-256 identities from this execution are:

| Artifact | SHA-256 |
| --- | --- |
| `package-lock.json` | `5c98159a9146bda94077f6fa549dcb843a116ed43e948e6bc7773368813db938` |
| `audit.json` | `984478e57c7f692a41be81a07bcb83a1739451f064644ac0c705f41fa1603e17` |
| `prepared.json` | `1abab757069f0195b1f160673dd753951314a52cd99db305deed332fbbf84ddd` |

The summary records **local scope satisfied**, `LocalUntrusted`, an absent
platform parent, six bundle records and audit exit 0. The
[contract](../../integrations/npm-workspaces.md#third-party-bundled-dependencies)
defines accepted parent identities and explicit `inBundle` limits. TJERP P8
must adopt a reviewed merged revision and re-verify its own frontend `prepare`.
