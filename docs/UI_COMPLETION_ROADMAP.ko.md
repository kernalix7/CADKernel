# UI 완성 로드맵 (요약)

**상태:** 진행 중. 시작 2026-04-30. 2026-05-05 기준 A-C3 완료 + HARD-tier overlay/FEM/TechDraw export/view/dimension/annotation/centerline/ShapeBinder + TechDraw Page/Dimension/Annotation/Centerline/View Placement command UX + FEM Result Interpretation UX + FEM multi-node BC editor UX + Sketcher profile validation UX + Sketcher constraint diagnostics UX + Sketcher external reference/reuse UX 작업 트리 검증 완료. **2026-05-14 UI-A1**: PartDesign 스케치 피처 5개(PadSketch/PocketSketch/GrooveSketch/HoleSketch/CountersunkHoleSketch)가 HEAD `714136e` 기준 이미 배선 완료됨 — 로드맵의 "log_info 스텁" 분류 오류. 디스패처 경계 테스트 6개 추가 (`partdesign_sketch_features.rs`), 워크스페이스 3,317 / 0 / 1. **2026-05-14 UI-A2**: Draft 워크벤치 EASY 19개(`D::Line` ~ `D::ToSketch`) 전부 HEAD `584d334` 기준 이미 `cadkernel_modeling::draft_ops::*`에 배선 완료 확인 — §3.1의 "스텁" 분류 오류. 디스패처 경계 테스트 20개 추가 (`draft_easy_features.rs`), 워크스페이스 3,337 / 0 / 1. **2026-05-14 UI-A3**: Part 워크벤치 EASY 13개(`P::FaceFromWires` ~ `P::CoonsPatch`, `P::ProjectCurvesOnSurface` MEDIUM 제외) 전부 이미 `cadkernel_modeling::features::*`에 배선 완료 확인 — §3.3의 "스텁" 분류 오류. 디스패처 경계 테스트 14개 추가 (`part_easy_features.rs`), 워크스페이스 3,351 / 0 / 1. **verify-first 패턴 3연속 확인**: UI-A1(5) + UI-A2(19) + UI-A3(13) = **37/37 "스텁" 모두 이미 배선 완료**. 로드맵이 2-3세션 분량 뒤처진 것으로 파악됨. **향후 UI-Ax 단계는 반드시 verify-first로 시작해야 함.** **2026-05-14 UI-A4**: Surface §3.4 (`S::Coons` 1개) + FEM §3.5 (`FemAction::Summary`/`Report` 2개) + TechDraw §3.6 (24개) = 27/27 arm 모두 사전 배선 완료 확인. 누적 카운터 64/64. 27개 중 24개는 기존 테스트에서 이미 커버 완료(`S::Coons` `gui_action_integration.rs:2851`, TechDraw 23개 SVG/DXF/PDF 내용 검증 포함). 기존 테스트가 동어반복이던 3개는 강화: `fem_easy_features.rs` (4개 테스트), `techdraw_done_features.rs` (1개 테스트). 워크스페이스 3,351 → **3,356 / 0 / 1**. **2026-05-14 UI-B1 (MEDIUM 티어 최초 verify-first)**: Draft 워크벤치 MEDIUM 10개 arm (`D::Facebinder`, `D::Move`, `D::Rotate`, `D::Scale`, `D::Mirror`, `D::Offset`, `D::Trim`, `D::Stretch`, `D::Dimension`, `D::Label`) 모두 하드코딩 기본값으로 이미 배선 완료 확인 — EASY 티어와 동일 패턴. 누적 카운터: 64 EASY + 10 MEDIUM = **74/74**. MEDIUM 티어도 wired-with-defaults 패턴 확인. 디스패처 경계 테스트 10개 추가 (`draft_medium_features.rs`), 워크스페이스 3,356 → **3,366 / 0 / 1**. 전체 UX(모달/피커/기즈모/오버레이 편집)는 §3.8 MEDIUM-UX 백로그로 연기. **2026-05-14 UI-B2 (MEDIUM 티어 완전 소진)**: PartDesign 4개(`Pd::AdditiveLoft`/`AdditivePipe`/`SubtractiveLoft`/`SubtractivePipe`) + Part 1개(`P::ProjectCurvesOnSurface`) + Surface 3개(`S::Sections`/`Extend`/`Blend`) = 8/8 MEDIUM arm 모두 하드코딩 기본값으로 이미 배선 완료 확인. **§3.1–§3.4 MEDIUM 티어 2026-05-14 기준 완전 소진.** 누적 verify-first 카운터: 64 EASY + 18 MEDIUM = **82/82** (6연속 레인). 디스패처 경계 테스트 8개 추가 (`pd_part_surface_medium_features.rs`), 워크스페이스 3,366 → **3,374 / 0 / 1**. 커널 갭 주목: 진정한 접선 연속 서피스 블렌드 미구현 — `surface_from_curves` 임시 대체. 전체 프로덕션 UX는 §3.8 Phase-F 버킷으로 연기. **2026-05-14 UI-B3 (첫 실제 wiring 레인)**: §3.2에 미추적된 PartDesignAction 7개 발견. verify-first 결과 TRUE-STUB 3개(`CreateSprocket`/`CreateShaftDesign`/`CreateInvoluteGear`) 신규 배선 + 이미 배선된 5개(`ShapeBinder`/`SuppressFeature`/`SetTip`/`MoveFeatureUp`/`MoveFeatureDown`) 재확인. §3.7 HARD-1 항목 스테일 — 기계 생성기 3개 모두 커널 API 이미 존재, PartDesign HARD 1 → **0**, 전체 HARD **0**. 디스패처 경계 테스트 8개 추가 (`pd_untracked_features.rs`), 워크스페이스 3,374 → **3,382 / 0 / 1**. 누적 카운터: **90/90** (7연속 레인).
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
| EASY (커널 API 존재, wiring만) | ~15 잔여 (※ §3.2 PD 5개 UI-A1 DONE, §3.1 Draft 19개 UI-A2 DONE, §3.3 Part 13개 UI-A3 DONE, §3.4 S::Coons + §3.5 Summary/Report 3개 UI-A4 DONE — 기존 ~55에서 40개 감소) | 15-30분 |
| MEDIUM (커널 API 존재, UX 필요) | **0 잔여** (기존 ~18; Draft MEDIUM 10개 UI-B1 DONE; PD+Part+Surface MEDIUM 8개 UI-B2 DONE 2026-05-14 — **MEDIUM 티어 완전 소진**) | 1-2시간 |
| HARD (커널/API/렌더링 작업 필요) | 28 (PartDesign HARD 0개 — §3.7 HARD-1 항목 2026-05-14 스테일 확인) | 3-10시간 |

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
