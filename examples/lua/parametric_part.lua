-- CADKernel Lua Scripting -- Parametric Part
-- Creates brackets of variable dimensions using a function.
--
-- Run with: cargo run -- --script examples/lua/parametric_part.lua

local function make_bracket(width, height, thickness, hole_radius)
    local base = cad.box(width, height, thickness)
    local hole = cad.cylinder(hole_radius, thickness + 2)
    local hole_moved = cad.translate(hole, width / 2, height / 2, -1)
    local result = cad.subtract(base, hole_moved)
    return result
end

-- Create brackets of different sizes
local small = make_bracket(20, 30, 5, 3)
local medium = make_bracket(40, 60, 8, 6)
local large = make_bracket(60, 90, 12, 10)

-- Offset them so they don't overlap
cad.translate(medium, 70, 0, 0)
cad.translate(large, 150, 0, 0)

-- Measure each bracket
local names = {"small", "medium", "large"}
local ids = {small, medium, large}
for i = 1, 3 do
    local m = cad.measure(ids[i])
    print(names[i] .. " bracket volume: " .. string.format("%.1f", m.volume))
end

-- Report total solid count
local all = cad.list()
print("\nTotal solids in scene: " .. #all)
