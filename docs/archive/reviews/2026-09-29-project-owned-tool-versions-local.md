# Project-owned tool versions — local execution, 2026-09-29

Scope: local Rust regressions and native frontend acceptance for a dirty
candidate based on `dc8a0ffc43736d2cc17ec3f4600ca73df0c3b351`. This is
LocalUntrusted development evidence. It is not a hosted result, protected merge,
authoritative CI decision, or production promotion.

The opt-in Rust test
`cargo test --locked -p policy-acceptance --test native_project_versions -- --ignored --nocapture`
passed 1/1 on macOS with Node 26.10.0, npm 10.9.3 and 12.1.0, Vitest 4.1.11,
Playwright 1.63.0 and Jest 30.2.0. It used the same kernel build for the
frontend schema-4 selections. Actual `npm ci` and audit passed for both npm
versions. The Jest fixture also passed its syntax and unit checks with npm
12.1.0. The test used a preinstalled npm 10.9.3 executable at
`/private/tmp/policy-tjerp-npm/node_modules/.bin/npm`; it verified that
executable's version before execution.

The native test rejected actual failing Vitest assertions, a Playwright test
that passed only on retry, a vulnerable `lodash@4.17.20` audit, and a controlled
registry connection failure. Empty/truncated/missing report content, stale head
replay and attempts to omit npm dev/optional/peer audit arguments were rejected.
The vulnerable audit reported one high severity lodash vulnerability; the service
failure reported `ECONNREFUSED 127.0.0.1:9`. Those are separate failure classes.
All direct sensor results remained LocalUntrusted. The base fixture lockfile
SHA-256 was `1c26424c667e517c0abbff76ed89f161ac5b6c53c0f64635ca07727189c16724`.

The successful output is retained at
`/private/tmp/policy-project-versions-20260929-09`. Its summary reports dirty
source digest `4a11d7e8a220277fad8dd1fc3ff88d102c5f38651de1e739c132cd9b1c7c3de5`
and harness executable digest
`b78742e9a1c4257c5d0f1ed0ae10ff115934d7a3d78304f77288f6d401b98f14`.

| Artifact | SHA-256 |
| --- | --- |
| `summary.json` | `f9d7f46280a5f543b07da83ae605d4be292286c4934edfd4a935eefb3965e53a` |
| `npm10-result.json` | `4f2111ad67f2aea4ba614d43c31bca0a1382e07ac5a44890fe642cbd6fa462b2` |
| `npm12-result.json` | `6c1508ae2a17e1feb06d892067660e93cc492bb26f7b0d979453fd1b04cfd24b` |
| `jest-result.json` | `5819a5b16039fc06fb5c0e6b9ce1f0145d1f547c5b5b67fc63a4c20891b4718e` |
| `audit-advisory-result.json` | `a4d912b15469c6cb3d2734458593b13eefcf5aef2fb1100a322595f4584c1e9e` |

The hosted candidate job is configured in
`.github/workflows/task-14-consumer-acceptance.yml` but has not run for this
candidate. A clean committed candidate, hosted results, independent review and
the separate promotion conditions remain outstanding.
