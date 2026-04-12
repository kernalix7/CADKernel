-- CADKernel Lua Scripting -- Batch Export
-- Creates spheres of increasing radius and exports each to STL.
--
-- Run with: cargo run -- --script examples/lua/batch_export.lua

for i = 1, 5 do
    local r = 5 * i
    local s = cad.sphere(r)
    cad.translate(s, i * 30, 0, 0)
    cad.export_stl(s, "sphere_r" .. r .. ".stl")
    local m = cad.measure(s)
    print("Sphere r=" .. r .. "  volume=" .. string.format("%.1f", m.volume))
end

print("\nExported 5 spheres with increasing radii")
print("Solids in scene: " .. #cad.list())
