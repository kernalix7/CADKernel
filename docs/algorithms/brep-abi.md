# `.cadk` Binary ABI

**Status:** spec v1, last updated 2026-05-06
**Owner:** io-engineer (`crates/io`)
**Implements:** roadmap §7 (file-format)
**Crate:** `cadkernel-io::cadk`

---

## 1. Goals

- **Stable across versions** — v1 readers can read all future v1.x files; v1 readers reject v2+ with a clear error.
- **Lossless** — round-trip B-Rep with bit-exact geometry (no f64 → f32 lossy conversion).
- **Streaming-friendly** — sections can be read/written independently.
- **Portable** — little-endian on disk regardless of host architecture.
- **Compact** — gzip- or zstd-compressed sections by default.
- **Inspectable** — header is human-readable when prefixed with `xxd` or `hexdump -C`.

---

## 2. Top-level structure

```
+--------+-------------+-------------+--------+--------+--------+--------+--------+--------+--------+
| HEADER | METADATA    | DOCUMENT    | TOPO   | GEOM   | TAGS   | HIST   | THUMB  | EXT    | TRAILER|
+--------+-------------+-------------+--------+--------+--------+--------+--------+--------+--------+
   64 B    var          var           var      var      var      var      opt      opt      32 B
```

Each section begins with a **section header**:

| Field | Type | Bytes | Description |
|---|---|---|---|
| magic | u32 LE | 4 | Section magic (see table below) |
| version | u16 LE | 2 | Section version |
| flags | u16 LE | 2 | Compression / encoding flags |
| size | u64 LE | 8 | Payload size in bytes (excluding this 16-byte header) |
| payload | bytes | size | Section content |

### 2.1 Section magic table

| Section | Magic (ASCII) | Hex |
|---|---|---|
| HEADER | `CADK` | `43 41 44 4B` |
| METADATA | `META` | `4D 45 54 41` |
| DOCUMENT | `DOCU` | `44 4F 43 55` |
| TOPO | `TOPO` | `54 4F 50 4F` |
| GEOM | `GEOM` | `47 45 4F 4D` |
| TAGS | `TAGS` | `54 41 47 53` |
| HIST | `HIST` | `48 49 53 54` |
| THUMB | `THMB` | `54 48 4D 42` |
| EXT | `XEXT` | `58 45 58 54` |
| TRAILER | `END!` | `45 4E 44 21` |

### 2.2 Compression flags

| Bit | Name | Meaning |
|---|---|---|
| 0 | `COMPRESS_ZSTD` | Payload is zstd-compressed |
| 1 | `COMPRESS_GZIP` | Payload is gzip-compressed |
| 2 | `ENCRYPTED` | Payload encrypted (future) |
| 3-15 | reserved | Must be 0 |

---

## 3. HEADER section (64 bytes, fixed)

```
Offset  Size  Field           Type     Description
0       4     magic           u32 LE   "CADK" = 0x4B444143
4       2     version_major   u16 LE   Format major (currently 1)
6       2     version_minor   u16 LE   Format minor (currently 0)
8       4     unit_system     u32 LE   0 = millimetre, 1 = inch, 2 = metre
12      4     creator_id      u32 LE   "cadkernel" hash
16      8     creation_time   i64 LE   UNIX micros UTC
24      8     content_hash    u64 LE   BLAKE3 truncated of (DOCUMENT||TOPO||GEOM||TAGS)
32      4     section_count   u32 LE   Number of sections in file
36      4     reserved_a      u32 LE   Must be 0
40      8     creator_string  ASCII    "cadkernel"  (zero-padded)
48      16    user_uuid       16 B     Optional user identifier; zeros if anonymous
```

The header is the only section without compression and is always exactly 64 bytes.

---

## 4. METADATA section (variable)

CBOR-encoded map (RFC 8949). Required keys:

| Key | Type | Description |
|---|---|---|
| `name` | text | Project name |
| `description` | text | Free-form description |
| `author` | text | Author name |
| `created_at` | epoch_micros | Creation timestamp |
| `modified_at` | epoch_micros | Last-modified timestamp |
| `software` | text | Generating software (e.g., "cadkernel 1.0.0") |
| `cadkernel_version` | text | Exact version string |
| `tags` | [text] | Free-form tags |
| `units` | text | "mm" / "in" / "m" |

