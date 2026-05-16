use cadkernel_api::{
    AxisRef, BodyId, Command, FeatureGraph, FeatureId, FeatureSpec, HelixSpec, Outcome, Session,
};
use cadkernel_modeling::body::{Body, FeatureKind};

fn helix_command(radius: f64, turns: f64) -> Command {
    Command::Helix {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns,
        cone_angle: 0.0,
    }
}

fn helix_spec(radius: f64, turns: f64) -> FeatureSpec {
    FeatureSpec::Helix(HelixSpec {
        axis: AxisRef::Z,
        radius,
        pitch: 2.0,
        height: 10.0,
        turns,
        cone_angle: 0.0,
    })
}

fn body_with_specs(count: u64) -> Body {
    let mut body = Body::new("Body");
    for feature_id in 1..=count {
        body.add_feature_with_spec(
            feature_id,
            &format!("Helix{feature_id}"),
            FeatureKind::Helix,
            "helix",
            serde_json::json!({
                "kind": "helix",
                "axis": { "axis": "z" },
                "radius": feature_id as f64,
                "pitch": 2.0,
                "height": 10.0,
                "turns": 3.0,
                "cone_angle": 0.0
            }),
        );
    }
    body
}

fn session_with_two_helixes() -> (Session, BodyId, FeatureId, FeatureId) {
    let mut session = Session::new();
    session.execute(helix_command(3.0, 5.0)).expect("helix 1");
    session.execute(helix_command(4.0, 3.0)).expect("helix 2");
    let body_id = session.document().active_body().expect("active body");
    let body = session.document().body(body_id).expect("body");
    let first = FeatureId(body.features[0].feature_id);
    let second = FeatureId(body.features[1].feature_id);
    (session, body_id, first, second)
}

#[test]
fn linear_chain_topological_sort_orders_all_features() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(1, 2).expect("edge");

    assert_eq!(graph.topological_sort().expect("topo"), vec![0, 1, 2]);
}

#[test]
fn diamond_topological_sort_respects_partial_order() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(0, 2).expect("edge");
    graph.add_edge(1, 3).expect("edge");
    graph.add_edge(2, 3).expect("edge");

    let topo = graph.topological_sort().expect("topo");
    let pos = |node| topo.iter().position(|seen| *seen == node).expect("node");
    assert!(pos(0) < pos(1));
    assert!(pos(0) < pos(2));
    assert!(pos(1) < pos(3));
    assert!(pos(2) < pos(3));
}

#[test]
fn cycle_detection_returns_strongly_connected_component() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(1, 0).expect("edge");

    assert_eq!(graph.detect_cycles(), Some(vec![0, 1]));
}

#[test]
fn topological_sort_rejects_cycle() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(1, 2).expect("edge");
    graph.add_edge(2, 0).expect("edge");

    let err = graph.topological_sort().expect_err("cycle");
    assert!(err.to_string().contains("cyclic feature graph"));
}

#[test]
fn dirty_propagate_walks_forward_breadth_first() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(0, 2).expect("edge");
    graph.add_edge(1, 3).expect("edge");
    graph.add_edge(2, 3).expect("edge");

    assert_eq!(graph.dirty_propagate(0), vec![0, 1, 2, 3]);
}

#[test]
fn dirty_propagate_from_leaf_returns_leaf_only() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(1, 2).expect("edge");

    assert_eq!(graph.dirty_propagate(2), vec![2]);
}

#[test]
fn duplicate_edges_are_deduplicated() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(0, 1).expect("edge");
    graph.add_edge(0, 1).expect("edge");

    assert_eq!(graph.edge_count(), 1);
    assert_eq!(graph.topological_sort().expect("topo"), vec![0, 1]);
}

#[test]
fn disconnected_components_sort_deterministically() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(2, 3).expect("edge");
    graph.add_edge(0, 1).expect("edge");

    assert_eq!(graph.topological_sort().expect("topo"), vec![0, 1, 2, 3]);
}

#[test]
fn empty_graph_has_empty_topological_order() {
    let graph = FeatureGraph::new();

    assert!(graph.topological_sort().expect("topo").is_empty());
    assert!(graph.detect_cycles().is_none());
}

#[test]
fn self_loop_is_reported_as_cycle() {
    let mut graph = FeatureGraph::new();
    graph.add_edge(4, 4).expect("edge");

    assert_eq!(graph.detect_cycles(), Some(vec![4]));
}

#[test]
fn body_new_starts_with_empty_graph() {
    let body = Body::new("Body");

    assert!(body.graph.is_empty());
}

#[test]
fn body_append_builds_default_linear_graph() {
    let body = body_with_specs(3);

    assert_eq!(body.graph.edge_count(), 2);
    assert_eq!(body.graph.topological_sort().expect("topo"), vec![0, 1, 2]);
}

#[test]
fn body_dependency_graph_falls_back_to_linear_for_legacy_body() {
    let mut body = body_with_specs(3);
    body.graph = FeatureGraph::new();

    assert_eq!(
        body.dependency_graph().topological_sort().expect("topo"),
        vec![0, 1, 2]
    );
}

#[test]
fn body_reorder_rebuilds_linear_graph() {
    let mut body = body_with_specs(3);

    assert!(body.move_feature_by_feature_id(1, 2));
    assert_eq!(body.graph.topological_sort().expect("topo"), vec![0, 1, 2]);
}

#[test]
fn session_recompute_populates_cache_from_linear_body_graph() {
    let (mut session, body_id, _, _) = session_with_two_helixes();

    let out = session
        .execute(Command::RecomputeBody { body: body_id })
        .expect("recompute");

    assert!(matches!(out, Outcome::FeatureRecomputed { .. }));
    assert_eq!(session.document().recompute_cache_len(), 2);
}

#[test]
fn edit_upstream_feature_reports_downstream_dirty_features() {
    let (mut session, _, first, second) = session_with_two_helixes();

    let out = session
        .execute(Command::EditFeature {
            feature: first,
            new_spec: helix_spec(3.5, 5.0),
        })
        .expect("edit");

    match out {
        Outcome::FeatureRecomputed {
            downstream_invalidated,
            ..
        } => {
            assert!(downstream_invalidated.contains(&first));
            assert!(downstream_invalidated.contains(&second));
        }
        other => panic!("expected FeatureRecomputed, got {other:?}"),
    }
}
