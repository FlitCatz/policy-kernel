# Project-owned tool versions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Execution method recommendation: inline, because schema, collector and bootstrap changes share interfaces. This document does not authorize implementation.

**Goal:** React ERP가 도구 버전을 바꿀 때 kernel 소스 수정 없이 실제 검사와 증거 수집을 계속할 수 있게 한다.

**Architecture:** frontend 전용 profile 4 / selection 3을 명시적으로 추가한다.
프로젝트의 버전 acceptance와 선택적 provisioning을 보고서·명령 계약에서 분리한다.
기존 profile과 core의 승인 의미는 보존한다.

**Tech Stack:** Rust workspace, serde/schemars, 기존 collector/report parsers,
React npm fixtures와 GitHub Actions의 최소 선언 설정.

**Spec:** [설계 초안](../specs/2026-09-29-project-owned-tool-versions.md)

상태: 2026-09-29 검토용 계획. 아래 작업은 아직 구현/실행하지 않았다.
사용자가 요청한 계획 작성까지 수행하며, 실행 전 설계와 계획을 함께 검토한다.

## Global Constraints

- Rust로 실행 로직·CLI·검증·보고서 변환·설치·테스트를 구현한다. Python을 추가하지 않는다.
- 신규 프로젝트 프로필 schema 4 / selection schema 3을 명시적으로 선택한다.
- 1차 schema 4는 frontend 컴포넌트와 npm graph만 허용한다.
- 기존 schema 1–3 및 bootstrap 1.0.0–1.2.0 파일·직렬화·판단을 보존한다.
- policy-core와 policy-contracts의 승인 의미 및 공개 wire 계약은 변경하지 않는다.
- 새 관측 토큰: 주변 공백 제거 후 1–128 ASCII 문자, `[A-Za-z0-9.+_-]`, 첫 문자 숫자.
- 프로필 변경은 새 digest·명시적인 재채택·새 증거를 요구한다.
- README.md/README_KO.md를 함께 갱신하고 현재 상태는 docs/status.md에 기록한다.
- 운영 승인은 별도이며 과거 실행 기록을 현재 결과에 맞춰 수정하지 않는다.

## Review Focus

1. record 모드가 probe 실패/빈 출력까지 허용하지 않도록 Task 1/3에서 검사한다.
2. 구 프로필에 새 필드를 섞어 기존 제한을 우회하지 못하도록 Task 1/5에서 검사한다.
3. 설치하지 않는 모드에서 runtime 파일·실제 실행 도구가 달라지는 문제를 Task 3에서 검사한다.
4. 버전 범위를 없애며 npm 감사 대상이 빠지는 문제를 Task 2/6에서 검사한다.
5. 최신 결과처럼 보이는 이전 head/설정/보고서 재사용을 Task 3/6에서 검사한다.

## 변경 지도

| 영역 | 파일 | 책임 |
| --- | --- | --- |
| 새 버전 규칙 | 새 crates/project-verification/src/tool_versions.rs | acceptance와 관측 정규화 |
| 계약 매핑 | 새 crates/project-verification/src/adapter_contracts.rs | report/command 계약 ID와 기존 실행기 매핑 |
| 프로필 | crates/project-verification/src/profile.rs, selection.rs, consumer.rs, lib.rs | 새 schema·legacy 분기·composition |
| 실행 | crates/project-verification/src/run.rs | 관측·runtime·계약·digest 결합 |
| CLI | crates/project-verification/src/main.rs | 선택적 provisioning와 명시적 migration |
| 설치 | bootstrap/src/plan.rs, templates.rs, 새 bootstrap/templates/1.3.0/ | 사용자 소유 profile과 receipt 4 |
| 실제 검증 | acceptance/src/, acceptance/tests/, examples/ | 실제 버전 변경과 음성 사례 |
| CI/문서 | .github/workflows/, docs/integrations/, README.md, README_KO.md, docs/status.md | 실행 확인과 적용 범위 |

## Task 1: 새로운 버전 규칙과 스키마 경계

**Files:** 위 tool_versions.rs, profile.rs, selection.rs, lib.rs;
새 tests/tool_versions.rs 및 기존 tests/selection.rs, tests/consumer.rs.
여기서 tests/는 crates/project-verification/tests/를 뜻한다.

