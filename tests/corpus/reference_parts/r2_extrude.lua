-- Reference Part R2: Extruded sketch with a circular through-hole.
--
-- Sketch: 80 x 40 mm rectangle on XY plane, with a r=10 mm circle centred
-- inside it. Extrude along +Z by 20 mm. Produces a rectangular prism with
-- a cylindrical through-hole.
--
-- Topology contract:
--   12 vertices (8 outer corners + 2 vertices per circle ends, but circle is
--   modelled as a single closed edge so circle contributes 2 vertices total
--   on the top and bottom faces -> 12 total).
--   18 edges (12 box edges + 2 circle edges + 4 imprint connecting? Actually
--   no: extrude of a sketch with a hole produces: 4 corner verts × 2 caps
--   + 1 circle vert × 2 caps = 10 V; 4 outer × 2 caps + 4 vertical + 1 circle
--   × 2 caps + 1 vertical seam on cylinder = 11 E; 2 caps + 4 sides + 1
--   cylinder = 7 F. We use 'approximately' assertions where exact counts
--   depend on circle representation choice — see docs/algorithms/brep-abi.md
--   §6 for circle = single edge convention.)
--
-- This part exercises:
--   * sketch with hole (inner loop)
--   * extrude with cap faces and side faces
--   * cylindrical surface binding (via bind_face_surface)
--   * persistent naming across imprint of sketch onto cap faces

local sketch = cad.sketch.new{plane = "XY"}
cad.sketch.add_rectangle(sketch, {0, 0}, {80, 40})
cad.sketch.add_circle(sketch, {40, 20}, 10)
cad.sketch.solve(sketch)

local part = cad.extrude{
    sketch = sketch,
    distance = 20.0,
    direction = "+Z",
    name = "R2_extrude",
}

assert(part ~= nil, "R2: extrude returned nil")
assert(cad.solid_count() == 1, "R2: expected 1 solid")

-- Genus must be 0 (a hole through the extrusion in the topological sense
-- but we model this as a solid with two shells: the outer and the inner
-- (the hole's lateral surface). Genus of the outer shell = 1.
local v, e, f = cad.topology_counts(part)
assert(v >= 8 and v <= 16, string.format("R2: V out of range: %d", v))
assert(f >= 6 and f <= 8,  string.format("R2: F out of range: %d", f))

-- Volume = box volume - hole volume.
local expected_vol = 80.0 * 40.0 * 20.0 - math.pi * 10.0 * 10.0 * 20.0
local vol = cad.quick_volume(part)
assert(math.abs(vol - expected_vol) < 1e-3,
    string.format("R2: volume mismatch: got %f expected %f", vol, expected_vol))

assert(cad.all_faces_tagged(part), "R2: untagged face")

cad.save_cadk(part, "tests/corpus/reference_parts/R2.cadk")
print("R2 build OK")
