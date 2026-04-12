-- CADKernel Lua Scripting -- Boolean Operations
-- Demonstrates subtract, union, and intersect on primitive solids.
--
-- Run with: cargo run -- --script examples/lua/boolean_operations.lua

-- Create a box and a cylinder that passes through it
local box1 = cad.box(30, 30, 30)
local cyl = cad.cylinder(10, 40)
local cyl_moved = cad.translate(cyl, 15, 15, -5)

-- Subtract the cylinder from the box to create a hole
local result = cad.subtract(box1, cyl_moved)
local m = cad.measure(result)
print("Box with cylindrical hole:")
print("  Volume: " .. string.format("%.1f", m.volume))
print("  Surface area: " .. string.format("%.1f", m.area))

-- Export to STL
cad.export_stl(result, "boolean_result.stl")
print("Exported to boolean_result.stl")

-- Union example: merge two overlapping boxes
local a = cad.box(20, 20, 20)
local b = cad.box(20, 20, 20)
local b_moved = cad.translate(b, 10, 10, 10)
local merged = cad.union(a, b_moved)
local m2 = cad.measure(merged)
print("\nUnion of two overlapping boxes:")
print("  Volume: " .. string.format("%.1f", m2.volume))

-- Intersect example: common region of two boxes
local p = cad.box(20, 20, 20)
local q = cad.box(20, 20, 20)
local q_moved = cad.translate(q, 10, 10, 10)
local common = cad.intersect(p, q_moved)
local m3 = cad.measure(common)
print("\nIntersection of two overlapping boxes:")
print("  Volume: " .. string.format("%.1f", m3.volume))
