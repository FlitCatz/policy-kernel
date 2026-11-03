# Native fixture audit refresh

Bootstrap preset 1.1.1 patches native fixture tooling without changing the bytes
of 1.1.0 or weakening dependency audits. Observations here are local and advisory;
hosted outcomes belong to the exact-head PR checks, not a production promotion.

## Scope and remaining failure

Preset profiles retain schema 2, receipt schema 2, existing sensors, Node
22.18.0/npm 10.9.3 and all other direct tool pins. Frontend Vite is 7.3.7 and
Vitest is 4.1.12. Acceptance stages the real 1.1.1 preset; CI derives runtimes
from that same release. Historical template assets and execution records remain
unchanged. No tag or release promotion is performed.

The Jest fixture remains on Jest 30.5.2. Its audit must still fail on unpatched
deepmerge 4.3.1, [GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4).
It executes inside Jest configuration merging, not the production React bundle.
The alternate Jest test asserts a clean audit, and the project-owned frontend
test runs the same Jest fixture after its npm10/npm12 frontend phases. Both jobs
therefore remain red; no ignore list, severity change or deepmerge workaround is
introduced. Five affected npm package entries represent this one advisory.

## Audit results

Audits include development, optional and peer dependencies at severity `low`.
The before locks are those committed on the base branch. Baseline observations
used Node 26.13.0/npm 11.20.0; updated locks were installed and audited using
Node 22.18.0/npm 10.9.3. These are actual npm audit results, not inferred fixes.

| Graph | Before | After | Audit exit |
| --- | --- | --- | --- |
| Alternate Jest | 19: 3 low, 16 moderate | 5 low, all deepmerge | 1 |
| Project-owned frontend, before its later Jest phase | 3: 2 moderate, 1 high | 0 | 0 |
| React TypeScript / fullstack frontend | 10: 9 moderate, 1 high | 0 | 0 |
| Fullstack shared | 3: 2 moderate, 1 high | 0 | 0 |

