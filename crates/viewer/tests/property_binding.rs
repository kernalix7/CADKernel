//! Gate #7 — property panel two-way binding for primitive parameters.
//!
//! When the user edits a numeric parameter in the Properties panel
//! (`gui/properties.rs::draw_params_editor`), the panel emits
//! `GuiAction::RebuildObject { id, params: CreationParams::* }`. The
//! dispatcher (`app.rs::process_actions`) rebuilds the BRep model and the
//! tessellated mesh, replacing the existing SceneObject's `model`, `solid`,
//! `mesh`, `vertices` and `params` fields. The Properties panel then reads
//! the new `obj.params` to display the value back to the user — that
//! round-trip is what this test locks in.
//!
//! For each primitive (Box, Cylinder), we:
//!
//!   1. Create the object via the same `CreateBox`/`CreateCylinder`
//!      dispatcher arms the menu uses.
//!   2. Dispatch a `RebuildObject` with new parameters.
//!   3. Assert mesh-derived bbox extent matches the new dimensions.
//!   4. Assert BRep model vertices' bbox matches the new dimensions.
//!   5. Assert `obj.params` round-trips back to the same values (this is
//!      what the Properties panel re-reads to render the DragValue).

use cadkernel_topology::BRepModel;
use cadkernel_viewer::{
    scene::{CreationParams, SceneObject},
    test_support::CadApp,
};

/// Compute the BRep model's bounding-box extents on each axis.
fn model_extents(model: &BRepModel) -> [f64; 3] {
    let mut mn = [f64::INFINITY; 3];
    let mut mx = [f64::NEG_INFINITY; 3];
    for (_, v) in model.vertices.iter() {
        mn[0] = mn[0].min(v.point.x);
        mn[1] = mn[1].min(v.point.y);
        mn[2] = mn[2].min(v.point.z);
        mx[0] = mx[0].max(v.point.x);
        mx[1] = mx[1].max(v.point.y);
        mx[2] = mx[2].max(v.point.z);
    }
    [mx[0] - mn[0], mx[1] - mn[1], mx[2] - mn[2]]
}

fn mesh_extents(obj: &SceneObject) -> [f32; 3] {
    [
        obj.aabb_max[0] - obj.aabb_min[0],
        obj.aabb_max[1] - obj.aabb_min[1],
        obj.aabb_max[2] - obj.aabb_min[2],
    ]
}

// ---------------------------------------------------------------------------
// Box — dx / dy / dz round-trip.
// ---------------------------------------------------------------------------

#[test]
fn box_width_edit_persists_through_rebuild() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(100.0, 50.0, 25.0);
    let id = app.scene_ref().objects.first().unwrap().id;

    // Sanity: initial extents reflect the create-time dimensions.
    let initial = mesh_extents(app.scene_ref().objects.first().unwrap());
    assert!(
        (initial[0] - 100.0).abs() < 1e-2,
        "initial width should be ~100, got {}",
        initial[0]
    );

    // Edit width: 100 → 200.
    app.dispatch_rebuild_object(
        id,
        CreationParams::Box {
            width: 200.0,
            height: 50.0,
            depth: 25.0,
        },
    );

    let obj = app
        .scene_ref()
        .objects
        .first()
        .expect("object survives rebuild");

    let mesh_ext = mesh_extents(obj);
    assert!(
        (mesh_ext[0] - 200.0).abs() < 1e-2,
        "mesh X extent after width=200 rebuild: got {}",
        mesh_ext[0]
    );
    assert!(
        (mesh_ext[1] - 50.0).abs() < 1e-2,
        "mesh Y extent should stay 50, got {}",
        mesh_ext[1]
    );
    assert!(
        (mesh_ext[2] - 25.0).abs() < 1e-2,
        "mesh Z extent should stay 25, got {}",
        mesh_ext[2]
    );

    let model_ext = model_extents(&obj.model);
    assert!(
        (model_ext[0] - 200.0).abs() < 1e-6,
        "BRep model X extent should be 200, got {}",
        model_ext[0]
    );

    // Params round-trip: Properties panel re-reads these to display the value.
    match obj.params {
        Some(CreationParams::Box {
            width,
            height,
            depth,
        }) => {
            assert!((width - 200.0).abs() < 1e-9);
            assert!((height - 50.0).abs() < 1e-9);
            assert!((depth - 25.0).abs() < 1e-9);
        }
        ref other => panic!("expected Box params after rebuild, got {other:?}"),
    }
}

#[test]
fn box_height_edit_persists_through_rebuild() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(100.0, 50.0, 25.0);
    let id = app.scene_ref().objects.first().unwrap().id;

    app.dispatch_rebuild_object(
        id,
        CreationParams::Box {
            width: 100.0,
            height: 300.0,
            depth: 25.0,
        },
    );

    let obj = app.scene_ref().objects.first().unwrap();
    let mesh_ext = mesh_extents(obj);
    assert!(
        (mesh_ext[1] - 300.0).abs() < 1e-2,
        "mesh Y extent after height=300: {}",
        mesh_ext[1]
    );
    let model_ext = model_extents(&obj.model);
    assert!(
        (model_ext[1] - 300.0).abs() < 1e-6,
        "BRep model Y extent should be 300, got {}",
        model_ext[1]
    );
    match obj.params {
        Some(CreationParams::Box { height, .. }) => {
            assert!((height - 300.0).abs() < 1e-9);
        }
        ref other => panic!("expected Box params, got {other:?}"),
    }
}

