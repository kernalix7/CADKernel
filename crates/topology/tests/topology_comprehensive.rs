//! Comprehensive topology crate tests — V32 coverage expansion.
//!
//! Covers: Tag/Naming, ModelHistory, geometry binding, inner loops,
//! wire ops, tagged entities, properties, Handle, EntityStore extras.

use std::sync::Arc;

use cadkernel_math::Point3;
use cadkernel_topology::*;

// =========================================================================
// Helpers
// =========================================================================

/// Build a triangle face with 3 vertices, 3 edges, 1 loop, 1 face.
fn make_triangle() -> (BRepModel, Handle<FaceData>) {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let lh = m.make_loop(&[he01, he12, he20]).unwrap();
    let fh = m.make_face(lh);
    (m, fh)
}

/// Build a quad face.
fn make_quad() -> (BRepModel, Handle<FaceData>) {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(1.0, 1.0, 0.0));
    let v3 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he23, _) = m.add_edge(v2, v3);
    let (_, he30, _) = m.add_edge(v3, v0);
    let lh = m.make_loop(&[he01, he12, he23, he30]).unwrap();
    let fh = m.make_face(lh);
    (m, fh)
}

// =========================================================================
// 1. Tag / Naming system
// =========================================================================

#[test]
fn tag_modified_appends_segment() {
    let t = Tag::generated(EntityKind::Edge, OperationId(1), 0);
    let m = t.modified(OperationId(2));
    assert_eq!(m.segments.len(), 2);
    assert_eq!(m.kind, EntityKind::Edge);
}

#[test]
fn tag_merged_appends_segment() {
    let t = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let m = t.merged(OperationId(3));
    assert_eq!(m.segments.len(), 2);
    assert_eq!(m.kind, EntityKind::Face);
}

#[test]
fn tag_chained_operations() {
    let t = Tag::generated(EntityKind::Vertex, OperationId(1), 5);
    let t2 = t.modified(OperationId(2));
    let t3 = t2.split(OperationId(3), 0);
    let t4 = t3.merged(OperationId(4));
    assert_eq!(t4.segments.len(), 4);
    assert_eq!(t4.kind, EntityKind::Vertex);
}

#[test]
fn tag_equality_by_content() {
    let a = Tag::generated(EntityKind::Face, OperationId(1), 0).modified(OperationId(2));
    let b = Tag::generated(EntityKind::Face, OperationId(1), 0).modified(OperationId(2));
    assert_eq!(a, b);
}

#[test]
fn tag_inequality_different_op() {
    let a = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let b = Tag::generated(EntityKind::Face, OperationId(2), 0);
    assert_ne!(a, b);
}

#[test]
fn tag_inequality_different_kind() {
    let a = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let b = Tag::generated(EntityKind::Edge, OperationId(1), 0);
    assert_ne!(a, b);
}

#[test]
fn tag_display_and_debug() {
    let t = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let dbg = format!("{t:?}");
    assert!(dbg.contains("Face"));
    assert!(dbg.contains("Op1"));
    let display = format!("{t}");
    assert!(!display.is_empty());
}

#[test]
fn tag_hash_consistency() {
    use std::collections::HashSet;
    let a = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let b = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let mut set = HashSet::new();
    set.insert(a);
    assert!(set.contains(&b));
}

// -- NameMap --

#[test]
fn name_map_typed_getters() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("test");

    let vh = m.add_vertex(Point3::ORIGIN);
    let v_tag = Tag::generated(EntityKind::Vertex, op, 0);
    m.name_map.insert(v_tag.clone(), EntityRef::Vertex(vh));
    assert_eq!(m.name_map.get_vertex(&v_tag), Some(vh));
    assert!(m.name_map.get_edge(&v_tag).is_none());

    let vh2 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (eh, _, _) = m.add_edge(vh, vh2);
    let e_tag = Tag::generated(EntityKind::Edge, op, 1);
    m.name_map.insert(e_tag.clone(), EntityRef::Edge(eh));
    assert_eq!(m.name_map.get_edge(&e_tag), Some(eh));
    assert!(m.name_map.get_face(&e_tag).is_none());
}

