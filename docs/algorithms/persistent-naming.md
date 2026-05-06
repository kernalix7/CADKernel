# Persistent Naming

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/topology`, `crates/modeling`)
**Implements:** roadmap §8 (history & persistent naming)
**Crate:** `cadkernel-topology::tag`, `cadkernel-modeling::history`

---

## 1. Problem

When user edits an upstream feature (e.g., changes a sketch dimension), downstream features (e.g., a fillet on a derived edge) must continue to refer to the **same logical edge** even though the underlying B-Rep edge is regenerated. Without persistent naming, every recompute breaks downstream selections.

---

## 2. Tag

```rust
struct Tag {
    op_id: u32,        // operation that produced this entity
    entity_kind: u8,   // 0=Vertex, 1=Edge, 2=Face, 3=Shell, 4=Solid
    local_index: u16,  // index within operation's outputs of this kind
    flags: u8,         // bit 0 = generated, bit 1 = derived
}
```

Tags are deterministic: same operation on same inputs produces identical tags.

---

## 3. Tag generation rules

Each operation generates tags for its outputs:

| Operation | Tag rule |
|---|---|
| Primitive (e.g., Box) | Pre-defined `local_index` per face/edge (e.g., box_face_+x = 0, box_face_-x = 1, ...) |
| Sketch | Sketch entity index = `local_index` |
| Extrude | Profile faces inherit tags; lateral faces get new tags indexed by source edge |
| Boolean | Output entities tagged by source operation + intersection index |
| Fillet | Blend faces tagged by source edge; corner patches by vertex |

---

## 4. Persistence under recompute

When operation $O$ is re-executed with new parameters:

1. New B-Rep produced.
2. Tags re-generated using same rules.
3. Output entities have **same tags** as before, even if their geometry changed.

Downstream operations refer to entities by tag, not by entity ID. Edits resolve correctly.

---

## 5. Survival under upstream change

If upstream feature changes topologically (e.g., box → cylinder), tags can no longer match exactly. We use a **survival heuristic**:

```text
fn resolve_reference(downstream_op, ref_tag, current_brep):
    if exact_tag in current_brep:
        return current_brep[exact_tag]
    candidates ← entities_in_brep_with_kind(current_brep, ref_tag.entity_kind)
    best ← argmax(candidates, key = |c| similarity(ref_tag, c.tag))
    if similarity(ref_tag, best.tag) > THRESHOLD:
        return best
    return Unresolved { suggestion = best, alternatives = candidates }
```

### 5.1 Similarity metric

Tags compared by:
- Same `op_id`: weight 0.5
- Same `entity_kind`: required (0 if mismatched)
- Adjacent local indices: weight 0.3
- Spatial proximity (centroid distance): weight 0.2

If best similarity $> 0.7$, auto-resolve. Else mark unresolved; user prompted.

---

## 6. UI feedback

| State | UI |
|---|---|
| Resolved exactly | Green checkmark on feature node |
| Resolved by heuristic | Yellow warning; "Geometry changed; click to confirm" |
| Unresolved | Red icon; "Reference lost; please re-select" |

---

## 7. Acceptance

- v1.0: 100 hand-crafted edits — 95 % resolved exactly, 4 % heuristic, 1 % unresolved (intentional).
- v2.0: 1000 random edits across industrial corpus — 90 % exact, 8 % heuristic, 2 % unresolved.
- v3.0: variational direct edit (drag a face) — 85 % auto-resolve.

---

## 8. References

1. Capoyleas, Chen, Hoffmann. "Generic naming in generative, constraint-based design." *CAD* 1996.
2. Bidarra, Bronsvoort. "Semantic feature modelling." *CAD* 2000.
3. Kripac. "A mechanism for persistently naming topological entities in history-based parametric solid models." *Solid Modeling and Applications* 1995.
4. PTC Creo "topology naming" documentation.