#[test]
fn box_depth_edit_persists_through_rebuild() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(100.0, 50.0, 25.0);
    let id = app.scene_ref().objects.first().unwrap().id;

    app.dispatch_rebuild_object(
        id,
        CreationParams::Box {
            width: 100.0,
            height: 50.0,
            depth: 500.0,
        },
    );

    let obj = app.scene_ref().objects.first().unwrap();
    let mesh_ext = mesh_extents(obj);
    assert!(
        (mesh_ext[2] - 500.0).abs() < 1e-2,
        "mesh Z extent after depth=500: {}",
        mesh_ext[2]
    );
    let model_ext = model_extents(&obj.model);
    assert!(
        (model_ext[2] - 500.0).abs() < 1e-6,
        "BRep model Z extent should be 500, got {}",
        model_ext[2]
    );
    match obj.params {
        Some(CreationParams::Box { depth, .. }) => {
            assert!((depth - 500.0).abs() < 1e-9);
        }
        ref other => panic!("expected Box params, got {other:?}"),
    }
}

#[test]
fn box_multi_axis_edit_round_trips() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(10.0, 10.0, 10.0);
    let id = app.scene_ref().objects.first().unwrap().id;

    app.dispatch_rebuild_object(
        id,
        CreationParams::Box {
            width: 7.5,
            height: 12.25,
            depth: 33.0,
        },
    );

    let obj = app.scene_ref().objects.first().unwrap();
    let mesh_ext = mesh_extents(obj);
    assert!((mesh_ext[0] - 7.5).abs() < 1e-2);
    assert!((mesh_ext[1] - 12.25).abs() < 1e-2);
    assert!((mesh_ext[2] - 33.0).abs() < 1e-2);

    let model_ext = model_extents(&obj.model);
    assert!((model_ext[0] - 7.5).abs() < 1e-6);
    assert!((model_ext[1] - 12.25).abs() < 1e-6);
    assert!((model_ext[2] - 33.0).abs() < 1e-6);

    match obj.params {
        Some(CreationParams::Box {
            width,
            height,
            depth,
        }) => {
            assert!((width - 7.5).abs() < 1e-9);
            assert!((height - 12.25).abs() < 1e-9);
            assert!((depth - 33.0).abs() < 1e-9);
        }
        ref other => panic!("expected Box params, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Cylinder — radius / height round-trip.
// ---------------------------------------------------------------------------

#[test]
fn cylinder_radius_edit_persists_through_rebuild() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_cylinder(2.0, 5.0);
    let id = app.scene_ref().objects.first().unwrap().id;

    app.dispatch_rebuild_object(
        id,
        CreationParams::Cylinder {
            radius: 7.0,
            height: 5.0,
        },
    );

    let obj = app.scene_ref().objects.first().unwrap();
    let mesh_ext = mesh_extents(obj);
    // Cylinder is tessellated to a polygon; expect the inscribed-polygon
    // diameter to be a bit less than 2r. Allow ~2% slack.
    let diameter = 14.0_f32;
    assert!(
        (mesh_ext[0] - diameter).abs() < diameter * 0.02,
        "mesh X extent should approximate cylinder diameter 14.0, got {}",
        mesh_ext[0]
    );
    // Z extent should still match height.
    assert!(
        (mesh_ext[2] - 5.0).abs() < 1e-2,
        "mesh Z extent should stay 5.0, got {}",
        mesh_ext[2]
    );

    let model_ext = model_extents(&obj.model);
    // BRep vertices for a cylinder live on the polygon, so the same diameter
    // tolerance applies.
    assert!(
        (model_ext[0] - diameter as f64).abs() < (diameter as f64) * 0.02,
        "BRep X extent should approximate 14.0, got {}",
        model_ext[0]
    );

    match obj.params {
        Some(CreationParams::Cylinder { radius, height }) => {
            assert!((radius - 7.0).abs() < 1e-9);
            assert!((height - 5.0).abs() < 1e-9);
        }
        ref other => panic!("expected Cylinder params, got {other:?}"),
    }
}

#[test]
fn cylinder_height_edit_persists_through_rebuild() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_cylinder(2.0, 5.0);
    let id = app.scene_ref().objects.first().unwrap().id;

    app.dispatch_rebuild_object(
        id,
        CreationParams::Cylinder {
            radius: 2.0,
            height: 42.0,
        },
    );

    let obj = app.scene_ref().objects.first().unwrap();
    let mesh_ext = mesh_extents(obj);
    assert!(
        (mesh_ext[2] - 42.0).abs() < 1e-2,
        "cylinder mesh height should be 42, got {}",
        mesh_ext[2]
    );
    let model_ext = model_extents(&obj.model);
    assert!(
        (model_ext[2] - 42.0).abs() < 1e-6,
        "BRep Z extent should be 42, got {}",
        model_ext[2]
    );
    match obj.params {
        Some(CreationParams::Cylinder { radius, height }) => {
            assert!((radius - 2.0).abs() < 1e-9);
            assert!((height - 42.0).abs() < 1e-9);
        }
        ref other => panic!("expected Cylinder params, got {other:?}"),
    }
}

/// Rebuilding must preserve the object's identity (`id`) and slot — the
/// Properties panel binds to a specific `ObjectId` and would lose the edit
/// target if the dispatcher replaced/appended.
#[test]
fn rebuild_preserves_object_id_and_count() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let id_before = app.scene_ref().objects.first().unwrap().id;
    let count_before = app.scene_ref().objects.len();

    app.dispatch_rebuild_object(
        id_before,
        CreationParams::Box {
            width: 9.0,
            height: 9.0,
            depth: 9.0,
        },
    );

    assert_eq!(
        app.scene_ref().objects.len(),
        count_before,
        "rebuild must not change scene object count"
    );
    let id_after = app.scene_ref().objects.first().unwrap().id;
    assert_eq!(id_after, id_before, "rebuild must preserve ObjectId");
}
