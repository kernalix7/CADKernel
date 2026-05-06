//! Advanced integration tests for `cadkernel-topology` — V35 coverage expansion.
//!
//! Complements `topology_comprehensive.rs` without duplicating its coverage.
//! Focus areas:
//!   - Half-edge traversal invariants (twin round-trip, next/prev chains, fans).
//!   - Euler characteristic on closed polyhedra and open sheets.
//!   - Handle ordering, hashing, and generational behaviour across reuse.
//!   - Persistent naming: Tag roundtrip, serialization, segment structure.
//!   - Wire / Shell / Solid construction invariants and edge cases.
//!   - Error paths: stale handles on traversal helpers.
//!   - EntityStore: serialization, mixed insert/remove patterns.
//!   - Properties: material overwrite, metadata overwrite, all PropertyValue variants.
//!   - Evolution record enumeration.

use std::collections::{HashMap, HashSet};

use cadkernel_math::Point3;
use cadkernel_topology::naming::history::Evolution;
use cadkernel_topology::naming::tag::{SegmentKind, TagSegment};
use cadkernel_topology::*;

const TOL: f64 = 1e-10;

// =========================================================================
// Helpers — DIFFERENT from those in topology_comprehensive.rs.
// =========================================================================

/// Build a closed tetrahedron (4 triangular faces, manifold) and return the model
/// along with the outer shell handle.
fn make_closed_tetrahedron() -> (BRepModel, Handle<ShellData>) {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.5, 1.0, 0.0));
    let v3 = m.add_vertex(Point3::new(0.5, 0.5, 1.0));

    let (_, he01_a, he01_b) = m.add_edge(v0, v1);
    let (_, he02_a, he02_b) = m.add_edge(v0, v2);
    let (_, he03_a, he03_b) = m.add_edge(v0, v3);
    let (_, he12_a, he12_b) = m.add_edge(v1, v2);
    let (_, he13_a, he13_b) = m.add_edge(v1, v3);
    let (_, he23_a, he23_b) = m.add_edge(v2, v3);

    let l0 = m.make_loop(&[he01_a, he12_a, he02_b]).unwrap();
    let f0 = m.make_face(l0);
    let l1 = m.make_loop(&[he03_a, he13_b, he01_b]).unwrap();
    let f1 = m.make_face(l1);
    let l2 = m.make_loop(&[he02_a, he23_a, he03_b]).unwrap();
    let f2 = m.make_face(l2);
    let l3 = m.make_loop(&[he13_a, he23_b, he12_b]).unwrap();
    let f3 = m.make_face(l3);

    let shell = m.make_shell(&[f0, f1, f2, f3]);
    (m, shell)
}

