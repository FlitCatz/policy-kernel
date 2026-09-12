# Task 15 — 호환성, 실행 독립성 및 검증 완결성 통합 작업 계획

작성일: 2026-09-11

기준 커밋: `abd6cb0a1fcd1440cc96a05bfe4ce00280742511`

상태: 작업 목록 작성 완료. 아래 구현·운영 작업은 미착수.

현재 상태 진입점: [Task 15](../../tasks/task-15/README.md)

## 목적과 사용자 요구

policy-kernel을 특정 Rust 버전, 소비 프로젝트 도구 버전, 개발 워크플로
프레임워크에 불필요하게 결합하지 않으면서 기존 정책 검증의 강도를 유지한다.
프로젝트가 필요한 도구와 호환 버전을 선택하고, 실행기는 실제 선택·실행 결과를
기록하며, core는 채택된 정책과 증거 계약을 평가하도록 책임을 나눈다.

superpowers, agent-skills, gsd-core가 실제 빌드·설치·실행·필수 CI 경로에
영향을 주는 결합은 제거 대상이다. 이름 검색 결과만으로 의존성을 판정하지 않는다.
이 계획의 실행에 특정 스킬, 에이전트, 모델 또는 워크플로 프레임워크는 필요하지 않다.

이 문서는 사용자가 요청한 통합 추가 작업 목록이다. 이번 문서 작성은 제품 구현,
외부 계정 설정 변경, 원격 게시 또는 운영 승격을 수행한 것으로 계산하지 않는다.
각 작업은 변경 대상, 수행 항목, 검증 및 완료 조건을 갖는 독립적인 검토 단위이다.

## 기준 자료와 현재 판정

- [최종 코드베이스 검증](../reviews/2026-09-10-final-codebase-verification.md):
  `235fe2c`의 결함 3건, 필수 knowledge 및 authoritative 연계 누락.
- [Task 14 후속 수정](../reviews/2026-09-10-task14-remediation.md):
  `5841d8b`에 포함된 결함 수정과 1.0.2 자산, 실행 및 운영 제약 기록.
- [버전·라이브러리 옵션화 검토](../reviews/2026-09-10-dependency-optionality-review.md):
  `5841d8b`의 종속 지점과 옵션화 권고. 대체 조합의 지원 인증은 수행하지 않음.
- [추출 설계](../specs/2026-09-04-policy-kernel-extraction-design.md),
  [프로젝트 검증 설계](../specs/2026-09-07-project-verification-bootstrap-design.md),
  [신뢰 모델](../../trust-model.md), [프로모션 프로토콜](../../promotion.md).

| 검토 항목 | 기준 커밋의 사실 | Task 15 처리 |
| --- | --- | --- |
| Fullstack 공용 npm 감사 | 1.0.2에는 필수 `shared:dependencies`와 회귀 테스트가 있음 | 수정 유지. 구버전으로 연결되는 현재 안내 정리 |
| SensorProfile 스키마 불일치 | 스키마 제약 및 Rust 대조 테스트 추가됨 | 새 계약·옵션화에서도 회귀 유지 |
| GitHub 자손 프로세스 잔존 | 소유 프로세스 그룹 및 제한된 I/O로 수정됨 | 성공·실패·타임아웃·출력 초과·막힌 입력 정리 유지 |
| 버전·도구 옵션화 | 프로필 일부 값은 데이터이나 설치기·인수검증기·CI는 고정 조합에 결합 | 15.03–15.06, 15.10 |
| .NET 테스트 의존성 감사 | 명시적 재감사는 앱 프로젝트만 포함. 테스트 전용 취약점의 전체 실행 우회는 미확정 | 15.07에서 재현 및 감사 범위 완성 |
| 필수 knowledge/index | 제출된 증거 검증은 있으나 필수 존재 선언은 없음 | 15.08 |
| authoritative 소비 프로젝트 연결 | 외부 trusted controller의 관측값을 소비. 배포된 controller 증거는 없음 | 15.09–15.11 |
| 프레임워크 이름 | 주로 과거 문서·현재 통합 안내·금지 문자열. 조사한 Cargo 의존성에는 세 프레임워크가 없음 | 15.02에서 실제 실행 독립성 입증 |
| 문서 재편 | 현재 상태·Task 색인·아카이브·LAT 탐색층 도입 | 새 구조에 남은 작업과 근거 연결 |