#[test]
fn name_map_remove() {
    let mut map = NameMap::new();
    let tag = Tag::generated(EntityKind::Vertex, OperationId(1), 0);
    let h = Handle::<VertexData>::from_raw_parts(0, 0);
    map.insert(tag.clone(), EntityRef::Vertex(h));
    assert!(map.get(&tag).is_some());

    let removed = map.remove(&tag);
    assert!(removed.is_some());
    assert!(map.get(&tag).is_none());
}

#[test]
fn name_map_len_and_is_empty() {
    let mut map = NameMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);

    let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let h = Handle::<FaceData>::from_raw_parts(0, 0);
    map.insert(tag, EntityRef::Face(h));
    assert!(!map.is_empty());
    assert_eq!(map.len(), 1);
}

#[test]
fn name_map_iter() {
    let mut map = NameMap::new();
    let t1 = Tag::generated(EntityKind::Vertex, OperationId(1), 0);
    let t2 = Tag::generated(EntityKind::Edge, OperationId(1), 1);
    let h1 = Handle::<VertexData>::from_raw_parts(0, 0);
    let h2 = Handle::<EdgeData>::from_raw_parts(1, 0);
    map.insert(t1, EntityRef::Vertex(h1));
    map.insert(t2, EntityRef::Edge(h2));
    assert_eq!(map.iter().count(), 2);
}

#[test]
fn name_map_serialization_roundtrip() {
    let mut map = NameMap::new();
    let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let h = Handle::<FaceData>::from_raw_parts(5, 2);
    map.insert(tag.clone(), EntityRef::Face(h));

    let json = serde_json::to_string(&map).expect("serialize NameMap");
    let restored: NameMap = serde_json::from_str(&json).expect("deserialize NameMap");
    assert_eq!(restored.len(), 1);
    assert_eq!(restored.get_face(&tag), Some(h));
}

#[test]
fn entity_ref_kind() {
    let vr = EntityRef::Vertex(Handle::from_raw_parts(0, 0));
    assert_eq!(vr.kind(), EntityKind::Vertex);
    let er = EntityRef::Edge(Handle::from_raw_parts(0, 0));
    assert_eq!(er.kind(), EntityKind::Edge);
    let fr = EntityRef::Face(Handle::from_raw_parts(0, 0));
    assert_eq!(fr.kind(), EntityKind::Face);
    let sr = EntityRef::Shell(Handle::from_raw_parts(0, 0));
    assert_eq!(sr.kind(), EntityKind::Shell);
    let so = EntityRef::Solid(Handle::from_raw_parts(0, 0));
    assert_eq!(so.kind(), EntityKind::Solid);
    let wr = EntityRef::Wire(Handle::from_raw_parts(0, 0));
    assert_eq!(wr.kind(), EntityKind::Wire);
    let hr = EntityRef::HalfEdge(Handle::from_raw_parts(0, 0));
    assert_eq!(hr.kind(), EntityKind::HalfEdge);
    let lr = EntityRef::Loop(Handle::from_raw_parts(0, 0));
    assert_eq!(lr.kind(), EntityKind::Loop);
}

// -- ShapeHistory --

#[test]
fn shape_history_current_op_id() {
    let mut h = ShapeHistory::new();
    assert!(h.current_op_id().is_none());
    let op = h.next_operation("first");
    assert_eq!(h.current_op_id(), Some(op));
}

#[test]
fn shape_history_get_record() {
    let mut h = ShapeHistory::new();
    let op1 = h.next_operation("create box");
    let op2 = h.next_operation("fillet");
    assert_eq!(h.get_record(op1).unwrap().label, "create box");
    assert_eq!(h.get_record(op2).unwrap().label, "fillet");
    assert!(h.get_record(OperationId(99)).is_none());
}

#[test]
fn shape_history_evolution_variants() {
    use cadkernel_topology::naming::history::Evolution;

    let mut h = ShapeHistory::new();
    let op = h.next_operation("complex op");

    let tag1 = Tag::generated(EntityKind::Face, op, 0);
    h.record(Evolution::Generated {
        tag: tag1.clone(),
        kind: EntityKind::Face,
    });

    let tag2 = tag1.modified(op);
    h.record(Evolution::Modified {
        old_tag: tag1.clone(),
        new_tag: tag2,
    });

    h.record(Evolution::Split {
        parent_tag: tag1.clone(),
        child_tags: vec![tag1.split(op, 0), tag1.split(op, 1)],
    });

    h.record(Evolution::Deleted { tag: tag1 });

    let rec = h.get_record(op).unwrap();
    assert_eq!(rec.evolutions.len(), 4);
}

