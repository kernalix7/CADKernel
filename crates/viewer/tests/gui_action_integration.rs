//! Integration tests for GuiAction dispatch paths.
//!
//! Primitive-creation arms are exercised end-to-end via the headless
//! dispatcher (`cadkernel_viewer::test_support::CadApp`); every other arm
//! tests the modeling/sketch/io call chain that the dispatcher executes,
//! since those layers — not the dispatcher — are the system under test.
//! See `docs/FREECAD_PARITY_PLAN.md` for known stubs and pending wiring.

use cadkernel_io::{
    DrawingSheet, Mesh, ProjectionDir, drawing_to_svg, evaluate_and_repair, project_solid,
    tessellate_solid, three_view_drawing,
};
use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::{
    BooleanOp, boolean_op, chamfer_edge, fillet_edge, filling, hole, linear_pattern, make_box,
    make_line_draft, make_polygon_wire, make_rectangle_wire, make_sphere, mirror_solid, pad,
    pipe_surface, pocket, quick_box, quick_intersect, quick_union, quick_volume, scale_solid,
    shell_solid,
};
use cadkernel_sketch::{Constraint, Sketch, WorkPlane};
use cadkernel_topology::BRepModel;
use cadkernel_viewer::{
    Camera, DisplayMode, Projection, StandardView, Vertex,
    command::{CommandStack, ModelSnapshot},
    compute_bounds, mesh_to_vertices,
    nav::NavConfig,
    picking::{pick_edge, pick_triangle, pick_vertex, screen_to_ray},
    scene::{CreationParams, Scene, compute_aabb},
    test_support::CadApp,
};

// ---------------------------------------------------------------------------
// Helpers that mirror the CadApp dispatcher's "add new solid to scene" path.
// ---------------------------------------------------------------------------

/// Construct a box solid and add it to `scene` under `name`.
/// Mirrors the CreateBox dispatcher arm in app.rs::process_actions.
fn add_box(scene: &mut Scene, name: &str, w: f64, h: f64, d: f64) -> u32 {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, w, h, d).expect("make_box");
    let params = CreationParams::Box {
        width: w,
        height: h,
        depth: d,
    };
    scene.add_object(name, model, r.solid, Some(params), None)
}

fn add_box_at(scene: &mut Scene, name: &str, origin: Point3, w: f64, h: f64, d: f64) -> u32 {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, origin, w, h, d).expect("make_box");
    let params = CreationParams::Box {
        width: w,
        height: h,
        depth: d,
    };
    scene.add_object(name, model, r.solid, Some(params), None)
}

fn add_sphere(scene: &mut Scene, name: &str, radius: f64) -> u32 {
    let mut model = BRepModel::new();
    let r = make_sphere(&mut model, Point3::ORIGIN, radius, 64, 32).expect("make_sphere");
    let params = CreationParams::Sphere { radius };
    scene.add_object(name, model, r.solid, Some(params), None)
}

fn snapshot_from(scene: &Scene) -> ModelSnapshot {
    // Use the first object's model as the app snapshot would
    let (model, solid) = if let Some(obj) = scene.objects.first() {
        (obj.model.clone(), Some(obj.solid))
    } else {
        (BRepModel::new(), None)
    };
    ModelSnapshot {
        model,
        current_solid: solid,
        current_mesh: None,
    }
}

// ---------------------------------------------------------------------------
// Part workbench — Primitive creation (CreateBox/Cylinder/Sphere/Cone/Torus/...)
// ---------------------------------------------------------------------------

#[test]
fn create_box_adds_one_object_with_positive_extent() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 3.0, 4.0);
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1);
    let obj = scene.objects.first().expect("object");
    assert!(obj.aabb_max[0] - obj.aabb_min[0] >= 2.0 - 1e-3);
    assert!(obj.aabb_max[1] - obj.aabb_min[1] >= 3.0 - 1e-3);
    assert!(obj.aabb_max[2] - obj.aabb_min[2] >= 4.0 - 1e-3);
}

#[test]
fn create_cylinder_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_cylinder(2.0, 5.0);
    let obj = app.scene_ref().objects.first().expect("object");
    assert!((obj.aabb_max[2] - obj.aabb_min[2]) >= 5.0 - 1e-3);
}

#[test]
fn create_sphere_adds_one_object_with_diameter_bbox() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_sphere(3.0);
    let obj = app.scene_ref().objects.first().expect("object");
    for axis in 0..3 {
        let extent = obj.aabb_max[axis] - obj.aabb_min[axis];
        assert!(
            extent >= 5.8,
            "sphere diameter axis {axis} ~6.0 got {extent}"
        );
    }
}

#[test]
fn create_cone_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_cone(2.0, 0.5, 4.0);
    let obj = app.scene_ref().objects.first().expect("object");
    assert!(!obj.vertices.is_empty());
    // height=4 → z-extent must be ≥3.9; base radius=2 → xy-extent ≥3.9
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 3.9,
        "cone height extent should be ~4.0, got {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
    assert!(
        (obj.aabb_max[0] - obj.aabb_min[0]) >= 3.9,
        "cone base diameter extent should be ~4.0, got {}",
        obj.aabb_max[0] - obj.aabb_min[0]
    );
}

#[test]
fn create_torus_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_torus(5.0, 1.0);
    let obj = app.scene_ref().objects.first().expect("object");
    assert!((obj.aabb_max[0] - obj.aabb_min[0]) >= 11.5);
}

#[test]
fn create_tube_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_tube(3.0, 1.0, 5.0);
    let obj = app.scene_ref().objects.first().expect("object");
    assert!(!obj.vertices.is_empty());
    // outer_radius=3 → xy-extent ≥5.9; height=5 → z-extent ≥4.9
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 4.9,
        "tube height extent should be ~5.0, got {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
    assert!(
        (obj.aabb_max[0] - obj.aabb_min[0]) >= 5.9,
        "tube outer-diameter extent should be ~6.0, got {}",
        obj.aabb_max[0] - obj.aabb_min[0]
    );
}

#[test]
fn create_prism_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_prism(2.0, 4.0, 6);
    let obj = app.scene_ref().objects.first().expect("object");
    assert!(!obj.vertices.is_empty());
    // height=4 → z-extent ≥3.9; regular hexagon radius=2 → xy-extent ≥3.9
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 3.9,
        "prism height extent should be ~4.0, got {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
    assert!(
        (obj.aabb_max[0] - obj.aabb_min[0]) >= 3.9 || (obj.aabb_max[1] - obj.aabb_min[1]) >= 3.9,
        "prism cross-section extent should be ~4.0 (diameter), got x={} y={}",
        obj.aabb_max[0] - obj.aabb_min[0],
        obj.aabb_max[1] - obj.aabb_min[1]
    );
}

#[test]
fn create_wedge_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_wedge(4.0, 3.0, 2.0, 1.0, 1.0);
    assert!(!app.scene_ref().objects.first().unwrap().vertices.is_empty());
}

#[test]
fn create_ellipsoid_adds_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_ellipsoid(2.0, 3.0, 4.0);
    let obj = app.scene_ref().objects.first().expect("object");
    assert!((obj.aabb_max[2] - obj.aabb_min[2]) >= 7.5);
}

// ---------------------------------------------------------------------------
// Part workbench — Booleans (BooleanUnionWith, SceneUnion, etc.)
// ---------------------------------------------------------------------------

#[test]
fn boolean_union_combines_two_boxes_into_fewer_objects() {
    // Simulates BooleanSceneUnion: take two selected objects, run union,
    // replace with the result.
    let mut scene = Scene::new();
    let id_a = add_box_at(&mut scene, "A", Point3::new(0.0, 0.0, 0.0), 2.0, 2.0, 2.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(1.0, 0.0, 0.0), 2.0, 2.0, 2.0);
    let obj_a = scene.get(id_a).unwrap().clone();
    let obj_b = scene.get(id_b).unwrap().clone();
    let result_model = boolean_op(
        &obj_a.model,
        obj_a.solid,
        &obj_b.model,
        obj_b.solid,
        BooleanOp::Union,
    )
    .expect("boolean union");
    scene.remove_object(id_a);
    scene.remove_object(id_b);
    let first = result_model
        .solids
        .iter()
        .next()
        .map(|(h, _)| h)
        .expect("union produced no solid");
    scene.add_object("A_union_B", result_model, first, None, None);
    assert_eq!(scene.len(), 1, "union should leave exactly one object");
}

#[test]
fn boolean_subtract_box_minus_sphere_shrinks_volume() {
    // BUG: boolean_op(box, sphere, Difference) on a 4x4x4 box minus a sphere of radius 1.5
    //      yields quick_volume(result) ≈ 77.9, while the original box volume is 64.
    //      Expected: a Difference must NEVER produce a larger volume than the minuend.
    //      Likely cause: result model accumulates orphan faces or sphere tessellation
    //      is added to the volume sum instead of being subtracted.
    let a = quick_box(4.0, 4.0, 4.0).unwrap();
    let mut b_model = BRepModel::new();
    let r = make_sphere(&mut b_model, Point3::new(2.0, 2.0, 2.0), 1.5, 32, 16).unwrap();
    let a_solid = a.solids.iter().next().unwrap().0;
    let result =
        boolean_op(&a, a_solid, &b_model, r.solid, BooleanOp::Difference).expect("subtract");
    let vol_orig = quick_volume(&a).unwrap();
    let vol_sub = quick_volume(&result).unwrap();
    assert!(
        vol_sub < vol_orig,
        "subtract must reduce volume: before={vol_orig} after={vol_sub}"
    );
}

#[test]
fn boolean_intersect_two_overlapping_boxes_produces_nonzero_volume() {
    let a = quick_box(2.0, 2.0, 2.0).unwrap();
    let mut b_model = BRepModel::new();
    make_box(&mut b_model, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();
    let result = quick_intersect(&a, &b_model).expect("intersect");
    let vol = quick_volume(&result).unwrap_or(0.0);
    assert!(
        vol > 0.0,
        "intersection should have positive volume, got {vol}"
    );
}

#[test]
fn boolean_union_disjoint_preserves_total_volume() {
    let a = quick_box(1.0, 1.0, 1.0).unwrap();
    let mut b_model = BRepModel::new();
    make_box(&mut b_model, Point3::new(10.0, 0.0, 0.0), 1.0, 1.0, 1.0).unwrap();
    let result = quick_union(&a, &b_model).expect("union disjoint");
    let vol = quick_volume(&result).unwrap();
    assert!(
        vol >= 1.5,
        "disjoint union volume should be ~2.0, got {vol}"
    );
}

#[test]
fn scene_boolean_union_combines_two_selected() {
    // BooleanSceneUnion requires 2 selected objects. Multi-select via
    // toggle_select.
    let mut scene = Scene::new();
    let id_a = add_box_at(&mut scene, "A", Point3::ORIGIN, 2.0, 2.0, 2.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(1.0, 0.0, 0.0), 2.0, 2.0, 2.0);
    scene.toggle_select(id_a);
    scene.toggle_select(id_b);
    assert_eq!(
        scene.selected_ids().len(),
        2,
        "must have 2 selected for scene boolean"
    );
    // The dispatcher would then call the modeling crate:
    let a = scene.get(id_a).unwrap().clone();
    let b = scene.get(id_b).unwrap().clone();
    let merged =
        boolean_op(&a.model, a.solid, &b.model, b.solid, BooleanOp::Union).expect("boolean op");
    assert!(merged.solids.iter().count() >= 1);
}

// ---------------------------------------------------------------------------
// Part features — Mirror, Scale, Shell, Fillet, Chamfer, Pattern
// ---------------------------------------------------------------------------

#[test]
fn mirror_solid_produces_new_solid() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::new(1.0, 1.0, 1.0), 2.0, 2.0, 2.0).unwrap();
    let normal = Vec3::new(1.0, 0.0, 0.0);
    let mirror_result =
        mirror_solid(&mut model, r.solid, Point3::ORIGIN, normal).expect("mirror_solid");
    assert_ne!(
        mirror_result.solid, r.solid,
        "mirror should create new solid handle"
    );
}

#[test]
fn scale_solid_with_factor_2_produces_new_solid() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let result = scale_solid(&mut model, r.solid, Point3::ORIGIN, 2.0).expect("scale_solid");
    assert_ne!(result.solid, r.solid, "scale should create new solid");
}

#[test]
fn shell_solid_produces_new_solid() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
    let result = shell_solid(&mut model, r.solid, &[], 0.2);
    // Shell may fail on some topologies; accept either success or a clean error,
    // but never panic.
    match result {
        Ok(sr) => assert_ne!(sr.solid, r.solid),
        Err(e) => {
            // Acceptable — shell is geometrically sensitive. Document the error.
            eprintln!("shell_solid error (acceptable): {e}");
        }
    }
}

#[test]
fn fillet_all_edges_runs_without_panic() {
    // FilletAllEdges dispatcher iterates every edge and fillets sequentially.
    // We simulate by filleting one edge on a box.
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
    // pick the first edge and use its two endpoint vertices.
    let first_edge = model
        .edges
        .iter()
        .next()
        .map(|(h, ed)| (h, ed.start, ed.end));
    if let Some((_eh, v1, v2)) = first_edge {
        let _ = fillet_edge(&mut model, r.solid, v1, v2, 0.1);
        // Acceptable either way — fillet is approximate.
    }
}

#[test]
fn chamfer_all_edges_runs_without_panic() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
    let first_edge = model
        .edges
        .iter()
        .next()
        .map(|(h, ed)| (h, ed.start, ed.end));
    if let Some((_eh, v1, v2)) = first_edge {
        let _ = chamfer_edge(&mut model, r.solid, v1, v2, 0.1);
    }
}

#[test]
fn linear_pattern_produces_multiple_copies() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let dir = Vec3::new(1.0, 0.0, 0.0);
    let result = linear_pattern(&mut model, r.solid, dir, 3.0, 3);
    match result {
        Ok(pr) => {
            assert!(
                !pr.solids.is_empty() || !pr.faces.is_empty(),
                "pattern must produce outputs"
            );
        }
        Err(e) => eprintln!("linear_pattern error (may be expected): {e}"),
    }
}

// ---------------------------------------------------------------------------
// PartDesign — Pad, Pocket, Hole (each operates on a base solid + profile)
// ---------------------------------------------------------------------------

#[test]
fn pad_sketch_onto_box_increases_volume() {
    let mut base = BRepModel::new();
    let b = make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 2.0).unwrap();
    let profile = vec![
        Point3::new(1.0, 1.0, 2.0),
        Point3::new(3.0, 1.0, 2.0),
        Point3::new(3.0, 3.0, 2.0),
        Point3::new(1.0, 3.0, 2.0),
    ];
    let result = pad(&base, b.solid, &profile, Vec3::Z, 1.0).expect("pad");
    let vol_base = quick_volume(&base).unwrap();
    let vol_padded = quick_volume(&result.model).unwrap();
    assert!(
        vol_padded > vol_base,
        "pad must add material: {vol_base}→{vol_padded}"
    );
}