작성 전 로컬 재검토에서는 `cargo test --workspace --locked --offline` 결과
281개 통과, live GitHub 테스트 1개 ignored를 관측했다. 상대 인라인 링크 175개의
대상 경로가 존재했고 `git diff --check`도 통과했다. 이는 위 기준 커밋의 로컬 관측이며,
Task 15 구현 후 결과나 네이티브 재실행, 새 mutation, 원격 CI 또는 운영 증거가 아니다.

## 공통 설계 제약

1. 실행 로직, CLI, 어댑터, 검증 도구, 보고서 변환, bootstrap, 테스트는 Rust로 작성한다.
   승인된 `examples/`의 네이티브 fixture 예외는 유지한다. Python 스크립트를 추가하지 않는다.
2. 소스 빌드의 컴파일러 호환성, 배포 빌드의 재현성, 소비 프로젝트의 실행 도구 선택을
   구분한다. 빌드된 바이너리 사용자가 Rust 컴파일러까지 설치해야 한다는 조건을 만들지 않는다.
3. 호환 범위와 기능 계약으로 지원 여부를 판정하고, 실제 실행 버전·argv·설정·lockfile·
   parser 식별자는 정확히 기록한다. 버전·계약 식별자를 증거에서 제거하지 않는다.
4. 검증된 프리셋은 기본값이다. 모든 사용자에게 같은 버전을 강제하지 않으며, 미지원 조합은
   설명 가능한 실패로 처리한다. 버전 문자열 검사 삭제만으로 지원 범위를 넓혔다고 하지 않는다.
5. 선택값과 호환성 정책 자체도 채택된 profile/policy digest에 결합한다. 실행자가 임의로
   허용 범위나 필수 검사를 느슨하게 바꾸어 기존 승인과 증거를 재사용할 수 없어야 한다.
6. 배포 검증에는 선택한 컴파일러·도구의 정확한 버전과 잠긴 의존성 조합을 사용한다.
   `Cargo.lock` 제거, 무제한 의존성 갱신, 미검증 모든 버전 허용은 목표가 아니다.
7. `bootstrap/templates/1.0.0`, `1.0.1`, `1.0.2`의 배포 의미와 기존 receipt의
   파일 소유권·복구 의미를 보존한다. 수정된 자산은 새 릴리스로 발행한다.
8. 계약 변경은 명시적으로 버전 관리한다. 과거 evidence, digest, 승인 및 실행 기록을
   새 의미로 조용히 재작성하지 않는다. 스키마와 Rust 역직렬화 제약을 함께 검증한다.
9. 필수 테스트, 빈 보고서 거부, 모든 대상 의존성 그래프 감사, 이전 head 거부,
   모든 수정자로부터의 리뷰 독립성, timeout/overflow 정리 및 신뢰 경계를 유지한다.
10. 일반 실행에 LAT·CodeGraph·개발 스킬 설치를 강제하지 않는다. 프로젝트가 knowledge를
    필수로 채택한 경우에만 해당 증거를 요구하며, 로컬 인덱스 존재를 CI 신뢰로 간주하지 않는다.
11. 공개 문서의 버전·명령·종료 코드를 실제 구현과 대조한다. README 두 언어판을 함께
    갱신하고, 현재 상태는 `docs/status.md`, Task별 상태는 `docs/tasks/`에 기록한다.

## 순서와 의존 관계

| ID | 우선순위 | 선행 조건 | 독립적으로 검토할 결과 |
| --- | --- | --- | --- |
| 15.01 | 우선 | 기준 자료 확인 | 현재 안내의 정확성 |
| 15.02 | 우선 | 없음 | 프레임워크 실행 독립성 |
| 15.03 | 기반 | 없음 | Rust 빌드·배포 호환성 정책 |
| 15.04 | 기반 | 15.01 | 공통 도구·경로 선택 계약 |
| 15.05 | 기반 | 15.04 | 사용자 프로필과 안전한 설치 소유권 |
| 15.06 | 확장 | 15.04–15.05 | 교체 가능한 도구·보고서 처리 |
| 15.07 | 우선 | 현행 재현은 즉시, 구현은 15.04–15.06 | 누락 없는 의존성 감사 |
| 15.08 | 기반 | 없음 | 명시된 필수 knowledge 강제 |
| 15.09 | 통합 | 15.02, 15.05, 15.07–15.08 | 인증된 수집·정책 판정 경로 |
| 15.10 | 검증 | 검증할 15.02–15.09 구현 | 실제 호환 조합과 회귀 증거 |
| 15.11 | 운영 | 15.09–15.10 및 실제 외부 기능 | 후보에 결합된 운영 증거 |
| 15.12 | 문서 | 각 변경과 병행, 최종적으로 15.10–15.11 | 정확한 최종 상태 및 인수 기록 |