// =========================================================================
// 2. ModelHistory undo/redo
// =========================================================================

#[test]
fn model_history_basic_undo_redo() {
    let m0 = BRepModel::new();
    let mut hist = ModelHistory::new(m0, 10);
    assert!(!hist.can_undo());
    assert!(!hist.can_redo());
    assert_eq!(hist.undo_count(), 0);
    assert_eq!(hist.redo_count(), 0);

    // Add a vertex → record state
    let mut m1 = hist.current_model().clone();
    m1.add_vertex(Point3::new(1.0, 0.0, 0.0));
    hist.record(m1, "add vertex");

    assert!(hist.can_undo());
    assert!(!hist.can_redo());
    assert_eq!(hist.undo_count(), 1);
    assert_eq!(hist.current_model().vertices.len(), 1);

    // Undo
    let undone = hist.undo().unwrap();
    assert_eq!(undone.vertices.len(), 0);
    assert!(!hist.can_undo());
    assert!(hist.can_redo());

    // Redo
    let redone = hist.redo().unwrap();
    assert_eq!(redone.vertices.len(), 1);
    assert!(hist.can_undo());
    assert!(!hist.can_redo());
}

#[test]
fn model_history_undo_empty_returns_none() {
    let m = BRepModel::new();
    let mut hist = ModelHistory::new(m, 5);
    assert!(hist.undo().is_none());
}

#[test]
fn model_history_redo_empty_returns_none() {
    let m = BRepModel::new();
    let mut hist = ModelHistory::new(m, 5);
    assert!(hist.redo().is_none());
}

#[test]
fn model_history_record_clears_redo() {
    let m0 = BRepModel::new();
    let mut hist = ModelHistory::new(m0, 10);

    let mut m1 = hist.current_model().clone();
    m1.add_vertex(Point3::ORIGIN);
    hist.record(m1, "step 1");

    let mut m2 = hist.current_model().clone();
    m2.add_vertex(Point3::new(1.0, 0.0, 0.0));
    hist.record(m2, "step 2");

    // Undo both → 2 redo available
    hist.undo();
    hist.undo();
    assert_eq!(hist.redo_count(), 2);

    // Record a new state → redo cleared
    let mut m3 = hist.current_model().clone();
    m3.add_vertex(Point3::new(5.0, 5.0, 5.0));
    hist.record(m3, "diverge");
    assert_eq!(hist.redo_count(), 0);
}

#[test]
fn model_history_max_cap() {
    let m0 = BRepModel::new();
    let mut hist = ModelHistory::new(m0, 3);

    for i in 0..5 {
        let mut m = hist.current_model().clone();
        m.add_vertex(Point3::new(i as f64, 0.0, 0.0));
        hist.record(m, format!("step {i}"));
    }

    // Only 3 undo steps kept despite 5 records
    assert_eq!(hist.undo_count(), 3);
}

#[test]
fn model_history_descriptions() {
    let m0 = BRepModel::new();
    let mut hist = ModelHistory::new(m0, 10);

    let mut m1 = hist.current_model().clone();
    m1.add_vertex(Point3::ORIGIN);
    hist.record(m1, "create vertex");

    let mut m2 = hist.current_model().clone();
    m2.add_vertex(Point3::new(1.0, 0.0, 0.0));
    hist.record(m2, "add edge");

    let descs = hist.history_descriptions();
    assert_eq!(descs, vec!["create vertex", "add edge"]);
}

