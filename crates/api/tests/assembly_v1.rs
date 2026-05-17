use cadkernel_api::{
    AssemblyAxisRef, AssemblyFaceRef, Command, ComponentId, Mate, Outcome, RigidTransform, Session,
    SolverOptions,
};

fn create_box(session: &mut Session) -> cadkernel_api::SolidId {
    match session
        .execute(Command::CreateBox {
            dx: 1.0,
            dy: 1.0,
            dz: 1.0,
        })
        .expect("box creation should succeed")
    {
        Outcome::SolidCreated { id, .. } => id,
        other => panic!("unexpected outcome: {other:?}"),
    }
}

fn face(component: ComponentId) -> AssemblyFaceRef {
    AssemblyFaceRef::new(component, [0.0, 0.0, 0.0], [0.0, 0.0, 1.0])
}

fn axis(component: ComponentId) -> AssemblyAxisRef {
    AssemblyAxisRef::new(component, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
}

fn session_with_three_component_assembly() -> (
    Session,
    cadkernel_api::AssemblyId,
    ComponentId,
    ComponentId,
    ComponentId,
) {
    let mut session = Session::new();
    let a = create_box(&mut session);
    let b = create_box(&mut session);
    let c = create_box(&mut session);
    let assembly = session
        .create_assembly("fixture")
        .expect("assembly should be created");
    let ca = session
        .add_assembly_component(assembly, "a", a)
        .expect("component a should be added");
    let cb = session
        .add_assembly_component_with_placement(
            assembly,
            "b",
            b,
            RigidTransform::translated(10.0, 0.0, 0.0),
        )
        .expect("component b should be added");
    let cc = session
        .add_assembly_component_with_placement(
            assembly,
            "c",
            c,
            RigidTransform::translated(0.0, 8.0, 0.0),
        )
        .expect("component c should be added");
    (session, assembly, ca, cb, cc)
}

#[test]
fn create_assembly_adds_document_slot() {
    let mut session = Session::new();
    let id = session
        .create_assembly("main")
        .expect("assembly should be created");
    assert_eq!(session.document().assembly_count(), 1);
    assert_eq!(
        session.document().assembly(id).expect("exists").name,
        "main"
    );
}

#[test]
fn push_assembly_preserves_name() {
    let mut session = Session::new();
    let id = session
        .push_assembly(cadkernel_api::Assembly::new("pushed"))
        .expect("assembly should be pushed");
    assert_eq!(
        session.document().assembly(id).expect("exists").name,
        "pushed"
    );
}

#[test]
fn add_component_requires_existing_solid() {
    let mut session = Session::new();
    let assembly = session
        .create_assembly("main")
        .expect("assembly should be created");
    let err = session
        .add_assembly_component(assembly, "missing", cadkernel_api::SolidId(99))
        .expect_err("missing solid should fail");
    assert!(err.to_string().contains("unknown solid"));
}

#[test]
fn add_component_records_source_solid() {
    let mut session = Session::new();
    let solid = create_box(&mut session);
    let assembly = session
        .create_assembly("main")
        .expect("assembly should be created");
    let component = session
        .add_assembly_component(assembly, "part", solid)
        .expect("component should be added");
    let assembly_ref = session.document().assembly(assembly).expect("exists");
    assert_eq!(
        assembly_ref
            .component(component)
            .expect("component exists")
            .source_solid,
        Some(u64::from(solid.0))
    );
}

#[test]
fn set_component_placement_updates_translation() {
    let (mut session, assembly, ca, _, _) = session_with_three_component_assembly();
    session
        .set_assembly_component_placement(assembly, ca, RigidTransform::translated(2.0, 3.0, 4.0))
        .expect("placement should be set");
    assert_eq!(
        session
            .document()
            .assembly(assembly)
            .expect("assembly exists")
            .component(ca)
            .expect("component exists")
            .placement
            .translation,
        [2.0, 3.0, 4.0]
    );
}

#[test]
fn add_mate_increments_mate_count() {
    let (mut session, assembly, ca, cb, _) = session_with_three_component_assembly();
    let mate = session
        .add_mate(
            assembly,
            Mate::Coincident {
                face_a: face(ca),
                face_b: face(cb),
            },
        )
        .expect("mate should be added");
    assert_eq!(mate.0, 0);
    assert_eq!(
        session
            .document()
            .assembly(assembly)
            .expect("assembly exists")
            .mate_count(),
        1
    );
}

#[test]
fn remove_mate_returns_removed_mate() {
    let (mut session, assembly, ca, cb, _) = session_with_three_component_assembly();
    let mate = session
        .add_mate(
            assembly,
            Mate::Distance {
                face_a: face(ca),
                face_b: face(cb),
                distance: 3.0,
            },
        )
        .expect("mate should be added");
    let removed = session
        .remove_mate(assembly, mate)
        .expect("mate should be removed");
    assert!(matches!(removed, Mate::Distance { .. }));
    assert_eq!(
        session
            .document()
            .assembly(assembly)
            .expect("assembly exists")
            .mate_count(),
        0
    );
}

#[test]
fn three_component_dof_count_uses_formula() {
    let (mut session, assembly, ca, cb, cc) = session_with_three_component_assembly();
    session
        .add_mate(assembly, Mate::Lock { component: ca })
        .expect("lock should be added");
    session
        .add_mate(
            assembly,
            Mate::Distance {
                face_a: face(ca),
                face_b: face(cb),
                distance: 4.0,
            },
        )
        .expect("distance should be added");
    session
        .add_mate(
            assembly,
            Mate::Distance {
                face_a: face(cb),
                face_b: face(cc),
                distance: 6.0,
            },
        )
        .expect("distance should be added");
    assert_eq!(session.assembly_dof_count(assembly).expect("dof"), 10);
}

#[test]
fn solve_assembly_converges_distance_chain() {
    let (mut session, assembly, ca, cb, _) = session_with_three_component_assembly();
    session
        .add_mate(assembly, Mate::Lock { component: ca })
        .expect("lock should be added");
    session
        .add_mate(
            assembly,
            Mate::Distance {
                face_a: face(ca),
                face_b: face(cb),
                distance: 5.0,
            },
        )
        .expect("distance should be added");
    let report = session
        .solve_assembly_with_options(
            assembly,
            SolverOptions {
                max_iterations: 80,
                tolerance: 1.0e-8,
            },
        )
        .expect("solve should run");
    assert!(report.converged);
    let x = session
        .document()
        .assembly(assembly)
        .expect("assembly exists")
        .component(cb)
        .expect("component exists")
        .placement
        .translation[0];
    assert!((x - 5.0).abs() < 1.0e-5);
}

#[test]
fn drag_component_moves_slider_component() {
    let (mut session, assembly, _, cb, _) = session_with_three_component_assembly();
    session
        .add_mate(
            assembly,
            Mate::Slider {
                axis: axis(cb),
                range_min: -100.0,
                range_max: 100.0,
            },
        )
        .expect("slider should be added");
    let report = session
        .drag_component(assembly, cb, [2.5, 0.0])
        .expect("drag should be valid");
    assert!(report.solved);
    let x = session
        .document()
        .assembly(assembly)
        .expect("assembly exists")
        .component(cb)
        .expect("component exists")
        .placement
        .translation[0];
    assert!((x - 12.5).abs() < 1.0e-9);
}

#[test]
fn drag_component_rejects_non_single_dof_component() {
    let (mut session, assembly, ca, cb, _) = session_with_three_component_assembly();
    session
        .add_mate(
            assembly,
            Mate::Distance {
                face_a: face(ca),
                face_b: face(cb),
                distance: 5.0,
            },
        )
        .expect("distance should be added");
    let err = session
        .drag_component(assembly, cb, [1.0, 0.0])
        .expect_err("distance mate should not allow drag");
    assert!(err.to_string().contains("single-DoF"));
}

#[test]
fn assembly_ids_are_stable() {
    let mut session = Session::new();
    let a = session.create_assembly("a").expect("assembly a");
    let b = session.create_assembly("b").expect("assembly b");
    assert_eq!(session.document().assembly_ids(), vec![a, b]);
}

#[test]
fn canonical_hash_changes_when_assembly_changes() {
    let mut session = Session::new();
    let before = session.canonical_hash();
    session
        .create_assembly("hash")
        .expect("assembly should be created");
    let after = session.canonical_hash();
    assert_ne!(before, after);
}

#[test]
fn unknown_assembly_errors_cleanly() {
    let mut session = Session::new();
    let err = session
        .add_mate(
            cadkernel_api::AssemblyId(44),
            Mate::Lock {
                component: ComponentId(0),
            },
        )
        .expect_err("unknown assembly should fail");
    assert!(err.to_string().contains("unknown assembly"));
}

#[test]
fn invalid_mate_errors_cleanly() {
    let (mut session, assembly, ca, _, _) = session_with_three_component_assembly();
    let err = session
        .add_mate(
            assembly,
            Mate::Slider {
                axis: AssemblyAxisRef::new(ca, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
                range_min: 0.0,
                range_max: 1.0,
            },
        )
        .expect_err("zero axis should fail");
    assert!(err.to_string().contains("invalid argument"));
}

#[test]
fn direct_assembly_calls_record_history() {
    let mut session = Session::new();
    let before = session.document().history().len();
    session
        .create_assembly("history")
        .expect("assembly should be created");
    assert_eq!(session.document().history().len(), before + 1);
    assert_eq!(
        session
            .document()
            .history()
            .last()
            .expect("history exists")
            .op,
        "create_assembly"
    );
}
