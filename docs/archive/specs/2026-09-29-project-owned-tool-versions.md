# 프로젝트 소유 도구 버전 — 설계 초안

작성일: 2026-09-29. 상태: 검토용 제안. 구현·호환성 인증·운영 승인을 의미하지 않는다.

## 목적과 사용자 맥락

사용자는 레거시 소스 없이 PowerBuilder ERP의 업무를 React/TypeScript/refine
프런트엔드로 약 2년 동안 점진 이관한다. GSD Core로 구현을 진행하고,
policy-kernel은 초기부터 검사 증거 수집·판단에 연결할 예정이다.
도구 버전을 올릴 때마다 kernel 소스를 수정하는 부담을 줄이는 것이 이번 목표다.

성공 기준: 프로젝트가 선택한 Node/npm/테스트 실행기 버전을 변경해도 기존
명령·보고서 계약을 만족하면 kernel 재빌드가 필요하지 않는다. 실제 버전과 원본
결과는 계속 기록하며, 프로젝트가 선언한 제한은 계속 강제한다.
보고서 계약이 바뀌면 어댑터 수정이 필요할 수 있다. 모든 미래 버전의 호환성을
자동 보증하는 기능은 아니다.

## 확인한 현재 구조

- [selection.rs](../../../crates/project-verification/src/selection.rs)의
  `ToolSelection::validate_capabilities`에 실행기별 허용 버전 범위가 있다.
  Playwright는 `>=1.55.0, <1.56.0`, npm은 `>=10.9.3, <11.0.0`이다.
- 같은 파일의 `VersionRule`은 `requirement`, `normalization`, `provision`을
  모두 요구한다. `validate_runtime_file`과 `ci_inputs`도 provision에 의존한다.
- [consumer.rs](../../../crates/project-verification/src/consumer.rs)는
  `Playwright163`, `Dotnet11Rc1` 및 VSTest 17.14.1에 대한 별도 계약을 가진다.
- [run.rs](../../../crates/project-verification/src/run.rs)는 버전 사전 관측,
  실제 수집 시 관측, 실행 파일 경로, 설정·보고서 digest를 결합한다.
- [bootstrap 계획](../../../bootstrap/src/plan.rs)은 사용자 프로필의
  스키마와 bootstrap release 조합을 제한한다.
- npm 감사는 현재 정해진 argv와 프로세스 결과를 검사한다. 테스트 JSON을
  읽을 수 있다는 사실만으로 npm 감사 의미까지 검증할 수는 없다.

## 접근 방식 비교

| 방식 | 이점 | 비용/문제 |
| --- | --- | --- |
| 기존 버전 범위를 계속 확장 | 변경이 작다 | 새 버전마다 kernel 수정이 반복된다 |
| 새 프로필에서 버전 선택과 검사 계약을 분리 | 기존 동작 보존, 프로젝트가 버전 선택 | 스키마·실행·설치 경계 작업 필요 |
| 범용 플러그인/정책 언어 도입 | 확장 폭이 크다 | 현재 ERP에 필요하지 않은 실행·인증 복잡성 |

두 번째 방식을 제안한다. 1차는 프런트엔드 npm 경로에서 끝까지 동작하는 단위다.

## 1차 범위와 호환성

- 신규 프로젝트 프로필 schema 4 / selection schema 3을 명시적으로 선택한다.
- 1차 schema 4는 frontend 컴포넌트와 npm graph만 허용한다. .NET/fullstack은
  기존 schema 2/3으로 유지한다. 전체 언어의 버전 독립화 완료로 보고하지 않는다.
- Vitest/Jest/Playwright 보고서와 process 검사, npm 설치·감사 계약을 지원한다.
- 기존 schema 1–3 및 bootstrap 1.0.0–1.2.0 파일·직렬화·판단을 보존한다.
- 새 schema에서는 `producer_contract` 버전 예외를 금지한다. 기존 schema에서만
  종전 의미를 유지한다. 새 버전용 enum을 추가하는 관행을 중단한다.
- policy-core와 policy-contracts의 승인 의미 및 공개 wire 계약은 변경하지 않는다.
  검토 중 증거 전달이 부족한 부분이 발견되면 별도 계약 변경으로 분리한다.