### 15.01 — 현재 프로필 진입점과 상태 안내 수정

대상: `README.md`, `README_KO.md`, `docs/integrations/bootstrap.md`,
`docs/integrations/project-verification.md`, `docs/status.md`, `docs/tasks/`.

- [ ] README의 직접 검증 예제가 가리키는 `profiles/fullstack/profile.json`이
  1.0.0이며 14개 센서라는 사실을 설명하고, 현재 사용 예제는 1.0.2 자산 또는
  설치된 `.policy/project-profile.json`을 명시적으로 선택하도록 바꾼다.
- [ ] `profiles/`의 과거 자산은 버전을 유지하고, 현재 지원 릴리스 안내와 구분한다.
  수정된 프로필에서 `shared:dependencies`가 필수이며 전체 15개 센서임을 대조한다.
- [ ] bootstrap 문서의 “1.0.0만 인식” 문구, 설치·제거 예제, 직접 프로필 안내의
  도구 pin 표를 실제 릴리스별 동작과 맞춘다. 설치 receipt와 제거 선택도 일치시킨다.
- [ ] Task 03의 필수 knowledge 누락, Task 10의 controller 연계, Task 14의
  authoritative 미완료를 현재 상태에서 찾을 수 있게 한다. 과거 검토의 결함 3건이
  수정됐다는 사실과 추가 미완료 항목을 별도로 표시한다.

검증: 현재 CLI로 구버전 및 1.0.2의 `validate-profile`을 실행해 센서 목록을 대조하고,
문서의 상대 링크·명령·종료 코드 및 `git diff --check`를 확인한다.
완료 조건: 일반 사용 안내를 따라가면 수정된 자산을 선택하며, 과거 기록과 현재 상태가 혼동되지 않는다.

### 15.02 — 실제 구동에 영향을 주는 프레임워크 결합 제거

대상: workspace 및 crate `Cargo.toml`, `Cargo.lock`, `.github/workflows/`,
`action/`, `bootstrap/`, `crates/`, `acceptance/`, `xtask/src/verification.rs`,
`xtask/tests/verification.rs`, `docs/integrations/agent-skills.md`, Task 09 문서.

- [ ] superpowers·agent-skills·gsd-core 관련 참조를 dependency, build/install hook,
  실행 명령, 필수 CI 단계, 환경·파일 전제, 문서 예시, 역사적 기록, 금지 테스트로 분류한다.
  이름 일치와 실제 호출·로딩·상태 조회를 구분한 결과를 남긴다.
- [ ] 실제 실행에 필요한 결합이 발견되면 제거하고, 필요한 기능은 일반적인 명령 실행·
  구조화된 증거 입력 계약으로 제공한다. `.planning` 상태나 특정 스킬 파일의 존재를
  정상 실행·정책 판정의 조건으로 삼지 않는다.
- [ ] 세 프레임워크, 사용자 전역 스킬 디렉터리, agent 설정이 없는 격리된 환경에서
  빌드된 CLI의 평가·검사, collector, project-verification, bootstrap을 확인한다.
  Rust fixture 실행기와 임시 설정을 사용해 로컬 환경의 우연한 설치에 기대지 않게 한다.
- [ ] 사용자 전역 설치를 삭제하지 않고도 위 검증을 할 수 있게 한다. 문서 파일 이름이나
  금지 문자열 테스트는 기능 결합이 아니므로 무조건 삭제하지 않는다.
- [ ] 현재 통합 문서는 일반적인 외부 리뷰 실행기 계약으로 정리한다. 이름을 바꾸면
  현재 링크를 갱신하고 과거 링크의 접근성을 보존한다. 아카이브의 필수 스킬 문구가
  현재 사용·개발의 필수 조건이 아니라는 안내를 아카이브 진입점에 둔다.
- [ ] 잔여물 검사의 범위에 bootstrap·acceptance·manifest·CI 경로를 포함할 필요를
  검토하고 실제 결합을 막는 검증을 추가한다. 금지 테스트 자신과 역사 문서를 이유로
  정상 배포가 실패하는 광범위한 문자열 차단을 만들지 않는다.

