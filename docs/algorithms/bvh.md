# BVH (Bounding Volume Hierarchy)

**Status:** spec v1, last updated 2026-05-06
**Owner:** kernel-engineer (`crates/geometry`)
**Implements:** roadmap §3 (geometry indexing), §9 (viewer picking)
**Crate:** `cadkernel-geometry::bvh`

---

## 1. Purpose

Spatial acceleration structure used for:

1. **Picking** — ray-vs-scene queries from viewer (mouse cursor).
2. **Boolean face-pair pruning** — find face pairs whose AABBs overlap.
3. **Snap finding** — nearest-point queries during sketching.
4. **Frustum culling** — viewer renders only visible objects.

---

## 2. Construction — SAH (surface-area heuristic)

For a set of primitives $\{P_i\}$ with AABBs $\{B_i\}$:

```text
fn build_bvh(prims):
    if |prims| ≤ LEAF_SIZE:
        return Leaf(prims)
    bbox ← union(BBox(p) for p in prims)
    axis, split ← find_best_split_SAH(prims, bbox)
    left, right ← partition(prims, axis, split)
    return Node {
        bbox,
        left:  build_bvh(left),
        right: build_bvh(right),
    }
```

### 2.1 SAH cost

Cost of a split is:

$$C(\text{split}) = C_{\text{trav}} + \frac{|B_L|}{|B|} \cdot |\text{prims}_L| \cdot C_{\text{isect}} + \frac{|B_R|}{|B|} \cdot |\text{prims}_R| \cdot C_{\text{isect}}$$

where $|B|$ is surface area of AABB. We pick the split (axis, position) minimising $C$.

### 2.2 Binning

For each axis, bin primitive centroids into $K = 32$ uniform bins. Evaluate SAH at each bin boundary: $K-1$ candidate splits per axis. Total $3 \cdot 31 = 93$ candidates per node — fast.

Reference: Wald, "On fast construction of SAH-based bounding volume hierarchies." *IEEE Symposium on Interactive Ray Tracing*, 2007.

### 2.3 Leaf size

`LEAF_SIZE = 4` — 4 primitives per leaf is empirically optimal for f64 geometry on modern CPUs.

---

## 3. Traversal — ray query

Slab method (Kay-Kajiya 1986):

```text
fn ray_intersect(node, ray) -> Option<Hit>:
    t_enter, t_exit ← intersect_aabb(ray, node.bbox)
    if t_enter > ray.t_max or t_exit < 0:
        return None

    if node is Leaf:
        return nearest hit among node.prims

    # Recurse into nearer child first
    near, far ← order_by_distance(node.left, node.right, ray)
    h ← ray_intersect(near, ray)
    if h.is_some(): ray.t_max ← h.t
    h2 ← ray_intersect(far, ray)
    return min(h, h2)
```

### 3.1 Slab intersection

For each axis $a \in \{x, y, z\}$:

$$t_{a,\min} = \frac{B_{a,\min} - \text{origin}_a}{\text{dir}_a}, \quad t_{a,\max} = \frac{B_{a,\max} - \text{origin}_a}{\text{dir}_a}$$

then $t_{\text{enter}} = \max_a \min(t_{a,\min}, t_{a,\max})$, $t_{\text{exit}} = \min_a \max(t_{a,\min}, t_{a,\max})$.

Hit iff $t_{\text{exit}} \geq t_{\text{enter}}$ and $t_{\text{exit}} \geq 0$.

Reference: Kay, Kajiya. "Ray tracing complex scenes." *SIGGRAPH* 1986.

---

## 4. Picking budget

Per-frame picking budget: $< 4$ ms on reference hardware. Implementation:

- Pre-built scene BVH cached.
- Query traversal early-exits on first hit (face-only picking).
- Edge / vertex picking: separate BVH per entity kind.
- Marquee selection: frustum traversal with conservative AABB-vs-frustum tests.

---

## 5. Updates

When geometry changes (recompute), rebuild affected sub-BVH:

- Single primitive moved: re-fit ancestor AABBs only ($O(\log N)$).
- Many primitives moved: rebuild from scratch ($O(N \log N)$).
- Threshold: rebuild if $> 5\%$ primitives changed.

---

## 6. Numerical guards

| Constant | Value |
|---|---|
| `LEAF_SIZE` | 4 |
| `BIN_COUNT` | 32 |
| `C_TRAV` | 1.0 |
| `C_ISECT` | 4.0 |
| `MAX_DEPTH` | 64 |

---

## 7. Acceptance

- v1.0: pick face from 1M-triangle scene in $< 4$ ms.
- v2.0: scene rebuild after recompute $< 50$ ms for 500-feature model.
- Memory overhead: $< 32$ bytes per primitive.

---

## 8. References

1. Wald. "On fast construction of SAH-based bounding volume hierarchies." 2007.
2. Kay, Kajiya. "Ray tracing complex scenes." *SIGGRAPH* 1986.
3. MacDonald, Booth. "Heuristics for ray tracing using space subdivision." *Visual Computer* 1990.
4. Embree (Intel) BVH design notes. <https://www.embree.org/>
5. PBRT 4th ed., Chapter 4 (BVHs).
