# Commercial CAD Roadmap v3.5 — CADKernel (한국어 요약)

**상태:** 활성. v3.5 "코퍼스 + ADR 심화 + 첫 실행 가능 시드" 2026-05-06 — v3.4 위에 ADR 5개 추가 (0011 automerge CRDT / 0012 wasm 플러그인 샌드박스 / 0013 CBOR / 0014 zstd / 0015 sparse Cholesky), [`docs/perf/memory-profile.md`](perf/memory-profile.md) 메모리 명세, 골든 TOML 6개 추가 (sketch slot/hexagon/triangle-in-circle/redundant-tangent + boolean box∩box/sphere∩box/idempotent), Lua 레퍼런스 파트 2개 (R1 박스 / R2 추출), 첫 컴파일 가능 Rust 시드 [`examples/build_reference_parts.rs`](../examples/build_reference_parts.rs). 누적: v3 산업급 + v3.1 UX + v3.2 전 트랙 심화 + v3.3 알고리즘 모듈 분리 + v3.4 ADR/perf/corpus + v3.5 시드. 5-기둥 (로드맵/알고리즘/ADR/perf/corpus) 이 ADR 15 + perf 4 + 알고리즘 13 + 골든 11 + Lua 2 + Rust 시드 1로 받쳐짐. 문서+코퍼스 9,299 줄 (로드맵 EN 3,169 + KO 287 + algorithms 2,717 + ADR 1,005 + perf 709 + corpus 1,309 + Rust 시드 103).
**원본(정본):** [영문판](COMMERCIAL_CAD_ROADMAP.md). 알고리즘 수준 세부 논증/스펙은 [`docs/algorithms/`](algorithms/), 결정 사유는 [`docs/adr/`](adr/), 성능 계약은 [`docs/perf/`](perf/), 골든 테스트는 [`tests/corpus/`](../tests/corpus/) 하위 파일들이 정본.
**대체:** v3.3, v3.2, v3.1, v3, v2, v1.

> 이 문서는 영문 원본의 **요약본**입니다. v3.2에서 추가된 세부 (Rust struct 정의, 알고리즘 티어 사다리, 쇼핑 테이블, 세부 IP 감사, 제품 재제 상세···)는 영문판 참조.

---

## 0. v3에서 바뀐 것

v2는 골격은 맞았지만 **알고리즘·자료구조·파일 포맷 바이트 레이아웃·SolidWorks/CATIA/Fusion 360/Inventor/FreeCAD 대비 정량 목표·산업 컴플라이언스 벤치마크**를 명시하지 않았다. v3에서 모두 보충.

- **트랙 4 → 6.** Track E (생태계), Track F (협업·클라우드) 추가.
- **Track C 13개 서브 트랙으로 분해.** C-Solid, C-Sketch, C-Surface, C-Assy, C-Mech, C-Sheet, C-Weld, C-Mold, C-Draw, C-Sim, C-CAM, C-Render, C-Reverse, C-Subdiv.
- **Non-negotiables 25 → 60.** STEP AP242 + JT 왕복, GD&T (ASME Y14.5-2018), 시트메탈 평전개 패리티, 어셈블리 메이트 솔버 수렴, T-spline 서브디비전, 제너레이티브 디자인, CAM 포스트 라이브러리, 실시간 협업 (CRDT), 플러그인 샌드박스, 교육 라이선스 등.
- **레퍼런스 파트 R1-R4 → R1-R12.** 시트메탈 인클로저, 용접 프레임, 몰드 툴 키트, 사출 부품, CAM 가공 부품, 제너레이티브 브래킷, 250-부품 어셈블리, 10k-피처 스트레스 모델 추가.
- **상용 CAD 패리티 매트릭스.** 모든 페이즈가 SW/CATIA/F360/Inventor/FreeCAD 대비 명시 게이트 인용.
- **알고리즘 참고 문헌.** The NURBS Book, Hoffmann Geometric and Solid Modeling, OCCT 문서, Real-Time Rendering, PBR Book, Nocedal-Wright Numerical Optimization, Bathe FEM, Pharr-Jakob-Humphreys 등 매 페이즈에 출처.
- **산업 컴플라이언스 벤치.** NIST CAX-IF, ASME Y14.5/Y14.41, JT B-Rep Annex, AP242 적합성 클래스, LinuxCNC 후처리.
- **구체 릴리스 캘린더.** v0.5 (Q3 2026) → v1.0 (Q1 2027) → v2.0 (Q4 2027) → v3.0 (Q3 2028).
- **종속성 IP 감사.** Apache-2.0 호환만 채택; OCCT/Parasolid/ACIS/CGAL는 명시적으로 거부 (런타임 도입 안 함).

핵심 야망: **v1.0에서 FreeCAD를 능가하는 경쟁자, v2.0에서 Onshape급 클라우드 CAD, v3.0에서 SolidWorks/Inventor급 데스크톱 CAD가 된다 — 명시 제품과 비교한 측정 가능한 패리티 게이트로.**

---

## 1. 비교 패리티 매트릭스 (정본 §1, 발췌)

