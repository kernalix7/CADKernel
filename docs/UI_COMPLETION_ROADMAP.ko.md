# UI 완성 로드맵 (요약)

**상태:** 진행 중. 시작 2026-04-30. 2026-05-05 기준 A-C3 완료 + HARD-tier overlay/FEM/TechDraw export/view/dimension/annotation/centerline/ShapeBinder + TechDraw Page/Dimension/Annotation/Centerline/View Placement command UX + FEM Result Interpretation UX + FEM multi-node BC editor UX + Sketcher profile validation UX + Sketcher constraint diagnostics UX + Sketcher external reference/reuse UX 작업 트리 검증 완료. **2026-05-14 UI-A1**: PartDesign 스케치 피처 5개(PadSketch/PocketSketch/GrooveSketch/HoleSketch/CountersunkHoleSketch)가 HEAD `714136e` 기준 이미 배선 완료됨 — 로드맵의 "log_info 스텁" 분류 오류. 디스패처 경계 테스트 6개 추가 (`partdesign_sketch_features.rs`), 워크스페이스 3,317 / 0 / 1. **2026-05-14 UI-A2**: Draft 워크벤치 EASY 19개(`D::Line` ~ `D::ToSketch`) 전부 HEAD `584d334` 기준 이미 `cadkernel_modeling::draft_ops::*`에 배선 완료 확인 — §3.1의 "스텁" 분류 오류. 디스패처 경계 테스트 20개 추가 (`draft_easy_features.rs`), 워크스페이스 3,337 / 0 / 1. **verify-first 패턴**: 연속 2개 레인(UI-A1 PD 5개, UI-A2 Draft 19개)에서 총 24개 "스텁"이 실제로 이미 배선 완료된 것으로 확인. 로드맵이 2-3세션 분량 뒤처진 것으로 파악됨. **향후 UI-Ax 단계는 반드시 verify-first로 시작해야 함.**
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
| EASY (커널 API 존재, wiring만) | ~21 잔여 (※ §3.2 PartDesign 5개 2026-05-14 DONE 확인, §3.1 Draft 19개 2026-05-14 DONE 확인 — 기존 ~55에서 24개 감소) | 15-30분 |
| MEDIUM (커널 API 존재, UX 필요) | 18 | 1-2시간 |
| HARD (커널/API/렌더링 작업 필요) | 28 | 3-10시간 |

## 단계별 계획 (요약)

| 단계 | 내용 | 기능 수 | 추정 시간 |
|---|---|---:|---:|
| A | **Critical CAD 워크플로** (Pad / Pocket / Hole / Draft Line·Circle·Arc·Ellipse·Point) | 10 | 4-6h |
| B | **Draft 2D + 어레이 + Part 작업 EASY** | 18 | 8-12h |
| C | **Surface + Loft / Pipe + Draft transform MEDIUM** | 12 | 15-20h |
| D | **Draft 주석 (Dimension / Label + overlay)** | 3 | 작업 트리 검증 |
| E | **FEM 솔버 + colormap 시각화** | 7 | solver 2개 + colormap 3개 작업 트리 검증 |
| F | **TechDraw 워크벤치** | 24 | page/export/view/dimension/annotation/centerline 24개 검증 |
| G | **ShapeBinder** | 1 | 작업 트리 검증 |

**A-C3 합계:** Draft / Part / Surface / PartDesign의 EASY+MEDIUM 티어 종료. **2026-05-05 작업 트리:** overlay annotation, FEM thermal/nonlinear solver, FEM stress/displacement/VonMises colormap, TechDraw page/export/view/dimension/annotation/centerline, ShapeBinder, TechDraw Page/Dimension/Annotation/Centerline/View Placement command UX, FEM result legend/probe/table 해석 UX, FEM SectionPrint/Tie/Rigid/Contact BC editor UX, Sketcher single-profile validation UX, Sketcher constraint diagnostics UX, Sketcher external reference/reuse UX까지 **2,844 / 0 / 0**으로 검증 완료. TechDraw log-only 잔여는 닫혔고 page/dimension/annotation/centerline/view-placement parameter-entry UX와 FEM post-processing/BC editor UX, sketch profile readiness guard, duplicate/conflicting/invalid constraint 진단, 선택 객체 기반 external projection 및 `Refs:` / `Reuse:` 상태 표시가 완료되었습니다.

## 장기 순차 계획 (요약)

무작위 스텁 처리 대신 아래 순서로 진행합니다. 각 슬라이스는 구현 → 테스트 → 영문/국문 문서 → `WORK_STATUS.md` 갱신 → build/clippy/test 검증으로 종료합니다.

1. 검증된 현재 작업 트리 보존 및 작은 단위 커밋.
2. UI/TechDraw 완료: 주석, 중심선, 볼트 원, export/overlay 일치성.
3. UI 명령 UX: placeholder 기본값을 task panel/modal/selection prompt/preview로 교체.
4. Sketcher 생산 워크플로우: profile 검증, 제약 진단, 외부 참조, sketch 재사용.
5. PartDesign history/body 모델: editable feature tree, recompute, persistent naming repair.
6. Assembly 워크플로우: mate/joint UX, exploded view, interference, BOM export.
7. FEM 워크플로우: node/face set picker, mesh control, legend/probe/result table.
8. I/O 상호운용성: STEP/IGES/DXF/SVG/PDF 실전 corpus, units/layers/metadata/healing.
9. 성능/대형 모델 UX: async job, progress/cancel, GPU/wire pipeline, 1000+ part 기준.
10. 릴리스 준비: binary/Python wheel, 튜토리얼, CI release gate.

**현재 활성 순서:** 3번 UI 명령 UX는 TechDraw Page/Dimension/Annotation/Centerline/View Placement까지 완료되었고, 7번 FEM 워크플로우는 결과 해석 UX(legend/probe/result table)와 range 기반 multi-node BC editor까지 완료되었습니다. 4번 Sketcher 생산 워크플로우는 profile-readiness, actionable constraint diagnostics, visible external reference / sketch reuse feedback까지 완료했습니다. 다음은 Sketcher reference 관리 심화 또는 FEM node/face viewport picker를 좁은 검증 슬라이스로 진행합니다.

## 품질 게이트 (단계별 필수)

1. `cargo build --workspace` 클린
2. `cargo clippy ... -D warnings` 클린
3. `cargo test --workspace --no-fail-fast` — 최소 직전 베이스라인 유지 (현재 2,844). 각 Tier 1/2 기능당 1개 이상 회귀 테스트 추가
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
