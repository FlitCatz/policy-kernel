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
