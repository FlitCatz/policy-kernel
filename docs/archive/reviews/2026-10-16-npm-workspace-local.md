# P1 npm workspace local verification — 2026-10-16

This is a local fixture execution record for the P1 change on top of
`9f0829da4765460b1f43276f07d0500c439b3137`. It is not TJERP adoption evidence,
an authoritative CI decision or production promotion. Status: **local scope
satisfied**. Direct observations remain **LocalUntrusted**.

The captain revised the Node pin to **26.12.0**, retained **npm 11.20.0**, and
authorized registry installation of TypeScript **6.0.3**, TypeScript **5.9.3**,
lodash **4.17.20** and their dependencies only in the temporary native fixture,
plus real npm audit there. Earlier attempts stopped at the old Node pin and an
offline `ENOTCACHED` response; registry access was used only after approval.
There were no repository, global or host-toolchain installations. Synthetic
unit-test lock/installed files were removed before npm generated the native tree.

## Native execution

The reviewed fixture ran with a cleared child environment and an isolated npm
cache/log directory. The sandbox could not resolve the registry; the authorized
network-enabled execution passed. The fixture directory was removed on exit.

```console
POLICY_NPM_WORKSPACE_OUTPUT=/private/tmp/p1-npm-workspace-native-20261016-reviewed cargo test --locked --offline -p policy-acceptance --test native_npm_workspace -- --ignored --nocapture
```

Result: **1 passed**, exit **0**. Rust dependency resolution stayed locked and
offline; the explicitly invoked npm fixture used the registry.

| Case | Observed result |
| --- | --- |
| Native Node resolution at root | TypeScript 6.0.3 |
| Native Node resolution at `tools/openapi` | TypeScript 5.9.3 |
| Baseline root audit | Exit 0, total vulnerabilities 0 |
| lodash declared only by member | One root audit exits 1 and reports lodash |
| Member installed TS5 manifest replaced with root TS6 manifest | Preflight rejects |
| lodash declared only by root | One root audit exits 1 and reports lodash |
| Restored baseline | Exit 0, total vulnerabilities 0 |

The root/member vulnerability reports happen to be byte-identical because npm
hoists lodash to the same installed location. Their distinct generated lockfiles
record which manifest declared it. Each phase runs one audit, not per-member
copies of the same audit. Raw audit JSON, generated locks, install stdout and
summary are retained at the output path above. Important SHA-256 values:

| Artifact | SHA-256 |
| --- | --- |
| `baseline-audit.json` / `recovery-audit.json` | `8dd2d2976872e12ea42015a0fea6df1750b47d2dc8514c9c97c47ebfd16ea9d7` |
| `member-vulnerability-audit.json` / `root-vulnerability-audit.json` | `3774c93f3d6fcb805e981b58fd91a5146039f117923d4ac93d9884689e256833` |
| `baseline-package-lock.json` / recovered `package-lock.json` | `5fa83fb9274cc9ccd57e440c1b269eac171a07dbdea50cf9cfd03bc5b7104671` |
| `member-vulnerability-package-lock.json` | `d2b0b661b1a6a5f8d98aa728576bb4d968d61e78ae33c497dfe62e32026b82a2` |
| `root-vulnerability-package-lock.json` | `68f3dbced856725da74f22b25bdbacd847393b8fe8c854a3b15cd6a802f6d7d3` |
| `summary.json` | `af595bdb4856031f464c1550cf2adabf824b1fe3025a6aeabd1699f3d0d4b867` |

## Local regression and repository checks

The workspace suite passed **405 tests**, with **6 opt-in tests ignored** and
zero failures, including doc tests. The new native test was then explicitly run
as recorded above. Eleven workspace regression groups cover manifest/lock
drift, omitted audit inventory, member add/remove, external dependencies,
unsafe paths, symlinks, config/environment exclusions, installed-manifest swaps,
one-audit binding and component-root selection.

Strict workspace Clippy, formatting, deterministic schema generation, runtime
residue and public API checks passed. `cargo deny --offline check` passed with
existing duplicate-dependency warnings; `cargo audit --no-fetch` passed against
the installed advisory database, not a freshly fetched database. Frozen
bootstrap assets, Cargo.lock and checked-in contract schemas were unchanged.

No authorization source covered by the mutation contract changed. The
promotion mutation suite was not rerun and this record is not promotion
evidence. Hosted CI and PR review are separate observations; consult
[current status](../../status.md) for later status instead of extending the
meaning of this local execution record.
