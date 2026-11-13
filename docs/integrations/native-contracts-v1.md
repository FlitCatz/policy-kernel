# Native contracts v1

The additive [debt ratchet v1](debt-ratchet-v1.md) defines a separate standalone
comparison API in these crates. It does not change scope evaluation or import.

**v1 types, an adapter importer and a kernel-owned local scope verifier.**
`policy-native-contracts` is the additive Rust contract crate for already-executed
native runs. It neither executes commands nor reads files, Git, reports or the
network. It does not construct core evidence or evaluate policy. Existing core
contracts, approved digests, five root schemas and TJERP execution are unchanged.

## Planned local-runner attestation

The
[2026-11-12 decision record](../decisions/2026-11-12-local-runner-attestation-adr.md)
plans authenticated acceptance of a signed local-runner statement and approved
scope-template digest. The sibling [wire contracts below](#local-runner-attestation-contracts)
are implemented; authenticated acceptance remains planned. The three original v1
documents and `policy-cli evaluate-scope` remain unchanged: results stay
`local_untrusted` with an unmet review gate. Signature verification belongs to
the future authenticated collection boundary.

## Documents and validation

Each document has its own `schema_version: 1`, independent of core policy v3,
ProjectProfile, selection and parser versions. Unknown versions and fields are
rejected, including nested fields. Rust callers can construct records and call
`validate()`; top-level Serde deserialization calls the same validation.

| Type | Contents |
| --- | --- |
| `ExpectedScopeV1` | Independently approved repository/head/source subject; raw requirements-file digest; required observation IDs and exit/count rules; AC, changed-behavior and negative-case requirements with suite mapping rules; exemptions; command/configuration/parser bindings; evaluation time, positive maximum age and local-advisory mode |
| `NativeRunV1` | Subject plus run/scope IDs; observations with execution ID, argv, relative cwd, times, completion and configuration digest; parser bindings, raw report references, counts and test identity/outcomes/attempts; artifacts, mappings and exclusions |
| `LocalScopeResultV1` | Satisfied/non-pass/error status, subject, per-requirement results, stable reason codes and suite/requirement/artifact locations, input/policy/optional evidence digests, evaluation time, verifier/adapter versions, local-untrusted trust and unmet review gate |

Required observations and requirements are nonempty. Requirements carry an
explicit kind; a category may be absent when outside the approved consumer scope,
but the overall approved requirement set cannot be empty. Required suites,
accepted exit codes and mapping test references are also nonempty and unique.
Requirement, observation, execution, artifact and exclusion IDs must be unique
within their respective namespaces. Repeating a record ID with different contents
is rejected. Normalized test IDs are unique within a suite; references include
both suite and test ID. Requirement kinds share one ID namespace.

Bindings must cover exactly the required observations. Only suite rules have
parser bindings. Requirement suite references must resolve to declared suites.
Test counts must match their records: passed/failed tests have positive attempts;
skipped/not-executed tests have zero attempts; retries sum attempts minus one for
executed tests. Mappings must reference executed tests, which may have failed.
A failed test remains data for a later policy evaluator, never an implicit pass.

Observation subjects must equal the run subject. `validate_subject_binding`
also compares the run with the independent expected subject. This checks typed
consistency only; matching strings do not authenticate a source checkout.
Observation timestamps must be ordered. Relative artifact/cwd paths reject
absolute paths, parent/dot components, empty components, backslashes, colons and
NUL; cwd alone may be `.`. This is lexical validation, not filesystem containment
or symlink checking. Raw test identity remains opaque and may retain absolute
producer paths. Repeated argv tokens and empty non-program arguments are valid.

Incomplete runs can preserve artifacts without test observations or mappings.
An error result may omit evidence and requirement results, but must retain a
reason. Satisfied results require evidence, nonempty satisfied requirements and
no failure reasons. Non-pass results require evidence and reasons. Only
`local_untrusted` and review `unmet` deserialize; no authorization state exists.
Reason codes describe failures and locations; their presence does not authenticate
a supplied result or recompute its decision.

The adapter importer below checks artifact bytes, subject/freshness and raw report
claims against approved parser bindings. Required-set completeness, approved
mapping/exclusion enforcement, command/configuration binding evaluation and
scope decisions belong to the typed verifier described below. JSON Schema covers wire
shape; Rust validation additionally covers ID collisions, references, count/time
consistency and digest roles. Consumers must use both layers as appropriate, not
substitute schema validation for these relational checks.

## Digest boundary

`RawByteDigest` is `{kind: "raw_byte_sha256", sha256}` over exact external file
bytes. This crate records it; it does not read or hash files. `CanonicalDigest`
is `{kind: "canonical_sha256", schema: "policy_native_v1", role, sha256}`.
The canonical preimage is exactly `{schema, role, value}`, using the existing
`policy-core` canonical JSON helper: sorted object keys and integers only.
The schema discriminator and role are inside the hashed envelope, not just labels.
Raw and canonical objects cannot deserialize into each other's fields. Source-set,
scope-definition, command, configuration and result digest roles are checked.

`canonical_digest(role, value)` and each document's `digest()` use that envelope.
Array order is preserved, including required-set order; sets are never silently
sorted or deduplicated. Hash a validated typed record, not an arbitrary JSON
object, to obtain its document digest. Canonical serialization normalizes existing
UTC timestamp types. Absolute cwd paths are forbidden; original report identities
may retain them in run data. Callers must keep checkout location out of approved
configuration values; this crate cannot inspect opaque configuration digests. Legacy approved
hashes are never silently replaced by these new digests.

## Schemas and corpus

The crate owns three scope [schemas](../../crates/policy-native-contracts/schemas/)
alongside the [debt ratchet](debt-ratchet-v1.md) and
[local-runner attestation](#local-runner-attestation-contracts) schemas,
[document vectors](../../crates/policy-native-contracts/tests/corpus/v1.json) and
[canonical vectors](../../crates/policy-native-contracts/tests/corpus/canonical-v1.json).
Document vectors contain valid records and invalid pointer replacements with
expected JSON Schema outcomes; relational failures may be schema-valid. For the
`pair` case, replace the run pointer and copy its subject to all observations,
then check it against the unchanged expected scope. Example artifact/policy
hashes are placeholders, not real execution or authenticated evidence.

```console
cargo test --locked -p policy-native-contracts
cargo run --locked -p policy-native-contracts --example generate_assets
```

The Rust development example regenerates only this crate's schemas and corpus.
Tests compare committed schemas byte-for-byte, validate corpus schema outcomes,
reject typed-invalid vectors and pin canonical hash values. The workspace CI
runs these tests, including on Rust 1.85.0 and 1.95.0. It does not run a native
consumer or establish promotion. See [Task 18](../tasks/task-18/README.md) and
[Current status](../status.md).

## Local-runner attestation contracts

`LocalAttestationV1` is a sibling signed statement with `schema_version: 1` and
the fixed domain `policy-kernel/local-runner-attestation/v1`. It binds a
`SubjectV1`, head tree and comparison base (40-character lowercase Git SHAs),
run/scope IDs, expected-scope/template/run/imported-observations/result digests,
runner ID, OS, architecture, attestor version and ordered
`started_at <= completed_at <= attested_at` timestamps. No PR number is required:
the run may precede push and PR creation. Runner metadata is nonblank opaque data,
not an allow-list or proof of platform identity.

`SignedLocalAttestationV1` contains exactly `statement` and `signature`.
The signature has format `sshsig`, the same runner ID as the statement and an
opaque nonblank `value`. Unknown fields/versions, malformed IDs/digests, wrong
digest roles, reversed timestamps and mismatched runner IDs fail typed decoding
and `validate()`. JSON Schema checks shape; timestamp, role and runner equality
checks require Rust validation. A structurally valid signature can be arbitrary
text. This crate performs no cryptography, observation, trust promotion or policy
evaluation. A result digest does not prove that its result was satisfied; the
authenticating consumer must check all referenced documents and recompute scope.

Signatures cover `policy_core::canonical_json(&statement)`, without a digest
envelope. The statement's separate `digest()` is a document identity using the
usual `{schema, role, value}` envelope. The new roles `scope_template`,
`imported_observations` and `local_attestation` use `LocalDigestRole` / `LocalDigest`
and `canonical_local_digest`, sharing the existing canonical envelope encoder.
Like the standalone ratchet roles, these are separate from `DigestRole` so that
every existing v1 schema, document and digest remains byte-identical. The
observations helper accepts a serializable value; its caller must validate the
typed `ImportedObservationsV1` before hashing it.

`ExpectedScopeV1::template_digest()` hashes role `scope_template` with exactly
this value projection (array order remains significant):

```text
{requirements, observations: bindings.observations,
 maximum_age_seconds: bindings.maximum_age_seconds, mode: bindings.mode}
```

It includes the requirements-file digest, all requirements, observation rules,
count bounds, exemptions, command/configuration/parser bindings and time/mode
policy. It excludes the entire source subject, document version and
`bindings.evaluated_at`; the enclosing digest retains `schema: policy_native_v1`.
The full expected-scope digest still covers those excluded fields.

The merged [ratchet contract](debt-ratchet-v1.md) stores approved/head allowances
in standalone `RatchetRuleV1`, whose `digest()` covers both arrays. There is no
ratchet baseline or rule digest in `ExpectedScopeV1`, its requirements or its
bindings, and standalone ratchet results do not satisfy scope. Thus this template
digest does **not** approve a ratchet baseline. Future scope integration must bind
the independently approved baseline/rule into its approved template before it
can rely on local attestation; no v1 field is added here to imply that guarantee.

The two sibling [schemas](../../crates/policy-native-contracts/schemas/),
[document corpus](../../crates/policy-native-contracts/tests/corpus/attestation-v1.json)
and [canonical vectors](../../crates/policy-native-contracts/tests/corpus/canonical-attestation-v1.json)
are generated with the other native assets:

```console
cargo run --locked -p policy-native-contracts --example generate_assets
cargo test --locked -p policy-native-contracts
```

The corpus includes schema-valid relational failures. Canonical vectors pin new
roles and the original three document digests; existing schema/corpus tests still
compare their assets byte-for-byte. Fixture signatures and digests are synthetic;
these tests do not establish actual runner execution, key possession,
authoritative readiness or promotion.

## Adapter import library (PR 4)

`policy-native-importer::import_run(run_directory, output_directory, expected,
identity, run)` is a Unix library, also exposed by the `policy-native-importer`
command described below. It never
executes argv, selects tools, reads Git, downloads artifacts, converts evidence or
constructs `LocalScopeResultV1`. Kernel crates do not depend on it. The existing
runner, core contracts, schemas, mutation pins and TJERP paths are unchanged.

`ExpectedIdentity { run_id, scope_id }` is a mandatory independent caller input:
`ExpectedScopeV1` has neither field, and its v1 schema is unchanged. The caller
must obtain fresh expected identity, subject and evaluation time independently,
not copy them from the run. This is consistency/replay checking, not a persistent
registry of used run IDs or authentication of producer claims.

The run directory contains exactly the declared artifacts and their ancestor
directories; input JSON records are passed separately, not implicit extra files.
All relative paths must be portable, and all directory components (including
parents of input/output roots) must be symlink-free. Files are opened relative to
pinned directory descriptors with no-follow and nonblocking flags. Symlinks,
special files, missing files, undeclared files and extra empty directories are
rejected. Limits are 16 MiB per file, 256 MiB total, 4096 entries and 32 nested
directory levels. Raw SHA-256 is over the bytes copied and parsed, never reserialized
JSON. Before/after inventories detect ordinary concurrent changes; producers must
quiesce the directory, and this is not isolation from a hostile concurrent writer.

The entire source subject (repository/head/source-set/source-inventory/scope-definition
digests) must match, as must independent run/scope IDs and every observation's
subject. Ordered timestamps must not be in the future or older than maximum age,
measured from observation start against the supplied evaluation time, including
subsecond precision. The raw report start must fall within its observation interval.
Source digests are compared, not recomputed from a checkout or authenticated.

`jest_json` / `jest-json/v1` remains supported by the Rust importer for historical
raw evidence, independently of ended managed Jest runner/CI guarantees.

Parser IDs are `vitest_json`, `jest_json`, `playwright_json` and `trx`; versions must
exactly equal the corresponding shared `ReportFormat::parser_version()` and the
expected suite parser binding. The namespace is the observation ID. The shared
parser remains authoritative for report validity, normalized IDs, outcomes,
counts and diagnostic messages. The importer supplements it with a projection
of raw identities and attempts; no parser behavior is changed.

For these adapters, `TestV1.raw_identity` is a compact JSON string encoding the
original two identity components, retaining any absolute producer path:

| Parser | Components, in order |
| --- | --- |
| Vitest/Jest | suite `name`, assertion `fullName` |
| Playwright | spec `id`, test `projectId` |
| TRX | result `testId`, result `testName` |

Test arrays must match parser traversal order. Claims must agree on every count,
normalized ID, raw identity, outcome and attempt count. Empty, skipped or retried
reports receive verification issues. Unsupported parser versions, malformed raw
reports, duplicate suites and contradictory summaries cannot silently pass.
Valid failed tests and process failures remain data; the importer does not compare
exit codes/counts to policy bounds or declare requirements satisfied. Parser-level
`successful` and messages remain in the audit record so downstream consumers
cannot lose a run-level failure that is not represented by test counts alone.
Required observation coverage, mappings/exclusions and command/configuration
policy evaluation belong to the kernel verifier. No issues means only that these import checks
found no disagreement, never that local scope is satisfied.

The output parent must exist, the output must be outside the input tree, and the
output directory must not already exist. Output contains `expected.json`,
`identity.json`, `run.json`, numbered artifact copies and `import.json`, mapping
original paths to copies, hashes, sizes, recomputed observations and issues.
Only an issue-free import also writes `observations.json` and exposes
`ImportRecord::verified_observations()`. This export carries the recomputed run,
independent expected-scope digest and run/scope IDs, adapter version, local trust,
unmet review and `unsuccessful_reports` IDs. The latter preserves parser-level
failures such as suite diagnostics even when every test count passes.
Records always say `local_untrusted` and review `unmet`. Regular extra files and
hash-mismatched files are copied for diagnosis within the limits. Unsafe,
oversize or unreadable artifacts are never followed or deleted: originals remain
in place, with issues explaining why they could not be copied. Inventory failure
also leaves source untouched and records the failure. The caller must retain the
source directory for such incomplete snapshots.

A synced pending marker is atomically linked as `complete` after all outputs are
written and synced. It marks a persisted import record, including rejected
imports, not a successful scope. Filesystem/output errors return `io::Error` and
leave any partial directory in place; absence of `complete` means incomplete
publication. Existing output is never overwritten. Local fixture verification
does not establish native adoption, authoritative CI or promotion.

```console
cargo test --locked -p policy-native-importer
```

## Native import command

The Unix `policy-native-importer` binary calls `import_run` directly; it does not
implement a second validator. It is separate from `policy-cli` to preserve the
existing dependency direction: the JSON-only CLI and kernel verifier do not
depend on the importer or report adapters. Both binaries ship in the release.

Keep the typed input documents outside the artifact-only run directory. Obtain
the approved scope, run ID, scope ID and evaluation time independently of the
producer's run document; copying those expectations from the run defeats replay
checking. Quiesce the artifact directory before importing. The output parent
must exist, its path components must be symlink-free, and the output must be a
fresh directory disjoint from the run directory.

```sh
policy-native-importer --run-directory native-artifacts --output imported-run \
  --expected approved-scope.json --run native-run.json \
  --run-id independently-expected-run --scope-id independently-expected-scope
policy-cli evaluate-scope --expected approved-scope.json \
  --observations imported-run/observations.json
```

Importer exits are **0** for a persisted verified import, **2** for a persisted
rejected import with issues, and **64** for arguments, typed JSON, identity or
filesystem/output errors. Help/version exit 0. JSON inputs are file paths only;
stdin is not supported. The importer writes no stdout; inspect `import.json` for
issues. Only exit 0 exports `observations.json`. A completed rejected import still
has `complete`; a filesystem error may leave a partial output directory. No
existing output is overwritten. A verified import can contain valid failed tests:
exit 0 is not a satisfied scope, and scope evaluation remains a separate step.

## Claim preparation from raw reports

For producers that do not implement Rust report parsing, the importer command
accepts repeated `--report OBSERVATION=ARTIFACT` options. The input `NativeRunV1`
must have empty `test_observations` and `mappings`, with observations and raw-report
artifacts already declared, including their exact byte digests. Parser IDs and
versions come from the independently approved expected-scope suite bindings.
The command calls `populate_claims`, reusing the importer's `reports::recompute`
and shared TRX, Vitest and Playwright parsers, then calls `import_run` on the
populated run. Historical Jest parsing remains compatible with the same path.
Consumers need neither Rust code nor duplicated report parsers.

**One raw report per observation.** Multi-project `dotnet test` runs produce one
observation per TRX, with distinct observation and execution IDs per test project.
The consumer lists every project as a separate required suite and parser binding
in `ExpectedScopeV1`. Do not merge TRX files or attach several reports to the same
observation; duplicate observation/artifact selections are rejected. Actual
command, subject, timing and configuration data remain producer inputs.

```sh
policy-native-importer --run-directory native-artifacts --output imported-run \
  --expected approved-scope.json --run native-run.json \
  --run-id independently-expected-run --scope-id independently-expected-scope \
  --report backend-domain=domain-trx --report backend-api=api-trx \
  --map AC-1=backend-domain --map AC-1=backend-api
policy-cli evaluate-scope --expected approved-scope.json \
  --observations imported-run/observations.json
```

`--map REQUIREMENT=SUITE` explicitly claims that **all executed tests** in that
selected suite cover the requirement, including failed tests. Repeat it to combine
suites for one requirement; requirement and suite must be independently approved
in the expected scope. No mappings are inferred when `--map` is absent. Use the
ordinary import path with producer-authored claims when coverage needs individual
test selection. Mapping flags do not prove semantic test adequacy or approve scope.
Selection and parser traversal order are preserved. Native tests/counts/raw
identities/attempts are generated by the same projection used for verification;
parser-level failures survive import and remain non-pass during evaluation.

Preparation reuses descriptor-relative, no-follow reads, bounded inventories,
16 MiB report / 256 MiB selected-report limits and raw hash checks. It detects
ordinary inventory changes; producers must quiesce the run directory. Invalid,
unsafe, malformed or ambiguous preparation exits 64 before creating import output.
Successful preparation still goes through every normal `import_run` check, including
freshness, subject, complete artifact inventory and independent run/scope identity.
The persisted `run.json` contains the generated claims and mappings. The original
envelope is unchanged, and no existing output is overwritten. The three native
v1 wire contracts and their existing canonical vectors/schemas are unchanged.

## Pinned binary releases

The [release workflow](../../.github/workflows/release-binaries.yml) builds both
commands with Rust **1.95.0**, `--locked` and `--release` on native macOS arm64 and
Linux x64 runners. An owner-created `v[0-9]*` tag triggers publication. No initial
release is claimed here; the owner cuts it after merge. Linux binaries are built
on Ubuntu 22.04 (glibc 2.35); they are GNU/glibc binaries, not musl binaries.
macOS binaries are built on macOS 15 arm64. CI smoke tests each native command.

Each version has four raw executable assets:
`policy-cli-<target>` and `policy-native-importer-<target>`, for targets
`aarch64-apple-darwin` and `x86_64-unknown-linux-gnu`. `SOURCE_COMMIT` records the
full source commit; `SHA256SUMS` covers all four executables and that record.
The Rust packager `xtask release-assets INPUT OUTPUT COMMIT` requires both targets,
copies exact bytes into a fresh output directory, and hashes those bytes. Its
input layout is `INPUT/<target>/<command>`. The workflow creates a draft only
after both builds succeed, uploads all assets, then makes the release public.

After a release exists, replace `vX.Y.Z` with the reviewed exact tag and select
the target explicitly. The URL pattern is
`https://github.com/FlitCatz/policy-kernel/releases/download/<tag>/<asset>`.
Run in a fresh download directory:

```sh
set -eu
tag=vX.Y.Z
target=aarch64-apple-darwin # Linux x64: x86_64-unknown-linux-gnu
base="https://github.com/FlitCatz/policy-kernel/releases/download/$tag"
for file in "policy-cli-$target" "policy-native-importer-$target" SOURCE_COMMIT SHA256SUMS; do
  curl --fail --location --output "$file" "$base/$file"
done
shasum -a 256 --check --strict --ignore-missing SHA256SUMS
# Linux alternative: sha256sum --check --strict --ignore-missing SHA256SUMS
install_dir="$HOME/.local/bin"
mkdir -p "$install_dir"
install -m 755 "policy-cli-$target" "$install_dir/policy-cli"
install -m 755 "policy-native-importer-$target" "$install_dir/policy-native-importer"
```

Check `SOURCE_COMMIT` against the reviewed tag's commit. Pin the exact tag and
the two platform-specific SHA-256 values from the reviewed manifest in consumer
configuration. For recurring CI installs, verify against those committed checksum
entries; do not use `latest` or build from source on every run. Downloaded
checksums detect byte corruption but do not independently authenticate the release
publisher. Binary distribution does not promote observations or grant readiness.

## Kernel verifier and local command

`policy-native-verifier::evaluate_scope(expected, imported)` accepts an independent
`ExpectedScopeV1` and `ImportedObservationsV1`. The latter is a strict version-1
kernel-owned transfer record, exported by the adapter; it is not an attestation.
The dependency direction is importer → verifier → native contracts/core. Neither
`policy-cli` nor the verifier depends on the importer or report adapters.

The verifier checks the expected digest, complete subject/run/scope identity,
exact required observation set, freshness, command digest (argv and relative cwd),
configuration and parser bindings. Every exclusion must match its approved entry,
including its explanation; exclusions never waive required observations or
mappings. Unknown mapping requirements fail. Missing mappings, insufficient mapped
executions, wrong allowed suites and failed tests cannot satisfy requirements.
Malformed/unexecuted references fail structural validation. Empty policies cannot
decode. Incomplete processes, absent reports and skipped/retried/partial reports
cannot yield successful evidence.

Assembly produces `EvidenceManifest`, `ExecutionEvidence`, `SensorEvidence`,
`TestAdequacyEvidence`, `SensorProfile` and `TestAdequacyPolicy`, delegating sensor
and adequacy evaluation to existing policy-core functions. Native minimum/maximum
bounds add constraints; they cannot relax core all-tests-passed rules. Independent
negative IDs become required core changed behaviors with negative obligations.
Suite/test tuple encoding prevents ID collisions. Parser-level failure flags
produce failed sensor evidence even with passing test counts.

The diagnostic Rust return includes core reasons and optional evidence separately
from `LocalScopeResultV1`. Invalid structural records return an error; incomplete
binding/input checks yield an error result without evidence. Valid evaluated
failures retain evidence and stable reason codes with suite/requirement/artifact
locations. Policy digests bind native requirements, approved file digest, bounds,
exemptions, bindings and time policy as well as the core profile. Evidence digests
cover the assembled bundle and its child digests; the expected and run digests
retain their separate roles.

All execution/sensor trust is `local_untrusted`; review is always `unmet` and the
review list is empty. Legacy execution identity slots explicitly record
`local-recorder` / `not-a-model`; neither represents an AI identity. The legacy
manifest uses the same head for base/head and a `local:` run label, without a
synthetic PR or authenticated workflow. No pre-merge evaluator is called and no
ReadyToMerge path exists. No core source or mutation-contract pin changes are
required by this façade.

```sh
cargo run --locked -p policy-cli -- evaluate-scope \
  --expected approved-scope.json --observations imported-run/observations.json
```

Exit 0 means satisfied, 2 valid non-pass, 64 invalid/incomplete input. Either path
may be `-` for stdin, but not both. A typed evaluation prints one result JSON;
structural/JSON/I/O errors print a diagnostic to stderr without a result. Existing
commands retain their behavior and exit codes. The command does not run an
importer, execute native commands or observe artifact files. Handwritten export
JSON has the same local-untrusted status: typed agreement cannot authenticate a
claimed parser execution or independently approve consumer policy.

Conformance runs with `cargo test --locked -p policy-native-verifier -p policy-cli
-p policy-native-importer`. Adapter-side tests pass fixture bytes through importer,
verifier and the command function, and compare core failure reasons with the
project-verification normalizer on the same bytes. Separate binary tests check
CLI argument parsing, stdin and exits. These are synthetic conformance checks,
not a new TJERP/native adoption or authoritative execution record.