#[test]
fn model_history_multi_undo_redo_cycle() {
    let m0 = BRepModel::new();
    let mut hist = ModelHistory::new(m0, 10);

    for i in 1..=4 {
        let mut m = hist.current_model().clone();
        for _ in 0..i {
            m.add_vertex(Point3::ORIGIN);
        }
        hist.record(m, format!("step {i}"));
    }

    // Current should have 1+2+3+4 = 10 vertices
    assert_eq!(hist.current_model().vertices.len(), 10);

    // Undo 2 → back to step 2 state (1+2 = 3 vertices)
    hist.undo();
    hist.undo();
    assert_eq!(hist.current_model().vertices.len(), 3);
    assert_eq!(hist.undo_count(), 2);
    assert_eq!(hist.redo_count(), 2);

    // Redo 1 → step 3 state (1+2+3 = 6 vertices)
    hist.redo();
    assert_eq!(hist.current_model().vertices.len(), 6);
}

// =========================================================================
// 3. Geometry binding
// =========================================================================

#[test]
fn bind_edge_curve_and_query() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (eh, _, _) = m.add_edge(v0, v1);

    assert!(!m.edge_has_curve(eh));

    let line = cadkernel_geometry::curve::line::Line::new(
        Point3::new(0.0, 0.0, 0.0),
        cadkernel_math::Vec3::X,
    );
    m.bind_edge_curve(eh, Arc::new(line), (0.0, 1.0));
    assert!(m.edge_has_curve(eh));

    let ed = m.edges.get(eh).unwrap();
    assert_eq!(ed.curve_domain, Some((0.0, 1.0)));
}

#[test]
fn bind_face_surface_and_query() {
    let (mut m, fh) = make_triangle();
    assert!(!m.face_has_surface(fh));

    let plane = cadkernel_geometry::surface::plane::Plane::new(
        Point3::ORIGIN,
        cadkernel_math::Vec3::X,
        cadkernel_math::Vec3::Y,
    )
    .unwrap();
    m.bind_face_surface(fh, Arc::new(plane), Orientation::Reversed);
    assert!(m.face_has_surface(fh));

    let fd = m.faces.get(fh).unwrap();
    assert_eq!(fd.orientation, Orientation::Reversed);
}

#[test]
fn bind_face_trim() {
    let (mut m, fh) = make_triangle();

    let outer = cadkernel_geometry::ParametricWire2D {
        segments: vec![],
        closed: true,
    };
    let inner = cadkernel_geometry::ParametricWire2D {
        segments: vec![],
        closed: true,
    };
    m.bind_face_trim(fh, outer, vec![inner]);

    let fd = m.faces.get(fh).unwrap();
    assert!(fd.outer_trim.is_some());
    assert_eq!(fd.inner_trims.len(), 1);
}

#[test]
fn bind_edge_pcurve_left_right() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (eh, _, _) = m.add_edge(v0, v1);

    let line2d = cadkernel_geometry::curve::curve2d::Line2D::new(
        cadkernel_math::Point2::new(0.0, 0.0),
        cadkernel_math::Point2::new(1.0, 0.0),
    );

    m.bind_edge_pcurve(eh, Arc::new(line2d), true);
    let ed = m.edges.get(eh).unwrap();
    assert!(ed.pcurve_left.is_some());
    assert!(ed.pcurve_right.is_none());

    let line2d_r = cadkernel_geometry::curve::curve2d::Line2D::new(
        cadkernel_math::Point2::new(0.0, 0.0),
        cadkernel_math::Point2::new(1.0, 0.0),
    );
    m.bind_edge_pcurve(eh, Arc::new(line2d_r), false);
    let ed = m.edges.get(eh).unwrap();
    assert!(ed.pcurve_right.is_some());
}

#[test]
fn edge_has_curve_false_for_dead_handle() {
    let m = BRepModel::new();
    let stale = Handle::<EdgeData>::from_raw_parts(999, 0);
    assert!(!m.edge_has_curve(stale));
}

#[test]
fn face_has_surface_false_for_dead_handle() {
    let m = BRepModel::new();
    let stale = Handle::<FaceData>::from_raw_parts(999, 0);
    assert!(!m.face_has_surface(stale));
}

// =========================================================================
// 4. Inner loops, wire ops, tagged entities, properties
// =========================================================================

// -- Inner loops --

