# 0011. automerge CRDT for real-time collaboration

**Status:** Accepted (Track F design)
**Date:** 2026-05-06
**Deciders:** io-engineer, tech-lead

## Context

Track F (Collaboration & Cloud) requires conflict-free real-time co-editing of CADKernel documents. Two users editing the same part on different machines must see merged results without an authoritative server arbitrating per keystroke (Onshape-style, but self-hostable).

Two families of solutions:

1. **Operational Transformation (OT)** — Google Docs's original approach. Requires a central authority to linearise operations and transform concurrent ops against history.
2. **Conflict-free Replicated Data Types (CRDTs)** — peer-to-peer; mathematical guarantee that all replicas converge given the same set of operations, regardless of order.

For a CAD document (graph-shaped: solids, faces, edges, sketches, history), we need the CRDT to handle:

- Tree / DAG structure (history is a DAG).
- Numeric properties (dimensions, parameters).
- Sets (selection, visibility).
- Bidirectional refs (a fillet refers to its target edges).

## Decision

Use **automerge 0.5+** (Rust crate, Apache 2.0) as the CRDT backbone. Wrap it in a CADKernel-specific facade `cadkernel-collab` exposing typed accessors for `Document`, `Sketch`, `Feature`, etc.

Operation log model: `(actor_id: u128, lamport: u64, parent_hash: BLAKE3, payload: Op)`.

Per-op-type conflict resolution policy in [docs/algorithms/](../algorithms/) (planned `crdt-merge.md`):

| Op kind | Conflict policy |
|---|---|
| Add feature | Both keep; visualised side-by-side; user picks |
| Modify dimension | Last-writer-wins by Lamport; loser shown in history |
| Delete feature | Tombstone; refs auto-redirect to predecessor; warn user |
| Selection | Per-user view; never merged |
| Reorder history | Merge as DAG, not as linear list |

## Alternatives considered

| Option | Rejected because |
|---|---|
| OT (à la ShareJS / Quill) | Requires central server for linearisation; doesn't fit our self-hostable + offline-first goal. Complex to extend with new op types. |
| Yjs (JS native) | Excellent CRDT but JS-first; Rust port `yrs` is sound but smaller community than automerge. Reasonable runner-up; revisit if automerge stalls. |
| Custom CRDT | Impossible to get right at v1. Years of formal verification work. |
| RGA / LSEQ for text-only | Wrong shape; we need tree+graph CRDTs. |
| Diamond Types | Promising but specifically text-focused. |
| State-based CRDT (delta) | Larger payloads; harder to ship over WebSocket frames. Op-based wins for our use case. |

## Consequences

**Positive:**
- automerge 0.5+ has a Rust core with sound CRDT semantics ([Kleppmann et al., POPL 2017](https://arxiv.org/abs/1608.03960)).
- Convergence proven under network partitions and reordering — critical for offline edits.
- Compact binary encoding; sync messages typically < 1 KB per edit.
- Embeddable in a desktop app without a central server (peer-to-peer via WebRTC data channels for small teams; relay server for NAT traversal).

**Negative:**
- Memory: every op is retained in history → grows unboundedly. Mitigated by snapshot-and-prune: every 1,000 ops or 30 minutes, take a snapshot, prune ops older than the snapshot, garbage-collect tombstones.
- Document size: ~3× the compact binary representation of the same data without history. Acceptable for collaboration mode; offline-only sessions skip CRDT overhead.
- Causal ordering metadata adds 16-32 bytes per op. Compresses well with zstd.

**Neutral:**
- Onshape uses a similar (but custom and proprietary) CRDT-ish approach. We re-use a public, audited library instead.

## References

- Kleppmann, Beresford, Svingen, "Conflict-free replicated data types," ICDCS 2017.
- Shapiro, Preguiça, Baquero, Zawirski, "Conflict-free replicated data types," INRIA TR 7687, 2011.
- Kleppmann et al., "A highly-available move operation for replicated trees," IEEE TPDS 2022.
- automerge: <https://automerge.org/>
- CADKernel Track F roadmap §13.
- [docs/COMMERCIAL_CAD_ROADMAP.md](../COMMERCIAL_CAD_ROADMAP.md) Track F.
