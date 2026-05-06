# STEP AP242 Mapping

**Status:** spec v1, last updated 2026-05-06
**Owner:** io-engineer (`crates/io`)
**Implements:** roadmap §7 (file-format / interop)
**Crate:** `cadkernel-io::step`

---

## 1. Scope

This document defines how CADKernel B-Rep entities map to/from ISO 10303-242:2020 (STEP AP242 ed.2) entities. AP214 (older automotive AP) is supported as a read-only subset. Both AP214 and AP242 share most geometric entities; AP242 adds PMI, GD&T, model-based engineering features, and CAE references.

References to AP242 entities use the ISO 10303-21 STEP form: `#NNN=ENTITY_NAME(args)`.

---

## 2. Entity mapping table

| CADKernel | AP242 entity | Notes |
|---|---|---|
| `Solid` | `MANIFOLD_SOLID_BREP` | Outer shell only; if shells > 1 (voids), use `BREP_WITH_VOIDS` |
| `Shell` | `CLOSED_SHELL` (outer) / `ORIENTED_CLOSED_SHELL` (oriented) | Closed shells only; use `OPEN_SHELL` for surface bodies |
| `Face` | `ADVANCED_FACE` | Wraps surface + outer/inner bound loops |
| `Loop` (outer) | `FACE_OUTER_BOUND` | Outer wire of an `ADVANCED_FACE` |
| `Loop` (inner) | `FACE_BOUND` | Inner (hole) wire |
| `Loop` (wire form) | `EDGE_LOOP` | Container for ordered `ORIENTED_EDGE`s |
| `HalfEdge` | `ORIENTED_EDGE` | Wraps an `EDGE_CURVE` with orientation flag |
| `Edge` | `EDGE_CURVE` | References geometry curve + start/end vertices |
| `Vertex` | `VERTEX_POINT` + `CARTESIAN_POINT` | Point in 3D space |
| `Curve::Line` | `LINE` + `VECTOR` + `CARTESIAN_POINT` | Line through point with direction |
| `Curve::Circle` | `CIRCLE` + `AXIS2_PLACEMENT_3D` | Circle with placement (centre + normal + reference dir) |
| `Curve::Ellipse` | `ELLIPSE` + `AXIS2_PLACEMENT_3D` | Semi-major + semi-minor + placement |
| `Curve::Parabola` | `PARABOLA` + `AXIS2_PLACEMENT_3D` | Focal distance + placement |
| `Curve::Hyperbola` | `HYPERBOLA` + `AXIS2_PLACEMENT_3D` | Semi-axis + placement |
| `Curve::Nurbs` (rational) | `RATIONAL_B_SPLINE_CURVE` (complex inheritance with `B_SPLINE_CURVE_WITH_KNOTS`) | Degree, knots, weights, control points |
| `Curve::Nurbs` (non-rational) | `B_SPLINE_CURVE_WITH_KNOTS` | Same as above but no `WEIGHTS_DATA` |
| `Curve::Composite` | `COMPOSITE_CURVE` + `COMPOSITE_CURVE_SEGMENT` | Sequence of segments |
| `Surface::Plane` | `PLANE` + `AXIS2_PLACEMENT_3D` | Implicit infinite plane (trimmed by face boundary) |
| `Surface::Cylinder` | `CYLINDRICAL_SURFACE` + `AXIS2_PLACEMENT_3D` | Axis + radius |
| `Surface::Sphere` | `SPHERICAL_SURFACE` + `AXIS2_PLACEMENT_3D` | Centre + radius |
| `Surface::Cone` | `CONICAL_SURFACE` + `AXIS2_PLACEMENT_3D` | Apex + half-angle + ref radius |
| `Surface::Torus` | `TOROIDAL_SURFACE` + `AXIS2_PLACEMENT_3D` | Major + minor radius |
| `Surface::Revolve` | `SURFACE_OF_REVOLUTION` + `AXIS1_PLACEMENT` + meridian curve | |
| `Surface::Extrude` | `SURFACE_OF_LINEAR_EXTRUSION` + base curve + direction | |
| `Surface::Ruled` | `B_SPLINE_SURFACE_WITH_KNOTS` (degree 1 in v) | No native ruled-surface entity in AP242 |
| `Surface::Nurbs` (rational) | `RATIONAL_B_SPLINE_SURFACE` (complex) + `B_SPLINE_SURFACE_WITH_KNOTS` | Bi-degree, knots, weights, control net |
| `Surface::Nurbs` (non-rational) | `B_SPLINE_SURFACE_WITH_KNOTS` | |
| `Surface::Trimmed` | underlying surface + `ADVANCED_FACE` bounds | Trimming is at face level |
| Document parameter | `PROPERTY_DEFINITION` + `MEASURE_REPRESENTATION_ITEM` | AP242 user-defined properties |
| Document assembly | `NEXT_ASSEMBLY_USAGE_OCCURRENCE` + `PRODUCT_DEFINITION_RELATIONSHIP` | Hierarchy via `PRODUCT_DEFINITION` tree |
| Material | `PRODUCT_RELATED_PRODUCT_CATEGORY` + `PROPERTY_DEFINITION` | Material as property |
| Tag (persistent name) | (proprietary EXT in step header `/* CADKERNEL-TAG: ... */`) | AP242 has no native persistent name; we round-trip via comment |
| PMI dimension | `DRAUGHTING_CALLOUT` + `MEASURE_REPRESENTATION_ITEM` | AP242 ed.2 only |
| PMI tolerance | `GEOMETRIC_TOLERANCE` (e.g., `FLATNESS_TOLERANCE`) + datum reference | AP242 GD&T |
| GD&T datum | `DATUM_FEATURE` + `DATUM` | AP242 datum |