/// Build an open triangular "sheet" (one face, no shell).
fn make_open_triangle() -> (BRepModel, Handle<FaceData>) {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(2.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 2.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let lh = m.make_loop(&[he01, he12, he20]).unwrap();
    let fh = m.make_face(lh);
    (m, fh)
}

// =========================================================================
// 1. Half-edge traversal invariants
// =========================================================================

#[test]
fn half_edge_twin_round_trips_every_edge() {
    let (m, _) = make_closed_tetrahedron();
    for (eh, ed) in m.edges.iter() {
        let a = ed.half_edge_a.expect("edge should have half_edge_a");
        let b = ed.half_edge_b.expect("edge should have half_edge_b");
        let a_twin = m.half_edges.get(a).unwrap().twin.unwrap();
        let b_twin = m.half_edges.get(b).unwrap().twin.unwrap();
        assert_eq!(a_twin, b, "edge {eh:?}: a.twin should be b");
        assert_eq!(b_twin, a, "edge {eh:?}: b.twin should be a");
    }
}

#[test]
fn half_edge_origin_differs_from_twin_origin() {
    let (m, _) = make_closed_tetrahedron();
    for (_, he) in m.half_edges.iter() {
        let twin = m.half_edges.get(he.twin.unwrap()).unwrap();
        assert_ne!(he.origin, twin.origin);
    }
}

#[test]
fn half_edge_next_prev_are_inverse_on_loops() {
    let (m, _) = make_closed_tetrahedron();
    for (_, he) in m.half_edges.iter() {
        let next_h = he.next.expect("every loop half-edge has next");
        let next = m.half_edges.get(next_h).unwrap();
        assert_eq!(
            next.prev,
            Some(
                m.half_edges
                    .iter()
                    .find(|(_, h)| h.next == Some(next_h))
                    .unwrap()
                    .0
            )
        );
    }
}

#[test]
fn half_edge_loop_closes_via_next_chain() {
    let (m, _) = make_closed_tetrahedron();
    for (_, ld) in m.loops.iter() {
        let start = ld.half_edge;
        let mut current = start;
        let mut visited = 0;
        loop {
            let he = m.half_edges.get(current).unwrap();
            let next = he.next.unwrap();
            visited += 1;
            if next == start {
                break;
            }
            current = next;
            assert!(visited < 1000, "loop did not close within 1000 steps");
        }
        assert_eq!(visited, 3, "tetrahedron face loop should have 3 edges");
    }
}

#[test]
fn half_edges_in_loop_all_reference_that_loop() {
    let (m, _) = make_open_triangle();
    for (loop_h, ld) in m.loops.iter() {
        let hes = m.loop_half_edges(ld.half_edge);
        assert_eq!(hes.len(), 3);
        for he_h in hes {
            let he = m.half_edges.get(he_h).unwrap();
            assert_eq!(he.loop_ref, Some(loop_h));
        }
    }
}

#[test]
fn sum_of_face_degrees_equals_twice_edge_count() {
    let (m, _) = make_closed_tetrahedron();
    let mut total_degree = 0usize;
    for (face_h, _) in m.faces.iter() {
        total_degree += m.edges_of_face(face_h).unwrap().len();
    }
    let e = m.edges.len();
    // Closed manifold: each edge is shared by exactly 2 faces.
    assert_eq!(total_degree, 2 * e);
}

#[test]
fn vertices_of_loop_match_half_edge_origins() {
    let (m, fh) = make_open_triangle();
    let verts = m.vertices_of_face(fh).unwrap();
    let fd = m.faces.get(fh).unwrap();
    let hes = m.loop_half_edges(m.loops.get(fd.outer_loop).unwrap().half_edge);
    let expected: Vec<_> = hes
        .iter()
        .map(|&h| m.half_edges.get(h).unwrap().origin)
        .collect();
    assert_eq!(verts, expected);
}

#[test]
fn loop_half_edges_is_deterministic() {
    let (m, fh) = make_open_triangle();
    let fd = m.faces.get(fh).unwrap();
    let start = m.loops.get(fd.outer_loop).unwrap().half_edge;
    let a = m.loop_half_edges(start);
    let b = m.loop_half_edges(start);
    assert_eq!(a, b);
}

// =========================================================================
// 2. Euler characteristic
// =========================================================================

#[test]
fn euler_characteristic_tetrahedron_equals_two() {
    let (m, _) = make_closed_tetrahedron();
    let v = m.vertices.len() as i64;
    let e = m.edges.len() as i64;
    let f = m.faces.len() as i64;
    assert_eq!(v, 4);
    assert_eq!(e, 6);
    assert_eq!(f, 4);
    assert_eq!(v - e + f, 2, "V-E+F of tetrahedron should be 2");
}

#[test]
fn euler_characteristic_open_triangle_equals_one() {
    let (m, _) = make_open_triangle();
    let v = m.vertices.len() as i64;
    let e = m.edges.len() as i64;
    let f = m.faces.len() as i64;
    assert_eq!(v - e + f, 1, "open sheet V-E+F should be 1");
}

#[test]
fn closed_tetrahedron_validates_as_manifold() {
    let (m, _) = make_closed_tetrahedron();
    assert!(m.validate_manifold().is_ok());
}

#[test]
fn tetrahedron_every_edge_has_two_faces() {
    let (m, _) = make_closed_tetrahedron();
    for (eh, _) in m.edges.iter() {
        let faces = m.faces_of_edge(eh).unwrap();
        assert_eq!(faces.len(), 2, "edge {eh:?} should be shared by 2 faces");
    }
}

#[test]
fn tetrahedron_every_vertex_has_three_faces() {
    let (m, _) = make_closed_tetrahedron();
    for (vh, _) in m.vertices.iter() {
        let faces = m.faces_around_vertex(vh).unwrap();
        assert_eq!(
            faces.len(),
            3,
            "tetrahedron vertex {vh:?} should belong to 3 faces"
        );
    }
}

#[test]
fn validate_detailed_reports_zero_errors_on_tetrahedron() {
    let (m, _) = make_closed_tetrahedron();
    let issues = m.validate_detailed();
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == ValidationSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "unexpected errors: {errors:?}");
}

// =========================================================================
// 3. Handle identity, equality, hashing
// =========================================================================

#[test]
fn handle_equality_reflexive_symmetric() {
    let a = Handle::<VertexData>::from_raw_parts(5, 3);
    let b = Handle::<VertexData>::from_raw_parts(5, 3);
    assert_eq!(a, b);
    assert_eq!(b, a);
    assert_eq!(a, a);
}

#[test]
fn handle_inequality_on_different_index() {
    let a = Handle::<EdgeData>::from_raw_parts(0, 0);
    let b = Handle::<EdgeData>::from_raw_parts(1, 0);
    assert_ne!(a, b);
}

#[test]
fn handle_inequality_on_different_generation() {
    let a = Handle::<FaceData>::from_raw_parts(0, 0);
    let b = Handle::<FaceData>::from_raw_parts(0, 1);
    assert_ne!(a, b);
}

#[test]
fn handle_hash_agrees_with_eq() {
    let a = Handle::<VertexData>::from_raw_parts(7, 11);
    let b = Handle::<VertexData>::from_raw_parts(7, 11);
    let mut set: HashSet<Handle<VertexData>> = HashSet::new();
    set.insert(a);
    assert!(set.contains(&b));
}

#[test]
fn handle_debug_is_non_empty() {
    let h = Handle::<EdgeData>::from_raw_parts(3, 2);
    let s = format!("{h:?}");
    assert!(!s.is_empty());
}

#[test]
fn handle_copy_does_not_consume() {
    let h = Handle::<VertexData>::from_raw_parts(1, 0);
    let h2 = h;
    // If copy works, both variables remain usable.
    assert_eq!(h, h2);
    assert_eq!(h.index(), 1);
    assert_eq!(h2.index(), 1);
}

