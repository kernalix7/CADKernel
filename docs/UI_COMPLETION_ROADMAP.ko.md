# UI 완성 로드맵 (요약)

**상태:** 진행 중. 시작 2026-04-30.
**원문:** [English (canonical)](UI_COMPLETION_ROADMAP.md)

이 한국어 파일은 영문 원문의 요약입니다 — 자세한 표 / 단계별 세부 사항 / 모든 79개 스텁 매핑은 영문판을 참조하십시오 (CLAUDE.md Section 5의 이중 언어 정책에 따라 영문 원문이 정본).

## 핵심 발견

- `crates/viewer/src/app.rs`에 **86개의 비기능 디스패처 arm** — UI 클릭에 `log_info` 한 줄만 출력하고 실제 기하학 / 씬 변경 / 사용자 가시 효과 0
- 기본 CAD 작업(스케치 → Pad / Pocket / Hole)이 동작하지 않음 — 이번 세션에서 사용자가 지적한 핵심 문제
- 직전 V37 작업(모듈 분할, sub-enum 분할, 패닉 안전성, 테스트 코퍼스)은 엔지니어링 위생 작업이었고 사용자 가시 기능을 한 개도 추가하지 않음

## 좋은 소식

86개 스텁 중 **약 40개는 EASY 티어** — 커널 API가 이미 존재하므로 디스패처 arm에서 호출만 하면 됨. 새 커널 코드 0, 순수 wiring.

| 티어 | 개수 | 단위 노력 |
|---|---:|---|
| EASY (커널 API 존재, wiring만) | 40 | 15-30분 |
| MEDIUM (커널 API 존재, UX 필요) | 18 | 1-2시간 |
| HARD (커널 API 없음) | 28 | 3-10시간 |

## 단계별 계획 (요약)

| 단계 | 내용 | 기능 수 | 추정 시간 |
|---|---|---:|---:|
| A | **Critical CAD 워크플로** (Pad / Pocket / Hole / Draft Line·Circle·Arc·Ellipse·Point) | 10 | 4-6h |
| B | **Draft 2D + 어레이 + Part 작업 EASY** | 18 | 8-12h |
| C | **Surface + Loft / Pipe + Draft transform MEDIUM** | 12 | 15-20h |
| D | **Draft 주석 (Dimension / Label)** | 3 | 8-12h |
| E | **FEM 솔버 + colormap 시각화** | 7 | 30-50h |
| F | **TechDraw 워크벤치** (대부분 새 커널 작업) | 22 | 60-100h |
| G | **ShapeBinder** (새 커널 개념) | 1 | 5-10h |

**A + B + C 합계: 40개 기능 / 27-38시간** — 이 세 단계만 완료해도 Draft / Part / Surface / PartDesign의 EASY+MEDIUM 티어 전체 종료, 즉 "기본 CAD가 작동" 목표 달성.

## 품질 게이트 (단계별 필수)

1. `cargo build --workspace` 클린
2. `cargo clippy ... -D warnings` 클린
3. `cargo test --workspace --no-fail-fast` — 최소 직전 베이스라인 유지 (현재 2,662). 각 Tier 1/2 기능당 1개 이상 회귀 테스트 추가
4. **수동 수락 테스트** — 단계당 최소 1개 핵심 기능 GUI에서 end-to-end 검증, 결과를 커밋 메시지에 명시
5. `CHANGELOG.md`(영문 정본) + `docs/CHANGELOG.ko.md` 요약 업데이트
6. `docs/UI_COMPLETION_ROADMAP.md` 진행 트래커 갱신

## 팀 라우팅

CLAUDE.md Section 11에 따라:
- 단계 A-C: **ui-engineer 주도**, kernel-engineer는 커널 API 부족 시에만
- 단계 D: ui-engineer (오버레이 렌더링)
- 단계 E-F: kernel + ui + qa 협업
- 단계 G: kernel-engineer 주도 (새 개념 설계)
- io-engineer: 단계 F (TechDraw) 시작 시까지 대기, 코퍼스 Phase 2는 무기한 보류

## 진행 트래커

(영문 원문의 Section 7 참조 — 단계별 상태가 이곳과 동기화됨)