| 능력군 | FreeCAD 1.0 | SW 2026 | CATIA V6 | F360 | Inventor 2026 | v1.0 | v2.0 | v3.0 |
|---|---|---|---|---|---|---|---|---|
| 스케처 well-determined 판정 | 부분 | 완전 | 완전 | 완전 | 완전 | 완전 | 완전 | 완전 |
| 스플라인 G2/G3 | 부분 | 완전 | 완전 | 부분 | 부분 | 부분 | 완전 | 완전 |
| 가변 반경/비대칭 필렛 | 부분 | 완전 | 완전 | 부분 | 부분 | 부분 | 완전 | 완전 |
| Class-A 서피싱 | 없음 | 부분 | 완전 | 부분 | 부분 | 없음 | 부분 | 완전 |
| T-spline 서브디비전 | 없음 | 부분 | 완전 | 완전 | 부분 | 없음 | 부분 | 완전 |
| 시트메탈 평전개 + K-팩터 | 부분 | 완전 | 완전 | 완전 | 완전 | 부분 | 완전 | 완전 |
| 용접 구조재 + 절단리스트 | 부분 | 완전 | 완전 | 부분 | 완전 | 없음 | 부분 | 완전 |
| 몰드 툴링 | 부분 | 완전 | 완전 | 부분 | 부분 | 없음 | 부분 | 완전 |
| Y14.5-2018 GD&T | 부분 | 완전 | 완전 | 완전 | 완전 | 부분 | 완전 | 완전 |
| MBD (Y14.41) | 없음 | 완전 | 완전 | 부분 | 부분 | 없음 | 부분 | 완전 |
| 비선형 FEM | 없음 | 완전 | 완전 | 부분 | 부분 | 없음 | 부분 | 완전 |
| 플라스틱 사출 시뮬 | 없음 | 완전 | 완전 | 없음 | 없음 | 없음 | 없음 | 부분 |
| CAM 5축 동시 | 없음 | 부분 | 완전 | 부분 | 부분 | 없음 | 없음 | 부분 |
| 패스트레이스 렌더 | 없음 | 완전 | 완전 | 완전 | 부분 | 없음 | 없음 | 완전 |
| 메시 → 서피스 | 부분 | 부분 | 완전 | 완전 | 부분 | 없음 | 부분 | 완전 |
| 위상 최적화 | 부분 | 완전 | 완전 | 완전 (Generative) | 부분 | 없음 | 부분 | 완전 |
| STEP AP242 PMI | 부분 | 완전 | 완전 | 완전 | 완전 | 부분 | 완전 | 완전 |
| JT 10.7 R/W | 없음 | 완전 | 완전 | 부분 | 부분 | 없음 | 부분 | 완전 |
| 실시간 공동 편집 | 없음 | 부분 | 부분 | 완전 | 부분 | 없음 | 부분 (CRDT) | 완전 |
| AI 에이전트 통합 (MCP) | 없음 | 없음 | 없음 | 없음 | 없음 | 완전 | 완전 | 완전 |

상세는 영문판.

---

## 2. 60개 Non-negotiables (요약)

- **v0.5 (12개)**: API SemVer, undo/redo, save/load, 스케마, MCP 분리, 선택, 프로퍼티 양방향, 스케처 DoF, R1/R2 빌드, 첫 페인트 < 500 ms, 패닉 금지.
- **v1.0 (24개)**: `.cadk` 포맷, 영구 명명, 자동 저장 복구, STEP AP214 왕복, NIST CAX 통과, 분기 undo, 피처 그래프, 캐시 재계산, 네비큐브 + 단면, 스케처 v2, 히스토리 트리, 멀티-doc, i18n en+ko, a11y, 명령 팔레트, 스케처 v3 솔버, 풀 피처 스위트, 패턴, 어셈블리 메이트 + DoF + 모션, TechDraw + GD&T-2018 + BOM, 30 fps R3, 1M 부울 퍼즈, 재현 가능 빌드, 서명 인스톨러, CVE 정책.
- **v2.0 (16개)**: AP242 PMI, JT 10.7, 3D PDF, 시트메탈, 용접, Class-A 블렌드, T-spline, 컨피그/디자인 테이블, 비선형 FEM, CAM 2.5축 + 포스트, 메시→서피스, 위상 최적화, 플러그인 SDK + 마켓, 교육 라이선스, CRDT 공동 편집, 클라우드 저장.
- **v3.0 (8개)**: 몰드 툴링, CAM 5축 + 선반, 플라스틱 사출 시뮬, CFD-lite, 제너레이티브 디자인, 패스트레이스 렌더, 워크스루/VR, PDM + ECO.

---

## 3. 레퍼런스 파트 R1-R12

| ID | 파트 | 용도 |
|---|---|---|
| R1 | 소형 브래킷 (~12 피처) | 솔리드 + 스케치 기본 |
| R2 | 하우징 (~30 피처) | 면-위 스케치 + 재계산 |
| R3 | 기어박스 어셈 (3 부품) | 어셈블리 + 도면 |
| R4 | 스트레스 부품 (200 면) | 솔버 + 성능 |
| R5 | 시트메탈 인클로저 | 시트메탈 |
| R6 | 용접 프레임 (12 부재) | 용접 |
| R7 | 몰드 툴 키트 | 몰드 툴링 |
| R8 | 사출 인클로저 | 플라스틱 + 분석 |
| R9 | CAM 가공 부품 | CAM |
| R10 | 제너레이티브 브래킷 | 제너레이티브 + 시뮬 |
| R11 | 빅 어셈 (250 부품) | 어셈 스트레스 |
| R12 | 10k-피처 모놀리식 | 스케일 스트레스 |

---

## 4. 트랙 개요

### Track A — Platform
- **A1** done — `cadkernel-api` v0
- **A2** in flight — Phase 2A 슬라이스 (Extrude/Pattern/Mirror/undo/save)
- **A3** — `.cadk` 네이티브 포맷 (zstd + Ed25519 서명 + 크래시 복구)
- **A4** — STEP AP214 + AP242 PMI 왕복 (NIST CAX 통과)
- **A5** — Document v2 (DAG + 위상 정렬 + 표현식 엔진 + 영구 명명)
- **A6** — MCP 서버 새 크레이트로 분리
- **A7** — 분기 undo + 명명 체크포인트 + 히스토리 스크럽
- **A8** — 멀티-도큐먼트 코디네이션 (where-used)

### Track B — Experience (UI/UX) — 20개 페이즈 (v3.1, "UX deep-dive")

> 정본 §5(영문)에 윈도우 셸 레이아웃, 디자인 토큰, 컴포넌트 카탈로그, 워크벤치별 와이어프레임/단축키, 인터랙션 모델 등 픽셀 단위 명세 포함. 한글판은 페이즈 제목만 요약.