완료 조건: 세 프레임워크를 설치하지 않아도 지원하는 제품 경로가 동작하고,
수집·리뷰에 필요한 계약은 특정 프레임워크 없이 사용할 수 있다.

### 15.03 — Rust 빌드 호환성과 배포 재현성 분리

대상: `Cargo.toml`, 각 crate manifest, `Cargo.lock`, `rust-toolchain.toml`,
`.github/workflows/ci.yml`, README 두 언어판, 검증·배포 문서.

- [ ] 소스 및 의존성이 실제로 요구하는 최소 Rust 버전을 조사하고 빌드로 확인한다.
  근거 없는 하한을 만들지 않고 `rust-version` 및 workspace 상속 정책으로 표현한다.
- [ ] 최소 지원 컴파일러와 별도의 지원 컴파일러에서 build/test를 실행한다.
  두 조합 이상을 검증하기 전에는 단일 버전 고정에서 벗어났다고 선언하지 않는다.
- [ ] 내부 라이브러리의 정확한 `=version` 제약을 항목별로 검토한다. 호환 범위로
  바꿀 수 있는 의존성은 검증 후 변경하고, 유지할 exact pin은 이유를 기록한다.
  배포 검증은 계속 채택된 lockfile에 결합한다.
- [ ] 개발 기본 toolchain 및 배포용 exact compiler 선택은 허용하되, 이를
  소비 프로젝트의 필수 runtime 버전으로 해석하지 않게 한다.
- [ ] 빌드된 바이너리를 Rust 도구가 없는 환경에서 실행한다. 플랫폼·시스템 라이브러리
  제약과 소스 빌드 요구를 문서에서 구분한다. 새 바이너리 배포 경로를 제공할 때는
  대상 OS/아키텍처와 내용 digest를 함께 명시한다.

완료 조건: 단일 컴파일러만 허용하는 제품 요구 없이 지원 빌드 범위가 입증되고,
릴리스 산출물에는 실제 compiler 및 의존성 조합이 기록된다.

### 15.04 — 도구·버전·프로젝트 경로 선택 계약 통합

대상: `crates/project-verification/src/profile.rs`, `src/run.rs`,
`bootstrap/src/plan.rs`, `src/templates.rs`, `src/main.rs`,
`acceptance/src/execution.rs`, `src/staging.rs`, `src/process.rs`, 관련 CI 설정.

- [ ] Node·패키지 관리자·.NET SDK·대상 프레임워크·패키지 버전·앱/테스트 경로의
  중복 상수를 조사하고 하나의 명시적 선택 계약을 정의한다. SDK와 target framework,
  요청 범위와 실제 해석된 버전, 필수 기능을 각각 표현한다.
- [ ] 도구별 버전 출력 정규화와 호환 범위 검사 규칙을 정의한다. 해석 불가능한 출력,
  미지원 기능, 범위 밖 버전은 원인을 구분해 실패시킨다. 범위 비교를 느슨한 부분 문자열
  비교로 구현하지 않는다.
- [ ] 선택값에서 프로필, 생성 설정, bootstrap 감지, 실행 전 검증, acceptance 준비 및
  CI 입력을 일관되게 도출한다. `.node-version`, `global.json`, lockfile 등의 충돌을
  명시하고 기존 프로젝트를 조용히 다른 runtime으로 이동시키지 않는다.
- [ ] 실제 실행 버전, 실행 파일, argv, 선택 계약, 설정·입력 digest를 증거에 결합한다.
  기존 정확한 버전 프리셋의 읽기·실행 의미를 보존한다.
- [ ] 지원 범위 안의 서로 다른 버전 및 다른 앱/테스트 경로는 허용되고,
  범위 밖 버전·충돌하는 선택·실행 중 입력 변경은 거부되는 회귀 테스트를 추가한다.

완료 조건: 동일 선택이 설치·실행·CI·증거에 전달되고 코드의 여러 상수를 따로 바꾸지
않아도 지원하는 다른 조합을 사용할 수 있다.

### 15.05 — 프로필 계약·릴리스·사용자 설정 및 receipt 분리

대상: `crates/project-verification/src/profile.rs`, `src/main.rs`,
`bootstrap/src/plan.rs`, `src/templates.rs`, `src/transaction.rs`,
`bootstrap/tests/installer.rs`, 새 버전의 bootstrap 자산.

