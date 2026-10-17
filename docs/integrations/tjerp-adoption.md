# TJERP split-profile acceptance (P7)

P7 adds Rust glue for project-owned TJERP backend and frontend profiles. It calls
`project_verification::run::prepare`, `project_verification::run`,
`evaluate_sensor_evidence`, `validate_bundle`, and advisory `validate_provenance`.
It produces **local scope satisfied** only when both profiles and the selected
catalog criteria pass. Every observation remains `LocalUntrusted`; no trusted
import, fake PR, independent review, authoritative decision or promotion is added.

The tests in `acceptance/tests/tjerp_adoption.rs` are **synthetic TJERP-shaped
protocol fixtures**, including their Rust subprocess reporters and temporary Git
commits. Their schema-1 process profiles isolate the protocol from native tools;
the production entry point requires backend schema 3 and frontend schema 4.
They do not compile or mutate TJERP C#/Svelte, run its architecture/FSD
checks, or establish its adoption. P8 owns the real split profiles and native run
procedure; P9 owns the first real dry-run and actual source fault injections.
The C# family remains schema 3 / selection 2; the frontend remains schema 4 /
selection 3. No tool installation or dependency download is performed by this
example. Profile commands must be reviewed separately, including any audit restore.

## Inputs and invocation

Build with installed dependencies only:

```console
cargo build --locked --offline -p policy-acceptance --example tjerp_adoption
target/debug/examples/tjerp_adoption --help
target/debug/examples/tjerp_adoption run --help
cargo test --locked --offline -p policy-acceptance --test tjerp_adoption
```

The example accepts only `run`, `--mode local-advisory`, and `--scenario baseline`.
Fault scenarios are acceptance tests, not an option to mutate a real checkout.
With approved tools, dependencies, P8 profiles, discovery reports and server
lifecycle already prepared by the operator, its interface is:

```console
target/debug/examples/tjerp_adoption run \
  --checkout /private/tmp/tjerp-run/source \
  --source-repository /path/to/read-only/TJERP \
  --source-revision <full-TJ-commit-sha> \
  --kernel-revision <full-PK-commit-sha> \
  --profiles-directory /private/tmp/tjerp-run/source/config/policy \
  --catalog /private/tmp/tjerp-run/source/config/policy/requirements.json \
  --catalog-digest <externally-approved-file-sha256> \
  --discovery-directory /private/tmp/tjerp-run/discovery \
  --output /private/tmp/tjerp-run/evidence \
  --provider <actual-provider> --model <actual-model> --execution-id <unique-run-id> \
  --mode local-advisory --scenario baseline --max-age-seconds 3600
```

Angle-bracket values are placeholders, not executable shell arguments. The source
must be a separate canonical Git archive without `.git`; evidence output must
be a new directory outside both source trees. Existing artifacts are never
overwritten. The kernel revision is recorded as a caller claim alongside the
actual harness binary SHA-256, not authenticated as a trusted build identity.
The local repository ID is `local/TJERP`, not a GitHub observation.

Discovery must come from the same archive path. Place the six unmodified raw
reports under `discovery/backend/.policy-reports/{unit,integration,contract}.trx`
and `discovery/frontend/.policy-reports/{unit,e2e,csp}.json`. Move discovery
outputs out of the source report paths before calling the harness: final
collection requires fresh, absent output paths. Discovery is explicitly unscored.
The example does not start API/SQLite/Playwright servers or generate discovery;
P8 must define and own readiness, exclusive ports, child cleanup, generation
checks and approved audit/network preparation before P9 execution. It executes
the reviewed profile argv arrays sequentially, backend then frontend.

Exit codes are 0 for local scope satisfied, 2 for a completed core evaluation
with non-pass reasons, and 64 for input, preflight, collection or binding errors.
A missing normalized result is an error, never a successful empty result.
Clap argument/syntax errors use exit 2; help uses exit 0.

## Catalog and report binding

The external approved digest hashes the original catalog bytes. Typed profile
and sorted source-inventory digests use `canonical_sha256`; these digest types
must not be substituted for the file hash. The checker reads the actual TJERP
`requirements.json`/`suites.json` shapes; historical `execution_snapshot` and
`execution_reference` IDs are deliberately unused. Catalog metadata is retained
in the source inventory but is not execution evidence.

The checker requires unique namespaced criterion IDs, exact parent-ID tokens in
`.planning/REQUIREMENTS.md`, nonempty selector lists, and each of AC/B/N nonempty
per component. Negative cases also belong to changed behaviors. Lists computed
from items must exactly equal both catalog summaries and profile policies; an
externally fixed catalog digest also rejects joint catalog/profile shrinkage.
This checks the reviewed pilot catalog; it does not infer completeness of natural
language requirements. Business owners still review the chosen AUTH-01 C# and
STMT-04 screen scope.

