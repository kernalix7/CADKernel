# Tessellation

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/geometry`), ui-engineer (consumer)
**Implements:** roadmap §3 (geometry), §9 (viewer)
**Crate:** `cadkernel-geometry::tessellate`

---

## 1. Purpose

Convert a NURBS-backed B-Rep face into a triangle mesh suitable for:

- GPU rendering (viewer).
- Mesh-based booleans (fallback path).
- STL/OBJ/glTF export.
- Volume / area / centroid computation.

---

## 2. UV grid sampling

For a face with surface $S(u, v)$ on domain $[u_0, u_1] \times [v_0, v_1]$:

1. Build initial uniform grid $N_u \times N_v$ samples.
2. Adaptively refine where curvature exceeds tolerance.
3. Tessellate trim loops into 2D polygons (Constrained Delaunay).
4. Generate triangles within trimmed region.
5. Lift 2D triangles to 3D via $S(u, v)$.
6. Compute per-vertex normals.

---

## 3. Adaptive refinement

For a quad cell with corners $S(u_i, v_j)$:

$$\text{deviation} = \max_{(u, v) \in \text{cell}} \|S(u, v) - \text{plane}(\text{corners})\|$$

If $\text{deviation} > \tau$, subdivide. Estimate via curvature:

$$\text{deviation} \approx \frac{1}{8} \kappa_{\max} \cdot \text{diag}^2$$

where $\kappa_{\max}$ is max principal curvature, $\text{diag}$ is cell diagonal.

---

## 4. Trim loop tessellation

Each trim loop in UV space is a closed polygon. Use **Constrained Delaunay Triangulation** (CDT) — Shewchuk's Triangle algorithm:

1. Insert loop vertices.
2. Insert constraint edges (loop segments).
3. Refine: insert Steiner points where triangles violate quality criteria.
4. Mark triangles inside / outside using winding.

Reference: Shewchuk. "Triangle: Engineering a 2D quality mesh generator and Delaunay triangulator." *WACG* 1996.

---

## 5. Smooth normals

Per-vertex normals computed via:

1. **Smooth groups via BFS.** Two adjacent triangles share a smooth edge iff dihedral angle $< \theta_{\text{crease}}$. Connected components in the smooth-edge graph form smooth groups.
2. **Area-weighted average.** Within a smooth group, vertex normal = $\sum A_i N_i / |\sum A_i N_i|$.
3. **Sharp edges preserved.** Vertices on a sharp edge get duplicated (one per smooth group).

`THETA_CREASE = 60°` is the default, configurable per face.

---

## 6. Parallelism

Faces tessellated in parallel via rayon. Per-face workload typically $< 10$ ms; full part of 1000 faces tessellated in $< 200$ ms.

---

## 7. Quality metrics

For each output triangle, compute:

- **Aspect ratio** = $\frac{\text{longest edge}}{\text{inradius}}$ — target $< 4$.
- **Min angle** — target $> 20°$.
- **Skew** — fraction of triangles with min angle $< 10°$.

Targets:
- Median aspect ratio: $< 2$.
- 99th percentile aspect ratio: $< 5$.
- Min angle: $> 15°$.

---

## 8. Numerical guards

| Constant | Value | Purpose |
|---|---|---|
| `TESS_TOL_DEFAULT` | $10^{-3} \cdot \text{diag}$ | Adaptive refinement threshold |
| `TESS_GRID_INITIAL_U` | 16 | Initial U samples |
| `TESS_GRID_INITIAL_V` | 16 | Initial V samples |
| `TESS_MAX_REFINEMENT_DEPTH` | 8 | Max subdivisions per cell |
| `THETA_CREASE` | 60° | Smooth-group dihedral cutoff |

---

## 9. Acceptance

- v1.0: tessellation of 12 primitives produces triangles with median aspect $< 2$.
- v2.0: 1M-triangle output for industrial part in $< 1$ s.
- v3.0: tessellation tolerance configurable per face for class-A export.

---

## 10. References

1. Shewchuk. "Triangle: Engineering a 2D quality mesh generator." 1996.
2. Piegl, Tiller. *The NURBS Book*, Ch. 9 (rendering).
3. Lloyd. *Voronoi Tessellation*. (For Lloyd relaxation if quality post-processing.)
4. Botsch, Kobbelt, Pauly, Alliez, Lévy. *Polygon Mesh Processing*. AK Peters, 2010.
