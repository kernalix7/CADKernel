# CADKernel 기여 가이드

[English](../CONTRIBUTING.md) | **한국어**

CADKernel에 기여해 주셔서 감사합니다.

## 개발 환경 준비

### 사전 요구사항
- Rust 1.85+
- CMake 3.16+
- GPU 드라이버 지원 (플랫폼별 Vulkan/Metal/DX12)

### 빌드
```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --release
```

### 테스트
```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## 작업 흐름

1. 저장소를 Fork 합니다
2. 기능 브랜치를 생성합니다: `git checkout -b feature/my-change`
3. Conventional Commits 스타일로 커밋합니다
4. Push 후 Pull Request를 생성합니다

## Pull Request 체크리스트

- [ ] 변경 범위와 목적이 명확한가?
- [ ] 필요한 테스트를 추가/갱신했는가?
- [ ] `cargo fmt`와 `cargo clippy`를 통과하는가?
- [ ] 공개 API에 문서 주석이 있는가?
- [ ] 동작 변경 시 README/문서를 갱신했는가?

## 커밋 메시지 규칙

[Conventional Commits](https://www.conventionalcommits.org/)를 사용합니다:
- `feat:` 새 기능
- `fix:` 버그 수정
- `docs:` 문서 변경
- `refactor:` 동작 변경 없는 구조 개선
- `test:` 테스트 변경
- `chore:` 유지보수 작업

## 보안

보안 이슈는 [SECURITY.ko.md](SECURITY.ko.md)의 제보 절차를 따라 주세요.
