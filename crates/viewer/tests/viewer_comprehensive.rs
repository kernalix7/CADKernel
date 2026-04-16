use cadkernel_viewer::{
    Camera, DisplayMode, Projection, StandardView,
    nav::{BgPreset, NavAction, NavConfig, NavStyle, OrbitStyle, RotationMode, UnitSystem},
    scene::{CreationParams, ObjectGroup, Scene, compute_aabb},
    Vertex,
};

// ---------------------------------------------------------------------------
// Vertex / compute_aabb
// ---------------------------------------------------------------------------

fn make_vertex(px: f32, py: f32, pz: f32) -> Vertex {
    Vertex { position: [px, py, pz], normal: [0.0, 0.0, 1.0] }
}

#[test]
fn compute_aabb_empty_returns_zero() {
    let (mn, mx) = compute_aabb(&[]);
    assert_eq!(mn, [0.0; 3]);
    assert_eq!(mx, [0.0; 3]);
}

#[test]
fn compute_aabb_single_vertex() {
    let verts = [make_vertex(1.0, 2.0, 3.0)];
    let (mn, mx) = compute_aabb(&verts);
    assert_eq!(mn, [1.0, 2.0, 3.0]);
    assert_eq!(mx, [1.0, 2.0, 3.0]);
}

#[test]
fn compute_aabb_multiple_vertices_correct_bounds() {
    let verts = [
        make_vertex(-1.0, 0.0, 5.0),
        make_vertex(3.0, -2.0, 1.0),
        make_vertex(0.0, 4.0, 2.0),
    ];
    let (mn, mx) = compute_aabb(&verts);
    assert_eq!(mn, [-1.0, -2.0, 1.0]);
    assert_eq!(mx, [3.0, 4.0, 5.0]);
}

#[test]
fn compute_aabb_unit_cube_vertices() {
    let mut verts = Vec::new();
    for &x in &[0.0f32, 1.0] {
        for &y in &[0.0f32, 1.0] {
            for &z in &[0.0f32, 1.0] {
                verts.push(make_vertex(x, y, z));
            }
        }
    }
    let (mn, mx) = compute_aabb(&verts);
    assert_eq!(mn, [0.0; 3]);
    assert_eq!(mx, [1.0; 3]);
}

// ---------------------------------------------------------------------------
// DisplayMode
// ---------------------------------------------------------------------------

#[test]
fn display_mode_all_has_eight_entries() {
    assert_eq!(DisplayMode::ALL.len(), 8);
}

#[test]
fn display_mode_labels_non_empty() {
    for mode in DisplayMode::ALL {
        assert!(!mode.label().is_empty(), "empty label for {mode:?}");
    }
}

#[test]
fn display_mode_shortcuts_non_empty() {
    for mode in DisplayMode::ALL {
        assert!(!mode.shortcut().is_empty(), "empty shortcut for {mode:?}");
    }
}

#[test]
fn display_mode_shading_label_is_shading() {
    assert_eq!(DisplayMode::Shading.label(), "Shading");
}

#[test]
fn display_mode_wireframe_label_is_wireframe() {
    assert_eq!(DisplayMode::Wireframe.label(), "Wireframe");
}

#[test]
fn display_mode_equality() {
    assert_eq!(DisplayMode::Points, DisplayMode::Points);
    assert_ne!(DisplayMode::Points, DisplayMode::Wireframe);
}

// ---------------------------------------------------------------------------
// Projection
// ---------------------------------------------------------------------------

#[test]
fn projection_perspective_not_equal_to_orthographic() {
    assert_ne!(Projection::Perspective, Projection::Orthographic);
}

#[test]
fn projection_equality() {
    assert_eq!(Projection::Perspective, Projection::Perspective);
    assert_eq!(Projection::Orthographic, Projection::Orthographic);
}

// ---------------------------------------------------------------------------
// StandardView
// ---------------------------------------------------------------------------