#[test]
fn handle_serialization_roundtrip() {
    let h = Handle::<FaceData>::from_raw_parts(42, 7);
    let json = serde_json::to_string(&h).expect("serialize Handle");
    let restored: Handle<FaceData> = serde_json::from_str(&json).expect("deserialize Handle");
    assert_eq!(h, restored);
    assert_eq!(restored.index(), 42);
    assert_eq!(restored.generation(), 7);
}

// =========================================================================
// 4. EntityStore: generational behaviour, serialization, mixed ops
// =========================================================================

#[test]
fn entity_store_stale_handle_after_remove() {
    let mut s = EntityStore::<i32>::new();
    let h = s.insert(100);
    assert!(s.is_alive(h));
    s.remove(h);
    assert!(!s.is_alive(h));
    assert!(s.get(h).is_none());
    assert!(s.get_mut(h).is_none());
    assert!(s.remove(h).is_none(), "second remove returns None");
}

#[test]
fn entity_store_generation_increments_on_remove() {
    let mut s = EntityStore::<i32>::new();
    let h = s.insert(1);
    let g1 = h.generation();
    s.remove(h);
    let h2 = s.insert(2);
    assert_eq!(h2.index(), h.index(), "slot reused");
    assert!(h2.generation() > g1, "generation must strictly increase");
}

#[test]
fn entity_store_fabricated_handle_returns_none() {
    let s = EntityStore::<i32>::new();
    let fake = Handle::<i32>::from_raw_parts(123, 456);
    assert!(s.get(fake).is_none());
    assert!(!s.is_alive(fake));
}

#[test]
fn entity_store_mixed_insert_remove_preserves_live_count() {
    let mut s = EntityStore::<i32>::new();
    let h1 = s.insert(1);
    let _h2 = s.insert(2);
    let h3 = s.insert(3);
    assert_eq!(s.len(), 3);
    s.remove(h1);
    assert_eq!(s.len(), 2);
    s.remove(h3);
    assert_eq!(s.len(), 1);
    assert!(!s.is_empty());
    // iter should yield only live entry (value 2).
    let vals: Vec<_> = s.iter().map(|(_, v)| *v).collect();
    assert_eq!(vals, vec![2]);
}

#[test]
fn entity_store_iter_skips_removed() {
    let mut s = EntityStore::<i32>::new();
    let _a = s.insert(10);
    let b = s.insert(20);
    let _c = s.insert(30);
    s.remove(b);
    let live: Vec<_> = s.iter().map(|(_, v)| *v).collect();
    assert_eq!(live, vec![10, 30]);
}

#[test]
fn entity_store_default_matches_new() {
    let a: EntityStore<i32> = EntityStore::default();
    let b: EntityStore<i32> = EntityStore::new();
    assert_eq!(a.len(), b.len());
    assert!(a.is_empty() && b.is_empty());
}

#[test]
fn entity_store_serialization_roundtrip() {
    let mut s = EntityStore::<i32>::new();
    s.insert(11);
    s.insert(22);
    s.insert(33);
    let json = serde_json::to_string(&s).unwrap();
    let restored: EntityStore<i32> = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.len(), 3);
    let vals: Vec<_> = restored.iter().map(|(_, v)| *v).collect();
    assert_eq!(vals, vec![11, 22, 33]);
}

#[test]
fn entity_store_is_empty_after_removing_all() {
    let mut s = EntityStore::<i32>::new();
    let h1 = s.insert(1);
    let h2 = s.insert(2);
    s.remove(h1);
    s.remove(h2);
    assert!(s.is_empty());
    assert_eq!(s.len(), 0);
    assert_eq!(s.iter().count(), 0);
}

// =========================================================================
// 5. ShapeHistory: monotonicity and record ordering
// =========================================================================

#[test]
fn shape_history_operation_ids_are_monotonic() {
    let mut h = ShapeHistory::new();
    let mut prev = 0;
    for i in 0..20 {
        let op = h.next_operation(format!("op_{i}"));
        assert!(op.0 > prev, "operation id must strictly increase");
        prev = op.0;
    }
}

#[test]
fn shape_history_first_op_id_is_one() {
    let mut h = ShapeHistory::new();
    let op = h.next_operation("first");
    assert_eq!(op.0, 1);
}

#[test]
fn shape_history_records_returns_slice_in_order() {
    let mut h = ShapeHistory::new();
    let op1 = h.next_operation("a");
    let op2 = h.next_operation("b");
    let op3 = h.next_operation("c");
    let recs = h.records();
    assert_eq!(recs.len(), 3);
    assert_eq!(recs[0].operation, op1);
    assert_eq!(recs[1].operation, op2);
    assert_eq!(recs[2].operation, op3);
    assert_eq!(recs[0].label, "a");
    assert_eq!(recs[2].label, "c");
}

#[test]
fn shape_history_evolution_records_attach_to_current_op() {
    let mut h = ShapeHistory::new();
    let op = h.next_operation("op");
    let t = Tag::generated(EntityKind::Face, op, 0);
    h.record(Evolution::Generated {
        tag: t,
        kind: EntityKind::Face,
    });
    let rec = h.get_record(op).unwrap();
    assert_eq!(rec.evolutions.len(), 1);
}

#[test]
fn shape_history_default_is_empty() {
    let h: ShapeHistory = ShapeHistory::default();
    assert!(h.current_op_id().is_none());
    assert!(h.records().is_empty());
}