CBOR allows extension; readers must ignore unknown keys.

---

## 5. DOCUMENT section

Wraps the document model (parameters, top-level part references):

```
struct Document {
    parameter_count: u32 LE,
    parameters: [Parameter; parameter_count],
    part_ref_count: u32 LE,
    part_refs: [PartRef; part_ref_count],
    assembly_root_id: Option<u64>,
}

struct Parameter {
    name_len: u16 LE,
    name: utf8 bytes,
    expression_len: u16 LE,
    expression: utf8 bytes,
    last_value: f64 LE,
    unit_id: u32 LE,
}

struct PartRef {
    id: u64 LE,
    name_len: u16 LE,
    name: utf8 bytes,
    transform: [f64; 16],     // row-major 4x4 affine
}
```

---

## 6. TOPO section

Sequence of records. Each record begins with a 1-byte type tag.

### 6.1 SolidRecord (tag = `0x10`)

```
type_tag:        u8   = 0x10
solid_id:        u64 LE
shell_count:     u32 LE
shell_ids:       [u64 LE; shell_count]
tag_len:         u16 LE
tag_bytes:       u8 array of length tag_len
volume_hint:     f64 LE          (optional cached value, may be 0.0)
material_id:     u32 LE          (0 = none)
```

### 6.2 ShellRecord (tag = `0x11`)

```
type_tag:        u8   = 0x11
shell_id:        u64 LE
parent_solid_id: u64 LE
face_count:      u32 LE
face_ids:        [u64 LE; face_count]
orientation:     u8           (0 = outward, 1 = inward / void)
tag_len:         u16 LE
tag_bytes:       u8 array
```

### 6.3 FaceRecord (tag = `0x12`)

```
type_tag:        u8   = 0x12
face_id:         u64 LE
parent_shell_id: u64 LE
surface_id:      u64 LE          (reference into GEOM section)
outer_loop_id:   u64 LE
inner_loop_count: u32 LE
inner_loop_ids:  [u64 LE; inner_loop_count]
orientation:     u8              (0 = aligned with surface normal, 1 = reversed)
tag_len:         u16 LE
tag_bytes:       u8 array
material_id:     u32 LE
```

### 6.4 LoopRecord (tag = `0x13`)

```
type_tag:        u8   = 0x13
loop_id:         u64 LE
parent_face_id:  u64 LE
half_edge_count: u32 LE
half_edge_ids:   [u64 LE; half_edge_count]
orientation:     u8           (0 = ccw, 1 = cw)
```

### 6.5 HalfEdgeRecord (tag = `0x14`)

```
type_tag:        u8   = 0x14
half_edge_id:    u64 LE
edge_id:         u64 LE
twin_id:         u64 LE          (0xFFFFFFFFFFFFFFFF if no twin — non-manifold)
next_id:         u64 LE
prev_id:         u64 LE
loop_id:         u64 LE
direction:       u8              (0 = same as edge curve, 1 = reversed)
```

### 6.6 EdgeRecord (tag = `0x15`)

```
type_tag:        u8   = 0x15
edge_id:         u64 LE
curve_id:        u64 LE          (reference into GEOM section)
start_vertex_id: u64 LE
end_vertex_id:   u64 LE
parameter_start: f64 LE          (curve param at start)
parameter_end:   f64 LE          (curve param at end)
tag_len:         u16 LE
tag_bytes:       u8 array
```

### 6.7 VertexRecord (tag = `0x16`)

```
type_tag:        u8   = 0x16
vertex_id:       u64 LE
position_x:      f64 LE
position_y:      f64 LE
position_z:      f64 LE
tag_len:         u16 LE
tag_bytes:       u8 array
```

### 6.8 Encoding order

Records are written in dependency order: Vertex → Edge → HalfEdge → Loop → Face → Shell → Solid. Readers may use a two-pass approach if they want random access.

---

## 7. GEOM section

Sequence of curve and surface records. Common header per geometry record:

```
type_tag:    u8                (0x20-0x2F = curves, 0x30-0x3F = surfaces)
geom_id:     u64 LE
payload:     varies by type
```

### 7.1 CurveRecord type tags

