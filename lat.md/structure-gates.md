# Structural verification gates

Engineering checks pin existing argument limits and workspace dependency directions. They preserve kernel verdicts and do not authorize promotion; see the [verification contract](../docs/integrations/structure-gates.md).

## Argument limits

Workspace-inherited Clippy deny rules enforce at most five arguments and two bool parameters, measured on the baseline before implementation. Struct bool fields remain outside the gate.

The existing release verification Clippy command reads root `clippy.toml`.
The full measurement and applicability boundaries are in the
[verification contract](../docs/integrations/structure-gates.md#clippy-limits).

## Dependency directions

The residue command requires path dependencies to target workspace members and checks internal directions across all declaration kinds. It reports dependency violations together in sorted order and preserves policy-core external exclusions.

Excluded local crates and absolute external paths cannot bridge the allow-list.
Renamed, optional, dev, build and inactive-target declarations all count;
repeated violations across declaration kinds appear once.

Rules [[xtask/src/verification.rs#WORKSPACE_DEPENDENCIES]] and
[[xtask/src/verification.rs#check_dependencies]] reuse
[[xtask/src/verification.rs#cargo_metadata]] with
[[xtask/src/verification.rs#scan_cargo_targets]].
The CLI entry point is [[xtask/src/verification.rs#residue]].

## Regression coverage

Tests accept the current graph, reject new/reversed edges and non-member paths, preserve external exclusions, and verify complete sorted diagnostics using real Cargo metadata and lockfiles.

The inline test module in [verification.rs](../xtask/src/verification.rs) covers
the current graph, new and reversed edges, and forbidden external dependencies.
CLI declarations [[xtask/tests/verification.rs#workspace_edges_include_renamed_optional_dev_build_and_inactive_target_dependencies]].
Non-member paths [[xtask/tests/verification.rs#excluded_path_dependencies_cannot_bridge_workspace_directions]]
and [[xtask/tests/verification.rs#absolute_path_dependency_outside_repository_is_rejected]].
Complete CLI diagnostics [[xtask/tests/verification.rs#all_dependency_violations_are_reported_in_stable_order]].
