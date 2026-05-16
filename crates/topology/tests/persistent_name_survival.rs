use cadkernel_math::Point3;
use cadkernel_topology::naming::FeatureId;
use cadkernel_topology::{
    BRepModel, EdgeData, EntityKind, FaceData, Handle, OperationId, PersistentNameTable, Tag,
    VertexData,
};

fn polygon_model(points: &[Point3]) -> (BRepModel, OperationId) {
    let mut model = BRepModel::new();
    let op = model.history.next_operation("polygon");
    let mut vertices = Vec::with_capacity(points.len());
    for (idx, point) in points.iter().copied().enumerate() {
        vertices.push(
            model.add_vertex_tagged(point, Tag::generated(EntityKind::Vertex, op, idx as u32)),
        );
    }

    let mut half_edges = Vec::with_capacity(vertices.len());
    for idx in 0..vertices.len() {
        let next = (idx + 1) % vertices.len();
        let edge_tag = Tag::generated(EntityKind::Edge, op, idx as u32);
        let (_edge, he, _) = model.add_edge_tagged(vertices[idx], vertices[next], edge_tag);
        half_edges.push(he);
    }

    let loop_h = model.make_loop(&half_edges).expect("loop");
    let face = model.make_face_tagged(loop_h, Tag::generated(EntityKind::Face, op, 0));
    let shell = model.make_shell_tagged(&[face], Tag::generated(EntityKind::Shell, op, 0));
    model.make_solid_tagged(&[shell], Tag::generated(EntityKind::Solid, op, 0));
    (model, op)
}

fn quad_model(width: f64, height: f64, z: f64) -> (BRepModel, OperationId) {
    polygon_model(&[
        Point3::new(0.0, 0.0, z),
        Point3::new(width, 0.0, z),
        Point3::new(width, height, z),
        Point3::new(0.0, height, z),
    ])
}

fn triangle_model() -> (BRepModel, OperationId) {
    polygon_model(&[
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(1.0, 1.0, 0.0),
    ])
}

fn first_face_tag(model: &BRepModel) -> Tag {
    model.faces.iter().next().unwrap().1.tag.clone().unwrap()
}

fn first_edge_tag(model: &BRepModel) -> Tag {
    model.edges.iter().next().unwrap().1.tag.clone().unwrap()
}

fn first_vertex_tag(model: &BRepModel) -> Tag {
    model.vertices.iter().next().unwrap().1.tag.clone().unwrap()
}

fn first_face(model: &BRepModel) -> Handle<FaceData> {
    model.faces.iter().next().unwrap().0
}

fn first_edge(model: &BRepModel) -> Handle<EdgeData> {
    model.edges.iter().next().unwrap().0
}

fn first_vertex(model: &BRepModel) -> Handle<VertexData> {
    model.vertices.iter().next().unwrap().0
}

fn register_all(table: &mut PersistentNameTable, feature: FeatureId, model: &BRepModel) {
    table.register_model_feature(feature, model);
}

#[test]
fn table_starts_empty() {
    let table = PersistentNameTable::new();
    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
}

#[test]
fn register_returns_generated_tag() {
    let mut table = PersistentNameTable::new();
    let tag = table.register(FeatureId(1), OperationId(7), EntityKind::Face, 3);
    assert_eq!(tag, Tag::generated(EntityKind::Face, OperationId(7), 3));
}

#[test]
fn register_same_axis_reuses_existing_tag() {
    let mut table = PersistentNameTable::new();
    let a = table.register(FeatureId(1), OperationId(1), EntityKind::Edge, 2);
    let b = table.register(FeatureId(1), OperationId(1), EntityKind::Edge, 2);
    assert_eq!(a, b);
    assert_eq!(table.len(), 1);
}

#[test]
fn resolve_registered_tag_returns_axis() {
    let mut table = PersistentNameTable::new();
    let tag = table.register(FeatureId(4), OperationId(9), EntityKind::Vertex, 5);
    assert_eq!(
        table.resolve(&tag),
        Some((FeatureId(4), OperationId(9), EntityKind::Vertex, 5))
    );
}

#[test]
fn resolve_missing_tag_returns_none() {
    let table = PersistentNameTable::new();
    let tag = Tag::generated(EntityKind::Face, OperationId(1), 0);
    assert_eq!(table.resolve(&tag), None);
}

#[test]
fn register_existing_uses_supplied_tag() {
    let mut table = PersistentNameTable::new();
    let supplied = Tag::generated(EntityKind::Face, OperationId(3), 0).modified(OperationId(4));
    let tag = table.register_existing(
        FeatureId(2),
        OperationId(3),
        EntityKind::Face,
        0,
        supplied.clone(),
    );
    assert_eq!(tag, supplied);
    assert_eq!(
        table.resolve(&supplied),
        Some((FeatureId(2), OperationId(3), EntityKind::Face, 0))
    );
}

