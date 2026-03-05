<div align="center">

# CADKernel Wiki

**Rust 기반 오픈소스 CAD 커널 — 개발자 가이드**

</div>

---

CADKernel 개발에 필요한 모든 정보를 담은 Wiki입니다.

## 빠른 시작

```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --release
cargo test --workspace
```

## Wiki 구조

### 핵심 문서
| 페이지 | 설명 |
|--------|------|
| [[Architecture]] | 아키텍처 개요, 계층 구조, 설계 결정 |
| [[Getting Started]] | 환경 설정, 빌드, 첫 번째 프로그램 |
| [[Implementation Status]] | Phase 1–17 구현 현황 및 로드맵 |

### 크레이트 가이드
| 페이지 | 크레이트 | 역할 |
|--------|----------|------|
| [[Crate: core]] | `cadkernel-core` | 공유 에러 타입 |
| [[Crate: math]] | `cadkernel-math` | 벡터, 행렬, 변환 |
| [[Crate: geometry]] | `cadkernel-geometry` | 커브, 서피스, 교차 |
| [[Crate: topology]] | `cadkernel-topology` | B-Rep 반변 구조 |
| [[Crate: sketch]] | `cadkernel-sketch` | 2D 스케치 + 솔버 |
| [[Crate: modeling]] | `cadkernel-modeling` | 프리미티브, Boolean, Feature Ops, Fillet/Split/Draft, Mirror/Shell/Scale |
| [[Crate: io]] | `cadkernel-io` | 테셀레이션, STL/OBJ, SVG, JSON, STEP, IGES |
| [[Crate: viewer]] | `cadkernel-viewer` | 네이티브 GUI, 3D 렌더링, 카메라, 내비게이션 |

### 심화 주제
| 페이지 | 설명 |
|--------|------|
| [[Persistent Naming]] | TNP 해결을 위한 Tag 시스템 |
| [[Error Handling]] | KernelError 패턴 및 가이드라인 |
| [[API Cookbook]] | 실전 코드 레시피 모음 |
| [[Glossary]] | CAD/기하 용어 사전 |

---

## 프로젝트 현황

| 항목 | 상태 |
|------|------|
| 에디션 | Rust 2024 (1.85+) |
| 테스트 | 300개+ 통과 |
| Clippy | 경고 0개 |
| 라이선스 | Apache 2.0 |
| 단계 | Application Phase 2 (pre-alpha) |
