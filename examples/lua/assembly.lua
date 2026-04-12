-- CADKernel Lua Scripting -- Simple Assembly
-- Builds a table with a flat top and 4 cylindrical legs.
--
-- Run with: cargo run -- --script examples/lua/assembly.lua

-- Table top: 100 x 60 x 3, elevated to z=50
local top = cad.box(100, 60, 3)
top = cad.translate(top, 0, 0, 50)

-- Four legs at the corners
local leg_positions = {
    {5, 5, 0},
    {85, 5, 0},
    {5, 45, 0},
    {85, 45, 0},
}

for i, pos in ipairs(leg_positions) do
    local leg = cad.cylinder(3, 50)
    cad.translate(leg, pos[1], pos[2], pos[3])
end

-- Report
local parts = cad.list()
print("Created table assembly with " .. #parts .. " parts")

local top_m = cad.measure(top)
print("Table top volume: " .. string.format("%.1f", top_m.volume))
print("Table top area: " .. string.format("%.1f", top_m.area))

-- Export the table top
cad.export_stl(top, "table_top.stl")
print("Exported table top to table_top.stl")
