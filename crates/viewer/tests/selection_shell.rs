use std::collections::HashSet;

use cadkernel_api::{FaceRef, SolidId};
use cadkernel_math::Point3;
use cadkernel_modeling::{make_box, make_cylinder};
use cadkernel_topology::{BRepModel, EntityKind, FaceData, Handle};
use cadkernel_viewer::scene::{ObjectId, Scene};

fn add_box(scene: &mut Scene, solid_id: Option<SolidId>) -> (ObjectId, Vec<Handle<FaceData>>) {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).expect("box");
    let faces = r.faces.clone();
    let id = scene.add_object("Box", model, r.solid, None, solid_id);
    (id, faces)
}

fn add_cylinder(scene: &mut Scene, solid_id: Option<SolidId>) -> (ObjectId, Vec<Handle<FaceData>>) {
    let mut model = BRepModel::new();
    let r = make_cylinder(&mut model, Point3::ORIGIN, 1.5, 4.0, 12).expect("cylinder");
    let mut faces = vec![r.bottom_face, r.top_face];
    faces.extend(r.lateral_faces.iter().copied());
    let id = scene.add_object("Cylinder", model, r.solid, None, solid_id);
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
    assert!(!face_ref.tag.segments.is_empty());
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
    let id = scene.add_object("UntaggedFace", model, solid, None, Some(SolidId(50)));
    (scene, id, face_h)
}

fn scene_with_box_and_untagged_face() -> (Scene, ObjectId, Handle<FaceData>, Handle<FaceData>) {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box");
    let tagged = r.faces[0];
    let v0 = model.add_vertex(Point3::new(10.0, 0.0, 0.0));
    let v1 = model.add_vertex(Point3::new(11.0, 0.0, 0.0));
    let v2 = model.add_vertex(Point3::new(10.0, 1.0, 0.0));
    let (_, he0, _) = model.add_edge(v0, v1);
    let (_, he1, _) = model.add_edge(v1, v2);
    let (_, he2, _) = model.add_edge(v2, v0);
    let loop_h = model.make_loop(&[he0, he1, he2]).expect("loop");
    let untagged = model.make_face(loop_h);
    let mut scene = Scene::new();
    let id = scene.add_object("MixedFaces", model, r.solid, None, Some(SolidId(51)));
    (scene, id, tagged, untagged)
}

#[test]
fn box_single_face_round_trips_through_name_map() {
    let mut scene = Scene::new();
    let (id, faces) = add_box(&mut scene, Some(SolidId(52)));

    let refs = scene.selected_faces(id, &[faces[0]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[0]);
}

#[test]
fn box_all_faces_round_trip() {
    let mut scene = Scene::new();
    let (id, faces) = add_box(&mut scene, Some(SolidId(53)));

    let refs = scene.selected_faces(id, &faces);

    assert_eq!(refs.len(), 6);
    for (face_ref, face_h) in refs.iter().zip(faces) {
        assert_face_ref(&scene, id, face_ref, face_h);
    }
}

#[test]
fn box_face_tags_are_distinct() {
    let mut scene = Scene::new();
    let (id, faces) = add_box(&mut scene, Some(SolidId(54)));

    let refs = scene.selected_faces(id, &faces);
    let unique: HashSet<_> = refs.iter().map(|r| r.tag.clone()).collect();

    assert_eq!(unique.len(), refs.len());
}

#[test]
fn empty_face_selection_returns_empty() {
    let mut scene = Scene::new();
    let (id, _) = add_box(&mut scene, Some(SolidId(55)));

    assert!(scene.selected_faces(id, &[]).is_empty());
}

#[test]
fn untagged_face_is_skipped() {
    let (scene, id, face_h) = scene_with_untagged_face();

    assert!(scene.selected_faces(id, &[face_h]).is_empty());
}

#[test]
fn no_solid_id_object_drops_tagged_face() {
    let mut scene = Scene::new();
    let (id, faces) = add_box(&mut scene, None);

    assert!(scene.selected_faces(id, &[faces[0]]).is_empty());
}

#[test]
fn selected_faces_any_object_prefers_selected_object() {
    let mut scene = Scene::new();
    let (_first, _) = add_box(&mut scene, Some(SolidId(56)));
    let (second, faces) = add_box(&mut scene, Some(SolidId(57)));
    scene.select_single(second);

    let refs = scene.selected_faces_any_object(&[faces[0]]);

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].solid, SolidId(57));
    assert_face_ref(&scene, second, &refs[0], faces[0]);
}

#[test]
fn selected_faces_preserves_input_order() {
    let mut scene = Scene::new();
    let (id, faces) = add_box(&mut scene, Some(SolidId(58)));
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
fn repeated_face_selection_is_not_deduplicated() {
    let mut scene = Scene::new();
    let (id, faces) = add_box(&mut scene, Some(SolidId(59)));

    let refs = scene.selected_faces(id, &[faces[0], faces[0]]);

    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0], refs[1]);
}

#[test]
fn invalid_object_id_returns_empty_faces() {
    let scene = Scene::new();
    let fake = Handle::<FaceData>::from_raw_parts(0, 0);

    assert!(scene.selected_faces(999, &[fake]).is_empty());
}

#[test]
fn stale_face_handle_is_ignored_by_single_object_lookup() {
    let mut scene = Scene::new();
    let (id, _) = add_box(&mut scene, Some(SolidId(60)));
    let mut other = BRepModel::new();
    let r = make_cylinder(&mut other, Point3::ORIGIN, 1.0, 2.0, 12).expect("cylinder");
    let stale = r.lateral_faces[8];

    assert!(scene.selected_faces(id, &[stale]).is_empty());
}

#[test]
fn cylinder_bottom_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_cylinder(&mut scene, Some(SolidId(61)));

    let refs = scene.selected_faces(id, &[faces[0]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[0]);
}

#[test]
fn cylinder_top_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_cylinder(&mut scene, Some(SolidId(62)));

    let refs = scene.selected_faces(id, &[faces[1]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[1]);
}

#[test]
fn cylinder_lateral_face_round_trips() {
    let mut scene = Scene::new();
    let (id, faces) = add_cylinder(&mut scene, Some(SolidId(63)));

    let refs = scene.selected_faces(id, &[faces[2]]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], faces[2]);
}

#[test]
fn cylinder_caps_and_lateral_faces_round_trip_together() {
    let mut scene = Scene::new();
    let (id, faces) = add_cylinder(&mut scene, Some(SolidId(64)));
    let selected = [faces[0], faces[1], faces[5]];

    let refs = scene.selected_faces(id, &selected);

    assert_eq!(refs.len(), selected.len());
    for (face_ref, face_h) in refs.iter().zip(selected) {
        assert_face_ref(&scene, id, face_ref, face_h);
    }
}

#[test]
fn any_object_preserves_face_input_order_for_selected_object() {
    let mut scene = Scene::new();
    let (id, faces) = add_cylinder(&mut scene, Some(SolidId(65)));
    scene.select_single(id);
    let selected = [faces[4], faces[0], faces[3]];

    let refs = scene.selected_faces_any_object(&selected);

    let actual: Vec<_> = refs.iter().map(|r| r.tag.clone()).collect();
    let expected: Vec<_> = selected
        .iter()
        .map(|&face_h| face_tag(&scene, id, face_h))
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn mixed_tagged_and_untagged_faces_return_only_tagged_refs() {
    let (scene, id, tagged, untagged) = scene_with_box_and_untagged_face();

    let refs = scene.selected_faces(id, &[tagged, untagged]);

    assert_eq!(refs.len(), 1);
    assert_face_ref(&scene, id, &refs[0], tagged);
}