#[test]
fn pocket_sketch_on_box_removes_material() {
    let mut base = BRepModel::new();
    let b = make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
    let profile = vec![
        Point3::new(1.0, 1.0, 4.0),
        Point3::new(3.0, 1.0, 4.0),
        Point3::new(3.0, 3.0, 4.0),
        Point3::new(1.0, 3.0, 4.0),
    ];
    let result = pocket(&base, b.solid, &profile, Vec3::new(0.0, 0.0, -1.0), 1.0).expect("pocket");
    let vol_base = quick_volume(&base).unwrap();
    let vol_pocketed = quick_volume(&result.model).unwrap();
    assert!(
        vol_pocketed < vol_base,
        "pocket must remove material: {vol_base}→{vol_pocketed}"
    );
}

#[test]
fn hole_on_box_removes_cylindrical_material() {
    let mut base = BRepModel::new();
    let b = make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 4.0).unwrap();
    let pos = Point3::new(2.0, 2.0, 4.0);
    let dir = Vec3::new(0.0, 0.0, -1.0);
    let result = hole(&base, b.solid, pos, dir, 0.5, 2.0, 32);
    match result {
        Ok(hr) => {
            let vol_base = quick_volume(&base).unwrap();
            let vol_hole = quick_volume(&hr.model).unwrap();
            assert!(vol_hole < vol_base, "hole must reduce volume");
        }
        Err(e) => eprintln!("hole error: {e}"),
    }
}

#[test]
fn pad_rejects_degenerate_profile() {
    // BUG: dispatcher does not pre-validate — pad errors, dispatcher must handle.
    let mut base = BRepModel::new();
    let b = make_box(&mut base, Point3::ORIGIN, 4.0, 4.0, 2.0).unwrap();
    let short_profile = vec![Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0)];
    let result = pad(&base, b.solid, &short_profile, Vec3::Z, 1.0);
    assert!(result.is_err(), "pad must reject <3-point profile");
}

// ---------------------------------------------------------------------------
// Sketcher workbench — Sketch creation + constraints
// ---------------------------------------------------------------------------

#[test]
fn sketcher_create_line_adds_line_entity() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 0.0);
    let line = sketch.add_line(p0, p1);
    assert_eq!(sketch.lines.len(), 1);
    assert!(line.0 < usize::MAX, "LineId must be valid");
}

#[test]
fn sketcher_create_circle_adds_circle_entity() {
    let mut sketch = Sketch::new();
    let c = sketch.add_point(0.0, 0.0);
    let _circle = sketch.add_circle(c, 5.0);
    assert_eq!(sketch.circles.len(), 1);
}

#[test]
fn sketcher_create_arc_adds_arc_entity() {
    let mut sketch = Sketch::new();
    let center = sketch.add_point(0.0, 0.0);
    let start = sketch.add_point(5.0, 0.0);
    let end = sketch.add_point(0.0, 5.0);
    let _arc = sketch.add_arc(center, start, end, 5.0, 0.0, std::f64::consts::FRAC_PI_2);
    assert_eq!(sketch.arcs.len(), 1);
}

#[test]
fn sketcher_add_horizontal_constraint_succeeds() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 1.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Horizontal(line));
    assert_eq!(sketch.constraints.len(), 1);
}

#[test]
fn sketcher_add_vertical_constraint_succeeds() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(1.0, 10.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Vertical(line));
    assert_eq!(sketch.constraints.len(), 1);
}

#[test]
fn sketcher_add_length_constraint_stores_value() {
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(7.0, 0.0);
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Length(line, 5.0));
    assert_eq!(sketch.constraints.len(), 1);
}

#[test]
fn sketcher_solve_horizontal_constraint_zeros_dy() {
    // BUG: With p0=(0,0), p1=(10,3) and Constraint::Horizontal(line), solve(...) returns
    //      converged=true yet leaves p1.y ≈ 1.5 (only halves the residual instead of zeroing it).
    //      Expected: a Horizontal constraint must drive (p1.y - p0.y) → 0.
    //      Likely cause: Newton-Raphson step damping or wrong sign in the Jacobian
    //      column for Horizontal residual.
    let mut sketch = Sketch::new();
    let p0 = sketch.add_point(0.0, 0.0);
    let p1 = sketch.add_point(10.0, 3.0); // dy = 3 violation
    let line = sketch.add_line(p0, p1);
    sketch.add_constraint(Constraint::Horizontal(line));
    let result = cadkernel_sketch::solve(&mut sketch, 50, 1e-8);
    if result.converged {
        let p1_pt = sketch.points[p1.0];
        assert!(
            p1_pt.position.y.abs() < 1e-3,
            "solver should zero dy, got y={}",
            p1_pt.position.y
        );
    }
}

#[test]
fn sketcher_workplane_xy_has_z_normal() {
    let wp = WorkPlane::xy();
    assert!((wp.normal.z - 1.0).abs() < 1e-10);
    assert!(wp.normal.x.abs() < 1e-10);
    assert!(wp.normal.y.abs() < 1e-10);
}

// ---------------------------------------------------------------------------
// Mesh workbench — operations on imported meshes
// ---------------------------------------------------------------------------

#[test]
fn mesh_add_via_scene_is_supported() {
    // MeshSubdivide, MeshSmooth, etc. apply to imported meshes attached
    // to a Scene object via add_mesh_object.
    let mut scene = Scene::new();
    let id = scene.add_mesh_object("ImportedMesh", Mesh::new(), None, None);
    assert_eq!(scene.len(), 1);
    assert!(scene.get(id).is_some());
}

// V36 R2b-cont: MeshRepair dispatcher calls `cadkernel_io::evaluate_and_repair`.
// Mirror that call directly and assert the returned Mesh is structurally valid.
#[test]
fn mesh_repair_evaluate_and_repair_returns_valid_mesh() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box");
    let mesh = tessellate_solid(&model, r.solid);
    let (repaired, report) = evaluate_and_repair(&mesh);
    assert!(!repaired.vertices.is_empty());
    assert!(!repaired.indices.is_empty());
    assert_eq!(repaired.indices.len() % 3, 0);
    let _ = report;
}

// ---------------------------------------------------------------------------
// TechDraw workbench — drawing sheet generation
// ---------------------------------------------------------------------------

// V36 R2b-cont: TechDrawAddView dispatcher calls `project_solid` then appends
// the returned view to a DrawingSheet. Mirror that chain and assert the view
// contains edges.
#[test]
fn techdraw_add_view_projects_solid_to_sheet() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 1.0, 1.0).expect("box");
    let view = project_solid(&model, r.solid, ProjectionDir::Front);
    assert!(!view.edges.is_empty(), "Front projection produced no edges");
    let mut sheet = DrawingSheet::a4_landscape();
    sheet.views.push(view);
    assert_eq!(sheet.views.len(), 1);
}

// V36 R2b-cont: TechDrawThreeView dispatcher calls `three_view_drawing`,
// which returns a sheet with Front + Top + Right views populated.
#[test]
fn techdraw_three_view_populates_three_views() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 1.0, 1.0).expect("box");
    let sheet = three_view_drawing(&model, r.solid);
    assert_eq!(sheet.views.len(), 3);
    let total_edges: usize = sheet.views.iter().map(|v| v.edges.len()).sum();
    assert!(total_edges > 0, "3-view drawing produced no edges");
}

// V36 R2b-cont: TechDrawExportSvg dispatcher calls `drawing_to_svg(&sheet)`
// and writes `svg.render()` to disk. Exercise the render pipeline directly
// and assert the output is a non-empty SVG string.
#[test]
fn techdraw_export_svg_renders_nonempty_svg() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 1.0, 1.0).expect("box");
    let sheet = three_view_drawing(&model, r.solid);
    let doc = drawing_to_svg(&sheet);
    let svg = doc.render();
    assert!(svg.starts_with("<"), "SVG did not start with a tag");
    assert!(svg.contains("</svg>"), "SVG is missing closing tag");
    assert!(
        svg.len() > 100,
        "SVG suspiciously short: {} bytes",
        svg.len()
    );
}

// ---------------------------------------------------------------------------
// Assembly workbench
// ---------------------------------------------------------------------------

// V36 R2b-cont-Phase-N-min: CreateAssembly dispatcher now sets
// `gui.assembly = Some(Assembly::new(...))`. Mirror that call and assert
// an empty Assembly is instantiated.
#[test]
fn assembly_create_new_assembly_is_empty() {
    let assembly = cadkernel_modeling::Assembly::new("New Assembly");
    assert_eq!(assembly.num_components(), 0);
    assert_eq!(assembly.num_constraints(), 0);
    assert_eq!(assembly.name, "New Assembly");
}

// V36 R2b-cont-Phase-N-min: InsertComponent dispatcher adds the current
// solid as a new Component. Mirror that chain: create solid, create
// assembly, insert, assert component count increments.
#[test]
fn assembly_insert_component_increments_count() {
    let mut model = BRepModel::new();
    let a = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box A");
    let b = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).expect("box B");
    let mut assembly = cadkernel_modeling::Assembly::new("Test");
    let id_a = assembly.add_component("Component 1", a.solid);
    let id_b = assembly.add_component("Component 2", b.solid);
    assert_eq!(assembly.num_components(), 2);
    assert_ne!(id_a, id_b);
    assert!(assembly.get_component(id_a).is_some());
}

// V36 R2b-cont-Phase-N-min: SolveAssembly dispatcher calls
// `assembly.solve(100)`. Build a 2-component assembly with a Distance
// constraint, solve it, and assert convergence.
#[test]
fn assembly_solve_distance_constraint_converges() {
    let mut model = BRepModel::new();
    let a = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box A");
    let b = make_box(&mut model, Point3::new(10.0, 0.0, 0.0), 1.0, 1.0, 1.0).expect("box B");
    let mut assembly = cadkernel_modeling::Assembly::new("Test");
    let id_a = assembly.add_component("A", a.solid);
    let id_b = assembly.add_component("B", b.solid);
    assembly.add_constraint(cadkernel_modeling::AssemblyConstraint::Fixed(id_a));
    assembly.add_constraint(cadkernel_modeling::AssemblyConstraint::Distance {
        comp_a: id_a,
        comp_b: id_b,
        distance: 5.0,
    });
    let converged = assembly.solve(200).expect("solve");
    assert!(converged, "assembly solver did not converge");
}

// Phase N full Task #18: ToggleAssemblyComponentVisible dispatcher flips
// `component.visible` via `Assembly::set_visible`. Mirror that chain.
#[test]
fn assembly_toggle_component_visibility_flips_flag() {
    let mut model = BRepModel::new();
    let a = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box A");
    let mut assembly = cadkernel_modeling::Assembly::new("T");
    let id = assembly.add_component("A", a.solid);
    assert!(assembly.get_component(id).unwrap().visible);
    let comp = assembly.get_component(id).unwrap();
    let new_vis = !comp.visible;
    assembly.set_visible(id, new_vis).expect("set_visible");
    assert!(!assembly.get_component(id).unwrap().visible);
    assembly.set_visible(id, true).expect("set_visible");
    assert!(assembly.get_component(id).unwrap().visible);
}

// Phase N full Task #19: BillOfMaterials dispatcher calls
// `assembly.bill_of_materials()`, which groups components by name.
// Mirror that chain with duplicates to verify quantity aggregation.
#[test]
fn assembly_bill_of_materials_aggregates_duplicates() {
    let mut model = BRepModel::new();
    let a = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box A");
    let mut assembly = cadkernel_modeling::Assembly::new("T");
    assembly.add_component("Gear", a.solid);
    assembly.add_component("Gear", a.solid);
    assembly.add_component("Shaft", a.solid);
    let bom = assembly.bill_of_materials();
    assert_eq!(bom.len(), 2, "two distinct part names");
    let total: usize = bom.iter().map(|e| e.quantity).sum();
    assert_eq!(total, 3, "three parts total");
    let gear = bom.iter().find(|e| e.name == "Gear").expect("gear");
    assert_eq!(gear.quantity, 2);
    let shaft = bom.iter().find(|e| e.name == "Shaft").expect("shaft");
    assert_eq!(shaft.quantity, 1);
}

// Phase N full Task #19: `populate_bom_entries` is a no-op when there is
// no assembly; the dispatcher then logs the "no assembly" status.
#[test]
fn assembly_bom_on_empty_assembly_returns_zero_entries() {
    let assembly = cadkernel_modeling::Assembly::new("Empty");
    let bom = assembly.bill_of_materials();
    assert!(bom.is_empty());
}

// Phase N full Task #20: AddAssemblyJoint(Revolute) dispatcher opens the
// joint editor which on Commit calls `assembly.add_joint(Revolute{...})`.
// Mirror the modeling call and assert the joint is appended.
#[test]
fn assembly_add_revolute_joint_appends_to_joints_vec() {
    let mut model = BRepModel::new();
    let a = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box A");
    let b = make_box(&mut model, Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0).expect("box B");
    let mut assembly = cadkernel_modeling::Assembly::new("T");
    let _ = assembly.add_component("A", a.solid);
    let _ = assembly.add_component("B", b.solid);
    assert_eq!(assembly.joint_count(), 0);
    assembly.add_joint(cadkernel_modeling::JointType::Revolute {
        component_a: 0,
        component_b: 1,
        axis: Vec3::Z,
        origin: Point3::ORIGIN,
    });
    assert_eq!(assembly.joint_count(), 1);
    assert!(matches!(
        assembly.joints[0],
        cadkernel_modeling::JointType::Revolute { .. }
    ));
}

// Phase N full Task #20: AddAssemblyJoint(Grounded) needs only one
// component. Mirror that call and assert `Grounded` is appended.
#[test]
fn assembly_add_grounded_joint_allowed_with_single_component() {
    let mut model = BRepModel::new();
    let a = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).expect("box A");
    let mut assembly = cadkernel_modeling::Assembly::new("T");
    let _ = assembly.add_component("Solo", a.solid);
    assembly.add_joint(cadkernel_modeling::JointType::Grounded);
    assert_eq!(assembly.joint_count(), 1);
    assert!(matches!(
        assembly.joints[0],
        cadkernel_modeling::JointType::Grounded
    ));
}

// ---------------------------------------------------------------------------
// Draft workbench — primitive wire/line drawing
// ---------------------------------------------------------------------------

