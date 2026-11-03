# Jest execution maintenance boundary

As of the 2026-11-03 two-PR transition, managed Jest producer combinations,
upgrades and native CI guarantees have ended. This release note describes a
support-scope change with unchanged profile/native/root schema versions and
workspace package version 0.1.0. It creates no binary release or tag.

The maintained alternate fixture is now `examples/react-vitest`: exact Node
22.19.0/npm 10.9.4, a `web/` React SSR root, Vitest 4.1.12/Vite 6.4.5, real
failure/correction and skip, empty/missing reports, previous-head rejection and
nested dev-only `.npmrc omit=dev` audit failure/correction. See
[native commands](acceptance.md#alternate-vitest-transition) and the
[lock/audit record](native-fixture-audit.md#alternate-vitest-replacement).

The workflow retains `alternate-vitest` with display name
`React SSR / Vitest / alternate Node and npm`. The former `alternate` Jest job
is removed. `Project-owned frontend versions` keeps its job identity and all
npm10/npm12 Vitest/Playwright, audit severity, report/head and trust checks;
only its final Jest fixture phase is removed. No audit exception or deepmerge
override is introduced to make the former failing Jest checks pass.
GitHub reported main unprotected (protection endpoint 404), no repository
rulesets and no tracked controller reference to the alternate display name
when the transition was prepared. This is an observation, not a protection edit.

## Compatibility retained

The Rust `jest_json` report reader and `jest-json/v1` parser version remain,
including strict retry, interruption, snapshot, open-handle and failure checks.
`ReportFormat::JestJson`, `Capability::JestJson`, `AdapterContract::JestJsonV1`,
schema-2/3/4 decoding/migration and the native importer `jest_json` path remain.
Original report bytes and an inert legacy profile corpus test that compatibility;
they do not install or execute Jest. See [report semantics](report-formats.md#jest).

Generic profile argv continues to execute existing Jest commands when supplied
by a consumer. This change does not add a runtime rejection or new exit code.
It ends maintenance guarantees for those producer combinations. Current
[CLI exit semantics](project-verification.md) and version bounds remain intact.

Frozen bootstrap 1.1.0/1.1.1 bytes, earlier assets, receipt inventories,
reconstruction, recovery and uninstall remain unchanged. Built-in React/fullstack
presets already use Vitest. No new preset, schema bump, core verdict change,
audit severity change or threshold is part of this transition.

## Moving a consumer to Vitest

Run a real Vitest producer and review its argv, config, report format/path,
version probes, capabilities/contracts, installed package/lock bindings and
source inventory. Re-adopt the profile/config/source digests and map real report
test IDs to the consumer's requirements. `migrate-profile --target-schema 4`
changes contract structure; it does not convert runners. Renaming a Jest report's
format to `vitest_json` does not supply fresh Vitest evidence.

Vitest JSON carries no retry history. The maintained fixture sets `retry: 0`,
which is not equivalent to Jest's success-after-retry rejection and does not
certify arbitrary user-owned test settings. Strict parity would require a
separate producer contract/reporting design. Playwright retry rejection remains
covered, with its own report contract.

Historical execution records retain their original Jest observations. The
[current-status authority](../status.md#jest-transition-managed-execution-ended)
records the maintained boundary; local tests and ordinary hosted checks do not
by themselves authorize protected merge, Trusted evidence or production promotion.