- [ ] `schema_version`의 호환성과 자산 `version`의 식별을 분리한다. 새 자산 버전이라는
  이유만으로 core/profile parser의 허용 문자열 목록을 고쳐야 하는 구조를 제거한다.
- [ ] 검증된 내장 프리셋과 사용자 프로필의 입력 경로를 정의한다. 사용자 도구·경로를
  허용하되 필수 컴포넌트·테스트·계약/E2E 등 채택된 구성 의무를 우회할 수 없게 한다.
- [ ] 사용자 프로필 입력과 installer의 파일 소유권을 구분한다. 임의의 receipt가
  기존 사용자 파일을 소유한다고 주장할 수 없도록 자산 출처·정확한 내용·관리 파일
  목록을 검증한다. 설치 계획에 선택값과 제안하는 파일 변경을 표시한다.
- [ ] 기존 1.0.0–1.0.2 receipt의 검증·업그레이드·다운그레이드·복구·제거를 보존한다.
  새 계약이 필요하면 명시적으로 version을 구분하고 구버전 경로를 유지한다.
- [ ] 알려지지 않은 계약, 변조한 receipt, 자산 변경, 사용자 파일 수정, 실패 중 복구,
  같은 선택의 반복 설치, 지원되는 다른 자산 릴리스에 대한 테스트를 추가한다.

완료 조건: 사용자 프로필 및 새 릴리스를 수용하면서 기존 소유권과 복구 보장이 유지된다.

### 15.06 — 패키지 관리자·빌드·테스트·보고서 어댑터 분리

대상: `crates/project-verification/src/profile.rs`, `src/reports.rs`, `src/run.rs`,
`bootstrap/src/plan.rs`, `src/templates.rs`, `acceptance/src/execution.rs`,
`src/failure.rs`, `docs/integrations/report-formats.md`, 해당 Rust 테스트.

- [ ] 패키지 관리자는 설치·lockfile 일치 확인·감사를 하나의 계약으로 표현한다.
  npm 이외 lockfile의 존재만으로 일괄 충돌시키는 대신 선택한 관리자와 입력을 검증한다.
- [ ] Vite·Vitest·ESLint·Playwright의 역할을 빌드·정적 검사·테스트·E2E 계약으로
  표현한다. 필요한 실제 소비 조합을 선정한 지원표를 먼저 작성하고 그 조합별 어댑터를
  구현한다. pnpm/yarn/bun, 다른 framework·runner 전체를 자동 지원한다고 선언하지 않는다.
- [ ] 기존 프리셋 외의 실제 소비 조합을 적어도 하나 검증해 교체 가능성을 입증한다.
  선택 기준은 소비 프로젝트의 기존 도구와 보고서 요구이며, 선정 결과·범위·제외 항목을
  해당 작업의 설계 기록에 남긴다.
- [ ] parser 계약 식별자, 지원 producer 범위, 실제 producer 버전을 분리한다.
  역사적 `vitest-json-3.2.4/v1` 이름을 실제 Vitest 설치 버전으로 해석하지 않는다.
- [ ] 보고서 어댑터마다 정상·실패·빈 보고서·누락·잘못된 형식·중복 ID·skip/retry·
  수정 후 새 head 및 이전 head 재사용 거부를 검증한다. 종료 코드 0만으로 테스트
  성공을 인정하지 않는다.

완료 조건: 지원표의 조합은 실제 실행 증거가 있고 도구 교체가 core 규칙 변경을 요구하지 않는다.
GitHub 이외의 호스팅 및 Windows 등 추가 플랫폼의 완전한 구현은 이 작업의 자동 확장 범위가
아니다. 현재 GitHub·Linux/macOS 경계를 명시하고, 추가 플랫폼은 별도 계약·인수검증 대상으로 추적한다.

### 15.07 — 모든 의존성 그래프 감사와 .NET 테스트 프로젝트 검증

대상: 새 프로필 자산, `crates/project-verification/src/run.rs`, 관련 audit/profile
테스트, `acceptance/src/execution.rs`, `acceptance/tests/`, 네이티브 fixture.

- [ ] 현행 .NET 앱·Unit·Integration의 restore/audit 경로를 각각 재현한다.
  테스트 프로젝트에만 취약한 의존성이 있는 경우, 사전 restore 유무, 캐시, 경고
  억제 설정을 구분해 기존 전체 흐름이 실패하는지 관측한다. 재현 전 우회로 단정하지 않는다.