#[test]
fn add_inner_loop_to_face() {
    let (mut m, fh) = make_quad();

    // Add a triangular hole inside the quad
    let v0 = m.add_vertex(Point3::new(0.3, 0.3, 0.0));
    let v1 = m.add_vertex(Point3::new(0.7, 0.3, 0.0));
    let v2 = m.add_vertex(Point3::new(0.5, 0.7, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let inner_loop = m.make_loop(&[he01, he12, he20]).unwrap();

    m.add_inner_loop(fh, inner_loop);

    let fd = m.faces.get(fh).unwrap();
    assert_eq!(fd.inner_loops.len(), 1);
    assert_eq!(fd.inner_loops[0], inner_loop);

    // Inner loop's face reference points back
    let ld = m.loops.get(inner_loop).unwrap();
    assert_eq!(ld.face, Some(fh));
}

#[test]
fn add_multiple_inner_loops() {
    let (mut m, fh) = make_quad();

    for i in 0..3 {
        let offset = i as f64 * 0.25;
        let va = m.add_vertex(Point3::new(0.1 + offset, 0.1, 0.0));
        let vb = m.add_vertex(Point3::new(0.2 + offset, 0.1, 0.0));
        let vc = m.add_vertex(Point3::new(0.15 + offset, 0.2, 0.0));
        let (_, h1, _) = m.add_edge(va, vb);
        let (_, h2, _) = m.add_edge(vb, vc);
        let (_, h3, _) = m.add_edge(vc, va);
        let il = m.make_loop(&[h1, h2, h3]).unwrap();
        m.add_inner_loop(fh, il);
    }

    let fd = m.faces.get(fh).unwrap();
    assert_eq!(fd.inner_loops.len(), 3);
}

// -- Wire --

#[test]
fn make_wire_open() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(2.0, 0.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);

    let wh = m.make_wire(vec![he01, he12], false);
    let wd = m.wires.get(wh).unwrap();
    assert!(!wd.is_closed);
    assert_eq!(wd.len(), 2);
    assert!(!wd.is_empty());
}

#[test]
fn make_wire_closed() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);

    let wh = m.make_wire(vec![he01, he12, he20], true);
    let wd = m.wires.get(wh).unwrap();
    assert!(wd.is_closed);
    assert_eq!(wd.len(), 3);
}

#[test]
fn make_wire_tagged() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("wire_test");
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (_, he, _) = m.add_edge(v0, v1);

    let tag = Tag::generated(EntityKind::Wire, op, 0);
    let wh = m.make_wire_tagged(vec![he], false, tag.clone());

    assert_eq!(m.find_wire_by_tag(&tag), Some(wh));
    let wd = m.wires.get(wh).unwrap();
    assert_eq!(wd.tag.as_ref(), Some(&tag));
}

#[test]
fn wire_data_empty() {
    let wd = WireData::new(vec![], false);
    assert!(wd.is_empty());
    assert_eq!(wd.len(), 0);
}

// -- Tagged entity constructors + lookups --

#[test]
fn add_vertex_tagged_and_lookup() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("test");
    let tag = Tag::generated(EntityKind::Vertex, op, 0);
    let vh = m.add_vertex_tagged(Point3::new(3.0, 4.0, 5.0), tag.clone());

    assert_eq!(m.find_vertex_by_tag(&tag), Some(vh));
    let vd = m.vertices.get(vh).unwrap();
    assert_eq!(vd.tag.as_ref(), Some(&tag));
    assert!(vd.point.approx_eq(Point3::new(3.0, 4.0, 5.0)));
}

#[test]
fn add_edge_tagged_and_lookup() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("test");
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let tag = Tag::generated(EntityKind::Edge, op, 0);
    let (eh, _, _) = m.add_edge_tagged(v0, v1, tag.clone());

    assert_eq!(m.find_edge_by_tag(&tag), Some(eh));
    let ed = m.edges.get(eh).unwrap();
    assert_eq!(ed.tag.as_ref(), Some(&tag));
}

#[test]
fn make_shell_tagged_and_lookup() {
    let (mut m, fh) = make_triangle();
    let op = m.history.next_operation("test");
    let tag = Tag::generated(EntityKind::Shell, op, 0);
    let sh = m.make_shell_tagged(&[fh], tag.clone());

    assert_eq!(m.find_shell_by_tag(&tag), Some(sh));
    let sd = m.shells.get(sh).unwrap();
    assert_eq!(sd.tag.as_ref(), Some(&tag));
}

