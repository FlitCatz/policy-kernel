# P2 local generated-output verification

Date: 2026-10-16. Scope: the uncommitted P2 candidate based on
`78b579fac05ce4c819ec0e4a469196ac10b3d6f9`. Result: **local scope satisfied**
for the synthetic native fixture. This is LocalUntrusted, not TJERP adoption,
an authoritative CI decision, protected-branch merge or production promotion.

The Rust harness `acceptance/tests/native_generated_files.rs` used the installed
SDK `11.0.100-rc.1.26425.128`, cached Mapperly 4.3.1 and the fixture's pinned
VSTest 17.14.1/xUnit packages. Every resolved package was checked in the existing
cache before execution. NuGet package and audit feeds were cleared. The locked
offline setup restore disabled audit; no package/tool installation or network
restore was performed, and no audit success is claimed.

The staged Modules project passed the P2 XML and filesystem validators. Actual
Mapperly-generated C# was built, read and inspected by
`ArchitectureTests.Reads_actual_mapperly_generated_source`. Its TRX records one
executed/passed test with no skips. The test also calls the generated mapper.
Replacing `obj/generated` with an empty directory and rerunning the same compiled
test without rebuilding failed specifically at `Assert.NotEmpty`.

The retained run used:

```console
POLICY_GENERATED_OUTPUT=/Users/dev-r/.paseo/worktrees/0crv3fh3/pk-msbuild-generated-output/target/p2-native-proof POLICY_GENERATED_DOTNET=/Users/dev-r/.local/share/mise/installs/dotnet/11.0.100-rc.1.26425.128/dotnet POLICY_GENERATED_PACKAGES=/Users/dev-r/.nuget/packages cargo test --locked --offline -p policy-acceptance --test native_generated_files -- --ignored --nocapture
```

The harness passed in 4.12 seconds. The output directory is local, git-ignored and
not a durable remote artifact. It contains `backend/native-{0,1,2,3}.*` command
records and `backend/.policy-reports/generated.trx`. The TRX SHA-256 is
`9e2b104ec621b7ae80015b3772be79bb4ead8c25caf19fb6f047869efe60f54a`.
The native process/socket sandbox attempt stalled and was stopped; the retained
run used explicitly approved native process access with the same disabled feeds.

Eight Rust contract tests cover project opt-in, exact values, path/expansion and
case-duplicate rejection, conditional declarations, inherited/CLI/environment
overrides, imports/targets, audit regressions, actual output symlinks and response
files. A regression also rejects XML comments hiding an unsafe value suffix.
The first suffix regression failed before its fix and passed afterwards.

The existing historical Task 16/17 records remain unchanged. Hosted checks and
review of the resulting PR are separate from this local execution record.
