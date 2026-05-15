use std::collections::HashSet;

use cadkernel_api::{FaceRef, SolidId};
use cadkernel_math::Point3;
use cadkernel_modeling::{make_prism, make_tube, make_wedge};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle};
use cadkernel_viewer::scene::{ObjectId, Scene};

fn add_tube(scene: &mut Scene, solid_id: Option<SolidId>) -> (ObjectId, Vec<Handle<FaceData>>) {
    let mut model = BRepModel::new();
    let r = make_tube(&mut model, Point3::ORIGIN, 2.0, 1.0, 4.0, 12).expect("tube");
    let faces = r.faces.clone();
    let id = scene.add_object("Tube", model, r.solid, None, solid_id);
    (id, faces)
}

fn add_wedge(scene: &mut Scene, solid_id: Option<SolidId>) -> (ObjectId, Vec<Handle<FaceData>>) {
    let mut model = BRepModel::new();
    let r = make_wedge(
        &mut model,
        Point3::ORIGIN,
        4.0,
        3.0,
        2.0,
        2.0,
        1.5,
        0.5,
        0.25,
    )
    .expect("wedge");
    let faces = r.faces.clone();
    let id = scene.add_object("Wedge", model, r.solid, None, solid_id);
    (id, faces)
}

fn add_prism(scene: &mut Scene, solid_id: Option<SolidId>) -> (ObjectId, Vec<Handle<FaceData>>) {
    let mut model = BRepModel::new();
    let r = make_prism(&mut model, Point3::ORIGIN, 2.0, 3.0, 6).expect("prism");
    let faces = r.faces.clone();
    let id = scene.add_object("Prism", model, r.solid, None, solid_id);
    (id, faces)
}

fn face_tag(scene: &Scene, id: ObjectId, face: Handle<FaceData>) -> cadkernel_topology::Tag {
    scene
        .get(id)
        .expect("object")
        .model
        .faces
        .get(face)
        .and_then(|f| f.tag.clone())
        .expect("face tag")
}

fn assert_face_ref(scene: &Scene, id: ObjectId, face_ref: &FaceRef, expected: Handle<FaceData>) {
    let obj = scene.get(id).expect("object");
    assert_eq!(face_ref.solid, obj.solid_id.expect("solid id"));
    assert_eq!(face_ref.tag.kind, EntityKind::Face);
    assert_eq!(obj.model.name_map.get_face(&face_ref.tag), Some(expected));
}

fn scene_with_untagged_face() -> (Scene, ObjectId, Handle<FaceData>) {
    let mut model = BRepModel::new();
    let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
    let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
    let v2 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));
    let (_, he0, _) = model.add_edge(v0, v1);
    let (_, he1, _) = model.add_edge(v1, v2);
    let (_, he2, _) = model.add_edge(v2, v0);
    let loop_h = model.make_loop(&[he0, he1, he2]).expect("loop");
    let face_h = model.make_face(loop_h);
    let shell = model.make_shell(&[face_h]);
    let solid = model.make_solid(&[shell]);
    let mut scene = Scene::new();
    let id = scene.add_object("Untagged", model, solid, None, Some(SolidId(70)));
    (scene, id, face_h)
}

#[test]
fn tube_outer_wall_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_tube(&mut scene, Some(SolidId(71)));

    let refs = scene.selected_faces(id, &[faces[0]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[0]);
}

#[test]
fn tube_inner_wall_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_tube(&mut scene, Some(SolidId(72)));

    let refs = scene.selected_faces(id, &[faces[12]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[12]);
}

#[test]
fn tube_ring_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_tube(&mut scene, Some(SolidId(73)));
    let face_h = *faces.last().expect("ring face");

    let refs = scene.selected_faces(id, &[face_h]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], face_h);
}

#[test]
fn tube_all_face_tags_are_distinct() {
    let mut scene = Scene::new();
    let (id, faces) = add_tube(&mut scene, Some(SolidId(74)));

    let refs = scene.selected_faces(id, &faces);
    let unique: HashSet<_> = refs.iter().map(|r| r.tag.clone()).collect();

    assert_eq!(unique.len(), refs.len());
}

#[test]
fn wedge_slanted_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_wedge(&mut scene, Some(SolidId(75)));
    let face_h = *faces.last().expect("slanted face");

    let refs = scene.selected_faces(id, &[face_h]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], face_h);
}

#[test]
fn wedge_all_faces_round_trip() {
    let mut scene = Scene::new();
    let (id, faces) = add_wedge(&mut scene, Some(SolidId(76)));

    let refs = scene.selected_faces(id, &faces);

    assert_eq!(refs.len(), faces.len());
    for (face_ref, face_h) in refs.iter().zip(faces) {
        assert_face_ref(&scene, id, face_ref, face_h);
    }
}

#[test]
fn prism_side_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_prism(&mut scene, Some(SolidId(77)));

    let refs = scene.selected_faces(id, &[faces[2]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[2]);
}

#[test]
fn prism_face_order_is_preserved() {
    let mut scene = Scene::new();
    let (id, faces) = add_prism(&mut scene, Some(SolidId(78)));
    let selected = [faces[4], faces[1], faces[3]];

    let refs = scene.selected_faces(id, &selected);

    let actual: Vec<_> = refs.iter().map(|r| r.tag.clone()).collect();
    let expected: Vec<_> = selected
        .iter()
        .map(|&face_h| face_tag(&scene, id, face_h))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn draft_empty_face_selection_returns_empty() {
    let mut scene = Scene::new();
    let (id, _) = add_prism(&mut scene, Some(SolidId(79)));

    assert!(scene.selected_faces(id, &[]).is_empty());
}

#[test]
fn selected_faces_any_object_prefers_selected_wedge() {
    let mut scene = Scene::new();
    let (_tube, _) = add_tube(&mut scene, Some(SolidId(80)));
    let (wedge, faces) = add_wedge(&mut scene, Some(SolidId(81)));
    scene.select_single(wedge);

    let refs = scene.selected_faces_any_object(&[faces[0]]);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].solid, SolidId(81));
}

#[test]
fn tube_without_solid_id_returns_empty() {
    let mut scene = Scene::new();
    let (id, faces) = add_tube(&mut scene, None);

    assert!(scene.selected_faces(id, &[faces[0]]).is_empty());
}

#[test]
fn draft_untagged_face_is_skipped() {
    let (scene, id, face_h) = scene_with_untagged_face();

    assert!(scene.selected_faces(id, &[face_h]).is_empty());
}

#[test]
fn repeated_prism_face_selection_is_not_deduplicated() {
    let mut scene = Scene::new();
    let (id, faces) = add_prism(&mut scene, Some(SolidId(82)));

    let refs = scene.selected_faces(id, &[faces[0], faces[0], faces[0]]);

    assert_eq!(refs.len(), 3);
    assert!(refs.iter().all(|face_ref| face_ref == &refs[0]));
}

#[test]
fn any_object_skips_untracked_tube_before_tracked_prism() {
    let mut scene = Scene::new();
    let (_tube, _) = add_tube(&mut scene, None);
    let (prism, faces) = add_prism(&mut scene, Some(SolidId(83)));

    let refs = scene.selected_faces_any_object(&[faces[0]]);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].solid, SolidId(83));
    assert_face_ref(&scene, prism, &refs[0], faces[0]);
}
