# Boolean goldens manifest

Hand-crafted boolean operations between primitive solids. Each row defines
an input pair, the operation, and asserted invariants on the output. The
output is regenerated each test run; CI verifies the BLAKE3 hash matches
the expected hash recorded in `<id>.expected_hash`.

| ID | A | B | Op | Expected output | Invariants |
|---|---|---|---|---|---|
| 001 | Box(2,1,0.5) at origin | Box(1,2,1) at origin | Union | Single solid, valid manifold | V_total ≤ V_A + V_B |
| 002 | Box(2,2,2) at origin | Box(1,1,1) at (0.5, 0.5, 0.5) | Difference (A−B) | Box with rectangular cavity | V_out = V_A − V_overlap |
| 003 | Box(2,2,2) | Box(1,1,1) at origin | Intersection | Box(1,1,1) | V_out = min(V_A, V_B_clipped) |
| 004 | Sphere(r=1) at origin | Box(2,2,2) at origin | Intersection | Sphere clipped to box (= sphere if box contains it) | Genus 0; volume = 4π/3 |
| 005 | Cylinder(r=1, h=2) | Cylinder(r=1, h=2) at 90° rotation | Union | Cross-shaped solid | Validate face count ≥ 8 |
| 006 | Box(2,2,2) | Box(2,2,2) coincident | Union | Same box (idempotency) | V_out = V_A; face count = 6 |
| 007 | Box(2,2,2) | Box(2,2,2) coincident | Intersection | Same box (idempotency) | V_out = V_A; face count = 6 |
| 008 | Box(2,2,2) | Box(2,2,2) coincident | Difference | Empty | V_out = 0 |
| 009 | Sphere(r=1) | Sphere(r=1) tangent at single point | Union | Two spheres joined at point | Non-manifold detected; reject or split |
| 010 | Cylinder(r=1, h=2) coaxial | Cylinder(r=1, h=2) coaxial | Union | Same cylinder | V_out = V_A |

The TOML schema for each test:

```toml
[test]
id = "001"
description = "..."

[a]
kind = "Box"
size = [2.0, 1.0, 0.5]
transform = "identity"

[b]
kind = "Box"
size = [1.0, 2.0, 1.0]
transform = "identity"

[op]
kind = "Union"

[expected]
solid_count = 1
manifold = true
volume_tol = 1e-9
hash = "blake3:abc123..."     # filled in by `cargo test --features regenerate-goldens`
```

Tests in `crates/modeling/tests/boolean_golden_corpus.rs` iterate this directory.
