use std::collections::HashSet;

use cadkernel_api::{Command, EdgeRef, Outcome, Session, SolidId};
use cadkernel_math::Point3;
use cadkernel_modeling::{make_box, make_cylinder, make_sphere, make_torus};
use cadkernel_topology::{BRepModel, EdgeData, EntityKind, Handle, SolidData};
use cadkernel_viewer::scene::{ObjectId, Scene};

fn add_box(scene: &mut Scene, solid_id: Option<SolidId>) -> ObjectId {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).expect("box");
    scene.add_object("Box", model, r.solid, None, solid_id)
}

fn add_cylinder(scene: &mut Scene, solid_id: Option<SolidId>) -> ObjectId {
    let mut model = BRepModel::new();
    let r = make_cylinder(&mut model, Point3::ORIGIN, 2.0, 4.0, 16).expect("cylinder");
    scene.add_object("Cylinder", model, r.solid, None, solid_id)
}

fn add_sphere(scene: &mut Scene, solid_id: Option<SolidId>) -> ObjectId {
    let mut model = BRepModel::new();
    let r = make_sphere(&mut model, Point3::ORIGIN, 2.0, 12, 6).expect("sphere");
    scene.add_object("Sphere", model, r.solid, None, solid_id)
}

fn add_torus(scene: &mut Scene, solid_id: Option<SolidId>) -> ObjectId {
    let mut model = BRepModel::new();
    let r = make_torus(&mut model, Point3::ORIGIN, 4.0, 1.0, 10, 6).expect("torus");
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
    assert!(!edge_ref.tag.segments.is_empty());
    assert_eq!(obj.model.name_map.get_edge(&edge_ref.tag), Some(expected));
}

fn scene_with_loose_edge() -> (Scene, ObjectId, Handle<EdgeData>, Handle<EdgeData>) {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box");
    let tagged = model.edges.iter().next().expect("tagged edge").0;
    let v0 = model.add_vertex(Point3::new(10.0, 0.0, 0.0));
    let v1 = model.add_vertex(Point3::new(11.0, 0.0, 0.0));
    let (untagged, _, _) = model.add_edge(v0, v1);
    let id = scene.add_object("Mixed", model, r.solid, None, Some(SolidId(9)));
    (scene, id, tagged, untagged)
}

#[test]
fn box_single_edge_round_trips_through_name_map() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(1)));
    let selected = [edge(&scene, id, 0)];

    let refs = scene.selected_edges(id, &selected);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], selected[0]);
}

#[test]
fn box_edge_ref_uses_object_solid_id() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(42)));

    let refs = scene.selected_edges(id, &[edge(&scene, id, 1)]);

    assert_eq!(refs[0].solid, SolidId(42));
}

#[test]
fn box_all_edges_round_trip() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(2)));
    let edges = scene.get(id).expect("object").edge_handles.clone();

    let refs = scene.selected_edges(id, &edges);

    assert_eq!(refs.len(), 12);
    for (edge_ref, edge_h) in refs.iter().zip(edges) {
        assert_edge_ref(&scene, id, edge_ref, edge_h);
    }
}

#[test]
fn box_all_edge_tags_are_distinct() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(3)));
    let edges = scene.get(id).expect("object").edge_handles.clone();

    let refs = scene.selected_edges(id, &edges);
    let unique: HashSet<_> = refs.iter().map(|r| r.tag.clone()).collect();

    assert_eq!(unique.len(), refs.len());
}

#[test]
fn empty_edge_selection_returns_empty() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(4)));

    assert!(scene.selected_edges(id, &[]).is_empty());
}

#[test]
fn untagged_edge_is_skipped() {
    let (scene, id, _, untagged) = scene_with_loose_edge();

    let refs = scene.selected_edges(id, &[untagged]);

    assert!(refs.is_empty());
}

#[test]
fn no_solid_id_object_drops_tagged_edge() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, None);

    let refs = scene.selected_edges(id, &[edge(&scene, id, 0)]);

    assert!(refs.is_empty());
}

#[test]
fn selected_edges_any_object_prefers_selected_object() {
    let mut scene = Scene::new();
    let first = add_box(&mut scene, Some(SolidId(10)));
    let second = add_box(&mut scene, Some(SolidId(11)));
    scene.select_single(second);
    let selected = [edge(&scene, second, 0)];

    let refs = scene.selected_edges_any_object(&selected);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].solid, SolidId(11));
    assert_edge_ref(&scene, second, &refs[0], selected[0]);
    assert_ne!(
        refs[0].solid,
        scene.get(first).expect("first").solid_id.expect("solid id")
    );
}

#[test]
fn selected_edges_preserves_input_order() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(12)));
    let selected = [
        edge(&scene, id, 3),
        edge(&scene, id, 1),
        edge(&scene, id, 2),
    ];

    let refs = scene.selected_edges(id, &selected);

    let expected: Vec<_> = selected
        .iter()
        .map(|&edge_h| edge_tag(&scene, id, edge_h))
        .collect();
    let actual: Vec<_> = refs.iter().map(|r| r.tag.clone()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn mixed_tagged_and_untagged_edges_return_only_tagged_refs() {
    let (scene, id, tagged, untagged) = scene_with_loose_edge();

    let refs = scene.selected_edges(id, &[tagged, untagged]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], tagged);
}

