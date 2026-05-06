# 0007. Tag-based persistent naming over geometric matching

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer

## Context

Persistent naming is the property that downstream features survive upstream edits. When a sketch dimension changes, the fillet on a derived edge must continue to refer to the *same* logical edge even though the underlying half-edge object is regenerated.

Two schools of thought exist in CAD:

1. **Tag-based (deterministic identity).** Each operation generates entities with deterministic identifiers based on `(op_id, entity_kind, local_index)`. Downstream features store these identifiers; resolution is a hash lookup.
2. **Geometric matching (heuristic identity).** No persistent identity at all; downstream features store geometric criteria (e.g., "the edge with the smallest radius adjacent to face $F$") and match by geometry on every recompute.

PTC Pro/E (now Creo) historically used geometric matching; Solidworks uses a hybrid; Catia uses a sophisticated hybrid; FreeCAD uses tag-based with limited heuristic fallback.

Geometric matching has the famous "topological naming problem" — small upstream edits cascade into large downstream confusion. PTC has published multiple papers on the suffering this caused.

## Decision

Adopt **tag-based** persistent naming as the primary mechanism, with a **bounded heuristic fallback** when tags fail to resolve (e.g., upstream topology changed structurally).

Spec: [persistent-naming.md](../algorithms/persistent-naming.md).

A `Tag` is an 8-byte struct: `{ op_id: u32, entity_kind: u8, local_index: u16, flags: u8 }`. Tags are deterministic: same operation on same inputs produces identical tags.

Resolution:
1. Exact tag match → done.
2. If exact tag absent, run similarity heuristic ([persistent-naming.md §5.1](../algorithms/persistent-naming.md)): same op_id (weight 0.5) + same entity_kind (required) + adjacent local index (weight 0.3) + spatial proximity (weight 0.2). If best > 0.7, auto-resolve. Else mark unresolved; user prompted.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Pure geometric matching (PTC Pro/E pre-2000) | Famous failure mode: "topological naming problem." Small edits cascade; users learn to fear edits. |
| UUID-based identity | UUIDs are non-deterministic; reopening the same file produces different IDs per session, breaking re-import / cross-session references. |
| Hash of geometry as identity | Geometry changes every recompute → IDs change → references break. Defeats the purpose. |
| Pointer / arena index as identity | Reset on every recompute; breaks across save/load. |
| Hybrid weighted of all of above | More complex without a clear win over tag + bounded heuristic. We can adopt later if data shows tag-based misses too often. |

## Consequences

**Positive:**
- Deterministic identity → cross-session, cross-platform, cross-architecture stable references.
- Hash lookup is O(1).
- Heuristic fallback gives PTC-style robustness without pure-geometric pain.
- UI can clearly distinguish "exact" (green check) from "heuristic" (yellow warn) from "unresolved" (red), giving users transparency that pure geometric systems lack.

**Negative:**
- Operations that produce many entities (boolean of complex parts) need careful local_index assignment to ensure determinism. Mitigated by `EdgeCache` deduplication helper which uses canonicalised vertex ordering.
- Tag survival across upstream restructuring (e.g., box → cylinder) requires the heuristic; not 100 %. Acceptance: 95 % exact, 4 % heuristic, 1 % unresolved on golden edit suite.

**Neutral:**
- Tags persist in `.cadk` file ([brep-abi.md §10](../algorithms/brep-abi.md)) at fixed 8-byte cost per tagged entity.

## References

- Capoyleas, Chen, Hoffmann, "Generic naming in generative, constraint-based design," *CAD* 1996.
- Bidarra, Bronsvoort, "Semantic feature modelling," *CAD* 2000.
- Kripac, "A mechanism for persistently naming topological entities in history-based parametric solid models," *Solid Modeling and Applications* 1995.
- FreeCAD Topological Naming Project: <https://wiki.freecad.org/Topological_naming_problem>
- [docs/algorithms/persistent-naming.md](../algorithms/persistent-naming.md)
- [docs/algorithms/brep-abi.md §10](../algorithms/brep-abi.md)
