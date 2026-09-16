# Rust and dependency compatibility

The workspace separates its supported lower bound from its reproducible default:

| Selection | Meaning | Current evidence boundary |
| --- | --- | --- |
| Rust 1.85.0 | `workspace.package.rust-version`; inherited by all ten packages and the supported minimum for native Linux/macOS builds | Full local macOS ARM64 suite and hosted Ubuntu x86_64/macOS ARM64 suites on Task 15 candidate `751fa76` |
| Rust 1.95.0 | `rust-toolchain.toml` development and release-verification default, including `rustfmt` and `clippy` | Full local and hosted suites on the same platforms/candidate; remains the default for release checks |

These observations do not certify every intermediate or newer Rust release. The
Ubuntu/macOS CI matrix selects 1.85.0 and 1.95.0 explicitly. Each hosted matrix
job on candidate `751fa76` passed 340 tests, with three ignored tests counted
separately. The [hosted record](../archive/reviews/2026-09-16-task-15-hosted-verification.md)
binds the actual versions and results to their run, head, App and artifact hashes.
See the [Task 15 landing page](../tasks/task-15/README.md) for current completion state.
The native minimum-version run recorded `rustc 1.85.0` commit
`4d91de4e4` on `aarch64-apple-darwin`.

The supported native paths are Linux and macOS. In the locked native graph,
`jsonschema`, `indexmap`, and `hashbrown` declare Rust 1.85 or lower, and the
workspace source also uses `Option::is_none_or`, available since Rust 1.82.
The lockfile also contains target-specific `wasip2`,
whose Rust 1.87 requirement is outside the supported native target selection; it
is not compiled for the supported Linux/macOS paths. This does not claim support
for a WASI build with Rust 1.85.

## Selecting a compiler reliably

Install the selected rustup-managed toolchain and bind Cargo plus compiler-spawned
fixture tests to the same selection:

```console
rustup toolchain install 1.85.0 --profile minimal
RUSTC="$(rustup which --toolchain 1.85.0 rustc)" RUSTDOC="$(rustup which --toolchain 1.85.0 rustdoc)" rustup run 1.85.0 cargo test --workspace --locked
```

Use `1.95.0` in all three positions to reproduce the default compiler selection.
Do not assume `RUSTUP_TOOLCHAIN=1.85.0 cargo ...` is sufficient on every host:
`cargo` or `rustc` may be a direct symlink to another toolchain rather than a
rustup proxy, and compile-fixture tests launch `RUSTC` at runtime. All six such
tests in this workspace honor the explicit `RUSTC` selection; `RUSTDOC` also
keeps documentation subprocesses on the selected toolchain.

The `rtk` prefix seen in some contributor commands is an optional output wrapper,
not a Cargo dependency, build hook, or product runtime requirement. It is omitted
from the commands above.

## Dependency resolution

Third-party manifest dependencies are compatibility constraints, not exact `=`
pins. Wire, CLI, and schema-facing dependencies `serde`, `serde_json`, `clap`, and
`schemars` use Cargo `~` requirements to remain within their reviewed minor line.
Other workspace dependencies use Cargo's compatible default requirements; for
0.x versions, Cargo's compatibility range is narrower than a new major line.

`Cargo.lock` remains the reproducibility boundary for builds and tests. Removing
the manifest `=` pins did not change any selected third-party version in the
lockfile. The foundation added existing `tempfile` dev-dependency edges for
`policy-cli` and `evidence-collector`; the follow-up added `policy-ci` edges to
the already-locked `rustix` and test-only `tempfile`. These are all four edge
additions relative to `abd6cb0`; no third-party selected version changed.
Run normal verification with
`--locked`; test and review any deliberate lockfile update rather than treating a
range match as compatibility evidence.

## Runtime independence and distribution

Four process-environment-isolated binary tests exercise supported CLI paths with
Rust tools and agent-framework tools absent from `PATH` and without user-global
agent configuration. This demonstrates logical environment/configuration
independence only. It is not an operating-system, process, or filesystem sandbox,
and a caller-selected external executable, native project tool, credential, or
report input must still be provisioned for the operation that requests it.
This no-Rust-runtime observation covers `policy-cli`, `evidence-collector`,
`project-verification`, and `policy-bootstrap` paths, not every workspace
verification command. For example, an already-built `xtask check-residue`
still invokes `cargo metadata` and `cargo tree` and requires their Rust tooling.
No agent orchestration framework is a Cargo dependency, build/install hook, or
required CI step in the supported product paths.

Task 15 does not create a new downloadable release or prebuilt binary
distribution. Consumers still build the locked Rust workspace or provide an
already-built trusted binary through their controlled release process. Later
Task 15 work remains separate from this compatibility baseline.
