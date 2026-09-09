# Technology Stack

**Analysis Date:** 2026-09-09

## Languages

- Rust 2021: 실행 코드와 테스트 전부. 근거: `Cargo.toml`, `AGENTS.md`.
- JSON/YAML: 계약 schema, fixture, CI 설정.
- Markdown: 설계, 계획, 리뷰, 개발 상태.
- Python 구현 추가 금지; 과거 ignore 항목은 현재 런타임 의존성을 뜻하지 않는다.

## Runtime

- `rust-toolchain.toml`: Rust/Cargo 1.95.0, rustfmt/clippy.
- Cargo workspace resolver 2, 7개 member, publish=false.
- `Cargo.lock`으로 의존성을 잠근다.
- GSD Core 1.13.0은 외부 개발 도구이며 workspace member가 아니다.

## Frameworks

- 서버/UI framework 없이 Rust 라이브러리와 CLI로 구성.
- clap 4.5.47: CLI 파싱.
- Rust test harness, assert_cmd 2.0.17, proptest 1.7.0, test-case 3.3.1.
- CI 정의: `.github/workflows/ci.yml`.

## Key Dependencies

- serde 1.0.228 / serde_json 1.0.145 / serde_yaml 0.9.34: 계약 직렬화.
- schemars 1.0.4 / jsonschema 0.55.0: schema 생성·검증.
- sha2 0.10.9 / hex 0.4.3: digest.
- chrono 0.4.42: 명시적인 시간 입력.
- thiserror 2.0.16: 오류 타입.
- globset 0.4.16: sensor 경로 선택.
- 위 버전은 `Cargo.toml`의 exact pins.

## Configuration

- `AGENTS.md`: Rust, RTK, GSD와 검증 범위.
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`: 빌드 기준.
- `schemas/`: 체크인된 계약 JSON schema.
- `.planning/`: 개발 상태만 관리; 커널 입력 아님.

## Platform Requirements

- CI는 Ubuntu runner와 pinned Rust를 사용한다.
- GitHub 연동은 외부 gh와 인증된 실행 환경이 필요하다.
- React/.NET 도구는 향후 외부 소비 프로젝트 실행 대상이며 현재 core 빌드 요건이 아니다.

*Stack analysis: 2026-09-09*