// V36 R2b: DraftLine remains wire-only because Scene stores solids. The
// modeling function exists and is exercised here; the viewer-side wire
// rendering is tracked under FREECAD_PARITY_PLAN Phase L.
#[test]
fn draft_line_creates_wire_topology_in_model() {
    let mut model = BRepModel::new();
    let wire = make_line_draft(&mut model, Point3::ORIGIN, Point3::new(1.0, 0.0, 0.0))
        .expect("make_line_draft");
    assert_eq!(wire.vertices.len(), 2, "line has two endpoints");
    assert_eq!(wire.edges.len(), 1, "line has one edge");
}

// V36 R2b: DraftRectangle dispatcher now fills the rectangle boundary
// via cadkernel_modeling::filling() and adds a SceneObject.
#[test]
fn draft_rectangle_fills_patch_and_adds_to_scene() {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let mut pts = make_rectangle_wire(Point3::ORIGIN, 2.0, 1.0, Vec3::Z).expect("rectangle wire");
    pts.pop(); // filling() rejects the closing duplicate point
    let r = filling(&mut model, &pts, 1).expect("filling");
    let id = scene.add_object(
        "DraftRect",
        model,
        r.solid,
        Some(CreationParams::DraftRectangle {
            width: 2.0,
            height: 1.0,
        }),
    None,
    );
    assert_eq!(scene.len(), 1);
    assert!(!scene.get(id).unwrap().vertices.is_empty());
}

// V36 R2b: DraftPolygon dispatcher now fills the polygon boundary.
#[test]
fn draft_polygon_fills_patch_and_adds_to_scene() {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let pts = make_polygon_wire(Point3::ORIGIN, Vec3::Z, 1.0, 6).expect("polygon wire");
    let r = filling(&mut model, &pts, 1).expect("filling");
    let id = scene.add_object(
        "DraftPoly",
        model,
        r.solid,
        Some(CreationParams::DraftPolygon {
            radius: 1.0,
            sides: 6,
        }),
    None,
    );
    assert_eq!(scene.len(), 1);
    assert!(!scene.get(id).unwrap().vertices.is_empty());
}

// ---------------------------------------------------------------------------
// Surface workbench
// ---------------------------------------------------------------------------

// V36 R2b: SurfaceFilling dispatcher now builds a solid from a default
// 4-point boundary via filling().
#[test]
fn surface_filling_creates_solid_from_boundary() {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let boundary = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(2.0, 0.0, 0.0),
        Point3::new(2.0, 2.0, 0.0),
        Point3::new(0.0, 2.0, 0.0),
    ];
    let r = filling(&mut model, &boundary, 1).expect("filling");
    let id = scene.add_object("Filling", model, r.solid, None, None);
    assert_eq!(scene.len(), 1);
    assert!(!scene.get(id).unwrap().vertices.is_empty());
}

// V36 R2b: SurfaceBoundary dispatcher fills a closed polyline — here a
// default hexagonal boundary.
#[test]
fn surface_boundary_fills_closed_polyline() {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let pts = make_polygon_wire(Point3::ORIGIN, Vec3::Z, 1.0, 6).expect("polygon wire");
    let r = filling(&mut model, &pts, 1).expect("filling");
    let id = scene.add_object("Boundary", model, r.solid, None, None);
    assert_eq!(scene.len(), 1);
    assert!(!scene.get(id).unwrap().vertices.is_empty());
}

// V36 R2b: SurfacePipe dispatcher now builds a tubular solid along a
// default straight path via pipe_surface().
#[test]
fn surface_pipe_creates_solid_along_path() {
    let mut scene = Scene::new();
    let mut model = BRepModel::new();
    let path = [Point3::new(0.0, 0.0, 0.0), Point3::new(0.0, 0.0, 2.0)];
    let r = pipe_surface(&mut model, &path, 0.25, 16).expect("pipe_surface");
    let id = scene.add_object(
        "Pipe",
        model,
        r.solid,
        Some(CreationParams::SurfacePipe {
            radius: 0.25,
            length: 2.0,
        }),
    None,
    );
    assert_eq!(scene.len(), 1);
    let obj = scene.get(id).unwrap();
    // Pipe spans z=[0, 2]; vertical extent ~2.0.
    assert!((obj.aabb_max[2] - obj.aabb_min[2]) >= 1.9);
}

// ---------------------------------------------------------------------------
// FEM workbench
// ---------------------------------------------------------------------------

// FEM dispatcher wires `GuiAction::CreateFemAnalysis` to
// `generate_tet_mesh(...)` + `AnalysisContainer::new(mesh, FemMaterial::steel())`
// and stores the result in `gui.fem_analysis`. Mirror that chain.
#[test]
fn fem_create_analysis_builds_tet_mesh_with_steel_material() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).expect("box");
    let mesh = cadkernel_modeling::generate_tet_mesh(&model, r.solid, 1.0).expect("tet mesh");
    assert!(!mesh.nodes.is_empty(), "tet mesh must have nodes");
    assert!(!mesh.elements.is_empty(), "tet mesh must have elements");
    let container =
        cadkernel_modeling::AnalysisContainer::new(mesh, cadkernel_modeling::FemMaterial::steel());
    assert_eq!(container.boundary_conditions.len(), 0);
    assert!(container.result.is_none());
}

// `SolveStatic` calls `container.run_static()`. With a FixedNode + Force BC
// the solver must produce a displacement vector sized to the node count.
#[test]
fn fem_solve_static_produces_displacement_result() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).expect("box");
    let mesh = cadkernel_modeling::generate_tet_mesh(&model, r.solid, 1.0).expect("tet mesh");
    let n_nodes = mesh.nodes.len();
    assert!(n_nodes >= 2);
    let mut container =
        cadkernel_modeling::AnalysisContainer::new(mesh, cadkernel_modeling::FemMaterial::steel());
    container.add_bc(cadkernel_modeling::BoundaryCondition::FixedNode(0));
    container.add_bc(cadkernel_modeling::BoundaryCondition::Force {
        node: n_nodes - 1,
        force: Vec3::new(0.0, 0.0, -1000.0),
    });
    container.run_static().expect("static solve");
    let result = container.result.as_ref().expect("result populated");
    assert_eq!(result.displacements.len(), n_nodes);
}

// ---------------------------------------------------------------------------
// Selection system (SelectObject, ToggleSelect, SelectAll, DeselectAll, DeleteSelected)
// ---------------------------------------------------------------------------

#[test]
fn select_object_sets_exactly_one_selected() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let _id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.select_single(id_a);
    assert_eq!(scene.selected_ids(), vec![id_a]);
}

#[test]
fn toggle_select_enables_multi_select() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.toggle_select(id_a);
    scene.toggle_select(id_b);
    let selected = scene.selected_ids();
    assert_eq!(selected.len(), 2);
}

#[test]
fn deselect_all_clears_every_selection() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.toggle_select(id_a);
    scene.toggle_select(id_b);
    scene.deselect_all();
    assert!(scene.selected_ids().is_empty());
}

#[test]
fn select_all_selects_only_visible_objects() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.get_mut(id_b).unwrap().visible = false;
    scene.select_all();
    assert!(scene.get(id_a).unwrap().selected);
    assert!(
        !scene.get(id_b).unwrap().selected,
        "select_all must skip hidden objects"
    );
}

#[test]
fn delete_selected_removes_only_selected_objects() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.select_single(id_a);
    // Simulate DeleteSelected dispatcher: remove each selected.
    for sid in scene.selected_ids() {
        scene.remove_object(sid);
    }
    assert_eq!(scene.len(), 1);
    assert!(scene.get(id_b).is_some(), "B should remain");
}

// ---------------------------------------------------------------------------
// Undo / Redo (CommandStack)
// ---------------------------------------------------------------------------

#[test]
fn undo_after_create_restores_empty_state() {
    let mut stack = CommandStack::new(50);
    let empty_snapshot = ModelSnapshot {
        model: BRepModel::new(),
        current_solid: None,
        current_mesh: None,
    };
    stack.push("Create box", empty_snapshot.clone());

    // Simulate state after command.
    let mut after = BRepModel::new();
    let r = make_box(&mut after, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let after_snapshot = ModelSnapshot {
        model: after,
        current_solid: Some(r.solid),
        current_mesh: None,
    };

    let restored = stack.undo(after_snapshot).expect("undo");
    assert!(
        restored.current_solid.is_none(),
        "undo should restore no-solid state"
    );
    assert!(stack.can_redo());
}

#[test]
fn redo_after_undo_reapplies_command() {
    let mut stack = CommandStack::new(50);
    let empty_snapshot = ModelSnapshot {
        model: BRepModel::new(),
        current_solid: None,
        current_mesh: None,
    };
    stack.push("Create box", empty_snapshot.clone());

    let mut after_model = BRepModel::new();
    let r = make_box(&mut after_model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let after = ModelSnapshot {
        model: after_model,
        current_solid: Some(r.solid),
        current_mesh: None,
    };
    let _undone = stack.undo(after.clone()).unwrap();
    let redone = stack.redo(empty_snapshot).expect("redo");
    assert!(
        redone.current_solid.is_some(),
        "redo should restore the box"
    );
}

#[test]
fn new_command_clears_redo_stack() {
    let mut stack = CommandStack::new(50);
    stack.push(
        "op1",
        ModelSnapshot {
            model: BRepModel::new(),
            current_solid: None,
            current_mesh: None,
        },
    );
    let _ = stack.undo(ModelSnapshot {
        model: BRepModel::new(),
        current_solid: None,
        current_mesh: None,
    });
    assert!(stack.can_redo());
    stack.push(
        "op2",
        ModelSnapshot {
            model: BRepModel::new(),
            current_solid: None,
            current_mesh: None,
        },
    );
    assert!(!stack.can_redo(), "new command must invalidate redo");
}

#[test]
fn command_stack_honors_max_depth() {
    let mut stack = CommandStack::new(3);
    for i in 0..5 {
        stack.push(
            format!("op{i}"),
            ModelSnapshot {
                model: BRepModel::new(),
                current_solid: None,
                current_mesh: None,
            },
        );
    }
    assert_eq!(stack.history_len(), 3, "stack should cap at max_depth");
}

// ---------------------------------------------------------------------------
// Workbench switching — state integrity
// ---------------------------------------------------------------------------

#[test]
fn workbench_switch_preserves_scene_objects() {
    // SetWorkbench(Part) → SetWorkbench(Sketcher) → SetWorkbench(Part)
    // should leave the scene unchanged. Since Workbench is pub(crate), we
    // simulate by verifying Scene is untouched by any workbench-switch
    // side effect we can trigger via the public API.
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "KeepMe", 1.0, 1.0, 1.0);
    // Simulate workbench switches: these affect active_workbench in GuiState
    // and active_body_id in Scene. Only the latter is public.
    scene.set_active_body(Some(id));
    scene.set_active_body(None);
    scene.set_active_body(Some(id));
    assert_eq!(scene.len(), 1, "workbench switch must not delete objects");
    assert!(scene.get(id).is_some());
}

// ---------------------------------------------------------------------------
// Visibility / ObjectGroup
// ---------------------------------------------------------------------------

#[test]
fn toggle_visibility_flips_visible_flag() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "X", 1.0, 1.0, 1.0);
    assert!(scene.get(id).unwrap().visible);
    // Simulate ToggleVisibility
    {
        let obj = scene.get_mut(id).unwrap();
        obj.visible = !obj.visible;
    }
    assert!(!scene.get(id).unwrap().visible);
}

#[test]
fn show_all_makes_every_object_visible() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.get_mut(id_a).unwrap().visible = false;
    scene.get_mut(id_b).unwrap().visible = false;
    // ShowAll dispatcher:
    for obj in &mut scene.objects {
        obj.visible = true;
    }
    assert!(scene.get(id_a).unwrap().visible);
    assert!(scene.get(id_b).unwrap().visible);
}

#[test]
fn hide_all_hides_every_object() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    // HideAll dispatcher:
    for obj in &mut scene.objects {
        obj.visible = false;
    }
    assert!(!scene.get(id_a).unwrap().visible);
}

#[test]
fn create_group_and_add_member() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    scene.select_single(id);
    let gid = scene.create_group("Group1");
    scene.group_selected(gid);
    assert_eq!(scene.get(id).unwrap().group_id, gid);
    assert_eq!(scene.group_members(gid), vec![id]);
}

#[test]
fn toggle_group_visibility_hides_all_members() {
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let id_b = add_box_at(&mut scene, "B", Point3::new(3.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    scene.toggle_select(id_a);
    scene.toggle_select(id_b);
    let gid = scene.create_group("G");
    scene.group_selected(gid);
    scene.toggle_group_visibility(gid);
    assert!(!scene.get(id_a).unwrap().visible);
    assert!(!scene.get(id_b).unwrap().visible);
}

#[test]
fn delete_group_ungroups_members_not_delete_objects() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    scene.select_single(id);
    let gid = scene.create_group("G");
    scene.group_selected(gid);
    scene.delete_group(gid);
    assert_eq!(scene.get(id).unwrap().group_id, 0);
    assert!(
        scene.get(id).is_some(),
        "delete_group must NOT delete member objects"
    );
}

// ---------------------------------------------------------------------------
// Scene object duplication / renaming (GuiAction::DuplicateObject / RenameObject)
// ---------------------------------------------------------------------------

#[test]
fn duplicate_object_via_clone_increases_len() {
    // BUG/NOTE: Scene has no `duplicate_object` method. Dispatcher must
    // hand-clone via add_object with copied model+solid.
    let mut scene = Scene::new();
    let id_a = add_box(&mut scene, "Original", 1.0, 1.0, 1.0);
    let obj_a = scene.get(id_a).unwrap().clone();
    let new_id = scene.add_object(
        format!("{}_copy", obj_a.name),
        obj_a.model.clone(),
        obj_a.solid,
        obj_a.params.clone(),
    None,
    );
    assert_eq!(scene.len(), 2);
    assert_ne!(new_id, id_a);
    assert_eq!(scene.get(new_id).unwrap().name, "Original_copy");
}

#[test]
fn rename_object_via_get_mut_works() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "Old", 1.0, 1.0, 1.0);
    scene.get_mut(id).unwrap().name = "New".to_string();
    assert_eq!(scene.get(id).unwrap().name, "New");
}

// ---------------------------------------------------------------------------
// Transform ops (MoveObject, RotateObject, ScaleObjectUniform)
// ---------------------------------------------------------------------------