| Tag | Type | Payload |
|---|---|---|
| `0x20` | Line | start: f64×3, end: f64×3 |
| `0x21` | Circle | center: f64×3, normal: f64×3, radius: f64, start_angle: f64, end_angle: f64 |
| `0x22` | Ellipse | center, normal, major_axis: f64×3, minor_radius: f64, ranges |
| `0x23` | Parabola | apex, axis, focal_distance, parameter range |
| `0x24` | Hyperbola | center, axes, semi-major, semi-minor, ranges |
| `0x25` | NurbsCurve | NurbsCurvePayload (see below) |
| `0x26` | CompositeCurve | segment_count: u32, then segment refs (curve_id, t_start, t_end) |

### 7.2 NurbsCurvePayload

```
degree:           u32 LE
control_count:    u32 LE
knot_count:       u32 LE         (= control_count + degree + 1)
control_points:   [f64×3; control_count]
weights:          [f64; control_count]      (1.0 if non-rational)
knots:            [f64; knot_count]
periodic:         u8             (0 / 1)
rational:         u8             (0 / 1; controls whether weights are written)
```

### 7.3 SurfaceRecord type tags

| Tag | Type | Payload |
|---|---|---|
| `0x30` | Plane | origin: f64×3, normal: f64×3, u_dir: f64×3 |
| `0x31` | Cylinder | origin, axis, radius, u/v ranges |
| `0x32` | Sphere | center, radius, u/v ranges |
| `0x33` | Cone | apex, axis, half_angle, u/v ranges |
| `0x34` | Torus | center, axis, major_radius, minor_radius, u/v ranges |
| `0x35` | Plane (revolve) | meridian curve_id + axis |
| `0x36` | Ruled | curve1_id, curve2_id |
| `0x37` | NurbsSurface | NurbsSurfacePayload (see below) |
| `0x38` | TrimmedSurface | base_id, trimming_loop_count, trimming_loops |

### 7.4 NurbsSurfacePayload

```
degree_u:         u32 LE
degree_v:         u32 LE
control_u:        u32 LE
control_v:        u32 LE
knot_u_count:     u32 LE
knot_v_count:     u32 LE
control_points:   [f64×3; control_u * control_v]   (row-major U-major)
weights:          [f64; control_u * control_v]
knots_u:          [f64; knot_u_count]
knots_v:          [f64; knot_v_count]
periodic_u:       u8
periodic_v:       u8
rational:         u8
```

---

## 8. TAGS section

Persistent-naming tags map. Each entry:

```
entity_id:     u64 LE          (vertex / edge / face / etc.)
entity_kind:   u8              (0=Vertex, 1=Edge, 2=Face, 3=Shell, 4=Solid)
tag:           Tag struct (8 bytes; see §10)
parent_count:  u8              (number of provenance pointers)
parents:       [u64; parent_count]
operation_id:  u64 LE
local_index:   u32 LE
```

---

## 9. HIST section

Operation history (recompute graph):

```
operation_count: u32 LE
operations:      [Operation; operation_count]

struct Operation {
    op_id:           u64 LE
    op_kind:         u8           (see operation kind table)
    op_name_len:     u16 LE
    op_name:         utf8
    input_count:     u32 LE
    inputs:          [u64; input_count]    (handles to prior operations or imports)
    output_count:    u32 LE
    outputs:         [u64; output_count]   (handles into TOPO)
    parameter_count: u32 LE
    parameters:      [(name_len: u16, name: utf8, value_kind: u8, value: bytes); parameter_count]
    cache_key:       [u8; 32]    (BLAKE3 hash)
    timestamp:       i64 LE      (unix micros when last computed)
}
```

### 9.1 Operation kinds

| Code | Kind |
|---|---|
| 0 | Import |
| 1 | Primitive |
| 2 | Sketch |
| 3 | Extrude |
| 4 | Revolve |
| 5 | Sweep |
| 6 | Loft |
| 7 | Boolean |
| 8 | Fillet |
| 9 | Chamfer |
| 10 | Pattern |
| 11 | Mirror |
| 12 | Transform |
| 13 | Shell |
| 14 | Draft |
| 15 | Thicken |
| 16-255 | reserved |

---

## 10. Tag struct (8 bytes)

```
struct Tag {
    op_id:        u32 LE     (which operation produced this entity)
    entity_kind:  u8         (Vertex / Edge / Face / etc.)
    local_index:  u16 LE     (index within operation's outputs)
    flags:        u8         (bit 0 = generated, bit 1 = derived, bits 2-7 reserved)
}
```

