# policy-kernel

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

이 워크스페이스에는 Task 1–14의 구성요소가 구현되어 있습니다. 근거와 남은 구현·통합·운영 조건은
[현재 상태](docs/status.md)와 [Task 색인](docs/tasks/README.md)에서 확인할 수 있습니다.
[Task 15](docs/tasks/task-15/README.md)는 호환 도구·버전 선택, 실행 독립성,
의존성·knowledge 증거의 누락 및 인증된 소비 프로젝트 통합을 다루는 추가 작업 계획입니다.
Task 15의 구현은 아직 시작하지 않았습니다.
이것이 운영 환경 프로모션 완료를 뜻하지는 않습니다. [프로모션 프로토콜](docs/promotion.md)에
정의된 실제 shadow 기간, 보호 브랜치 병합 증거, 독립 제공자 검토는 별도로 충족해야 합니다.

프로젝트는 Rust 우선 정책을 따릅니다. 저장소의 실행 로직, CLI, 검증 도구, 보고서 변환,
bootstrap 설치기, 테스트는 Rust로 작성합니다. JavaScript/TypeScript와 C#은 `examples/`
아래에서 승인된 네이티브 소비 프로젝트 fixture 및 선언형 도구 설정에만 사용합니다.

## 요구 사항

- [rust-toolchain.toml](rust-toolchain.toml)에 고정된 `rustfmt`, `clippy` 포함 Rust **1.95.0**
- CI 어댑터와 소비 프로젝트 인수 검증에 필요한 Git
- 네이티브 소비 프로젝트 프로필을 실행할 때만 Node 22.18.0 / npm 10.9.3 및 .NET SDK 9.0.304.
  자세한 내용은 [소비 프로젝트 인수 검증](docs/integrations/acceptance.md)을 참고하세요.

## 시작하기

저장소를 받은 뒤, 잠긴(lockfile 고정) 워크스페이스를 빌드하고 테스트합니다.

```console
cargo build --locked --workspace
cargo test --locked --workspace
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
```

워크스페이스는 정확한 의존성 버전과 `Cargo.lock`을 관리합니다. 재현 가능한 빌드와 검증을
위해 `--locked`를 사용하세요.

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
| `policy-core` | 결정론적 정책·출처·검토·센서·병합 평가 |
| `policy-cli` | 로컬 자문 병합 전 평가와 타입 객체 검사 |
| `policy-ci` | 권한 있는 GitHub Actions 평가 및 전용 검사 게시 |
| `evidence-collector` | 명시된 argv 실행을 제한하고 아티팩트 digest를 수집 |
| `github-adapter` | CI 통합에 사용하는 GitHub 관측 및 프로세스 어댑터 |
| `project-verification` | 프로젝트 프로필 검증·준비·실행·보고서 정규화 |
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
cargo run --locked -p xtask -- check-mutation-results verification/mutants.out/outcomes.json
cargo test --locked -p xtask
```

`xtask rollback-drill`은 호출자가 제공한 정규화 snapshot만 비교합니다. GitHub를 조회하거나
변경하지 않습니다. 필수 인수와 실제 롤백 절차는
[프로모션 및 롤백](docs/promotion.md)에 설명되어 있습니다.

## 프로젝트 검증과 bootstrap

`project-verification`은 채택된 프로젝트 프로필과 검증된 소스 인벤토리로부터 로컬 관측값을
만듭니다. React/TypeScript, .NET, 풀스택의 세 자산을 지원하며 Vitest JSON, Playwright JSON,
VSTest TRX 보고서를 정규화합니다. 이 결과는 로컬 자문용입니다. 권한 있는 결정을 뒷받침하려면
소비 프로젝트가 신뢰할 수 있는 CI 경로에 결과를 바인딩해야 합니다.

```console
cargo run --locked -p project-verification -- validate-profile profiles/fullstack/profile.json
cargo run --locked -p project-verification -- schema
cargo run --locked -p project-verification -- prepare --input request.json
cargo run --locked -p project-verification -- run --input request.json
```

`policy-bootstrap`은 검토된 고정 설정을 기존 소비 프로젝트 디렉터리에 설치합니다. 적용 전
계획을 검사하고 저장하세요. `apply`는 파일을 변경하기 전에 계획을 다시 검증합니다.

```console
cargo run --locked -p policy-bootstrap -- inspect --target /absolute/project --profile fullstack --release 1.0.2 --component frontend=apps/web --component backend=apps/api
cargo run --locked -p policy-bootstrap -- plan --target /absolute/project --profile fullstack --release 1.0.2 --component frontend=apps/web --component backend=apps/api > /absolute/bootstrap-plan.json
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

## CI 통합

체크인된 복합 [GitHub Action](action/README.md)은 `PATH`에 이미 구축되어 신뢰할 수 있는
`policy-ci` 바이너리를 실행합니다. runner가 제어하는 환경에서 정확한 GitHub workflow와
HEAD 신원을 검증하고 `policy-kernel/pre-merge` 검사를 게시합니다. 기여자가 제어하는
checkout에서 실행하거나 로컬 `policy-cli` 출력을 필수 검사로 취급해서는 안 됩니다.

`policy-ci`에는 `pre-merge`, `post-merge` 명령이 있고 authoritative 모드만 허용합니다.
필요한 runner 제어 관측값과 CLI 인수는 action 문서에 설명되어 있습니다.

## 추가 문서

- [아키텍처 명세](docs/archive/specs/2026-09-04-policy-kernel-extraction-design.md)
- [신뢰 모델](docs/trust-model.md)
- [프로모션 및 롤백 프로토콜](docs/promotion.md)
- [프로젝트 검증 및 bootstrap 설계](docs/archive/specs/2026-09-07-project-verification-bootstrap-design.md)
- [현재 상태](docs/status.md)
- [Task 색인](docs/tasks/README.md)
- [저장소 CI workflow](.github/workflows/)