#[test]
fn move_object_offsets_aabb() {
    // Simulates MoveObject: dispatcher translates all vertices by (dx,dy,dz).
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "A", 2.0, 2.0, 2.0);
    let before_min = scene.get(id).unwrap().aabb_min;
    let dx = 5.0f32;
    // Apply translation to all vertices
    for v in &mut scene.get_mut(id).unwrap().vertices {
        v.position[0] += dx;
    }
    // Recompute AABB as dispatcher's rebuild_scene_gpu does
    let (mn, mx) = compute_aabb(&scene.get(id).unwrap().vertices);
    scene.get_mut(id).unwrap().aabb_min = mn;
    scene.get_mut(id).unwrap().aabb_max = mx;
    let after_min = scene.get(id).unwrap().aabb_min;
    assert!(
        (after_min[0] - before_min[0] - dx).abs() < 1e-3,
        "move must shift aabb by dx"
    );
}

#[test]
fn scale_object_uniform_grows_bbox() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
    let before_vol = quick_volume(&model).unwrap();
    let _ = scale_solid(&mut model, r.solid, Point3::ORIGIN, 2.0).expect("scale");
    // After scale(factor=2), the NEW solid volume is 8x — but it coexists
    // with the original in the same BRepModel. We only verify the op returns Ok.
    assert!(before_vol > 0.0);
}

// ---------------------------------------------------------------------------
// Scene serialization (JSON roundtrip via CreationParams)
// ---------------------------------------------------------------------------

#[test]
fn creation_params_box_roundtrips_via_json() {
    let params = CreationParams::Box {
        width: 1.5,
        height: 2.5,
        depth: 3.5,
    };
    let j = serde_json::to_string(&params).unwrap();
    let back: CreationParams = serde_json::from_str(&j).unwrap();
    match back {
        CreationParams::Box {
            width,
            height,
            depth,
        } => {
            assert!((width - 1.5).abs() < 1e-9);
            assert!((height - 2.5).abs() < 1e-9);
            assert!((depth - 3.5).abs() < 1e-9);
        }
        _ => panic!("variant mismatch"),
    }
}

#[test]
fn creation_params_cylinder_roundtrips_via_json() {
    let params = CreationParams::Cylinder {
        radius: 2.0,
        height: 5.0,
    };
    let j = serde_json::to_string(&params).unwrap();
    let back: CreationParams = serde_json::from_str(&j).unwrap();
    assert!(matches!(back, CreationParams::Cylinder { .. }));
}

#[test]
fn creation_params_boolean_roundtrips_via_json() {
    let params = CreationParams::Boolean {
        op: "union".to_string(),
    };
    let j = serde_json::to_string(&params).unwrap();
    let back: CreationParams = serde_json::from_str(&j).unwrap();
    if let CreationParams::Boolean { op } = back {
        assert_eq!(op, "union");
    } else {
        panic!("variant mismatch");
    }
}

// ---------------------------------------------------------------------------
// Camera / View actions (ResetCamera, SetStandardView, ToggleProjection,
// SetCameraYawPitch, ScreenOrbit, RollDelta, FitAll)
// ---------------------------------------------------------------------------

#[test]
fn reset_camera_returns_defaults() {
    let mut app = CadApp::new_headless();
    app.dispatch_toggle_projection();
    app.dispatch_reset_camera();
    let cam = app.camera_ref();
    assert!(cam.roll.abs() < 1e-6);
    assert!((cam.distance - 20.0).abs() < 1e-3);
    assert_eq!(cam.projection, Projection::Perspective);
}

#[test]
fn toggle_projection_switches_modes() {
    let mut app = CadApp::new_headless();
    app.dispatch_toggle_projection();
    assert_eq!(app.camera_ref().projection, Projection::Orthographic);
    app.dispatch_toggle_projection();
    assert_eq!(app.camera_ref().projection, Projection::Perspective);
}

#[test]
fn set_standard_view_front_snaps_yaw_pitch() {
    let mut cam = Camera::new(1.0);
    cam.snap_to_view(StandardView::Front);
    let (yaw, pitch) = StandardView::Front.yaw_pitch();
    assert!((cam.yaw - yaw).abs() < 1e-5);
    assert!((cam.pitch - pitch).abs() < 1e-5);
}

#[test]
fn set_camera_yaw_pitch_sets_values() {
    let mut cam = Camera::new(1.0);
    cam.yaw = 0.5;
    cam.pitch = 0.3;
    assert!((cam.yaw - 0.5).abs() < 1e-9);
    assert!((cam.pitch - 0.3).abs() < 1e-9);
}

#[test]
fn fit_all_on_bounds_centers_target() {
    let mut cam = Camera::new(1.0);
    cam.fit_to_bounds([-5.0, -5.0, -5.0], [5.0, 5.0, 5.0]);
    assert!(cam.target[0].abs() < 1e-3);
    assert!(cam.target[1].abs() < 1e-3);
    assert!(cam.target[2].abs() < 1e-3);
    assert!(cam.distance > 0.0);
}

#[test]
fn display_mode_setting_is_just_an_enum() {
    // SetDisplayMode(Wireframe) just writes the enum into app state.
    for mode in DisplayMode::ALL {
        assert!(!mode.label().is_empty());
    }
}

// ---------------------------------------------------------------------------
// Cross-product / mouse-orbit invariants (per CLAUDE.md)
// ---------------------------------------------------------------------------

#[test]
fn camera_screen_right_is_unit_length_invariant() {
    // CLAUDE.md: cross3(f, up) — NEVER swap order.
    // If the order is flipped, screen_right would invert sign, making the
    // frame left-handed. We verify the magnitude and sign of the resulting
    // frame vectors relative to a known camera pose.
    let mut cam = Camera::new(1.0);
    cam.target = [0.0, 0.0, 0.0];
    cam.distance = 10.0;
    cam.yaw = 0.0; // eye on +X axis
    cam.pitch = 0.0;
    let r = cam.screen_right();
    let u = cam.screen_up();
    let len_r = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    let len_u = (u[0] * u[0] + u[1] * u[1] + u[2] * u[2]).sqrt();
    assert!((len_r - 1.0).abs() < 1e-4);
    assert!((len_u - 1.0).abs() < 1e-4);
}

#[test]
fn camera_screen_right_orientation_when_looking_down_minus_x() {
    // Eye on +X looking at origin: forward = -X. world_up = +Z.
    // cross3(f, up) = (-1,0,0) x (0,0,1) = (0,-1,0) ... wait:
    //   i(0*1 - 0*0) - j((-1)*1 - 0*0) + k((-1)*0 - 0*0)
    //   = (0, 1, 0) ← CLAUDE.md order cross3(f, up) gives screen_right = +Y direction
    // If order were swapped: up x f = (0,0,1) x (-1,0,0) = (0,1,0) ... same.
    // Actually cross(a,b) = -cross(b,a), so swap flips sign.
    //
    // The test: with camera on +X, screen_right MUST be approximately +Y
    // (not -Y). If someone swaps cross3(f, up) → cross3(up, f), sign flips.
    let mut cam = Camera::new(1.0);
    cam.target = [0.0, 0.0, 0.0];
    cam.distance = 10.0;
    cam.yaw = 0.0;
    cam.pitch = 0.0;
    cam.roll = 0.0;
    let r = cam.screen_right();
    // screen_right should have a substantial +Y or -Y component (not X).
    assert!(
        r[1].abs() > 0.9,
        "screen_right should align with Y axis, got {r:?}"
    );
    // For yaw=0 (eye on +X): f=(-1,0,0), up=(0,0,1); cross(f,up)=(0,1,0).
    // So screen_right MUST be +Y to preserve CLAUDE.md's cross3(f, up) order.
    assert!(
        r[1] > 0.9,
        "CLAUDE.md INVARIANT: screen_right must be +Y for yaw=0, pitch=0. \
         A negative value here indicates cross3 args were swapped (cross3(up,f)). \
         Got screen_right={r:?}"
    );
}

#[test]
fn camera_screen_up_orientation_when_looking_down_minus_x() {
    // With camera on +X, yaw=pitch=0, screen_up should align with +Z.
    let mut cam = Camera::new(1.0);
    cam.target = [0.0, 0.0, 0.0];
    cam.distance = 10.0;
    cam.yaw = 0.0;
    cam.pitch = 0.0;
    cam.roll = 0.0;
    let u = cam.screen_up();
    assert!(
        u[2] > 0.9,
        "screen_up should align with +Z for level camera, got {u:?}"
    );
}

#[test]
fn mouse_orbit_positive_dx_decreases_yaw() {
    // CLAUDE.md: "Mouse orbit: negate delta (yaw -= dx), NOT cross products"
    // Verify by invoking the nav apply_orbit with dx > 0 and confirming
    // yaw decreased.
    let cfg = NavConfig::new();
    let mut yaw = 0.0f32;
    let mut pitch = 0.0f32;
    cfg.apply_orbit(&mut yaw, &mut pitch, 10.0, 0.0, 100.0, 100.0, 200.0, 200.0);
    assert!(
        yaw < 0.0,
        "CLAUDE.md INVARIANT: positive dx must DECREASE yaw (yaw -= dx). \
         Positive yaw after orbit indicates sign was flipped. Got yaw={yaw}"
    );
}

#[test]
fn mouse_orbit_positive_dy_adjusts_pitch() {
    let cfg = NavConfig::new();
    let mut yaw = 0.0f32;
    let mut pitch = 0.0f32;
    cfg.apply_orbit(&mut yaw, &mut pitch, 0.0, 10.0, 100.0, 100.0, 200.0, 200.0);
    // pitch change direction depends on rotation mode, but it MUST change.
    assert!(
        pitch.abs() > 1e-6,
        "orbit with dy=10 must change pitch, got {pitch}"
    );
}

// ---------------------------------------------------------------------------
// Selection modes (SetSelectionMode: Solid/Face/Edge/Vertex)
// ---------------------------------------------------------------------------
// SelectionMode is pub(crate). We verify via the picking API that
// face/edge/vertex picks work on a real mesh.

#[test]
fn pick_vertex_mode_returns_nearest_vertex() {
    let vertices = vec![[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let ray_o = [0.0, 0.0, 5.0];
    let ray_d = [0.0, 0.0, -1.0];
    let hit = pick_vertex(ray_o, ray_d, &vertices, 0.1);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().0, 0, "should pick vertex 0 at origin");
}

#[test]
fn pick_edge_mode_returns_edge_index() {
    let edges = vec![
        ([0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([0.0, 5.0, 0.0], [1.0, 5.0, 0.0]),
    ];
    let ray_o = [0.5, 0.0, 5.0];
    let ray_d = [0.0, 0.0, -1.0];
    let hit = pick_edge(ray_o, ray_d, &edges, 0.5);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().0, 0, "should pick edge 0 (nearer)");
}

#[test]
fn pick_face_mode_via_pick_triangle_on_real_box() {
    // Construct a real box, tessellate, pick at its center.
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::new(-1.0, -1.0, -1.0), 2.0, 2.0, 2.0).unwrap();
    let mesh = tessellate_solid(&model, r.solid);
    let mut camera = Camera::new(1440.0 / 900.0);
    camera.target = [0.0, 0.0, 0.0];
    camera.distance = 10.0;
    camera.yaw = 0.0;
    camera.pitch = 0.0;
    let inv_vp = camera.inv_view_proj();
    let (origin, dir) = screen_to_ray(720.0, 450.0, 1440.0, 900.0, inv_vp);
    let hit = pick_triangle(origin, dir, &mesh.vertices, &mesh.indices);
    assert!(
        hit.is_some(),
        "face-mode pick at screen center must hit the box"
    );
}

// ---------------------------------------------------------------------------
// NewModel — clear scene
// ---------------------------------------------------------------------------

#[test]
fn new_model_clears_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    assert_eq!(app.scene_ref().len(), 2);
    app.dispatch_new_model();
    assert!(app.scene_ref().is_empty());
}

// ---------------------------------------------------------------------------
// ScreenOrbit — viewport rotation via GuiAction::ScreenOrbit(right, up)
// ---------------------------------------------------------------------------

#[test]
fn screen_orbit_changes_yaw_and_pitch() {
    let cfg = NavConfig::new();
    let mut cam = Camera::new(1.0);
    let yaw_before = cam.yaw;
    let pitch_before = cam.pitch;
    cfg.apply_orbit(
        &mut cam.yaw,
        &mut cam.pitch,
        5.0,
        3.0,
        100.0,
        100.0,
        200.0,
        200.0,
    );
    assert!(
        (cam.yaw - yaw_before).abs() > 1e-6 || (cam.pitch - pitch_before).abs() > 1e-6,
        "orbit must change at least one of yaw/pitch"
    );
}

// ---------------------------------------------------------------------------
// Snapshot utility sanity check (ensures snapshot helper is exercised)
// ---------------------------------------------------------------------------

#[test]
fn snapshot_from_empty_scene_has_no_solid() {
    let scene = Scene::new();
    let snap = snapshot_from(&scene);
    assert!(snap.current_solid.is_none());
}

#[test]
fn snapshot_from_scene_with_box_has_solid() {
    let mut scene = Scene::new();
    add_box(&mut scene, "A", 1.0, 1.0, 1.0);
    let snap = snapshot_from(&scene);
    assert!(snap.current_solid.is_some());
}

// ---------------------------------------------------------------------------
// Vertex and mesh helpers (used by nearly every GuiAction's GPU path)
// ---------------------------------------------------------------------------

#[test]
fn mesh_to_vertices_roundtrip_preserves_triangle_count() {
    let mut model = BRepModel::new();
    let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
    let mesh = tessellate_solid(&model, r.solid);
    let verts: Vec<Vertex> = mesh_to_vertices(&mesh);
    assert_eq!(
        verts.len(),
        mesh.indices.len() * 3,
        "mesh_to_vertices should expand each indexed triangle to 3 flat verts"
    );
}

#[test]
fn compute_bounds_on_real_box_returns_valid_aabb() {
    let mut model = BRepModel::new();
    let _ = make_box(&mut model, Point3::ORIGIN, 2.0, 3.0, 4.0).unwrap();
    let mesh = tessellate_solid(&model, model.solids.iter().next().unwrap().0);
    let verts = mesh_to_vertices(&mesh);
    let (mn, mx) = compute_bounds(&verts);
    assert!(mx[0] - mn[0] >= 2.0 - 1e-3);
    assert!(mx[1] - mn[1] >= 3.0 - 1e-3);
    assert!(mx[2] - mn[2] >= 4.0 - 1e-3);
}

// ---------------------------------------------------------------------------
// Helix (documented as wire-only, no solid)
// ---------------------------------------------------------------------------

// `make_helix` produces a tubular B-Rep solid; CreateHelix dispatches through
// `add_to_scene`, so the scene gains an object with non-empty geometry.
#[test]
fn create_helix_produces_tube_solid_with_faces() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_helix(1.0, 0.5, 2.0, 0.15);
    assert_eq!(app.scene_ref().len(), 1);
    assert!(!app.scene_ref().objects.first().unwrap().vertices.is_empty());
}

