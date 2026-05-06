# 0001. B-Rep topology: half-edge data structure

**Status:** Accepted
**Date:** 2026-05-06
**Deciders:** kernel-engineer, tech-lead

## Context

CADKernel needs a topological data structure for B-Rep solids that supports:

- O(1) traversal: face → its loops → its half-edges → its vertices and twin face.
- Manifold and non-manifold (locally) representation (for sheet bodies, mid-surface, tee-junctions during boolean intermediate states).
- Persistent identity through edits (for naming).
- Cache-friendly layout for batch operations (tessellation, validation).
- Send + Sync compatibility for parallel face processing.

The three classical candidates are **half-edge**, **winged-edge** (Baumgart 1972), and **quad-edge** (Guibas-Stolfi 1985). Industry survey:

| System | Topology |
|---|---|
| OpenCascade (OCCT) | Half-edge variant (`TopOpeBRep`) |
| Parasolid | Half-edge (vendor-private but reverse-engineered as such) |
| ACIS | "ACIS topology" — half-edge family |
| FreeCAD | Inherits OCCT half-edge |
| CGAL Polyhedron_3 | Half-edge |
| OpenMesh | Half-edge |
| Blender mesh (BMesh) | Half-edge |
| Surface Evolver | Quad-edge |

Half-edge dominates production CAD and mesh processing.

## Decision

Adopt **half-edge** as the primary topological data structure for `crates/topology`.

A half-edge $h$ stores:
- `vertex`: target vertex (origin = `h.twin().vertex` or `h.prev().vertex`)
- `twin`: the half-edge on the opposite side of the same edge (or `None` for non-manifold)
- `next`, `prev`: cyclic order around the loop
- `loop`: parent loop
- `edge`: parent edge (carries geometry and tag)

Faces own outer loop + 0..N inner loops. Edges own a curve binding. Vertices own a point binding.

## Alternatives considered

| Option | Rejected because |
|---|---|
| Winged-edge (Baumgart 1972) | Non-uniform traversal: distinguishes left/right edge wings, complicates ccw/cw uniform iteration. Higher per-edge memory than half-edge. Mostly historical interest. |
| Quad-edge (Guibas-Stolfi 1985) | Designed for general subdivision (handles dual graphs uniformly). Overkill for B-Rep where we mostly traverse primal. ~2× memory per edge. |
| Indexed face set (no half-edge) | O(face_count) to find adjacencies. Would force quadratic boolean and rendering. Acceptable for STL/OBJ, not for B-Rep. |
| Vertex-vertex (Campagna et al.) | Compact but slow loop traversal. Wrong trade-off for our use case. |
| Combinatorial maps / generalised maps (CMap/GMap) | More general than half-edge (handles arbitrary dimension). Implementation complexity not justified for 3D-only solids. |

## Consequences

**Positive:**
- O(1) face↔face adjacency via twin pointer.
- Idiomatic Rust via `Handle<HalfEdge>` (generational arena, see [persistent-naming.md](../algorithms/persistent-naming.md)).
- Compatible with most CAD literature and OCCT documentation, easing onboarding.
- Memory: ~64 bytes per half-edge (4 × `Handle<u64>` + small flags) — acceptable for industrial parts ($10^6$ half-edges = 64 MB, well under target 1 GB peak RSS).

**Negative:**
- Non-manifold support requires explicit "no twin" sentinel; intermediate boolean states briefly produce non-manifold half-edges that must be cleaned up.
- Twin pointer must be maintained in lockstep with all topology mutations — bug-prone. Mitigated by `EdgeCache` deduplication helper and validation pass after every mutation.

**Neutral:**
- Persistent naming (Tag) is orthogonal to topology choice; would have been needed regardless.

## References

- Mäntylä, *An Introduction to Solid Modeling*, 1988, Chapter 4.
- Baumgart, "Winged-edge polyhedron representation," Stanford AI Memo 179, 1972.
- Guibas, Stolfi, "Primitives for the manipulation of general subdivisions and the computation of Voronoi diagrams," *ACM TOG* 4(2), 1985.
- OpenMesh design notes: <https://www.graphics.rwth-aachen.de/software/openmesh/>
- Botsch et al., *Polygon Mesh Processing*, AK Peters 2010, Chapter 2.
- [docs/algorithms/persistent-naming.md](../algorithms/persistent-naming.md)
- [docs/algorithms/brep-abi.md](../algorithms/brep-abi.md) §6 (TOPO section layout)
