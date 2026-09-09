# Coding Conventions

**Analysis Date:** 2026-09-09

## Naming Patterns

- 함수/모듈: snake_case; 타입: PascalCase.
- 진단 식별자는 `PK-...` reason code.
- 예: `evaluate_pre_merge`, `PolicySnapshot`.

## Code Style

- `rust-toolchain.toml`의 rustfmt/clippy 사용.
- CI clippy는 warnings를 오류로 취급한다.
- `AGENTS.md`: 실행 로직과 테스트는 Rust, 명령은 RTK.
- 전역 TDD/반복 리뷰를 강제하지 않고 변경 위험에 맞춰 검증한다.

## Import Organization

- `use`로 표준 라이브러리, 외부 crate, 계약 타입을 가져온다.
- `crates/policy-core/src/lib.rs`에서 공개 API를 명시적으로 재수출한다.
- 내부 구현 모듈은 필요 없이 공개하지 않는다.

## Error Handling

- `crates/github-adapter/src/lib.rs`: thiserror 오류 enum과 원인 전환.
- 경계 오류는 Result, 정책 판단은 상태/reasons로 표현한다.
- `crates/policy-contracts/src/policy.rs`: serde deny_unknown_fields와 schema version 검사.
- 새 필드와 digest 의미 변경은 명시적인 계약 버전 변경으로 다룬다.

## Logging

- CLI 경계에서 JSON stdout과 오류 stderr를 구분한다.
- core 판단에 ambient logging/network/process 의존성을 넣지 않는다.

## Comments

- trust, 시간 경계, binding의 이유를 설명한다.
- `crates/policy-core/tests/mutation_regressions.rs`에 회귀 의도를 설명하는 주석이 있다.

## Function Design

- 평가 함수는 명시적인 입력을 받고 결과를 반환한다.
- core 시간은 입력으로 주입하며 현재 시각을 직접 읽지 않는다.
- collector의 실제 시각/프로세스 관측은 외부 경계 책임이다.

## Module Design

- contracts/core 라이브러리와 I/O 실행기를 분리한다.
- `crates/policy-core/src/lib.rs`는 unsafe_code를 금지한다.
- 테스트 보조 구현은 production에 넣지 않는다.

*Convention analysis: 2026-09-09*