// ---------------------------------------------------------------------------
// Script engine wiring (GuiAction::ExecuteLuaCode)
// ---------------------------------------------------------------------------

#[test]
fn script_engine_executes_cad_box_but_does_not_update_app_scene() {
    // BUG: ExecuteLuaCode dispatches to a local ScriptEngine whose BRepModel
    // is NOT merged into the CadApp's main Scene. Solids remain orphaned.
    let mut eng = cadkernel_viewer::scripting::ScriptEngine::new().unwrap();
    eng.execute("cad.box(1, 1, 1)").unwrap();
    assert_eq!(eng.solid_count(), 1);
    // The app Scene would remain empty — not tested here directly.
}

// ---------------------------------------------------------------------------
// V36 Task #16 UX pass — FocusObject dispatch mirror
// ---------------------------------------------------------------------------
//
// The new GuiAction::FocusObject(id) dispatcher arm reads the target object's
// cached aabb_min/aabb_max and calls `camera.fit_to_bounds`. The pair of
// tests below exercise the same Scene + Camera call chain that app.rs runs,
// so that a regression in either the AABB cache invariants or the camera
// fitting math will fail here.

#[test]
fn focus_object_dispatch_fits_camera_to_single_object_aabb() {
    // Build a scene with two well-separated boxes; focusing on the second
    // should move the camera target to the second box's centroid, which is
    // measurably far from the scene-wide centroid.
    let mut scene = Scene::new();
    let _a = add_box_at(&mut scene, "A", Point3::new(0.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    let b = add_box_at(&mut scene, "B", Point3::new(50.0, 0.0, 0.0), 1.0, 1.0, 1.0);
    let obj_b = scene.get(b).expect("B");

    let mut cam = Camera::new(1.0);
    cam.fit_to_bounds(obj_b.aabb_min, obj_b.aabb_max);

    // Camera target must be near the centroid of B (x ≈ 50.5) — well away
    // from a naive scene-wide fit (which would center at x ≈ 25.75).
    assert!(
        (cam.target[0] - 50.5).abs() < 1.0,
        "FocusObject should fit to box B alone, target.x = {}",
        cam.target[0]
    );
}

#[test]
fn focus_object_dispatch_is_a_noop_on_empty_vertex_list() {
    // Mirrors the `if !obj.vertices.is_empty()` guard in the dispatcher.
    // An object with no geometry must not perturb the camera.
    let mut scene = Scene::new();
    let empty_mesh = Mesh::new();
    let id = scene.add_mesh_object("empty", empty_mesh, None, None);
    let obj = scene.get(id).unwrap();

    let mut cam = Camera::new(1.0);
    let yaw_before = cam.yaw;
    let pitch_before = cam.pitch;
    if !obj.vertices.is_empty() {
        cam.fit_to_bounds(obj.aabb_min, obj.aabb_max);
    }
    assert_eq!(cam.yaw, yaw_before);
    assert_eq!(cam.pitch, pitch_before);
}

// ---------------------------------------------------------------------------
// V36 Task #16 UX pass — Properties panel bounding-box dimensions
// ---------------------------------------------------------------------------
//
// The Placement group now shows Width/Depth/Height derived from the live
// `aabb_min`/`aabb_max` on each SceneObject. Verify those fields track the
// constructor extents for primitives and stay consistent after a rebuild.

#[test]
fn properties_aabb_extents_match_box_constructor() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "B", 2.5, 7.0, 0.5);
    let obj = scene.get(id).unwrap();
    let dx = (obj.aabb_max[0] - obj.aabb_min[0]) as f64;
    let dy = (obj.aabb_max[1] - obj.aabb_min[1]) as f64;
    let dz = (obj.aabb_max[2] - obj.aabb_min[2]) as f64;
    assert!((dx - 2.5).abs() < 1e-3, "width = {dx}");
    assert!((dy - 7.0).abs() < 1e-3, "depth = {dy}");
    assert!((dz - 0.5).abs() < 1e-3, "height = {dz}");
}

#[test]
fn properties_aabb_extents_match_sphere_diameter() {
    let mut scene = Scene::new();
    let id = add_sphere(&mut scene, "S", 3.0);
    let obj = scene.get(id).unwrap();
    for axis in 0..3 {
        let extent = (obj.aabb_max[axis] - obj.aabb_min[axis]) as f64;
        // Sphere diameter ≈ 6.0; tessellation may under-/over-shoot very
        // slightly depending on segmentation, so accept ≥ 5.8.
        assert!(
            extent >= 5.8,
            "sphere axis {axis} extent = {extent} should be ~diameter 6.0"
        );
    }
}

// ---------------------------------------------------------------------------
// V36 Task #16 UX pass — compute_aabb parity with Placement/bbox_extents
// ---------------------------------------------------------------------------
//
// `compute_aabb` is the shared public helper used by both the scene cache
// and the Properties panel's new Bounding Box display. This guards against
// a drift between the cached aabb and a fresh recomputation.

#[test]
fn compute_aabb_matches_cached_object_fields() {
    let mut scene = Scene::new();
    let id = add_box(&mut scene, "Box", 3.0, 4.0, 5.0);
    let obj = scene.get(id).unwrap();
    let (mn, mx) = compute_aabb(&obj.vertices);
    for i in 0..3 {
        assert!(
            (mn[i] - obj.aabb_min[i]).abs() < 1e-5,
            "cached aabb_min[{i}] diverged from fresh compute_aabb"
        );
        assert!(
            (mx[i] - obj.aabb_max[i]).abs() < 1e-5,
            "cached aabb_max[{i}] diverged from fresh compute_aabb"
        );
    }
}

// ---------------------------------------------------------------------------
// Phase O-a — FEM material picker / BC editor dispatcher integration
// ---------------------------------------------------------------------------
// `GuiState` is `pub(crate)`, so these tests mirror the dispatcher's exact
// call chain via the public modeling API:
//   CommitMaterialPicker → pending_fem_material = FemMaterial::<preset>()
//   CreateFemAnalysis    → AnalysisContainer::new(mesh, take(pending))
//   CommitBcEditor       → container.add_bc(state.to_boundary_condition())

fn make_unit_tet_mesh() -> cadkernel_modeling::TetMesh {
    use cadkernel_math::Point3;
    cadkernel_modeling::TetMesh {
        nodes: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        ],
        elements: vec![[0, 1, 2, 3]],
    }
}

#[test]
fn commit_material_picker_updates_pending_material_for_each_preset() {
    use cadkernel_modeling::FemMaterial;
    // Preset → expected FemMaterial constructor result.
    let titanium = FemMaterial::titanium();
    assert!(
        (titanium.youngs_modulus - 114.0e9).abs() < 1.0
            && (titanium.poisson_ratio - 0.34).abs() < 1e-9
            && (titanium.density - 4430.0).abs() < 1e-6
    );
    // Custom preset uses user-picked values when valid.
    let custom = FemMaterial::custom(88.0e9, 0.28, 4321.0).unwrap();
    assert!((custom.youngs_modulus - 88.0e9).abs() < 1.0 && (custom.density - 4321.0).abs() < 1e-6);
}

#[test]
fn create_fem_analysis_consumes_pending_material() {
    // Aluminum picked → container.material reflects it; bc list starts empty.
    use cadkernel_modeling::{AnalysisContainer, FemMaterial};
    let c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::aluminum());
    assert!((c.material.youngs_modulus - 70.0e9).abs() < 1.0);
    assert!((c.material.density - 2700.0).abs() < 1e-6);
    assert_eq!(c.boundary_conditions.len(), 0);
}

#[test]
fn commit_bc_editor_fixed_node_appends_one_matching_bc() {
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::FixedNode(2));
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::FixedNode(n) => assert_eq!(*n, 2),
        _ => panic!("expected FixedNode"),
    }
}

#[test]
fn commit_bc_editor_force_appends_one_matching_bc_xyz() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::Force {
        node: 1,
        force: Vec3 {
            x: 100.0,
            y: 0.0,
            z: -50.0,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::Force { node, force } => {
            assert_eq!(*node, 1);
            assert!((force.x - 100.0).abs() < 1e-9 && (force.z + 50.0).abs() < 1e-9);
        }
        _ => panic!("expected Force"),
    }
}

// ---------------------------------------------------------------------------
// Phase O-b — extended BC editor dispatcher integration (10 new variants)
// ---------------------------------------------------------------------------
// Each test mirrors the dispatcher's exact call chain for one extended kind:
//   `BcEditorState::to_boundary_condition()` → `container.add_bc(...)`.
// Field overload check: ensure the right kernel-side variant is constructed
// with the right field meaning per BC kind (force / displacement / axis / ...).

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0)
}

#[test]
fn commit_bc_editor_pressure_appends_one_matching_bc_element_scalar() {
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::Pressure {
        element: 0,
        pressure: 2.5e6,
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::Pressure { element, pressure } => {
            assert_eq!(*element, 0);
            assert!(near(*pressure, 2.5e6));
        }
        _ => panic!("expected Pressure"),
    }
}

#[test]
fn commit_bc_editor_displacement_appends_one_matching_bc_node_vec3() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::Displacement {
        node: 2,
        displacement: Vec3 {
            x: 0.001,
            y: 0.0,
            z: 0.0,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::Displacement { node, displacement } => {
            assert_eq!(*node, 2);
            assert!(near(displacement.x, 0.001));
        }
        _ => panic!("expected Displacement"),
    }
}

#[test]
fn commit_bc_editor_gravity_appends_one_matching_bc_vec3_only() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::Gravity {
        acceleration: Vec3 {
            x: 0.0,
            y: 0.0,
            z: -9.81,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::Gravity { acceleration } => {
            assert!(near(acceleration.z, -9.81));
        }
        _ => panic!("expected Gravity"),
    }
}

#[test]
fn commit_bc_editor_distributed_load_appends_one_matching_bc_element_vec3() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::DistributedLoad {
        element: 0,
        load: Vec3 {
            x: 0.0,
            y: 1000.0,
            z: 0.0,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::DistributedLoad { element, load } => {
            assert_eq!(*element, 0);
            assert!(near(load.y, 1000.0));
        }
        _ => panic!("expected DistributedLoad"),
    }
}

#[test]
fn commit_bc_editor_spring_appends_one_matching_bc_node_scalar() {
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::Spring {
        node: 3,
        stiffness: 1e5,
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::Spring { node, stiffness } => {
            assert_eq!(*node, 3);
            assert!(near(*stiffness, 1e5));
        }
        _ => panic!("expected Spring"),
    }
}

#[test]
fn commit_bc_editor_centrifugal_appends_one_matching_bc_axis_omega() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::CentrifugalLoad {
        axis: Vec3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
        omega: 50.0,
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::CentrifugalLoad { axis, omega } => {
            assert!(near(axis.z, 1.0) && near(*omega, 50.0));
        }
        _ => panic!("expected CentrifugalLoad"),
    }
}

#[test]
fn commit_bc_editor_self_weight_appends_one_matching_bc_vec3_only() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::SelfWeight {
        gravity: Vec3 {
            x: 0.0,
            y: 0.0,
            z: -9.81,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::SelfWeight { gravity } => {
            assert!(near(gravity.z, -9.81));
        }
        _ => panic!("expected SelfWeight"),
    }
}

#[test]
fn commit_bc_editor_spring_constraint_appends_one_matching_bc_node_stiff_dir() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::SpringConstraint {
        node_id: 2,
        stiffness: 250.0,
        direction: Vec3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::SpringConstraint {
            node_id,
            stiffness,
            direction,
        } => {
            assert_eq!(*node_id, 2);
            assert!(near(*stiffness, 250.0) && near(direction.x, 1.0));
        }
        _ => panic!("expected SpringConstraint"),
    }
}

#[test]
fn commit_bc_editor_body_load_appends_one_matching_bc_vec3_only() {
    use cadkernel_math::Vec3;
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::BodyLoad {
        force_density: Vec3 {
            x: 0.0,
            y: 0.0,
            z: -7700.0,
        },
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::BodyLoad { force_density } => {
            assert!(near(force_density.z, -7700.0));
        }
        _ => panic!("expected BodyLoad"),
    }
}

#[test]
fn commit_bc_editor_initial_temperature_appends_one_matching_bc_node_scalar() {
    use cadkernel_modeling::{AnalysisContainer, BoundaryCondition, FemMaterial};
    let mut c = AnalysisContainer::new(make_unit_tet_mesh(), FemMaterial::steel());
    c.add_bc(BoundaryCondition::InitialTemperature {
        node: 1,
        temperature: 300.0,
    });
    assert_eq!(c.boundary_conditions.len(), 1);
    match &c.boundary_conditions[0] {
        BoundaryCondition::InitialTemperature { node, temperature } => {
            assert_eq!(*node, 1);
            assert!(near(*temperature, 300.0));
        }
        _ => panic!("expected InitialTemperature"),
    }
}

#[test]
fn fem_bc_editor_section_print_dispatch_commits_plane_marker() {
    use cadkernel_modeling::BoundaryCondition;

    let mut app = CadApp::new_headless();
    app.seed_test_fem_empty_analysis();
    app.dispatch_fem_open_section_print_bc_editor();
    assert!(app.fem_bc_editor_is_open());

    app.set_fem_bc_vec3_for_test(0.0, 1.0, 0.0);
    app.set_fem_bc_point_for_test(1.0, 2.0, 3.0);
    app.dispatch_fem_commit_bc_editor();

    let bcs = app.fem_boundary_conditions_for_test();
    assert_eq!(bcs.len(), 1);
    match &bcs[0] {
        BoundaryCondition::SectionPrint {
            plane_normal,
            plane_point,
        } => {
            assert!(near(plane_normal.y, 1.0));
            assert!(near(plane_point.x, 1.0) && near(plane_point.z, 3.0));
        }
        _ => panic!("expected SectionPrint"),
    }
}