## 버전의 소유권과 새 계약

새 tool.version은 `normalization`과 명시적인 `acceptance`를 가진다.
`acceptance`는 다음 세 가지 중 하나이며, 누락을 허용 모드로 해석하지 않는다.

| kind | 필드 | 동작 |
| --- | --- | --- |
| record | 없음 | 정상 관측된 버전을 기록, 프로젝트 차원의 버전 제한 없음 |
| exact | value | 프로젝트가 지정한 버전 토큰과 정확히 일치 |
| numeric_range | requirement | 기존 3개 정수의 AND 비교 문법 적용 |

예: `{"normalization":"node","acceptance":{"kind":"record"}}`.
exact는 RC 등도 문자열 일치로 다룬다. numeric_range는 기존 제한 문법을 유지하며
prerelease를 자동 허용하지 않는다. 완전한 npm SemVer 해석기는 이번에 만들지 않는다.

관측은 성공한 bounded probe의 UTF-8 출력이어야 한다. 주변 공백 제거 후
1–128 ASCII 문자, `[A-Za-z0-9.+_-]`만 허용하고 첫 문자는 숫자로 제한한다.
node normalization은 선두 `v` 하나를 제거한 뒤 같은 규칙을 적용한다.
빈 출력, 여러 줄, NUL, 출력 초과, 실패한 probe는 record에서도 통과하지 않는다.
전체 raw stdout/stderr는 기존 수집 기록에 보존한다. 이 문법은 Go/JDK의 전체
배너를 처리하는 범용 문법이 아니다. 해당 언어의 probe 정규화는 후속 작업이다.

설치 선택은 `selection.provisioning: {tool_id: exact_version}`으로 분리한다.
선택 항목이며, 있으면 해당 도구의 acceptance를 만족해야 한다. 생략하면 실행
환경을 프로젝트 CI가 준비한다. kernel은 설치 버전을 추정하거나 최신판을 선택하지 않는다.
명시한 provisioning 값은 그 도구의 실제 관측과도 일치해야 한다.

`packages`의 새 acceptance도 exact/numeric_range/record를 사용한다. record라도
설치된 package 버전과 lockfile 값의 일치, manifest/lock 전체 graph 검증은 유지한다.
package 변경은 잠금파일·프로필·설정 digest와 새 증거에 반영한다.

## 실행·보고서·감사 계약

새 selection에는 도구별 `contracts`를 선언한다. 초기 식별자는 `process/v1`,
`vitest-json/v1`, `jest-json/v1`, `playwright-json/v1`, `npm-ci-audit/v1`이다.
이 식별자는 도구 버전이 아닌 이 저장소가 검증하는 동작·데이터 계약이다.
지원하지 않는 계약은 거부한다. capabilities/센서/graph와의 교차 검증을 유지한다.
보고서 없는 process 센서로 필수 테스트나 npm audit을 대신할 수 없다.

계약 ID는 기존 파서로 명시적으로 매핑한다. 기존 parser_version 문자열을
일괄 변경하지 않는다. 계약 선언은 전체 profile digest에, 실행·파서·설정·원본
보고서는 기존 증거 결합 경로에 들어간다. 새 selection에는 생산자 버전 범위표를
적용하지 않는다. record를 선택해도 알려진 실패/누락 검사 규칙은 모두 유지한다.

보고서의 필수 값·타입·카운터·실행 시각·개별 테스트·중복·재시도/오류 처리를
검사한다. 알 수 없는 필수 의미나 상태는 실패한다. 무관한 추가 metadata는 기존
규칙에 따라 허용한다. 성공 모양의 JSON으로 표현되지 않는 생산자 의미 변경은
파서만으로 발견할 수 없다는 한계를 명시한다.

`npm-ci-audit/v1`은 기존 manifest/lockfile v3 검사, 명시 argv, dev/optional/peer
포함, 전체 선택 graph 감사, 비정상 프로세스 거부를 유지한다. 다른 lockfile 형식,
다른 package manager 또는 다른 감사 프로토콜은 별도 어댑터가 필요하다.
새 npm의 명령 의미까지 자동 추정하지 않는다. 프로젝트 버전 변경 PR에서 실제
감사 성공·실패·서비스 오류를 확인하는 acceptance 실행을 제공한다.

