# Codebase Structure

**Analysis Date:** 2026-09-09

## Directory Layout

```text
policy-kernel/
  crates/       contracts, core, CLI, CI, collector, GitHub adapter
  xtask/        Rust 개발 검증 도구
  schemas/      계약 JSON schema
  tests/        공유 fixture
  action/       재사용 GitHub Action
  .github/      CI workflow
  docs/         설계, 계획, 리뷰, 통합 문서
  .planning/    GSD 개발 상태
```

## Directory Purposes

- `crates/*/src/`: production Rust.
- `crates/*/tests/`: 패키지 통합/회귀 테스트.
- `tests/fixtures/`: evidence, review, sensor, merge 등의 공통 데이터.
- `docs/plans/`, `docs/specs/`, `docs/reviews/`: 현재 문서 경로.
- `docs/superpowers/`: 과거 증거 보존.

## Key File Locations

- 정책 타입: `crates/policy-contracts/src/policy.rs`.
- 판단 조합: `crates/policy-core/src/pre_merge.rs`, `post_merge.rs`.
- digest: `crates/policy-core/src/canonical.rs`, `pre_merge_digest.rs`.
- 외부 실행: `crates/evidence-collector/src/process.rs`.
- API 접근: `crates/github-adapter/src/gh.rs`.
- schema 생성: `xtask/src/main.rs`.
- residue/promotion 보조 검증: `xtask/src/verification.rs`.

## Naming Conventions

- crate 이름은 kebab-case, Rust 파일/함수는 snake_case.
- `lib.rs`에서 공개 API를 모으고 세부 모듈을 분리한다.
- 테스트 보조 코드는 각 `tests/support/`에 둔다.

## Where to Add New Code

- wire 계약은 contracts, 순수 판단은 core, 실행은 collector/adapter.
- Task 12의 project-verification과 Task 13 bootstrap은 아직 계획 단계.
- 새 workspace member는 원래 Task ID와 계약 선행 조건에 맞춰 추가한다.
- 테스트는 Rust, 데이터 fixture는 JSON/YAML 등으로 둔다.

## Special Directories

- `schemas/`는 생성되지만 체크인하고 재생성 차이를 검사한다.
- `target/`과 mutation 결과는 `.gitignore`에서 제외한다.
- `.planning/`에 실행 코드·인증 정보·운영 증거를 넣지 않는다.

*Structure analysis: 2026-09-09*