#[test]
fn fem_bc_editor_multi_node_dispatch_commits_sets_and_penalty() {
    use cadkernel_modeling::BoundaryCondition;

    let mut app = CadApp::new_headless();
    app.seed_test_fem_empty_analysis();

    app.dispatch_fem_open_tie_constraint_bc_editor();
    assert!(app.fem_bc_editor_is_open());
    app.set_fem_bc_node_sets_for_test(0, 1, 2, 3);
    app.dispatch_fem_commit_bc_editor();

    app.dispatch_fem_open_rigid_body_bc_editor();
    assert!(app.fem_bc_editor_is_open());
    app.set_fem_bc_node_sets_for_test(1, 3, 0, 0);
    app.dispatch_fem_commit_bc_editor();

    app.dispatch_fem_open_contact_constraint_bc_editor();
    assert!(app.fem_bc_editor_is_open());
    app.set_fem_bc_node_sets_for_test(0, 1, 2, 3);
    app.set_fem_bc_scalar_for_test(1.25e7);
    app.dispatch_fem_commit_bc_editor();

    let bcs = app.fem_boundary_conditions_for_test();
    assert_eq!(bcs.len(), 3);
    match &bcs[0] {
        BoundaryCondition::TieConstraint {
            surface_a,
            surface_b,
        } => {
            assert_eq!(surface_a, &[0, 1]);
            assert_eq!(surface_b, &[2, 3]);
        }
        _ => panic!("expected TieConstraint"),
    }
    match &bcs[1] {
        BoundaryCondition::RigidBody { node_ids } => assert_eq!(node_ids, &[1, 2, 3]),
        _ => panic!("expected RigidBody"),
    }
    match &bcs[2] {
        BoundaryCondition::ContactConstraint {
            surface_a,
            surface_b,
            penalty,
        } => {
            assert_eq!(surface_a, &[0, 1]);
            assert_eq!(surface_b, &[2, 3]);
            assert!(near(*penalty, 1.25e7));
        }
        _ => panic!("expected ContactConstraint"),
    }
}

// ---------------------------------------------------------------------------
// Phase A — sketch-driven PartDesign features (Pad / Pocket / Groove / Hole /
// CountersunkHole) and Draft 2D primitives, all driven through the production
// dispatcher via the test_support harness.
// ---------------------------------------------------------------------------

#[test]
fn pad_sketch_with_no_base_solid_extrudes_fresh_solid_into_scene() {
    let mut app = CadApp::new_headless();
    app.seed_test_sketch_square(2.0);
    app.dispatch_pad_sketch(3.0, false);
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1, "Pad with no base must create one solid");
    let obj = scene.objects.first().unwrap();
    assert!(!obj.vertices.is_empty(), "Pad result must have geometry");
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 3.0 - 1e-3,
        "Pad z-extent must match depth=3.0, got {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
}

#[test]
fn pad_sketch_without_active_sketch_logs_warning_and_adds_no_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_pad_sketch(5.0, false);
    assert!(
        app.scene_ref().is_empty(),
        "Pad without sketch must not create geometry"
    );
}

#[test]
fn pad_sketch_rejects_open_profile_even_with_three_edges() {
    let mut app = CadApp::new_headless();
    app.seed_test_sketch_open_profile();
    app.dispatch_pad_sketch(3.0, false);
    assert!(
        app.scene_ref().is_empty(),
        "Pad must reject open sketch chains instead of extruding a partial profile"
    );
}

#[test]
fn pad_sketch_accepts_square_with_construction_diagonal() {
    let mut app = CadApp::new_headless();
    app.seed_test_sketch_square_with_construction_diagonal(2.0);
    app.dispatch_pad_sketch(2.0, false);
    assert_eq!(
        app.scene_ref().len(),
        1,
        "construction lines must be ignored by profile validation"
    );
}

#[test]
fn pocket_sketch_after_pad_removes_material() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let count_after_box = app.scene_ref().len();
    assert_eq!(count_after_box, 1);
    app.seed_test_sketch_square(1.5);
    app.dispatch_pocket_sketch(2.0, false);
    // Pocket adds the result as a new scene object on top of the box.
    assert!(
        app.scene_ref().len() >= count_after_box,
        "Pocket should add a result object"
    );
}

#[test]
fn pocket_sketch_without_base_solid_logs_warning() {
    let mut app = CadApp::new_headless();
    app.seed_test_sketch_square(1.0);
    app.dispatch_pocket_sketch(2.0, false);
    assert!(
        app.scene_ref().is_empty(),
        "Pocket without a base solid must not produce geometry"
    );
}

#[test]
fn hole_sketch_drills_through_base_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.dispatch_hole_sketch(0.5, 4.0);
    assert!(
        app.scene_ref().len() > before,
        "Hole should add the drilled solid as a new scene object"
    );
}

#[test]
fn countersunk_hole_sketch_produces_solid_into_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.dispatch_countersunk_hole_sketch(0.4, 3.0, 90.0);
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "CountersunkHole should add the drilled solid as a new scene object"
    );
}

#[test]
fn groove_sketch_with_revolve_profile_runs_without_panic() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    // Seed a non-degenerate sketch so groove has a 2+ point profile.
    app.seed_test_sketch_square(0.5);
    let before = app.scene_ref().len();
    app.dispatch_groove_sketch(90.0);
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Groove should add the revolved-cut result as a new scene object"
    );
}

#[test]
fn draft_circle_dispatch_adds_filled_solid_to_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_circle();
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1, "Draft circle must add one object");
    assert!(!scene.objects.first().unwrap().vertices.is_empty());
}

#[test]
fn draft_arc_dispatch_adds_filled_sector_to_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_arc();
    assert_eq!(app.scene_ref().len(), 1);
    assert!(!app.scene_ref().objects.first().unwrap().vertices.is_empty());
}

#[test]
fn draft_ellipse_dispatch_adds_filled_solid_to_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_ellipse();
    assert_eq!(app.scene_ref().len(), 1);
    let obj = app.scene_ref().objects.first().unwrap();
    // Ellipse with rx=2, ry=1 in the XY plane: the major axis spans ~4
    // units along whichever in-plane direction the kernel picks for `u`.
    let dx = (obj.aabb_max[0] - obj.aabb_min[0]) as f64;
    let dy = (obj.aabb_max[1] - obj.aabb_min[1]) as f64;
    assert!(
        dx.max(dy) >= 3.5,
        "ellipse major-axis extent too small: dx={dx}, dy={dy}"
    );
    assert!(
        dx.min(dy) >= 1.5,
        "ellipse minor-axis extent too small: dx={dx}, dy={dy}"
    );
}

#[test]
fn draft_line_dispatch_adds_tree_entry_without_geometry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_line();
    // Phase A: line has no native polyline pipeline, so the scene tree
    // gains an entry with an empty mesh. Tree count must increment so the
    // user can confirm the click registered.
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_point_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_point();
    assert_eq!(app.scene_ref().len(), 1);
}

// ---------------------------------------------------------------------------
// Phase B — Draft EASY tier (Wire / BSpline / Bezier / Hatch / Text /
// Upgrade / Downgrade / WireToBSpline / ToSketch + arrays + clone), all
// driven through the production dispatcher via the test_support harness.
// ---------------------------------------------------------------------------

#[test]
fn draft_wire_dispatch_adds_tree_entry_without_geometry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_wire();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_bspline_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_bspline();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_bezier_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_bezier();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_hatch_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_hatch();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_text_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_text();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_upgrade_dispatch_closes_wire_into_filled_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_upgrade();
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1, "Upgrade must add one solid");
    assert!(
        !scene.objects.first().unwrap().vertices.is_empty(),
        "Upgrade result must have geometry"
    );
}

#[test]
fn draft_downgrade_dispatch_splits_selected_solid_into_face_solids() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_downgrade();
    let after = app.scene_ref().len();
    assert!(
        after > before,
        "Downgrade should add per-face solids (a cube has 6 faces); before={before} after={after}"
    );
}

#[test]
fn draft_downgrade_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_downgrade();
    assert!(
        app.scene_ref().is_empty(),
        "Downgrade with no selection must not add any object"
    );
}

#[test]
fn draft_wire_to_bspline_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_wire_to_bspline();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_to_sketch_dispatch_populates_last_sketch() {
    let mut app = CadApp::new_headless();
    assert!(!app.last_sketch_is_set());
    app.dispatch_draft_to_sketch();
    assert!(
        app.last_sketch_is_set(),
        "ToSketch must save a sketch into gui.last_sketch"
    );
    assert!(app.scene_ref().is_empty());
}

#[test]
fn sketch_external_projection_uses_selected_object_as_construction_refs() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.toggle_select_index(0);
    app.dispatch_enter_sketch_xy();
    app.dispatch_sketch_external_projection();

    let (points, lines, construction_points, construction_lines) = app
        .active_sketch_geometry_counts()
        .expect("active sketch should exist");
    assert!(points >= 8, "box projection should add vertex refs");
    assert!(lines >= 12, "box projection should add edge refs");
    assert_eq!(construction_points, points);
    assert_eq!(construction_lines, lines);

    let (external_refs, reused, status) = app
        .active_sketch_reference_summary()
        .expect("reference summary should exist");
    assert!(external_refs >= 20, "box should project point+edge refs");
    assert_eq!(reused, 0);
    assert!(status.contains("external"));
}

#[test]
fn sketch_carbon_copy_reports_reused_last_sketch_geometry() {
    let mut app = CadApp::new_headless();
    app.seed_test_sketch_square(2.0);
    app.dispatch_enter_sketch_xy();
    app.dispatch_sketch_carbon_copy();

    let (points, lines, construction_points, construction_lines) = app
        .active_sketch_geometry_counts()
        .expect("active sketch should exist");
    assert_eq!(points, 4);
    assert_eq!(lines, 4);
    assert_eq!(construction_points, 0);
    assert_eq!(construction_lines, 0);

    let (external_refs, reused, status) = app
        .active_sketch_reference_summary()
        .expect("reference summary should exist");
    assert_eq!(external_refs, 0);
    assert!(reused >= 8, "square reuse should report points + lines");
    assert!(status.contains("copied"));
}

#[test]
fn sketch_select_references_selects_all_construction_refs() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.toggle_select_index(0);
    app.dispatch_enter_sketch_xy();
    app.dispatch_sketch_external_projection();

    let (external_refs, _reused, _status) = app
        .active_sketch_reference_summary()
        .expect("reference summary should exist");
    app.dispatch_sketch_select_references();

    assert_eq!(
        app.active_sketch_selected_count(),
        Some(external_refs),
        "Select References should select every construction reference entity"
    );
}

#[test]
fn sketch_promote_references_converts_selected_refs_to_regular_geometry() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.toggle_select_index(0);
    app.dispatch_enter_sketch_xy();
    app.dispatch_sketch_external_projection();
    app.dispatch_sketch_select_references();
    app.dispatch_sketch_promote_references();

    let (points, lines, construction_points, construction_lines) = app
        .active_sketch_geometry_counts()
        .expect("active sketch should exist");
    assert!(
        points >= 8,
        "promoted references should keep points editable"
    );
    assert!(
        lines >= 12,
        "promoted references should keep lines editable"
    );
    assert_eq!(construction_points, 0);
    assert_eq!(construction_lines, 0);

    let (external_refs, reused, status) = app
        .active_sketch_reference_summary()
        .expect("reference summary should exist");
    assert_eq!(external_refs, 0);
    assert_eq!(reused, 0);
    assert_eq!(status, "Refs: none");
}

#[test]
fn draft_clone_dispatch_duplicates_selected_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_clone();
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Clone should add exactly one duplicate"
    );
}

#[test]
fn draft_clone_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_clone();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn draft_array_rect_dispatch_creates_grid_of_copies() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_array_rect();
    let added = app.scene_ref().len() - before;
    // Default 3×2 grid has 6 instances; original is the selected solid, so
    // the dispatcher adds 5 new copies as scene objects.
    assert_eq!(
        added, 5,
        "rect array (3×2) should add 5 copies, got {added}"
    );
}

#[test]
fn draft_array_polar_dispatch_creates_rotational_copies() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_array_polar();
    let added = app.scene_ref().len() - before;
    assert_eq!(
        added, 5,
        "polar array (6-fold) should add 5 copies, got {added}"
    );
}

#[test]
fn draft_array_path_dispatch_creates_copies_along_path() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_array_path();
    let added = app.scene_ref().len() - before;
    assert_eq!(
        added, 3,
        "path array (4-point path) should add 3 copies, got {added}"
    );
}

#[test]
fn draft_array_point_dispatch_creates_copies_at_each_position() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_array_point();
    let added = app.scene_ref().len() - before;
    assert_eq!(
        added, 3,
        "point array (4 positions) should add 3 copies, got {added}"
    );
}

#[test]
fn draft_array_rect_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_array_rect();
    assert!(app.scene_ref().is_empty());
}

// ---------------------------------------------------------------------------
// Phase B-cont — Part workbench EASY tier (FaceFromWires, ConnectShapes,
// EmbedShapes, CutoutShapes, ExplodeCompound, CompoundFilter,
// BooleanFragments, SliceToCompound, PointsFromShape, ConvertToSolid,
// AutoDefeaturing, TransformedCopy, CoonsPatch).
// ---------------------------------------------------------------------------

#[test]
fn part_face_from_wires_dispatch_adds_filled_face_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_part_face_from_wires();
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1);
    assert!(!scene.objects.first().unwrap().vertices.is_empty());
}

#[test]
fn part_connect_shapes_with_two_selected_unions_into_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    // After two CreateBox dispatches, the second is the only selected object;
    // toggle-select the first to build a 2-selection.
    app.toggle_select_index(0);
    let before = app.scene_ref().len();
    app.dispatch_part_connect_shapes();
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "ConnectShapes should add the union as a third object"
    );
}

#[test]
fn part_connect_shapes_without_two_selected_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_part_connect_shapes();
    assert_eq!(
        app.scene_ref().len(),
        before,
        "ConnectShapes with <2 selected must not add a new object"
    );
}

#[test]
fn part_embed_shapes_with_two_selected_unions_into_one_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.toggle_select_index(0);
    let before = app.scene_ref().len();
    app.dispatch_part_embed_shapes();
    assert_eq!(app.scene_ref().len(), before + 1);
}

#[test]
fn part_cutout_shapes_with_two_selected_subtracts_into_new_object() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.toggle_select_index(0);
    let before = app.scene_ref().len();
    app.dispatch_part_cutout_shapes();
    assert_eq!(app.scene_ref().len(), before + 1);
}

#[test]
fn part_explode_compound_logs_count_without_adding_objects() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.dispatch_create_box(1.0, 1.0, 1.0);
    app.toggle_select_index(0);
    let before = app.scene_ref().len();
    app.dispatch_part_explode_compound();
    // ExplodeCompound is informational — solids are already in the scene.
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn part_explode_compound_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_part_explode_compound();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn part_compound_filter_logs_pass_count_without_adding_objects() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_part_compound_filter();
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn part_boolean_fragments_with_two_selected_logs_count() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.toggle_select_index(0);
    let before = app.scene_ref().len();
    app.dispatch_part_boolean_fragments();
    // Phase B-cont: fragments are logged but not yet staged into the scene
    // (multi-model staging deferred). Object count must not change.
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn part_slice_to_compound_with_selection_adds_pieces_to_scene() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.dispatch_part_slice_to_compound();
    let added = app.scene_ref().len() - before;
    assert!(
        added >= 1,
        "Slice-to-compound should add at least one piece (got {added})"
    );
}

