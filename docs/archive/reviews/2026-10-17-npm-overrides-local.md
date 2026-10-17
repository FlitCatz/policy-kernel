# 2026-10-17 reviewed npm overrides: local execution

This record describes local advisory verification of the override extension on
branch `fm/pk-npm-overrides`, based on
`e6eea97b559cfa7cbc22b04f86e471a41fed7d2e`. It records a dirty candidate, not
an exact committed-head attestation. Historical P1 execution records are unchanged.

The implementation adds a reviewed `workspace.overrides` profile map that must
match the root manifest. Omitting the map preserves existing profile bytes;
changing it changes the canonical profile digest. Stable exact global pins and
one-level package scopes are supported, capped at 32 distinct targets. Rules
reject member overrides, root/member selectors, selected contract packages,
frozen bootstrap contract names, unsupported names/specs, deeper scopes, dead
rules and affected lock drift. Dependency traversal follows logical ancestry,
including hoisted scoped targets. Frozen bootstrap assets were not edited.
Ambiguous target declarations shared inside/outside a scope are rejected.

## Native observations

The installed toolchain reported Node `v26.12.0` and npm `11.20.0`. The explicit
native command was:

```console
POLICY_NPM_OVERRIDES_OUTPUT=/absolute/worktree/target/npm-overrides-native cargo test --locked --offline -p policy-acceptance --test native_npm_workspace native_npm_overrides_audit_resolved_versions -- --ignored --nocapture
```

The single selected test passed. All registry installs, npm cache and logs were
inside a `tempfile` directory. npm used `--ignore-scripts --no-audit --no-fund`
and explicit root/member inclusion with isolated absent user/global config files.
Cargo used its existing locked offline cache. No host toolchain, repository npm
installation, or frozen bootstrap installation was performed.

A member depended on `mkdirp` 0.5.0. The global override `minimist: 0.0.8`
resolved to that version in every locked copy and in Node's actual resolution;
preparation accepted the matching reviewed profile. One root audit exited 1 and
reported minimist with nodes matching the installed lock paths. Reinstallation
with scoped `mkdirp: { minimist: 1.2.8 }` resolved to 1.2.8, passed preparation,
and one root audit exited 0 with zero vulnerabilities. Each phase generated a
fresh native lock and installation. Live advisory responses are observations
at execution time, not a permanent guarantee about those versions.

Raw `vulnerable-audit.json`, `fixed-audit.json`, both root manifests and locks,
and `summary.json` are retained locally under `target/npm-overrides-native/`.
The subsequent two-test native workspace/override regression run also passed,
with override reports retained under `target/npm-overrides-final/`.
They are not committed policy evidence. The fixture originally failed before
installation because a relative output path was interpreted from Cargo's test
working directory; the successful execution used an absolute output path.

## Limits

This is **local scope satisfied**, remains LocalUntrusted, and does not establish
an authoritative decision, a protected merge, production promotion, or TJERP
adoption. The native audit proof exercises global and scoped overrides with
minimist; the exact three-rule TJERP set is covered by synthetic lock fixtures,
not a real TJERP frontend installation. After integration TJERP P8 must raise
its reviewed policy-kernel revision, declare the reviewed override map in its
profile, and re-verify frontend `prepare` against its actual lock and install.
See the [integration contract](../../integrations/npm-workspaces.md#reviewed-root-overrides)
and [current status](../../status.md).

## Repository verification

Local `cargo test --workspace --locked` passed 449 tests with 14 ignored native
or live tests; the two native npm workspace tests were separately executed and
passed. The npm workspace regression file contains 18 tests. Strict Clippy,
formatting, locked workspace check, deterministic schema generation, cargo-deny
0.20.2, cargo-audit 0.22.2, residue and public API checks passed. cargo-deny
reported existing duplicate-version warnings but no policy failure.

The changed documentation's 147 relative Markdown file links resolved, and
`lat check` 0.12.2 reported `All checks passed`. `git diff --check` passed.
No mutation-contract-covered source or frozen bootstrap asset changed, so no
mutation inventory reconciliation was needed. This worktree has no CodeGraph
index; the requested sync reported it was not initialized, and no index was
created or committed. The local Paseo Progress plugin was unavailable.