#[test]
fn shape_history_record_without_op_is_noop() {
    let mut h = ShapeHistory::new();
    // No next_operation — no current record — record() silently skips.
    h.record(Evolution::Deleted {
        tag: Tag::generated(EntityKind::Face, OperationId(1), 0),
    });
    assert!(h.records().is_empty());
}

// =========================================================================
// 6. Tag structure & persistent naming
// =========================================================================

#[test]
fn tag_new_matches_generated() {
    let op = OperationId(5);
    let a = Tag::new(
        EntityKind::Face,
        vec![TagSegment {
            operation: op,
            kind: SegmentKind::Generated(3),
        }],
    );
    let b = Tag::generated(EntityKind::Face, op, 3);
    assert_eq!(a, b);
}

#[test]
fn tag_segments_are_copies_not_aliases() {
    let a = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let b = a.modified(OperationId(2));
    // Appending to b must not affect a.
    assert_eq!(a.segments.len(), 1);
    assert_eq!(b.segments.len(), 2);
}

#[test]
fn tag_operation_ids_are_preserved_in_segments() {
    let t = Tag::generated(EntityKind::Vertex, OperationId(10), 7);
    assert_eq!(t.segments[0].operation, OperationId(10));
    assert_eq!(t.segments[0].kind, SegmentKind::Generated(7));
}

#[test]
fn tag_local_index_is_preserved_in_split() {
    let t = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let s0 = t.split(OperationId(2), 5);
    let s1 = t.split(OperationId(2), 6);
    assert_ne!(s0, s1);
    assert_eq!(s0.segments[1].kind, SegmentKind::Split(5));
    assert_eq!(s1.segments[1].kind, SegmentKind::Split(6));
}

#[test]
fn tag_serialization_roundtrip() {
    let t = Tag::generated(EntityKind::Edge, OperationId(2), 4)
        .modified(OperationId(3))
        .split(OperationId(4), 1);
    let json = serde_json::to_string(&t).unwrap();
    let restored: Tag = serde_json::from_str(&json).unwrap();
    assert_eq!(t, restored);
    assert_eq!(restored.segments.len(), 3);
}

#[test]
fn tag_uniqueness_across_kind_op_index_tuple() {
    let mut seen: HashSet<Tag> = HashSet::new();
    for kind in [
        EntityKind::Vertex,
        EntityKind::Edge,
        EntityKind::Face,
        EntityKind::Shell,
        EntityKind::Solid,
        EntityKind::HalfEdge,
        EntityKind::Loop,
        EntityKind::Wire,
    ] {
        for op in 1u64..=3 {
            for idx in 0u32..4 {
                let t = Tag::generated(kind, OperationId(op), idx);
                assert!(seen.insert(t), "duplicate tag for {kind:?} op{op} idx{idx}");
            }
        }
    }
    // 8 kinds * 3 ops * 4 indices = 96.
    assert_eq!(seen.len(), 96);
}

#[test]
fn segment_kind_equality_and_hash() {
    let a = SegmentKind::Generated(3);
    let b = SegmentKind::Generated(3);
    let c = SegmentKind::Split(3);
    assert_eq!(a, b);
    assert_ne!(a, c);

    let mut set: HashSet<SegmentKind> = HashSet::new();
    set.insert(SegmentKind::Modified);
    set.insert(SegmentKind::Merged);
    set.insert(SegmentKind::Generated(0));
    set.insert(SegmentKind::Split(0));
    assert_eq!(set.len(), 4);
}

#[test]
fn operation_id_equality_and_hash() {
    let a = OperationId(42);
    let b = OperationId(42);
    let c = OperationId(43);
    assert_eq!(a, b);
    assert_ne!(a, c);
    let mut set: HashSet<OperationId> = HashSet::new();
    set.insert(a);
    assert!(set.contains(&b));
    assert!(!set.contains(&c));
}

#[test]
fn entity_kind_equality_and_variants_distinct() {
    let kinds = [
        EntityKind::Vertex,
        EntityKind::Edge,
        EntityKind::HalfEdge,
        EntityKind::Loop,
        EntityKind::Wire,
        EntityKind::Face,
        EntityKind::Shell,
        EntityKind::Solid,
    ];
    // All variants must be distinct.
    let set: HashSet<EntityKind> = kinds.iter().copied().collect();
    assert_eq!(set.len(), kinds.len());
}

// =========================================================================
// 7. NameMap edge cases
// =========================================================================

#[test]
fn name_map_insert_overwrites_existing() {
    let mut map = NameMap::new();
    let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let h1 = Handle::<FaceData>::from_raw_parts(0, 0);
    let h2 = Handle::<FaceData>::from_raw_parts(1, 0);
    map.insert(tag.clone(), EntityRef::Face(h1));
    map.insert(tag.clone(), EntityRef::Face(h2));
    assert_eq!(map.len(), 1);
    assert_eq!(map.get_face(&tag), Some(h2));
}

#[test]
fn name_map_kind_mismatch_returns_none() {
    let mut map = NameMap::new();
    let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let h = Handle::<FaceData>::from_raw_parts(0, 0);
    map.insert(tag.clone(), EntityRef::Face(h));
    // Asking for a different kind should return None.
    assert!(map.get_vertex(&tag).is_none());
    assert!(map.get_edge(&tag).is_none());
    assert!(map.get_wire(&tag).is_none());
    assert!(map.get_shell(&tag).is_none());
    assert!(map.get_solid(&tag).is_none());
}

