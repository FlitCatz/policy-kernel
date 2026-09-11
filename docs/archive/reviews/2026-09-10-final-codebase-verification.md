**최종 코드베이스 검증 — 2026-09-10**

검증 대상은 `235fe2c3c549d5f880a1185a0cb239870ba75ca9` (`feat/task-14-consuming-acceptance`)이다. 시작 시 작업 트리는 깨끗했다. 전체 구현을 설계·계획·의무 매트릭스와 대조하고, 세 영역의 독립적인 코드 검토와 실제 실행 검증을 수행했다. 이 문서 외에 저장소의 구현·테스트·설정은 변경하지 않았다.

**판정: 자동검사는 통과했지만, 결함 3건과 미완료 설계·통합 요구사항이 있어 “빠진 부분 없이 완료” 또는 운영 승격 가능으로 판정할 수 없다.** 검토 범위에서 P1 승인 우회는 확인하지 못했다. 이는 우회가 없다는 증명이 아니다. 아래 P2는 수정이 필요한 항목이며, 문서화된 설계 누락은 별도로 구분했다.

**확인된 결함**

1. **P2 — Fullstack 공용 npm 의존성 그래프가 감사에서 빠진다.**

   위치: `bootstrap/templates/1.0.1/profiles/fullstack/profile.json:287`, 같은 파일의 `shared:contract`와 `shared:e2e` 입력(651, 705행).

   유일한 `npm audit --audit-level=low`는 `frontend` 컴포넌트에서 실행한다. 계약/E2E 도구는 별도의 루트 `package.json`·`package-lock.json`을 사용하지만 `shared` 의존성 감사 센서가 없다. 새로 실행한 Fullstack baseline의 설치된 프로필과 14개 센서 결과에서도 이 누락을 확인했다. 따라서 공용 그래프에만 생긴 취약점은 현재 필수 센서가 모두 성공해도 검출되지 않을 수 있다. 이는 현재 공용 그래프에 취약점이 발견되었다는 주장이 아니다.

   근거: [프로필 설계 §2–3](../specs/2026-09-07-project-verification-bootstrap-design.md)의 잠금파일 기반 의존성 검사와 실패 명시 요구. 고정된 1.0.0/1.0.1 자산을 조용히 바꾸지 않고 새 프로필 릴리스에 필수 공용 감사 센서를 추가해야 한다. 공용 그래프만 취약한 경우의 회귀검증도 필요하다.

2. **P2 — 공개 SensorProfile 스키마와 Rust 역직렬화의 허용 범위가 다르다.**

   위치: `crates/policy-contracts/src/policy.rs:148` 및 `schemas/sensor-profile.schema.json:94`; 실제 제한은 `policy.rs:59`에 있다.

   `SensorResultBinding`의 `parser_version`과 `resolved_command`는 스키마에서 일반 문자열/배열로 생성된다. 반면 Rust는 빈/공백 parser, 빈 argv, 공백 실행 파일명, NUL을 거부한다. v2 입력의 `resolved_command`를 `[]` 또는 `[""]`로 주었을 때 저장소의 JSON Schema validator는 성공하고 `SensorProfile` 역직렬화는 실패하는 것을 Rust 재현기로 확인했다.

   ```text
   command=[] schema_accepts=true rust_accepts=false
   command=[""] schema_accepts=true rust_accepts=false
   ```

   스키마를 이용하는 소비자는 유효 판정을 받은 설정을 실제 Rust 도구에 넘겼다가 거부당한다. [Task 8 계획](../superpowers/plans/2026-09-08-task-8-cli-schema.md)의 custom deserializer 제약 일치 요구와 어긋난다. 스키마 생성 원본을 보완하고 동일 입력에 대한 스키마/역직렬화 대조 테스트를 추가해야 한다. 현재 스키마 재생성에 차이가 없다는 검사는 이 의미상 불일치를 발견하지 못한다.

   재현 소스: `/private/tmp/policy-core-review-repro.rs`. 부모 검토자가 재실행하여 같은 결과를 확인했다.