**5.0 정보 아키텍처:** 5개 영역 (TitleBar 32 px / RibbonBar 96 px / Left dock 패널 / Central Viewport / Right 패널 / StatusBar 24 px). 워크벤치 = `Workbench { id, default_layout, ribbon, shortcuts, contributed_panels }` 추상화. v1.0 워크벤치: Part / Sketcher (모달) / Assembly / Drawing. v2.0: + SheetMetal / Weldment / Surface / Mold. v3.0: + Sim / CAM / Render / Inspect.

**5.1 디자인 시스템:** 컬러 토큰 (light/dark/HC 30+개, WCAG AA 보장) · 타이포그래피 (Inter Variable + JetBrains Mono + Noto Sans CJK, 11종 역할) · 8 px 그리드 간격 토큰 · 라운드/섀도우 토큰 · 모션 토큰 (호버 80 ms, 모달 280 ms 스프링, 카메라 핏 350 ms cubic) · 39종 컴포넌트 카탈로그 · 380→700 인하우스 아이콘 · 15종 컨텍스트 커서.

**5.2 인터랙션 모델:** 마우스 스킴 3종 (CAD / Maya / Blender) 전환 가능 · 글로벌+뷰포트+워크벤치별 단축키 · 터치 제스처 (1F 선택 / 2F 핀치 줌 / 2F 회전 / 3F 오빗 / long-press 메뉴) · 펜 (압력은 힌트만, 팜 리젝션 기본 ON, 펜 버튼 툴 사이클).

**5.3 워크벤치별 와이어프레임:** Part / Sketcher / Assembly / Drawing은 ASCII 와이어프레임 포함; SheetMetal / Weldment / Surface / Mold / Render / Sim / CAM / Inspect는 `docs/wireframes/<workbench>.png` (B0 산출물).

**5.4 워크벤치별 단축키 표:** Sketcher 30+개 · Part 20+개 (canonical: `docs/keymap.md`).

**5.5 20개 페이즈:**
- **B0** [선결] UI 파운데이션 리팩터링 (`viewer/src/lib.rs` 5kloc → 모듈 분할, Workbench 추상화, ThemeContext, AccessKit 랜드마크)
- **B1** [최우선] GPU id-buffer 선택 (R32Uint), 서브샵 우선순위 V>E>F>B, 박스/라쏘, 호버 프리뷰 100 ms 페이드, 선택 세트 직렬화
- **B2** ViewCube (120 px, 26 hit-zone), ViewGizmo (80 px), 단면 박스, 단면 평면+sketch-on-section, 분해도 애니, 워크스루 (WASD), 뷰 북마크 99개, perspective/ortho/2-point/롤락
- **B3** 프로퍼티 패널 양방향 라이브 (200 ms 디바운스), `NumericInput` (단위 접미어, 스피너, 드래그 스크럽, fx 모드), 단위 시스템 (uom 0.36, SI 내부), 표현식 (sin/cos/clamp/lerp/lookup/conditional), 파라미터 테이블, Inspector 패널
- **B4** 스케처 v2 (in-place dim, 드래그 핸들, 제약 글리프, DoF readout + Solver Diagnostics, 외부 참조, 스냅 8종, 자동 제약 추론, 곡률 빗 G2/G3, 스케치 내 패턴, 코닉 제약)
- **B5** 테마 (라이트/다크/HC 토큰 시스템, 워크벤치 액센트, hot-reload), i18n (en/ko v1.0 → +ja/zh-CN v2.0 → +de/fr/es/ru v3.0; fluent + CI 게이트), a11y (AccessKit, 단축키 오버레이 `?`, HC 7:1, 포커스 링, 스크린리더 announce, 디슬렉시아 폰트, UI 스케일 1×–4×), DPI 멀티모니터 반응
- **B6** 히스토리 트리 (가시성/억제/리오더/그룹/분기 그래프 / 명명 체크포인트 `Ctrl-Shift-K` / 히스토리 스크럽)
- **B7** 멀티-doc 탭 (드래그 분할, 컨텍스트 메뉴, 드래그-드롭 열기), Welcome 화면, Recents + 핀, `.cadkws` 워크스페이스, `Ctrl-Tab` 썸네일 스위처
- **B8** PBR Disney BRDF · 4-cascade CSM 2048² · HBAO+ · TAA · depth-peel 투명 · 분석 라인 렌더링 (silhouette+crease) · 9개 표시 모드 (`Z` 사이클) · 80→200 머티리얼 프리셋 · HDRI IBL · 데칼 (post v1.0)
- **B9** 명령 팔레트 `Ctrl-K` (퍼지 매치, `>`/`?`/`:`/`@`/`#` 모드), 매크로 레코더, 핫키 커스터마이저 (CAD/Maya/F360/Onshape/Inventor/FreeCAD 프로파일, 충돌 검출), 워크스페이스 세이버, `Ctrl-P` quick switch
- **B10** 3D 뷰포트 내 스케치 오버레이, 곡률 빗, 반사선, iso-curve, 매니퓰레이터 (translate/rotate/scale/combo, 90 px 화면-스페이스), 드래그 중 숫자 입력, 데이텀 시각화
- **B11** 첫 실행 투어 (8단계, 10분 내 첫 성공), 12개 튜토리얼 라이브러리 (R1-R12 기반), 인라인 코치마크, F1 컨텍스추얼 도움말, In-app 체인지로그, 샘플 문서 20개
- **B12** 터치/펜 모드 (44 px 히트 타깃), 제스처 7종, 펜 압력 힌트, 펜 버튼 사이클, 온스크린 키보드, 라디얼 컨텍스트 메뉴 (v2.0)
- **B13** 모션 (`motion::transition`), spring/cubic-bezier 곡선, reduced-motion OS 연동, 토스트 슬라이드, 코치마크 펄스 (60 fps 보장)
- **B14** 토스트/알림 (info/success/warning/error 4종, 4 s 기본), 알림 센터, 모달 에러 (Copy details), 리컴퓨트 에러 인디케이터, 진행률 다이얼로그 ESC 취소
- **B15** 문서 내 검색 `Ctrl-F`, 참조 찾기, 파라미터 일괄 치환 `Ctrl-H`, 이름으로 점프 `Ctrl-G`, 태그 시스템
- **B16** 설정 다이얼로그 `Ctrl-,` (9개 카테고리, 검색, 프로파일 import/export, 문서별 오버라이드, 섹션별 리셋)
- **B17** 옵트인 텔레메트리 (커맨드 빈도/perf/크래시), 사용자 검토 후 업로드, GDPR DPA 90일, ε=1.0 차분 프라이버시
- **B18** 도면 워크벤치 UX (시트 룰러, 뷰 자동 정렬, smart-dim, GD&T 프레임 에디터, BOM 드래그, 해치 에디터, 레이어 패널, 인쇄 미리보기 PDF)
- **B19** 실시간 협업 UX (프레전스 아바타, 라이브 커서/셀렉션/카메라, in-doc 채팅, 코멘트 스레드, suggestion 모드, 피처 락, CRDT 충돌 모달, WebRTC 음성)
- **B20** a11y deep-dive (스크린리더-퍼스트 뷰포트, HC 7:1 AAA, 음성 제어 훅, 매그니파이어 4× 호환, 스위치 입력)