- [ ] 선택된 모든 앱·테스트·shared manifest와 lockfile에서 감사 대상을 도출한다.
  .NET은 앱뿐 아니라 Unit·Integration 그래프도 명시적으로 현재 실행의 감사에 포함한다.
- [ ] `backend:dependencies`라는 특정 ID에 붙은 NuGet XML 검증을 감사 도구의
  선언된 검증 계약으로 옮긴다. 센서 이름 변경으로 audit suppression 검사 등이 빠지지 않게 한다.
- [ ] 모든 감사의 입력, 실제 버전, 명령, 결과, 실패 원인을 증거에 결합한다.
  감사 DB/서비스 오류·누락·timeout은 통과로 처리하지 않는다.
- [ ] frontend-only, shared-only, test-only 취약점 및 각 그래프의 감사 증거 누락을
  별도 회귀 사례로 만들고, 정상 그래프가 다른 그래프의 실패를 가리지 못함을 확인한다.
  네트워크 의존 재현과 결정론적 Rust fixture 회귀 결과를 구분한다.

완료 조건: 선택된 모든 의존성 그래프의 감사 수행 여부를 확인할 수 있고, 한 그래프만의
실패·누락도 정책에 반영된다. 기존 Fullstack 1.0.2 수정은 계속 보존된다.

### 15.08 — 정책이 선언한 필수 knowledge/index 증거 강제

대상: `crates/policy-contracts/src/policy.rs`, `src/evidence.rs`,
`crates/policy-core/src/provenance.rs`, `src/pre_merge.rs`, `src/pre_merge_digest.rs`,
관련 계약/core 테스트, `schemas/`, `xtask/tests/schemas.rs`, `docs/trust-model.md`.

- [ ] 정책에 필요한 knowledge의 종류·출처 식별·적용 조건을 명시하는 계약을 정의한다.
  선언이 없는 기존 정책에서는 knowledge가 비어 있는 것 자체로 실패시키지 않는다.
- [ ] 필수 항목 누락·잘못된 종류/출처·head 불일치·source-set 불일치·신뢰 부족을
  각각 실패 사유로 구분한다. knowledge는 테스트나 독립 리뷰를 대체할 수 없다.
- [ ] 새 필수 선언이 policy digest와 결정에 반영되게 하고, 구버전 계약·증거의
  해석을 보존한다. 새 reader/old reader의 거부 및 호환 동작을 명시한다.
- [ ] 필수 선언 없음/있음, 누락/유효/불일치, LocalUntrusted/인증된 관측의 조합을
  테스트하고 스키마·Rust 대조 및 정책 변경 후 이전 증거 거부를 검증한다.
- [ ] CodeGraph를 필수로 선택한 소비 프로젝트는 CI 재생성 또는 신뢰된 attestation으로
  tool version·index configuration·content·source set·head를 결합한다.
  로컬 `.codegraph/`나 LAT 탐색 문서만 존재하는 것으로 충족시키지 않는다.

완료 조건: knowledge가 기본 실행 의존성이 되지 않으며, 명시적으로 필수인 정책에서는
누락된 증거를 통과시키지 않는다.

### 15.09 — 인증된 수집에서 authoritative CI 판정까지 연결

대상: `crates/evidence-collector/`, `crates/policy-ci/src/main.rs`,
`crates/policy-ci/tests/`, `crates/github-adapter/`, `action/README.md`,
`action/action.yml`, 소비 프로젝트용 신뢰된 CI 연계 코드와 workflow.

- [ ] trusted controller의 소유자·실행 위치·입력 출처·권한 및 PR 코드와의 격리 경계를
  설계한다. collector의 로컬 출력은 LocalUntrusted를 유지한다.
- [ ] 현재 실행에서 직접 관측하거나 독립 인증한 evidence/context/work-item/policy와
  workflow 신원을 묶어 `identity.json`을 발행하는 참조 경로를 구현한다.
  파일을 checkout 밖으로 복사하거나 trust 필드를 바꾸는 것으로 인증을 대신하지 않는다.
- [ ] profile/policy 채택, 모든 수정자의 신원, 독립 리뷰, required sensors 및
  knowledge 관측이 실제 소비 프로젝트에서 같은 evaluated head로 연결되게 한다.
  core에 모델 선택·로그인·재시도·수정 루프를 추가하지 않는다.
