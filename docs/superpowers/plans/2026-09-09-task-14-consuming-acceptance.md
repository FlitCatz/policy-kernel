# Task 14 consuming-project acceptance implementation plan

**Goal:** Exercise installed profiles on real React, .NET and fullstack consumers and retain honest positive/negative execution records.

**Architecture:** Small native consumer fixtures feed the existing Rust bootstrap and project-verification libraries. A separate Rust acceptance executable stages isolated scenario directories, binds source inventories to their Git heads, executes the installed profiles and records observations. It never upgrades local evidence to CI trust. Existing kernel regression paths verify authority and stale-head rejection separately.

**Tech stack:** Rust orchestration; pinned Node 22.18.0/npm 10.9.3 and .NET SDK 9.0.304; TS/TSX/C# only for example apps/tests (user exception approved 2026-09-09).

**Spec:** `docs/plans/2026-09-04-policy-kernel-implementation.md`, Task 14; existing Task 12/13 integration contracts.

## Constraints

- No Python or shell implementation. CI YAML only provisions tools and calls Rust.
- Keep all existing release/native CI gates. No policy adoption, remote write, or fabricated authoritative approval.
- Preserve bootstrap 1.0.0 inventory; consumer-specific reviewed configuration may be added after installation and included in run bindings.
- Record missing tools, restore errors, unexecuted CI and failed baseline distinctly; an unrelated failure does not prove a negative scenario.

## Execution checklist

- [x] Add native example tests first, then minimal React/.NET apps and fullstack API/contract/E2E checks. Pin manifests and real resolved lockfiles where provisioning is available.
- [x] Add Rust acceptance tests for scenario selection, safe isolated staging, explicit failure attribution, unavailable runtime reporting and source/head bindings; run to observe failures before implementation.
- [x] Implement bootstrap installation, dependency provisioning, native profile execution and report retention in a separate Rust crate. Scenarios: baseline, type error, component assertion failure, rejected authorized API request, empty required report, frontend/backend contract mismatch and corrected rerun.
- [x] Exercise stale previous-head rejection through real project normalization and existing authoritative kernel checks; keep simulation results separate from authenticated CI observations.
- [x] Add a pinned CI profile matrix invoking Rust, uploading results even on failure, without changing existing checks.
- [x] Run focused Rust suites and relevant predecessor suites, then workspace checks. Record actual local observations and CI limitations in `docs/integrations/acceptance.md` and a Task 14 review record.

Implementation stays in the current checkout on a Task 14 feature branch; no remote publication is required.

## Acceptance remaining (2026-09-10)

The implementation checklist is complete, not the Task 14 promotion gate.
See [the execution record](../../reviews/2026-09-09-task-14-consuming-acceptance.md).

- [x] Resolve the frozen profile dependency audit failures through a reviewed
  release/pin update; 1.0.1 native React/.NET/fullstack runs all passed.
- [ ] Execute the three-profile matrix in real CI and retain its artifacts.
- [ ] Obtain authenticated evaluated-head evidence and independent approvals.

All three local baselines and all five negative scenarios passed with 1.0.1.
Corrections restored their targeted checks and old-head replay was rejected.
The original 1.0.0 inventory remains unchanged; 1.0.1 is an explicit security
release. Remote CI and authenticated independent approvals remain outstanding.