**Interfaces:**
- `VersionAcceptance`: serde tag `kind`, `Record`, `Exact { value: String }`,
  `NumericRange { requirement: String }`.
- `ProjectVersionRule { normalization: VersionNormalization, acceptance: VersionAcceptance }`.
- `ProjectVersionRule::observe(&self, stdout: &[u8]) -> Result<String, Error>`.
- 기존 `VersionRule`은 legacy 타입으로 보존. tool.version은 strict한 legacy/project
  variant를 가지는 `ToolVersionRule`로 읽고 profile/selection pair에서 variant를 강제한다.
- packages에도 대응하는 legacy/new wire 분기를 두어 old requirement 직렬화를 보존한다.
- `selection.provisioning`과 `selection.contracts`는 새 schema에만 허용한다.
  legacy 직렬화에는 키를 출력하지 않으며 legacy 입력에서 빈 값으로 넣어도 거부한다.

- [ ] 실패 테스트 작성: record는 `99.0.0`, exact는 `11.0.100-rc.1.26425.128`을
  처리하고, 범위 밖 numeric/빈값/멀티라인/129자/unknown field/null을 거부한다.
- [ ] 기존 schema와 새 필드의 모든 잘못된 조합을 reader/JSON Schema 양쪽에서 거부하는
  테스트를 작성한다. legacy 직렬화·digest가 같고 새 policy가 digest를 바꾸는지 확인한다.
- [ ] `rtk cargo test --locked -p project-verification --test tool_versions`로 실패 확인 후
  새 규칙과 variant dispatch를 구현한다. 우선 기존 API 호출부를 legacy 경로로 유지한다.
- [ ] 새 테스트 및 기존 selection/consumer 테스트를 실행한다. 지원하지 않는 fullstack/.NET
  새 schema는 명확한 오류이고 기존 profile은 그대로 통과해야 한다.
- [ ] 변경분을 검토하고 기능 단위로 커밋한다.

## Task 2: 생산자 버전과 어댑터 계약 분리

**Files:** adapter_contracts.rs, selection.rs, consumer.rs, reports.rs;
새 tests/adapter_contracts.rs, 기존 tests/reports.rs, tests/normalization.rs.

**Interfaces:**
- `AdapterContract`: `process/v1`, `vitest-json/v1`, `jest-json/v1`,
  `playwright-json/v1`, `npm-ci-audit/v1`의 serde 식별자.
- `Selection.contracts`: tool ID에서 `BTreeSet<AdapterContract>`로 매핑.
- `validate_adapter_contracts(selection: &Selection, profile: &ProjectProfile) -> Result<(), Error>`.
  tool/capability/report/graph 계약을 교차 검증하고 기존 parser_version으로 매핑한다.
- `validate_capabilities`의 버전 범위 검사는 legacy 경로에서만 사용한다.

- [ ] 실패 테스트: 합성 버전 `99.0.0`+정상 Playwright 보고서는 새 schema의 계약을
  만족하고, 같은 입력의 legacy 범위 위반은 계속 거부되어야 한다.
- [ ] 알 수 없는 계약, 계약 누락, producer_contract 혼합, test를 process로 대체,
  npm audit capability만 선언하고 graph 감사를 생략하는 사례를 거부한다.
- [ ] 기존 파서의 카운터 불일치·빈/누락/실패/재시도 보고서 거부를 새 경로에서 재실행한다.
- [ ] 계약 검사 구현 후 `rtk cargo test --locked -p project-verification --test adapter_contracts`
  및 report/normalization 테스트를 통과시킨다. npm lock v3·argv·전체 dependency class
  요구는 유지한다. 패키지 매니저 버전 범위만 새 schema에서 분리한다.
- [ ] 결과를 검토하고 커밋한다. 합성 버전 테스트를 실제 도구 호환성으로 설명하지 않는다.

## Task 3: 외부 설치 환경과 증거 결합

**Files:** run.rs, selection.rs, tests/selection.rs, tests/normalization.rs,
tests/runtime_independence.rs, tests/path_safety.rs.

**Interfaces:**
- `Selection::observe_tool(&self, tool_id: &str, stdout: &[u8]) -> Result<String, Error>`:
  legacy/new acceptance와 선택적 provisioning 일치를 한곳에서 처리한다.
