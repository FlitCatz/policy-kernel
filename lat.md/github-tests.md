# GitHub process fixtures

Adapter process tests share a fully compiled executable inode while keeping each fixture's mode and descendant marker private to its pathname.

## Executable publication

Hard links avoid opening executable files for writing during parallel test setup. A copy writer inherited by a concurrent fork can otherwise survive the parent's close until the child executes, causing Linux ETXTBSY.

[[crates/github-adapter/tests/github.rs#process_fixture]] waits for the compiler
through `OnceLock` before publishing links in the same temporary filesystem.
[[crates/github-adapter/tests/github.rs#process_fixtures_share_the_closed_executable_inode]]
checks the shared inode and independent modes. No executable is rewritten when
switching modes.

## Spawn errors and deadlines

A writable executable fails at spawn before timeout handling. A Linux counterfactual holds a copy writer open, checks both route and inherited-pipe modes, then closes it to restore execution.

[[crates/github-adapter/tests/github.rs#executable_writer_explains_route_and_timeout_spawn_errors]]
holds the writer open in an isolated test subprocess to widen the race condition
deterministically without leaking a writer into parallel tests; linked
fixtures retain their expected output and Timeout while that writer is open.
This reproduces the failure condition, not the historical scheduler interleaving.
[[crates/github-adapter/tests/github.rs#inherited_output_pipe_does_not_extend_gh_timeout]]
preserves its deadline and descendant-cleanup assertions and reports the actual
result if the expected Timeout is absent. Production error semantics are unchanged.
