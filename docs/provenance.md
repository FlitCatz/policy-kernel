# Provenance

## Frozen legacy source

- Repository: `FlitCatz/agent-dev-kit`
- Commit: `e5f6ec8584cc91fdeca3aab40687c6cdcd303b43`
- Specification: `docs/specs/2026-09-04-policy-kernel-extraction-design.md`
- Extraction mode: contract-first selective reimplementation
- Copied production code: none
- Distribution license: intentionally unset pending owner selection

The legacy repository is evidence, not a source-tree template. The Rust files below
are behavioral sources only. Their filesystem access, subprocess execution,
configuration fallback, ambient clocks, provider bindings, and GSD lifecycle state
are explicitly rejected as implementation coupling.

| Legacy path | Git blob SHA | SHA-256 content digest | Classification |
|---|---|---|---|
| `tools/src/consensus.rs` | `9e253adc9bdd5028db7f03baf1428f9b14a27d0d` | `e53babb3131369e7952bf03d3ee018914e689c00838fd4d271d9c9d06335d361` | Behavioral source; do not copy |
| `tools/src/sensor_resolver.rs` | `aa16445b13001289b5c4753759c5bb8fb55283c8` | `a5e0b718bfe5eea294826e65ddc7ab1cd944ea88c0083450cd1e0de3b2fdf2f4` | Behavioral source; do not copy |
| `tools/src/git_provider.rs` | `3a5837521b966240ef778db29bc45072c5cab720` | `c96936675075dbe3cfa81fcb390b8759d2a06af4f0a890158ec7090779a1d93f` | Behavioral source; do not copy |
| `tools/src/lifecycle.rs` | `ba1c92358f08fc66265a935330d632570c92e17b` | `8aa6160467daaef1c014ccdade223472af8466a13079526e72c40ba9fe5e3a3b` | Behavioral source; do not copy |
| `tools/tests/integration_tests.rs` | `bb40dbdacdba4b09f5fcc92df2f360ae0b459499` | `04f1dffd4e8b45f0d2fd92a6d73c262687f62266c22b7841798c55f31202f1de` | Adversarial behavior source |
| `AGENTS_GUIDE.md` | `ac3282911ebf292e3c2d318222245d09c4e93560` | `cd70fc7b78b70f4738571f57ceccc0887daf9f2553dea8b2d26ea6822edb89f8` | Normative documentation source |
| `docs/ARCHITECTURE.md` | `babaf7ff47c0475da735170cdc6ea69d72382685` | `645623f55f650dae6e9beb55c1f7b2d09843efef7898ad9ff56d61acfb3d9c92` | Architecture documentation source |

## Rust dependency pins

The workspace declares only the approved technology stack. Exact versions are pinned
for deterministic resolution as the crates begin using them.

| Package | Pin |
|---|---:|
| serde | 1.0.228 |
| serde_json | 1.0.145 |
| serde_yaml | 0.9.34 |
| chrono | 0.4.42 |
| sha2 | 0.10.9 |
| hex | 0.4.3 |
| thiserror | 2.0.16 |
| clap | 4.5.47 |
| schemars | 1.0.4 |
| globset | 0.4.16 |
| assert_cmd | 2.0.17 |
| test-case | 3.3.1 |
| proptest | 1.7.0 |

## Verification status

GitHub Actions run `33868276500` validated commit
`ae56817e05376e352ff3a56fcc3cb28ff611b8c1` with Rust 1.95.0. The locked workspace
check, frozen source-inventory test, and exact seven-member workspace metadata check
all completed successfully. Dependency pins not yet referenced by a crate will enter
the lockfile when their corresponding implementation task introduces them.