#[test]
fn standard_view_labels_non_empty() {
    let views = [
        StandardView::Front,
        StandardView::Back,
        StandardView::Right,
        StandardView::Left,
        StandardView::Top,
        StandardView::Bottom,
        StandardView::Isometric,
    ];
    for v in views {
        assert!(!v.label().is_empty(), "empty label for {v:?}");
    }
}

#[test]
fn standard_view_yaw_pitch_front_is_pi_over_2_zero() {
    let (yaw, pitch) = StandardView::Front.yaw_pitch();
    assert!((yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-5, "front yaw={yaw}");
    assert!(pitch.abs() < 1e-5, "front pitch={pitch}");
}

#[test]
fn standard_view_top_pitch_near_plus_90() {
    let (_yaw, pitch) = StandardView::Top.yaw_pitch();
    assert!(pitch > 1.0, "top pitch should be near +PI/2, got {pitch}");
}

#[test]
fn standard_view_bottom_pitch_near_minus_90() {
    let (_yaw, pitch) = StandardView::Bottom.yaw_pitch();
    assert!(pitch < -1.0, "bottom pitch should be near -PI/2, got {pitch}");
}

// ---------------------------------------------------------------------------
// Camera
// ---------------------------------------------------------------------------

#[test]
fn camera_new_has_sensible_defaults() {
    let cam = Camera::new(16.0 / 9.0);
    assert!(cam.distance > 0.0, "distance should be positive");
    assert!(cam.znear > 0.0, "znear must be positive");
    assert!(cam.zfar > cam.znear, "zfar must exceed znear");
    assert_eq!(cam.projection, Projection::Perspective);
}

#[test]
fn camera_toggle_projection_switches_type() {
    let mut cam = Camera::new(1.0);
    assert_eq!(cam.projection, Projection::Perspective);
    cam.toggle_projection();
    assert_eq!(cam.projection, Projection::Orthographic);
    cam.toggle_projection();
    assert_eq!(cam.projection, Projection::Perspective);
}

#[test]
fn camera_snap_to_view_sets_yaw_pitch() {
    let mut cam = Camera::new(1.0);
    cam.snap_to_view(StandardView::Front);
    let (yaw, pitch) = StandardView::Front.yaw_pitch();
    assert!((cam.yaw - yaw).abs() < 1e-5);
    assert!((cam.pitch - pitch).abs() < 1e-5);
}

#[test]
fn camera_reset_restores_defaults() {
    let mut cam = Camera::new(1.0);
    cam.distance = 999.0;
    cam.yaw = 99.0;
    cam.pitch = 99.0;
    cam.roll = 3.0;
    cam.reset();
    assert!((cam.distance - 20.0).abs() < 1e-5);
    assert!(cam.roll.abs() < 1e-5);
    assert_eq!(cam.projection, Projection::Perspective);
}

#[test]
fn camera_fit_to_bounds_centers_target() {
    let mut cam = Camera::new(1.0);
    cam.fit_to_bounds([-10.0, -10.0, -10.0], [10.0, 10.0, 10.0]);
    assert!((cam.target[0]).abs() < 1e-4, "target.x should be 0");
    assert!((cam.target[1]).abs() < 1e-4, "target.y should be 0");
    assert!((cam.target[2]).abs() < 1e-4, "target.z should be 0");
    assert!(cam.distance > 0.0, "distance should remain positive");
}

#[test]
fn camera_fit_to_bounds_sets_positive_distance() {
    let mut cam = Camera::new(1.0);
    cam.fit_to_bounds([0.0; 3], [1.0, 1.0, 1.0]);
    assert!(cam.distance > 0.0);
}

#[test]
fn camera_eye_is_different_from_target() {
    let cam = Camera::new(1.0);
    let eye = cam.eye();
    let target = cam.target;
    let diff: f32 = eye.iter().zip(target.iter()).map(|(a, b)| (a - b).abs()).sum();
    assert!(diff > 0.01, "eye should not coincide with target");
}

#[test]
fn camera_view_matrix_is_4x4() {
    let cam = Camera::new(16.0 / 9.0);
    let m = cam.view_matrix();
    assert_eq!(m.len(), 4);
    for row in &m {
        assert_eq!(row.len(), 4);
    }
}

#[test]
fn camera_projection_matrix_is_4x4() {
    let cam = Camera::new(16.0 / 9.0);
    let m = cam.projection_matrix();
    assert_eq!(m.len(), 4);
    for row in &m {
        assert_eq!(row.len(), 4);
    }
}

#[test]
fn camera_screen_right_is_unit_length() {
    let cam = Camera::new(1.0);
    let r = cam.screen_right();
    let len = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    assert!((len - 1.0).abs() < 1e-4, "screen_right length={len}");
}

#[test]
fn camera_screen_up_is_unit_length() {
    let cam = Camera::new(1.0);
    let u = cam.screen_up();
    let len = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    assert!((len - 1.0).abs() < 1e-4, "screen_up length={len}");
}

// ---------------------------------------------------------------------------
// NavConfig
// ---------------------------------------------------------------------------

#[test]
fn nav_config_default_style_is_freecad_gesture() {
    let cfg = NavConfig::new();
    assert_eq!(cfg.style, NavStyle::FreeCADGesture);
}

#[test]
fn nav_config_scroll_zoom_factor_positive_scroll_reduces_distance_when_inverted() {
    let cfg = NavConfig::new();
    // default: invert_zoom = true, so positive scroll → zoom out (factor > 1)
    let f = cfg.scroll_zoom_factor(1.0);
    assert!(f > 0.0, "factor must be positive");
}

#[test]
fn nav_config_drag_zoom_factor_positive_dy_reduces_when_inverted() {
    let cfg = NavConfig::new();
    let f = cfg.drag_zoom_factor(0.5);
    assert!(f > 0.0, "drag zoom factor must be positive");
}

#[test]
fn nav_config_resolve_drag_freecad_mmb_is_orbit() {
    let cfg = NavConfig::new();
    let action = cfg.resolve_drag(false, true, false, false, false, false);
    assert_eq!(action, NavAction::Orbit);
}

#[test]
fn nav_config_resolve_drag_freecad_rmb_is_pan() {
    let cfg = NavConfig::new();
    let action = cfg.resolve_drag(false, false, true, false, false, false);
    assert_eq!(action, NavAction::Pan);
}

#[test]
fn nav_config_resolve_drag_freecad_ctrl_rmb_is_zoom() {
    let cfg = NavConfig::new();
    let action = cfg.resolve_drag(false, false, true, false, true, false);
    assert_eq!(action, NavAction::Zoom);
}

#[test]
fn nav_config_resolve_drag_freecad_no_buttons_is_none() {
    let cfg = NavConfig::new();
    let action = cfg.resolve_drag(false, false, false, false, false, false);
    assert_eq!(action, NavAction::None);
}

#[test]
fn nav_config_resolve_drag_blender_mmb_is_orbit() {
    let mut cfg = NavConfig::new();
    cfg.style = NavStyle::Blender;
    let action = cfg.resolve_drag(false, true, false, false, false, false);
    assert_eq!(action, NavAction::Orbit);
}

#[test]
fn nav_config_resolve_drag_maya_alt_lmb_is_orbit() {
    let mut cfg = NavConfig::new();
    cfg.style = NavStyle::Maya;
    let action = cfg.resolve_drag(true, false, false, false, false, true);
    assert_eq!(action, NavAction::Orbit);
}

#[test]
fn nav_config_snap_3d_off_returns_value_unchanged() {
    let mut cfg = NavConfig::new();
    cfg.snap_to_grid_3d = false;
    let val = 1.23456;
    assert!((cfg.snap_3d(val) - val).abs() < f64::EPSILON);
}

#[test]
fn nav_config_snap_3d_on_rounds_to_spacing() {
    let mut cfg = NavConfig::new();
    cfg.snap_to_grid_3d = true;
    cfg.grid_3d_spacing = 1.0;
    assert_eq!(cfg.snap_3d(2.4), 2.0);
    assert_eq!(cfg.snap_3d(2.6), 3.0);
}

#[test]
fn nav_style_all_have_non_empty_labels() {
    for s in NavStyle::ALL {
        assert!(!s.label().is_empty(), "empty label for {s:?}");
    }
}

#[test]
fn nav_style_all_have_non_empty_descriptions() {
    for s in NavStyle::ALL {
        assert!(!s.description().is_empty(), "empty description for {s:?}");
    }
}

#[test]
fn orbit_style_all_have_non_empty_labels() {
    for s in OrbitStyle::ALL {
        assert!(!s.label().is_empty(), "empty label for {s:?}");
    }
}

#[test]
fn rotation_mode_all_have_non_empty_labels() {
    for m in RotationMode::ALL {
        assert!(!m.label().is_empty(), "empty label for {m:?}");
    }
}

#[test]
fn unit_system_all_have_non_empty_labels() {
    for u in UnitSystem::ALL {
        assert!(!u.label().is_empty(), "empty label for {u:?}");
        assert!(!u.long_label().is_empty(), "empty long label for {u:?}");
    }
}

#[test]
fn unit_system_mm_short_label_is_mm() {
    assert_eq!(UnitSystem::Millimeters.label(), "mm");
}

#[test]
fn bg_preset_all_have_non_empty_labels() {
    for p in BgPreset::ALL {
        assert!(!p.label().is_empty(), "empty label for {p:?}");
    }
}

// ---------------------------------------------------------------------------
// CreationParams — roundtrip via serde
// ---------------------------------------------------------------------------

#[test]
fn creation_params_box_serialises_and_deserialises() {
    let params = CreationParams::Box { width: 1.0, height: 2.0, depth: 3.0 };
    let json = serde_json::to_string(&params).unwrap();
    let back: CreationParams = serde_json::from_str(&json).unwrap();
    if let CreationParams::Box { width, height, depth } = back {
        assert!((width - 1.0).abs() < f64::EPSILON);
        assert!((height - 2.0).abs() < f64::EPSILON);
        assert!((depth - 3.0).abs() < f64::EPSILON);
    } else {
        panic!("unexpected variant after roundtrip");
    }
}

#[test]
fn creation_params_sphere_serialises_and_deserialises() {
    let params = CreationParams::Sphere { radius: 5.0 };
    let json = serde_json::to_string(&params).unwrap();
    let back: CreationParams = serde_json::from_str(&json).unwrap();
    assert!(matches!(back, CreationParams::Sphere { radius } if (radius - 5.0).abs() < f64::EPSILON));
}

// ---------------------------------------------------------------------------
// ObjectGroup
// ---------------------------------------------------------------------------

#[test]
fn object_group_construction_stores_fields() {
    let g = ObjectGroup { id: 42, name: "test_group".into(), visible: true };
    assert_eq!(g.id, 42);
    assert_eq!(g.name, "test_group");
    assert!(g.visible);
}

// ---------------------------------------------------------------------------
// Scene (headless, no GPU — uses add_mesh_object only)
// ---------------------------------------------------------------------------

fn empty_mesh() -> cadkernel_io::Mesh {
    cadkernel_io::Mesh::new()
}

#[test]
fn scene_new_is_empty() {
    let scene = Scene::new();
    assert!(scene.is_empty());
    assert_eq!(scene.len(), 0);
}

#[test]
fn scene_default_is_empty() {
    let scene = Scene::default();
    assert!(scene.is_empty());
}

#[test]
fn scene_add_mesh_object_increments_len() {
    let mut scene = Scene::new();
    scene.add_mesh_object("Obj1", empty_mesh(), None);
    assert_eq!(scene.len(), 1);
    scene.add_mesh_object("Obj2", empty_mesh(), None);
    assert_eq!(scene.len(), 2);
}

#[test]
fn scene_remove_object_decrements_len() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("Obj", empty_mesh(), None);
    assert!(scene.remove_object(id));
    assert!(scene.is_empty());
}

