use std::collections::HashSet;

use cadkernel_api::{EdgeRef, SolidId};
use cadkernel_math::Point3;
use cadkernel_modeling::{make_cylinder, make_torus};
use cadkernel_topology::{BRepModel, EdgeData, EntityKind, Handle};
use cadkernel_viewer::scene::{ObjectId, Scene};

fn add_cylinder(scene: &mut Scene, solid_id: Option<SolidId>) -> ObjectId {
    let mut model = BRepModel::new();
    let r = make_cylinder(&mut model, Point3::ORIGIN, 1.5, 5.0, 16).expect("cylinder");
    scene.add_object("Cylinder", model, r.solid, None, solid_id)
}

fn add_torus(scene: &mut Scene, solid_id: Option<SolidId>) -> ObjectId {
    let mut model = BRepModel::new();
    let r = make_torus(&mut model, Point3::ORIGIN, 5.0, 0.8, 12, 8).expect("torus");
    scene.add_object("Torus", model, r.solid, None, solid_id)
}

fn edge(scene: &Scene, id: ObjectId, index: usize) -> Handle<EdgeData> {
    scene.get(id).expect("object").edge_handles[index]
}

fn edge_tag(scene: &Scene, id: ObjectId, edge: Handle<EdgeData>) -> cadkernel_topology::Tag {
    scene
        .get(id)
        .expect("object")
        .model
        .edges
        .get(edge)
        .and_then(|e| e.tag.clone())
        .expect("edge tag")
}

fn assert_edge_ref(scene: &Scene, id: ObjectId, edge_ref: &EdgeRef, expected: Handle<EdgeData>) {
    let obj = scene.get(id).expect("object");
    assert_eq!(edge_ref.solid, obj.solid_id.expect("solid id"));
    assert_eq!(edge_ref.tag.kind, EntityKind::Edge);
    assert_eq!(obj.model.name_map.get_edge(&edge_ref.tag), Some(expected));
}

fn scene_with_cylinder_and_loose_edge() -> (Scene, ObjectId, Handle<EdgeData>) {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let r = make_cylinder(&mut model, Point3::ORIGIN, 1.0, 2.0, 8).expect("cylinder");
    let v0 = model.add_vertex(Point3::new(10.0, 0.0, 0.0));
    let v1 = model.add_vertex(Point3::new(11.0, 0.0, 0.0));
    let (untagged, _, _) = model.add_edge(v0, v1);
    let id = scene.add_object("Cylinder", model, r.solid, None, Some(SolidId(30)));
    (scene, id, untagged)
}

#[test]
fn cylinder_bottom_ring_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(31)));
    let edge_h = edge(&scene, id, 0);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn cylinder_top_ring_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(32)));
    let edge_h = edge(&scene, id, 16);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn cylinder_side_seam_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(33)));
    let edge_h = edge(&scene, id, 32);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn cylinder_multi_edge_selection_uses_distinct_tags() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(34)));
    let selected = [
        edge(&scene, id, 0),
        edge(&scene, id, 16),
        edge(&scene, id, 32),
    ];

    let refs = scene.selected_edges(id, &selected);
    let unique: HashSet<_> = refs.iter().map(|r| r.tag.clone()).collect();

    assert_eq!(refs.len(), 3);
    assert_eq!(unique.len(), 3);
}

#[test]
fn cylinder_empty_selection_returns_empty() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(35)));

    assert!(scene.selected_edges(id, &[]).is_empty());
}

#[test]
fn cylinder_without_solid_id_returns_empty() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, None);

    assert!(scene.selected_edges(id, &[edge(&scene, id, 0)]).is_empty());
}

#[test]
fn cylinder_untagged_edge_is_skipped() {
    let (scene, id, untagged) = scene_with_cylinder_and_loose_edge();

    assert!(scene.selected_edges(id, &[untagged]).is_empty());
}

#[test]
fn torus_first_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_torus(&mut scene, Some(SolidId(36)));
    let edge_h = edge(&scene, id, 0);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn torus_major_ring_edges_round_trip() {
    let mut scene = Scene::new();
    let id = add_torus(&mut scene, Some(SolidId(37)));
    let selected = [
        edge(&scene, id, 0),
        edge(&scene, id, 8),
        edge(&scene, id, 16),
    ];

    let refs = scene.selected_edges(id, &selected);

    assert_eq!(refs.len(), selected.len());
    for (edge_ref, edge_h) in refs.iter().zip(selected) {
        assert_edge_ref(&scene, id, edge_ref, edge_h);
    }
}

#[test]
fn torus_repeated_edge_selection_is_preserved() {
    let mut scene = Scene::new();
    let id = add_torus(&mut scene, Some(SolidId(38)));
    let edge_h = edge(&scene, id, 5);

    let refs = scene.selected_edges(id, &[edge_h, edge_h, edge_h]);

    assert_eq!(refs.len(), 3);
    assert!(refs.iter().all(|edge_ref| edge_ref == &refs[0]));
}

#[test]
fn torus_any_object_uses_selected_scene_object() {
    let mut scene = Scene::new();
    let cylinder = add_cylinder(&mut scene, Some(SolidId(39)));
    let torus = add_torus(&mut scene, Some(SolidId(40)));
    scene.select_single(torus);
    let selected = [edge(&scene, torus, 3)];

    let refs = scene.selected_edges_any_object(&selected);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].solid, SolidId(40));
    assert_ne!(
        refs[0].solid,
        scene
            .get(cylinder)
            .expect("cylinder")
            .solid_id
            .expect("solid id")
    );
}

#[test]
fn any_object_preserves_torus_input_order() {
    let mut scene = Scene::new();
    let id = add_torus(&mut scene, Some(SolidId(41)));
    scene.select_single(id);
    let selected = [
        edge(&scene, id, 7),
        edge(&scene, id, 2),
        edge(&scene, id, 11),
    ];

    let refs = scene.selected_edges_any_object(&selected);

    let actual: Vec<_> = refs.iter().map(|r| r.tag.clone()).collect();
    let expected: Vec<_> = selected
        .iter()
        .map(|&edge_h| edge_tag(&scene, id, edge_h))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn invalid_cylinder_edge_handle_is_ignored() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(42)));
    let fake = Handle::<EdgeData>::from_raw_parts(999, 0);

    assert!(scene.selected_edges(id, &[fake]).is_empty());
}

#[test]
fn any_object_skips_untracked_cylinder_before_tracked_torus() {
    let mut scene = Scene::new();
    let _cylinder = add_cylinder(&mut scene, None);
    let torus = add_torus(&mut scene, Some(SolidId(44)));
    let torus_edge = edge(&scene, torus, 4);

    let refs = scene.selected_edges_any_object(&[torus_edge]);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].solid, SolidId(44));
    assert_edge_ref(&scene, torus, &refs[0], torus_edge);
}