3. **P2 — GitHub API 타임아웃 후 하위 프로세스와 I/O 작업이 남는다.**

   위치: `crates/github-adapter/src/gh.rs:149`, 프로세스 생성 `gh.rs:87`, 직접 자식만 종료하는 `gh.rs:359`.

   `gh` 역할의 프로세스가 stdout/stderr를 상속한 자손을 생성하고 먼저 종료하면, API는 채널 수신 제한시간에 맞춰 `Timeout`을 반환하지만 자손과 파이프를 읽는 스레드를 정리하지 않는다. Rust 재현에서 제한시간 50ms 후 반환했는데도 자손이 300ms 시점에 파일을 생성했다. 반복 호출 시 프로세스·스레드·파일 디스크립터가 누적될 수 있다. 승인 우회가 아니라 실행 수명과 자원 정리 결함이다.

   기존 `inherited_output_pipe_does_not_extend_gh_timeout` 테스트는 반환 시간만 검사한다. 소유한 프로세스 그룹과 I/O 작업을 오류 경로에서도 정리하고, 자손이 실제 종료되는 회귀검증이 필요하다. 같은 저장소의 evidence-collector는 별도로 프로세스 그룹 정리를 구현한다.

   재현 소스: `/private/tmp/policy-review-gh-cleanup.rs`. 부모 검토자가 재실행하여 `Confirmed: descendant executed 300ms after a 50ms API timeout`을 확인했다.

**설계 대비 미완료 항목**

| 항목 | 현재 구현과 남은 조건 |
| --- | --- |
| 필수 knowledge/index 증거 | `PolicySnapshot` (`policy.rs:16`)에 필수 knowledge 선언이 없고 provenance는 제출된 knowledge만 순회한다. CodeGraph를 프로젝트 정책상 필수로 지정·강제할 수 없다. [추출 설계 §10.2](../specs/2026-09-04-policy-kernel-extraction-design.md), 원래 Task 3 요구와의 차이이며 [trust-model](../../trust-model.md)도 이미 인정한 누락이다. 일반 정책에서 knowledge가 비어 있는 것 자체는 오류가 아니다. |
| 수집 → 인증된 CI 판정 연결 | collector와 project-verification은 `LocalUntrusted`를 유지한다. production `policy-ci`는 외부 controller가 만든 `identity.json`과 증거를 소비하며, 이를 발행하는 production controller는 저장소에 없다. [Action 계약](../../../action/README.md)은 이 책임을 외부에 명시적으로 둔다. 단순히 JSON을 신뢰 디렉터리에 놓는 것으로 해결할 수 없다. 실제 소비 프로젝트에서 인증된 수집·독립 리뷰·정책 채택·보호된 merge까지 연결한 READY/SHIPPED 검증이 남는다. |

현재 Task 14는 실제 네이티브 검사와 로컬 증거를 검증한다. CI에서 실행되어도 summary의 `ci_authoritative`는 계속 `unverified`이다. 이 동작은 신뢰 모델에 부합하지만, 원래 Task 14의 실제 authoritative 판정 인수조건 전체를 충족하지는 않는다. 별도의 core simulation 테스트와 실제 GitHub 승인은 구분해야 한다.

**이번에 실행한 검증**

모든 Cargo 실행은 고정된 Rust 1.95.0과 잠금파일을 사용했다. 네트워크가 필요 없는 명령은 `--offline`으로 실행했고, deny/audit는 DB 갱신까지 실행했다.

