# Glossary

CAD 커널 개발에 사용되는 주요 용어 정리입니다.

---

## B-Rep (Boundary Representation)

| 용어 | 영문 | 설명 |
|------|------|------|
| 꼭짓점 | Vertex | 3D 공간의 점. 모서리의 끝점 |
| 모서리 | Edge | 두 꼭짓점을 잇는 1D 경계. 기하학적으로 커브에 바인딩 |
| 반변 | Half-Edge | 모서리의 방향 있는 절반. 인접 관계 저장의 핵심 |
| 쌍둥이 | Twin | 같은 모서리의 반대 방향 반변 |
| 루프 | Loop | 반변의 닫힌 순환. 면의 외곽 또는 구멍 경계 |
| 와이어 | Wire | 반변의 순서 있는 체인 (닫히지 않을 수 있음) |
| 면 | Face | 2D 영역. 외부 루프 1개 + 내부 루프(구멍) N개 |
| 셸 | Shell | 면의 연결된 집합 |
| 솔리드 | Solid | 닫힌 셸로 둘러싸인 부피 |
| 오리엔테이션 | Orientation | 면의 법선 방향 (Forward/Reversed) |

## 기하 (Geometry)

| 용어 | 영문 | 설명 |
|------|------|------|
| 커브 | Curve | 1D 매개변수 경로. `point_at(t) → Point3` |
| 서피스 | Surface | 2D 매개변수 면. `point_at(u, v) → Point3` |
| 도메인 | Domain | 매개변수 유효 범위. 예: 원은 `[0, 2π]` |
| 접선 | Tangent | 커브 위의 점에서의 방향 벡터 |
| 법선 | Normal | 서피스 위의 점에서의 수직 벡터 |
| NURBS | Non-Uniform Rational B-Spline | 가중 제어점 기반 자유곡선/자유곡면 |
| 곡률 | Curvature | 커브가 얼마나 휘어있는지의 척도. `1/R` |
| 바운딩 박스 | Bounding Box | 축 정렬 최소 외접 직육면체 (AABB) |
| 테셀레이션 | Tessellation | 연속 곡면 → 삼각형 메시 변환 |

## 연산 (Operations)

| 용어 | 영문 | 설명 |
|------|------|------|
| 돌출 | Extrude | 2D 프로파일을 직선 방향으로 밀어내어 솔리드 생성 |
| 회전 | Revolve | 2D 프로파일을 축 주위로 회전하여 솔리드 생성 |
| 스윕 | Sweep | 2D 프로파일을 경로를 따라 이동하여 솔리드 생성 |
| 불리언 | Boolean | 두 솔리드의 합집합/차집합/교집합 |
| 필렛 | Fillet | 모서리를 둥글게 깎음 |
| 챔퍼 | Chamfer | 모서리를 평면으로 면취 |
| 셸 | Shell (op) | 솔리드를 속을 비워 일정 두께의 벽으로 만듦 |
| 로프트 | Loft | 복수 단면을 보간하여 솔리드 생성 |

## 수학 (Math)

| 용어 | 설명 |
|------|------|
| Vec2/3/4 | 방향과 크기를 가진 벡터 |
| Point2/3 | 공간의 위치 (벡터와 구분) |
| Mat3/4 | 3×3, 4×4 행렬 |
| Transform | 어파인 변환 (회전 + 이동 + 스케일) |
| Quaternion | 3D 회전을 표현하는 4원수. 짐벌락 없음 |
| Epsilon (ε) | 부동소수점 비교 허용 오차. `1e-9` |
| Rodrigues | 임의 축 회전 공식 |
| 발산 정리 | 체적분을 면적분으로 변환. 메시에서 부피 계산에 사용 |

## 시스템 (System)

| 용어 | 영문 | 설명 |
|------|------|------|
| Handle | Handle<T> | Arena 인덱스 + generation. use-after-free 방지 |
| EntityStore | Entity Store | Arena 할당자. O(1) insert/remove/lookup |
| Tag | Persistent Name | 인덱스 대신 의미론적 이름으로 엔티티 참조 |
| TNP | Topological Naming Problem | 위상 변경 시 인덱스 기반 참조가 깨지는 문제 |
| Prelude | — | 자주 사용하는 타입을 한 번에 import |
| Feature Flag | — | Cargo feature로 조건부 컴파일 |
| Newton-Raphson | — | 비선형 방정식의 반복적 근사해 알고리즘 |
| Armijo | — | 라인 서치에서 충분한 감소 조건 |
