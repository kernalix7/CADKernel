-- Reference Part R1: Axis-aligned box, 100 x 50 x 25 mm at origin.
--
-- Topology contract (per docs/algorithms/persistent-naming.md):
--   8 vertices, 12 edges, 6 faces, 1 shell, 1 solid.
--   Every face carries a tag generated from this script's OperationId.
-- Verification:
--   cargo test --release --test reference_parts_corpus -- r1_box

local box = cad.box{
    size = {100.0, 50.0, 25.0},
    origin = {0.0, 0.0, 0.0},
    name = "R1_box",
}

assert(box ~= nil, "R1: box construction returned nil")
assert(cad.solid_count() == 1, "R1: expected exactly 1 solid")

-- Topology assertions (Euler-Poincaré: V - E + F = 2 for genus 0).
local v, e, f = cad.topology_counts(box)
assert(v == 8,  string.format("R1: expected 8 vertices, got %d", v))
assert(e == 12, string.format("R1: expected 12 edges, got %d", e))
assert(f == 6,  string.format("R1: expected 6 faces, got %d", f))
assert(v - e + f == 2, "R1: Euler-Poincaré violated")

-- Volume and area.
local vol = cad.quick_volume(box)
local area = cad.quick_area(box)
assert(math.abs(vol  - 100.0 * 50.0 * 25.0)             < 1e-9, "R1: volume mismatch")
assert(math.abs(area - 2 * (100*50 + 100*25 + 50*25))   < 1e-9, "R1: area mismatch")

-- Persistent naming: every face must have a tag (not None).
assert(cad.all_faces_tagged(box), "R1: untagged face detected")

-- Centroid at the geometric centre.
local cx, cy, cz = cad.quick_centroid(box)
assert(math.abs(cx - 50.0) < 1e-9, "R1: centroid x")
assert(math.abs(cy - 25.0) < 1e-9, "R1: centroid y")
assert(math.abs(cz - 12.5) < 1e-9, "R1: centroid z")

-- Save to .cadk and verify content hash is reproducible.
cad.save_cadk(box, "tests/corpus/reference_parts/R1.cadk")
local hash = cad.content_hash("tests/corpus/reference_parts/R1.cadk")
print(string.format("R1 content hash: %s", hash))
print("R1 build OK")