---

## 3. Worked example — primitive box (R1)

CADKernel `Solid::Box(2, 1, 0.5)` exports as:

```
ISO-10303-21;
HEADER;
FILE_DESCRIPTION(('cadkernel R1 box'),'2;1');
FILE_NAME('R1.step','2026-05-06T00:00:00',
  ('cadkernel'),('cadkernel'),'cadkernel 1.0.0','cadkernel 1.0.0','');
FILE_SCHEMA(('AUTOMOTIVE_DESIGN { 1 0 10303 442 1 1 4 }'));
ENDSEC;

DATA;
#1=APPLICATION_PROTOCOL_DEFINITION('international standard',
   'automotive_design',2010,#2);
#2=APPLICATION_CONTEXT('core data for automotive mechanical design processes');
#3=PRODUCT_DEFINITION_CONTEXT('part definition',#2,'design');
#4=PRODUCT_CONTEXT('',#2,'mechanical');
#5=PRODUCT('R1_box','R1','',(#4));
#6=PRODUCT_DEFINITION_FORMATION('','',#5);
#7=PRODUCT_DEFINITION('design','',#6,#3);
#8=PRODUCT_DEFINITION_SHAPE('','',#7);
#10=CARTESIAN_POINT('',(0.0, 0.0, 0.0));
#11=CARTESIAN_POINT('',(2.0, 0.0, 0.0));
#12=CARTESIAN_POINT('',(2.0, 1.0, 0.0));
#13=CARTESIAN_POINT('',(0.0, 1.0, 0.0));
#14=CARTESIAN_POINT('',(0.0, 0.0, 0.5));
#15=CARTESIAN_POINT('',(2.0, 0.0, 0.5));
#16=CARTESIAN_POINT('',(2.0, 1.0, 0.5));
#17=CARTESIAN_POINT('',(0.0, 1.0, 0.5));
#20=VERTEX_POINT('',#10);
... (8 vertices, 12 edges, 6 faces, 6 advanced_faces, 1 closed_shell, 1 manifold_solid_brep)
ENDSEC;
END-ISO-10303-21;
```

Round-trip property: import → export → import yields identical CADKernel B-Rep within tolerance $10^{-9}$.

