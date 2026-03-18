# CADKernel — FreeCAD Feature Parity Master Plan V2

> **Goal**: Implement every FreeCAD feature in CADKernel.
> **Reference**: wiki.freecad.org (all workbench pages, fetched 2026-03-14)
> **Created**: 2026-03-06 | **Updated**: 2026-03-14
> **Current**: 609 tests, Phases A–U complete

---

## Table of Contents

1. [Current State Summary](#1-current-state-summary)
2. [FreeCAD Feature Gap Analysis (V2)](#2-freecad-feature-gap-analysis-v2)
3. [Implementation Roadmap](#3-implementation-roadmap)
4. [Phase Details](#4-phase-details)
5. [Verification Criteria](#5-verification-criteria)

---

## 1. Current State Summary

### 1.1 What's Fully Implemented (as of 2026-03-14)

| Category | Count | Features |
|----------|:-----:|----------|
| **Primitives** | 13 | Box, Cylinder, Sphere, Cone, Torus, Tube, Prism, Wedge, Ellipsoid, Helix, Spiral, Polygon, Plane Face |
| **Boolean** | 5 | Union, Intersection, Difference, XOR, Exact Boolean (face-split SSI) |
| **Features** | 24 | Extrude, Revolve, Sweep, Loft, Chamfer, Fillet, Draft, Shell, Mirror, Scale, Linear/Circular Pattern, Split, Section, Offset, Thickness, Taper Extrude, Pad, Pocket, Groove, Hole, Countersunk Hole, Multi-Transform, Refine, Reverse |
| **PartDesign** | 13 | Pad, Pocket, Groove, Hole, Additive/Subtractive Box/Cylinder/Sphere/Cone/Torus, Body |
| **Curves** | 8 | Line, LineSegment, Circle, Arc, Ellipse, NurbsCurve, TrimmedCurve, OffsetCurve |
| **Surfaces** | 9 | Plane, Cylinder, Sphere, Cone, Torus, NurbsSurface, TrimmedSurface, RevolutionSurface, ExtrusionSurface |
| **NURBS** | 28 | Full NURBS kernel (A01-A28): derivatives, fitting, knots, trimming, SSI |
| **Intersection** | 8 | Curve-Curve, Plane-Plane/Sphere/Cylinder, Sphere-Sphere, Ray-Surface, Curve-Surface, Surface-Surface (SSI) |
| **Sketch Entities** | 6 | Point, Line, Arc, Circle, Ellipse, BSpline |
| **Sketch Constraints** | 24 | Coincident, H/V, Parallel, Perpendicular, Tangent, Symmetric, Distance, Angle, Radius, Length, Fixed, PointOnLine/Circle, EqualLength/Radius, Midpoint, Collinear, Concentric, Diameter, Block, HorizontalDistance, VerticalDistance, PointOnObject |
| **I/O Formats** | 11 | STL, OBJ, glTF, SVG, JSON, CADK, STEP, IGES, DXF, PLY, 3MF, BREP |
| **Mesh Ops** | 12 | Decimate, Fill Holes, Curvature, Subdivide, Flip Normals, Smooth, Boolean Union, Cut With Plane, Section From Plane, Split By Components, Harmonize Normals, Check Watertight |
| **TechDraw** | 8 | Project Solid, Section View, Detail View, Three-View, SVG Export, Dimensions, Hatching, Leaders |
| **Assembly** | 5 | Assembly, Components, Constraints (5 types), Interference Detection, BOM |
| **FEM** | 3 | TetMesh, Materials, Static Analysis |
| **Surface Ops** | 4 | Ruled Surface, Surface From Curves, Extend Surface, Pipe Surface |
| **Draft Ops** | 5 | Wire, BSpline Wire, Clone, Rectangular Array, Path Array |
| **Geometry Kernel** | 6 | Isocurve, Surface Curvatures, Offset Curve, Revolution Surface, Extrusion Surface, Blend Curve |
| **Performance** | 3 | BVH, Parallel Tessellation, Merge Meshes |
| **Viewer** | 10 | 6 Workbenches, ViewCube, 8 Display Modes, 4x MSAA, Sketcher GUI |

### 1.2 Test Count: 609 passing, 0 warnings

---

## 2. FreeCAD Feature Gap Analysis (V2)

### Legend
- ✅ = Implemented
- 🔶 = Partial (underlying op exists, missing integration/options)
- ❌ = Missing entirely

---

### 2.1 Part Workbench (FreeCAD: ~55 tools)

#### Primitives & Shapes (20)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | Cube (Box) | make_box | ✅ |
| 2 | Cylinder | make_cylinder | ✅ |
| 3 | Sphere | make_sphere | ✅ |
| 4 | Cone | make_cone | ✅ |
| 5 | Torus | make_torus | ✅ |
| 6 | Tube | make_tube | ✅ |
| 7 | Plane | make_plane_face | ✅ |
| 8 | Ellipsoid | make_ellipsoid | ✅ |
| 9 | Prism | make_prism | ✅ |
| 10 | Wedge | make_wedge | ✅ |
| 11 | Helix | make_helix | ✅ |
| 12 | Spiral | make_spiral | ✅ |
| 13 | Regular Polygon | make_polygon | ✅ |
| 14 | Circle (arc primitive) | — | ❌ |
| 15 | Ellipse (arc primitive) | — | ❌ |
| 16 | Point (shape) | — | ❌ |
| 17 | Line (shape) | — | ❌ |
| 18 | Shape Builder | — | ❌ |
| 19 | Primitive (dialog) | — | 🔶 |

#### Shape Creation & Modification (18)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 20 | Extrude | extrude | ✅ |
| 21 | Revolve | revolve | ✅ |
| 22 | Mirror | mirror_solid | ✅ |
| 23 | Scale | scale_solid | ✅ |
| 24 | Fillet | fillet_edge | ✅ |
| 25 | Chamfer | chamfer_edge | ✅ |
| 26 | Loft | loft | ✅ |
| 27 | Sweep | sweep | ✅ |
| 28 | Section | section_solid | ✅ |
| 29 | Cross-Sections | cross_sections | ✅ |
| 30 | 3D Offset | offset_solid | ✅ |
| 31 | 2D Offset | offset_polygon_2d | 🔶 |
| 32 | Thickness (Shell) | shell_solid / thickness_solid | ✅ |
| 33 | Face From Wires | — | ❌ |
| 34 | Ruled Surface | ruled_surface | ✅ |
| 35 | Project on Surface | project_points_on_surface | 🔶 |
| 36 | Appearance per Face | — | ❌ |
| 37 | Attachment | — | ❌ |

#### Boolean & Compound (10)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 38 | Union (Fuse) | boolean_op Union | ✅ |
| 39 | Cut (Difference) | boolean_op Difference | ✅ |
| 40 | Intersection (Common) | boolean_op Intersection | ✅ |
| 41 | Boolean XOR | boolean_xor | ✅ |
| 42 | Compound | Compound | ✅ |
| 43 | Explode Compound | — | ❌ |
| 44 | Compound Filter | — | ❌ |
| 45 | Boolean Fragments | — | ❌ |
| 46 | Slice Apart | split_solid | ✅ |
| 47 | Slice to Compound | — | ❌ |

#### Join Operations (3)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 48 | Connect Shapes | — | ❌ |
| 49 | Embed Shapes | — | ❌ |
| 50 | Cutout Shape | — | ❌ |

#### Checking & Conversion (8)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 51 | Check Geometry | check_geometry | ✅ |
| 52 | Defeaturing | remove_face / simplify_solid | 🔶 |
| 53 | Shape From Mesh | shape_from_mesh | ✅ |
| 54 | Points From Shape | — | ❌ |
| 55 | Convert to Solid | — | ❌ |
| 56 | Reverse Shapes | reverse_solid | ✅ |
| 57 | Refine Shape | refine_shape | ✅ |
| 58 | Simple/Transformed Copy | clone_solid | 🔶 |

**Part Summary: 37/58 implemented (64%), 14 missing, 7 partial**

---

### 2.2 PartDesign Workbench (FreeCAD: ~53 tools)

#### Structure (8)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | New Body | Body | ✅ |
| 2 | New Sketch | Sketch | ✅ |
| 3 | Attach Sketch | — | ❌ |
| 4 | Edit Sketch | — | 🔶 |
| 5 | Validate Sketch | — | ❌ |
| 6 | Check Geometry | check_geometry | ✅ |
| 7 | Sub-Shape Binder | — | ❌ |
| 8 | Clone | clone_solid | ✅ |

#### Additive Features (13)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 9 | Pad | pad | ✅ |
| 10 | Revolution | revolve + union | 🔶 |
| 11 | Additive Loft | loft + union | 🔶 |
| 12 | Additive Pipe (Sweep) | sweep + union | 🔶 |
| 13 | Additive Helix | — | ❌ |
| 14 | Additive Box | additive_box | ✅ |
| 15 | Additive Cylinder | additive_cylinder | ✅ |
| 16 | Additive Sphere | additive_sphere | ✅ |
| 17 | Additive Cone | additive_cone | ✅ |
| 18 | Additive Ellipsoid | — | ❌ |
| 19 | Additive Torus | additive_torus | ✅ |
| 20 | Additive Prism | — | ❌ |
| 21 | Additive Wedge | — | ❌ |

#### Subtractive Features (14)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 22 | Pocket | pocket | ✅ |
| 23 | Hole | hole | ✅ |
| 24 | Groove | groove | ✅ |
| 25 | Subtractive Loft | — | ❌ |
| 26 | Subtractive Pipe | — | ❌ |
| 27 | Subtractive Helix | — | ❌ |
| 28 | Subtractive Box | subtractive_box | ✅ |
| 29 | Subtractive Cylinder | subtractive_cylinder | ✅ |
| 30 | Subtractive Sphere | subtractive_sphere | ✅ |
| 31 | Subtractive Cone | subtractive_cone | ✅ |
| 32 | Subtractive Ellipsoid | — | ❌ |
| 33 | Subtractive Torus | subtractive_torus | ✅ |
| 34 | Subtractive Prism | — | ❌ |
| 35 | Subtractive Wedge | — | ❌ |

#### Dress-Up (4)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 36 | Fillet | fillet_edge | ✅ |
| 37 | Chamfer | chamfer_edge | ✅ |
| 38 | Draft | draft_faces | ✅ |
| 39 | Thickness | thickness_solid | ✅ |

#### Transformations (5)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 40 | Mirror | mirror_solid | ✅ |
| 41 | Linear Pattern | linear_pattern | ✅ |
| 42 | Polar Pattern | circular_pattern | ✅ |
| 43 | Multi-Transform | multi_transform | ✅ |
| 44 | Scale | scale_solid | ✅ |

#### Additional (5)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 45 | Boolean Operation | boolean_op | ✅ |
| 46 | Involute Gear | make_involute_gear | ✅ |
| 47 | Sprocket | — | ❌ |
| 48 | Shaft Design Wizard | — | ❌ |
| 49 | Shape Binder | — | ❌ |

#### Context Menu (4)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 50 | Suppressed | — | ❌ |
| 51 | Set Tip | — | ❌ |
| 52 | Move Object To Body | — | ❌ |
| 53 | Move Feature After | — | ❌ |

**PartDesign Summary: 31/53 implemented (58%), 18 missing, 4 partial**

---

### 2.3 Sketcher Workbench (FreeCAD: ~109 tools)

#### General (14)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | New Sketch | Sketch::new() | ✅ |
| 2 | Edit Sketch | GUI SketchMode | ✅ |
| 3 | Attach Sketch | — | ❌ |
| 4 | Reorient Sketch | — | ❌ |
| 5 | Validate Sketch | — | ❌ |
| 6 | Merge Sketches | — | ❌ |
| 7 | Mirror Sketch | — | ❌ |
| 8 | Leave Sketch | GUI | ✅ |
| 9 | Align View to Sketch | — | ❌ |
| 10 | Toggle Section View | — | ❌ |
| 11 | Stop Operation | — | ❌ |
| 12 | Grid | — | ❌ |
| 13 | Snap | — | ❌ |
| 14 | Rendering Order | — | ❌ |

#### Geometry Creation (29)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 15 | Point | add_point | ✅ |
| 16 | Polyline | add_polyline | ✅ |
| 17 | Line | add_line | ✅ |
| 18 | Arc From Center | add_arc | ✅ |
| 19 | Arc From 3 Points | add_arc_3pt | ✅ |
| 20 | Elliptical Arc | — | ❌ |
| 21 | Hyperbolic Arc | — | ❌ |
| 22 | Parabolic Arc | — | ❌ |
| 23 | Circle From Center | add_circle | ✅ |
| 24 | Circle From 3 Points | — | ❌ |
| 25 | Ellipse From Center | add_ellipse | ✅ |
| 26 | Ellipse From 3 Points | — | ❌ |
| 27 | Rectangle | GUI rectangle | ✅ |
| 28 | Centered Rectangle | — | ❌ |
| 29 | Rounded Rectangle | — | ❌ |
| 30 | Triangle | add_regular_polygon(3) | 🔶 |
| 31 | Square | add_regular_polygon(4) | 🔶 |
| 32 | Pentagon | add_regular_polygon(5) | 🔶 |
| 33 | Hexagon | add_regular_polygon(6) | 🔶 |
| 34 | Heptagon | add_regular_polygon(7) | 🔶 |
| 35 | Octagon | add_regular_polygon(8) | 🔶 |
| 36 | Polygon (N-sided) | add_regular_polygon | ✅ |
| 37 | Slot | — | ❌ |
| 38 | Arc Slot | — | ❌ |
| 39 | B-Spline | add_bspline | ✅ |
| 40 | Periodic B-Spline | — | ❌ |
| 41 | B-Spline From Knots | — | ❌ |
| 42 | Periodic B-Spline From Knots | — | ❌ |
| 43 | Toggle Construction Geometry | — | ❌ |

#### Dimensional Constraints (9)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 44 | Dimension (contextual) | — | ❌ |
| 45 | Horizontal Dimension | HorizontalDistance | ✅ |
| 46 | Vertical Dimension | VerticalDistance | ✅ |
| 47 | Distance Dimension | Distance | ✅ |
| 48 | Radius Dimension | Radius | ✅ |
| 49 | Diameter Dimension | Diameter | ✅ |
| 50 | Angle Dimension | Angle | ✅ |
| 51 | Lock Position | Fixed | ✅ |
| 52 | Radius/Diameter (unified) | — | 🔶 |

#### Geometric Constraints (13)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 53 | Coincident (unified) | Coincident | ✅ |
| 54 | Point-On-Object | PointOnObject | ✅ |
| 55 | Horizontal/Vertical (unified) | — | 🔶 |
| 56 | Horizontal | Horizontal | ✅ |
| 57 | Vertical | Vertical | ✅ |
| 58 | Parallel | Parallel | ✅ |
| 59 | Perpendicular | Perpendicular | ✅ |
| 60 | Tangent/Collinear | Tangent + Collinear | ✅ |
| 61 | Equal | EqualLength + EqualRadius | ✅ |
| 62 | Symmetric | Symmetric | ✅ |
| 63 | Block | Block | ✅ |
| 64 | Refraction (Snell) | — | ❌ |
| 65 | Toggle Driving/Reference | — | ❌ |

#### Sketcher Tools (22)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 66 | Fillet (sketch) | — | ❌ |
| 67 | Chamfer (sketch) | — | ❌ |
| 68 | Trim Edge | — | ❌ |
| 69 | Split Edge | — | ❌ |
| 70 | Extend Edge | — | ❌ |
| 71 | External Projection | — | ❌ |
| 72 | External Intersection | — | ❌ |
| 73 | Carbon Copy | — | ❌ |
| 74 | Select Origin | — | ❌ |
| 75 | Select H/V Axis | — | ❌ |
| 76 | Move/Array Transform | — | ❌ |
| 77 | Rotate/Polar Transform | — | ❌ |
| 78 | Scale | — | ❌ |
| 79 | Offset | — | ❌ |
| 80 | Mirror | — | ❌ |
| 81 | Remove Axes Alignment | — | ❌ |
| 82 | Delete All Geometry | — | ❌ |
| 83 | Delete All Constraints | — | ❌ |
| 84 | Copy/Cut/Paste | — | ❌ |
| 85 | Toggle Constraints | — | ❌ |

#### B-Spline Tools (7)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 86 | Geometry to B-Spline | — | ❌ |
| 87 | Increase B-Spline Degree | — | ❌ |
| 88 | Decrease B-Spline Degree | — | ❌ |
| 89 | Increase Knot Multiplicity | — | ❌ |
| 90 | Decrease Knot Multiplicity | — | ❌ |
| 91 | Insert Knot | — | ❌ |
| 92 | Join Curves | — | ❌ |

#### Visual Helpers (13)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 93–105 | All display toggles | — | ❌ |

**Sketcher Summary: 30/109 implemented (28%), 72 missing, 7 partial**

---

### 2.4 TechDraw Workbench (FreeCAD: ~114 tools)

#### Pages (7)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | New Page | DrawingSheet | ✅ |
| 2 | New Page From Template | — | ❌ |
| 3 | Update Template Fields | — | ❌ |
| 4 | Redraw Page | — | ❌ |
| 5 | Print All Pages | — | ❌ |
| 6 | Export Page as SVG | drawing_to_svg | ✅ |
| 7 | Export Page as DXF | export_dxf | 🔶 |

#### Views (12)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 8 | New View | project_solid | ✅ |
| 9 | Broken View | — | ❌ |
| 10 | Section View | section_view | ✅ |
| 11 | Complex Section View | — | ❌ |
| 12 | Detail View | detail_view | ✅ |
| 13 | Projection Group | three_view_drawing | ✅ |
| 14 | Clip Group | — | ❌ |
| 15 | Insert SVG | — | ❌ |
| 16 | Bitmap Image | — | ❌ |
| 17 | Share View | — | ❌ |
| 18 | Project Shape | — | ❌ |
| 19 | Active View | — | ❌ |

#### Dimensions (12)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 20 | Dimension (contextual) | — | ❌ |
| 21 | Length Dimension | DimensionType::Linear | ✅ |
| 22 | Horizontal Length | DimensionType::HorizontalDistance | ✅ |
| 23 | Vertical Length | DimensionType::VerticalDistance | ✅ |
| 24 | Radius Dimension | DimensionType::Radius | ✅ |
| 25 | Diameter Dimension | DimensionType::Diameter | ✅ |
| 26 | Angle Dimension | DimensionType::Angle | ✅ |
| 27 | Angle From 3 Points | — | ❌ |
| 28 | Area Annotation | — | ❌ |
| 29 | H/V Extent Dimension | — | ❌ |
| 30 | Arc Length Dimension | — | ❌ |
| 31 | Repair Dimension References | — | ❌ |

#### Hatching (2)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 32 | Image Hatch | HatchPattern | ✅ |
| 33 | Geometric Hatch | — | ❌ |

#### Symbols (3)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 34 | Weld Symbol | — | ❌ |
| 35 | Surface Finish Symbol | SurfaceFinishSymbol | ✅ |
| 36 | Hole/Shaft Fit | — | ❌ |

#### Annotations (4)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 37 | Text Annotation | TextAnnotation | ✅ |
| 38 | Rich Text Annotation | — | ❌ |
| 39 | Balloon Annotation | — | ❌ |
| 40 | Axonometric Length | — | ❌ |

#### Add Lines / Centerlines (15)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 41 | Leader Line | LeaderLine | ✅ |
| 42 | Centerline on Face | — | ❌ |
| 43 | Centerline Between 2 Lines | — | ❌ |
| 44 | Centerline Between 2 Points | — | ❌ |
| 45 | Cosmetic Line Through 2 Points | — | ❌ |
| 46 | Circle Centerlines | CenterMark | ✅ |
| 47 | Bolt Circle Centerlines | — | ❌ |
| 48 | Cosmetic Thread (4 types) | — | ❌ |
| 49 | Cosmetic Vertices (3 types) | — | ❌ |
| 50 | Edit Line Appearance | — | ❌ |
| 51 | Toggle Edge Visibility | — | ❌ |
| 52 | Cosmetic Circle (3 types) | — | ❌ |
| 53 | Cosmetic Arc | — | ❌ |
| 54 | Cosmetic Parallel Line | — | ❌ |
| 55 | Cosmetic Perpendicular Line | — | ❌ |

#### Dimension Formatting (16)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 56–71 | Chain/Coordinate/Chamfer dims, Prefix symbols, Decimal places | — | ❌ |

#### Stacking / Alignment / Attributes (16)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 72–87 | Stack order, Align, Lock, Position section, Line attributes | — | ❌ |

**TechDraw Summary: 15/114 implemented (13%), 99 missing**

---

### 2.5 Assembly Workbench (FreeCAD: ~23 tools)

| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | New Assembly | Assembly | ✅ |
| 2 | Component | Component | ✅ |
| 3 | New Part | — | ❌ |
| 4 | Solve Assembly | — | ❌ |
| 5 | Exploded View | exploded_view | ✅ |
| 6 | Simulation | — | ❌ |
| 7 | Bill of Materials | bill_of_materials | ✅ |
| 8 | Export ASMT File | — | ❌ |
| 9 | Toggle Grounded (Fixed) | Fixed constraint | ✅ |
| 10 | Fixed Joint | Fixed | ✅ |
| 11 | Revolute Joint | Revolute | 🔶 |
| 12 | Cylindrical Joint | Cylindrical | 🔶 |
| 13 | Slider Joint | Prismatic | 🔶 |
| 14 | Ball Joint | Ball | 🔶 |
| 15 | Distance Joint | Distance | ✅ |
| 16 | Parallel Joint | — | ❌ |
| 17 | Perpendicular Joint | — | ❌ |
| 18 | Angle Joint | Angle | ✅ |
| 19 | Rack and Pinion | RackAndPinion | 🔶 |
| 20 | Screw Joint | Screw | 🔶 |
| 21 | Gears Joint | Gears | 🔶 |
| 22 | Belt Joint | Belt | 🔶 |
| 23 | Preferences | — | ❌ |

**Assembly Summary: 10/23 implemented (43%), 5 missing, 8 partial (joint types defined but no solver)**

---

### 2.6 Mesh Workbench (FreeCAD: ~35 tools)

| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | Import Mesh | import_stl/obj | ✅ |
| 2 | Export Mesh | export_stl/obj/gltf | ✅ |
| 3 | Mesh From Shape | tessellate_solid | ✅ |
| 4 | Refinement (Remesh) | — | ❌ |
| 5 | Regular Solid | — | ❌ |
| 6 | Unwrap Mesh | — | ❌ |
| 7 | Unwrap Face | — | ❌ |
| 8 | Evaluate and Repair | check_mesh_watertight | 🔶 |
| 9 | Face Info | — | ❌ |
| 10 | Curvature Info | compute_curvature | ✅ |
| 11 | Evaluate Solid | check_mesh_watertight | ✅ |
| 12 | Bounding Box Info | — | ❌ |
| 13 | Curvature Plot | — | ❌ |
| 14 | Harmonize Normals | harmonize_normals | ✅ |
| 15 | Flip Normals | flip_normals | ✅ |
| 16 | Fill Holes | fill_holes | ✅ |
| 17 | Close Holes | — | ❌ |
| 18 | Add Triangle | — | ❌ |
| 19 | Remove Components | — | ❌ |
| 20 | Remove Components Manually | — | ❌ |
| 21 | Smooth | smooth_mesh | ✅ |
| 22 | Decimate | decimate_mesh | ✅ |
| 23 | Scale | — | 🔶 |
| 24 | Union (mesh boolean) | mesh_boolean_union | ✅ |
| 25 | Intersection (mesh boolean) | — | ❌ |
| 26 | Difference (mesh boolean) | — | ❌ |
| 27 | Cut | cut_mesh_with_plane | 🔶 |
| 28 | Trim | — | ❌ |
| 29 | Trim With Plane | cut_mesh_with_plane | ✅ |
| 30 | Section From Plane | mesh_section_from_plane | ✅ |
| 31 | Cross-Sections | — | ❌ |
| 32 | Merge | merge_meshes | ✅ |
| 33 | Split by Components | split_mesh_by_components | ✅ |
| 34 | Segmentation | — | ❌ |
| 35 | Segmentation (Best-Fit) | — | ❌ |

**Mesh Summary: 16/35 implemented (46%), 15 missing, 4 partial**

---

### 2.7 Surface Workbench (FreeCAD: 6 tools)

| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | Filling (N-sided patch) | — | ❌ |
| 2 | Fill Boundary Curves (Coons) | surface_from_curves | 🔶 |
| 3 | Sections (Skinning) | — | ❌ |
| 4 | Extend Face | extend_surface | ✅ |
| 5 | Curve on Mesh | — | ❌ |
| 6 | Blend Curve | blend_curve | ✅ |

**Surface Summary: 2/6 implemented (33%), 3 missing, 1 partial**

---

### 2.8 Draft Workbench (FreeCAD: ~80 tools)

#### Drafting (16)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 1 | Line | make_wire (2 pts) | 🔶 |
| 2 | Polyline (Wire) | make_wire | ✅ |
| 3 | Fillet | make_fillet_wire | ✅ |
| 4 | Arc | make_arc_wire | ✅ |
| 5 | Arc From 3 Points | — | ❌ |
| 6 | Circle | make_circle_wire | ✅ |
| 7 | Ellipse | — | ❌ |
| 8 | Rectangle | — | ❌ |
| 9 | Polygon | — | ❌ |
| 10 | B-Spline | make_bspline_wire | ✅ |
| 11 | Cubic Bézier Curve | — | ❌ |
| 12 | Bézier Curve | — | ❌ |
| 13 | Point | — | ❌ |
| 14 | Facebinder | — | ❌ |
| 15 | Shape From Text | — | ❌ |
| 16 | Hatch | — | ❌ |

#### Annotation (4)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 17 | Text | make_dimension_text | 🔶 |
| 18 | Dimension | — | ❌ |
| 19 | Label | — | ❌ |
| 20 | Annotation Styles | — | ❌ |

#### Modification (22)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 21 | Move | — | ❌ |
| 22 | Rotate | — | ❌ |
| 23 | Scale | — | ❌ |
| 24 | Mirror | — | ❌ |
| 25 | Offset | — | ❌ |
| 26 | Trimex | — | ❌ |
| 27 | Stretch | — | ❌ |
| 28 | Clone | clone_solid | ✅ |
| 29 | Array (Ortho) | rectangular_array | ✅ |
| 30 | Polar Array | polar_array | ✅ |
| 31 | Circular Array | — | ❌ |
| 32 | Path Array | path_array | ✅ |
| 33 | Path Link Array | — | ❌ |
| 34 | Point Array | point_array | ✅ |
| 35 | Point Link Array | — | ❌ |
| 36 | Edit | — | ❌ |
| 37 | Join | — | ❌ |
| 38 | Split | — | ❌ |
| 39 | Upgrade | — | ❌ |
| 40 | Downgrade | — | ❌ |
| 41 | Convert Wire/B-Spline | — | ❌ |
| 42 | Draft to Sketch | — | ❌ |

#### Snap Tools (16)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 43–58 | Snap Lock, Endpoint, Midpoint, Center, Angle, Intersection, etc. | — | ❌ |

#### Utilities / Working Plane / Layers (12+)
| # | FreeCAD Tool | CADKernel | Status |
|---|-------------|-----------|:------:|
| 59–80 | Layers, Working Plane, Styles, etc. | — | ❌ |

**Draft Summary: 10/80 implemented (13%), 66 missing, 4 partial**

---

### 2.9 FEM Workbench (FreeCAD: ~80+ tools)

| Category | FreeCAD | CADKernel | Status |
|----------|:-------:|-----------|:------:|
| Analysis container | 1 | — | ❌ |
| Materials | 5 | FemMaterial (2 presets) | 🔶 |
| Element Geometry | 4 | — | ❌ |
| EM Boundary Conditions | 4 | — | ❌ |
| Fluid Boundary Conditions | 3 | — | ❌ |
| Geometrical Features | 3 | — | ❌ |
| Mechanical Constraints | 6 | BoundaryCondition (3 types) | 🔶 |
| Mechanical Loads | 4 | Force in BC | 🔶 |
| Thermal Constraints/Loads | 4 | — | ❌ |
| Mesh Generation | 7 | generate_tet_mesh | 🔶 |
| Solvers | 4 | Gauss-Seidel | 🔶 |
| Equations | 9 | Elasticity only | 🔶 |
| Post-Processing | 15 | FemResult (stress/disp) | 🔶 |
| Filter Functions | 4 | — | ❌ |
| Visualization | 3 | — | ❌ |
| Utilities | 6 | — | ❌ |

**FEM Summary: 3/80 implemented (4%), basic framework only**

---

### 2.10 File Format Support

| Format | Import | Export | Status |
|--------|:------:|:------:|:------:|
| STL | ✅ | ✅ | ✅ |
| OBJ | ✅ | ✅ | ✅ |
| glTF/glb | ❌ | ✅ | 🔶 |
| SVG | ❌ | ✅ | 🔶 |
| JSON | ✅ | ✅ | ✅ |
| CADK (native) | ✅ | ✅ | ✅ |
| STEP | ✅ | ✅ | ✅ |
| IGES | ✅ | ✅ | ✅ |
| DXF | ✅ | ✅ | ✅ |
| PLY | ✅ | ✅ | ✅ |
| 3MF | ❌ | ✅ | 🔶 |
| BREP | ✅ | ✅ | ✅ |
| DWG | ❌ | ❌ | ❌ |
| PDF | ❌ | ❌ | ❌ |
| DAE (Collada) | ❌ | ❌ | ❌ |
| VRML (.wrl) | ❌ | ❌ | ❌ |
| AMF | ❌ | ❌ | ❌ |
| OCA/GCAD | ❌ | ❌ | ❌ |

**I/O Summary: 9/18 full, 4 partial, 5 missing**

---

## Summary Table

| Workbench | FreeCAD Tools | CADKernel | Coverage |
|-----------|:----------:|:----------:|:--------:|
| Part | ~58 | 37 | **64%** |
| PartDesign | ~53 | 31 | **58%** |
| Sketcher | ~109 | 30 | **28%** |
| TechDraw | ~114 | 15 | **13%** |
| Assembly | ~23 | 10 | **43%** |
| Mesh | ~35 | 16 | **46%** |
| Surface | 6 | 2 | **33%** |
| Draft | ~80 | 10 | **13%** |
| FEM | ~80 | 3 | **4%** |
| I/O Formats | 18 | 13 | **72%** |
| **Total** | **~576** | **~167** | **29%** |

---

## 3. Implementation Roadmap

### Priority Tiers

**Tier 1 — Core CAD Kernel (Critical Path)**
These are the features that make CADKernel a usable CAD system.

| Phase | Name | Gap Count | Priority |
|-------|------|:---------:|:--------:|
| V1 | Sketcher Completion | ~72 | Critical |
| V2 | PartDesign Completion | ~18 | Critical |
| V3 | Part Workbench Completion | ~14 | Critical |

**Tier 2 — Engineering Tooling (High Value)**
Features that enable production use.

| Phase | Name | Gap Count | Priority |
|-------|------|:---------:|:--------:|
| V4 | TechDraw Completion | ~99 | High |
| V5 | Assembly Solver & Joints | ~13 | High |
| V6 | Surface Workbench Completion | ~3 | High |
| V7 | File Format Expansion | ~5 | High |

**Tier 3 — Specialist Workbenches (Medium)**
Full-featured specialist tools.

| Phase | Name | Gap Count | Priority |
|-------|------|:---------:|:--------:|
| V8 | Mesh Workbench Completion | ~15 | Medium |
| V9 | Draft Workbench | ~66 | Medium |
| V10 | FEM Workbench | ~70+ | Medium |

**Tier 4 — Viewer & Polish**

| Phase | Name | Priority |
|-------|------|:--------:|
| V11 | Viewer UI — All Operations in Toolbar | High |
| V12 | Python Bindings | Medium |
| V13 | Performance & Validation | Final |

---

## 4. Phase Details

---

### Phase V1: Sketcher Completion (~72 gaps)

> **Why first**: The sketcher is the foundation of parametric modeling. PartDesign features depend on sketches. FreeCAD's sketcher has 109 tools — we have 30.

#### V1.1 Geometry Creation (12 missing)
- Elliptical Arc, Hyperbolic Arc, Parabolic Arc
- Circle From 3 Points, Ellipse From 3 Points
- Centered Rectangle, Rounded Rectangle
- Slot, Arc Slot
- Periodic B-Spline, B-Spline From Knots, Periodic B-Spline From Knots
- Construction Geometry flag

#### V1.2 Constraint Additions (2 missing)
- Refraction (Snell's Law) constraint
- Toggle Driving/Reference mode

#### V1.3 Sketcher Tools (19 missing)
- Fillet (sketch), Chamfer (sketch)
- Trim Edge, Split Edge, Extend Edge
- External Projection, External Intersection, Carbon Copy
- Move/Array, Rotate/Polar, Scale, Offset, Mirror
- Toggle/Delete constraints tools
- Copy/Cut/Paste

#### V1.4 B-Spline Tools (7 missing)
- Geometry to B-Spline conversion
- Increase/Decrease Degree
- Increase/Decrease Knot Multiplicity
- Insert Knot, Join Curves

#### V1.5 Sketch Management (8 missing)
- Attach Sketch, Reorient Sketch
- Validate Sketch, Merge Sketches, Mirror Sketch
- Grid, Snap, Rendering Order

#### V1.6 Visual Helpers (13 missing)
- All 13 display/select toggles (lower priority — GUI features)

#### Tests: ~60 new tests expected

---

### Phase V2: PartDesign Completion (~18 gaps)

#### V2.1 Missing Additive/Subtractive (10)
- Additive/Subtractive Helix (sweep along helix path)
- Additive/Subtractive Ellipsoid
- Additive/Subtractive Prism
- Additive/Subtractive Wedge
- Additive/Subtractive Loft (integrated with Body)
- Additive/Subtractive Pipe (integrated with Body)

#### V2.2 Structure Tools (4)
- Attach Sketch, Validate Sketch
- Sub-Shape Binder, Shape Binder

#### V2.3 Context Menu (4)
- Suppressed (feature suppression)
- Set Tip
- Move Object To Body
- Move Feature After

#### V2.4 Additional Tools (2)
- Sprocket profile generator
- Shaft Design Wizard

#### Tests: ~20 new tests expected

---

### Phase V3: Part Workbench Completion (~14 gaps)

#### V3.1 Missing Primitives (4)
- Circle (arc shape), Ellipse (arc shape), Point (shape), Line (shape)

#### V3.2 Missing Operations (7)
- Face From Wires, Shape Builder
- Explode Compound, Compound Filter, Slice to Compound
- Boolean Fragments
- Appearance per Face, Attachment

#### V3.3 Join Operations (3)
- Connect Shapes, Embed Shapes, Cutout Shape

#### V3.4 Conversion (2)
- Points From Shape, Convert to Solid

#### Tests: ~15 new tests expected

---

### Phase V4: TechDraw Completion (~99 gaps)

> This is the largest gap. FreeCAD's TechDraw has 114 tools.

#### V4.1 Views (7)
- Broken View, Complex Section View
- Clip Group, Insert SVG, Bitmap Image
- Share View, Project Shape, Active View

#### V4.2 Dimensions (5)
- Contextual Dimension, Angle From 3 Points
- Area Annotation, Arc Length, H/V Extent

#### V4.3 Centerlines & Cosmetics (25)
- All centerline types, cosmetic threads, cosmetic vertices
- Cosmetic circles/arcs/lines

#### V4.4 Annotations (5)
- Rich Text, Balloon, Axonometric Length
- Weld Symbol, Hole/Shaft Fit

#### V4.5 Dimension Formatting (16)
- Chain/Coordinate/Chamfer dimensions
- Prefix symbols, decimal places

#### V4.6 Stacking/Alignment/Attributes (16)
- Stack order, alignment, line attributes

#### V4.7 Templates & Output (5)
- Page templates, DXF export, Print support

#### Tests: ~30 new tests expected

---

### Phase V5: Assembly Solver & Joints (~13 gaps)

#### V5.1 Assembly Solver
- 6-DOF constraint solver (Newton-Raphson)
- Under/Over-constrained detection
- DOF counting

#### V5.2 Missing Joints (2)
- Parallel Joint, Perpendicular Joint

#### V5.3 Joint Implementation (8 partial)
- All 8 partial joints need actual constraint equations
- Currently defined as enum variants only

#### V5.4 Additional Features (3)
- New Part (within assembly)
- Simulation (kinematic)
- Export ASMT

#### Tests: ~15 new tests expected

---

### Phase V6: Surface Workbench Completion (~3 gaps)

- Filling (N-sided patch) — energy minimization
- Sections (Skinning) — surface through cross-sections
- Curve on Mesh — spline approximation on mesh
- Improve Fill Boundary Curves (full Coons/Gordon)

#### Tests: ~8 new tests expected

---

### Phase V7: File Format Expansion (~5 gaps)

- glTF import
- 3MF import
- DWG (via conversion or native)
- PDF export (TechDraw pages)
- DAE (Collada) import/export

#### Tests: ~10 new tests expected

---

### Phase V8: Mesh Workbench Completion (~15 gaps)

- Refinement (remesh), Regular Solid
- Unwrap Mesh, Unwrap Face
- Evaluate and Repair (full)
- Close Holes, Add Triangle
- Remove Components
- Mesh Boolean Intersection/Difference
- Cut, Trim
- Cross-Sections, Segmentation (2 types)
- Bounding Box Info, Face Info, Curvature Plot
- Scale Mesh

#### Tests: ~15 new tests expected

---

### Phase V9: Draft Workbench (~66 gaps)

#### V9.1 Drafting Tools (10)
- Ellipse, Rectangle, Polygon, Point
- Arc From 3 Points, Cubic Bézier, Bézier
- Facebinder, Shape From Text, Hatch

#### V9.2 Annotation (3)
- Dimension, Label, Annotation Styles

#### V9.3 Modification Tools (15)
- Move, Rotate, Scale, Mirror, Offset, Trimex, Stretch
- Join, Split, Upgrade, Downgrade
- Edit, Convert Wire/B-Spline, Draft to Sketch

#### V9.4 Array Extensions (4)
- Circular Array, Path/Point Link Arrays

#### V9.5 Snap System (16)
- All 16 snap modes

#### V9.6 Utilities/Layers (12)
- Layer management, Working Plane, Styles

#### Tests: ~20 new tests expected

---

### Phase V10: FEM Workbench (~70+ gaps)

> Lowest priority for a CAD kernel. FEM is heavily solver-dependent.

#### V10.1 Mesh Generation
- Netgen/Gmsh integration or native tet meshing improvements
- Boundary layer mesh, mesh refinement regions

#### V10.2 Materials
- Material database expansion
- Nonlinear, fluid, reinforced materials

#### V10.3 Boundary Conditions (20+)
- Full mechanical, thermal, EM, fluid boundary conditions

#### V10.4 Solvers
- CalculiX INP export
- Elmer SIF export
- Result file parsing

#### V10.5 Post-Processing (15)
- Color maps, deformation visualization
- Filter functions, linearization

---

### Phase V11: Viewer UI

> All new operations accessible from toolbar

- All 13 primitives in Part toolbar with creation dialogs
- All PartDesign additive/subtractive ops in toolbar
- All boolean operations accessible
- Sketcher tools (all geometry, constraints, editing)
- TechDraw dimensions and annotations
- Assembly joints UI
- Mesh operations UI
- Draft tools UI

---

### Phase V12: Python Bindings

- PyO3 bindings for all public APIs
- Jupyter notebook integration
- Python scripting console in viewer

---

### Phase V13: Performance & Validation

- BVH optimization for large models
- Parallel boolean operations
- Memory optimization for assemblies
- Comprehensive geometry validation
- Stress testing (1000+ face booleans)
- Benchmark suite expansion

---

## 5. Verification Criteria

### Per-Phase Checklist
1. `cargo build --workspace` — zero errors
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings` — zero warnings
3. `cargo test --workspace` — all tests pass
4. Documentation updated (CHANGELOG.md, docs/CHANGELOG.ko.md, docs/DEVELOPER_WIKI.md, docs/DEVELOPER_WIKI.ko.md)
5. New tests for every new function

### Target Metrics

| Metric | Current | Target |
|--------|:-------:|:------:|
| Tests | 609 | 1000+ |
| Clippy warnings | 0 | 0 |
| Feature coverage | 29% (~167/576) | 80%+ (~460/576) |
| Workbenches | 6 | 6 (all complete) |
| File formats | 13/18 | 16/18 |
| Primitives | 13 | 17+ |
| Feature ops | 24 | 35+ |
| Sketch entities | 6 | 12+ |
| Sketch constraints | 24 | 26+ |
| Assembly joints | 10 (8 partial) | 14 (all functional) |
| NURBS ops | 28 (full) | 28 |

---

## Appendix: FreeCAD Feature Count (Updated 2026-03-14)

| Workbench | FreeCAD | Implemented | Partial | Missing | Coverage |
|-----------|:-------:|:-----------:|:-------:|:-------:|:--------:|
| Part | 58 | 37 | 7 | 14 | 64% |
| PartDesign | 53 | 31 | 4 | 18 | 58% |
| Sketcher | 109 | 30 | 7 | 72 | 28% |
| TechDraw | 114 | 15 | 0 | 99 | 13% |
| Assembly | 23 | 10 | 8 | 5 | 43% |
| Mesh | 35 | 16 | 4 | 15 | 46% |
| Surface | 6 | 2 | 1 | 3 | 33% |
| Draft | 80 | 10 | 4 | 66 | 13% |
| FEM | 80 | 3 | 7 | 70 | 4% |
| I/O | 18 | 13 | 4 | 5 | 72% |
| **Total** | **576** | **167** | **46** | **367** | **29%** |
