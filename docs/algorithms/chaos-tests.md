# Chaos & Failure-Injection Tests

**Status:** spec v1, last updated 2026-05-06
**Owner:** qa-engineer
**Implements:** roadmap §11 (verification, robustness)

---

## 1. Purpose

Verify that CADKernel **degrades gracefully** under adverse conditions. No panic, no silent corruption, clear error to user.

---

## 2. Failure-injection catalogue

| ID | Injection | Expected behaviour |
|---|---|---|
| C1 | Disk full during save | `KernelError::Io(NoSpace)` returned; in-memory state preserved; original file untouched (atomic write). |
| C2 | Truncated `.cadk` file (last 100 bytes cut) | `KernelError::CorruptFile { offset, expected }` with line/byte report; no panic. |
| C3 | NaN injected into vertex position | Operations referencing vertex fail with `KernelError::Numerical(Nan)`; B-Rep marked invalid. |
| C4 | Infinite f64 in NURBS control point | Same as C3; pre-import validator catches. |
| C5 | Concurrent edit (two threads modify same op) | One thread succeeds, other gets `KernelError::Conflict`; consistent state. |
| C6 | OOM during tessellation | Tessellation aborts; partial mesh discarded; `KernelError::OutOfMemory` reported. |
| C7 | Cancellation token fires mid-boolean | Boolean rolls back to pre-op state; UI receives `KernelError::Cancelled`. |
| C8 | Malformed STEP entity | Parse error reported with line/col; partial import optionally saved. |
| C9 | Symlink loop in import path | `KernelError::Io(Loop)`; no infinite recursion. |
| C10 | f64 underflow in Jacobian eval | LM solver detects via NaN check; reports `Inconsistent` with diagnostic. |
| C11 | Plugin throws during `on_model_changed` | Plugin disabled with error log; main op succeeds. |
| C12 | OS signal (SIGTERM) during long op | Async shutdown: current op completes or rolls back; state saved before exit. |

---

## 3. Test methodology

For each injection ID:

1. Setup: known-good initial state.
2. Inject: simulate failure (mock allocator returning OOM, mock filesystem returning ENOSPC, etc.).
3. Trigger: run operation that triggers the failure path.
4. Verify:
   - Correct error type returned.
   - State consistency (no partial writes, no dangling references).
   - Logs contain expected diagnostic.
   - No panic / no abort.
5. Recover: re-run operation without injection succeeds.

---

## 4. Tooling

- **Allocator hooks**: `mockall` + custom `GlobalAlloc` to inject OOM at chosen call sites.
- **Filesystem mock**: `tempfile` + custom `Write` impl returning `ErrorKind::WriteZero` / `OutOfSpace`.
- **Cancellation**: `CancellationToken` (tokio-util) with timer-based fire.
- **Concurrency**: `loom` (deterministic concurrency exploration) for C5.
- **Signal**: `nix` for sending SIGTERM to subprocess in test harness.

---

## 5. CI integration

- Smoke chaos suite (C1, C2, C5, C7, C8): every PR.
- Full chaos suite (C1-C12): nightly.
- Loom run for C5: weekly (slow).

---

## 6. Coverage measurement

For each injection, verify the corresponding error-handling code path is exercised. Measured via `cargo llvm-cov --tests --include-pattern '**/error.rs'`.

Target: 100 % coverage of error-handling branches by chaos suite.

---

## 7. Acceptance gate

Chaos suite must pass on Linux, macOS, Windows. Failure of any case blocks release.

---

## 8. References

1. Netflix, "Chaos Engineering" white paper.
2. *Site Reliability Engineering*, O'Reilly, 2016.
3. Loom: <https://github.com/tokio-rs/loom>
4. Mockall: <https://github.com/asomers/mockall>
