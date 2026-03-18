# Contributing to CADKernel

**English** | [한국어](docs/CONTRIBUTING.ko.md)

Thanks for your interest in contributing to CADKernel.

## Development Setup

### Prerequisites
- Rust 1.85+
- CMake 3.16+
- GPU driver support (Vulkan/Metal/DX12 depending on platform)

### Build
```bash
git clone https://github.com/kernalix7/CADKernel.git
cd CADKernel
cargo build --release
```

### Test
```bash
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

## Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-change`
3. Commit with Conventional Commits style
4. Push and open a Pull Request

## Pull Request Checklist

- [ ] The change has a clear scope and rationale
- [ ] Tests are added/updated where applicable
- [ ] `cargo fmt` and `cargo clippy` pass
- [ ] Public APIs include documentation comments
- [ ] README / docs are updated when behavior changes

## Commit Message Convention

Use [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` for new features
- `fix:` for bug fixes
- `docs:` for documentation changes
- `refactor:` for internal improvements without behavior changes
- `test:` for test updates
- `chore:` for maintenance tasks

## Security

For security issues, follow the process in [SECURITY.md](SECURITY.md).