#[test]
fn stale_edge_handle_is_ignored_by_single_object_lookup() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(13)));
    let mut other = BRepModel::new();
    let _ = make_cylinder(&mut other, Point3::ORIGIN, 2.0, 4.0, 16).expect("cylinder");
    let stale = other.edges.iter().nth(20).expect("stale edge").0;

    let refs = scene.selected_edges(id, &[stale]);

    assert!(refs.is_empty());
}

#[test]
fn repeated_edge_selection_is_not_deduplicated() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(14)));
    let edge_h = edge(&scene, id, 0);

    let refs = scene.selected_edges(id, &[edge_h, edge_h]);

    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0], refs[1]);
}

#[test]
fn invalid_object_id_returns_empty_edges() {
    let scene = Scene::new();
    let fake = Handle::<EdgeData>::from_raw_parts(0, 0);

    assert!(scene.selected_edges(999, &[fake]).is_empty());
}

#[test]
fn invalid_edge_handle_returns_empty() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, Some(SolidId(15)));
    let fake = Handle::<EdgeData>::from_raw_parts(99, 0);

    assert!(scene.selected_edges(id, &[fake]).is_empty());
}

#[test]
fn cylinder_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_cylinder(&mut scene, Some(SolidId(16)));
    let edge_h = edge(&scene, id, 0);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn sphere_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_sphere(&mut scene, Some(SolidId(17)));
    let edge_h = edge(&scene, id, 4);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn torus_edge_round_trips() {
    let mut scene = Scene::new();
    let id = add_torus(&mut scene, Some(SolidId(18)));
    let edge_h = edge(&scene, id, 7);

    let refs = scene.selected_edges(id, &[edge_h]);

    assert_eq!(refs.len(), 1);
    assert_edge_ref(&scene, id, &refs[0], edge_h);
}

#[test]
fn any_object_preserves_input_order_for_selected_object() {
    let mut scene = Scene::new();
    let id = add_torus(&mut scene, Some(SolidId(19)));
    scene.select_single(id);
    let selected = [
        edge(&scene, id, 8),
        edge(&scene, id, 3),
        edge(&scene, id, 5),
    ];

    let refs = scene.selected_edges_any_object(&selected);

    let expected: Vec<_> = selected
        .iter()
        .map(|&edge_h| edge_tag(&scene, id, edge_h))
        .collect();
    let actual: Vec<_> = refs.iter().map(|r| r.tag.clone()).collect();
    assert_eq!(actual, expected);
}

#[test]
fn any_object_ignores_objects_without_solid_id() {
    let mut scene = Scene::new();
    let legacy = add_cylinder(&mut scene, None);
    let tracked = add_box(&mut scene, Some(SolidId(20)));
    scene.select_single(tracked);
    let selected = [edge(&scene, legacy, 20), edge(&scene, tracked, 1)];

    let refs = scene.selected_edges_any_object(&selected);

    assert_eq!(refs.len(), 1);
    assert!(refs.iter().all(|edge_ref| edge_ref.solid == SolidId(20)));
}

#[test]
fn loose_untagged_edge_can_live_in_invalid_solid_object_without_panic() {
    let mut model = BRepModel::new();
    let v0 = model.add_vertex(Point3::ORIGIN);
    let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let (edge_h, _, _) = model.add_edge(v0, v1);
    let mut scene = Scene::new();
    let id = scene.add_object(
        "Loose",
        model,
        Handle::<SolidData>::from_raw_parts(99, 0),
        None,
        Some(SolidId(21)),
    );

    let refs = scene.selected_edges(id, &[edge_h]);

    assert!(refs.is_empty());
}

#[test]
fn session_execute_fillet_resolves_selected_edge_ref() {
    let mut session = Session::new();
    let solid_id = match session
        .execute(Command::CreateBox {
            dx: 4.0,
            dy: 4.0,
            dz: 4.0,
        })
        .expect("box")
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("unexpected outcome: {other:?}"),
    };
    let (model, solid) = session
        .document()
        .clone_solid_brep(solid_id)
        .expect("solid");
    let base_face_count = model.faces.iter().count();
    let mut scene = Scene::new();
    let object_id = scene.add_object("Box", model, solid, None, Some(solid_id));
    let edge_h = edge(&scene, object_id, 0);
    let refs = scene.selected_edges(object_id, &[edge_h]);

    let outcome = session
        .execute(Command::Fillet {
            edges: refs,
            radius: 0.1,
            variable: None,
        })
        .expect("fillet");

    assert!(matches!(outcome, Outcome::FeatureAdded { solid, .. } if solid == solid_id));
    let (model, _) = session
        .document()
        .clone_solid_brep(solid_id)
        .expect("filleted solid");
    assert!(model.faces.iter().count() > base_face_count);
}