All six suites must be required, nonempty and bound to matching report formats,
paths and argv, with retry/max-skipped zero. Backend class lists cannot overlap.
Native discovery/classification completeness remains a P8/P9 input and execution
obligation; six synthetic reports do not establish real test-discovery completeness.
The profiles also require process sensors `backend:build`, `backend:api-audit`,
`backend:modules-audit`, `backend:tests-audit`, `frontend:fsd`, `frontend:build`,
`frontend:specs`, `frontend:api` and `frontend:audit`.

The existing parser supplies IDs and outcomes. The harness zips raw identities
in that parser's traversal order to its normalized IDs; it never invents hashes
or rewrites reports. Vitest matches exact source-relative path plus `fullName`;
absolute names must belong to the current archive. Playwright resolves `file`
against raw `config.rootDir` and matches exact title. Ambiguous matches fail.
TRX matches the catalog FQN/class/source filename and preserves every flat
parameterized row's original `testId`/`testName`; exact matches cannot be mixed
with multiple theory rows. Failed, skipped, renamed, zero, malformed and retry
reports cannot satisfy a selector. Each required regression selector also passes.
Final-run mappings must equal discovery mappings and reference passed tests from
the final raw reports, within the same component namespace.

## Inventory, results and retained artifacts

The harness compares archive bytes to `git ls-tree -r` and `git cat-file blob`
for every tracked regular file except `authenticator/`, and requires the supplied
revision to equal the source repository's current HEAD. This deliberately includes
more documentation than the minimum pilot scope. It rejects scoped symlinks,
submodules, changed/missing files, and extra source files. Runtime directories
`node_modules`, `bin`, `obj`, `.policy-reports`, `.svelte-kit`, `test-results`,
`playwright-report`, `build`, and `dist` are excluded from the extra-file walk;
tracked files there are still checked against Git. Tool/package/generated-output
checks remain owned by project-verification. This is local consistency, not a
certificate of repository ownership or inventory approval.

Both requests bind the same root, repository, actual commit and sorted inventory.
Unknown impact executes all sensors. Each result separately binds its profile,
configuration, parser, command, collection, raw report, source and head. Core
checks sensor/adequacy obligations and advisory freshness; manifest child digests
are checked separately. The source archive is compared with the external commit
again after collection. Trust claims above LocalUntrusted are rejected; there
is no JSON result-import command.

Output includes `context.json`, unscored discovery identities and raw reports,
`{backend,frontend}.request.json`, prepared bindings, and available normalized
results (including process stdout/stderr). Final raw reports and identity tables
are saved even if the existing project runner returns an error before producing
a `RunResult`. `requirements.json` links each criterion to observed test outcomes
and marks missing/failed selectors failed/unproven. Collection failures produce
`failure.json`; successful evaluation produces `scope.json` with core reason
codes, profile/source digests, exclusions and the unfulfilled review gate.
If collection aborts without a normalized result, the existing runner API does
not return its intermediate process records; that absence is preserved rather
than inventing command logs. P9 must retain native command logs externally too.

Flutter/Dart, devices, live authentication handlers, DB2 and production
CI/required-check/merge/deployment guarantees remain excluded. See the
[project verification contract](project-verification.md), [adequacy contract](adequacy.md)
and [trust model](../trust-model.md).

## Synthetic fault matrix

Each row has an explicit non-pass assertion in the Rust acceptance suite.
FI-00 is a positive baseline; source-level native demonstrations are deferred to P9.

| Case | Synthetic injection and expected result |
|---|---|
| FI-00 | Six complete reports plus FSD process; both core results pass, local scope satisfied, LocalUntrusted, review unfulfilled. |
| FI-01 | Failed backend contract assertion; AUTH-01 AC and negative are failed/unproven despite other successes. |
| FI-02 | Failed backend unit/architecture-shaped report; backend:unit fails and composition is non-pass. |
| FI-03 | FSD-shaped process exit 1 with layer-direction stderr; frontend:fsd fails. |
| FI-04 | Missing required report despite process exit 0; collection non-pass, raw reports retained, no scope success. |
| FI-05 | Old head/results replay; split/result mismatch and PK-PROV-BINDING-MISMATCH; actual temporary old-commit archive mismatches current Git source. |
| FI-06 | Two models/executions sharing a reviewer provider; PK-PROV-DUPLICATE-PROVIDER even in advisory mode, plus PK-SELF-PROVIDER when the modifier shares that provider. |
| FI-07 | Empty policy with absent or positive-count evidence; PK-ADEQ-POLICY-EMPTY and catalog completeness failure. |
| FI-08 | Missing AC/negative mapping or unexecuted ID; PK-ADEQ-CRITERION-UNMAPPED, PK-ADEQ-BEHAVIOR-UNMAPPED or PK-ADEQ-UNEXECUTED-MAPPING. Report/selector regressions also reject skip, zero, malformed, renamed, ambiguous and retry inputs. |
| FI-09 | Caller changes local trust; harness TJ-TRUST rejection. Authoritative core evaluation of original local evidence returns PK-PROV-UNTRUSTED. |
