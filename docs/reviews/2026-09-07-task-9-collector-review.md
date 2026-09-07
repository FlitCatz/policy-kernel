# Task 9 collector verification

An independent read-only review checked process cleanup, timing, trust/digest
bindings, artifact containment and conformance to task 9. It found that resolving
an executable symlink changed the child's actual `argv[0]` relative to the recorded
argv. Added `actual_argv_zero_matches_record_for_symlink_dispatch`, observed the
failure, then set `CommandExt::arg0` for both main and version executions. The
regression passed and the reviewer reported no remaining important findings.

Additional boundary review replaced artifact canonicalize/open with
descriptor-relative `openat` and `NOFOLLOW` for every component. A regression
first demonstrated acceptance of symlink components, then verified rejection
while nested regular artifacts still pass.

macOS cleanup requires handling an exited, unreaped group leader: XNU's group
signal path can return EPERM when no live member receives the signal. Cleanup
checks the leader's waitable exit state for that Darwin-specific case. A running
leader's permission errors still fail, and timeout/truncation retain their explicit
status alongside any cleanup diagnostic. The runner must prohibit credential
changes/session escape; process groups are not a sandbox. Reference:
[Apple XNU signal implementation](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/kern/kern_sig.c).

Final checks on macOS:

- `cargo test --workspace --locked --offline`: 109 tests passed (19 collector).
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: passed.
- `cargo fmt --all -- --check`: passed.
- `git diff --check`: passed.

Linux execution is implemented through the same rustix/POSIX path but was not
runtime-tested in this macOS workspace. Other platforms fail before execution.
Task 8 schema regeneration is an integration step after the shared enum change
is merged; this branch does not change task 8's generated files or worktree.