#[test]
fn name_map_remove_twice_returns_none() {
    let mut map = NameMap::new();
    let tag = Tag::generated(EntityKind::Solid, OperationId(1), 0);
    let h = Handle::<SolidData>::from_raw_parts(0, 0);
    map.insert(tag.clone(), EntityRef::Solid(h));
    assert!(map.remove(&tag).is_some());
    assert!(map.remove(&tag).is_none());
}

#[test]
fn entity_ref_copy_and_eq() {
    let r = EntityRef::Face(Handle::<FaceData>::from_raw_parts(0, 0));
    let r2 = r;
    assert_eq!(r, r2);
    assert_eq!(r2.kind(), EntityKind::Face);
}

#[test]
fn entity_ref_serialization_roundtrip() {
    let r = EntityRef::Edge(Handle::<EdgeData>::from_raw_parts(3, 2));
    let json = serde_json::to_string(&r).unwrap();
    let restored: EntityRef = serde_json::from_str(&json).unwrap();
    assert_eq!(r, restored);
}

// =========================================================================
// 8. Wire / Shell / Solid construction invariants
// =========================================================================

#[test]
fn wire_empty_is_empty() {
    let mut m = BRepModel::new();
    let wh = m.make_wire(vec![], false);
    let wd = m.wires.get(wh).unwrap();
    assert!(wd.is_empty());
    assert_eq!(wd.len(), 0);
    assert!(!wd.is_closed);
}

#[test]
fn wire_single_half_edge_open() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (_, he, _) = m.add_edge(v0, v1);
    let wh = m.make_wire(vec![he], false);
    let wd = m.wires.get(wh).unwrap();
    assert_eq!(wd.len(), 1);
    assert!(!wd.is_closed);
}

#[test]
fn wire_data_new_captures_flags() {
    let hes: Vec<Handle<HalfEdgeData>> = Vec::new();
    let w = WireData::new(hes.clone(), true);
    assert!(w.is_closed);
    assert!(w.is_empty());
    let w2 = WireData::new(hes, false);
    assert!(!w2.is_closed);
}

#[test]
fn shell_new_is_empty() {
    let s = ShellData::new();
    assert!(s.faces.is_empty());
    assert!(s.solid.is_none());
    assert!(s.tag.is_none());
}

#[test]
fn shell_default_equals_new() {
    let s1 = ShellData::default();
    let s2 = ShellData::new();
    assert_eq!(s1.faces.len(), s2.faces.len());
    assert_eq!(s1.solid, s2.solid);
}

#[test]
fn shell_construction_links_face_back_to_shell() {
    let (mut m, fh) = make_open_triangle();
    let sh = m.make_shell(&[fh]);
    let fd = m.faces.get(fh).unwrap();
    assert_eq!(fd.shell, Some(sh));
}

#[test]
fn shell_with_multiple_faces_links_all() {
    let (m, _) = make_closed_tetrahedron();
    for (_, fd) in m.faces.iter() {
        assert!(fd.shell.is_some(), "every face should belong to the shell");
    }
}

#[test]
fn solid_new_is_empty() {
    let s = SolidData::new();
    assert!(s.shells.is_empty());
    assert!(s.tag.is_none());
}

#[test]
fn solid_default_equals_new() {
    let s1 = SolidData::default();
    let s2 = SolidData::new();
    assert_eq!(s1.shells.len(), s2.shells.len());
}

#[test]
fn solid_make_links_shell_back_to_solid() {
    let (mut m, sh) = make_closed_tetrahedron();
    let solid = m.make_solid(&[sh]);
    let shell = m.shells.get(sh).unwrap();
    assert_eq!(shell.solid, Some(solid));
}

// =========================================================================
// 9. BRepModel: traversal error paths
// =========================================================================

#[test]
fn vertices_of_face_returns_invalid_handle_for_stale() {
    let m = BRepModel::new();
    let stale = Handle::<FaceData>::from_raw_parts(0, 99);
    let err = m.vertices_of_face(stale).unwrap_err();
    assert!(err.is_invalid_handle());
}

#[test]
fn edges_of_face_returns_invalid_handle_for_stale() {
    let m = BRepModel::new();
    let stale = Handle::<FaceData>::from_raw_parts(1000, 0);
    let err = m.edges_of_face(stale).unwrap_err();
    assert!(err.is_invalid_handle());
}

#[test]
fn faces_of_edge_returns_invalid_handle_for_stale() {
    let m = BRepModel::new();
    let stale = Handle::<EdgeData>::from_raw_parts(1000, 0);
    let err = m.faces_of_edge(stale).unwrap_err();
    assert!(err.is_invalid_handle());
}

#[test]
fn faces_around_vertex_returns_invalid_handle_for_stale() {
    let m = BRepModel::new();
    let stale = Handle::<VertexData>::from_raw_parts(1000, 0);
    let err = m.faces_around_vertex(stale).unwrap_err();
    assert!(err.is_invalid_handle());
}

#[test]
fn make_loop_zero_half_edges_is_error() {
    let mut m = BRepModel::new();
    let err = m.make_loop(&[]).unwrap_err();
    assert!(err.to_string().contains("2 half-edges"));
}

// =========================================================================
// 10. BRepModel transform propagation
// =========================================================================