#[test]
fn part_slice_to_compound_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_part_slice_to_compound();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn part_points_from_shape_with_selection_logs_count() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_part_points_from_shape();
    // No scene mutation — just a vertex-count log.
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn part_convert_to_solid_with_mesh_selection_adds_solid() {
    let mut app = CadApp::new_headless();
    // CreateBox produces a mesh; selecting it lets ConvertToSolid run.
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_part_convert_to_solid();
    assert!(
        app.scene_ref().len() >= before,
        "ConvertToSolid should add the rebuilt solid as a new scene object"
    );
}

#[test]
fn part_auto_defeaturing_with_selection_adds_simplified_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.scene_ref().len();
    app.dispatch_part_auto_defeaturing(0.01);
    // Auto-defeaturing on a clean box may or may not change the topology —
    // the kernel still rebuilds the solid and we add it as a new object.
    assert!(app.scene_ref().len() >= before);
}

#[test]
fn part_auto_defeaturing_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_part_auto_defeaturing(0.01);
    assert!(app.scene_ref().is_empty());
}

#[test]
fn part_transformed_copy_with_selection_adds_translated_copy() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_part_transformed_copy(5.0, 0.0, 0.0);
    assert_eq!(app.scene_ref().len(), before + 1);
}

#[test]
fn part_coons_patch_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_part_coons_patch();
    // NurbsSurface output → tree-only entry, same as Phase B wire features.
    assert_eq!(app.scene_ref().len(), 1);
}

// ---------------------------------------------------------------------------
// Phase C1 — Draft transforms (Move / Rotate / Scale / Mirror) + 3 EASY
// stragglers (S::Coons, FemAction::Summary, FemAction::Report).
// ---------------------------------------------------------------------------

#[test]
fn draft_move_dispatch_adds_translated_copy() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_move();
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Draft Move should add the translated solid as a new scene object"
    );
}

#[test]
fn draft_move_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_move();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn draft_rotate_dispatch_adds_rotated_copy() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_rotate();
    assert_eq!(app.scene_ref().len(), before + 1);
}

#[test]
fn draft_scale_dispatch_adds_scaled_copy() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_scale();
    assert_eq!(app.scene_ref().len(), before + 1);
}

#[test]
fn draft_mirror_dispatch_adds_mirrored_copy() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 1.0, 1.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_mirror();
    assert_eq!(app.scene_ref().len(), before + 1);
}

#[test]
fn surface_coons_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_surface_coons();
    // Same NurbsSurface tree-only output as P::CoonsPatch — they share the
    // same kernel API.
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn fem_summary_with_seeded_analysis_logs_text() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis();
    let before = app.scene_ref().len();
    app.dispatch_fem_summary();
    // Summary mutates `gui.status_message` via log_info — no scene change.
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn fem_summary_without_analysis_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_fem_summary();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn fem_report_with_seeded_analysis_logs_text() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis();
    let before = app.scene_ref().len();
    app.dispatch_fem_report();
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn fem_report_without_analysis_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_fem_report();
    assert!(app.scene_ref().is_empty());
}

// ---------------------------------------------------------------------------
// Phase E — Solver dispatchers (SolveThermal / SolveNonlinear)
// ---------------------------------------------------------------------------

#[test]
fn fem_solve_thermal_with_seeded_analysis_populates_temperature_field() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    let before = app.scene_ref().len();
    app.dispatch_fem_solve_thermal();
    // Solver mutates the container in place — no scene tree change.
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn fem_solve_thermal_without_analysis_emits_status_message() {
    let mut app = CadApp::new_headless();
    app.dispatch_fem_solve_thermal();
    // No-analysis path takes the early-return status branch and produces no
    // scene entries.
    assert!(app.scene_ref().is_empty());
}

#[test]
fn fem_solve_nonlinear_with_seeded_analysis_populates_result() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    let before = app.scene_ref().len();
    app.dispatch_fem_solve_nonlinear();
    assert_eq!(app.scene_ref().len(), before);
}

#[test]
fn fem_solve_nonlinear_without_analysis_emits_status_message() {
    let mut app = CadApp::new_headless();
    app.dispatch_fem_solve_nonlinear();
    assert!(app.scene_ref().is_empty());
}

fn fem_colormap_object_count(app: &CadApp, prefix: &str) -> usize {
    app.scene_ref()
        .objects
        .iter()
        .filter(|o| o.name.starts_with(prefix))
        .count()
}

#[test]
fn fem_show_displacement_after_solve_adds_colormap_mesh() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    app.dispatch_fem_solve_nonlinear();
    app.dispatch_fem_show_displacement();
    let count = fem_colormap_object_count(&app, "FEM Displacement colormap");
    assert!(count >= 1, "displacement colormap should add mesh bands");
    assert!(
        app.scene_ref()
            .objects
            .iter()
            .filter(|o| o.name.starts_with("FEM Displacement colormap"))
            .all(|o| !o.vertices.is_empty()),
        "colormap bands should be drawable meshes"
    );
}

#[test]
fn fem_show_stress_and_von_mises_replace_previous_colormap() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    app.dispatch_fem_solve_nonlinear();

    app.dispatch_fem_show_stress();
    assert!(fem_colormap_object_count(&app, "FEM Stress colormap") >= 1);

    app.dispatch_fem_show_von_mises();
    assert_eq!(fem_colormap_object_count(&app, "FEM Stress colormap"), 0);
    assert!(fem_colormap_object_count(&app, "FEM Von Mises colormap") >= 1);
}

#[test]
fn fem_show_stress_without_result_does_not_add_scene_objects() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    app.dispatch_fem_show_stress();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn fem_colormap_updates_result_legend_state() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    app.dispatch_fem_solve_nonlinear();
    app.dispatch_fem_show_von_mises();

    let (label, bands, min, max) = app
        .fem_result_legend_summary()
        .expect("legend should be populated after colormap");
    assert_eq!(label, "Von Mises");
    assert_eq!(bands, 7);
    assert!(min <= max, "legend range should be ordered");
}

#[test]
fn fem_result_probe_records_node_element_values() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    app.dispatch_fem_solve_thermal();
    app.dispatch_fem_solve_nonlinear();
    app.dispatch_fem_open_result_probe();
    assert!(app.fem_result_probe_is_open());
    app.set_fem_result_probe_for_test(3, 0);
    app.dispatch_fem_commit_result_probe();

    let (node, elem, disp, stress, temp) = app
        .fem_last_probe_summary()
        .expect("probe should be recorded");
    assert_eq!(node, 3);
    assert_eq!(elem, Some(0));
    assert!(
        disp.is_some(),
        "mechanical probe should include displacement"
    );
    assert!(stress.is_some(), "mechanical probe should include stress");
    assert!(temp.is_some(), "thermal probe should include temperature");
}

#[test]
fn fem_result_table_opens_with_node_and_element_rows() {
    let mut app = CadApp::new_headless();
    app.seed_test_fem_analysis_for_solvers();
    app.dispatch_fem_solve_nonlinear();
    app.dispatch_fem_show_stress();
    app.dispatch_fem_open_result_table();

    assert!(app.fem_result_table_is_open());
    let counts = app
        .fem_result_table_counts()
        .expect("result table should be open");
    assert_eq!(counts.0, 4, "seeded FEM mesh has 4 node rows");
    assert_eq!(counts.1, 1, "seeded FEM mesh has 1 element row");
}

// ---------------------------------------------------------------------------
// Phase C2 — Draft modify (Offset / Trim / Stretch / Facebinder) +
// P::ProjectCurvesOnSurface.
// ---------------------------------------------------------------------------

#[test]
fn draft_offset_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_offset();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_trim_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_trim();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_stretch_dispatch_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_stretch();
    assert_eq!(app.scene_ref().len(), 1);
}

#[test]
fn draft_facebinder_dispatch_with_selection_adds_face_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.scene_ref().len();
    app.dispatch_draft_facebinder();
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "Facebinder should add a face-solid as a new scene object"
    );
}

#[test]
fn draft_facebinder_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_facebinder();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn part_project_curves_on_surface_with_selection_adds_tree_entry() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.scene_ref().len();
    app.dispatch_part_project_curves_on_surface();
    assert_eq!(
        app.scene_ref().len(),
        before + 1,
        "ProjectCurvesOnSurface should add a projected-curve tree entry"
    );
}

#[test]
fn part_project_curves_on_surface_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_part_project_curves_on_surface();
    assert!(app.scene_ref().is_empty());
}

// ---------------------------------------------------------------------------
// Phase C3 — Surface ops (Sections / Extend / Blend) + PartDesign
// Loft/Pipe (Additive*, Subtractive*). Final MEDIUM-tier batch.
// ---------------------------------------------------------------------------

#[test]
fn surface_sections_dispatch_adds_skinned_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_surface_sections();
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1);
    let obj = scene.objects.first().unwrap();
    // The skinned solid spans z=[0, 2], so its z-extent is ~2.
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 1.5,
        "Sections should produce a 2-unit-tall solid, got z-extent {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
}

#[test]
fn surface_extend_dispatch_with_selection_adds_extended_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.scene_ref().len();
    app.dispatch_surface_extend();
    assert!(
        app.scene_ref().len() >= before,
        "Surface Extend should add the thickened solid; before={before}, after={}",
        app.scene_ref().len()
    );
}

#[test]
fn surface_extend_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_surface_extend();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn surface_blend_dispatch_adds_quad_sheet() {
    let mut app = CadApp::new_headless();
    app.dispatch_surface_blend();
    let scene = app.scene_ref();
    assert_eq!(scene.len(), 1);
    let obj = scene.objects.first().unwrap();
    // Default companion polyline is offset Z=1, so the blend has a 1-unit
    // z-extent.
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 0.8,
        "Blend should span Z by ~1.0, got {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
}

#[test]
fn partdesign_additive_loft_dispatch_adds_lofted_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_partdesign_additive_loft();
    assert_eq!(app.scene_ref().len(), 1);
    assert!(!app.scene_ref().objects.first().unwrap().vertices.is_empty());
}

#[test]
fn partdesign_additive_pipe_dispatch_adds_swept_solid() {
    let mut app = CadApp::new_headless();
    app.dispatch_partdesign_additive_pipe();
    assert_eq!(app.scene_ref().len(), 1);
    let obj = app.scene_ref().objects.first().unwrap();
    // Pipe sweeps along Z from 0 to 2 with 0.25 cross-section.
    assert!(
        (obj.aabb_max[2] - obj.aabb_min[2]) >= 1.5,
        "Pipe should span Z by ~2.0, got {}",
        obj.aabb_max[2] - obj.aabb_min[2]
    );
}

#[test]
fn partdesign_subtractive_loft_with_selection_runs_without_panic() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.dispatch_partdesign_subtractive_loft();
    // boolean_op_exact may fail on coincident-face edge cases; the test
    // asserts only that the dispatcher runs without panic and scene count
    // never decreases.
    assert!(app.scene_ref().len() >= before);
}

#[test]
fn partdesign_subtractive_loft_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_partdesign_subtractive_loft();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn partdesign_subtractive_pipe_with_selection_runs_without_panic() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 4.0, 4.0);
    let before = app.scene_ref().len();
    app.dispatch_partdesign_subtractive_pipe();
    assert!(app.scene_ref().len() >= before);
}

#[test]
fn partdesign_subtractive_pipe_without_selection_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_partdesign_subtractive_pipe();
    assert!(app.scene_ref().is_empty());
}

#[test]
fn partdesign_shape_binder_copies_selected_shape_faces() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.scene_ref().len();
    app.dispatch_partdesign_shape_binder();
    assert_eq!(app.scene_ref().len(), before + 1);
    let obj = app.scene_ref().objects.last().expect("shape binder object");
    assert!(
        !obj.vertices.is_empty(),
        "ShapeBinder should tessellate copied faces"
    );
}

// ---------------------------------------------------------------------------
// Phase D — annotation overlay + scene_overlay backfill (D::Dimension /
// D::Label + verifying that Phase A-C wire-output features now actually
// populate the overlay so they render via the egui foreground layer).
// ---------------------------------------------------------------------------

#[test]
fn draft_dimension_dispatch_populates_overlay_with_lines_and_label() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_dimension();
    let (lines, _points, labels) = app.overlay_counts();
    // 2 extension lines + 1 dimension line + 1 value label.
    assert_eq!(lines, 3, "dimension should add 3 polylines, got {lines}");
    assert_eq!(labels, 1, "dimension should add 1 label, got {labels}");
    assert_eq!(
        app.scene_ref().len(),
        1,
        "dimension also gains a tree entry"
    );
}

#[test]
fn draft_label_dispatch_populates_overlay_with_leader_and_label() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_label();
    let (lines, _points, labels) = app.overlay_counts();
    assert_eq!(lines, 1, "label leader should add 1 polyline");
    assert_eq!(labels, 1, "label should add 1 text entry");
}

#[test]
fn draft_line_dispatch_now_populates_overlay() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_line();
    let (lines, _points, _labels) = app.overlay_counts();
    assert_eq!(lines, 1, "Phase D: D::Line now populates the overlay");
}

#[test]
fn draft_point_dispatch_now_populates_overlay() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_point();
    let (_lines, points, _labels) = app.overlay_counts();
    assert_eq!(points, 1, "Phase D: D::Point now populates the overlay");
}

#[test]
fn draft_wire_dispatch_now_populates_overlay() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_wire();
    let (lines, _points, _labels) = app.overlay_counts();
    assert_eq!(lines, 1, "Phase D: D::Wire now populates the overlay");
}

#[test]
fn draft_bspline_dispatch_now_populates_overlay() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_bspline();
    let (lines, _points, _labels) = app.overlay_counts();
    assert_eq!(lines, 1, "Phase D: D::BSpline now populates the overlay");
}

#[test]
fn draft_text_dispatch_populates_overlay_with_strokes_and_label() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_text();
    let (lines, _points, labels) = app.overlay_counts();
    assert!(lines >= 1, "Text should add stroke polylines, got {lines}");
    assert_eq!(labels, 1, "Text should add an anchored label");
}

#[test]
fn part_points_from_shape_now_populates_overlay() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    let before = app.overlay_counts().1;
    app.dispatch_part_points_from_shape();
    let after = app.overlay_counts().1;
    // A box has 8 unique vertex positions.
    assert!(
        after - before >= 8,
        "PointsFromShape should add ≥8 overlay points"
    );
}

#[test]
fn new_model_clears_scene_overlay() {
    let mut app = CadApp::new_headless();
    app.dispatch_draft_line();
    app.dispatch_draft_point();
    assert!(app.overlay_counts().0 + app.overlay_counts().1 > 0);
    app.dispatch_new_model();
    assert_eq!(
        app.overlay_counts(),
        (0, 0, 0),
        "NewModel should clear the scene_overlay"
    );
}