---

## 4. Round-trip test methodology

### 4.1 Per-part procedure

```text
fn round_trip_test(part: BRepModel) -> bool:
    # 1. Export to STEP
    step1 ← export_step(part)
    # 2. Re-import
    part2 ← import_step(step1)
    # 3. Re-export
    step2 ← export_step(part2)
    # 4. Compare structurally
    if not structurally_equal(part, part2):
        return false
    # 5. Compare STEP files (canonicalised)
    if canonicalise(step1) != canonicalise(step2):
        return false
    return true
```

### 4.2 Canonicalisation

STEP files have free entity numbering. Canonicalise by:
1. Topologically sort entities by dependency.
2. Renumber starting from #1.
3. Round all floats to 9 significant digits (well above $10^{-9}$ tolerance).

### 4.3 Structural equality

Two B-Reps are structurally equal iff:
- Same number of solids, shells, faces, edges, vertices.
- Topology graphs isomorphic (canonical ordering by spatial hash).
- Geometry agrees within tolerance:
  - Vertex positions: $\|p_1 - p_2\| < 10^{-9}$
  - Curve sample points: $\|c_1(t) - c_2(t)\| < 10^{-9}$ at 64 chord-uniform samples
  - Surface sample points: $\|s_1(u, v) - s_2(u, v)\| < 10^{-9}$ at $32 \times 32$ UV grid

---

## 5. AP214 vs AP242 differences

| Feature | AP214 | AP242 |
|---|---|---|
| Geometry entities | Same | Same |
| PMI / GD&T | None | Native (`GEOMETRIC_TOLERANCE`, `DATUM_FEATURE`, etc.) |
| Tessellation embedding | Not standard | `TESSELLATED_SOLID` etc. |
| User-defined properties | Limited | Extended (CBOR-like attribute model) |
| Composite / additive manufacturing | Not supported | Native entities |
| File-level signatures | None | `STEP_FILE_PROVENANCE` |
| Encoding | ASCII (Part 21) | ASCII (Part 21) or XML (Part 28) |

CADKernel writes AP242 by default. AP214 exporter available via `cadkernel-io::step::export_ap214()` for legacy compatibility — drops PMI, retains geometry.

---

## 6. PMI / GD&T export

Roadmap milestone v3+. Mapping:

| GD&T concept | AP242 entity |
|---|---|
| Datum feature | `DATUM_FEATURE` |
| Datum reference frame | `DATUM_REFERENCE_FRAME` |
| Flatness tolerance | `FLATNESS_TOLERANCE` |
| Perpendicularity | `PERPENDICULARITY_TOLERANCE` |
| Position (true position) | `POSITION_TOLERANCE` |
| Profile of surface | `SURFACE_PROFILE_TOLERANCE` |
| Linear dimension | `DIMENSIONAL_LOCATION` + `MEASURE_REPRESENTATION_ITEM` |
| Angular dimension | `ANGULAR_LOCATION` + `MEASURE_REPRESENTATION_ITEM` |
| Note (free-text annotation) | `ANNOTATION_TEXT_OCCURRENCE` |

---

## 7. NIST CAX-IF compliance procedure

The CAX Implementor Forum publishes test models (rounds R1-R12 + recommended practices). Compliance procedure:

1. Download CAX-IF round R$N$ test models from <https://www.cax-if.org/>.
2. Import each model into CADKernel.
3. Export back to STEP.
4. Submit to CAX-IF round-trip test harness.
5. Report results to CAX-IF for next-round publication.

CADKernel target: pass rounds R1–R12 (geometry only) by v1.0; rounds R8–R12 PMI by v3.0.

---

## 8. LOTAR alignment

LOTAR (LOng-Term Archiving and Retrieval) extends AP242 for long-term preservation:

- Forbid certain entities (e.g., parametric history) — store only validated geometry.
- Embed validation properties (volume, surface area, centroid).
- Embed PMI in machine-readable form.

