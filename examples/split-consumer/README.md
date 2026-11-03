# Synthetic split-consumer conformance fixture

This fixture mirrors inherited net11.0, API/library/test dependency graphs, a
combined test project with filtered suites, frontend unit tests and browser-to-API
E2E. It contains no ERP code or data. All results are local advisory evidence.

The profile requires `frontend:greeting`. Map it to the normalized ID of the
executed greeting test before evaluating adequacy; suite counts alone cannot
satisfy it. This synthetic requirement is not a product business catalog. See the
[adequacy contract](../../docs/integrations/adequacy.md).

The checked-in `project-profile.json` is the maintained conformance input; it
preserves report, audit and tool-version regression coverage.

The frontend selects Vite 6.4.5 and Vitest 4.1.12; its lock resolves
source-map-js 1.2.2. Node 26.10.0/npm 10.9.3 remain the native runtime and audit
selections. See the [audit record](../../docs/integrations/native-fixture-audit.md#split-consumer-follow-up)
for lock generation and local verification.

Run the opt-in [native harness](../../docs/integrations/consumer-schema3.md#native-verification)
from the workspace root. The harness uses a new staged checkout and preserves
native producer failure artifacts. Ports 4180 and 5077 must be available; it does
not reuse existing servers.