**5.6 페이즈 → v-target:** B0/B1/B3 v0.5 · B2/B4/B5/B6/B7/B8/B9/B10/B11/B13/B14/B15/B16/B17/B18/B20 v1.0 · B12/B19 v2.0.

**5.7 UX 검증:** 비주얼 리그레션 (pixelmatch ≤ 0.1 %, 60 베이스라인) · 인터랙션 테스트 (`viewer/tests/interaction/`) · a11y CI (AccessKit 자동 검사) · i18n CI (누락/일치 키) · D1 성능 버짓 · 도크 레이아웃 직렬화 라운드트립 · 출시 전 5명 외부 사용자 첫 실행 스터디.

### Track C — Capability (13 서브 트랙)

| 서브 트랙 | v0.5 | v1.0 | v2.0 | v3.0 |
|---|---|---|---|---|
| C-Solid | 핵심 ops | 풀 피처 스위트 | 다이렉트 에디트 | 자유형 |
| C-Sketch | v1 | v2 | v3 솔버 | 코닉 폴리시 |
| C-Surface | — | ruled / blend | Class-A | 반사선 청정 |
| C-Assy | — | 풀 메이트 + DoF | 컨피그 + 디자인 테이블 | SW 패리티 |
| C-Mech | — | — | 기본 모션 | 풀 메커니즘 |
| C-Sheet | — | — | 풀 시트메탈 | 폴리시 |
| C-Weld | — | — | 구조재 | Inventor 패리티 |
| C-Mold | — | — | 부분 | 완전 |
| C-Draw | — | 멀티뷰 + GD&T | MBD | 완전 |
| C-Sim | — | 선형 정적 | 비선형 + 열 | 플라스틱 + CFD-lite |
| C-CAM | — | — | 2.5축 + 포스트 | 5축 + 선반 |
| C-Render | — | PBR | 패스트레이스 | 워크스루 / VR |
| C-Reverse | — | — | 메시→서피스 + 위상 최적화 | 완전 |
| C-Subdiv | — | — | T-spline 기본 | 완전 |

각 서브 트랙의 알고리즘·기능 목록·게이트는 영문판 §6 참조.

### Track D — Trust
- **D1** 성능 예산 (R1 < 250 ms, R2 < 1 s, R3 < 3 s, 30 fps R3, 60 fps 스케치)
- **D2** 견고성 — 1M 부울 퍼즈 + 100h cargo-fuzz
- **D3** 테스트 — 비주얼 회귀 + 변이 테스트 + R1-R12 회귀 + NIST/Y14.5/JT 컴플라이언스
- **D4** 패키징 — Linux/macOS/Windows 서명 + 자동 업데이트 + 재현 가능 빌드 + SBOM
- **D5** 문서 — mdBook + 12개 튜토리얼 + ADR + AI 쿡북
- **D6** 보안 — `cargo deny/vet/public-api` + Wasm 샌드박스 + GDPR
- **D7** 릴리스 — SemVer + 분기별 stable + 마이그레이션 가이드 자동 생성

### Track E — Ecosystem (신규)
- **E1** 플러그인 SDK v2 (Wasm 호스트 + 네이티브 호스트)
- **E2** 플러그인 마켓플레이스 (`plugins.cadkernel.dev`)
- **E3** 교육 — 학교/대학 프로그램 + 인증 시험 + 커뮤니티 갤러리
- **E4** API 안정성 — RFC 프로세스 + `cadkernel-compat` 호환 셰임

### Track F — Collaboration & Cloud (신규)
- **F1** Document CRDT (Automerge 기반)
- **F2** 클라우드 저장 (S3 호환, 셀프 호스팅 가능)
- **F3** 실시간 협업 UX (라이브 커서 + 음성 + 프레즌스)
- **F4** PDM 워크플로 (리비전 + ECO + where-used)

---

## 5. 횡단 관심사