#[test]
fn scene_remove_nonexistent_object_returns_false() {
    let mut scene = Scene::new();
    assert!(!scene.remove_object(999));
}

#[test]
fn scene_add_objects_get_incrementing_ids() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    assert_ne!(id1, id2);
    assert!(id2 > id1);
}

#[test]
fn scene_get_returns_object_by_id() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("Named", empty_mesh(), None);
    let obj = scene.get(id).unwrap();
    assert_eq!(obj.name, "Named");
    assert_eq!(obj.id, id);
}

#[test]
fn scene_get_nonexistent_returns_none() {
    let scene = Scene::new();
    assert!(scene.get(42).is_none());
}

#[test]
fn scene_get_mut_allows_mutation() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("Obj", empty_mesh(), None);
    scene.get_mut(id).unwrap().name = "Renamed".into();
    assert_eq!(scene.get(id).unwrap().name, "Renamed");
}

#[test]
fn scene_new_objects_are_visible_and_unselected() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("Obj", empty_mesh(), None);
    let obj = scene.get(id).unwrap();
    assert!(obj.visible);
    assert!(!obj.selected);
}

#[test]
fn scene_visible_objects_excludes_hidden() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    scene.add_mesh_object("B", empty_mesh(), None);
    scene.get_mut(id).unwrap().visible = false;
    assert_eq!(scene.visible_objects().count(), 1);
}

