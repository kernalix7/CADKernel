# Crate: cadkernel-core

> **역할**: 모든 크레이트가 공유하는 에러 타입 정의  
> **의존성**: 없음 (leaf crate)  
> **경로**: `crates/core/`

## 개요

`cadkernel-core`는 의존성 그래프의 최하위에 위치하여 모든 크레이트에서 import 가능한 공유 타입을 제공합니다.

## 주요 타입

### KernelError

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelError {
    InvalidHandle(&'static str),      // 유효하지 않은 엔티티 핸들
    InvalidArgument(String),           // 잘못된 인자 (생성자 검증 실패)
    ValidationFailed(String),          // B-Rep 검증 실패
    TopologyError(String),             // 위상 상태 오류
    GeometryError(String),             // 기하 연산 실패
    IoError(String),                   // 파일 I/O 실패
}
```

### KernelResult

```rust
pub type KernelResult<T> = Result<T, KernelError>;
```

## 특성

| Trait | 지원 | 용도 |
|-------|:----:|------|
| `Debug` | ✅ | 디버그 출력 |
| `Display` | ✅ | 사용자 친화적 메시지 |
| `Clone` | ✅ | 에러 복사 |
| `PartialEq + Eq` | ✅ | 테스트에서 에러 직접 비교 |
| `std::error::Error` | ✅ | 표준 에러 트레이트 호환 |
| `From<std::io::Error>` | ✅ | `?` 연산자로 IO 에러 자동 변환 |

## 사용 패턴

```rust
use cadkernel_core::{KernelError, KernelResult};

pub fn validate_input(n: usize) -> KernelResult<()> {
    if n < 3 {
        return Err(KernelError::InvalidArgument(
            format!("need at least 3, got {n}")
        ));
    }
    Ok(())
}
```

## 파일 구조

```
crates/core/
├── Cargo.toml
└── src/
    ├── lib.rs          ← mod error + prelude
    ├── error.rs        ← KernelError, KernelResult
    └── prelude.rs      ← pub use re-exports
```