#[test]
fn register_model_feature_tracks_existing_entity_tags() {
    let (model, op) = quad_model(1.0, 1.0, 0.0);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(10), &model);
    assert_eq!(
        table.resolve(&Tag::generated(EntityKind::Face, op, 0)),
        Some((FeatureId(10), op, EntityKind::Face, 0))
    );
    assert_eq!(
        table.resolve(&Tag::generated(EntityKind::Edge, op, 0)),
        Some((FeatureId(10), op, EntityKind::Edge, 0))
    );
    assert_eq!(
        table.resolve(&Tag::generated(EntityKind::Vertex, op, 0)),
        Some((FeatureId(10), op, EntityKind::Vertex, 0))
    );
}

#[test]
fn register_model_feature_skips_already_known_tags() {
    let (model, _) = quad_model(1.0, 1.0, 0.0);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &model);
    let len = table.len();
    register_all(&mut table, FeatureId(2), &model);
    assert_eq!(table.len(), len);
}

#[test]
fn rebind_after_recompute_keeps_face_axis_for_moved_face() {
    let (old_model, _old_op) = quad_model(1.0, 1.0, 0.0);
    let (new_model, _new_op) = quad_model(1.0, 1.0, 2.0);
    let old_face_tag = first_face_tag(&old_model);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &old_model);
    table.rebind_after_recompute(&old_model, &new_model);
    assert_eq!(
        table.resolve(&old_face_tag),
        Some((FeatureId(1), OperationId(1), EntityKind::Face, 0))
    );
}

#[test]
fn rebind_model_after_recompute_updates_face_name_map() {
    let (old_model, _) = quad_model(1.0, 1.0, 0.0);
    let (mut new_model, _) = quad_model(2.0, 1.0, 0.0);
    let old_face_tag = first_face_tag(&old_model);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &old_model);
    table.rebind_model_after_recompute(&old_model, &mut new_model);
    assert_eq!(
        new_model.name_map.get_face(&old_face_tag),
        Some(first_face(&new_model))
    );
}

#[test]
fn rebind_model_after_recompute_updates_edge_name_map() {
    let (old_model, _) = quad_model(1.0, 1.0, 0.0);
    let (mut new_model, _) = quad_model(3.0, 1.0, 0.0);
    let old_edge_tag = first_edge_tag(&old_model);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &old_model);
    table.rebind_model_after_recompute(&old_model, &mut new_model);
    assert_eq!(
        new_model.name_map.get_edge(&old_edge_tag),
        Some(first_edge(&new_model))
    );
}

#[test]
fn rebind_model_after_recompute_updates_vertex_name_map() {
    let (old_model, _) = quad_model(1.0, 1.0, 0.0);
    let (mut new_model, _) = quad_model(1.0, 1.0, 1.0);
    let old_vertex_tag = first_vertex_tag(&old_model);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &old_model);
    table.rebind_model_after_recompute(&old_model, &mut new_model);
    assert_eq!(
        new_model.name_map.get_vertex(&old_vertex_tag),
        Some(first_vertex(&new_model))
    );
}

#[test]
fn rebind_prunes_old_entries_that_have_no_candidate() {
    let (old_model, _) = quad_model(1.0, 1.0, 0.0);
    let (mut new_model, _) = triangle_model();
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &old_model);
    table.rebind_model_after_recompute(&old_model, &mut new_model);
    let vertex_entries = table
        .entries()
        .into_iter()
        .filter(|(key, _)| key.2 == EntityKind::Vertex)
        .count();
    assert_eq!(vertex_entries, new_model.vertices.len());
}

#[test]
fn unmatched_new_entities_keep_generated_tags() {
    let (old_model, _) = triangle_model();
    let (mut new_model, op) = quad_model(1.0, 1.0, 0.0);
    let extra_vertex_tag = Tag::generated(EntityKind::Vertex, op, 3);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &old_model);
    register_all(&mut table, FeatureId(1), &new_model);
    table.rebind_model_after_recompute(&old_model, &mut new_model);
    assert!(new_model.name_map.get_vertex(&extra_vertex_tag).is_some());
}

#[test]
fn serde_roundtrip_preserves_table_entries() {
    let (model, _) = quad_model(1.0, 1.0, 0.0);
    let mut table = PersistentNameTable::new();
    register_all(&mut table, FeatureId(1), &model);
    let json = serde_json::to_string(&table).expect("serialize");
    let back: PersistentNameTable = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, table);
}

#[test]
fn entries_are_returned_in_stable_order() {
    let mut table = PersistentNameTable::new();
    table.register(FeatureId(2), OperationId(3), EntityKind::Face, 1);
    table.register(FeatureId(1), OperationId(5), EntityKind::Vertex, 7);
    table.register(FeatureId(1), OperationId(1), EntityKind::Edge, 0);
    let keys: Vec<_> = table.entries().into_iter().map(|(key, _)| key).collect();
    assert_eq!(
        keys,
        vec![
            (FeatureId(1), OperationId(1), EntityKind::Edge, 0),
            (FeatureId(1), OperationId(5), EntityKind::Vertex, 7),
            (FeatureId(2), OperationId(3), EntityKind::Face, 1),
        ]
    );
}