- `Selection::validate_observed_runtime(&self, tool_id: &str, actual: &str,
  files: &BTreeMap<(String, String), Vec<u8>>) -> Result<(), Error>`:
  선언한 component/path별 runtime 파일과 관측의 일치를 검사한다.
- 기존 NormalizedResult.observed_version, version_preflight_digest와 전체 configuration/
  profile digest를 사용한다. RunResult/wire에 새 필드 없이도 감사 가능한지 확인한다.

- [ ] 실패 테스트: provisioning/runtime 파일 없는 외부 환경에서도 정상 probe와 실행을
  수집하고, probe 실패·실행 경로 변경·사전/수집 버전 불일치는 거부한다.
- [ ] 선언한 `.node-version`과 관측 불일치, provisioning과 관측 불일치, package/lock
  불일치, 입력 변경, 이전 head/보고서 재사용을 거부하는 테스트를 작성한다.
- [ ] 실행 전후 검증을 구현한다. 설정·계약·lock 변경이 configuration/profile/result
  digest에 반영되는지 검증한다. 수집 결과는 계속 LocalUntrusted다.
- [ ] `rtk cargo test --locked -p project-verification`을 실행한다. 기존 명령 실패 2와
  수집·구성 오류 64의 CLI 계약이 바뀌지 않았는지 확인한다.
- [ ] 실제 tool 교체를 경로/버전 관측 이상으로 완전히 방지한다고 주장하지 않으며,
  실행 환경 신뢰의 한계를 기록한 뒤 커밋한다.

## Task 4: CLI와 명시적 migration

**Files:** main.rs, selection.rs, 새 tests/project_versions_cli.rs,
docs/integrations/tool-selection.md, README.md, README_KO.md.

**Interfaces:**
- `upgrade_frontend_profile(profile: ProjectProfile) -> Result<ProjectProfile, Error>`.
- 제안 CLI: `migrate-profile FILE --target-schema 4`; 기존 인자는 기존 의미 유지.
- `ci_inputs(&ProjectProfile)` 시그니처 유지, 새 schema에서는 명시 provisioning만 출력.
- `provision-npm`은 npm provisioning이 없으면 설치 실행 없이 exit 64.

- [ ] CLI 테스트 작성: provisioning 없음/있음/충돌, migration stdout만 변경,
  .NET/fullstack 거부, legacy migration 보존, release/version/lock 값 보존을 확인한다.
- [ ] 변환은 기존 요청 범위와 exact producer 요구를 유지하고 record로 자동 완화하지 않는다.
  새 schema 채택으로 기존 adapter interval을 적용하지 않는다는 사실을 stderr/문서에 알린다.
- [ ] CLI를 구현하고 `rtk cargo test --locked -p project-verification --test project_versions_cli`
  통과를 확인한다. 실제 usage와 문서의 인자·exit code를 대조한다.
- [ ] 양쪽 README와 사용자 지침을 함께 갱신하고 커밋한다.

## Task 5: custom bootstrap 1.3.0

**Files:** bootstrap/src/plan.rs, bootstrap/src/templates.rs,
bootstrap/templates/1.3.0/, bootstrap/tests/consumer.rs, installer.rs,
docs/integrations/bootstrap.md, README.md, README_KO.md.

**Interfaces:** release 1.3.0 + profile 4 + receipt 4 조합만 새로 허용.
기존 설치 CLI 인자는 재사용한다. native 파일은 사용자 소유이며 설치 도구는 package
manager를 실행하지 않는다. helper/example 파일만 고정 소유 inventory로 관리한다.

- [ ] 실패 테스트: 올바른 새 조합 설치, schema 섞기 거부, 계획 이후 profile 변경 거부,
  provisioning 없는 사용자 profile 설치, 원자적 복구와 uninstall 소유권 보존.
- [ ] 새 template/release/receipt dispatch 구현. 수정·누락된 사용자 profile로도 기존
  receipt 기준 uninstall이 보존 동작을 수행하고 편집된 managed file은 충돌해야 한다.
- [ ] `rtk cargo test --locked -p policy-bootstrap` 실행. 구 receipt/template 바이트와
  기존 설치·uninstall 회귀 결과를 확인한다.
