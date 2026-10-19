# policy-kernel

Core 평가를 통과하려면 adequacy 정책이 필요하다. 빈 정책은
`PK-ADEQ-POLICY-EMPTY` 사유를 내며, 소비자 profile schema 3/4는 실행 전에
거부한다. Negative ID는 `changed_behavior_ids`에도 포함하고 실행된 test ID에
매핑해야 한다. Frozen profile의 파싱은 유지하지만 빈 placeholder로 준비 완료를
입증할 수 없다. [Adequacy 계약과 호환성 변경](docs/integrations/adequacy.md)을 참조한다.

[npm workspace 공유 lock 확장](docs/integrations/npm-workspaces.md)은 schema 4 /
selection 3의 단일 root graph에 명시적 member를 추가합니다. Root audit 한 번으로
root와 member를 감사하며 member별 lock은 만들지 않습니다. Node 26.12.0/npm 11.20.0
네이티브 fixture 결과는 **로컬 범위 충족(local scope satisfied)** 이며 TJERP 도입 완료나
Trusted 증거가 아닙니다. [현재 상태](docs/status.md)를 참고하세요.

Root npm override는 graph의 `workspace.overrides`와 `package.json.overrides`에 동일한 검토된
객체를 선언하여 사용합니다. 정확한 stable 버전과 package scope 한 단계만
지원하며 contract 대상, member override, 적용되지 않는 규칙과 lock 불일치는
거부합니다. 고정 toolchain의 네이티브 fixture는 취약/수정 override 버전을 단일
root audit으로 검증합니다. [제한과 검증 명령](docs/integrations/npm-workspaces.md#reviewed-root-overrides)을 참고하세요.

서드파티 `bundleDependencies`는 lock의 부모 package에 HTTPS npm registry tarball과
sha512 integrity가 있어야 허용합니다. Audit configuration은 각 항목을
`bundled, not individually audited`로 기록하며, 호스트에 설치되지 않은 optional
플랫폼 부모도 포함합니다. Root/member bundle은 계속 거부합니다.
[Bundle 계약](docs/integrations/npm-workspaces.md#third-party-bundled-dependencies)을 참고하세요.

증거(evidence)를 릴리스 정책과 대조해 결정하는 결정론적 Rust 구성 요소 모음입니다.
이 커널은 에이전트 실행 환경이나 모델 제공자에 의존하지 않습니다. 타입이 정해진 입력을
평가해 타입이 정해진 결정을 만들고, 인증과 릴리스 권한은 명시적인 통합 경계에서만
다룹니다.

[English README](README.md)

## 제공하는 기능

- 버전이 관리되는 정책·증거·신원·결정 계약과 JSON Schema
- `policy-core`의 결정론적 병합 전·병합 후 정책 평가
- 로컬 자문용 CLI와 GitHub Actions 권한 평가 경계인 `policy-ci`
- 제한된 증거 수집, 프로젝트 프로필 검증, 보고서 정규화, 트랜잭션 bootstrap 설치기
- React/TypeScript, .NET, 풀스택 소비 프로젝트를 위한 Rust 기반 인수 검증 fixture

`ReadyToMerge`는 제공된 증거에 대한 평가 결과일 뿐, 배포 권한이 아닙니다. 릴리스에는
인증된 CI 관측값, 정확히 같은 HEAD의 필수 검사, 보호 브랜치 병합 증거, 소유자 승인이
추가로 필요합니다. 릴리스 용도로 결과를 사용하기 전
[신뢰 모델](docs/trust-model.md)을 읽어야 합니다.

## 현재 상태와 범위

[범위 결정(Task 18)](docs/tasks/task-18/README.md)은 제품을 결정론적
evidence·scope·release 판정으로 좁힙니다. 새로운 exact native 도구 조합과
bootstrap preset 추가는 중단했습니다. [Native 계약](docs/integrations/native-contracts-v1.md)은
v1 타입과 adapter 측 원본·subject importer를 제공합니다. scope 판정과 core evidence 변환은 아직 계획 단계입니다. 기존 profile·CLI 경로·bootstrap 복구와 필수 환경 검사는
유지하고, TJERP 실행은 변경하지 않습니다.

[P7 TJERP 인수 harness](docs/integrations/tjerp-adoption.md)는 기존 Rust API로
두 로컬 profile, 승인 catalog와 원본 test identity를 결합합니다. FI-00–FI-09는
합성 시험입니다. TJERP는 2026-10-17 P8 profile·실행 절차와 P9 실제 dry-run을
완료했습니다. FI-00과 회복은 **local scope satisfied**이며 FI-01–FI-09는
의도대로 거부되었습니다. 증거는 LocalUntrusted이고 review gate는 미충족이며,
pre-merge 판정이나 production promotion은 없습니다.
[현재 상태와 TJERP revision 근거](docs/status.md#tjerp-p8-adoption-and-p9-first-actual-dry-run)를 참고하세요.
`cargo test --locked --offline -p policy-acceptance --test tjerp_adoption`으로 검증합니다.

네이티브 fixture는 Jest 30.5.2와 범위를 제한한 `js-yaml` 4 override를 사용합니다.
React/TypeScript는 parser 8.42.0 및 동결된 bootstrap 버전 고정을 유지하며,
`typescript-estree` 8.48.0에만 scoped override를 적용합니다.
자세한 내용은 [현재 상태](docs/status.md)를 참고하세요.

이 워크스페이스에는 Task 1–14의 구성요소가 구현되어 있습니다. 근거와 남은 구현·통합·운영 조건은
[현재 상태](docs/status.md)와 [Task 색인](docs/tasks/README.md)에서 확인할 수 있습니다.
[Task 15](docs/tasks/task-15/README.md)는 호환 도구·버전 선택, 실행 독립성,
의존성·knowledge 증거의 누락 및 인증된 소비 프로젝트 통합을 추적합니다.
완료된 수정과 남은 작업은 해당 Task 랜딩 페이지와 현재 상태 문서에서 확인하세요.
15.10까지의 로컬 구현·검증은 완료했습니다. 게시된 후보 `751fa76`은 실제 GitHub에서
Release 검증, Rust 1.85/1.95 × Ubuntu/macOS 매트릭스와 네 가지 네이티브 소비 조합을
통과했습니다. 근거는 [원격 실행 기록](docs/archive/reviews/2026-09-16-task-15-hosted-verification.md)에
있습니다. 같은 후보의 원격 mutation은 당시 정책에서 통과했습니다(448개 검출,
13개 실행 불가, 미검출·시간 초과 0개). 이 결과는 채택한 모든 변형의 빌드와 검출을
요구하는 강화된 정책에서는 실패합니다. 정확한 생성·대체 계약과 이후 로컬 실행은
[mutation 후속 기록](docs/archive/reviews/2026-09-21-task-15-strict-mutation.md)에
구분해 기록합니다. 새 로컬 전체 실행에서는 원본 선택 449개와 대체 변형 12개를 모두
검출했고, 실행 불가·미검출·시간 초과는 0개이며 baseline 5회가 성공했습니다.
교체한 원래 생성식 12개 자체를 검출 성공으로 집계하지 않습니다. 측정 당시 dirty였던
변경은 이후 `1d48a92`로 커밋됐고 `c79571e`에서 원격 main에 반영됐습니다.
새 원격 검증 결과는 여기에 기록하지 않았습니다.
15.11의 운영 증거와 소유자 승인은 남아 있습니다.
이것이 운영 환경 프로모션 완료를 뜻하지는 않습니다. [프로모션 프로토콜](docs/promotion.md)에
정의된 실제 shadow 기간, 보호 브랜치 병합 증거, 독립 제공자 검토는 별도로 충족해야 합니다.

2026-09-30 [코드베이스 리뷰 보완](docs/archive/reviews/2026-09-30-codebase-review-remediation.md)은
커널 판정을 강화합니다. 센서 not-applicable 예외는 변경된 모든 경로를 덮어야 하고,
모든 조치 대상 finding이 유효하게 waiver된 blocked 리뷰는 승인 리뷰와 충돌로 보지 않으며,
사유 없이 준비 상태를 보류한 gate가 준비 상태를 유지할 수 없습니다. controller 서명 바이트는
명시적 canonical JSON에서 만듭니다. 이 기록은 로컬 검증만 담고 있으며 원격 결과나 프로모션을
주장하지 않습니다. 후속 [미검토 영역 리뷰](docs/archive/reviews/2026-09-30-remaining-area-review.md)는
bootstrap이 설치하는 파일이 umask를 따르도록 하고 엄격 mutation 검사 범위를 센서 판정까지 넓혔으며,
이 역시 로컬 검증만 기록합니다. [설계 결정 변경](docs/archive/reviews/2026-09-30-design-decisions.md)은
configuration 바인딩이 checkout 위치에 좌우되지 않게 하고, authoritative 판정에서 execution·adequacy
증거의 head 바인딩을 요구하며, merge 보호 여부를 GitHub 브랜치 관측과 대조하고, 경로 glob의 `*`가
한 경로 단계 안에서만 매칭되게 합니다(디렉터리를 넘으려면 `**`를 씁니다).

센서와 knowledge 경로 필터는 [GitHub 호환 부분집합](docs/trust-model.md)만 허용합니다:
ASCII 영숫자와 `.`, `_`, `-`, `/`, 한 경로 단계의 `*`, 단계 전체를 차지하는 `**`입니다.
그 밖의 문법은 fail-closed로 거부합니다. `src/*.jsx?` 대신 `src/*.js`와 `src/*.jsx`를
별도 항목으로 사용해야 하며, 부정 패턴과 순서에 따른 제외는 지원하지 않습니다.

프로젝트는 Rust 우선 정책을 따릅니다. 저장소의 실행 로직, CLI, 검증 도구, 보고서 변환,
bootstrap 설치기, 테스트는 Rust로 작성합니다. JavaScript/TypeScript와 C#은 `examples/`
아래에서 승인된 네이티브 소비 프로젝트 fixture 및 선언형 도구 설정에만 사용합니다.

## 요구 사항

Schema 3 소비자는 검토한 NuGet 프로젝트에
[P2 생성 파일 계약](docs/integrations/consumer-schema3.md#compiler-generated-files-p2)을
명시적으로 적용할 수 있습니다. `EmitCompilerGeneratedFiles=true`와 프로젝트 기준
`obj/generated` 조합만 허용하며 명령, 상속 및 symlink를 검사합니다. 로컬 검증은
advisory이며 증거를 Trusted로 승격하지 않습니다.

프런트엔드 프로젝트는 [프로젝트 소유 도구 버전](docs/integrations/tool-selection.md#project-owned-frontend-versions-schema-4) 경로를 선택할 수 있습니다. 프로필 schema 4 / selection schema 3은 버전 수용 규칙과 Vitest·Jest·Playwright·process·npm 감사 계약을 분리합니다. 설치 버전 선택은 선택 사항이지만 실제 도구 관측과 선언한 runtime·lock 검사는 필수입니다. `migrate-profile FILE --target-schema 4`는 원본을 바꾸지 않고 후보를 출력합니다. 사용자 프로필 전용 bootstrap 1.3.0은 네이티브 파일을 사용자 소유로 둡니다. [native 적합성 안내](docs/integrations/acceptance.md#project-owned-frontend-conformance)에는 소비자 후보 입력과 원본 보고서 보존 절차가 있습니다. 검증 범위와 특정 커밋의 hosted 실행 결과는 [Task 17](docs/tasks/task-17/README.md)에 기록하며, 운영 승격은 별도입니다.

명시적 [schema 3 소비 프로필](docs/integrations/consumer-schema3.md)은 선택된
.NET 11 RC1/VSTest 17.14.1 및 Playwright 1.63.0 조합, 프레임워크 설정 상속,
통합 테스트 프로젝트를 지원합니다. Schema 3에는 사용자 프로필 전용 bootstrap
1.2.0을 사용하며 기존 1.1.0 프리셋은 유지합니다. 로컬 검증 근거와 남은 적용 조건은
[Task 16](docs/tasks/task-16/README.md)에 기록합니다.

- 소스 빌드에는 Rust **1.85.0** 이상을 요구합니다.
  [rust-toolchain.toml](rust-toolchain.toml)은 재현 가능한 개발 및 릴리스 검증의
  기본값으로 `rustfmt`, `clippy`를 포함한 **1.95.0**을 유지합니다. 자세한 내용은
  [Rust 및 의존성 호환성](docs/integrations/rust-compatibility.md)을 참고하세요.
- 검증한 `policy-cli`, collector, project-verification, bootstrap 바이너리
  실행 경로에는 Rust나 에이전트 프레임워크 설치가 필요하지 않습니다.
  호환 OS·시스템 런타임과 호출자가 선택한 외부 도구는 필요합니다.
  `xtask check-residue` 같은 저장소 검증 명령은 Cargo와 선택된 Rust 도구가 필요합니다.
- CI 어댑터와 소비 프로젝트 인수 검증에 필요한 Git
- 네이티브 도구는 채택된 [선택 계약](docs/integrations/tool-selection.md)을 따릅니다.
  내장 프로필의 설치 선택은 Node 22.18.0 / npm 10.9.3 및 .NET SDK 9.0.304이며,
  추가 React/Jest 소비 조합은 Node 22.19.0 / npm 10.9.4를 선택합니다.
  요청 호환 범위, 정확한 설치 선택, 실제 관측 버전은 각각 기록합니다.
- 인증 컨트롤러와 암호 검증 테스트에는 OpenSSL이 필요합니다. 소유자가 관리할 신뢰 기준은
  [컨트롤러 참조 구현](action/controller.md)을 참고하세요.

## 시작하기

저장소를 받은 뒤, 잠긴(lockfile 고정) 워크스페이스를 빌드하고 테스트합니다.

```console
cargo build --locked --workspace
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

manifest는 검토된 호환 의존성 범위를 사용하고, 체크인된 `Cargo.lock`은 정확히 테스트된
해결 결과를 기록합니다. 재현 가능한 빌드와 검증에는 `--locked`를 사용하세요. 새로 해결
가능해진 호환 버전이 자동으로 테스트된 버전이 되는 것은 아닙니다.

### 로컬 자문 평가 실행

`policy-cli`는 JSON 파일 또는 표준 입력(`-`)을 읽습니다. 입력 JSON이 authoritative 모드를
요청해도 병합 전 평가는 항상 advisory 모드로 수행합니다.

```console
cargo run --locked -p policy-cli -- evaluate-pre tests/fixtures/pre-merge/ready.json
cargo run --locked -p policy-cli -- inspect tests/fixtures/evidence/valid.json
cat decision.json | cargo run --locked -p policy-cli -- inspect -
```

`evaluate-pre`의 종료 코드는 `ReadyToMerge`이면 0, `Blocked`이면 2,
`Inconclusive`이면 3, 잘못된 입력이면 64입니다. `inspect`는 타입이 정해진 병합 전/후
결정 또는 증거 묶음을 검증해 출력하지만, 제공된 결정을 재계산하거나 인증하지는 않습니다.

## 워크스페이스 구성

| 패키지 | 역할 |
| --- | --- |
| `policy-contracts` | 엄격하고 버전이 관리되는 전송 타입 및 스키마 원본 타입 |
| `policy-native-contracts` | Native v1 전송 타입·구조 검증·스키마·적합성 corpus; importer 없음 |
| `policy-core` | 결정론적 정책·출처·검토·센서·병합 평가 |
| `policy-cli` | 로컬 자문 병합 전 평가와 타입 객체 검사 |
| `policy-ci` | 권한 있는 GitHub Actions 평가 및 전용 검사 게시 |
| `evidence-collector` | 명시된 argv 실행을 제한하고 아티팩트 digest를 수집 |
| `github-adapter` | CI 통합에 사용하는 GitHub 관측 및 프로세스 어댑터 |
| `project-verification` | 프로젝트 프로필 검증·준비·실행·보고서 정규화 |
| `policy-native-importer` | Unix adapter 라이브러리: 원본·subject 검증, 공용 parser 재계산과 실패 자료 보존; scope 판정 없음 |
| `policy-report-adapters` | 공용 TRX·Vitest·Jest·Playwright reader; `project_verification::reports`에서 재노출 |
| `policy-bootstrap` | 프로필의 트랜잭션 설치·복구·제거 |
| `policy-acceptance` | 네이티브 소비 프로젝트 fixture를 stage하고 실행하는 Rust harness |
| `xtask` | 스키마 생성 및 저장소·릴리스 검증 명령 |

## 스키마와 저장소 검증

다음 명령은 [`schemas/`](schemas/)에 체크인된 다섯 계약 스키마를 생성합니다.

```console
cargo run --locked -p xtask -- schema
```

이 명령은 워크스페이스 출력 디렉터리를 자체적으로 찾으므로, 저장소 밖에서도 Cargo의
`--manifest-path`로 실행할 수 있습니다. 생성 스키마, 공개 문서 표면, 소스 언어 잔여물,
제공된 mutation 결과를 다음과 같이 검증합니다.

```console
cargo run --locked -p xtask -- check-residue
cargo run --locked -p xtask -- check-public-api target/doc schemas
cargo fetch --locked
cargo run --locked --offline -p xtask -- run-mutation-suite target/promotion-mutation
cargo run --locked --offline -p xtask -- check-mutation-suite target/promotion-mutation
cargo test --locked -p xtask
```

전체 실행은 core 권한 판정 모듈 다섯 개, controller, workflow-source를 포함합니다.
실행 불가·미검출·시간 초과·불완전 결과를 거부하고, 필터 전 인벤토리를 모든 원본 및
타입이 유효한 대체 변형의 실행과 대조합니다. 매번 새 출력 디렉터리를 사용하세요.
개별 인벤토리용 `check-mutation-results OUTCOMES [--scope core|controller|workflow-source]`의
기본 범위는 core이며, 이 검사만으로 전체 실행 완료를 판정할 수 없습니다.
자세한 내용은 [mutation 계약과 명령](docs/promotion.md#mutation-generation-and-execution-contract)을 참고하세요.

`xtask rollback-drill`은 호출자가 제공한 정규화 snapshot만 비교합니다. GitHub를 조회하거나
변경하지 않습니다. 필수 인수와 실제 롤백 절차는
[프로모션 및 롤백](docs/promotion.md)에 설명되어 있습니다.

## 프로젝트 검증과 bootstrap

`project-verification`은 채택된 프로젝트 프로필과 검증된 소스 인벤토리로부터 로컬 관측값을
만듭니다. React/TypeScript, .NET, 풀스택 자산과 사용자 프로필을 지원하며 Vitest JSON,
Jest JSON, Playwright JSON, VSTest TRX 보고서를 정규화합니다. 이 결과는 로컬 자문용입니다. 권한 있는 결정을 뒷받침하려면
소비 프로젝트가 신뢰할 수 있는 CI 경로에 결과를 바인딩해야 합니다.

스키마 2의 의존성 감사는 선택한 그래프를 각각 검사합니다. npm 설치와 감사는
dev·optional·peer 의존성을 명시적으로 포함하여 npm 설정으로 누락되지 않도록 합니다.
네이티브 인수검증은 Vitest·TRX·Playwright 프로세스가 성공해도 빈 보고서나 누락된
보고서를 거부하고, 수정 후 새 head 검증과 이전 head 결과 거부를 확인합니다.

```console
cargo run --locked -p project-verification -- validate-profile bootstrap/templates/1.1.0/profiles/fullstack/profile.json
cargo run --locked -p project-verification -- schema
cargo run --locked -p project-verification -- prepare --input request.json
cargo run --locked -p project-verification -- run --input request.json
```

저장소 루트의 `profiles/fullstack/profile.json`은 센서 14개인 1.0.0 과거
자산으로 유지됩니다. 동결된 릴리스 1.0.2에는 `shared:dependencies`를 포함한
필수 센서 15개가 있습니다. 현재 1.1.0은 프로필 schema 2를 사용하며 Unit·Integration
NuGet 감사를 각각 추가해 필수 센서가 17개입니다. Bootstrap 설치
후에는 선택된 `.policy/project-profile.json`을 검증하세요. 과거 자산과
현재 자산의 구분은 [프로필 자산 안내](profiles/README.md)를 참고하세요.

`policy-bootstrap`은 검토된 선택 설정을 기존 소비 프로젝트 디렉터리에 설치합니다. 적용 전
계획을 검사하고 저장하세요. `apply`는 파일을 변경하기 전에 계획을 다시 검증합니다.

```console
cargo run --locked -p policy-bootstrap -- inspect --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api
cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.1.0 --component frontend=apps/web --component backend=apps/api > /absolute/bootstrap-plan.json
cargo run --locked -p policy-bootstrap -- apply --plan /absolute/bootstrap-plan.json
```

`policy-acceptance`는 새 출력 디렉터리를 사용해야 합니다. `--provision`은 stage된 fixture에서
잠긴 의존성을 복원하고 브라우저를 준비하도록 허용합니다.

```console
cargo run --locked -p policy-acceptance -- run --profile fullstack --output /private/tmp/policy-acceptance-fullstack --provision
```

프로젝트에 이 도구를 적용하기 전 [프로젝트 검증](docs/integrations/project-verification.md),
[bootstrap](docs/integrations/bootstrap.md), [보고서 형식](docs/integrations/report-formats.md),
[소비 프로젝트 인수 검증](docs/integrations/acceptance.md)을 읽어 보세요.

## 로컬 코드 그래프

이 저장소는 Rust 소스 구조와 영향 분석을 위한 로컬 개발 보조 도구로
[CodeGraph](https://github.com/colbymchenry/codegraph)를 사용합니다. 이는 CI 입력,
정책 증거 또는 릴리스 승인 수단이 아닙니다. 생성되는 `.codegraph/` SQLite 인덱스는
의도적으로 Git에서 제외합니다.

CLI를 한 번 설치하고 Codex에 연결한 다음, 이 저장소의 인덱스를 생성하거나 갱신합니다.

```console
npm install -g @colbymchenry/codegraph
codegraph install --target=codex --location=global --yes
codegraph init
codegraph status
```

MCP 설치 뒤에는 Codex를 재시작하세요. 인덱스를 사용할 수 있으면 구조 관련 질문에는
`codegraph_explore` MCP 도구를 사용하고, CLI에서는
`codegraph explore "How does pre-merge evaluation flow?"` 또는
`codegraph impact PolicySnapshot` 같은 명령을 사용할 수 있습니다.

인덱스는 CodeGraph MCP daemon이 실행 중일 때만 자동 동기화되며, `codegraph status`는
뒤처진 인덱스도 "up to date"로 표시할 수 있습니다. 소스를 수정한 뒤에는
`codegraph sync`를 실행하거나, 선택적으로 로컬 hook을 설치합니다.

```console
printf '#!/bin/sh\ncommand -v codegraph >/dev/null && codegraph sync -q || true\n' > .git/hooks/post-commit
chmod +x .git/hooks/post-commit
```

## 지식 그래프

[`lat.md/`](lat.md/lat.md)는 [lat.md](https://github.com/vercel-labs/lat.md) 그래프로,
문서를 서로 연결하고 [`lat.md/kernel.md`](lat.md/kernel.md)에서 권한 판정 규칙을 Rust
심볼에 고정합니다. CodeGraph와 마찬가지로 탐색 보조 수단이며 정책 증거나 현재 상태의
근거가 아닙니다. CLI를 설치하고 변경할 때마다 링크와 코드 참조를 검증합니다.

```console
npm install -g --ignore-scripts lat.md@0.12.2
lat check
```

Release verification workflow의 `lat.md knowledge graph` job이 같은 버전으로 `lat check`를
실행하므로, 고정된 심볼의 이름이 바뀌거나 삭제되면 CI가 실패합니다. 유지 규칙은
[AGENTS.md](AGENTS.md)에 있습니다.

## CI 통합

현재 게시기는 `READY_TO_MERGE`를 `success`로, `BLOCKED`와 `INCONCLUSIVE`를
모두 `failure`로 게시하며 제목과 출력의 구분은 유지합니다. 전용 필수 검사만으로
차단하려면 소유자가 설치한 바이너리를 업데이트하고 기존 neutral 검사를 다시
게시해야 합니다. [마이그레이션 안내](action/README.md#results)를 참고하세요.

체크인된 복합 [GitHub Action](action/README.md)은 소유자가 미리 빌드·설치한 `policy-ci`를
절대 경로인 `executable-path` 입력으로 실행합니다(기본값 `/opt/policy-kernel/bin/policy-ci`).
runner가 제어하는 환경에서 정확한 GitHub workflow와
HEAD 신원을 검증하고 `policy-kernel/pre-merge` 검사를 게시합니다. 기여자가 제어하는
checkout에서 실행하거나 로컬 `policy-cli` 출력을 필수 검사로 취급해서는 안 됩니다.

`policy-ci`에는 `pre-merge`, `post-merge` 명령이 있고 authoritative 모드만 허용합니다.
필요한 runner 제어 관측값과 CLI 인수는 action 문서에 설명되어 있습니다.
[인증 컨트롤러 참조 구현](action/controller.md)은 독립 관측자와 제공자의 서명을 검증한 뒤
receipt를 발행합니다. 로컬 암호 테스트가 실제 제공자 서비스 배포를 입증하지는 않습니다.
정책 schema 2는 필수 knowledge를 명시할 수 있으며, 선언이 없는 정책은 계속 빈 knowledge를
허용합니다.

이제 readiness에는 digest에 결합된 `required_reviews`를 갖춘 정책 schema 3이 필요합니다:
양수인 `minimum_reviewers`, `require_provider_independence: true`,
`allowed_providers`(빈 배열이면 모든 제공자 허용)를 명시합니다. Core는 서로 다른 승인
리뷰 실행만 정족수로 계산하며 waiver는 승인으로 계산하지 않습니다. V1/v2 문서의 바이트와
digest는 유지되지만, 새 digest 및 결합된 증거를 갖춘 v3로 명시적으로 재승인하기 전에는
`PK-POLICY-V3-MIGRATION-REQUIRED`로 fail closed 됩니다. 컨트롤러의
`minimum_reviewers`는 `max(2, 정책 minimum_reviewers)`와 같아야 합니다.
이는 소비 프로젝트 프로필의 자동 마이그레이션이나 운영 승인을 의미하지 않습니다.
[리뷰 요구사항 및 마이그레이션](docs/trust-model.md)을 참고하세요.

## 추가 문서

- [아키텍처 명세](docs/archive/specs/2026-09-04-policy-kernel-extraction-design.md)
- [신뢰 모델](docs/trust-model.md)
- [프로모션 및 롤백 프로토콜](docs/promotion.md)
- [프로젝트 검증 및 bootstrap 설계](docs/archive/specs/2026-09-07-project-verification-bootstrap-design.md)
- [현재 상태](docs/status.md)
- [Task 색인](docs/tasks/README.md)
- [저장소 CI workflow](.github/workflows/)
