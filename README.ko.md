<div align="center">

# CADKernel

**Rust 기반 오픈소스 CAD 소프트웨어**

[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/kernalix7/CADKernel/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/kernalix7/CADKernel/actions/workflows/ci.yml)

*안정적이고, 빠르고, 호환성 높고, 확장 가능한 차세대 오픈소스 CAD 소프트웨어*

[English](README.md) | **한국어**

</div>

---

## 목차

- [프로젝트 소개](#프로젝트-소개)
- [핵심 가치](#핵심-가치)
  - [안정성 (Stability)](#안정성-stability)
  - [성능 (Performance)](#성능-performance)
  - [호환성 (Compatibility)](#호환성-compatibility)
  - [확장성 (Extensibility)](#확장성-extensibility)
- [기술 스택](#기술-스택)
- [타 CAD 소프트웨어와의 비교](#타-cad-소프트웨어와의-비교)
- [아키텍처 개요](#아키텍처-개요)
- [지원 파일 형식](#지원-파일-형식)
- [확장 시스템](#확장-시스템)
- [AI 통합 (MCP)](#ai-통합-mcp)
- [로드맵](#로드맵)
- [빌드 및 설치](#빌드-및-설치)
- [데모](#데모)
- [FAQ](#faq)
- [버전 정책](#버전-정책)
- [변경 이력](#변경-이력)
- [감사의 글](#감사의-글)
- [기여하기](#기여하기)
- [라이선스](#라이선스)

---

## 프로젝트 소개

**CADKernel**은 Rust로 작성된 오픈소스 CAD 소프트웨어입니다. 기존 상용 CAD 소프트웨어의 높은 비용, 플랫폼 종속성, 제한된 확장성 문제를 해결하고자 시작되었습니다.

Rust의 메모리 안전성과 제로 코스트 추상화를 활용하여, 크래시 없는 안정적인 CAD 환경을 제공하면서도 네이티브에 가까운 최상의 성능을 달성하는 것이 목표입니다. 또한, 다양한 CAD 파일 형식과의 호환성, 플러그인 기반의 확장 시스템, 그리고 생성형 AI와의 통합을 통해 누구나 자유롭게 사용하고 확장할 수 있는 CAD 생태계를 구축합니다.

### 왜 CADKernel인가?

- **비용 장벽 제거** — 무료 오픈소스로 누구나 사용 가능
- **플랫폼 자유** — Windows, macOS, Linux 모두 지원
- **커뮤니티 중심** — 사용자와 개발자가 함께 만들어가는 소프트웨어
- **미래 지향** — AI 시대에 맞는 설계 도구

---

## 핵심 가치

### 안정성 (Stability)

> CAD 소프트웨어에서 안정성은 타협할 수 없는 최우선 가치입니다.

CAD 작업은 수시간에 걸친 정밀한 설계가 수반되며, 예기치 않은 크래시는 막대한 작업 손실로 이어집니다. CADKernel은 Rust의 언어적 특성을 최대한 활용하여 안정성을 보장합니다.

- **메모리 안전성** — Rust의 소유권(Ownership) 시스템과 빌림 검사(Borrow Checker)를 통해 메모리 관련 버그를 컴파일 타임에 원천 차단
- **스레드 안전성** — 데이터 레이스가 불가능한 동시성 모델을 통해 멀티스레드 환경에서도 안정적 동작 보장
- **강력한 에러 처리** — `Result`/`Option` 타입 기반의 명시적 에러 핸들링으로 예외 상황을 빠짐없이 처리
- **자동 저장 및 복구** — 주기적 자동 저장과 크래시 복구 메커니즘을 내장하여 데이터 손실 최소화
- **광범위한 테스트** — 단위 테스트, 통합 테스트, 퍼징(Fuzzing) 테스트를 통한 지속적 품질 보증
- **기하학적 견고성** — 부동소수점 연산의 한계를 고려한 견고한(Robust) 기하 연산 엔진 구현

### 성능 (Performance)

> 복잡한 3D 모델도 실시간으로 다룰 수 있는 최상의 성능을 추구합니다.

CAD 소프트웨어는 수십만 개 이상의 기하학적 요소를 실시간으로 처리해야 합니다. CADKernel은 Rust의 제로 코스트 추상화와 최적화된 알고리즘으로 C/C++ 수준의 성능을 달성합니다.

- **제로 코스트 추상화** — 고수준의 추상화를 사용하면서도 런타임 오버헤드 없는 코드 생성
- **병렬 연산** — Rayon 기반 데이터 병렬 처리로 멀티코어 CPU 활용 극대화
- **GPU 가속** — wgpu를 활용한 크로스 플랫폼 GPU 연산 및 렌더링 파이프라인
- **지연 평가 (Lazy Evaluation)** — 필요한 시점에만 연산을 수행하여 불필요한 계산 제거
- **효율적 메모리 관리** — 가비지 컬렉터 없이 결정적(Deterministic) 메모리 관리로 예측 가능한 성능 제공
- **캐싱 전략** — 반복되는 기하 연산 결과를 지능적으로 캐싱하여 재계산 비용 절감
- **공간 인덱싱** — BVH, R-Tree 등의 공간 데이터 구조를 활용한 효율적 공간 탐색

### 호환성 (Compatibility)

> 기존 CAD 생태계와의 높은 호환성, 그리고 어디서든 동작하는 크로스 플랫폼 지원을 제공합니다.

CAD 작업은 단독으로 이뤄지지 않습니다. 다양한 팀원, 협력사, 도구와의 데이터 교환이 필수적입니다. CADKernel은 플랫폼 종속성을 최소화하고, 주요 CAD 파일 형식을 폭넓게 지원합니다.

- **크로스 플랫폼** — Windows, macOS, Linux에서 동일한 경험 제공. 플랫폼별 렌더링 백엔드 자동 선택 (Vulkan / Metal / DX12 / OpenGL)
- **60개 이상의 파일 형식** — 8개 카테고리에 걸친 포괄적 지원: 산업 표준 (STEP, IGES, Parasolid, ACIS, JT, IFC), 상용 CAD (SolidWorks, CATIA, Creo, Inventor, Fusion 360, Rhino, SketchUp, AutoCAD), 메시/시각화 (glTF, FBX, USD, COLLADA), 3D 프린팅 (3MF, AMF, G-code), 2D 도면 (SVG, PDF, DXF/DWG), 포인트 클라우드 (PCD, LAS, E57) 등
- **상용 CAD 임포트** — SolidWorks, CATIA V5, PTC Creo, Autodesk Inventor, Fusion 360 등 독점 포맷의 읽기 전용 지원
- **포인트 클라우드 및 스캔 데이터** — LiDAR (LAS/LAZ), 3D 스캔 (E57), 원시 포인트 클라우드 (PCD, XYZ) 데이터 임포트/익스포트
- **유니코드 완전 지원** — 파일 경로, 레이어 이름, 주석 등 모든 텍스트 요소에서 다국어 완전 지원
- **네이티브 형식** — 자체 `.cadk` 형식으로 모든 기능과 메타데이터를 무손실 저장

### 확장성 (Extensibility)

> 사용자가 필요로 하는 기능을 직접 만들고, 공유하고, 통합할 수 있는 열린 생태계를 지향합니다.

CAD 소프트웨어의 요구사항은 분야마다 다릅니다. 건축, 기계, 전자, 산업디자인 등 각 영역의 전문적인 니즈를 하나의 코어에 모두 담을 수는 없습니다. CADKernel은 강력한 확장 시스템으로 이를 해결합니다.

- **Add-on 시스템** — 플러그인 API를 통해 사용자가 직접 기능 개발 및 배포 가능
- **공식 병합 경로** — 커뮤니티에서 검증된 우수 Add-on은 정식 버전에 기능으로 병합
- **스크립팅 지원** — Lua/Python 스크립팅 인터페이스로 반복 작업 자동화
- **MCP (Model Context Protocol) 지원** — 생성형 AI(LLM)와의 표준화된 통합 인터페이스 제공
- **파라메트릭 설계** — 매개변수 기반 모델링과 제약 조건(Constraint) 시스템으로 유연한 설계 변경
- **커스텀 렌더러** — 렌더링 파이프라인 확장을 통한 사용자 정의 시각화

---

## 기술 스택

CADKernel은 엄선된 Rust 크레이트와 기술로 구성됩니다:

| 카테고리 | 기술 | 용도 |
|----------|------|------|
| **언어** | Rust 1.85+ | 메모리 안전성, 성능, 걸림 없는 동시성 |
| **GPU / 렌더링** | wgpu | 크로스 플랫폼 GPU API (Vulkan / Metal / DX12 / OpenGL) |
| **병렬리즘** | Rayon | 멀티코어 CPU를 위한 데이터 병렬 연산 |
| **수학** | nalgebra, glam | 선형대수, 벡터, 행렬, 변환 |
| **기하** | 자체 B-Rep / NURBS 엔진 | 경계 표현 및 자유곡면 모델링 |
| **공간 인덱스** | bvh, rstar | BVH와 R-Tree 기반 효율적 공간 탐색 |
| **GUI** | egui 0.31 + winit 0.30 | 크로스 플랫폼 immediate 모드 GUI (네이티브 데스크톱) |
| **스크립팅** | mlua, PyO3 | Lua 및 Python 스크립팅 바인딩 |
| **직렬화** | serde, bincode | 고성능 데이터 직렬화 |
| **파일 I/O** | iso-10303 *(계획)* | STEP 파일 파싱 및 쓰기 |
| **AI / MCP** | JSON-RPC, tower | 생성형 AI 통합을 위한 MCP 서버 |
| **테스팅** | cargo-fuzz, proptest | 퍼징 테스트 및 속성 기반 테스트 |

---

## 타 CAD 소프트웨어와의 비교

| 기능 | **CADKernel** | FreeCAD | OpenSCAD | BRL-CAD | LibreCAD |
|------|:------------:|:-------:|:--------:|:-------:|:--------:|
| 언어 | Rust | C++ / Python | C++ | C / Tcl | C++ |
| 3D 모델링 | 🚧 | ✅ | ✅ | ✅ | ❌ (2D 전용) |
| 파라메트릭 설계 | 🚧 | ✅ | ✅ (코드) | ✅ | ❌ |
| B-Rep + NURBS | 🚧 | ✅ (OCCT) | ❌ (CSG) | ✅ | ❌ |
| GUI | ✅ | ✅ | 제한적 | ✅ | ✅ |
| 플러그인 시스템 | 🚧 | ✅ (Python) | ❌ | ❌ | ❌ |
| STEP 지원 | 🚧 | ✅ | ❌ | ✅ | ❌ |
| 60+ 파일 형식 | 🚧 | 부분적 | ❌ | 부분적 | ❌ |
| GPU 렌더링 | ✅ (wgpu) | 부분적 | OpenGL | OpenGL | ❌ |
| 메모리 안전성 | ✅ (Rust) | ❌ | ❌ | ❌ | ❌ |
| AI / MCP | 🚧 | ❌ | ❌ | ❌ | ❌ |
| 크로스 플랫폼 | 🚧 (목표: Win/Mac/Linux) | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux | Win/Mac/Linux |
| 라이선스 | Apache 2.0 | LGPL 2.1 | GPL 2 | LGPL 2.1 | GPL 2 |

> 참고: CADKernel 상태 표기는 현재 **pre-alpha** 구현 상태를 기준으로 합니다.

---

## 아키텍처 개요

CADKernel은 계층화된 모듈 아키텍처를 채택하여 각 레이어의 독립성과 교체 가능성을 보장합니다.

```
┌───────────────────────────────────────────────────────────┐
│                     Application Layer                     │
│         (GUI · CLI · Scripting · AI/MCP Interface)        │
├───────────────────────────────────────────────────────────┤
│                      Extension Layer                      │
│          (Add-on Manager · Plugin API · Registry)         │
├───────────────────────────────────────────────────────────┤
│                       Service Layer                       │
│      (Modeling · Rendering · I/O · Constraint · Undo)     │
├───────────────────────────────────────────────────────────┤
│                     Core Kernel Layer                     │
│  (Geometry Engine · Topology · Spatial Index · Math Lib)  │
├───────────────────────────────────────────────────────────┤
│                    Platform Abstraction                   │
│          (Window · GPU · FileSystem · Threading)          │
└───────────────────────────────────────────────────────────┘
```

| 레이어 | 역할 | 주요 크레이트 |
|--------|------|--------------|
| **Core Kernel** | 기하 연산, 위상(Topology), 수학 라이브러리 | `cadkernel-core`, `cadkernel-math` |
| **Service** | 모델링 오퍼레이션, 렌더링, 파일 I/O | `cadkernel-modeling`, `cadkernel-viewer`, `cadkernel-io` |
| **Extension** | 플러그인 로드/관리, API 노출 | `cadkernel-extension` (예정) |
| **Application** | GUI, CLI, 스크립팅, AI 연동 | `cadkernel-viewer`, `cadkernel-python` |
| **Platform** | OS 및 하드웨어 추상화 | `cadkernel-platform` (예정) |

---

## 지원 파일 형식

### 네이티브 (Native)

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| CADKernel | `.cadk` | 🔲 | 🔲 | 무손실 네이티브 형식 |

### 산업 표준 (중립 교환 형식)

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| STEP AP203 | `.step`, `.stp` | 🚧 | 🚧 | 형상 교환 표준 |
| STEP AP214 | `.step`, `.stp` | 🚧 | 🚧 | 자동차 산업 표준 (부분 지원) |
| STEP AP242 | `.step`, `.stp` | 🔲 | 🔲 | PMI/GD&T 포함 |
| IGES | `.iges`, `.igs` | 🚧 | 🚧 | 레거시 교환 형식 (부분 지원) |
| Parasolid | `.x_t`, `.x_b` | 🔲 | 🔲 | Siemens Parasolid 커널 |
| ACIS SAT/SAB | `.sat`, `.sab` | 🔲 | 🔲 | Spatial ACIS 커널 |
| JT | `.jt` | 🔲 | 🔲 | Siemens 경량 시각화 형식 |
| IFC | `.ifc` | 🔲 | 🔲 | BIM / 건축 (ISO 16739) |
| BREP | `.brep`, `.brp` | 🔲 | 🔲 | OpenCASCADE 경계 표현 |

### 상용 CAD (3D)

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| DWG | `.dwg` | 🔲 | 🔲 | AutoCAD 네이티브 |
| DXF | `.dxf` | 🔲 | 🔲 | AutoCAD 교환 형식 |
| 3DM | `.3dm` | 🔲 | 🔲 | Rhino / OpenNURBS |
| FCStd | `.fcstd` | 🔲 | 🔲 | FreeCAD |
| SLDPRT / SLDASM | `.sldprt`, `.sldasm` | 🔲 | — | SolidWorks 파트 / 어셈블리 |
| IPT / IAM | `.ipt`, `.iam` | 🔲 | — | Autodesk Inventor |
| CATPART / CATPRODUCT | `.catpart`, `.catproduct` | 🔲 | — | Dassault CATIA V5 |
| PRT / ASM | `.prt`, `.asm` | 🔲 | — | PTC Creo (Pro/E) |
| F3D | `.f3d` | 🔲 | — | Autodesk Fusion 360 |
| DGN | `.dgn` | 🔲 | 🔲 | Bentley MicroStation |
| SKP | `.skp` | 🔲 | 🔲 | Trimble SketchUp |
| 3DS | `.3ds` | 🔲 | 🔲 | Autodesk 3ds Max (레거시) |
| BLEND | `.blend` | 🔲 | — | Blender (임포트 전용) |

### 2D 도면 및 벡터

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| SVG | `.svg` | 🔲 | ✅ | 스케일러블 벡터 그래픽스 |
| PDF | `.pdf` | 🔲 | 🔲 | 2D 도면 / 3D PDF 내보내기 |
| EPS | `.eps` | 🔲 | 🔲 | Encapsulated PostScript |
| HPGL | `.plt`, `.hpgl` | 🔲 | 🔲 | 플로터 출력 형식 |

### 메시 및 시각화

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| STL | `.stl` | ✅ | ✅ | 3D 프린팅 표준 (ASCII/Binary) |
| OBJ | `.obj` | ✅ | ✅ | Wavefront 메시 형식 |
| JSON | `.json` | ✅ | ✅ | BRepModel 직렬화 (serde) |
| glTF / GLB | `.gltf`, `.glb` | 🔲 | ✅ | 웹 3D 표준 (Khronos) |
| FBX | `.fbx` | 🔲 | 🔲 | Autodesk 교환 형식 |
| COLLADA | `.dae` | 🔲 | 🔲 | XML 기반 3D 교환 |
| PLY | `.ply` | 🔲 | 🔲 | Polygon / Stanford 형식 |
| OFF | `.off` | 🔲 | 🔲 | Object File Format |
| VRML | `.wrl` | 🔲 | 🔲 | 가상현실 모델링 언어 |
| X3D | `.x3d` | 🔲 | 🔲 | VRML 후속 (ISO/IEC 19775) |
| USD / USDA / USDC | `.usd`, `.usda`, `.usdc` | 🔲 | 🔲 | Pixar Universal Scene Description |

### 3D 프린팅 및 제조

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| 3MF | `.3mf` | 🔲 | 🔲 | 차세대 3D 프린팅 (3MF Consortium) |
| AMF | `.amf` | 🔲 | 🔲 | 적층 제조 파일 (ISO/ASTM 52915) |
| G-code | `.gcode`, `.nc` | — | 🔲 | CNC / 3D 프린터 툴패스 |
| SLC | `.slc` | 🔲 | 🔲 | 광조형 컨투어 |

### 포인트 클라우드 및 스캔 데이터

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| PCD | `.pcd` | 🔲 | 🔲 | Point Cloud Library 형식 |
| LAS / LAZ | `.las`, `.laz` | 🔲 | 🔲 | LiDAR 데이터 (ASPRS) |
| E57 | `.e57` | 🔲 | 🔲 | 3D 스캔 데이터 (ASTM E2807) |
| XYZ / PTS | `.xyz`, `.pts` | 🔲 | 🔲 | ASCII 포인트 클라우드 |
| PLY | `.ply` | 🔲 | 🔲 | 포인트 클라우드 변형 |

### 이미지 및 텍스처

| 형식 | 확장자 | 읽기 | 쓰기 | 비고 |
|------|--------|:----:|:----:|------|
| PNG | `.png` | 🔲 | 🔲 | 렌더 / 텍스처 내보내기 |
| JPEG | `.jpg`, `.jpeg` | 🔲 | 🔲 | 텍스처 가져오기 |
| HDR / EXR | `.hdr`, `.exr` | 🔲 | 🔲 | HDR 환경 맵 |
| BMP | `.bmp` | 🔲 | 🔲 | 비트맵 이미지 |
| TIFF | `.tif`, `.tiff` | 🔲 | 🔲 | 고품질 이미지 내보내기 |

> 🔲 = 계획됨 · ✅ = 지원됨 · 🚧 = 개발 중 · — = 해당 없음

---

## 확장 시스템

CADKernel의 확장 시스템은 **개발 → 공유 → 검증 → 통합**의 선순환적 생태계를 목표로 합니다.

```
사용자 Add-on 개발
        │
        ▼
  커뮤니티 공유 및 사용
        │
        ▼
  품질 검증 및 리뷰
        │
        ▼
  정식 버전 기능 병합 ◀── 코어 팀 승인
```

### Add-on 개발

- **Plugin API** — 안정적인 버전 관리를 가진 공개 API를 통해 Add-on 개발
- **샌드박스 실행** — Add-on은 격리된 환경에서 실행되어 코어 시스템의 안정성에 영향 없음
- **핫 리로드** — 개발 시 재시작 없이 Add-on 변경사항 즉시 반영

### 공식 기능 병합

커뮤니티에서 개발된 Add-on 중 아래 기준을 충족하는 경우 정식 기능으로 병합됩니다:

1. 충분한 사용자 기반과 긍정적 피드백
2. 코드 품질 기준 통과 (테스트 커버리지, 문서화, 코드 리뷰)
3. 코어 아키텍처와의 정합성
4. 라이선스 호환성 (Apache 2.0)

---

## AI 통합 (MCP)

CADKernel은 **MCP (Model Context Protocol)** 를 지원하여 생성형 AI와의 원활한 통합을 제공합니다.

```
┌──────────────┐     MCP      ┌──────────────────┐
│  AI / LLM    │◄────────────►│  CADKernel       │
│  (Client)    │  (JSON-RPC)  │  (MCP Server)    │
└──────────────┘              └──────────────────┘
```

### MCP를 통해 가능한 것들

- **자연어 → 3D 모델** — "직경 50mm, 높이 100mm의 원기둥을 만들어 줘" 같은 자연어 명령을 3D 모델로 변환
- **설계 어시스턴트** — AI가 설계 의도를 이해하고 최적의 모델링 접근법 제안
- **자동화된 설계 검증** — AI가 설계 규칙 위반, 제조 가능성 등을 자동으로 검토
- **파라메트릭 최적화** — 주어진 조건하에서 최적의 파라미터 조합을 AI가 탐색
- **문서 자동 생성** — 설계 데이터로부터 도면, BOM, 보고서 자동 생성

### MCP 지원 도구 (Tools)

| 도구 | 설명 |
|------|------|
| `create_geometry` | 기하 요소 생성 (점, 선, 면, 솔리드) |
| `transform` | 이동, 회전, 스케일 등 변환 연산 |
| `boolean_operation` | 합집합, 차집합, 교집합 불리언 연산 |
| `query_model` | 모델 속성 조회 (체적, 면적, 질량 등) |
| `export_model` | 지정 형식으로 모델 내보내기 |
| `apply_constraint` | 치수 및 기하 제약 조건 적용 |
| `undo` / `redo` | 작업 이력 관리 |

---

## 로드맵

### ~~Kernel Phase 1 — 기반 구축~~ ✅
- [x] Cargo workspace (7 크레이트 모노레포)
- [x] 코어 수학 라이브러리 (Vec2/3/4, Point2/3, Mat3/4, Transform, Quaternion, BoundingBox)
- [x] 기하 엔진 (B-Rep, NURBS 커브/서피스, 교차 연산)
- [x] 위상 구조 (Half-edge, EntityStore, Handle, Wire)
- [x] CLI 버전 배너
- [x] GitHub Actions CI

### ~~Kernel Phase 2 — Persistent Naming + Boolean~~ ✅
- [x] Persistent Naming (Tag, NameMap, ShapeHistory, OperationId)
- [x] Geometry-Topology 바인딩 (feature flag)
- [x] 불리언 연산 (Union, Subtract, Intersect)
- [x] SSI (Surface-Surface Intersection) 4종 + LSI (Line-Surface) 3종

### ~~Kernel Phase 3 — Parametric + Sketch + I/O~~ ✅
- [x] 2D 파라메트릭 스케치 (14개 제약 조건 + Newton-Raphson 솔버)
- [x] Feature Ops: Extrude, Revolve (auto-tagging)
- [x] 프리미티브: Box, Cylinder, Sphere
- [x] 테셀레이션 → STL (ASCII/Binary) + OBJ 내보내기

### ~~Kernel Phase 4 — Core Hardening~~ ✅
- [x] `cadkernel-core` 독립 에러 크레이트
- [x] 전체 공개 API panic 경로 제거 → KernelResult
- [x] Send + Sync (Curve/Surface 스레드 안전성)
- [x] Math 타입 표준 trait (Default, Display, From, 연산자)
- [x] EntityStore O(1) len + 안전성 가드

### ~~Kernel Phase 5 — Mass Properties + Sweep~~ ✅
- [x] MassProperties (부피, 면적, 무게중심 — 발산 정리)
- [x] Sweep 연산 (프로파일 × 경로 → 솔리드, RMF 기반)

### ~~Kernel Phase 6 — Loft + Pattern~~ ✅
- [x] Loft 연산 (N개 단면 보간 → 솔리드)
- [x] Linear Pattern (방향 + 간격 반복 복사)
- [x] Circular Pattern (축 회전 반복 복사)

### ~~Kernel Phase 7 — Chamfer + I/O Import~~ ✅
- [x] Chamfer 연산 (모서리 면취)
- [x] STL Import (ASCII + Binary 자동 감지)
- [x] OBJ Import (v/vt/vn, N-gon 삼각화)

### ~~Kernel Phase 8 — Modeling Enhancements~~ ✅
- [x] Mirror 연산 (평면 반사 복사)
- [x] Shell 연산 (박벽 중공 솔리드)
- [x] 비균일 Scale 연산
- [x] `copy_solid_with_transform` 공유 유틸리티

### ~~Kernel Phase 9 — Math & Geometry Enhancements~~ ✅
- [x] 수학 유틸리티 함수 11개 (거리, 각도, 투영, 보간, 면적)
- [x] Plane 향상 (from_three_points, signed_distance, project_point 등)
- [x] BoundingBox 향상 (overlaps, expand, volume, surface_area, longest_axis, size)

### ~~Kernel Phase 10 — Quality & Testing~~ ✅
- [x] E2E 통합 테스트 10개 (전체 파이프라인 라운드트립)
- [x] B-Rep 검증: 댕글링 참조 감지, 방향 일관성 체크
- [x] 새 검증 API: validate_manifold(), validate_detailed()

### ~~Kernel Phase 11 — I/O Format Expansion~~ ✅
- [x] SVG 2D 내보내기 (SvgDocument, 5가지 요소 타입, auto-fit viewBox)
- [x] JSON 직렬화 (BRepModel ↔ JSON 라운드트립, 파일 I/O)
- [x] 모든 토폴로지/수학 타입에 serde Serialize/Deserialize

### ~~Kernel Phase 12 — Rustdoc Documentation~~ ✅
- [x] 모든 `pub` 항목에 문서 주석 (`///`)
- [x] 크레이트 수준 문서 (`//!`)
- [x] 문서 내 예제 코드 블록
- [x] Doc 테스트 컴파일 검증

### ~~Kernel Phase 13 — 고우선순위 기능~~ ✅
- [x] Fillet 연산 (호 근사 기반 모서리 라운딩)
- [x] 솔리드 분할 (평면 기반 솔리드 이분할)
- [x] 점-솔리드 포함 판정 (레이캐스팅 기반)

### ~~Kernel Phase 14 — 기하 & 제조~~ ✅
- [x] 2D 커브 오프셋 (폴리라인 & 폴리곤 평행 오프셋)
- [x] 구배 각도 (중립면 기반 금형 테이퍼)
- [x] 적응형 테셀레이션 (현오차 & 각도 기반 세분화)

### ~~Kernel Phase 15 — 인프라~~ ✅
- [x] Undo/Redo 시스템 (ModelHistory, 스냅샷 기반)
- [x] 속성 시스템 (Color, Material 프리셋, PropertyStore)
- [x] 최근접점 쿼리 (Voronoi 영역 삼각형 투영)

### ~~Kernel Phase 16 — 산업 형식~~ ✅
- [x] STEP I/O (ISO 10303-21 AP214 부분 지원 — StepWriter, 읽기/파싱/내보내기)
- [x] IGES I/O (IGES 5.3 — IgesWriter, 점/선 엔티티 교환)

### ~~Kernel Phase 17 — 품질 & 고급 기능~~ ✅
- [x] 벤치마크 스위트 (14개 criterion 벤치마크)
- [x] NURBS 고급 기능 (Boehm 노트 삽입, 차수 승격)
- [x] 스레드 안전성 (전체 크레이트 컴파일 타임 Send+Sync 어서션)

### ~~Application Phase 1 — 네이티브 GUI 애플리케이션~~ ✅
- [x] wgpu 렌더링 파이프라인 (Solid, Wireframe, Transparent, Flat Lines 디스플레이 모드)
- [x] egui 기반 네이티브 데스크톱 GUI (메뉴바, 모델 트리, 속성 패널, 상태 바)
- [x] 카메라 시스템 (궤도 회전, 팬, 줌, 투시/직교, 표준 뷰 프리셋)
- [x] 마우스 내비게이션 설정 (FreeCAD Gesture, Blender, SolidWorks, Inventor, OpenCascade)
- [x] 동적 그리드 오버레이 (줌 자동 스케일링) + XYZ 원점 축
- [x] glTF 2.0 내보내기
- [x] 멀티스레드 I/O (rayon 병렬화)
- [x] Python 바인딩 (PyO3)

### ~~Application Phase 2 — ViewCube & 카메라~~ ✅
- [x] ViewCube: 절두 큐브 — 면/엣지/코너 클릭 뷰 스냅 (26 방향)
- [x] ViewCube: 방향 조명, 드롭 섀도, 오비트 링 + 나침반 레이블
- [x] ViewCube: CW/CCW 인플레인 롤 버튼, 스크린 스페이스 화살표 버튼
- [x] 카메라 롤 시스템 (시점 축 기준 인플레인 회전, 뷰 스냅 시 자동 리셋)
- [x] 카메라 애니메이션 시스템 (smooth-step 이징, 최단 경로 yaw 보간)
- [x] 뷰 애니메이션 설정 (활성화/비활성화 토글, 지속 시간 슬라이더)
- [x] 45도 오비트 스텝, 미니 축 인디케이터 음방향 페이드 라인

### Application Phase 3 — 호환성
- [ ] DXF/DWG, 3DM 임포트/익스포트
- [ ] Parasolid, ACIS, JT, IFC 임포트/익스포트
- [ ] 상용 CAD (SolidWorks, CATIA, Creo, Inventor, Fusion 360) 임포트

### Application Phase 4 — 확장 생태계
- [ ] Plugin API + Add-on 매니저
- [ ] Lua/Python 스크립팅
- [ ] MCP (AI 통합) 서버
- [ ] 커뮤니티 마켓플레이스

---

## 빌드 및 설치

### 요구사항

- **Rust** 1.85 이상 ([rustup](https://rustup.rs/)을 통해 설치)
- **GPU 드라이버** — Vulkan, Metal, 또는 DX12 지원
- **CMake** 3.16+ (일부 네이티브 의존성용)
- **Python** 3.10+ *(선택사항, 스크립팅 지원용)*

### 플랫폼별 사전 요구사항

<details>
<summary><b>Linux (Ubuntu / Debian)</b></summary>

```bash
# 시스템 의존성 설치
sudo apt update
sudo apt install -y build-essential cmake pkg-config \
  libx11-dev libxkbcommon-dev libwayland-dev \
  libvulkan-dev mesa-vulkan-drivers

# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
</details>

<details>
<summary><b>macOS</b></summary>

```bash
# Xcode 커맨드 라인 도구 설치
xcode-select --install

# Homebrew로 의존성 설치
brew install cmake

# Rust 설치
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
> macOS는 자동으로 Metal을 렌더링 백엔드로 사용합니다.
</details>

<details>
<summary><b>Windows</b></summary>

1. [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) 설치 (C++ 워크로드)
2. [CMake](https://cmake.org/download/) 설치
3. [rustup](https://rustup.rs/)을 통해 Rust 설치
4. 최신 GPU 드라이버 확인 (Vulkan 또는 DX12)
</details>

### 소스에서 빌드

```bash
# 저장소 클론
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel

# 빌드
cargo build --release

# 실행
cargo run --release
```

### 테스트

```bash
# 전체 테스트
cargo test

# 벤치마크
cargo bench

# 퍼징 테스트 (nightly 필요)
cargo +nightly fuzz run geometry_fuzz
```

---

## 데모

### GUI 애플리케이션 실행

```bash
cargo run --release --bin cadkernel
```

**사용 가능한 기능:**
- File → Open으로 STL/OBJ 파일 열기
- Create 메뉴로 프리미티브 생성 (Box, Cylinder, Sphere)
- 디스플레이 모드 전환: D 키 또는 View → Display Mode
- 표준 뷰: 1/3/7 키 (Front/Right/Top), Ctrl+1/3/7 (Back/Left/Bottom), 0 (Isometric)
- 그리드 토글: G 키
- 투영 전환: 5 키
- 모델에 맞춤: V 키
- 마우스 내비게이션은 FreeCAD Gesture 프리셋이 기본값 (Settings에서 변경 가능)

---

## FAQ

### CADKernel은 이미 실무(프로덕션) 사용 가능한가요?

아직은 아닙니다. 현재는 적극 개발 중인 pre-alpha 단계입니다.

### 어떤 플랫폼을 지원하나요?

Windows, macOS, Linux를 목표 플랫폼으로 지원합니다.

### 상용 CAD 파일 임포트를 지원하나요?

계획되어 있습니다. 독점 포맷 지원은 로드맵과 호환성 매트릭스에 구현 목표로 명시되어 있습니다.

### 반복 작업 자동화가 가능한가요?

가능합니다. Lua/Python 스크립팅과 MCP 기반 AI 통합을 핵심 확장 목표로 두고 있습니다.

---

## 버전 정책

CADKernel은 [Semantic Versioning](https://semver.org/lang/ko/) (`MAJOR.MINOR.PATCH`)을 따릅니다.

- `MAJOR` — 호환되지 않는 API/포맷 변경
- `MINOR` — 하위 호환 기능 추가
- `PATCH` — 하위 호환 버그 수정

---

## 변경 이력

프로젝트 변경 사항은 [CHANGELOG.ko.md](CHANGELOG.ko.md)에서 관리합니다.

---

## 감사의 글

CADKernel은 오픈소스 생태계 위에서 성장합니다. 특히 아래 커뮤니티에 감사드립니다:

- Rust 및 Cargo 생태계
- wgpu 및 그래픽스 인프라 프로젝트
- 기하 연산/CAD 상호운용성 표준 커뮤니티

---

## 기여하기

CADKernel은 커뮤니티의 기여를 환영합니다.

전체 기여 절차와 체크리스트는 [CONTRIBUTING.ko.md](CONTRIBUTING.ko.md)를 참고해 주세요.

빠른 시작:

1. 이 저장소를 **Fork** 합니다.
2. 새로운 브랜치를 생성합니다: `git checkout -b feature/amazing-feature`
3. 변경사항을 커밋합니다: `git commit -m 'feat: add amazing feature'`
4. 브랜치에 Push 합니다: `git push origin feature/amazing-feature`
5. **Pull Request**를 생성합니다.

### 기여 가이드라인

- [Conventional Commits](https://www.conventionalcommits.org/) 스타일의 커밋 메시지 사용
- 새로운 기능에는 반드시 테스트 코드 포함
- `cargo fmt` 및 `cargo clippy`로 코드 스타일 검증
- 문서화 — 공개 API에는 반드시 문서 주석(`///`) 작성

### 보안 정책

보안 취약점을 발견한 경우, 공개 이슈 대신 [GitHub Security Advisories](https://github.com/kernalix7/CADKernel/security/advisories/new)를 통해 책임감 있게 보고해 주세요. 자세한 내용은 [SECURITY.ko.md](SECURITY.ko.md)를 참고하세요.

### 행동 강령

이 프로젝트는 [Contributor Covenant 행동 강령](CODE_OF_CONDUCT.ko.md)을 따릅니다. 참여함으로써 이 규정을 준수하는 것에 동의하는 것으로 간주됩니다. 부적절한 행동은 프로젝트 메인테이너에게 보고해 주세요.

---

## 라이선스

이 프로젝트는 [Apache License 2.0](LICENSE)에 따라 배포됩니다.

```
Copyright 2026 Kim DaeHyun (kernalix7@kodenet.io)

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0
```

---

<div align="center">

**CADKernel** — *모두를 위한 오픈소스 CAD*

[이슈 리포트](https://github.com/kernalix7/CADKernel/issues) · [기능 제안](https://github.com/kernalix7/CADKernel/issues) · [디스커션](https://github.com/kernalix7/CADKernel/discussions)

</div>
