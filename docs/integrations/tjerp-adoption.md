# TJERP harness ownership

TJERP now owns its harness in `tools/policy-adoption` (TJERP main `3c62b9e`).
It pins policy-kernel `rev = 4df10fa` over public HTTPS; removal of the product
code from current policy-kernel does not change that revision or its artifacts.
The catalog, parent and split-scope assertions belong to TJERP.

Plan PR 8 removes the former policy-kernel module, examples and product tests.
The generic [split-consumer fixture](../../examples/split-consumer/README.md)
retains synthetic report/audit/version conformance coverage.

See [Task 18](../tasks/task-18/README.md), the
[current ownership status](../status.md#scope-reduction-decision), and the
[P8/P9 revision and execution records](../status.md#tjerp-p8-adoption-and-p9-first-actual-dry-run).
TJERP's `docs/policy/local-adoption.md` owns its current execution procedure.
Historical evidence remains LocalUntrusted with the review gate unmet; this
ownership transfer establishes no new native run or promotion.
