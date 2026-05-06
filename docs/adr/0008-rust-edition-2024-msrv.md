# 0008. Rust edition 2024, MSRV 1.85

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** tech-lead, qa-engineer

## Context

Rust edition 2024 was stabilised in Rust 1.85 (released 2025-02). It changes:

- `cargo` resolver default → 3 (more correct feature unification).
- Lifetime capture rules in RPIT (return-position impl trait).
- New keyword reservations (`gen`, `try`, `become`).
- Stricter `unsafe extern` blocks.

We need to choose:

1. Which edition.
2. Which MSRV (minimum supported Rust version).

## Decision

- **Edition: 2024** workspace-wide.
- **MSRV: 1.85** (the version that stabilised edition 2024).
- Enforce in CI via `rust-toolchain.toml` and a `cargo +1.85 check --workspace --all-targets --all-features` job.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Edition 2021, MSRV 1.70 | Misses ~2 years of stabilised features (let-else patterns matured, GAT, async-fn-in-trait, lazy_cell, OffsetDateTime). Forces workarounds in code. |
| Edition 2024, MSRV 1.80 | Edition 2024 stabilised in 1.85; can't have edition without the version. |
| Edition 2024, MSRV "latest stable" (rolling) | Breaks contributors with slightly older toolchains. Annoying for distros that lag (Debian stable Rust is often N-3). |
| Drop MSRV policy entirely | Industry expectation for libraries is to publish an MSRV. Distros and downstream rely on it. |

## Consequences

**Positive:**
- All modern Rust idioms available: `let ... else`, GATs, `impl Trait` in traits, `async fn` in traits (with limitations).
- Resolver 3 catches feature unification issues that resolver 2 misses (we hit these with `serde` features in `cadkernel-io`).
- Stricter `unsafe extern` blocks make our (very few) FFI borders safer to audit.

**Negative:**
- Contributors on Rust 1.84 or older must upgrade. Mitigated by `rust-toolchain.toml` auto-installing the right version via rustup.
- Some downstream library consumers may still be on edition 2021; our public API is unaffected by edition (edition is per-crate compile setting, not API).

**Neutral:**
- We will bump MSRV at most once per minor release (every 3 months), and only when justified by a specific feature need or security fix in `std`/`core`.

## References

- Rust 1.85 release notes: <https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html>
- Edition 2024 guide: <https://doc.rust-lang.org/edition-guide/rust-2024/index.html>
- Resolver 3 RFC: <https://rust-lang.github.io/rfcs/3243-resolver.html>
- `Cargo.toml` workspace edition setting.