- [ ] Action 및 실행 바이너리·정책·관측 디렉터리는 contributor가 교체하지 못하게 한다.
  PR head와 synthetic merge SHA, workflow run/attempt, publishing App identity를
  구별해 검증한다.
- [ ] 위조 JSON·로컬 artifact·다른 head/run/attempt·정책 변경·누락된 리뷰·불완전한
  인벤토리를 거부하는 통합 테스트와 인증된 정상 경로를 검증한다.

완료 조건: 신뢰된 생산 경로와 소비 경계가 구현·검증되고, 임의 입력의 형식 일치만으로
authoritative가 되지 않는다. 실제 배포·원격 관측 완료는 15.11에서 별도로 기록한다.

### 15.10 — 호환성 매트릭스 및 회귀·네이티브 인수검증

대상: `acceptance/src/`, `acceptance/tests/`, `bootstrap/tests/`,
`crates/project-verification/tests/`, `xtask/`, `.github/workflows/`, 실행 기록.

- [ ] 선택 계약에서 CI 매트릭스와 runtime provisioning을 도출하고, 최소 Rust 및
  추가 지원 compiler, 기존 프리셋 및 15.06의 대체 소비 조합을 검증한다.
- [ ] 기존 3건의 회귀 테스트, 구버전 receipt, 새 계약의 schema/Rust 대조,
  버전·설정·lockfile·명령·head 변경 시 증거 거부를 같은 후보에서 실행한다.
- [ ] React·.NET·Fullstack의 정상·실패·빈/누락 보고서·수정 후 재검증과 새 감사/
  사용자 프로필 시나리오를 실제 도구로 실행한다. 제공자·도구·경로별 실패 attribution을 남긴다.
- [ ] 지원 조합별 parser contract, 실제 producer/compiler/runtime 버전, 후보 SHA,
  dirty 여부, profile/policy digest, 원본 보고서, 실행 로그를 함께 보존한다.
- [ ] fmt/check/Clippy/workspace tests, schema 생성 차이, deny/audit, residue/public API,
  빈 target build 및 변경된 권한 판정 범위의 mutation을 실행한다.
  기존 다섯 mutation 대상은 유지하고 새 필수 knowledge 등 추가 권한 경로도 검증 범위에 넣는다.
- [ ] ignored·skipped·unviable·네트워크 실패 및 미지원 조합을 통과 수에 합치지 않는다.
  과거 후보의 mutation·CI 결과를 새 후보의 결과로 재사용하지 않는다.

완료 조건: 선언된 지원표에 실제 양성·음성 실행 증거가 있고, 새 후보의 자동검증 결과가
원본 산출물과 연결된다. 네이티브 인수검증의 성공만으로 authoritative 완료라고 하지 않는다.

### 15.11 — 실제 소비 프로젝트 및 운영 승격 증거

대상: 실제 소비 프로젝트, 신뢰된 CI controller 배포, required-check 관측,
`docs/promotion.md`의 프로토콜, 새 날짜의 실행·검토 기록.

- [ ] 실제 대상 repository·branch·후보·정책 및 현재 GitHub 권한/보호 기능을 확인한다.
  과거의 401/403 기록을 현재 기능 제한으로 단정하지 않는다.
- [ ] 선택한 후보와 정책에 대해 인증된 수집·독립 리뷰·exact-head required check·
  보호된 merge·post-merge 판정까지 실제 관측을 연결한다. READY와 SHIPPED를 구분한다.
- [ ] 서로 및 모든 수정자로부터 독립인 두 provider의 리뷰와 필요한 owner 승인을
  후보에 결합한다. 단순 엔지니어링 검토나 동일 provider 보조 검토로 대체하지 않는다.
- [ ] 현행 프로토콜의 최소 14일·30개 서로 다른 head shadow 관측과 필수 사례,
  설명된 불일치, 실제 보호 설정 baseline 및 live rollback 관측을 수집한다.
  프로토콜에 명시된 후보/기준 승인 조건을 지킨다.
- [ ] 계정 기능·외부 신원·배포 환경·운영 승인이 없으면 정확한 미완료 조건을 기록한다.
  Task 15 목록 작성이나 로컬 구현 완료를 보호 설정 변경·과금 변경·원격 게시·승격의
  포괄적 승인으로 사용하지 않는다.

완료 조건: 요구되는 실제 실행 기록이 후보·정책·head에 결합되어 있다. 외부 조건이
충족되지 않으면 구현/로컬 검증 상태는 따로 표시하고 이 항목 및 Task 15 전체는 미완료로 유지한다.

