# External Integrations

**Analysis Date:** 2026-09-09

## APIs & External Services

- GitHub API: `crates/github-adapter/src/gh.rs`의 `GhCliApi`가 gh 프로세스로 접근.
- `crates/github-adapter/src/parse.rs`: PR, work item, agent-ready, merge, trusted check 관측 검증.
- Check Run 발행과 receipt binding은 adapter/CI 경계에 있다.
- GitHub 접근 성공을 core가 직접 추측하지 않는다.

## Data Storage

- JSON 입력과 파일 artifact 중심이며 DB client는 workspace에 없다.
- `crates/evidence-collector/src/artifacts.rs`: 수집 artifact.
- `crates/policy-ci/src/main.rs`: trusted 파일 입력과 결과 artifact 기록.
- `tests/fixtures/`는 테스트 데이터이며 운영 증거가 아니다.

## Authentication & Identity

- GitHub 자격 증명은 외부 gh/CI 환경이 제공한다.
- `crates/policy-ci/src/main.rs`가 런타임 환경과 trusted observation을 연결한다.
- repository/head/workflow/run/attempt 결합을 검증한다.
- 인증 파일과 값은 이 매핑에서 읽거나 기록하지 않았다.

## Monitoring & Observability

- CLI JSON 결과와 stderr 진단, CI verification 로그.
- `.github/workflows/ci.yml`에서 verification artifact 업로드.
- 별도 SaaS 분석/모니터링 통합은 조사한 workspace에 없다.

## CI/CD & Deployment

- GitHub Actions: `.github/workflows/ci.yml`, `.github/workflows/promotion.yml`.
- 재사용 Action: `action/action.yml`, `action/README.md`.
- `.github/workflows/task-10-live-smoke.yml`는 실제 publication 경로 확인용.
- 기존 smoke 성공이 READY_TO_MERGE/SHIPPED의 운영 증거를 대신하지 않는다.

## Environment Configuration

- 로컬 `policy-cli`는 advisory 평가만 제공.
- authoritative 입력 경계: `docs/trust-model.md`.
- GSD는 외부 개발 환경; `docs/integrations/gsd-core.md`에 설치 기록.
- OpenCode/Pi는 선택형이며 현재 repo 필수 의존성이 아니다.

## Webhooks & Callbacks

- HTTP webhook 서버 대신 Actions/CLI에서 GitHub 관측을 전달한다.
- merge 승인 조건은 `crates/policy-core/src/post_merge.rs`에 있다.

*Integration audit: 2026-09-09*