#[test]
fn transform_translation_shifts_all_vertices() {
    let (mut m, _) = make_closed_tetrahedron();
    let before: Vec<Point3> = m.vertices.iter().map(|(_, v)| v.point).collect();
    let t = cadkernel_math::Transform::translation(3.0, -2.0, 5.0);
    m.transform(&t);
    for ((_, v), p0) in m.vertices.iter().zip(before.iter()) {
        assert!((v.point.x - p0.x - 3.0).abs() < TOL);
        assert!((v.point.y - p0.y + 2.0).abs() < TOL);
        assert!((v.point.z - p0.z - 5.0).abs() < TOL);
    }
}

#[test]
fn transform_identity_leaves_vertices_unchanged() {
    let (mut m, _) = make_open_triangle();
    let before: Vec<Point3> = m.vertices.iter().map(|(_, v)| v.point).collect();
    let t = cadkernel_math::Transform::translation(0.0, 0.0, 0.0);
    m.transform(&t);
    for ((_, v), p0) in m.vertices.iter().zip(before.iter()) {
        assert!(v.point.approx_eq(*p0));
    }
}

// =========================================================================
// 11. Properties: overwrite and variants
// =========================================================================

#[test]
fn property_store_material_overwrite() {
    let mut s = PropertyStore::new();
    s.set_material(0, Material::steel());
    s.set_material(0, Material::aluminum());
    let m = s.get_material(0).unwrap();
    assert_eq!(m.name, "Aluminum");
}

#[test]
fn property_store_metadata_overwrite() {
    let mut s = PropertyStore::new();
    s.set_metadata(0, "speed", PropertyValue::Int(10));
    s.set_metadata(0, "speed", PropertyValue::Int(20));
    assert_eq!(s.get_metadata(0, "speed"), Some(&PropertyValue::Int(20)));
}

#[test]
fn property_store_separate_entities_are_isolated() {
    let mut s = PropertyStore::new();
    s.set_material(0, Material::steel());
    s.set_material(1, Material::wood());
    assert_eq!(s.get_material(0).unwrap().name, "Steel");
    assert_eq!(s.get_material(1).unwrap().name, "Wood");
    assert!(s.get_material(2).is_none());
}

#[test]
fn property_value_variant_equality() {
    assert_eq!(PropertyValue::Int(1), PropertyValue::Int(1));
    assert_ne!(PropertyValue::Int(1), PropertyValue::Int(2));
    assert_ne!(PropertyValue::Int(1), PropertyValue::Float(1.0));
    assert_eq!(
        PropertyValue::String("x".into()),
        PropertyValue::String("x".into())
    );
    assert_eq!(PropertyValue::Bool(true), PropertyValue::Bool(true));
    assert_ne!(PropertyValue::Bool(true), PropertyValue::Bool(false));
}

#[test]
fn property_value_serialization_roundtrip() {
    let cases = [
        PropertyValue::String("label".into()),
        PropertyValue::Int(-42),
        PropertyValue::Float(2.5),
        PropertyValue::Bool(false),
    ];
    for p in cases {
        let json = serde_json::to_string(&p).unwrap();
        let restored: PropertyValue = serde_json::from_str(&json).unwrap();
        assert_eq!(p, restored);
    }
}

#[test]
fn property_store_serialization_roundtrip() {
    let mut s = PropertyStore::new();
    s.set_material(0, Material::steel());
    s.set_metadata(0, "count", PropertyValue::Int(5));
    s.set_metadata(1, "name", PropertyValue::String("bolt".into()));
    let json = serde_json::to_string(&s).unwrap();
    let restored: PropertyStore = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.get_material(0).unwrap().name, "Steel");
    assert_eq!(
        restored.get_metadata(0, "count"),
        Some(&PropertyValue::Int(5))
    );
    assert_eq!(
        restored.get_metadata(1, "name"),
        Some(&PropertyValue::String("bolt".into()))
    );
}

#[test]
fn color_rgba_alpha_respected() {
    let c = Color::rgba(0.1, 0.2, 0.3, 0.25);
    assert!((c.r - 0.1).abs() < TOL);
    assert!((c.g - 0.2).abs() < TOL);
    assert!((c.b - 0.3).abs() < TOL);
    assert!((c.a - 0.25).abs() < TOL);
}

#[test]
fn color_serialization_roundtrip() {
    let c = Color::rgba(0.5, 0.25, 0.125, 0.75);
    let json = serde_json::to_string(&c).unwrap();
    let restored: Color = serde_json::from_str(&json).unwrap();
    assert_eq!(c, restored);
}

#[test]
fn material_default_name_used_in_new() {
    let m = Material::new("Custom");
    assert_eq!(m.name, "Custom");
    assert_eq!(m.density, 0.0);
    // Default color is GRAY.
    assert!((m.color.r - 0.5).abs() < TOL);
    assert!((m.roughness - 0.5).abs() < TOL);
    assert!((m.metallic - 0.0).abs() < TOL);
}

// =========================================================================
// 12. BRepModel: tagged construction and NameMap sync
// =========================================================================