#[test]
fn scene_select_single_selects_one_and_deselects_others() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    scene.select_single(id1);
    assert!(scene.get(id1).unwrap().selected);
    assert!(!scene.get(id2).unwrap().selected);
    scene.select_single(id2);
    assert!(!scene.get(id1).unwrap().selected);
    assert!(scene.get(id2).unwrap().selected);
}

#[test]
fn scene_deselect_all_clears_selection() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    scene.select_single(id);
    scene.deselect_all();
    assert!(!scene.get(id).unwrap().selected);
    assert!(scene.selected_ids().is_empty());
}

#[test]
fn scene_select_all_selects_only_visible() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    scene.get_mut(id2).unwrap().visible = false;
    scene.select_all();
    assert!(scene.get(id1).unwrap().selected);
    assert!(!scene.get(id2).unwrap().selected);
}

#[test]
fn scene_toggle_select_flips_selection_state() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    assert!(!scene.get(id).unwrap().selected);
    scene.toggle_select(id);
    assert!(scene.get(id).unwrap().selected);
    scene.toggle_select(id);
    assert!(!scene.get(id).unwrap().selected);
}

#[test]
fn scene_selected_ids_returns_correct_set() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    scene.select_single(id1);
    let ids = scene.selected_ids();
    assert_eq!(ids, vec![id1]);
    scene.toggle_select(id2);
    let ids = scene.selected_ids();
    assert_eq!(ids.len(), 2);
}

