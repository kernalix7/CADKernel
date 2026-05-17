use cadkernel_assembly::{
    Assembly, AxisRef, ComponentId, CurveRef, EdgeRef, FaceRef, Mate, RigidTransform, SolverOptions,
};

fn assembly_with_components(count: usize) -> Assembly {
    let mut assembly = Assembly::new("test");
    for idx in 0..count {
        assembly
            .add_component(format!("part-{idx}"))
            .expect("component should be valid");
    }
    assembly
}

fn face(component: ComponentId) -> FaceRef {
    FaceRef::new(component, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0])
}

fn edge(component: ComponentId) -> EdgeRef {
    EdgeRef::new(component, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
}

fn axis(component: ComponentId) -> AxisRef {
    AxisRef::new(component, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
}

#[test]
fn coincident_mate_adds_three_constraints() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Coincident {
            face_a: face(ComponentId(0)),
            face_b: face(ComponentId(1)),
        })
        .expect("mate should be valid");
    assert_eq!(assembly.mate_count(), 1);
    assert_eq!(assembly.dof_count(), 9);
}

#[test]
fn concentric_mate_adds_four_constraints() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Concentric {
            axis_a: axis(ComponentId(0)),
            axis_b: axis(ComponentId(1)),
        })
        .expect("mate should be valid");
    assert_eq!(assembly.dof_count(), 8);
}

#[test]
fn parallel_mate_adds_two_constraints() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Parallel {
            face_a: face(ComponentId(0)),
            face_b: face(ComponentId(1)),
        })
        .expect("mate should be valid");
    assert_eq!(assembly.dof_count(), 10);
}

#[test]
fn perpendicular_mate_adds_one_constraint() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Perpendicular {
            face_a: face(ComponentId(0)),
            face_b: face(ComponentId(1)),
        })
        .expect("mate should be valid");
    assert_eq!(assembly.dof_count(), 11);
}

#[test]
fn tangent_mate_adds_one_constraint() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Tangent {
            face_a: face(ComponentId(0)),
            edge_b: edge(ComponentId(1)),
        })
        .expect("mate should be valid");
    assert_eq!(assembly.dof_count(), 11);
}

#[test]
fn distance_mate_solves_component_spacing() {
    let mut assembly = assembly_with_components(2);
    assembly
        .set_component_placement(ComponentId(1), RigidTransform::translated(10.0, 0.0, 0.0))
        .expect("placement should be valid");
    assembly
        .add_mate(Mate::Lock {
            component: ComponentId(0),
        })
        .expect("lock should be valid");
    assembly
        .add_mate(Mate::Distance {
            face_a: face(ComponentId(0)),
            face_b: face(ComponentId(1)),
            distance: 4.0,
        })
        .expect("distance should be valid");

    let report = assembly
        .solve(SolverOptions::default())
        .expect("solver should run");
    assert!(report.converged);
    let x = assembly
        .component(ComponentId(1))
        .expect("component exists")
        .placement
        .translation[0];
    assert!((x - 4.0).abs() < 1.0e-6);
}

#[test]
fn angle_mate_adds_one_constraint() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Angle {
            face_a: face(ComponentId(0)),
            face_b: face(ComponentId(1)),
            angle_rad: std::f64::consts::FRAC_PI_2,
        })
        .expect("angle should be valid");
    assert_eq!(assembly.dof_count(), 11);
}

#[test]
fn lock_mate_removes_six_dof() {
    let mut assembly = assembly_with_components(1);
    assembly
        .add_mate(Mate::Lock {
            component: ComponentId(0),
        })
        .expect("lock should be valid");
    assert_eq!(assembly.dof_count(), 0);
}

#[test]
fn slider_mate_allows_drag_motion() {
    let mut assembly = assembly_with_components(1);
    assembly
        .add_mate(Mate::Slider {
            axis: axis(ComponentId(0)),
            range_min: -10.0,
            range_max: 10.0,
        })
        .expect("slider should be valid");
    let report = assembly
        .drag_component(ComponentId(0), [3.0, 0.0])
        .expect("drag should be valid");
    assert!(report.solved);
    assert_eq!(
        assembly
            .component(ComponentId(0))
            .expect("component exists")
            .placement
            .translation,
        [3.0, 0.0, 0.0]
    );
}

