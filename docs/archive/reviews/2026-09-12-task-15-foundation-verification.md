# Task 15 foundation — local verification record

## Scope and candidate

Recorded on 2026-09-12 for the first implementation batch, 15.01–15.03, on
`feat/task-15-compatibility`. The candidate is the uncommitted worktree based on
`abd6cb0a1fcd1440cc96a05bfe4ce00280742511`, not that base commit alone.
These observations are local, not an authenticated CI decision, independent-provider
approval, published binary release, protected merge, or production promotion.

Host: native `aarch64-apple-darwin`. Exact compiler observations:

| Compiler | Commit | Build and workspace test result |
| --- | --- | --- |
| Rust 1.85.0 | `4d91de4e48198da2e33413efdcd9cd2cc0c46688` | Build passed; 290 tests passed, 1 live test ignored |
| Rust 1.95.0 | `59807616e1fa2540724bfbac14d7976d7e4a3860` | Build passed; 290 tests passed, 1 live test ignored |

The lockfile SHA-256 for these checks is
`6ce749b4ba8b30b4ea369d360b98c19bde6036ffe1e981e998c3bf6e11005b53`.
No selected third-party package version changed. The two added lock edges connect
the already-locked `tempfile` to CLI/collector dev tests. This record does not
authenticate later source changes that retain the same lockfile.

## 15.01 — current profile entry points

README examples now select embedded bootstrap 1.0.2, or direct users to the
installed `.policy/project-profile.json`. The root fullstack 1.0.0 profile
retains 14 sensors; bootstrap 1.0.2 has 15 required sensors, including
`shared:dependencies`. Both validated successfully with the current CLI.
Invalid profile input returned exit 64.

Root 1.0.0 assets are not byte-identical to embedded bootstrap 1.0.0: the latter
already adds the parser/jsdom bindings. Current profile/bootstrap tables now
distinguish these locations. Existing profile JSON and all embedded
1.0.0–1.0.2 assets remain unchanged, as verified by Git diff.

Task 03's missing required-knowledge policy, Task 10's external observation
controller, and Task 14's unproven authoritative consumer path remain visible in
the current status pages.

## 15.02 — framework references and execution independence

The source/configuration inventory distinguished these classes:

| Reference class | Observation and treatment |
| --- | --- |
| Cargo dependencies and executable product source | No required `superpowers`, `agent-skills`, or `gsd-core` library/call found in the inspected product paths |
| Build/install hooks, generated executable configuration, action and CI paths | No mandatory framework invocation found; residue coverage expanded |
| User-global skill/configuration or planning state | Not a prerequisite for the exercised CLI paths; no global installation was removed |
| Current reviewer integration prose | Rewritten as a generic executable/structured-evidence contract; historical `agent-skills.md` link retained |
| Dated archive plans/specifications and links to them | Preserved as historical records; archive entry point clarifies old mandatory-sounding process instructions are not current prerequisites |
| Deny-list and negative fixtures | Retained intentionally; added names assert rejection or absence, not runtime use |

Four new process tests copy built binaries into temporary work areas and clear
the child environment. They select empty PATH/config/home directories, verify
that Rust and the three framework command names cannot be resolved, and leave
agent-state directories absent. Rust fixtures are compiled before invoking the
isolated product process.

| Exercised binary path | Checked behavior |
| --- | --- |
| `policy-cli evaluate-pre` and `inspect` | Advisory decision and valid inspection pass; tampered evidence is rejected |
| `evidence-collector` | A real Rust fixture runs and emits a bound artifact/version observation; trust remains `local_untrusted` |
| `project-verification validate-profile`, `prepare`, and `run` | One actual fixture test passes; trust remains local; an existing report cannot be replayed |
| `policy-bootstrap plan` and `apply` | Fullstack 1.0.2 installation and receipt-bound uninstall succeed without native project tools or an agent framework |

This is logical process-environment independence, not a filesystem or network
sandbox. It does not prove isolation from malicious hard-coded host paths.
Caller-selected executables, native toolchains, credentials, and report inputs
remain the caller's responsibility when an operation needs them.

The residue regression guard now includes package manifests/lockfile, bootstrap
and acceptance sources, executable templates, CI/configuration paths, and Cargo
metadata's declared custom build/lib/bin targets and workspace members. Root-level
targets also scan sibling Rust modules. Its own deny-list, ordinary negative-test
trees, and historical prose are not treated as framework dependencies.
This is a bounded lexical guard, not a complete Rust import analyzer or a detector
for arbitrary obfuscation/dynamically assembled commands.

New regressions failed before the corresponding scanner changes and passed
afterward, including custom build paths and root-level sibling modules. The
focused verification suite contains 15 passing tests.

## 15.03 — compiler compatibility and reproducibility

All ten packages inherit workspace `rust-version = "1.85"`. Native locked
dependencies including `jsonschema`, `indexmap`, and `hashbrown` provide the
1.85 floor used for the tested selection; target-specific `wasip2` does not
participate in the supported native graph.

Manifest exact pins became compatible requirements. Wire/CLI/schema producers
`serde`, `serde_json`, `clap`, and `schemars` retain their reviewed minor
lines via `~`; the other dependencies use Cargo-compatible ranges. Deliberate
new resolutions still require lockfile review and contract/regression verification.
No arbitrary newer dependency or compiler is certified by a range match.

The development/release default remains Rust 1.95.0. That default is not a
consumer runtime prerequisite for the four exercised product paths. Those built
binaries ran without Rust tools on PATH; repository verification utilities such
as `xtask check-residue` still invoke Cargo and require its Rust tooling.
No new downloadable binary distribution was introduced.

The first attempted 1.85 run exposed a local environment issue: host Cargo and
rustc were direct symlinks to 1.95, while rustdoc obeyed rustup's 1.85 selection.
That failed mixed-compiler run was not counted as compatibility evidence.
After selecting Cargo/RUSTC/RUSTDOC explicitly, six fixture-compilation call sites
were also corrected to honor `RUSTC`. The successful results above use the
selected compiler consistently, including those Rust fixtures.

The release workflow now configures a two-compiler, two-OS compatibility matrix
and records actual compiler/Cargo/head observations. These hosted jobs have not
been executed for this candidate; local macOS observations do not stand in for
Linux CI or an exact-head required check.

## Verification and remaining work

The following passed locally with the selected locked dependency graph:

- Workspace build and tests on Rust 1.85.0 and 1.95.0.
- Default-toolchain format check and strict all-target Clippy.
- Deterministic schema regeneration, with no checked-in schema changes.
- Public API/schema residue check and expanded runtime residue check.
- Markdown relative-link checks and `git diff --check`.

The ignored live test requires explicitly enabled same-repository GitHub Actions
execution and `checks:write`. No live permission or promotion was inferred.
Native Node/npm/.NET consumer acceptance, advisory database refresh, full mutation
campaign, and new controller/provider/live evidence were not rerun or completed
as part of this first batch.

Continue with 15.04's shared tool/version/path selection contract. Work packages
15.04–15.09 remain unimplemented, 15.10 remains partial, 15.11 requires external
execution, and 15.12's final handoff remains pending. Follow the
[Task 15 landing page](../../tasks/task-15/README.md) and
[current status](../../status.md) for later updates without rewriting this record.
