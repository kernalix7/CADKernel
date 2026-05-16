use cadkernel_api::{Command, SolidId, command_schemas};
use cadkernel_viewer::test_support::CadApp;

fn current_command_ops() -> &'static [&'static str] {
    &[
        "create_box",
        "create_cylinder",
        "create_sphere",
        "create_cone",
        "create_torus",
        "boolean_union",
        "boolean_subtract",
        "boolean_intersect",
        "translate",
        "scale",
        "scale_non_uniform",
        "center_on_origin",
        "align_to",
        "scale_to_fit",
        "translate_to",
        "rename",
        "delete_solid",
        "extrude",
        "linear_pattern",
        "mirror",
        "pad",
        "pocket",
        "revolve",
        "groove",
        "hole",
        "sweep",
        "loft",
        "helix",
        "fillet",
        "chamfer",
        "shell",
        "draft",
        "create_sketch",
        "edit_sketch",
        "delete_sketch",
        "map_sketch_to_face",
        "create_body",
        "set_tip",
        "suppress_feature",
        "reorder_feature",
        "recompute_body",
        "edit_feature",
        "new_document",
        "measure",
        "validate",
        "list_solids",
        "find_by_label",
        "history_events",
        "stats",
        "bounds",
        "distance",
        "volume",
        "surface_area",
        "centroid",
        "intersects_aabb",
        "exists",
        "diagonal",
        "aabb_center",
        "aabb_volume",
        "contains_aabb",
        "aabb_corners",
        "solid_label",
        "is_empty",
        "aabb_surface_area",
        "solid_count",
        "history_count",
        "has_label",
        "solid_ids",
        "aabb_extents",
        "aabb_longest_axis",
        "aabb_shortest_axis",
        "aabb_aspect_ratio",
        "is_cubic",
        "is_square_xy",
        "history_description",
        "is_square_yz",
        "is_square_xz",
        "operation_count",
        "last_operation",
        "has_operation",
        "first_operation",
        "duplicate",
        "rotate",
        "noop",
    ]
}

#[test]
fn palette_catalog_lists_every_current_command_variant() {
    let ops = CadApp::command_palette_api_ops_for_test();
    for expected in current_command_ops() {
        assert!(
            ops.contains(expected),
            "palette catalog is missing Command::{expected}"
        );
    }
}

#[test]
fn palette_catalog_covers_api_command_schema_surface() {
    let ops = CadApp::command_palette_api_ops_for_test();
    for schema in command_schemas() {
        assert!(
            ops.contains(&schema.op),
            "palette catalog missing schema op {}",
            schema.op
        );
    }
}

#[test]
fn palette_catalog_has_no_duplicate_api_ops() {
    let mut ops = CadApp::command_palette_api_ops_for_test();
    ops.sort_unstable();
    ops.dedup();
    assert_eq!(ops.len(), CadApp::command_palette_api_ops_for_test().len());
}

#[test]
fn palette_retains_ctrl_k_and_legacy_shortcuts() {
    let shortcuts = CadApp::command_palette_shortcuts_for_test();
    assert_eq!(shortcuts, ["Ctrl+K", "Ctrl+Shift+P"]);
}

#[test]
fn fuzzy_match_pad_finds_api_pad_command() {
    let matches = CadApp::command_palette_match_labels_for_test("pad");
    assert!(matches.iter().any(|label| label == "API: Pad"));
}

#[test]
fn fuzzy_match_alias_cube_finds_create_box() {
    let matches = CadApp::command_palette_match_labels_for_test("cube");
    assert!(matches.iter().any(|label| label == "API: Create Box"));
}

#[test]
fn build_create_box_command_from_palette_params() {
    let command =
        CadApp::build_palette_command_for_test("create_box", r#"{"dx":2.0,"dy":3.0,"dz":4.0}"#)
            .expect("command");
    assert_eq!(
        command,
        Command::CreateBox {
            dx: 2.0,
            dy: 3.0,
            dz: 4.0
        }
    );
}

#[test]
fn build_translate_command_from_palette_params() {
    let command = CadApp::build_palette_command_for_test(
        "translate",
        r#"{"id":0,"dx":1.0,"dy":2.0,"dz":3.0}"#,
    )
    .expect("command");
    assert_eq!(
        command,
        Command::Translate {
            id: SolidId(0),
            dx: 1.0,
            dy: 2.0,
            dz: 3.0
        }
    );
}

#[test]
fn build_noop_command_from_empty_palette_params() {
    let command = CadApp::build_palette_command_for_test("noop", "{}").expect("command");
    assert_eq!(command, Command::Noop);
}

#[test]
fn invalid_palette_json_reports_error() {
    let err = CadApp::build_palette_command_for_test("create_box", r#"{"dx":2.0"#)
        .expect_err("invalid json");
    assert!(err.contains("Invalid parameter JSON"));
}

#[test]
fn mismatched_palette_params_report_command_error() {
    let err = CadApp::build_palette_command_for_test("create_box", r#"{"dx":"bad"}"#)
        .expect_err("invalid params");
    assert!(err.contains("create_box"));
}

#[test]
fn palette_dispatch_create_box_executes_through_session() {
    let mut app = CadApp::new_headless();
    let command =
        CadApp::build_palette_command_for_test("create_box", r#"{"dx":2.0,"dy":2.0,"dz":2.0}"#)
            .expect("command");
    app.dispatch_api_command_for_test(command);
    assert_eq!(app.session_solid_count_for_test(), 1);
    assert_eq!(app.scene_ref().len(), 1);
    assert_eq!(app.session_history_len_for_test(), 1);
}