#[test]
fn make_solid_tagged_and_lookup() {
    let (mut m, fh) = make_triangle();
    let op = m.history.next_operation("test");
    let shell = m.make_shell(&[fh]);
    let tag = Tag::generated(EntityKind::Solid, op, 0);
    let solid = m.make_solid_tagged(&[shell], tag.clone());

    assert_eq!(m.find_solid_by_tag(&tag), Some(solid));
    let sd = m.solids.get(solid).unwrap();
    assert_eq!(sd.tag.as_ref(), Some(&tag));
}

#[test]
fn find_tag_returns_none_for_unknown() {
    let m = BRepModel::new();
    let tag = Tag::generated(EntityKind::Face, OperationId(99), 0);
    assert!(m.find_face_by_tag(&tag).is_none());
    assert!(m.find_vertex_by_tag(&tag).is_none());
    assert!(m.find_edge_by_tag(&tag).is_none());
    assert!(m.find_wire_by_tag(&tag).is_none());
    assert!(m.find_shell_by_tag(&tag).is_none());
    assert!(m.find_solid_by_tag(&tag).is_none());
}

// -- faces_around_vertex --

#[test]
fn faces_around_vertex_single_face() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let lh = m.make_loop(&[he01, he12, he20]).unwrap();
    let fh = m.make_face(lh);

    let faces = m.faces_around_vertex(v0).unwrap();
    assert_eq!(faces.len(), 1);
    assert_eq!(faces[0], fh);
}

#[test]
fn faces_around_vertex_invalid_handle() {
    let m = BRepModel::new();
    let stale = Handle::<VertexData>::from_raw_parts(999, 0);
    assert!(m.faces_around_vertex(stale).is_err());
}

// -- Properties --

#[test]
fn property_store_material() {
    let mut store = PropertyStore::new();
    assert!(store.get_material(0).is_none());

    store.set_material(0, Material::steel());
    let mat = store.get_material(0).unwrap();
    assert_eq!(mat.name, "Steel");
    assert!((mat.density - 7850.0).abs() < 1e-6);
}

#[test]
fn property_store_metadata() {
    let mut store = PropertyStore::new();
    store.set_metadata(1, "name", PropertyValue::String("bolt".into()));
    store.set_metadata(1, "count", PropertyValue::Int(42));
    store.set_metadata(1, "active", PropertyValue::Bool(true));
    store.set_metadata(1, "weight", PropertyValue::Float(1.5));

    assert_eq!(
        store.get_metadata(1, "name"),
        Some(&PropertyValue::String("bolt".into()))
    );
    assert_eq!(
        store.get_metadata(1, "count"),
        Some(&PropertyValue::Int(42))
    );
    assert_eq!(
        store.get_metadata(1, "active"),
        Some(&PropertyValue::Bool(true))
    );
    assert!(store.get_metadata(2, "name").is_none());
}

#[test]
fn color_constructors() {
    let c = Color::rgb(0.5, 0.6, 0.7);
    assert!((c.r - 0.5).abs() < 1e-10);
    assert!((c.a - 1.0).abs() < 1e-10);

    let c2 = Color::rgba(0.1, 0.2, 0.3, 0.4);
    assert!((c2.a - 0.4).abs() < 1e-10);
}

#[test]
fn color_constants() {
    assert!((Color::RED.r - 1.0).abs() < 1e-10);
    assert!((Color::GREEN.g - 1.0).abs() < 1e-10);
    assert!((Color::BLUE.b - 1.0).abs() < 1e-10);
    assert!((Color::WHITE.r - 1.0).abs() < 1e-10);
    assert!((Color::BLACK.r).abs() < 1e-10);
    assert!((Color::GRAY.r - 0.5).abs() < 1e-10);
}

#[test]
fn material_presets() {
    let steel = Material::steel();
    assert!((steel.metallic - 1.0).abs() < 1e-10);

    let alu = Material::aluminum();
    assert!((alu.density - 2700.0).abs() < 1e-6);

    let abs = Material::plastic_abs();
    assert!((abs.metallic).abs() < 1e-10);

    let wood = Material::wood();
    assert!((wood.density - 600.0).abs() < 1e-6);
}