The Vite patches address GHSA-698v-4r3x-3vcv, GHSA-r825-w8mm-5pxw,
GHSA-587h-7wmj-h5pm, GHSA-9jrq-w75r-8gcw and GHSA-rq7h-c2jc-7f22;
the shared Vite patch also addresses GHSA-vfpm-58rq-9qcg. Vitest 4.1.12 fixes
@vitest/mocker GHSA-qxg6-89mg-rqv8 and GHSA-xp26-3454-mjgp. The scoped overrides
fix minimatch [GHSA-g3hv-75wg-m249](https://github.com/advisories/GHSA-g3hv-75wg-m249)
and glob [GHSA-3g3m-2fjr-6pw4](https://github.com/advisories/GHSA-3g3m-2fjr-6pw4).

## Override boundaries and compatibility

React/TypeScript scopes minimatch 11.0.4 beneath ESLint and the existing
parser-owned typescript-estree 8.48.0 override. This covers ESLint's config-array
and eslintrc descendants. Jest scopes glob 14.0.3 beneath @jest/reporters,
jest-config, jest-runtime and test-exclude; test-exclude also pins minimatch
11.0.4. Glob's own range resolves minimatch 11.0.4. Shared Vitest's declared Vite
range accepts 8.2.4; a lock refresh retains that patched resolution without a Vite
override. The project-owned frontend retains React 19.0.0, Playwright 1.63.0
and Vite 6, patching Vite to 6.4.5 and Vitest to 4.1.12.

Minimatch/glob overrides cross transitive major versions. Their used named
`minimatch`, `Minimatch`, `glob` and `glob.sync` APIs remain available, and their
Node ranges include the selected runtimes. Local probes passed ESLint/config-array
include/exclude matching, eslintrc's CommonJS `OverrideTester` include/exclude
matching, typed parser `projectService.allowDefaultProject` acceptance and
rejection, test-exclude `shouldInstrument` plus synchronous/asynchronous globbing,
and glob's named/synchronous APIs. Jest's two SSR tests passed with these locks.

This is compatibility with the fixtures' actual callers, not a general-purpose
minimatch replacement: importing eslintrc's ESM entry directly fails because it
default-imports minimatch. ESLint uses eslintrc's CommonJS entry, which passed
the probe. Consumers using the ESM entry must not copy this override unchanged.
Bootstrap templates do not install these fixture-owned overrides.

Lock generation with npm 10.9.3 hit an internal Arborist `edgesOut` exception
for the Vitest graphs. npm 11.20.0 generated those locks; actual `npm ci` with
10.9.3 succeeded. Jest's lock was refreshed with 10.9.3 after removing stale
glob/minimatch resolutions, preserving unrelated locked package versions.
Generation used `--ignore-scripts`; audits remained enabled as separate checks.
The final shared lock refresh without the Vite override succeeded with npm
10.9.3, retaining all resolved versions and removing npm 11's extra libc metadata.

## Dependency and licence changes

The committed lockfiles are the complete resolution inventory, including
optional platform packages. The changes below list package identities rather
than counting deduplicated locations as new dependencies.

- All three Vitest graphs: vitest and @vitest/{expect,mocker,pretty-format,runner,
  snapshot,spy,utils} 4.1.11 → 4.1.12; chai 6.2.2 → 6.3.0 and tinyrainbow
  3.1.1 → 3.2.0. All retain MIT.
- React: Vite 7.3.6 → 7.3.7 (MIT); minimatch 3.1.5/9.0.9 (ISC) → 11.0.4
  (BlueOak-1.0.0); brace-expansion 1.1.21/2.1.7 → 5.0.12 and balanced-match
  1.0.2 → 4.0.4 (MIT). Remove concat-map 0.0.1 (MIT).
- Project-owned frontend: Vite 6.4.3 → 6.4.5 (MIT); no added package identities.
- Shared: Vite 8.2.2 → 8.2.4, rolldown and its existing @rolldown/binding-*
  packages 1.2.8 → 1.2.16, @oxc-project/types 0.149.0 → 0.156.0,
  @types/estree 1.0.9 → 1.0.10, nanoid 3.3.18 → 3.3.20 and postcss 8.5.28 →
  8.5.32. All retain MIT; no added package identities. Binding suffixes are
  android-arm-eabi, android-arm64, darwin-arm64, darwin-x64, freebsd-x64,
  linux-arm-gnueabihf, linux-arm64-gnu, linux-arm64-musl, linux-ppc64-gnu,
  linux-s390x-gnu, linux-x64-gnu, linux-x64-musl, openharmony-arm64,
  win32-arm64-msvc and win32-x64-msvc.
- Jest: glob 10.5.0 (ISC)/13.0.6 (BlueOak-1.0.0) → 14.0.3 (BlueOak-1.0.0);
  minimatch 9.0.9 (ISC)/10.2.6 (BlueOak-1.0.0) → 11.0.4 (BlueOak-1.0.0);
  path-scurry 1.11.1/2.0.2 → 3.0.0 (BlueOak-1.0.0). Remove the obsolete
  balanced-match 1.0.2, brace-expansion 2.1.7, lru-cache 10.4.3,
  ansi-regex 6.4.0, ansi-styles 6.2.3, emoji-regex 9.2.2, string-width 5.1.2,
  strip-ansi 7.2.0 and wrap-ansi 8.1.0 resolutions; other locked versions of
  those package names remain. Entirely removed identities are @isaacs/cliui,
  @pkgjs/parseargs, eastasianwidth, foreground-child, jackspeak,
  package-json-from-dist, string-width-cjs, strip-ansi-cjs and wrap-ansi-cjs.
- Newly added in React and Jest: @isaacs/nonrecursive-glob-match 1.0.3
  (BlueOak-1.0.0). No other package identity or licence family is added.

## Verification procedure

Use the selected toolchains and a fresh physical output directory for each run.
The ordinary workspace suite does not execute ignored native tests.

Local validation on 2026-11-03 passed the locked workspace suite, the expanded
bootstrap installer/recovery suite, and native React TypeScript (five scenarios)
and fullstack (four scenarios), including rejection/correction and prior-head
checks. Alternate Jest with Node 22.19.0/npm 10.9.4 failed at its baseline audit
assertion; node, syntax and unit sensors passed. Project-owned versions with
Node 26.10.0/npm 10.9.3 and 12.1.0 passed both frontend graphs and their negative
cases, then failed at the later Jest audit assertion. Both local raw failing audit
outputs contain only deepmerge GHSA-gcqx-w48m-w2h4 (five low entries). Reproduce
audits locally against the retained locks. Hosted logs identify the failing
assertions; the advisory details here come from those local audits.

Formatting and strict workspace/all-target Clippy passed. The CI-pinned lat.md
0.12.2 check, 195 relative Markdown link paths and `git diff --check` passed.
No local execution is a hosted CI success or authorization decision.

```console
cargo test --workspace --locked -- --test-threads=1
cargo run --locked -p policy-acceptance -- run --profile react-typescript --output /absolute/new-react --provision
cargo run --locked -p policy-acceptance -- run --profile fullstack --output /absolute/new-fullstack --provision
POLICY_JEST_OUTPUT=/absolute/new-jest cargo test --locked -p policy-acceptance --test native_jest -- --ignored --nocapture
POLICY_PROJECT_VERSIONS_OUTPUT=/absolute/new-project cargo test --locked -p policy-acceptance --test native_project_versions -- --ignored --nocapture
git diff --exit-code origin/main -- bootstrap/templates/1.1.0
lat check
git diff --check
```

The final two native tests are expected to fail only at the Jest audit. Their
raw npm output must contain deepmerge GHSA-gcqx-w48m-w2h4 and no remaining
glob/minimatch advisories. Such failures are retained failures, not green checks.
Installation regressions exercise fresh/repeat/uninstall, schema-2 receipts,
old-release upgrade/downgrade and interrupted install/removal recovery.

## Split-consumer follow-up

On 2026-11-03, the synthetic split-consumer frontend separately refreshed Vite
6.4.3 → 6.4.5, Vitest and @vitest/{expect,mocker,pretty-format,runner,snapshot,
spy,utils} 4.1.11 → 4.1.12, and source-map-js 1.2.1 → 1.2.2. Its schema-3 profile
package requirements and Vitest tool selection match the manifest and lock.
React 19.0.0, Playwright 1.63.0, Node 26.10.0 and npm 10.9.3 remain unchanged.
There are no new package identities, licence families or overrides; updated
Vite/Vitest packages retain MIT and source-map-js retains BSD-3-Clause.
Unrelated chai and tinyrainbow resolutions remain at 6.2.2 and 3.1.1.

Both baseline and updated `npm ci` and audits used Node 26.10.0/npm 10.9.3,
including dev, optional and peer dependencies at severity `low`. The baseline
audit exited 1 with four vulnerable package entries (two moderate, two high):
vitest, @vitest/mocker, vite and source-map-js. The updated audit exited 0 with
zero vulnerabilities. This addresses Dependabot alerts #24–#31, including
source-map-js [GHSA-68fv-2mgg-jv7q](https://github.com/advisories/GHSA-68fv-2mgg-jv7q)
and the Vite/mocker advisories listed above. Alert #9, the separate Jest
deepmerge failure, remains unchanged.

Initial lock generation with npm 10.9.3 failed with Arborist's internal
`Cannot read properties of null (reading 'edgesOut')` error. As with the earlier
refresh, npm 11.20.0 generated the lock (Node 26.13.0); a subsequent npm 10.9.3
`install --package-lock-only --ignore-scripts --no-audit --no-fund` succeeded
and removed libc metadata. A targeted npm 10.9.3 `update source-map-js` selected
1.2.2. After restoring the two unrelated Vitest-range refreshes, npm 10.9.3
normalized the lock again, installed it and performed the clean audit.
No audit policy or frozen bootstrap asset changes are included.

Local split-consumer staging and the opt-in native harness passed. The native
run used Node 26.10.0/npm 10.9.3, .NET 11.0.100-rc.1.26425.128 and Playwright
1.63.0 Chromium. All ten sensors passed on both baseline and corrected heads,
including one frontend unit test, one browser-to-API E2E test and the three
filtered backend suites. All 26 negative checks passed: empty/truncated/missing
reports and stale report bindings, actual Vitest/TRX/Playwright failures,
prior-head evidence, stale profile and changed framework props. These results
remain local advisory observations.

```console
cargo test --locked -p policy-acceptance --test native_split_consumer -- --test-threads=1
POLICY_SPLIT_CONSUMER_OUTPUT=/absolute/fresh-split-consumer cargo test --locked -p policy-acceptance --test native_split_consumer native_split_consumer_reports -- --ignored --nocapture --test-threads=1
```

The native command requires the selected tools on PATH and DOTNET_ROOT set to
the selected SDK installation, as in the [native procedure](consumer-schema3.md#native-verification).

The locked workspace suite, formatting, strict workspace/all-target Clippy,
deterministic schema regeneration, xtask residue/public-API checks, cargo-deny
0.20.2 and cargo-audit 0.22.2 also passed. The CI-pinned lat.md 0.12.2 reported
`All checks passed`; 185 relative Markdown link paths and `git diff --check`
passed. Hosted outcomes must be read from the exact-head PR checks separately.

## Alternate Vitest replacement

On 2026-11-03 the first Jest-transition PR adds `examples/react-vitest`, using
split-consumer Vitest 4.1.12/Vite 6.4.5, React 19.0.0, a Node environment with
`retry: 0`, and the existing alternate Node 22.19.0/npm 10.9.4/web-root SSR intent.
The former Jest fixture and jobs remain in this first PR.

npm 10.9.4 lock generation failed in Arborist (`edgesOut` on null). npm 11.20.0
then generated the lock, and npm 10.9.4 normalized it with
`install --package-lock-only --ignore-scripts --no-audit --no-fund`. The new lock
contains 96 dependency entries: 92 MIT, two ISC, one BSD-3-Clause and one
Apache-2.0 license declaration. There are no overrides. Its SHA-256 is
`2f40bc98a1335d8c7b0b62210ee4f7ade23bfc293e720ae03287cf9024079294`.

Local `native_vitest` ran with Node 22.19.0/npm 10.9.4 on macOS arm64 and passed
all nine cases: baseline, failed, corrected, skipped, empty, missing,
final-corrected, nested-audit-failed and nested-audit-corrected. The baseline's
selected `npm audit --audit-level=low --include=dev --include=optional
--include=peer` reported zero vulnerabilities. The skipped producer exited zero
and reported two pending tests, zero passed; normalization rejected that evidence.
The nested scenario observed legacy audit zero, a full-graph installed vulnerable
dev-only lodash 4.17.20, selected audit rejection and a passing corrected head.
Previous-head evidence was rejected. Requests/results, reports and logs remain in
local `target/native-vitest-pr1/`; this initial observation used a dirty harness
at base `7e25ab6e5d80deefdd9e69849cf956d6ef99d623`, not a hosted final-head decision.

Vitest JSON has no retry history. Configured retry zero is not equivalent to
Jest's success-after-retry rejection. This record establishes local advisory
coverage only; it does not establish hosted success, protected merge or promotion.