### 단일 깔때기 계약
GUI / Lua / AI / 테스트 / CRDT op → `Command` (typed, JSON-schema'd) → `Session::execute` (deterministic) → Document 변경 + history → Outcome → 뷰어 더티 전파.

### 커널 아키텍처 결정 (잠금, 2026-05-06)
- 기존 half-edge + NURBS 커널 유지.
- OCCT / Parasolid / ACIS 런타임 도입 **거부** (라이선스 비호환 + 프로젝트 존재 이유).
- 다년간 자체 커널 성장 비용 수용.

### IP 감사 — 종속성
- **채택**: nalgebra, glam, wgpu, egui, winit, rayon, serde, mlua, wasmtime, automerge (감사 대기), Eigen (FEM 후보).
- **거부**: cgal (GPL), OCCT (런타임으로는), Parasolid / ACIS (상용).
- 신규 deps는 `cargo vet` 리뷰 필수.

### 산업 컴플라이언스
- NIST CAX-IF (STEP) → v1.0 게이트
- ASME Y14.5 GD&T → v1.0
- ISO 10303-242 PMI → v2.0
- JT B-Rep Annex → v2.0
- LinuxCNC G-code → v2.0
- 3MF Consortium → v1.0

### 상용 대비 성능 목표 (영문판 §10.6 참조)
v1.0에서 R2 cold open < 1 s, R2 dim 재계산 < 200 ms, R3 30 fps, 스케치 60 fps — SW/Inventor와 비교 가능한 수준.

### 팀 분담 (CLAUDE.md §11)
- A: kernel-engineer (lead) + io-engineer
- B: ui-engineer (lead)
- C-Solid/Sketch/Surface/Mech/Sim: kernel-engineer
- C-Sheet/Weld/Mold: kernel-engineer + ui-engineer
- C-Draw/CAM: io-engineer + ui-engineer
- C-Render: ui-engineer + kernel-engineer
- D: qa-engineer (lead)
- E: qa-engineer + io-engineer + tech-lead
- F: io-engineer (프로토콜) + ui-engineer (프레즌스)

크로스 트랙 변경(시그널/Command enum/Document 스키마)은 **항상** tech-lead 경유.

---

## 6. 리스크 (정본 §10.9 발췌)

| # | 리스크 | P | I | 완화 |
|---|---|---|---|---|
| 1 | `.cadk` 스키마 너무 일찍 잠금 | M | H | 모든 필드 `Option<T>` 1 minor 후 필수화 |
| 4 | 재계산 캐시 무효화 버그 | H | C | 모든 입력 해시; R1-R12 해시 베이스라인 PR 게이트 |
| 9 | STEP 라이브러리가 AP242에서 발산 | M | H | 감사 + 골든 파일 + 빠른 실패 |
| 10 | 성능 예산 드리프트 | H | M | PR별 벤치 게이트; 예산은 경고 아닌 실패 |
| 11 | 부울 엔진 퇴화 입력 실패 | M | C | 정밀 산술 코어 + 1M 퍼즈 릴리스 전 |
| 13 | 플러그인 마켓 멀웨어 | M | C | Wasm 샌드박스 + 서명 + 네이티브는 수동 리뷰 |
| 14 | CRDT 머지가 무효 토폴로지 생성 | M | C | `apply_remote_op`에서 검증; 실패 시 롤백 |
| 15 | 다이렉트 에디트/파라메트릭 특허 분쟁 | L | C | C-Solid 다이렉트 에디트 출시 전 특허 풍경 감사 |
| 20 | 리팩토링 후 레퍼런스 파트 회귀 | H | H | R1-R12 빌드 스크립트 머지마다 게이트 |

전체 20개 영문판 참조.

---

## 7. 릴리스 캘린더

| 버전 | 시기 | 헤드라인 |
|---|---|---|
| **v0.5 "Honest Beta"** | Q3 2026 | API + 선택 + 프로퍼티 + 스케처 v2 부분 |
| v0.6-0.8 | Q4 2026 - Q1 2027 | 멀티-doc, 자동 저장, STEP AP214, 솔버 v3, 어셈블리, 도면 |
| **v1.0 "FreeCAD 경쟁자"** | Q1 2027 | 36개 게이트 |
| v1.x | 2027 | 강화 |
| **v2.0 "Onshape급"** | Q4 2027 | 시트메탈 + 용접 + 비선형 FEM + CAM 2.5축 + CRDT 클라우드 |
| v2.x | 2028 | 폴리시 |
| **v3.0 "SW/Inventor 패리티"** | Q3 2028 | 몰드 + 5축 CAM + 플라스틱 + 패스트레이스 + PDM |

---

## 8. 현황 스냅샷 (2026-05-06)

- 9-크레이트 워크스페이스 + 신규 `cadkernel-api`; 약 2,865 / 0 / 0 테스트.
- 13 프리미티브, 24 피처, 5 부울, NURBS 커널 (28 알고리즘), 24 스케치 제약, 11 파일 포맷, 9 뷰어 워크벤치.
- A1 완료; A2 ~70 % 진행.
- v3 매트릭스 대비 패리티: v1.0 셀의 ~25 %, v2.0 셀의 ~5 %, v3.0 셀의 ~0 %.

**v1.0까지의 작업:** A2 마무리, A3-A5 출하, B1-B11 출하, C-Solid v1, C-Sketch v3, C-Assy v1, C-Draw v1, D 코어 전부. 5팀 분담 시 집중 작업 2-3분기 소요 추정.

---

## 9. 상호 참조

- `WORK_STATUS.md`
- `UI_COMPLETION_ROADMAP.md` (대체됨; Track B 입력)
- `FREECAD_PARITY_PLAN.md` (대체됨; Track C 입력)
- `docs/wiki/Architecture.md` — A2 후 6-트랙 모델 부록
- `docs/VERIFICATION_CHECKLIST.md` — 페이즈마다 한 줄 + 레퍼런스 파트 어서션
- `docs/adr/` — 아키텍처 결정 기록 (커널 선택, CRDT 선택, PBR vs deferred 등)

---

## 10. 변경 이력

- **2026-05-06 (v3.5, "코퍼스 + ADR 심화 + 첫 실행 가능 시드")** — v3.4 위에 세 갈래 진전. **+ADR 5개**: 0011 automerge CRDT (Kleppmann POPL 2017 인용; vs OT/Yjs/custom/RGA/LSEQ/state-based), 0012 wasm-first 플러그인 샌드박스 (wasmtime + WASI Preview 2 + capability tokens; vs native-only/Lua-only/JS/seccomp/wasmer), 0013 CBOR RFC 8949 메타데이터 (vs JSON/msgpack/BSON/protobuf/FlatBuffers/YAML/TOML), 0014 zstd RFC 8878 압축 (level 3 기본/level 19 아카이브; vs gzip/xz/brotli/LZ4/snappy), 0015 sparse Cholesky 스케치 LM 선형 스텝 (nalgebra-sparse 기본; CHOLMOD opt-in; vs dense LU/dense Cholesky/CG/MINRES/GPU/Eigen). **+perf 명세 1개**: memory-profile.md — 모든 위상·기하 타입 별 힙 비용표, R12 산업 파트 1 GB peak-RSS 예산 분해 (테셀레이션이 최대 비용원), dhat 측정 코드, OS별 RSS (`getrusage` Linux/macOS, `GetProcessMemoryInfo` Windows), 할당자 선택 근거 (system 기본 + jemalloc/mimalloc opt-in), 할당 핫스팟 arena 정책, 5/15/50 % 회귀 게이트. **+골든 6개**: 슬롯, 육각형, 원에 내접 삼각형 (UnderDetermined + 잔여 DoF 명시), 평행선-중복 접선 (OverDetermined + 중복 제약 ID), 박스∩박스, 구∩박스 (sphere face kind 보존), 박스∪박스 idempotent (대수 속성 검증). **+Lua 스크립트 2개**: r1_box.lua (Euler-Poincaré + 부피 + 면적 + 중심 + 태그 완전성 + content-hash), r2_extrude.lua (구멍 있는 추출 + π·r²·h 부피 검증). **+첫 컴파일 가능 Rust 시드**: examples/build_reference_parts.rs — quick_box/cadk::write API가 들어오면 R1-R12를 빌드, 아니면 NotImplemented를 깨끗이 보고. 5-기둥은 이제 ADR 15 + perf 4 + algorithms 13 + 골든 11 + Lua 2 + Rust 시드 1로 받쳐짐. 문서+코퍼스 9,299 줄.
- **2026-05-06 (v3.4, "결정 기록 + 성능 방법론 + 코퍼스 시드")** — v3.3 알고리즘 시리즈를 보완하는 세 개의 신규 문서 기둥 추가. **[`docs/adr/`](adr/README.md)** — Michael Nygard 표준 ADR 형식(Status/Context/Decision/Alternatives/Consequences/References)으로 10개 결정 기록: 0001 half-edge B-Rep (vs winged-edge / quad-edge / IFS / vertex-vertex / GMap), 0002 BLAKE3 캐시 키 (vs SHA-256 / SHA-3 / xxHash3 / FNV / MD5 / Blake2b — $10^9$ 엔트리 충돌 확률 $1.5 \cdot 10^{-21}$ 계산 포함), 0003 nalgebra(커널 f64) + glam(뷰어 f32) 분리, 0004 egui+wgpu+winit GUI 스택 (vs Qt6/GTK4/Slint/iced/Bevy UI/네이티브/Tauri/ImGui), 0005 mlua Lua 5.4 (vs Rhai/Python in-proc/V8/Wasm/커스텀 DSL), 0006 PyO3 별도 크레이트 + 기본 빌드 제외, 0007 태그 기반 영구 명명 (vs PTC Pro/E 위상 명명 문제 + UUID + hash-of-geometry + pointer/arena ID), 0008 Rust edition 2024 + MSRV 1.85, 0009 커스텀 바이너리 `.cadk` (vs JSON/msgpack/sqlite/Arrow-Parquet/HDF5/protobuf-flatbuffers-capnp/OCCT BinXCAFFormat/STEP-as-native), 0010 Rayon 데이터 병렬, 비동기 커널 거부. **[`docs/perf/`](perf/methodology.md)** — 3개 성능 명세: methodology.md (R1-R5 레퍼런스 HW + RUSTFLAGS + Criterion 설정 + cold-vs-warm + RNG 시드 + CPU 격리 + 결정성 강제 + dhat 힙 프로파일 + 16.67ms 프레임 예산 분해 + 5/15/50% 회귀 정책 + PGO 계획 + perf 카운터 + flame graph + 크로스 아키텍처), dispatch-matrix.md (SSI 18행 분석적→NURBS-일반, 불리언 coplanar 전처리, 필렛 5-티어 자동 승급, 스케치 LM↔dogleg 전환, 정밀도 에스컬레이션 f64→interval→exact-rational→Yap, STEP 9행 표면 종류별), condition-numbers.md (알고리즘별 $\kappa$ 안전 한계 — 스케치 LM $10^8$, NURBS SSI $10^7$, sparse Cholesky $10^{10}$ damped 등 — Hager 1-norm 저비용 검출, 거의 특이한 스케치의 worked example, coplanar 일치 면 worked example, `EPSILON_*` 명명된 톨러런스 계층, FMA + 반복 순서 결정성). **[`tests/corpus/`](../tests/corpus/README.md)** — 골든 코퍼스 골격: README + sketch/golden/MANIFEST (12 레퍼런스 스케치) + 3 worked TOML (001 사각형 WellDetermined, 002 동심원, 007 모순 정사각형 Inconsistent + 최소충돌집합 검증) + boolean/golden/MANIFEST (10 케이스) + 2 worked TOML (001 box∪box, 002 box − 내부 box genus-0 캐비티) + reference_parts/MANIFEST (R1-R12 빌드 스크립트 + 파트별 타이밍 예산). 전체 문서 ~5,900 → ~10,000 줄. 5-기둥 구조 확립: **로드맵=계약, algorithms=어떻게, adr=왜 다른 것이 아닌가, perf=얼마나 빠른가 어떻게 측정하는가, corpus=계약 충족 증명**.
- **2026-05-06 (v3.3, "모듈 스펙 시리즈")** — v3.2의 인라인 §16 알고리즘 부록을 [`docs/algorithms/`](algorithms/) 하위 13개 별도 명세 파일로 분리하면서 인라인 때보다 **더 깊게 확장**: [sketch-solver.md](algorithms/sketch-solver.md) (Marquardt damping + Powell-dogleg 폴백 + DR-planning Bouma-Fudos-Hoffmann-Cai-Paige + SVD deflation + QuickXplain 충돌 집합 + 24행 Jacobian + drag 예산), [nurbs-ssi.md](algorithms/nurbs-ssi.md) (Patrikalakis 3-phase + marching ODE + predictor-corrector + Newton projection + Krishnan-Manocha tangent 폴백), [boolean.md](algorithms/boolean.md) (8-phase + Yap 1990 심볼 퍼터베이션 worked example + interval arithmetic + Jacobson winding-number + Cherchi mesh 폴백), [fillet.md](algorithms/fillet.md) (5-tier 사다리 rolling-ball / variable-radius / Choi-Lee setback / Vida-Martin-Várady $G^2$ / conic-Bezier $G^3$), [brep-abi.md](algorithms/brep-abi.md) (HEADER 64B + METADATA CBOR + DOCUMENT + TOPO 7타입 + GEOM 6곡선+9면 + TAGS + HIST + THUMB + EXT + TRAILER 바이트-exact 레이아웃 + v0→v1 마이그레이션 예제), [step-mapping.md](algorithms/step-mapping.md) (24행 AP242 엔티티 매핑 + R1-box worked example + canonicalisation + LOTAR + NIST CAX-IF 절차 + GD&T), [recompute.md](algorithms/recompute.md), [bvh.md](algorithms/bvh.md) (SAH+binning+Kay-Kajiya), [tessellation.md](algorithms/tessellation.md) (Shewchuk Triangle CDT + 60° crease BFS), [persistent-naming.md](algorithms/persistent-naming.md), [test-corpus.md](algorithms/test-corpus.md) (4-tier + 1M boolean fuzz + 290-part STEP + 60 visual baseline), [microbenchmarks.md](algorithms/microbenchmarks.md) (20행 + ±5% 회귀 감지), [chaos-tests.md](algorithms/chaos-tests.md) (12 장애 주입 + loom + allocator hooks). 각 파일은 새 소유자 / 버전 도장 / ISO·표준 정렬 (ISO 10303-242:2020, ISO 10303-21:2016, RFC 8949 CBOR, RFC 8878 zstd, BLAKE3, IEEE 754) / 1차 문헌 (Patrikalakis-Maekawa, Yap, Hoffmann, Choi-Lee, Vida-Martin-Várady, Shewchuk, Wald, Kay-Kajiya, Junker, Bouma-Fudos-Hoffmann-Cai-Paige, Mäntylä, Jacobson-Kavan-Sorkine-Hornung, Cherchi-Pellacini-Attene-Livesu) 선언. 로드맵 §16은 새 시리즈를 가리키는 슬림 TOC + 매니페스트로 교체. 전체 문서량 3,712 줄 → ~5,900 줄, 항상 소유자 경계(kernel-engineer / io-engineer / qa-engineer)로 머지 충돌 방지.
- **2026-05-06 (v3.2, "산업급 심화")** — 이전 버전에서 얇았던 모든 섹션을 Track B 수준 깊이로 확장. **Track A**: A1·A2 각각 Rust struct 정의 풀세트 (Session/Command/Outcome 14+ 변형, ExtrudeSpec/LinearPatternSpec/MirrorSpec, Document::history), undo-redo 결합 시맨틱, JSON 저장/로드, SessionSnapshot, Lua 브릿지 마이그레이션, 테스트 인벤토리 (14 단위 + 6 통합 + 2 proptest + doc + workspace verify), `cargo public-api` 베이스라인 게이트. **Track C**: 13개 서브트랙 모두에 v0.5/v1.0/v2.0/v3.0 페이징 + 티어별 구체 산출물; C-Solid 페이처 스펙 계약 템플릿, 필렛 알고리즘 사다리 (rolling-ball / NURBS-swept-blend / set-back 3-way / G2 / conic-Bezier), 불리언 견고성 (Yap 1990 / 인터벌 / 사후검증), 패턴 인스턴스 오버라이드; C-Sketch 28행 제약 커버리지 표; C-Surface 16행 NURBS 알고리즘 표; C-Assy 10단계 깊이 + configurations + design tables + top-down + 10k+ 컴포넌트; C-Mech kinematic v2 / dynamics+contact+flexible-body MOR v3 + 4-테스트 수용 (Watt 링키지 / Stanford-arm IK / 기어트레인 / 낙하), Featherstone/Mirtich/Anitescu-Potra/Shabana 인용; C-Sheet, C-Weld, C-Mold, C-Draw, C-Sim, C-CAM, C-Render, C-Reverse, C-Subdiv 모두 페이징 (자동 파팅 라인 / conformal cooling, Y14.5-2018 / Y14.41 MBD / 3D PDF PRC / DWG 왕복, 선형 / 비선형+열+접촉 / 플라스틱+CFD-lite+explicit+토폴로지옵트, 2.5/3/5축 + Haas/Fanuc/Heidenhain/Mazak/Siemens 포스트, PBR-IBL / GPU PT+SVGF+ReSTIR / VR + USD, 메시리페어+자동세그먼트+영역별피팅 / B-spline 임의 토폴로지, T-spline 기본 / Class-A 자동 변환). **Track D**: D1 3-플랫폼 레퍼런스 HW (X1 Carbon / M1 Air / Surface Pro 9) + CI 매트릭스 + 40행 op별 예산표 (warm start, R4-R7-R11-R12, 다중 스케일 sketch add/drag/solve, 증분 recompute, R3 tessellation, hover, property commit, 스케일별 undo, .cadk save/load, STEP I/O, 다중 스케일 fps, 16ms 프레임 분해, 메모리 목표); D2 견고성 fuzz 타깃 리스트별 + 코퍼스 크기 + soak + 패닉 감사 + Send+Sync static_assertions + 결정론 크로스플랫폼; D3 17행 테스트 카테고리 매트릭스 + R1-R12 타이밍 예산 + 산업 컴플라이언스 (NIST CAX-IF ≥ 95/99/100 %, ASME Y14.5 200-part, JT 10.7, LinuxCNC, DXF AC1027); D4 11행 플랫폼별 패키징 매트릭스 (AppImage/.deb/.rpm/snap/flatpak 서명 + macOS notarisation entitlements + Authenticode EV + RFC 3161 + Sigstore cosign + 델타-rsync + SLSA L3 + 재현 빌드); D5 14항 문서 (mdBook 11 섹션 + Command별 doc-test + 12 비디오 + AI 쿡북 + ADR + 글로서리 자동 추출 + 다국어 요약); D6 18항 보안 (SECURITY.md + cargo-deny + cargo-vet + cargo-public-api + wasmtime 캐퍼빌리티 + GDPR DPA + CycloneDX 1.5 SBOM + SLSA L3 + Sigstore Rekor + 재현빌드 검증 + 위협 모델 + 연간 펜테스트 + 버그 바운티 + CVE); D7 분기별 stable / 월별 patch / 주간 beta / 야간 nightly / LTS 2년 + SemVer 티어 + 마이그레이션 cadkernel-compat + 릴리스 체크리스트. **Track E**: E1 플러그인 SDK 아키텍처 다이어그램 + Wasm + native + 매니페스트 + 라이프사이클 후크 + 권한 모델 + 서명 워크플로우 + cadkernel-sdk + CLI + 5개 레퍼런스 플러그인 + 디버거; E2 마켓플레이스 제출 워크플로우 + 자동 보안 스캔 + 네이티브 수동 리뷰 + 카테고리 + 80/20 수익 분배 + 인앱 브라우저 + 신고; E3 학술 라이선스 + 클래스룸 키트 + Certified Associate/Professional 시험 + 커뮤니티 갤러리 + 레퍼런스 교과서 + YouTube + Discord + CADKernel Day; E4 4-티어 안정성 보장 + RFC 프로세스 + cadkernel-compat + API v1 5년 약속. **Track F**: F1 CRDT 풀 아키텍처 + automerge 래퍼 + (actor_id, lamport, parent_hash) op 모델 + op 타입별 충돌 해결 정책 + 스냅샷 압축 + GC + 동기화 프로토콜 + 검증 + 3-actor 수렴 수용; F2 자체 호스팅 가능 아키텍처 + S3-compat (MinIO/R2/B2/Wasabi) + OAuth2+OIDC + ACL + revisions/branches/diff/merge + GDPR 데이터 거주지 + 감사 로그; F3 WebRTC SFU (mediasoup/LiveKit) + 활동 피드 + @멘션 + 따라가기 모드 + 동시 편집 경고 + 충돌 UI + 오프라인 표시 + 세션 녹화; F4 PDM 풀 라이프사이클 + ECO + where-used 그래프 + 레퍼런스 검사 + ISO 9001/FDA 21 CFR Part 11 감사 트레일 + 역할 + 잠금 + PLMXML + 다단계 BOM. **§3 R1-R12** 각 레퍼런스 파트가 별도 섹션으로 풀 명세 (geometry, feature script, asserted invariants, build target, memory target, 사용처). **§10 cross-cutting**: §10.4 IP 감사 17행 → 30행 (`Why we need it` / `Fallback if removed` 컬럼 + tokio/axum/proptest/cargo-fuzz/cargo-mutants/pixelmatch/image/naga/ash/metal-rs/dhat-rs/tracing/Featherstone-rs/nalgebra-sparse 추가); §10.6 perf-vs-commercial 6행 → 30행 (v1.0/v2.0/v3.0 컬럼 + R1-R12 + 다중 스케일 sketch drag + 불리언 + STEP I/O + 레이트레이싱 + RAM + 디스크 + undo). 정본 2,162 → ~3,500줄 규모.
- **2026-05-06 (v3.1, "UX deep-dive")** — Track B 재작성: 12 → 20 페이즈, stub 제거. §5.0 정보 아키텍처 (윈도우 셸/도크/멀티모니터/워크벤치 추상화), §5.1 디자인 시스템 (light/dark/HC 컬러 토큰 30+, 타이포 11종, 8 px 그리드, 모션 토큰, 39종 컴포넌트 카탈로그, 380→700 아이콘, 15 커서), §5.2 인터랙션 모델 (마우스 스킴 3종, 글로벌+뷰포트+워크벤치별 단축키, 터치 7제스처, 펜), §5.3 워크벤치별 와이어프레임 (Part/Sketcher/Assembly/Drawing ASCII), §5.4 워크벤치별 단축키 표, B0 파운데이션 리팩터링 추가, B1-B5에 픽셀/ms 단위 명세 추가, B6/B7/B9/B10 stub → 완전 명세 승격, B13 모션/B14 알림/B15 검색/B16 설정/B17 텔레메트리/B18 도면 UX/B19 협업 UX/B20 a11y deep-dive 신설, §5.7 UX 검증 (visual regression / interaction tests / a11y CI / i18n CI / 첫 실행 외부 스터디). 정본 1,435 → 2,163줄.
- **2026-05-06 (v3, "산업급")** — 전면 재작성. 트랙 4 → 6. Non-negotiables 25 → 60. R1-R4 → R1-R12. Track C 13 서브 트랙. 상용 5종 패리티 매트릭스. 알고리즘 참고 문헌. IP 감사. 산업 컴플라이언스. v0.5 → v3.0 캘린더.
- 2026-05-06 (v2) — 4-트랙 재구성.
- 2026-05-06 (v1) — 8-페이즈 단조.