Tags are deterministic: same operation on same inputs produces identical tags. This is what enables persistent naming under feature edits.

---

## 11. THUMB section (optional)

PNG-encoded thumbnail of the top-level part:

```
type_tag:    u8 = 0x40
width:       u32 LE
height:      u32 LE
png_size:    u32 LE
png_bytes:   [u8; png_size]
```

---

## 12. EXT section (optional)

Reserved for future / vendor extensions:

```
ext_id:      [u8; 8]      (8-char ASCII identifier)
ext_size:    u32 LE
ext_bytes:   [u8; ext_size]
```

Readers must skip unknown EXT records.

---

## 13. TRAILER section (32 bytes, fixed)

```
Offset  Size  Field           Type     Description
0       4     magic           u32 LE   "END!" = 0x21444E45
4       4     section_count   u32 LE   Confirmation count (must match HEADER)
8       8     content_hash    u64 LE   BLAKE3 truncated of all preceding bytes
16      8     file_size       u64 LE   Total file size in bytes (including trailer)
24      8     reserved        u64 LE   Must be 0
```

---

## 14. Backwards/forwards compatibility

- **Major-minor versioning.** v1.0 readers must accept any v1.x file. v2.x readers must reject v1.x with a clear migration message.
- **Unknown sections.** Readers must skip unrecognised section magics (size known from section header).
- **Unknown CBOR keys** in METADATA must be ignored.
- **Unknown EXT records** must be skipped.
- **Field appended to fixed records.** Forbidden in v1.x. To extend a record, add a new section or use EXT.
- **Numeric ranges.** All sizes are u32 or u64; no implicit limits beyond architecture.

---

## 15. Migration framework

When a future v2.x is introduced:

1. v1 reader reads file, populates v1 in-memory model.
2. **Migration shim** transforms v1 model → v2 model (additive only, lossless).
3. v2 writer writes v2 file.

Migration shim lives in `crates/io/src/cadk/migrate/` and has full unit-test coverage of every v1 field.

### 15.1 v0 → v1 example

In v0 (pre-spec), `FaceRecord` did not have `material_id`. Migration:

```rust
fn migrate_face_v0_to_v1(face_v0: FaceRecordV0) -> FaceRecordV1 {
    FaceRecordV1 {
        face_id: face_v0.face_id,
        parent_shell_id: face_v0.parent_shell_id,
        surface_id: face_v0.surface_id,
        outer_loop_id: face_v0.outer_loop_id,
        inner_loop_count: face_v0.inner_loop_count,
        inner_loop_ids: face_v0.inner_loop_ids,
        orientation: face_v0.orientation,
        tag_len: face_v0.tag_len,
        tag_bytes: face_v0.tag_bytes,
        material_id: 0,    // default added in v1
    }
}
```

---

## 16. Validation algorithm

```
fn validate(file: &[u8]) -> KernelResult<ValidationReport> {
    1. Verify HEADER magic and bounds.
    2. Verify TRAILER magic and bounds.
    3. Recompute content_hash; compare to TRAILER.
    4. Iterate sections; verify section magics, sizes sum to file_size.
    5. Decompress sections; verify each parses.
    6. Cross-reference checks:
       - Every face_id in ShellRecord exists in FaceRecord set.
       - Every surface_id in FaceRecord exists in GEOM.
       - Every twin_id is reciprocal (a.twin = b → b.twin = a) or 0xFFFF...FFFF.
       - Every loop closes (next chain returns to start).
    7. Topology validity:
       - Manifold check.
       - Euler-Poincaré.
    8. Geometry validity:
       - Knot vectors monotone non-decreasing.
       - Weights positive.
    Return report.
}
```

---

## 17. Endianness and alignment

- All multi-byte integers and floats are **little-endian** on disk.
- Records are **byte-packed**, no padding. Readers must use unaligned reads (`read_unaligned`) on platforms requiring alignment.
- f64 is IEEE 754 binary64.

---

## 18. References

1. ISO 10303-21: STEP file format (for comparison).
2. RFC 8949: CBOR (Concise Binary Object Representation).
3. zstd format spec: <https://datatracker.ietf.org/doc/html/rfc8878>
4. BLAKE3 spec: <https://github.com/BLAKE3-team/BLAKE3-specs>
5. OCCT BinXCAFFormat (binary XDE format) for comparison.