#[test]
fn material_builders() {
    let m = Material::new("Custom")
        .with_density(1234.0)
        .with_color(Color::RED)
        .with_metallic(0.8)
        .with_roughness(0.3);

    assert_eq!(m.name, "Custom");
    assert!((m.density - 1234.0).abs() < 1e-6);
    assert!((m.color.r - 1.0).abs() < 1e-10);
    assert!((m.metallic - 0.8).abs() < 1e-10);
    assert!((m.roughness - 0.3).abs() < 1e-10);
}

// =========================================================================
// 5. Handle & EntityStore extras
// =========================================================================

#[test]
fn handle_index_and_generation() {
    let h = Handle::<VertexData>::from_raw_parts(7, 3);
    assert_eq!(h.index(), 7);
    assert_eq!(h.generation(), 3);
}

#[test]
fn handle_from_raw_parts_roundtrip() {
    let mut store = EntityStore::new();
    let original = store.insert(42);
    let rebuilt = Handle::<i32>::from_raw_parts(original.index(), original.generation());
    assert_eq!(store.get(rebuilt), Some(&42));
}

#[test]
fn entity_store_is_alive_and_is_empty() {
    let mut store = EntityStore::new();
    assert!(store.is_empty());
    let h = store.insert(10);
    assert!(!store.is_empty());
    assert!(store.is_alive(h));

    store.remove(h);
    assert!(!store.is_alive(h));
    assert!(store.is_empty());
}

#[test]
fn entity_store_get_mut() {
    let mut store = EntityStore::new();
    let h = store.insert(10);
    *store.get_mut(h).unwrap() = 20;
    assert_eq!(store.get(h), Some(&20));
}

#[test]
fn entity_store_iter_mut() {
    let mut store = EntityStore::new();
    store.insert(1);
    store.insert(2);
    store.insert(3);

    for (_, v) in store.iter_mut() {
        *v *= 10;
    }

    let values: Vec<_> = store.iter().map(|(_, v)| *v).collect();
    assert_eq!(values, vec![10, 20, 30]);
}

#[test]
fn entity_store_slot_reuse() {
    let mut store = EntityStore::new();
    let h1 = store.insert(100);
    let idx1 = h1.index();
    store.remove(h1);

    let h2 = store.insert(200);
    // Reused slot → same index, different generation
    assert_eq!(h2.index(), idx1);
    assert_ne!(h2.generation(), h1.generation());
    assert_eq!(store.get(h2), Some(&200));
    assert!(store.get(h1).is_none());
}

// =========================================================================
// 6. Validation edge cases
// =========================================================================

#[test]
fn make_loop_rejects_single_half_edge() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (_, he, _) = m.add_edge(v0, v1);
    assert!(m.make_loop(&[he]).is_err());
}

#[test]
fn validate_orientation_consistency() {
    let (m, _) = make_triangle();
    // Triangle model should have consistent orientation
    assert!(m.validate().is_ok());
}

#[test]
fn brep_model_default() {
    let m = BRepModel::default();
    assert!(m.vertices.is_empty());
    assert!(m.edges.is_empty());
    assert!(m.faces.is_empty());
}

#[test]
fn brep_model_serialization_roundtrip() {
    let (m, _) = make_triangle();
    let json = serde_json::to_string(&m).expect("serialize BRepModel");
    let restored: BRepModel = serde_json::from_str(&json).expect("deserialize BRepModel");
    assert_eq!(restored.vertices.len(), 3);
    assert_eq!(restored.edges.len(), 3);
    assert_eq!(restored.faces.len(), 1);
}

#[test]
fn edges_of_face_invalid_handle() {
    let m = BRepModel::new();
    let stale = Handle::<FaceData>::from_raw_parts(999, 0);
    assert!(m.edges_of_face(stale).is_err());
}

#[test]
fn vertices_of_face_invalid_handle() {
    let m = BRepModel::new();
    let stale = Handle::<FaceData>::from_raw_parts(999, 0);
    assert!(m.vertices_of_face(stale).is_err());
}

#[test]
fn faces_of_edge_invalid_handle() {
    let m = BRepModel::new();
    let stale = Handle::<EdgeData>::from_raw_parts(999, 0);
    assert!(m.faces_of_edge(stale).is_err());
}
