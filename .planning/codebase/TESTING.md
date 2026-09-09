# Testing Patterns

**Analysis Date:** 2026-09-09

## Test Framework

- Rust 기본 test harness, assert!/assert_eq!, proptest, test-case, assert_cmd.
- 버전은 `Cargo.toml`의 workspace dependencies.
- 현재 테스트 실행 금지 유지: 이번 매핑에서 모든 테스트는 not_run.
- 금지 해제 후 관련 검사 예: `rtk cargo test --locked -p policy-core --test pre_merge`.
- 최종 workspace CI 정의: `.github/workflows/ci.yml`.

## Test File Organization

- `crates/policy-core/tests/`: provenance, reviews, sensors, pre/post merge, adversarial/property 회귀.
- `crates/policy-cli/tests/`: CLI 출력과 종료 코드.
- `crates/policy-ci/tests/`: CI binding, Action, live GitHub 경계.
- `xtask/tests/`: schema와 Rust 검증 도구.
- 공유 입력: `tests/fixtures/`.

## Test Structure

- 정상 fixture를 읽고 한 필드를 변경한 뒤 상태와 reason code를 검증한다.
- 예: `crates/policy-core/tests/mutation_regressions.rs`.
- 순수 평가 함수는 실제로 호출하고 결과를 assert한다.

## Mocking

- 외부 gh/process 경계에 Rust fixture 사용.
- `crates/policy-ci/tests/support/fake_gh.rs`.
- `crates/evidence-collector/tests/support/fixture.rs`.
- 핵심 정책 판단을 fake 성공으로 대체하지 않는다.

## Fixtures and Factories

- JSON을 serde로 읽고 수정하는 보조 함수가 테스트 파일에 있다.
- 고정 timestamp, head, digest를 사용한다.
- 테스트 fixture는 운영 승인 증거로 재사용할 수 없다.

## Coverage

- `crates/policy-core/tests/property_release_gate.rs`: 결정성 및 경계 property.
- `crates/policy-core/tests/mutation_regressions.rs`: mutation 관련 회귀.
- 이번 매핑은 coverage나 mutation 점수를 측정하지 않았다.
- 최종 mutation/독립 review는 `docs/promotion.md`의 별도 릴리스 요구.

## Test Types

- contracts/schema 검증, 순수 core 통합, CLI/process 경계, 실제 GitHub smoke.
- fake-process와 실제 외부 서비스 검증을 구별한다.
- React/.NET 실제 소비 프로젝트 acceptance는 Task 14의 미구현 범위.

## Common Patterns

- 상태와 reason code를 함께 검증.
- nanosecond freshness, stale head, 빈/누락 증거, trust 변경 부정 사례.
- schema 재생성 결과를 체크인 파일과 비교한다.

*Testing analysis: 2026-09-09*