#[test]
fn hinge_mate_allows_rotational_drag() {
    let mut assembly = assembly_with_components(1);
    assembly
        .add_mate(Mate::Hinge {
            axis: AxisRef::new(ComponentId(0), [0.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
            range_min: -2.0,
            range_max: 2.0,
        })
        .expect("hinge should be valid");
    assembly
        .drag_component(ComponentId(0), [1.25, 0.0])
        .expect("drag should be valid");
    let rz = assembly
        .component(ComponentId(0))
        .expect("component exists")
        .placement
        .rotation[2];
    assert!((rz - 1.25).abs() < 1.0e-9);
}

#[test]
fn gear_mate_validates_nonzero_ratio() {
    let mut assembly = assembly_with_components(2);
    assert!(
        assembly
            .add_mate(Mate::Gear {
                axis_a: axis(ComponentId(0)),
                axis_b: axis(ComponentId(1)),
                ratio: 2.0,
            })
            .is_ok()
    );
}

#[test]
fn rack_mate_applies_pitch_to_drag() {
    let mut assembly = assembly_with_components(1);
    assembly
        .add_mate(Mate::Rack {
            axis: axis(ComponentId(0)),
            pitch: 2.0,
        })
        .expect("rack should be valid");
    assembly
        .drag_component(ComponentId(0), [4.0, 0.0])
        .expect("drag should be valid");
    assert_eq!(
        assembly
            .component(ComponentId(0))
            .expect("component exists")
            .placement
            .translation,
        [8.0, 0.0, 0.0]
    );
}

#[test]
fn universal_mate_adds_four_constraints() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Universal {
            axis_a: axis(ComponentId(0)),
            axis_b: axis(ComponentId(1)),
        })
        .expect("universal should be valid");
    assert_eq!(assembly.dof_count(), 8);
}

#[test]
fn cam_mate_adds_two_constraints() {
    let mut assembly = assembly_with_components(2);
    assembly
        .add_mate(Mate::Cam {
            profile_face: face(ComponentId(0)),
            follower: edge(ComponentId(1)),
        })
        .expect("cam should be valid");
    assert_eq!(assembly.dof_count(), 10);
}

#[test]
fn screw_mate_combines_translation_and_rotation() {
    let mut assembly = assembly_with_components(1);
    assembly
        .add_mate(Mate::Screw {
            axis: axis(ComponentId(0)),
            pitch: std::f64::consts::TAU,
        })
        .expect("screw should be valid");
    assembly
        .drag_component(ComponentId(0), [1.0, 0.0])
        .expect("drag should be valid");
    let component = assembly
        .component(ComponentId(0))
        .expect("component exists");
    assert!((component.placement.translation[0] - 1.0).abs() < 1.0e-9);
    assert!((component.placement.rotation[0] - 1.0).abs() < 1.0e-9);
}

#[test]
fn path_mate_moves_along_curve_direction() {
    let mut assembly = assembly_with_components(1);
    assembly
        .add_mate(Mate::Path {
            component: ComponentId(0),
            path_curve: CurveRef::new(ComponentId(0), vec![[0.0, 0.0, 0.0], [0.0, 5.0, 0.0]]),
        })
        .expect("path should be valid");
    assembly
        .drag_component(ComponentId(0), [2.0, 0.0])
        .expect("drag should be valid");
    assert_eq!(
        assembly
            .component(ComponentId(0))
            .expect("component exists")
            .placement
            .translation,
        [0.0, 2.0, 0.0]
    );
}

#[test]
fn invalid_axis_is_rejected() {
    let mut assembly = assembly_with_components(1);
    let err = assembly
        .add_mate(Mate::Slider {
            axis: AxisRef::new(ComponentId(0), [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
            range_min: 0.0,
            range_max: 1.0,
        })
        .expect_err("zero axis should fail");
    assert!(err.to_string().contains("non-zero"));
}
