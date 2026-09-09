# Architecture

**Analysis Date:** 2026-09-09

## Pattern Overview

- 계약 중심 Rust workspace, 순수 평가 core와 I/O adapter 분리.
- 시간·정책·증거를 입력으로 받아 결정과 reason code를 반환한다.
- 로컬 advisory와 CI authoritative를 별도로 취급한다.

## Layers

- `crates/policy-contracts/src/`: identity, policy, evidence, GitHub observation, decision 타입.
- `crates/policy-core/src/`: canonical digest, provenance, reviews, sensors, pre/post-merge 순수 평가.
- `crates/evidence-collector/src/`: 제한된 프로세스 실행과 artifact 수집.
- `crates/github-adapter/src/`: gh 접근, 관측 파싱 및 binding.
- `crates/policy-cli/src/main.rs`: 로컬 inspect/evaluate-pre.
- `crates/policy-ci/src/main.rs`: trusted 입력, core 호출, check 발행과 결과 기록.
- `xtask/src/`: schema 및 Rust 검증 도구.

## Data Flow

1. Collector가 명시된 명령을 실행하고 기록/artifact를 수집한다.
2. Contracts가 입력 identity와 wire 형태를 표현한다.
3. Pre-merge evaluator가 정책, provenance, reviews, sensors를 결합한다.
4. CI 경계가 GitHub 관측과 결과 publication binding을 확인한다.
5. Post-merge evaluator가 pre-merge 결정과 check/merge 관측을 검증한다.
6. 모든 필수 조건을 충족해야 shipped 상태를 낼 수 있다.

상태는 명시 입력/출력에 있으며 개발용 `.planning`을 조회하지 않는다.

## Key Abstractions

- `PolicySnapshot`, `SensorProfile`: `crates/policy-contracts/src/policy.rs`.
- `EvidenceBundle`: `crates/policy-contracts/src/evidence.rs`.
- `evaluate_pre_merge`, `evaluate_post_merge`: `crates/policy-core/src/lib.rs`의 공개 API.
- `GitHubApi`/`GhCliApi`: 외부 관측 경계.
- `CollectionRequest`: collector의 검증된 실행 요청.

## Entry Points

- CLI/CI/collector의 `src/main.rs`, `xtask/src/main.rs`.
- CI workflow와 `action/action.yml`이 Rust 실행기를 호출한다.

## Error Handling

- 입력/프로세스 오류는 Result와 오류 타입으로 전달한다.
- 정책 non-success는 상태 및 안정적인 reason code로 설명한다.
- 누락·stale·trust 불일치를 성공으로 변환하지 않는다.

## Cross-Cutting Concerns

- canonical bytes/digest, exact head, provider 독립성은 여러 평가 경로에 영향을 준다.
- `AGENTS.md`의 순수 core와 advisory 경계를 유지한다.

*Architecture analysis: 2026-09-09*