CADKernel exports LOTAR-compliant STEP via `cadkernel-io::step::export_lotar()`. Drops history, embeds:

```
#900=PROPERTY_DEFINITION_REPRESENTATION(#901,#902);
#901=PROPERTY_DEFINITION('Volume','validation property',#8);
#902=REPRESENTATION('',(#903),#904);
#903=MEASURE_REPRESENTATION_ITEM('volume',VOLUME_MEASURE(1.0),#910);
```

Reference: prEN 9300 LOTAR specification.

---

## 9. Failure modes

| Failure | Detection | Mitigation |
|---|---|---|
| Unsupported entity in import | unknown entity name | Skip with warning; build report |
| Non-watertight import | open shell detected | Auto-stitch within tolerance; if fails, import as surface body |
| Tolerance mismatch | exported part has self-intersections | Lower export tolerance; tessellate-then-rebuild as fallback |
| Knot vector inconsistency | descending knot vector | Reject; report parser error |
| Missing AXIS2_PLACEMENT_3D ref | dangling reference | Use default placement at origin; warn |
| ISO 10303-21 syntax violation | unparseable | Reject with line/col report |
| Schema version mismatch | requested AP242 but file is AP214 | Auto-upgrade if no PMI; warn |
| Encoding (Part 21 vs Part 28) | unsupported XML | Convert via external tool; not in scope |

---

## 10. Numerical guards

| Constant | Value | Purpose |
|---|---|---|
| `STEP_TOL_GEOM` | $10^{-9}$ | Round-trip geometry tolerance |
| `STEP_TOL_TOPO` | $10^{-7}$ | Vertex coincidence tolerance for stitching |
| `STEP_FLOAT_PRECISION` | 15 | Significant digits in output (well below f64 precision) |
| `STEP_MAX_ENTITY_COUNT` | $10^7$ | Sanity cap on file complexity |

---

## 11. Acceptance gates

### 11.1 v1.0 gate (geometry only)

- 12 primitives round-trip.
- 290-part STEP corpus (CAX-IF rounds R1-R7) imports + re-exports without geometry loss.
- Volume preservation: $|V_{\text{import}} - V_{\text{original}}| / V_{\text{original}} < 10^{-9}$ for all parts.
- 0 panics on the corpus.

### 11.2 v2.0 gate (assembly + properties)

- AP242 assembly hierarchy round-trip for parts with up to 1000 sub-components.
- User-defined properties survive round-trip.

### 11.3 v3.0 gate (PMI / GD&T)

- CAX-IF rounds R8-R12 PMI tests pass.
- Datum + tolerance round-trip.
- LOTAR-compliant export passes external LOTAR validator.

---

## 12. References

1. ISO 10303-242:2020 — Industrial automation systems and integration — Product data representation and exchange — Part 242: Application protocol: Managed model-based 3D engineering.
2. ISO 10303-214:2010 — AP214 (legacy automotive).
3. ISO 10303-21:2016 — Implementation methods: Clear text encoding of the exchange structure.
4. ISO 10303-28:2007 — Implementation methods: XML representations of EXPRESS schema and data.
5. CAX Implementor Forum: <https://www.cax-if.org/>
6. prEN 9300 — LOTAR specification series.
7. NIST AP242 Conformance Class documentation.
8. OCCT STEP I/O User's Guide. <https://dev.opencascade.org/doc/overview/html/occt_user_guides__step.html>

---

## 13. Implementation notes

- **Streaming parser.** Large STEP files (> 1 GB) parsed without loading entire file into memory.
- **Two-pass assembly.** First pass: build entity table; second pass: resolve references.
- **Tag preservation via comments.** Persistent-naming tags written as `/* CADKERNEL-TAG: ... */` immediately before each entity. Importer reads them; foreign STEP files have tags regenerated on import.
- **Determinism.** Same B-Rep → byte-identical STEP output (modulo timestamp).
- **Logging.** Per-entity errors logged with line/column for diagnostic.