- [ ] bootstrap 문서와 양쪽 README를 갱신하고 커밋한다.

## Task 6: 실제 frontend 검증과 운영 인계

**Files:** 새 examples/react-project-versions/의 최소 native fixture,
새 acceptance/tests/native_project_versions.rs, 필요한 acceptance/src/ 확장,
선택한 기존 GitHub Actions consumer workflow, docs/integrations/report-formats.md,
docs/integrations/acceptance.md, docs/status.md, README.md, README_KO.md.

**Interfaces:** 같은 빌드의 kernel에 다른 profile/lock/환경을 공급해 반복 실행하는
Rust harness. 새 무시된 native test는 `native_project_versions`라는 이름을 사용한다.
출력 디렉터리는 기존 harness처럼 매 실행 새 경로를 요구하고 버전·artifact·head를 기록한다.

- [ ] native matrix 후보를 고정한다: 기존 React producer 조합과 Task 16에서 사용한
  Node 26.10.0/Playwright 1.63.0 조합을 출발점으로 재사용한다. npm은 10.9.3과
  12.1.0 후보를 실제 설치·감사로 검증한다. 실패하면 숨기지 말고 원인을 기록한다.
  후보 값은 이번 계획에서 실행 확인한 버전이 아니다. 실제 결과만 지원 근거로 적는다.
- [ ] 정상 보고서, 실패/빈/누락 보고서, Playwright retry, 이전 head 재사용,
  npm dev/optional/peer 누락 시도, 실제 감사 실패 및 통제된 서비스 오류 사례를 실행한다.
  서비스 실패 테스트와 실제 advisory 테스트의 증거를 구분한다.
- [ ] kernel 소스·바이너리를 바꾸지 않고 두 환경을 통과시키고, 부정 사례는 거부한다.
  같은 frozen conformance runner를 소비자 버전 변경 PR에서 호출하는 절차를 문서화한다.
- [ ] 필요 조건을 갖춘 환경에서 다음을 실행하고 정확한 후보·결과를 새 기록으로 남긴다:
  `rtk cargo test --locked -p policy-acceptance --test native_project_versions -- --ignored --nocapture`.
- [ ] `rtk cargo test --locked --workspace`, `rtk cargo fmt --all -- --check`,
  `rtk cargo clippy --locked --workspace --all-targets -- -D warnings`,
  `rtk cargo run --locked -p xtask -- check-residue`를 실행한다.
  reader/schema 일치 테스트와 기존 core contract schema 무변경도 확인한다.
- [ ] Markdown 상대 링크와 `rtk git diff --check`를 검사한다. 실제 workflow의 기존
  필수 gate를 확인해 hosted 후보 검증을 수행하고, 로컬 결과와 별도로 기록한다.
- [ ] 새 Task landing page와 index/current status를 연결한다. profile 4 frontend 범위,
  legacy .NET 제한, 실제 확인 버전, 미확인 환경을 명시하고 양쪽 README를 맞춘다.
- [ ] 코드 검토 후 커밋한다. 보호 브랜치·인증·독립 리뷰·승인·복구 요건이 충족되지
  않았다면 관찰 운영까지만 인계하고 필수 병합 검사 전환 완료로 보고하지 않는다.

## 완료 기준과 후속 작업

1. 새 프런트엔드 경로에서 버전 선택만 바꿔도 kernel 재빌드가 불필요하다.
2. 설치 버전 선택은 선택 사항이고, 실제 버전 관측은 필수다.
3. 프로젝트가 선언한 버전 제한·runtime/lock 일치는 강제된다.
4. 기존 보고서/감사/증거 실패 조건과 legacy 동작이 유지된다.
5. 실제 producer 검증과 hosted 검증의 수행 여부가 명확히 기록된다.
6. 이 작업 완료와 production promotion은 별개로 보고한다.

후속 순서: ERP 관찰 운영 → .NET의 별도 버전 결합 해소 → 실제 소비자 필요에 따른
Go/Kotlin/package manager 지원. GSD adapter와 인증 서비스 구축은 별도 작업이다.
이 계획은 해당 작업들이 이미 완료됐다는 전제를 두지 않는다.
