# agy Gemini 3.8 Flash read-only review

Candidate: `5841d8baece42a251d80f73467b77af403e957f1`.
Requested model: `gemini-3.8-flash-high`, via `agy` in plan/sandbox mode.
The user explicitly authorized this private-code review through agy with Gemini 3.8 Flash and excluded Claude.

Packet SHA-256: `d28bd21185493ac0dfac9461b13515f465a777a2ad027df0f005512d5194b000`.
Conversation: `e91a3a5c-6279-40f8-9e06-48396a7e2796`.
CLI status: `SUCCESS`; duration: 134.273157 seconds.
The completed run returned a nonempty response with no denied_actions field and empty stderr. Earlier attempts are not counted as completed reviews.

Scope: remediation diff from 235fe2c to 5841d8b and selected current source/docs from the frozen candidate. The packet was supplied inline; no reviewer tools, repository edits or tests were requested. This is a packet-scoped engineering review, not authenticated policy promotion evidence. The implementation commit is unchanged.

## Reviewer response (verbatim)

# Engineering Code Review: Candidate `5841d8baece42a251d80f73467b77af403e957f1`

## Review Summary & Status

- **Candidate SHA**: `5841d8baece42a251d80f73467b77af403e957f1`
- **Review Mode**: Read-Only Engineering Review (Packet-Constrained)
- **Substantive Correctness / Security Findings**: **None Found**
- **Approval Provenance**: **Unauthenticated / Inconclusive** (This review does not confer authenticated policy approval, promote the candidate, or satisfy provider-independent approval criteria).

---

## Detailed Evaluation by Component

### 1. Bootstrap 1.0.2 & Fullstack Shared Dependency Audit
- **Files**:
  - `acceptance/src/execution.rs`
  - `acceptance/src/staging.rs`
  - `acceptance/tests/staging.rs`
  - `bootstrap/src/templates.rs`
  - `bootstrap/tests/installer.rs`
  - `bootstrap/templates/1.0.2/profiles/fullstack/profile.json`
  - `crates/project-verification/src/profile.rs`
- **Analysis**:
  - The addition of release `1.0.2` properly updates `templates::profile()`, `release_asset!`, and `project_verification::ProjectProfile::validate()` to recognize `"1.0.2"`.
  - In `fullstack/profile.json`, the `shared:dependencies` sensor is registered with exact pins, version probes (`npm 10.9.3`), bounded inputs (`package.json`, `package-lock.json`, `.node-version`, `.npmrc`), and `npm audit --audit-level=low`.
  - `acceptance/tests/staging.rs` confirms that a failure or omission of `shared:dependencies` prevents fullstack acceptance with `PK-SENSOR-FAILED` and `PK-SENSOR-MISSING`.
  - `bootstrap/tests/installer.rs` verifies that existing 1.0.0 and 1.0.1 receipts upgrade cleanly without conflicts, freeze existing sensor pins, downgrade back to 1.0.1 with byte-identical manifest restoration, and cleanly uninstall.
- **Finding**: No correctness or security regressions identified.

---

### 2. Contract & Schema Synchronization (`SensorResultBinding`)
- **Files**:
  - `crates/policy-contracts/src/policy.rs`
  - `schemas/sensor-profile.schema.json`
  - `xtask/tests/schemas.rs`
- **Analysis**:
  - `crates/policy-contracts/src/policy.rs` aligns Rust deserialization validation (`SensorProfile::validate_version`) with JSON Schema draft 2020-12 generation.
  - Non-blank string schema (`binding_nonblank_string_schema`) precisely mirrors Rust's `str::trim` Unicode whitespace character set (`[^\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]`) and rejects NUL bytes (`\u0000`).
  - Command binding schema (`binding_command_schema`) correctly enforces `minItems: 1`, requires the executable name to be non-blank and NUL-free via `prefixItems`, and constrains subsequent arguments to disallow NUL bytes while permitting valid empty or whitespace arguments.
  - Differential testing in `xtask/tests/schemas.rs` exhaustively exercises empty strings, Unicode whitespace, control characters, NUL bytes, BOM, and zero-width spaces across both serde deserialization and the compiled JSON Schema validator. Schema generation remains deterministic.
- **Finding**: No contract divergence, schema mismatch, or validation bypass found.

---

### 3. GitHub CLI Process Management & Descendant Cleanup
- **Files**:
  - `crates/github-adapter/Cargo.toml`
  - `crates/github-adapter/src/gh.rs`
  - `crates/github-adapter/src/lib.rs`
  - `crates/github-adapter/src/process.rs`
  - `crates/github-adapter/tests/github.rs`
  - `crates/github-adapter/tests/support/process.rs`
- **Analysis**:
  - Process group management is created cleanly via `.process_group(0)`.
  - `OwnedGroup` retains the direct child leader until final cleanup. By using `waitid` with `WaitIdOptions::NOWAIT | WaitIdOptions::NOHANG | WaitIdOptions::EXITED`, the process remains a zombie until explicitly reaped in `finish()` or `drop()`, preventing kernel PID recycling while signaling the process group (`kill_process_group(self.pid, Signal::KILL)`).
  - Darwin (macOS) kernel behavior regarding `EPERM` when signaling a group containing only an unreaped zombie leader is explicitly caught and verified against `self.exited()`.
  - Non-blocking I/O (`fcntl_setfl` with `OFLAGS::NONBLOCK`) across `stdout`, `stderr`, and `stdin` prevents unbounded stalls or pipe deadlocks.
  - Memory consumption is guarded against denial of service via strict bounded chunks (`read_chunk`), evaluating `count > limit.saturating_sub(bytes.len())` before allocating buffer storage.
  - `Drop` guarantees process group termination (`Signal::KILL`) and process reaping if an unexpected error, timeout, or panic unwinds execution.
  - Target platforms outside Linux/macOS fail closed with an explicit `ErrorKind::Unsupported` error before process creation.
- **Finding**: No resource leaks, process group leakage, or zombie races identified.

---

### 4. CLI Regression Test Stabilization
- **Files**:
  - `crates/policy-cli/tests/cli_regressions.rs`
- **Analysis**:
  - In `local_post_merge_evaluation_is_not_exposed`, unknown commands reject execution before reading stdin. Passing `None` instead of `Some("{}")` removes the race condition between child exit and parent writing to a closed pipe (`BrokenPipe`), resolving flaky failures under CI without altering rejection assertions (exit code 64, empty stdout).
- **Finding**: Correct and verified.

---

## Scope Limitations

1. **Packet-Bound Static Analysis**: All findings are based strictly on the provided git diffs, regression test definitions, and architectural documentation. No live terminal execution, network communication, or file reading was performed.
2. **Platform Specificity**: The process cleanup mechanism in `crates/github-adapter/src/process.rs` is compiled strictly for Linux and macOS targets (`cfg(any(target_os = "linux", target_os = "macos"))`). Windows and other non-POSIX environments fail closed by design.
3. **No Authority or Provenance Claim**: As detailed in `docs/trust-model.md` and `docs/promotion.md`:
   - This review does not establish trusted contributor provenance or substitute for independent external provider reviews.
   - Remote branch protection constraints (HTTP 403 on private repository ruleset reads) and live shadow sampling remain outside local code remediation scope.