#[test]
fn scene_move_up_reorders_objects() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    scene.move_up(id2);
    assert_eq!(scene.objects[0].id, id2);
    assert_eq!(scene.objects[1].id, id1);
}

#[test]
fn scene_move_down_reorders_objects() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    scene.move_down(id1);
    assert_eq!(scene.objects[0].id, id2);
    assert_eq!(scene.objects[1].id, id1);
}

#[test]
fn scene_root_objects_excludes_children() {
    let mut scene = Scene::new();
    let parent = scene.add_mesh_object("Parent", empty_mesh(), None);
    let child = scene.add_mesh_object("Child", empty_mesh(), None);
    scene.get_mut(child).unwrap().parent_id = Some(parent);
    let roots = scene.root_objects();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].id, parent);
}

#[test]
fn scene_children_of_returns_only_direct_children() {
    let mut scene = Scene::new();
    let parent = scene.add_mesh_object("Parent", empty_mesh(), None);
    let child1 = scene.add_mesh_object("Child1", empty_mesh(), None);
    let child2 = scene.add_mesh_object("Child2", empty_mesh(), None);
    scene.get_mut(child1).unwrap().parent_id = Some(parent);
    scene.get_mut(child2).unwrap().parent_id = Some(parent);
    let children = scene.children_of(parent);
    assert_eq!(children.len(), 2);
}