#[test]
fn make_face_tagged_registers_in_name_map() {
    let (mut m, _) = make_open_triangle();
    let op = m.history.next_operation("op");
    let v0 = m.add_vertex(Point3::new(5.0, 5.0, 0.0));
    let v1 = m.add_vertex(Point3::new(6.0, 5.0, 0.0));
    let v2 = m.add_vertex(Point3::new(5.5, 6.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let lh = m.make_loop(&[he01, he12, he20]).unwrap();
    let tag = Tag::generated(EntityKind::Face, op, 0);
    let fh = m.make_face_tagged(lh, tag.clone());
    assert_eq!(m.find_face_by_tag(&tag), Some(fh));
    assert_eq!(m.faces.get(fh).unwrap().tag.as_ref(), Some(&tag));
}

#[test]
fn add_vertex_tagged_registers_in_name_map() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("op");
    let tag = Tag::generated(EntityKind::Vertex, op, 0);
    let vh = m.add_vertex_tagged(Point3::new(1.0, 2.0, 3.0), tag.clone());
    assert_eq!(m.find_vertex_by_tag(&tag), Some(vh));
    let vd = m.vertices.get(vh).unwrap();
    assert!(vd.point.approx_eq(Point3::new(1.0, 2.0, 3.0)));
}

#[test]
fn find_by_wrong_kind_returns_none() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("op");
    let vtag = Tag::generated(EntityKind::Vertex, op, 0);
    m.add_vertex_tagged(Point3::ORIGIN, vtag.clone());
    // The vertex tag exists but looking it up as a face returns None.
    assert!(m.find_face_by_tag(&vtag).is_none());
    assert!(m.find_edge_by_tag(&vtag).is_none());
}

#[test]
fn duplicate_tag_overwrite_is_last_write_wins() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("op");
    let tag = Tag::generated(EntityKind::Vertex, op, 0);
    let v1 = m.add_vertex_tagged(Point3::new(1.0, 0.0, 0.0), tag.clone());
    let v2 = m.add_vertex_tagged(Point3::new(2.0, 0.0, 0.0), tag.clone());
    assert_ne!(v1, v2);
    assert_eq!(m.find_vertex_by_tag(&tag), Some(v2));
}

#[test]
fn many_tags_coexist_in_name_map() {
    let mut m = BRepModel::new();
    let op = m.history.next_operation("op");
    let mut handles: HashMap<Tag, Handle<VertexData>> = HashMap::new();
    for i in 0..30u32 {
        let tag = Tag::generated(EntityKind::Vertex, op, i);
        let vh = m.add_vertex_tagged(Point3::new(i as f64, 0.0, 0.0), tag.clone());
        handles.insert(tag, vh);
    }
    assert_eq!(handles.len(), 30);
    for (tag, expected) in &handles {
        assert_eq!(m.find_vertex_by_tag(tag), Some(*expected));
    }
}

// =========================================================================
// 13. BRepModel: serialization of full model
// =========================================================================

#[test]
fn brep_model_new_has_zero_counts_everywhere() {
    let m = BRepModel::new();
    assert_eq!(m.vertices.len(), 0);
    assert_eq!(m.edges.len(), 0);
    assert_eq!(m.half_edges.len(), 0);
    assert_eq!(m.loops.len(), 0);
    assert_eq!(m.wires.len(), 0);
    assert_eq!(m.faces.len(), 0);
    assert_eq!(m.shells.len(), 0);
    assert_eq!(m.solids.len(), 0);
    assert!(m.name_map.is_empty());
    assert!(m.history.records().is_empty());
}

#[test]
fn brep_tetrahedron_serialization_preserves_counts() {
    let (m, _) = make_closed_tetrahedron();
    let json = serde_json::to_string(&m).unwrap();
    let restored: BRepModel = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.vertices.len(), 4);
    assert_eq!(restored.edges.len(), 6);
    assert_eq!(restored.faces.len(), 4);
    assert_eq!(restored.shells.len(), 1);
}

#[test]
fn brep_tetrahedron_serialization_preserves_euler() {
    let (m, _) = make_closed_tetrahedron();
    let json = serde_json::to_string(&m).unwrap();
    let restored: BRepModel = serde_json::from_str(&json).unwrap();
    let v = restored.vertices.len() as i64;
    let e = restored.edges.len() as i64;
    let f = restored.faces.len() as i64;
    assert_eq!(v - e + f, 2);
}

// =========================================================================
// 14. ValidationIssue / ValidationSeverity
// =========================================================================

#[test]
fn validation_severity_equality() {
    assert_eq!(ValidationSeverity::Error, ValidationSeverity::Error);
    assert_ne!(ValidationSeverity::Error, ValidationSeverity::Warning);
}

#[test]
fn validate_dangling_shell_face_is_error() {
    let (mut m, fh) = make_open_triangle();
    let _sh = m.make_shell(&[fh]);
    // Delete the face — the shell now has a dangling reference.
    m.faces.remove(fh);
    let result = m.validate();
    assert!(result.is_err());
}

#[test]
fn validate_detailed_empty_model_has_no_errors() {
    let m = BRepModel::new();
    let issues = m.validate_detailed();
    // Empty model is trivially valid.
    let errors: Vec<_> = issues
        .iter()
        .filter(|i| i.severity == ValidationSeverity::Error)
        .collect();
    assert!(errors.is_empty());
}

// =========================================================================
// 15. Evolution record variants (smoke + equality)
// =========================================================================