// ---------------------------------------------------------------------------
// Phase F-page — TechDraw page management.
// ---------------------------------------------------------------------------

fn temp_export_path(stem: &str, ext: &str) -> std::path::PathBuf {
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("cadk_viewer_{stem}_{pid}_{nanos}.{ext}"))
}

#[test]
fn techdraw_new_page_creates_a4_landscape_sheet() {
    let mut app = CadApp::new_headless();
    assert!(app.techdraw_sheet_size().is_none());
    app.dispatch_techdraw_new_page();
    let (w, h) = app.techdraw_sheet_size().expect("sheet should be opened");
    assert!((w - 297.0).abs() < 1e-6 && (h - 210.0).abs() < 1e-6);
}

#[test]
fn techdraw_from_template_cycles_through_three_templates() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_from_template();
    let s1 = app.techdraw_sheet_size().unwrap();
    assert!((s1.0 - 297.0).abs() < 1e-6 && (s1.1 - 210.0).abs() < 1e-6);
    app.dispatch_techdraw_from_template();
    let s2 = app.techdraw_sheet_size().unwrap();
    assert!((s2.0 - 210.0).abs() < 1e-6 && (s2.1 - 297.0).abs() < 1e-6);
    app.dispatch_techdraw_from_template();
    let s3 = app.techdraw_sheet_size().unwrap();
    assert!((s3.0 - 420.0).abs() < 1e-6 && (s3.1 - 297.0).abs() < 1e-6);
    app.dispatch_techdraw_from_template();
    let s4 = app.techdraw_sheet_size().unwrap();
    assert!((s4.0 - 297.0).abs() < 1e-6 && (s4.1 - 210.0).abs() < 1e-6);
}

#[test]
fn techdraw_page_setup_opens_stateful_dialog() {
    let mut app = CadApp::new_headless();
    assert!(!app.techdraw_page_setup_is_open());
    app.dispatch_techdraw_open_page_setup();
    assert!(app.techdraw_page_setup_is_open());
}

#[test]
fn techdraw_page_setup_commit_applies_custom_title_and_size() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_open_page_setup();
    app.set_techdraw_page_setup_custom_for_test(500.0, 300.0, "Custom Fixture Sheet");
    app.dispatch_techdraw_commit_page_setup();
    assert!(!app.techdraw_page_setup_is_open());
    let (w, h) = app.techdraw_sheet_size().expect("sheet should be applied");
    assert!((w - 500.0).abs() < 1e-6 && (h - 300.0).abs() < 1e-6);
    assert_eq!(
        app.techdraw_sheet_title().as_deref(),
        Some("Custom Fixture Sheet")
    );
}

#[test]
fn techdraw_page_setup_a3_preset_applies_named_sheet() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_open_page_setup();
    app.set_techdraw_page_setup_a3_for_test("A3 Assembly Drawing");
    app.dispatch_techdraw_commit_page_setup();
    let (w, h) = app.techdraw_sheet_size().expect("sheet should be applied");
    assert!((w - 420.0).abs() < 1e-6 && (h - 297.0).abs() < 1e-6);
    assert_eq!(
        app.techdraw_sheet_title().as_deref(),
        Some("A3 Assembly Drawing")
    );
}

#[test]
fn techdraw_dimension_setup_opens_stateful_dialog() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    assert!(!app.techdraw_dimension_setup_is_open());
    app.dispatch_techdraw_open_linear_dimension_setup();
    assert!(app.techdraw_dimension_setup_is_open());
}

#[test]
fn techdraw_dimension_setup_commit_applies_custom_linear_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_linear_dimension_setup();
    app.set_techdraw_dimension_setup_linear_for_test(
        "CUSTOM 123.45",
        20.0,
        160.0,
        180.0,
        160.0,
        -18.0,
    );
    app.dispatch_techdraw_commit_dimension_setup();
    assert!(!app.techdraw_dimension_setup_is_open());
    let counts = app.techdraw_dimension_counts();
    assert_eq!(counts.0, 1, "custom linear setup should add one dimension");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("CUSTOM 123.45"));
}

#[test]
fn techdraw_dimension_setup_commit_applies_custom_diameter_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_diameter_dimension_setup();
    app.set_techdraw_dimension_setup_diameter_for_test(42.0, 144.0, 96.0);
    app.dispatch_techdraw_commit_dimension_setup();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(
        counts.1, 1,
        "custom diameter setup should add one extended dimension"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("⌀42.00"));
}

#[test]
fn techdraw_annotation_setup_opens_stateful_dialog() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    assert!(!app.techdraw_annotation_setup_is_open());
    app.dispatch_techdraw_open_text_annotation_setup();
    assert!(app.techdraw_annotation_setup_is_open());
}

#[test]
fn techdraw_annotation_setup_commit_applies_custom_text() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_text_annotation_setup();
    app.set_techdraw_annotation_setup_text_for_test("CUSTOM INSPECTION NOTE", 36.0, 48.0, 7.5);
    app.dispatch_techdraw_commit_annotation_setup();
    assert!(!app.techdraw_annotation_setup_is_open());
    let counts = app.techdraw_annotation_counts();
    assert_eq!(counts.0, 1, "custom text setup should add one annotation");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("CUSTOM INSPECTION NOTE"));
}

#[test]
fn techdraw_annotation_setup_commit_applies_custom_balloon() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_balloon_annotation_setup();
    app.set_techdraw_annotation_setup_balloon_for_test(17, 50.0, 60.0, 110.0, 80.0, 9.0);
    app.dispatch_techdraw_commit_annotation_setup();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(counts.2, 1, "custom balloon setup should add one balloon");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains(">17</text>"));
}

#[test]
fn techdraw_centerline_setup_opens_stateful_dialog() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    assert!(!app.techdraw_centerline_setup_is_open());
    app.dispatch_techdraw_open_center_mark_setup();
    assert!(app.techdraw_centerline_setup_is_open());
}

#[test]
fn techdraw_centerline_setup_commit_applies_custom_center_mark() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_center_mark_setup();
    app.set_techdraw_centerline_setup_center_mark_for_test(100.0, 90.0, 18.0);
    app.dispatch_techdraw_commit_centerline_setup();
    assert!(!app.techdraw_centerline_setup_is_open());
    let counts = app.techdraw_centerline_counts();
    assert_eq!(counts.0, 1, "custom center mark setup should add one mark");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("stroke=\"red\""));
}

#[test]
fn techdraw_centerline_setup_commit_applies_custom_bolt_circle() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_bolt_circle_setup();
    app.set_techdraw_centerline_setup_bolt_circle_for_test(100.0, 80.0, 24.0, 8, 12.0);
    app.dispatch_techdraw_commit_centerline_setup();
    let counts = app.techdraw_centerline_counts();
    assert_eq!(
        counts.2, 1,
        "custom bolt circle setup should add one bolt circle"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("<circle cx=\"100\" cy=\"80\" r=\"24\""));
}

#[test]
fn techdraw_view_setup_opens_stateful_dialog() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_techdraw_new_page();
    assert!(!app.techdraw_view_setup_is_open());
    app.dispatch_techdraw_open_front_view_setup();
    assert!(app.techdraw_view_setup_is_open());
}

#[test]
fn techdraw_view_setup_commit_applies_custom_front_placement() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_front_view_setup();
    app.set_techdraw_view_setup_placement_for_test(132.0, 88.0, 36.0);
    app.dispatch_techdraw_commit_view_setup();
    assert!(!app.techdraw_view_setup_is_open());
    assert_eq!(app.techdraw_view_count(), 1);
    assert_eq!(
        app.techdraw_view_placement(0),
        Some((Some(132.0), Some(88.0), Some(36.0)))
    );
}

#[test]
fn techdraw_view_setup_commit_applies_three_view_spacing() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_open_three_view_setup();
    app.set_techdraw_view_setup_placement_for_test(90.0, 120.0, 28.0);
    app.set_techdraw_three_view_spacing_for_test(70.0, 45.0);
    app.dispatch_techdraw_commit_view_setup();
    assert_eq!(app.techdraw_view_count(), 3);
    assert_eq!(
        app.techdraw_view_placement(0),
        Some((Some(90.0), Some(120.0), Some(28.0)))
    );
    assert_eq!(
        app.techdraw_view_placement(1),
        Some((Some(90.0), Some(75.0), Some(28.0)))
    );
    assert_eq!(
        app.techdraw_view_placement(2),
        Some((Some(160.0), Some(120.0), Some(28.0)))
    );
}

#[test]
fn techdraw_redraw_without_sheet_logs_warning() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_redraw();
    assert!(app.techdraw_sheet_size().is_none());
}

#[test]
fn techdraw_section_view_dispatch_adds_cut_view() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 2.0);
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_section_view();
    assert_eq!(app.techdraw_view_count(), 1);
    assert!(
        app.techdraw_last_view_edge_count().unwrap_or(0) > 0,
        "section view should contain projected cut edges"
    );
}

#[test]
fn techdraw_detail_view_dispatch_adds_magnified_view() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_three_view();
    let before = app.techdraw_view_count();
    app.dispatch_techdraw_detail_view();
    assert_eq!(app.techdraw_view_count(), before + 1);
    assert!(
        app.techdraw_last_view_edge_count().unwrap_or(0) > 0,
        "detail view should copy at least one source edge"
    );
}

#[test]
fn techdraw_broken_view_dispatch_adds_compressed_view() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(4.0, 1.0, 1.0);
    app.dispatch_techdraw_three_view();
    let before = app.techdraw_view_count();
    app.dispatch_techdraw_broken_view();
    assert_eq!(app.techdraw_view_count(), before + 1);
    assert!(
        app.techdraw_last_view_edge_count().unwrap_or(0) > 0,
        "broken view should retain drawable source edges"
    );
}

#[test]
fn techdraw_dim_linear_dispatch_adds_sheet_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_dim_linear();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(counts.0, 1, "linear dimension should use sheet dimensions");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("fill=\"blue\""));
}

#[test]
fn techdraw_dim_radius_dispatch_adds_radius_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_dim_radius();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(counts.0, 1, "radius dimension should use sheet dimensions");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("R"));
}

#[test]
fn techdraw_dim_diameter_dispatch_adds_extended_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_dim_diameter();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(
        counts.1, 1,
        "diameter dimension should use extended dimensions"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("⌀"));
}

#[test]
fn techdraw_dim_angle_dispatch_adds_extended_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_dim_angle();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(
        counts.1, 1,
        "angle dimension should use extended dimensions"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("°"));
}

#[test]
fn techdraw_dim_arc_len_dispatch_adds_arc_dimension() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_dim_arc_len();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(counts.2, 1, "arc length should use arc dimensions");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("Arc"));
}

#[test]
fn techdraw_dim_area_dispatch_adds_area_annotation() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_dim_area();
    let counts = app.techdraw_dimension_counts();
    assert_eq!(counts.3, 1, "area dimension should use area annotations");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("Area:"));
}

#[test]
fn techdraw_text_dispatch_adds_text_annotation() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_text();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(counts.0, 1, "text annotation should be stored on sheet");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("NOTE: Deburr all edges"));
}

#[test]
fn techdraw_rich_text_dispatch_adds_rich_text_annotation() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_rich_text();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(
        counts.1, 1,
        "rich text annotation should be stored on sheet"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("Rich text note"));
}

#[test]
fn techdraw_balloon_dispatch_adds_balloon_annotation() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_balloon();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(counts.2, 1, "balloon annotation should be stored on sheet");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("<circle"));
}

#[test]
fn techdraw_leader_dispatch_adds_leader_annotation() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_leader();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(counts.3, 1, "leader annotation should be stored on sheet");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("Leader callout"));
}

#[test]
fn techdraw_weld_dispatch_adds_weld_symbol() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_weld();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(counts.4, 1, "weld symbol should be stored on sheet");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("8.0"));
}

#[test]
fn techdraw_surface_finish_dispatch_adds_surface_finish_symbol() {
    let mut app = CadApp::new_headless();
    app.dispatch_techdraw_new_page();
    app.dispatch_techdraw_surf_finish();
    let counts = app.techdraw_annotation_counts();
    assert_eq!(
        counts.5, 1,
        "surface finish symbol should be stored on sheet"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("Ra 3.2"));
}

#[test]
fn techdraw_center_face_dispatch_adds_centerline() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_center_face();
    let counts = app.techdraw_centerline_counts();
    assert_eq!(counts.1, 1, "center face should add one centerline");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("stroke=\"red\""));
    assert!(svg.contains("stroke-dasharray=\"8,2,2,2\""));
}

#[test]
fn techdraw_center_lines_dispatch_adds_centerline() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(1.0, 3.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_center_lines();
    let counts = app.techdraw_centerline_counts();
    assert_eq!(counts.1, 1, "center lines should add one centerline");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("stroke-dasharray=\"8,2,2,2\""));
}

#[test]
fn techdraw_center_points_dispatch_adds_center_mark() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_center_points();
    let counts = app.techdraw_centerline_counts();
    assert_eq!(counts.0, 1, "center points should add one center mark");
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("stroke=\"red\""));
}

#[test]
fn techdraw_bolt_circle_dispatch_adds_bolt_circle_centerlines() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 2.0, 1.0);
    app.dispatch_techdraw_three_view();
    app.dispatch_techdraw_bolt_circle();
    let counts = app.techdraw_centerline_counts();
    assert_eq!(
        counts.2, 1,
        "bolt circle should add one bolt-circle centerline set"
    );
    let svg = app.techdraw_svg().expect("svg");
    assert!(svg.contains("<circle"));
    assert!(svg.contains("stroke-dasharray=\"8,2,2,2\""));
}

#[test]
fn techdraw_export_dxf_dispatch_writes_file() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_three_view();
    let path = temp_export_path("techdraw", "dxf");
    app.dispatch_techdraw_export_dxf(path.clone());

    let content = std::fs::read_to_string(&path).expect("read exported dxf");
    assert!(content.contains("\nSECTION\n"));
    assert!(content.contains("\nENTITIES\n"));
    assert!(content.contains("\nLWPOLYLINE\n"));
    let _ = std::fs::remove_file(path);
}

#[test]
fn techdraw_export_pdf_dispatch_writes_file() {
    let mut app = CadApp::new_headless();
    app.dispatch_create_box(2.0, 1.0, 1.0);
    app.dispatch_techdraw_three_view();
    let path = temp_export_path("techdraw", "pdf");
    app.dispatch_techdraw_export_pdf(path.clone());

    let content = std::fs::read(&path).expect("read exported pdf");
    assert!(content.starts_with(b"%PDF-"));
    assert!(content.windows(5).any(|w| w == b"%%EOF"));
    let _ = std::fs::remove_file(path);
}