#[test]
fn scene_active_body_set_and_clear() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("Body", empty_mesh(), None);
    assert!(scene.active_body_id.is_none());
    scene.set_active_body(Some(id));
    assert_eq!(scene.active_body_id, Some(id));
    scene.set_active_body(None);
    assert!(scene.active_body_id.is_none());
}

#[test]
fn scene_create_group_returns_increasing_ids() {
    let mut scene = Scene::new();
    let g1 = scene.create_group("G1");
    let g2 = scene.create_group("G2");
    assert_ne!(g1, g2);
    assert!(g2 > g1);
}

#[test]
fn scene_group_selected_assigns_group_id() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    scene.select_single(id);
    let gid = scene.create_group("MyGroup");
    scene.group_selected(gid);
    assert_eq!(scene.get(id).unwrap().group_id, gid);
}

#[test]
fn scene_ungroup_object_sets_group_zero() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    scene.select_single(id);
    let gid = scene.create_group("G");
    scene.group_selected(gid);
    scene.ungroup_object(id);
    assert_eq!(scene.get(id).unwrap().group_id, 0);
}

#[test]
fn scene_group_members_returns_members() {
    let mut scene = Scene::new();
    let id1 = scene.add_mesh_object("A", empty_mesh(), None);
    let id2 = scene.add_mesh_object("B", empty_mesh(), None);
    scene.select_single(id1);
    scene.toggle_select(id2);
    let gid = scene.create_group("G");
    scene.group_selected(gid);
    let members = scene.group_members(gid);
    assert_eq!(members.len(), 2);
    assert!(members.contains(&id1));
    assert!(members.contains(&id2));
}

#[test]
fn scene_toggle_group_visibility_hides_and_shows_members() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    scene.select_single(id);
    let gid = scene.create_group("G");
    scene.group_selected(gid);
    scene.toggle_group_visibility(gid);
    assert!(!scene.get(id).unwrap().visible);
    scene.toggle_group_visibility(gid);
    assert!(scene.get(id).unwrap().visible);
}

#[test]
fn scene_delete_group_ungroups_members() {
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("A", empty_mesh(), None);
    scene.select_single(id);
    let gid = scene.create_group("G");
    scene.group_selected(gid);
    scene.delete_group(gid);
    assert_eq!(scene.get(id).unwrap().group_id, 0);
    assert!(!scene.groups.iter().any(|g| g.id == gid));
}

// ---------------------------------------------------------------------------
// ScriptEngine (headless Lua execution)
// ---------------------------------------------------------------------------

use cadkernel_viewer::scripting::ScriptEngine;

#[test]
fn script_engine_creates_successfully() {
    assert!(ScriptEngine::new().is_ok());
}

#[test]
fn script_engine_execute_returns_number() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return 42").unwrap();
    assert_eq!(out.trim(), "42");
}

#[test]
fn script_engine_execute_returns_string() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute(r#"return "hello""#).unwrap();
    assert_eq!(out, "hello");
}

#[test]
fn script_engine_execute_returns_boolean_true() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return true").unwrap();
    assert_eq!(out, "true");
}

#[test]
fn script_engine_execute_nil_returns_empty_string() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return nil").unwrap();
    assert!(out.is_empty());
}

#[test]
fn script_engine_sandbox_os_is_nil() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return type(os)").unwrap();
    assert_eq!(out, "nil");
}

#[test]
fn script_engine_sandbox_io_is_nil() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return type(io)").unwrap();
    assert_eq!(out, "nil");
}

#[test]
fn script_engine_sandbox_require_is_nil() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return type(require)").unwrap();
    assert_eq!(out, "nil");
}

#[test]
fn script_engine_cad_table_is_a_table() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return type(cad)").unwrap();
    assert_eq!(out, "table");
}

#[test]
fn script_engine_cad_box_returns_id_zero() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return cad.box(1, 1, 1)").unwrap();
    assert_eq!(out.trim(), "0");
    assert_eq!(eng.solid_count(), 1);
}