| 검사 | 결과 |
| --- | --- |
| `cargo fmt --all --check` | 통과 |
| `cargo check --workspace --locked --offline` | 통과 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | 통과 |
| `cargo test --workspace --locked --offline` | 277 통과, 0 실패, 1 ignored; 53 suites. 목록 278개와도 대조했다. |
| `xtask schema`, tracked/untracked schema 차이 확인 | 통과, 차이 없음 |
| `cargo deny check` | advisories/bans/licenses/sources 모두 통과. syn 2/3 중복 경고는 남는다. |
| `cargo audit` | 갱신된 1,243개 권고 DB로 151개 의존성 검사, exit 0 |
| `xtask check-residue` | 통과 |
| `cargo doc --no-deps` 및 `xtask check-public-api target/doc schemas` | 통과 |
| 빈 `/private/tmp/policy-final-clean-235fe2c` target에서 전체 workspace build | 통과 |
| `xtask rollback-drill` | 제공된 fixture snapshot 비교 통과. `live_protection_verified: false` 유지 |
| 현재 커밋의 지정 5개 모듈 mutation 전체 실행 | 284개 완료: 275 caught, 9 unviable, 0 missed, 0 timeout. baseline 및 inventory 완전성 검사 통과 |

`ignored`는 인증된 Actions 실행과 `checks:write`가 필요한 live GitHub 테스트 1개이다. 통과로 계산하지 않았다. 기존 `verification/mutants.out`의 과거 성공 결과는 현재 커밋의 mutation 증거로 재사용하지 않았다.

새 mutation 결과는 `/private/tmp/policy-final-mutation-235fe2c/mutants.out/`에 있다. cargo-mutants 27.1.0으로 아래 명령을 실행했고, 종료 시각이 있는 전체 outcomes와 mutants inventory를 저장소의 판정기로 다시 검증하여 exit 0을 확인했다. 9개 unviable을 검출 성공으로 계산하지 않았다. 이 검사는 Task 11이 지정한 다섯 파일의 범위이며 모든 모듈에 대한 mutation 보증이 아니다.

```console
rtk proxy cargo mutants --no-config -p policy-core --file crates/policy-core/src/canonical.rs --file crates/policy-core/src/provenance.rs --file crates/policy-core/src/reviews.rs --file crates/policy-core/src/pre_merge.rs --file crates/policy-core/src/post_merge.rs --test-workspace false --jobs 2 --timeout 120 --build-timeout 600 -C --locked -C --offline --output /private/tmp/policy-final-mutation-235fe2c
rtk proxy cargo run --locked --offline -p xtask -- check-mutation-results /private/tmp/policy-final-mutation-235fe2c/mutants.out/outcomes.json
```

**현재 커밋의 새 네이티브 인수검증**

빈 target에서 만든 `policy-acceptance`로 실행했다. 세 summary 모두 `harness_head`가 검증 대상 SHA이고 `harness.dirty: false`이다. 이 문서는 관측을 끝낸 뒤 추가했다. 런타임은 Node 22.18.0/npm 10.9.3/.NET SDK 9.0.304이며 프로필 릴리스는 1.0.1이다.

| 프로필 | 결과 | 로컬 증거 디렉터리 |
| --- | --- | --- |
| React | baseline 및 type-error/component-failure/empty-report의 실패·수정·이전 head 거부 모두 통과 | `/private/tmp/policy-final-native-react-235fe2c` |
| .NET | baseline 및 api-authorization의 실패·수정·이전 head 거부 통과 | `/private/tmp/policy-final-native-dotnet-235fe2c` |
| Fullstack | baseline 및 contract-mismatch의 실패·수정·이전 head 거부 통과. API/계약/E2E 세 실패 attribution 모두 true | `/private/tmp/policy-final-native-fullstack-235fe2c` |

호출 형식은 다음과 같고, 나머지 두 프로필도 프로필명·출력 경로만 바꾸어 같은 환경에서 실행했다.

