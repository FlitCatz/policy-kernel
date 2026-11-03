# Structural verification gates

Engineering checks pin existing argument limits and workspace dependency directions. They preserve kernel verdicts and do not authorize promotion; see the [verification contract](../docs/integrations/structure-gates.md).

## Argument limits

Workspace-inherited Clippy deny rules enforce at most five arguments and two bool parameters, measured on the baseline before implementation. Struct bool fields remain outside the gate.

The existing release verification Clippy command reads root `clippy.toml`.
The full measurement and applicability boundaries are in the
[verification contract](../docs/integrations/structure-gates.md#clippy-limits).

## Dependency directions

The residue command checks every declared internal edge against the reviewed direction allow-list, including renamed, optional, dev, build and inactive-target declarations. Existing policy-core external exclusions remain enforced.

Rules [[xtask/src/verification.rs#WORKSPACE_DEPENDENCIES]] and
[[xtask/src/verification.rs#check_dependencies]] reuse
[[xtask/src/verification.rs#cargo_metadata]] with
[[xtask/src/verification.rs#scan_cargo_targets]].
The CLI entry point is [[xtask/src/verification.rs#residue]].

## Regression coverage

Tests accept the current graph, reject new and reversed edges with named diagnostics, preserve forbidden external dependencies, and exercise renamed and inactive declarations through real Cargo metadata.

The inline test module in [verification.rs](../xtask/src/verification.rs) covers
the current graph, new and reversed edges, and forbidden external dependencies.
CLI declarations [[xtask/tests/verification.rs#workspace_edges_include_renamed_optional_dev_build_and_inactive_target_dependencies]].