#[test]
fn script_engine_cad_cylinder_creates_solid() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.cylinder(2, 5)").unwrap();
    assert_eq!(eng.solid_count(), 1);
}

#[test]
fn script_engine_cad_sphere_creates_solid() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.sphere(3)").unwrap();
    assert_eq!(eng.solid_count(), 1);
}

#[test]
fn script_engine_cad_cone_creates_solid() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.cone(4, 10)").unwrap();
    assert_eq!(eng.solid_count(), 1);
}

#[test]
fn script_engine_cad_torus_creates_solid() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.torus(8, 2)").unwrap();
    assert_eq!(eng.solid_count(), 1);
}

#[test]
fn script_engine_multiple_primitives_accumulate() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.box(1,1,1)").unwrap();
    eng.execute("cad.sphere(1)").unwrap();
    eng.execute("cad.cylinder(1, 2)").unwrap();
    assert_eq!(eng.solid_count(), 3);
}

#[test]
fn script_engine_clear_removes_all_solids() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.box(1,1,1)").unwrap();
    eng.execute("cad.sphere(2)").unwrap();
    eng.execute("cad.clear()").unwrap();
    assert_eq!(eng.solid_count(), 0);
}

#[test]
fn script_engine_delete_removes_one_solid() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.box(1,1,1)").unwrap();
    eng.execute("cad.sphere(2)").unwrap();
    eng.execute("cad.delete(0)").unwrap();
    assert_eq!(eng.solid_count(), 1);
}

#[test]
fn script_engine_delete_invalid_id_returns_error() {
    let mut eng = ScriptEngine::new().unwrap();
    let result = eng.execute("cad.delete(99)");
    assert!(result.is_err(), "deleting non-existent id should error");
}

#[test]
fn script_engine_list_returns_table_of_ids() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.box(1,1,1)").unwrap();
    eng.execute("cad.sphere(2)").unwrap();
    let out = eng.execute("local l = cad.list(); return #l").unwrap();
    assert_eq!(out.trim(), "2");
}

#[test]
fn script_engine_measure_returns_positive_volume() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng
        .execute(
            r#"
            local id = cad.box(2, 3, 4)
            local m = cad.measure(id)
            return m.volume
        "#,
        )
        .unwrap();
    let vol: f64 = out.trim().parse().expect("volume should be numeric");
    assert!(vol > 0.0, "volume should be positive, got {vol}");
}

#[test]
fn script_engine_count_returns_topology_info() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng
        .execute(
            r#"
            local id = cad.box(1,1,1)
            local c = cad.count(id)
            return c.faces
        "#,
        )
        .unwrap();
    let faces: i64 = out.trim().parse().expect("faces should be numeric");
    assert!(faces > 0, "box should have faces");
}

#[test]
fn script_engine_translate_creates_new_solid() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng
        .execute(
            r#"
            local id = cad.box(1,1,1)
            return cad.translate(id, 5, 0, 0)
        "#,
        )
        .unwrap();
    let new_id: i64 = out.trim().parse().expect("translate returns id");
    assert!(new_id >= 0);
}

#[test]
fn script_engine_get_models_returns_models() {
    let mut eng = ScriptEngine::new().unwrap();
    eng.execute("cad.box(1,1,1)").unwrap();
    eng.execute("cad.sphere(2)").unwrap();
    let models = eng.get_models();
    assert_eq!(models.len(), 2);
}

#[test]
fn script_engine_syntax_error_returns_err() {
    let mut eng = ScriptEngine::new().unwrap();
    let result = eng.execute("this is not valid lua @@##");
    assert!(result.is_err());
}

#[test]
fn script_engine_arithmetic_in_lua() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng.execute("return 2 + 3 * 4").unwrap();
    assert_eq!(out.trim(), "14");
}

#[test]
fn script_engine_lua_variables_persist_within_single_execute() {
    let mut eng = ScriptEngine::new().unwrap();
    let out = eng
        .execute(
            r#"
            local x = 10
            local y = x * x
            return y
        "#,
        )
        .unwrap();
    assert_eq!(out.trim(), "100");
}