#[test]
fn evolution_variants_all_serializable() {
    let tag_a = Tag::generated(EntityKind::Face, OperationId(1), 0);
    let tag_b = Tag::generated(EntityKind::Face, OperationId(1), 1);
    let cases = [
        Evolution::Generated {
            tag: tag_a.clone(),
            kind: EntityKind::Face,
        },
        Evolution::Modified {
            old_tag: tag_a.clone(),
            new_tag: tag_b.clone(),
        },
        Evolution::Split {
            parent_tag: tag_a.clone(),
            child_tags: vec![
                tag_a.split(OperationId(2), 0),
                tag_a.split(OperationId(2), 1),
            ],
        },
        Evolution::Deleted { tag: tag_b },
    ];
    for ev in cases {
        let json = serde_json::to_string(&ev).unwrap();
        // Round-trip only checks it deserializes; Evolution is not Eq.
        let _restored: Evolution = serde_json::from_str(&json).unwrap();
    }
}

// =========================================================================
// 16. Orientation enum
// =========================================================================

#[test]
fn orientation_equality() {
    assert_eq!(Orientation::Forward, Orientation::Forward);
    assert_eq!(Orientation::Reversed, Orientation::Reversed);
    assert_ne!(Orientation::Forward, Orientation::Reversed);
}

#[test]
fn orientation_copy() {
    let a = Orientation::Forward;
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn orientation_serialization_roundtrip() {
    for o in [Orientation::Forward, Orientation::Reversed] {
        let json = serde_json::to_string(&o).unwrap();
        let restored: Orientation = serde_json::from_str(&json).unwrap();
        assert_eq!(o, restored);
    }
}

#[test]
fn face_new_default_orientation_is_forward() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let lh = m.make_loop(&[he01, he12, he20]).unwrap();
    let fh = m.make_face(lh);
    assert_eq!(m.faces.get(fh).unwrap().orientation, Orientation::Forward);
}

// =========================================================================
// 17. Vertex outgoing half-edge bookkeeping
// =========================================================================

#[test]
fn first_add_edge_sets_vertex_outgoing_half_edge() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    assert!(m.vertices.get(v0).unwrap().half_edge.is_none());
    let (_, he_a, he_b) = m.add_edge(v0, v1);
    assert_eq!(m.vertices.get(v0).unwrap().half_edge, Some(he_a));
    assert_eq!(m.vertices.get(v1).unwrap().half_edge, Some(he_b));
}

#[test]
fn second_add_edge_does_not_overwrite_vertex_outgoing() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(2.0, 0.0, 0.0));
    let (_, he_a, _) = m.add_edge(v0, v1);
    // Second edge uses v1 again — v1's half_edge should not be overwritten.
    let before = m.vertices.get(v1).unwrap().half_edge;
    let (_, _, _) = m.add_edge(v1, v2);
    let after = m.vertices.get(v1).unwrap().half_edge;
    assert_eq!(before, after, "v1 outgoing half-edge should be stable");
    // v0 was only set by the first edge.
    assert_eq!(m.vertices.get(v0).unwrap().half_edge, Some(he_a));
}

// =========================================================================
// 18. make_loop parameter validation
// =========================================================================

#[test]
fn make_loop_sets_loop_ref_on_every_half_edge() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    let lh = m.make_loop(&[he01, he12, he20]).unwrap();
    for &h in &[he01, he12, he20] {
        assert_eq!(m.half_edges.get(h).unwrap().loop_ref, Some(lh));
    }
}

#[test]
fn make_loop_next_prev_form_cycle() {
    let mut m = BRepModel::new();
    let v0 = m.add_vertex(Point3::ORIGIN);
    let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = m.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he01, _) = m.add_edge(v0, v1);
    let (_, he12, _) = m.add_edge(v1, v2);
    let (_, he20, _) = m.add_edge(v2, v0);
    m.make_loop(&[he01, he12, he20]).unwrap();

    let a = m.half_edges.get(he01).unwrap();
    let b = m.half_edges.get(he12).unwrap();
    let c = m.half_edges.get(he20).unwrap();
    assert_eq!(a.next, Some(he12));
    assert_eq!(b.next, Some(he20));
    assert_eq!(c.next, Some(he01));
    assert_eq!(a.prev, Some(he20));
    assert_eq!(b.prev, Some(he01));
    assert_eq!(c.prev, Some(he12));
}

// =========================================================================
// 19. Cross-module: validation on realistic topology
// =========================================================================

#[test]
fn validate_triangle_sheet_without_shell_succeeds() {
    let (m, _) = make_open_triangle();
    // No shell => no Euler check applies => validates OK.
    assert!(m.validate().is_ok());
}

#[test]
fn validate_detailed_on_empty_model_produces_no_issues() {
    let m = BRepModel::new();
    let issues = m.validate_detailed();
    assert!(issues.is_empty());
}

#[test]
fn validate_manifold_on_open_sheet_fails() {
    let (mut m, fh) = make_open_triangle();
    let _sh = m.make_shell(&[fh]);
    // Open sheet with a shell — every edge is shared by only 1 face — not manifold.
    let result = m.validate_manifold();
    assert!(result.is_err());
}

// =========================================================================
// 20. Misc: basic smoke around model clone
// =========================================================================

#[test]
fn brep_model_clone_is_independent() {
    let (mut a, _) = make_open_triangle();
    let b = a.clone();
    a.add_vertex(Point3::new(9.0, 9.0, 9.0));
    assert_ne!(a.vertices.len(), b.vertices.len());
}

#[test]
fn brep_model_default_is_empty_like_new() {
    let a = BRepModel::default();
    let b = BRepModel::new();
    assert_eq!(a.vertices.len(), b.vertices.len());
    assert_eq!(a.edges.len(), b.edges.len());
}