```console
rtk proxy env PATH=/private/tmp/policy-kernel-node-22.18.0/node_modules/.bin:/private/tmp/task14-dotnet-9.0.304:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin DOTNET_ROOT=/private/tmp/task14-dotnet-9.0.304 NUGET_PACKAGES=/private/tmp/policy-task14-nuget NPM_CONFIG_CACHE=/private/tmp/policy-task14-npm-cache /private/tmp/policy-final-clean-235fe2c/debug/policy-acceptance run --profile fullstack --output /private/tmp/policy-final-native-fullstack-235fe2c --provision
```

각 디렉터리에 `summary.json`, 단계별 request/result/observation, 원본 보고서, 이전 head 거부 기록이 있다. 최상위 실행 로그는 해당 디렉터리명에 `.log`를 붙인 경로에 있다. 임시 파일은 영구 보존 저장소가 아니다.

**실제 GitHub 관측**

- [Task 14 native consumer acceptance #1](https://github.com/FlitCatz/policy-kernel/actions/runs/34412249100): 세 프로필 job과 원본 보고서 업로드 모두 success. workflow artifact 메타데이터의 head SHA는 `235fe2c3c549d5f880a1185a0cb239870ba75ca9`이다. artifact 이름에는 PR synthetic merge SHA가 들어가므로 이름만으로 평가 head를 판단하지 않았다.
- [Release verification #10](https://github.com/FlitCatz/policy-kernel/actions/runs/34412249018): 모든 단계 success. 내려받은 실제 로그에서도 277 통과·1 ignored를 확인했다. 이 PR workflow는 synthetic merge `57230d99044e59f711897301e6e77879e3631122`를 checkout했다. 따라서 이것을 실제 PR head를 직접 checkout한 검사와 혼동하지 않는다. 로컬 검증과 Task 14 workflow는 위의 실제 PR head를 사용했다.
- [Task 10 live API smoke #4](https://github.com/FlitCatz/policy-kernel/actions/runs/34412249010): 브랜치 조건에 의해 skipped. 이전 커밋의 blocked-check 발행 성공은 현재 후보의 READY/SHIPPED 증거가 아니다.

CLI의 GitHub 접근은 실패했으나 연결된 GitHub 앱으로 run/job/log/artifact 메타데이터를 읽어 확인했다. 원격 쓰기, workflow 재실행, merge, 보호 설정 변경은 수행하지 않았다.

**검토 범위와 한계**

계약/core/CLI/스키마, GitHub·CI·수집기, 프로젝트 검증·보고서·bootstrap, acceptance·xtask·workflow 및 관련 설계·테스트를 검토했다. advisory 강제, digest와 head 결합, 모든 수정자에 대한 리뷰 독립성, waiver와 시간 경계, required test counts, merge/squash 구분, 보고서 변조·경로 제한, bootstrap 복구·구버전 receipt는 기존 검사와 이번 실행에서 의도한 동작을 확인했다.

세 보조 검토는 동일 provider의 엔지니어링 검토이다. [promotion 프로토콜](../../promotion.md)이 요구하는 두 독립 provider 승인, 실제 보호 merge, 14일/30개 head shadow 관측, 실제 보호 설정 rollback을 대신하지 않는다. 현재 보호 설정·계정 기능은 이번에 재확인하지 않았으므로 이전 문서의 제한을 현재 사실로 단정하지 않는다.

추가 조사 후보: .NET dependency 센서는 App.csproj만 명시적으로 재감사하고 테스트 프로젝트는 `--no-restore`로 실행한다. 다만 설치 시 restore와 TreatWarningsAsErrors도 작동하므로 테스트 전용 취약점의 전체 실행 우회는 재현하지 않았고 확정 결함 수에 포함하지 않았다.

README와 일부 Task 12/13 상태 문서는 아직 실행 전 상태를 서술한다. 이 기록의 실제 검증 결과에 맞춰 현재 상태와 역사적 기록을 구분하는 문서 정리가 필요하다. 설계 변경과 실제 authoritative 소비 프로젝트 통합은 별도의 작업으로 추적해야 한다.