## runtime 파일과 CI

- schema 4의 `.node-version` 선언은 선택 사항이다. 선언했으면 파일 digest를
  결합하고 그 값, 실제 runtime 관측, acceptance, 선택적 provisioning이 일치해야 한다.
  파일에는 정확한 버전만 허용한다. `lts/*` 같은 동적 alias는 1차 지원하지 않는다.
- 선언하지 않은 runtime 파일을 kernel이 임의로 설정하거나 인증한 것으로 취급하지 않는다.
  tool 관측과 프로젝트가 선언한 입력에 대한 보장 범위를 문서화한다.
- `ci-inputs`는 명시한 provisioning 값만 기존 이름으로 출력한다. 없으면 해당 키를
  생략하고 profile digest는 출력한다. 여러 도구가 충돌하는 값을 내면 오류다.
- `provision-npm`은 명시적인 npm provisioning이 없으면 64로 종료하고 설치하지 않는다.
- 출력 형식 오류 및 새 프로필 검증 오류는 기존 CLI 오류 경로 64를 사용한다.
  실행 실패의 2와 구성/수집 오류의 64를 기존 run 계약에 맞게 테스트한다.

## 마이그레이션과 bootstrap

기존 `migrate-profile FILE`은 유지한다. 새 명시 옵션
`migrate-profile FILE --target-schema 4`를 추가한다(제안이며 아직 실행할 수 없음).
schema 2/3 frontend/npm 프로필만 입력으로 받는다. 기본 변환은 기존 요청 범위와
provisioning을 보존한다. exact producer 계약은 exact acceptance로 변환한다.
record로 완화하거나 provisioning을 제거하는 일은 프로젝트 소유자의 별도 편집이다.

생산자 범위표를 적용하지 않는 새 의미 때문에 변환 후 digest가 달라지고 재채택·재검사가
필요하다. 변환은 stdout으로 후보를 출력할 뿐 파일·정책·설치 상태를 변경하지 않는다.

custom-only bootstrap 1.3.0 / receipt schema 4를 제안한다. 사용자 profile과
package/runtime/CI 파일은 계속 사용자 소유다. 새 설치 안내는 외부 환경 준비를
기본으로 설명한다. 계획 검증·원자적 적용·복구·uninstall의 기존 안전 조건을 보존한다.

## 검증 및 운영 도입의 분리

개발 검증은 (1) Rust 회귀/음성 테스트, (2) 실제 frontend producer 두 조합,
(3) hosted 후보 검증으로 나눈다. 가짜 버전 배너 테스트는 범위표 제거의 증거일 뿐
실제 생산자 호환성 증거로 보고하지 않는다. 전체 버전 조합의 인증은 주장하지 않는다.

실제 프로젝트의 버전 변경 PR에서 frozen conformance 사례를 실행할 수 있게 한다.
정상·실패 테스트, 누락/빈 보고서, 재시도, 감사 실패·서비스 오류를 확인하고
정확한 버전·lockfile·실행 기록을 보존한다. fixture나 검사 도구를 같은 PR에서
임의 변경한 결과를 독립적인 승인 근거로 삼지 않는다.

그 뒤 ERP의 실제 PR에서 관찰 운영한다. 인증된 실행자/리뷰어, 격리 환경,
GitHub App 및 보호 브랜치, 독립 리뷰, 운영 승인·복구 절차는
[현재 상태](../../status.md)와 [promotion](../../promotion.md)의 별도 조건이다.
이번 버전 분리가 그 공백을 해결하거나 로컬 증거의 신뢰 등급을 올리지 않는다.

## 후속 범위

.NET SDK/TRX/테스트 SDK/프레임워크 선택의 분리는 별도 단계에서 진행한다.
Go/Kotlin, pnpm/yarn/bun, GSD 전용 연동, 범용 플러그인·정책 언어도 이번 범위 밖이다.
프런트엔드 결과에서 공통 인터페이스를 확인한 뒤 각각 실제 소비자가 필요할 때 추가한다.

[구현 계획](../plans/2026-09-29-project-owned-tool-versions.md)
