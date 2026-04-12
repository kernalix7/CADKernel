-- CADKernel Lua Scripting -- Hello World
-- Creates a simple box and measures it.
--
-- Run with: cargo run -- --script examples/lua/hello_cad.lua
-- Or from the viewer scripting console.

local id = cad.box(20, 30, 40)
local m = cad.measure(id)
print("Created box with volume: " .. string.format("%.1f", m.volume))
print("Surface area: " .. string.format("%.1f", m.area))

local c = cad.count(id)
print("Faces: " .. c.faces .. ", Edges: " .. c.edges .. ", Vertices: " .. c.vertices)
