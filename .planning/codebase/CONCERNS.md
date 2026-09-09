# Codebase Concerns

**Analysis Date:** 2026-09-09

## Tech Debt

- `docs/plans/2026-09-08-gsd-transition-and-tasks-12-14.md`의 Task 12–14는 미구현.
- 기존 리뷰의 과거 head/분기 상태를 현재 미통합 상태로 해석하지 않는다.
- Task 1–11 통합 근거: `docs/reviews/2026-09-08-main-integration.md`.

## Known Bugs

- 이번 작업은 정적 매핑이며 새 런타임 버그 재현은 수행하지 않았다.
- GSD installer의 미변환 Claude 참조 경고는 `docs/integrations/gsd-core.md`에 기록.
- 일부 fallback 안내가 Claude용이다. host workflow 호환성은 G3 확인 필요.

## Security Considerations

- 필수 지식 증거 요구가 `crates/policy-contracts/src/policy.rs`의 PolicySnapshot에 없다.
- 제출된 knowledge의 binding 검사와 필수 knowledge 누락 차단은 다른 문제다.
- Task 12A-K에서 정책 요구 및 core 연결을 추가하도록 계획돼 있다.
- trusted CI 수집, provider 독립성, 보호 merge가 없으면 릴리스 성공을 선언할 수 없다.

## Performance Bottlenecks

- 이 매핑에서 benchmark/프로파일링은 수행하지 않았다.
- collector/gh 경계의 timeout/output bounds를 변경할 때 실패 의미를 유지해야 한다.
- 근거 파일: `crates/evidence-collector/src/process.rs`, `crates/github-adapter/src/gh.rs`.

## Fragile Areas

- `crates/policy-core/src/canonical.rs`, `pre_merge_digest.rs`: bytes/digest 호환성.
- `crates/policy-core/src/post_merge.rs`: check/merge/head/policy 결합.
- schema와 fixture를 계약 타입 변경과 함께 조정해야 한다.

## Scaling Limits

- 운영 성능/수용량 수치는 미측정.
- bounded output/timeout은 non-success 보존 장치이며 throughput 보장이 아니다.

## Dependencies at Risk

- `docs/plans/2026-09-08-gsd-transition-and-tasks-12-14.md`는 deny.toml 및 의존성 정책 확정을 미완료로 기록.
- `.github/workflows/ci.yml`는 cargo deny/audit를 호출한다.
- 새로운 취약점 조회나 의존성 업그레이드는 이 매핑 범위에서 수행하지 않았다.

## Missing Critical Features

- 정책에 결합된 실행 정의/필수 report count, Rust normalizer/profile runner: Task 12.
- 안전한 inspect/plan/apply/upgrade/uninstall: Task 13.
- 실제 React/.NET/fullstack acceptance: Task 14.
- 기존 GitHub 403/보호 설정 관측은 과거 기록이며 현재 계정 상태는 재조회가 필요하다.

## Test Coverage Gaps

- 최종 후보 전체 mutation, 독립 provider review, shadow/rollback 운영 증거가 남아 있다.
- `docs/reviews/2026-09-08-live-promotion-readiness.md`의 smoke는 non-ready publication 확인이다.
- 현재 테스트 금지로 이번 head의 통과 여부는 미확인.
- GSD phase 완료와 authoritative 승인 증거를 혼동하지 않는다.

*Concerns audit: 2026-09-09*
