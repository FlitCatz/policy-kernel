# Debt ratchet v1

The native contracts and verifier expose a standalone deterministic comparison
of static-analysis debt. Producers own language analysis, diagnostic extraction,
suppression detection, aggregation, and context hashing. The kernel accepts only
typed counts; it has no language parser or toolchain dependency.

## Inputs

`RatchetRuleV1` contains `schema_version: 1`, `approved_baseline` and
`head_baseline`. Each baseline is an array of `RatchetEntryV1` allowances.
`RatchetObservationV1` contains `schema_version: 1` and an `entries` array of
actual head diagnostic counts. The proposed head baseline is separate from the
head observations so unused allowance growth can be rejected.

Each entry has this shape (snake_case follows the native wire convention):

```json
{
  "key": {
    "tool": "analyzer",
    "rule": "R1",
    "path": "src/lib.rs",
    "symbol": "compute",
    "context_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  },
  "count": 2
}
```

The identity is the entire `(tool, rule, path, symbol, context_hash)` tuple.
IDs must be nonblank; context hashes are lowercase 64-character SHA-256 hex.
Paths use the existing lexical portable relative-path check: no absolute paths,
backslashes, colons, NUL, empty segments, `.` or `..` segments. The verifier does
not inspect the filesystem or normalize paths, symbols, IDs or hashes.

Counts are positive `u64` integers. Omit entries with zero counts; empty arrays
are valid and mean no debt or allowance. Duplicate keys, including duplicates
with different counts, are invalid within each of the three arrays. Unknown
fields or versions, missing fields, duplicate object fields, negative, fractional,
out-of-range or nonnumeric counts fail closed at the JSON boundary.

The caller must independently approve the base baseline, bind all three sets to
the intended repository/head/source inventory, and ensure comparable analyzer
and context-hashing configurations. These records do not authenticate approval,
execution, completeness, tool versions or suppression coverage.

## Evaluation

`policy_native_verifier::evaluate_ratchet(&rule, &observation)` returns
`Result<RatchetResultV1, policy_core::CanonicalError>` and performs no I/O.
`evaluate_ratchet_json(rule_bytes, observation_bytes)` adds strict JSON decoding
and returns malformed-input reasons rather than a passing default. Both inputs
are decoded independently. A malformed document has one `MALFORMED_INPUT` reason;
the other document's structural violations are still collected if it decodes.

For every complete key `k`, missing counts are zero:

```text
allowance(k) = min(approved_baseline(k), head_baseline(k))
head_baseline(k) <= approved_baseline(k)
observation(k) <= allowance(k)
```

Every increased head allowance is rejected, even if no observation uses it.
Decreases, equal counts and removed entries pass when every other entry satisfies
the bounds. A reduction at one key cannot offset an increase at another key.
Replacing a context hash at the same total count fails because the new context
has no approved allowance. No totals or subtractions are needed, so even multiple
maximum `u64` counts cannot overflow the comparison.

`RatchetResultV1` has `pass` only when `violations` is empty; otherwise it has
`fail`. All detectable structural violations are returned before comparison.
Invalid structures skip numeric comparison rather than selecting a duplicate's
count. With valid structures, every comparison violation is returned.

| Reason code | Meaning |
| --- | --- |
| `MALFORMED_INPUT` | A rule or observation JSON document cannot be decoded |
| `INVALID_COUNT` | A decoded entry has count zero |
| `INVALID_PATH` | An entry's path fails portable relative-path validation |
| `DUPLICATE_KEY` | A complete key occurs more than once in one input array |
| `ALLOWANCE_INCREASED` | A head baseline entry exceeds its approved allowance, including new entries |
| `DEBT_INCREASED` | Observed count exceeds the minimum allowance for an approved key |
| `CONTEXT_REPLACEMENT` | An observed unapproved context shares tool/rule/path/symbol with an approved key |
| `NEW_DEBT` | An observed key has neither an approved allowance nor an approved context group |

Violations contain `input`, optional `key`, and `code`. They are unique and
sorted by input (`rule`, `approved_baseline`, `head_baseline`, `observation`),
then key fields in tuple order, then reason code in the table-independent
declaration order of `RatchetReasonCode` (alphabetical wire-code order).
String comparison is Rust's case-sensitive lexicographic order, without locale
collation. Duplicate-key violations are emitted once per key. A parse violation
has a null key. Entry-array reordering preserves the verdict and violation list.

## Digests and schemas

Validly structured comparisons include `rule_digest` and `observation_digest`;
both are null on structural or parse failure. Comparison failures still include
the digests of their valid inputs. `digest()` is also available on rule,
observation and result records; hashing alone does not validate or authenticate
a record.

The existing native canonical envelope is reused:
`{"schema":"policy_native_v1","role":ROLE,"value":RECORD}`. Roles are
`ratchet_rule`, `ratchet_observation`, and `ratchet_result`. Object keys are sorted
and only integral numbers are allowed. Arrays retain wire order, so reordering
entries changes input digests even when the verdict and violation order agree.
Existing native canonical vectors remain unchanged.

Crate-owned schemas:
[rule](../../crates/policy-native-contracts/schemas/ratchet-rule-v1.schema.json),
[observation](../../crates/policy-native-contracts/schemas/ratchet-observation-v1.schema.json),
[result](../../crates/policy-native-contracts/schemas/ratchet-result-v1.schema.json).
Schemas describe wire shape; duplicate-key and path checks still require typed
validation/evaluation. Unlike the existing scope documents, ratchet input
decoding checks wire types without invoking semantic validation, so evaluation
can collect all structural violations together. Direct Rust callers receive the
same entry validation. Regenerate schemas using the existing Rust
`generate_assets` example; run conformance with:

```bash
cargo fetch --locked
cargo test --locked -p policy-native-contracts -p policy-native-verifier
```

## Trust and integration boundary

This API makes a mathematical comparison only. A passing result cannot establish
trusted execution, satisfy review, authorize a merge, or promote a baseline.
Digests identify content and do not attest the producer. See the
[trust model](../trust-model.md).

The existing scope evaluator, importer, CLI and pre-merge contracts are unchanged.
Scope integration is follow-up work: it needs an independently approved ratchet
requirement and source/run bindings before a result can satisfy any scope gate.
No consuming-project behavior is changed by this library addition.