### 15.12 — 최종 문서·추적성 및 인수 기록

대상: `README.md`, `README_KO.md`, `docs/status.md`, `docs/tasks/task-15/README.md`,
Task 03·08·09·10·12·13·14 문서, `docs/integrations/`, `docs/tasks/README.md`,
`lat.md/`, 새 날짜의 설계·실행·검토 기록.

- [ ] 지원하는 Rust/도구 조합, 미지원 조합, 바이너리 실행과 소스 빌드 요구,
  사용자 프로필·선택값·새 receipt·마이그레이션 절차를 실제 동작에 맞춰 문서화한다.
- [ ] 기본 프리셋, 역사적 자산, 선택 가능한 도구, 명시적으로 필수인 정책 증거를
  구분하고 특정 개발 프레임워크가 kernel 실행에 필요하다는 인상을 제거한다.
- [ ] 구현 완료·로컬 검증·CI 관측·운영 증거를 구분해 Task별 상태와 남은 조건을
  갱신한다. 해결된 항목은 변경 커밋과 테스트/실행 기록으로 연결한다.
- [ ] 이 날짜의 계획과 기존 검토 기록은 보존한다. 이후 진행 상태는 Task landing page,
  `docs/status.md`와 새 기록에 남긴다. 실행 명령과 종료 코드 및 Markdown 상대 링크,
  LAT 링크, `git diff --check`를 검증한다.

완료 조건: 다른 작업자가 대화 기록 없이 지원 범위, 남은 일, 결과의 근거 및 운영 한계를 찾을 수 있다.

## 검증 명령과 완료 판정

다음은 기준 커밋에 이미 존재하는 Rust 검증 명령이다. 새 옵션·CLI·어댑터 명령은 각
작업에서 정의·구현·문서화한 뒤 추가한다. 아래 명령 목록 자체는 실행 결과가 아니다.

```console
rtk cargo test --locked --offline -p policy-bootstrap -p policy-acceptance -p project-verification
rtk cargo test --locked --offline -p policy-contracts -p policy-core -p policy-ci -p github-adapter -p evidence-collector -p xtask
rtk cargo fmt --all --check
rtk cargo check --workspace --locked --offline
rtk cargo clippy --workspace --all-targets --locked --offline -- -D warnings
rtk cargo test --workspace --locked --offline
rtk cargo run --locked --offline -p xtask -- schema
rtk git diff --exit-code -- schemas
rtk git ls-files --others --exclude-standard -- schemas
rtk cargo deny check
rtk cargo audit
rtk cargo run --locked --offline -p xtask -- check-residue
rtk cargo doc --no-deps --locked --offline -p policy-core -p policy-contracts
rtk cargo run --locked --offline -p xtask -- check-public-api target/doc schemas
rtk git diff --check
```

오프라인 명령에는 선택한 버전의 의존성 캐시가 먼저 필요하다. 새 버전 호환성 시험의
환경 준비와 실행 실패를 구분한다. schema 검증은 tracked diff가 없고 새 untracked
schema 목록도 비어 있어야 한다. audit, 실제 네이티브 도구 및 live CI 검증은 필요한
외부 서비스와 인증을 갖춘 환경에서 별도로 실행한다.

최종 인수 체크리스트:

- [ ] 구버전 안내의 모순과 수정 전 프로필로의 기본 진입이 해소됨.
- [ ] 세 개발 프레임워크 없이 지원 제품 경로가 동작하고 실제 필수 결합이 제거됨.
- [ ] Rust 및 소비 도구의 복수 호환 조합이 검증되고 실제 실행 조합은 정확히 기록됨.
- [ ] 사용자 프로필·새 자산 릴리스가 기존 receipt 및 정책 검증을 훼손하지 않음.
- [ ] 모든 대상 의존성 그래프가 감사되고 명시적 필수 knowledge가 강제됨.
- [ ] 인증된 controller와 CI 판정이 실제 소비 프로젝트의 같은 head로 연결됨.
- [ ] 기존 결함 회귀 및 새 호환성·신뢰 경계 시험의 후보별 증거가 보존됨.
- [ ] 프로모션에 필요한 외부 관측·승인·shadow·merge·rollback 증거가 충족됨.
- [ ] 현재 상태 문서가 각 항목의 근거와 한계를 정확히 반영함.
