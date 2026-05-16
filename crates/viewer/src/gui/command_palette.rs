//! Command palette — VS Code-style fuzzy command finder.
//!
//! Opens with **Ctrl + K** (Cmd + K on macOS) or the legacy
//! **Ctrl + Shift + P** binding. Type to
//! filter by command label; arrow keys navigate; Enter dispatches; Esc
//! closes. Results are scored by a simple fuzzy match (subsequence with
//! contiguous-bonus + start-of-word-bonus).
//!
//! Commands dispatch via the existing `gui.actions` queue — the palette
//! never executes work itself, it only enqueues `GuiAction` variants.
//!
//! Reference: `docs/COMMERCIAL_CAD_ROADMAP.md` Track B §B15 (search).

use super::theme;
use super::{GuiAction, GuiState};
use crate::render::{DisplayMode, StandardView};
use cadkernel_api::Command;
use serde_json::Value;

/// Internal state for the command palette popup.
#[derive(Default)]
pub(crate) struct CommandPaletteState {
    /// Whether the palette is currently visible.
    pub open: bool,
    /// Current search query.
    pub query: String,
    /// Index of the highlighted result (within the filtered list).
    pub selected: usize,
    /// Set to `true` for one frame after opening so the input grabs focus.
    pub request_focus: bool,
    pub api_param_op: Option<&'static str>,
    pub api_param_json: String,
    pub api_param_error: Option<String>,
}

impl CommandPaletteState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle the palette open/closed and reset state.
    pub fn toggle(&mut self) {
        if self.open {
            self.close();
        } else {
            self.open = true;
            self.query.clear();
            self.selected = 0;
            self.request_focus = true;
        }
    }

    pub fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected = 0;
        self.request_focus = false;
        self.api_param_op = None;
        self.api_param_json.clear();
        self.api_param_error = None;
    }
}

/// One palette entry. `run` is a function pointer that mutates `GuiState`
/// directly — most entries enqueue a `GuiAction`, but some (e.g. opening
/// a dialog) flip a UI flag instead.
struct Entry {
    label: &'static str,
    category: &'static str,
    shortcut: Option<&'static str>,
    run: fn(&mut GuiState),
}

struct ApiCommandDef {
    op: &'static str,
    aliases: &'static [&'static str],
    default_json: &'static str,
}

/// Static catalogue of palette-reachable commands.
///
/// New commands are added by appending an `Entry` here. Categories are
/// free-form strings shown in dim text after the label.
fn catalogue() -> Vec<Entry> {
    vec![
        // ---- File ----
        Entry {
            label: "New Model",
            category: "File",
            shortcut: Some("Ctrl+N"),
            run: |g| g.actions.push(GuiAction::NewModel),
        },
        Entry {
            label: "Clear Recent Files",
            category: "File",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::ClearRecentFiles),
        },
        // ---- View / Camera ----
        Entry {
            label: "Reset Camera",
            category: "View",
            shortcut: Some("Home"),
            run: |g| g.actions.push(GuiAction::ResetCamera),
        },
        Entry {
            label: "Fit All",
            category: "View",
            shortcut: Some("F"),
            run: |g| g.actions.push(GuiAction::FitAll),
        },
        Entry {
            label: "Toggle Projection",
            category: "View",
            shortcut: Some("5"),
            run: |g| g.actions.push(GuiAction::ToggleProjection),
        },
        Entry {
            label: "Toggle Grid",
            category: "View",
            shortcut: Some("G"),
            run: |g| g.actions.push(GuiAction::ToggleGrid),
        },
        Entry {
            label: "View: Front",
            category: "View",
            shortcut: Some("1"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Front))
            },
        },
        Entry {
            label: "View: Back",
            category: "View",
            shortcut: Some("Shift+1"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Back))
            },
        },
        Entry {
            label: "View: Right",
            category: "View",
            shortcut: Some("3"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Right))
            },
        },
        Entry {
            label: "View: Left",
            category: "View",
            shortcut: Some("Shift+3"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Left))
            },
        },
        Entry {
            label: "View: Top",
            category: "View",
            shortcut: Some("7"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Top))
            },
        },
        Entry {
            label: "View: Bottom",
            category: "View",
            shortcut: Some("Shift+7"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Bottom))
            },
        },
        Entry {
            label: "View: Isometric",
            category: "View",
            shortcut: Some("0"),
            run: |g| {
                g.actions
                    .push(GuiAction::SetStandardView(StandardView::Isometric))
            },
        },
        // ---- Display modes ----
        Entry {
            label: "Display: Shading",
            category: "Display",
            shortcut: Some(DisplayMode::Shading.shortcut()),
            run: |g| {
                g.actions
                    .push(GuiAction::SetDisplayMode(DisplayMode::Shading))
            },
        },
        Entry {
            label: "Display: Wireframe",
            category: "Display",
            shortcut: Some(DisplayMode::Wireframe.shortcut()),
            run: |g| {
                g.actions
                    .push(GuiAction::SetDisplayMode(DisplayMode::Wireframe))
            },
        },
        Entry {
            label: "Display: Hidden Line",
            category: "Display",
            shortcut: Some(DisplayMode::HiddenLine.shortcut()),
            run: |g| {
                g.actions
                    .push(GuiAction::SetDisplayMode(DisplayMode::HiddenLine))
            },
        },
        Entry {
            label: "Display: Flat Lines",
            category: "Display",
            shortcut: Some(DisplayMode::FlatLines.shortcut()),
            run: |g| {
                g.actions
                    .push(GuiAction::SetDisplayMode(DisplayMode::FlatLines))
            },
        },
        Entry {
            label: "Display: Points",
            category: "Display",
            shortcut: Some(DisplayMode::Points.shortcut()),
            run: |g| {
                g.actions
                    .push(GuiAction::SetDisplayMode(DisplayMode::Points))
            },
        },
        Entry {
            label: "Display: Transparent",
            category: "Display",
            shortcut: Some(DisplayMode::Transparent.shortcut()),
            run: |g| {
                g.actions
                    .push(GuiAction::SetDisplayMode(DisplayMode::Transparent))
            },
        },
        // ---- Edit ----
        Entry {
            label: "Undo",
            category: "Edit",
            shortcut: Some("Ctrl+Z"),
            run: |g| g.actions.push(GuiAction::Undo),
        },
        Entry {
            label: "Redo",
            category: "Edit",
            shortcut: Some("Ctrl+Shift+Z"),
            run: |g| g.actions.push(GuiAction::Redo),
        },
        Entry {
            label: "Select All",
            category: "Edit",
            shortcut: Some("Ctrl+A"),
            run: |g| g.actions.push(GuiAction::SelectAll),
        },
        Entry {
            label: "Deselect All",
            category: "Edit",
            shortcut: Some("Esc"),
            run: |g| g.actions.push(GuiAction::DeselectAll),
        },
        Entry {
            label: "Delete Selected",
            category: "Edit",
            shortcut: Some("Delete"),
            run: |g| g.actions.push(GuiAction::DeleteSelected),
        },
        // ---- Visibility ----
        Entry {
            label: "Show All",
            category: "Visibility",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::ShowAll),
        },
        Entry {
            label: "Hide All",
            category: "Visibility",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::HideAll),
        },
        // ---- Boolean (scene) ----
        Entry {
            label: "Boolean: Union (Selected)",
            category: "Boolean",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::BooleanSceneUnion),
        },
        Entry {
            label: "Boolean: Subtract (Selected)",
            category: "Boolean",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::BooleanSceneSubtract),
        },
        Entry {
            label: "Boolean: Intersect (Selected)",
            category: "Boolean",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::BooleanSceneIntersect),
        },
        // ---- Create ----
        Entry {
            label: "Create: Box (10×10×10)",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateBox {
                    width: 10.0,
                    height: 10.0,
                    depth: 10.0,
                })
            },
        },
        Entry {
            label: "Create: Cylinder (r=5, h=10)",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateCylinder {
                    radius: 5.0,
                    height: 10.0,
                })
            },
        },
        Entry {
            label: "Create: Sphere (r=5)",
            category: "Create",
            shortcut: None,
            run: |g| g.actions.push(GuiAction::CreateSphere { radius: 5.0 }),
        },
        Entry {
            label: "Create: Cone (5→0, h=10)",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateCone {
                    base_radius: 5.0,
                    top_radius: 0.0,
                    height: 10.0,
                })
            },
        },
        Entry {
            label: "Create: Torus (R=10, r=2)",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateTorus {
                    major_radius: 10.0,
                    minor_radius: 2.0,
                })
            },
        },
        Entry {
            label: "Create: Tube (R=10, r=8, h=10)",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateTube {
                    outer_radius: 10.0,
                    inner_radius: 8.0,
                    height: 10.0,
                })
            },
        },
        Entry {
            label: "Create: Wedge",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateWedge {
                    dx: 10.0,
                    dy: 10.0,
                    dz: 5.0,
                    dx2: 6.0,
                    dy2: 6.0,
                })
            },
        },
        Entry {
            label: "Create: Ellipsoid (5,3,2)",
            category: "Create",
            shortcut: None,
            run: |g| {
                g.actions.push(GuiAction::CreateEllipsoid {
                    rx: 5.0,
                    ry: 3.0,
                    rz: 2.0,
                })
            },
        },
        // ---- Help ----
        Entry {
            label: "Help: Keyboard Shortcuts",
            category: "Help",
            shortcut: Some("F1"),
            run: |g| {
                g.show_shortcuts = true;
            },
        },
    ]
}

const API_COMMANDS: &[ApiCommandDef] = &[
    ApiCommandDef {
        op: "create_box",
        aliases: &["box", "cube", "primitive"],
        default_json: r#"{"dx":10.0,"dy":10.0,"dz":10.0}"#,
    },
    ApiCommandDef {
        op: "create_cylinder",
        aliases: &["cylinder", "primitive"],
        default_json: r#"{"radius":5.0,"height":10.0}"#,
    },
    ApiCommandDef {
        op: "create_sphere",
        aliases: &["sphere", "primitive"],
        default_json: r#"{"radius":5.0}"#,
    },
    ApiCommandDef {
        op: "create_cone",
        aliases: &["cone", "frustum", "primitive"],
        default_json: r#"{"radius":5.0,"height":10.0,"top_radius":0.0}"#,
    },
    ApiCommandDef {
        op: "create_torus",
        aliases: &["torus", "donut", "primitive"],
        default_json: r#"{"major_radius":10.0,"minor_radius":2.0}"#,
    },
    ApiCommandDef {
        op: "boolean_union",
        aliases: &["union", "fuse", "boolean"],
        default_json: r#"{"lhs":0,"rhs":1}"#,
    },
    ApiCommandDef {
        op: "boolean_subtract",
        aliases: &["subtract", "cut", "difference", "boolean"],
        default_json: r#"{"lhs":0,"rhs":1}"#,
    },
    ApiCommandDef {
        op: "boolean_intersect",
        aliases: &["intersect", "common", "boolean"],
        default_json: r#"{"lhs":0,"rhs":1}"#,
    },
    ApiCommandDef {
        op: "translate",
        aliases: &["move", "transform"],
        default_json: r#"{"id":0,"dx":1.0,"dy":0.0,"dz":0.0}"#,
    },
    ApiCommandDef {
        op: "scale",
        aliases: &["uniform scale", "transform"],
        default_json: r#"{"id":0,"factor":2.0}"#,
    },
    ApiCommandDef {
        op: "scale_non_uniform",
        aliases: &["nonuniform scale", "transform"],
        default_json: r#"{"id":0,"factors":[1.0,1.0,1.0],"point":[0.0,0.0,0.0]}"#,
    },
    ApiCommandDef {
        op: "center_on_origin",
        aliases: &["center", "origin", "transform"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "align_to",
        aliases: &["align", "snap", "transform"],
        default_json: r#"{"id":0,"target_id":1}"#,
    },
    ApiCommandDef {
        op: "scale_to_fit",
        aliases: &["fit size", "normalize", "transform"],
        default_json: r#"{"id":0,"target_size":10.0}"#,
    },
    ApiCommandDef {
        op: "translate_to",
        aliases: &["move to", "transform"],
        default_json: r#"{"id":0,"point":[0.0,0.0,0.0]}"#,
    },
    ApiCommandDef {
        op: "rename",
        aliases: &["label", "name"],
        default_json: r#"{"id":0,"label":"Solid"}"#,
    },
    ApiCommandDef {
        op: "delete_solid",
        aliases: &["delete", "remove"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "extrude",
        aliases: &["extrusion", "profile"],
        default_json: r#"{"profile":[[0.0,0.0,0.0],[1.0,0.0,0.0],[1.0,1.0,0.0],[0.0,1.0,0.0]],"direction":[0.0,0.0,1.0],"distance":1.0}"#,
    },
    ApiCommandDef {
        op: "linear_pattern",
        aliases: &["pattern", "array"],
        default_json: r#"{"id":0,"direction":[1.0,0.0,0.0],"spacing":10.0,"count":3}"#,
    },
    ApiCommandDef {
        op: "mirror",
        aliases: &["reflect", "symmetry"],
        default_json: r#"{"id":0,"point":[0.0,0.0,0.0],"normal":[1.0,0.0,0.0]}"#,
    },
    ApiCommandDef {
        op: "pad",
        aliases: &["partdesign", "additive extrude"],
        default_json: r#"{"sketch":{"sketch_id":0},"distance":10.0}"#,
    },
    ApiCommandDef {
        op: "pocket",
        aliases: &["partdesign", "cut pocket"],
        default_json: r#"{"sketch":{"sketch_id":0},"distance":5.0}"#,
    },
    ApiCommandDef {
        op: "revolve",
        aliases: &["partdesign", "additive revolution"],
        default_json: r#"{"sketch":{"sketch_id":0},"axis":{"axis":"z"},"angle_rad":6.283185307179586}"#,
    },
    ApiCommandDef {
        op: "groove",
        aliases: &["partdesign", "subtractive revolution"],
        default_json: r#"{"sketch":{"sketch_id":0},"axis":{"axis":"z"},"angle_rad":6.283185307179586}"#,
    },
    ApiCommandDef {
        op: "hole",
        aliases: &["partdesign", "drill"],
        default_json: r#"{"face":{"solid":0,"tag":{"kind":"Face","segments":[{"operation":1,"kind":{"Generated":0}}]}},"position":[0.0,0.0],"radius":2.0,"depth":10.0}"#,
    },
    ApiCommandDef {
        op: "sweep",
        aliases: &["pipe", "path feature"],
        default_json: r#"{"profile_sketch":{"sketch_id":0},"path_sketch":{"sketch_id":0}}"#,
    },
    ApiCommandDef {
        op: "loft",
        aliases: &["blend profiles"],
        default_json: r#"{"profiles":[]}"#,
    },
    ApiCommandDef {
        op: "helix",
        aliases: &["coil", "thread"],
        default_json: r#"{"axis":{"axis":"z"},"radius":2.0,"pitch":1.0,"height":5.0,"turns":3.0}"#,
    },
    ApiCommandDef {
        op: "fillet",
        aliases: &["round", "edge"],
        default_json: r#"{"edges":[],"radius":1.0}"#,
    },
    ApiCommandDef {
        op: "chamfer",
        aliases: &["bevel", "edge"],
        default_json: r#"{"edges":[],"distance":1.0}"#,
    },
    ApiCommandDef {
        op: "shell",
        aliases: &["hollow", "thin wall"],
        default_json: r#"{"solid":0,"removed_faces":[],"thickness":1.0}"#,
    },
    ApiCommandDef {
        op: "draft",
        aliases: &["taper", "face"],
        default_json: r#"{"faces":[],"neutral_plane":{"solid":0,"tag":{"kind":"Face","segments":[{"operation":1,"kind":{"Generated":0}}]}},"angle_rad":0.08726646259971647}"#,
    },
    ApiCommandDef {
        op: "create_sketch",
        aliases: &["sketch", "new sketch"],
        default_json: r#"{"name":"Sketch"}"#,
    },
    ApiCommandDef {
        op: "edit_sketch",
        aliases: &["sketch edits"],
        default_json: r#"{"sketch":0,"edits":[]}"#,
    },
    ApiCommandDef {
        op: "delete_sketch",
        aliases: &["remove sketch"],
        default_json: r#"{"sketch":0}"#,
    },
    ApiCommandDef {
        op: "map_sketch_to_face",
        aliases: &["attach sketch", "map sketch"],
        default_json: r#"{"sketch":0,"face":{"solid":0,"tag":{"kind":"Face","segments":[{"operation":1,"kind":{"Generated":0}}]}}}"#,
    },
    ApiCommandDef {
        op: "create_body",
        aliases: &["body", "partdesign body"],
        default_json: r#"{"name":"Body"}"#,
    },
    ApiCommandDef {
        op: "set_tip",
        aliases: &["tip", "active feature"],
        default_json: r#"{"body":1,"feature":1}"#,
    },
    ApiCommandDef {
        op: "suppress_feature",
        aliases: &["suppress", "feature toggle"],
        default_json: r#"{"feature":1,"suppressed":true}"#,
    },
    ApiCommandDef {
        op: "reorder_feature",
        aliases: &["reorder", "move feature"],
        default_json: r#"{"from":1,"to_position":0}"#,
    },
    ApiCommandDef {
        op: "recompute_body",
        aliases: &["recompute", "body"],
        default_json: r#"{"body":1}"#,
    },
    ApiCommandDef {
        op: "edit_feature",
        aliases: &["edit spec", "feature spec"],
        default_json: r#"{"feature":1,"new_spec":{"kind":"helix","axis":{"axis":"z"},"radius":2.0,"pitch":1.0,"height":5.0,"turns":3.0}}"#,
    },
    ApiCommandDef {
        op: "new_document",
        aliases: &["reset document", "new"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "measure",
        aliases: &["mass properties", "measure"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "validate",
        aliases: &["health", "check"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "list_solids",
        aliases: &["solids", "list"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "find_by_label",
        aliases: &["find", "search label"],
        default_json: r#"{"query":""}"#,
    },
    ApiCommandDef {
        op: "history_events",
        aliases: &["history", "events"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "stats",
        aliases: &["statistics", "counts"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "bounds",
        aliases: &["bbox", "bounding box"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "distance",
        aliases: &["measure distance"],
        default_json: r#"{"id_a":0,"id_b":1}"#,
    },
    ApiCommandDef {
        op: "volume",
        aliases: &["measure volume"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "surface_area",
        aliases: &["area", "measure area"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "centroid",
        aliases: &["center of mass"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "intersects_aabb",
        aliases: &["overlap", "collision"],
        default_json: r#"{"id_a":0,"id_b":1}"#,
    },
    ApiCommandDef {
        op: "exists",
        aliases: &["solid exists"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "diagonal",
        aliases: &["bbox diagonal"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "aabb_center",
        aliases: &["bbox center"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "aabb_volume",
        aliases: &["bbox volume"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "contains_aabb",
        aliases: &["bbox contains"],
        default_json: r#"{"id_outer":0,"id_inner":1}"#,
    },
    ApiCommandDef {
        op: "aabb_corners",
        aliases: &["bbox corners"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "solid_label",
        aliases: &["label"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "is_empty",
        aliases: &["empty document"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "aabb_surface_area",
        aliases: &["bbox surface area"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "solid_count",
        aliases: &["count solids"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "history_count",
        aliases: &["count history"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "has_label",
        aliases: &["label exists"],
        default_json: r#"{"query":""}"#,
    },
    ApiCommandDef {
        op: "solid_ids",
        aliases: &["ids"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "aabb_extents",
        aliases: &["bbox extents"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "aabb_longest_axis",
        aliases: &["longest axis"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "aabb_shortest_axis",
        aliases: &["shortest axis"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "aabb_aspect_ratio",
        aliases: &["aspect ratio", "slenderness"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "is_cubic",
        aliases: &["cube test"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "is_square_xy",
        aliases: &["square xy"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "history_description",
        aliases: &["history item"],
        default_json: r#"{"index":0}"#,
    },
    ApiCommandDef {
        op: "is_square_yz",
        aliases: &["square yz"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "is_square_xz",
        aliases: &["square xz"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "operation_count",
        aliases: &["op count"],
        default_json: r#"{"op_name":"create_box"}"#,
    },
    ApiCommandDef {
        op: "last_operation",
        aliases: &["last history"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "has_operation",
        aliases: &["op exists"],
        default_json: r#"{"op_name":"create_box"}"#,
    },
    ApiCommandDef {
        op: "first_operation",
        aliases: &["first history"],
        default_json: r#"{}"#,
    },
    ApiCommandDef {
        op: "duplicate",
        aliases: &["clone", "copy"],
        default_json: r#"{"id":0}"#,
    },
    ApiCommandDef {
        op: "rotate",
        aliases: &["rotate", "transform"],
        default_json: r#"{"id":0,"axis":[0.0,0.0,1.0],"angle_rad":1.5707963267948966,"point":[0.0,0.0,0.0]}"#,
    },
    ApiCommandDef {
        op: "noop",
        aliases: &["ping"],
        default_json: r#"{}"#,
    },
];

fn api_command_defs() -> &'static [ApiCommandDef] {
    API_COMMANDS
}

fn api_label(op: &str) -> String {
    let mut label = String::from("API: ");
    for (idx, part) in op.split('_').enumerate() {
        if idx > 0 {
            label.push(' ');
        }
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            label.extend(first.to_uppercase());
            label.push_str(chars.as_str());
        }
    }
    label
}

fn api_category(op: &str) -> &'static str {
    if op.contains("aabb")
        || matches!(
            op,
            "measure"
                | "validate"
                | "list_solids"
                | "find_by_label"
                | "history_events"
                | "stats"
                | "bounds"
                | "distance"
                | "volume"
                | "surface_area"
                | "centroid"
                | "exists"
                | "diagonal"
                | "solid_label"
                | "is_empty"
                | "solid_count"
                | "history_count"
                | "has_label"
                | "solid_ids"
                | "is_cubic"
                | "is_square_xy"
                | "history_description"
                | "is_square_yz"
                | "is_square_xz"
                | "operation_count"
                | "last_operation"
                | "has_operation"
                | "first_operation"
        )
    {
        "API / Query"
    } else if matches!(
        op,
        "pad"
            | "pocket"
            | "revolve"
            | "groove"
            | "hole"
            | "sweep"
            | "loft"
            | "helix"
            | "fillet"
            | "chamfer"
            | "shell"
            | "draft"
            | "create_body"
            | "set_tip"
            | "suppress_feature"
            | "reorder_feature"
            | "recompute_body"
            | "edit_feature"
    ) {
        "API / PartDesign"
    } else if op.contains("sketch") {
        "API / Sketch"
    } else {
        "API"
    }
}

fn build_api_command(op: &str, json: &str) -> Result<Command, String> {
    let parsed: Value =
        serde_json::from_str(json).map_err(|err| format!("Invalid parameter JSON: {err}"))?;
    let mut object = match parsed {
        Value::Object(map) => map,
        _ => return Err("Parameter JSON must be an object".into()),
    };
    object.insert("op".to_string(), Value::String(op.to_string()));
    serde_json::from_value(Value::Object(object))
        .map_err(|err| format!("Command parameters do not match {op}: {err}"))
}

#[derive(Clone, Copy)]
enum ScoredEntry {
    Gui(usize),
    Api(usize),
}

enum PendingDispatch {
    Gui(fn(&mut GuiState)),
    Api(Command),
}

fn entry_score(query: &str, entry: &Entry) -> Option<i32> {
    fuzzy_score(query, entry.label)
}

fn api_score(query: &str, def: &ApiCommandDef) -> Option<i32> {
    let label = api_label(def.op);
    let mut best = fuzzy_score(query, &label).or_else(|| fuzzy_score(query, def.op));
    for alias in def.aliases {
        if let Some(score) = fuzzy_score(query, alias) {
            best = Some(best.map_or(score, |current| current.max(score + 4)));
        }
    }
    best
}

fn scored_entries(query: &str, gui_entries: &[Entry]) -> Vec<(i32, ScoredEntry)> {
    let mut scored: Vec<(i32, ScoredEntry)> = gui_entries
        .iter()
        .enumerate()
        .filter_map(|(idx, entry)| {
            entry_score(query, entry).map(|score| (score, ScoredEntry::Gui(idx)))
        })
        .collect();
    scored.extend(
        api_command_defs()
            .iter()
            .enumerate()
            .filter_map(|(idx, def)| {
                api_score(query, def).map(|score| (score, ScoredEntry::Api(idx)))
            }),
    );
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    scored
}

fn dispatch_for_entry(
    entry: ScoredEntry,
    gui_entries: &[Entry],
    state: &mut CommandPaletteState,
) -> Option<PendingDispatch> {
    match entry {
        ScoredEntry::Gui(idx) => Some(PendingDispatch::Gui(gui_entries[idx].run)),
        ScoredEntry::Api(idx) => {
            let def = &api_command_defs()[idx];
            match build_api_command(def.op, &state.api_param_json) {
                Ok(command) => {
                    state.api_param_error = None;
                    Some(PendingDispatch::Api(command))
                }
                Err(err) => {
                    state.api_param_error = Some(err);
                    None
                }
            }
        }
    }
}

fn ensure_api_param_state(def: &ApiCommandDef, state: &mut CommandPaletteState) {
    if state.api_param_op != Some(def.op) {
        state.api_param_op = Some(def.op);
        state.api_param_json = def.default_json.to_string();
        state.api_param_error = None;
    }
}

// ---------------------------------------------------------------------------
// Fuzzy match — subsequence scoring with start-of-word and contiguous bonus
// ---------------------------------------------------------------------------

/// Score `query` against `target`. Returns `Some(score)` if every character
/// in `query` appears in `target` in order (case-insensitive), else `None`.
/// Higher score = better match. Empty query scores 0 (match all).
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();
    let target_lower: Vec<char> = target.to_lowercase().chars().collect();

    let mut score: i32 = 0;
    let mut qi: usize = 0;
    let mut last_match_idx: Option<usize> = None;

    for (ti, tc) in target_lower.iter().enumerate() {
        if qi >= query_lower.len() {
            break;
        }
        if query_lower[qi] == *tc {
            // Base point for a match.
            score += 1;
            // Contiguous bonus: previous query char matched the previous target char.
            if let Some(prev) = last_match_idx
                && prev + 1 == ti
            {
                score += 5;
            }
            // Start-of-word bonus: matched at index 0, or after a space / colon.
            if ti == 0
                || matches!(
                    target_lower[ti - 1],
                    ' ' | ':' | '/' | '-' | '_' | '(' | ','
                )
            {
                score += 8;
            }
            last_match_idx = Some(ti);
            qi += 1;
        }
    }

    if qi == query_lower.len() {
        // Bonus for short targets (less noise).
        score += (50 - target.len() as i32).max(0);
        Some(score)
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// Detect the global Ctrl+K / Ctrl+Shift+P shortcuts and toggle the palette. Call
/// once per frame before `draw_command_palette`.
pub(crate) fn handle_global_shortcut(ctx: &egui::Context, state: &mut CommandPaletteState) {
    let triggered = ctx.input_mut(|i| {
        let ctrl_k = i.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND,
            egui::Key::K,
        ));
        let legacy = i.consume_shortcut(&egui::KeyboardShortcut::new(
            egui::Modifiers::COMMAND | egui::Modifiers::SHIFT,
            egui::Key::P,
        ));
        ctrl_k || legacy
    });
    if triggered {
        state.toggle();
    }
}

/// Render the palette modal. Pushes the chosen `GuiAction` (if any) into
/// `gui.actions` and closes the palette.
pub(crate) fn draw_command_palette(ctx: &egui::Context, gui: &mut GuiState) {
    if !gui.command_palette.open {
        return;
    }

    // Close on Escape.
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        gui.command_palette.close();
        return;
    }

    // Score and sort the catalogue once per frame.
    let entries = catalogue();
    let query = gui.command_palette.query.clone();
    let mut scored = scored_entries(&query, &entries);
    let max_results = 12usize;
    scored.truncate(max_results);

    // Clamp selected index.
    if scored.is_empty() {
        gui.command_palette.selected = 0;
    } else if gui.command_palette.selected >= scored.len() {
        gui.command_palette.selected = scored.len() - 1;
    }

    // Arrow-key navigation. Read inputs once for the frame.
    let (down_pressed, up_pressed, enter_pressed) = ctx.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowDown),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::Enter),
        )
    });
    if !scored.is_empty() {
        if down_pressed {
            gui.command_palette.selected = (gui.command_palette.selected + 1) % scored.len();
        }
        if up_pressed {
            gui.command_palette.selected = if gui.command_palette.selected == 0 {
                scored.len() - 1
            } else {
                gui.command_palette.selected - 1
            };
        }
    }

    let selected_api =
        scored
            .get(gui.command_palette.selected)
            .and_then(|(_, entry)| match entry {
                ScoredEntry::Api(idx) => Some(&api_command_defs()[*idx]),
                ScoredEntry::Gui(_) => None,
            });
    if let Some(def) = selected_api {
        ensure_api_param_state(def, &mut gui.command_palette);
    }

    // Decide whether to dispatch _after_ rendering (so the closed state
    // isn't inconsistent during the same frame).
    let mut to_dispatch: Option<PendingDispatch> = None;

    let screen = ctx.screen_rect();
    let palette_width = 600.0_f32.min(screen.width() - 32.0);
    let palette_height = 520.0_f32.min(screen.height() - 64.0);

    egui::Area::new(egui::Id::new("__command_palette"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 92.0))
        .show(ctx, |ui| {
            // Themed popup frame: dark panel with thin teal accent stroke +
            // soft drop shadow, matches the rest of the CADKernel chrome.
            let frame = egui::Frame {
                fill: egui::Color32::from_rgb(0x1C, 0x20, 0x28),
                stroke: egui::Stroke::new(1.0, theme::COLOR_ACCENT.gamma_multiply(0.55)),
                inner_margin: egui::Margin::same(0),
                corner_radius: egui::CornerRadius::same(10),
                shadow: egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(110),
                },
                ..egui::Frame::NONE
            };
            frame.show(ui, |ui| {
                ui.set_width(palette_width);
                ui.set_max_height(palette_height);

                // -- Search header --
                let header_h = 44.0;
                let header_rect = ui.allocate_space(egui::vec2(palette_width, header_h)).1;
                let painter = ui.painter();
                painter.rect_filled(
                    header_rect,
                    egui::CornerRadius {
                        nw: 10,
                        ne: 10,
                        sw: 0,
                        se: 0,
                    },
                    egui::Color32::from_rgb(0x16, 0x19, 0x20),
                );
                painter.line_segment(
                    [
                        egui::pos2(header_rect.left() + 8.0, header_rect.bottom() - 0.5),
                        egui::pos2(header_rect.right() - 8.0, header_rect.bottom() - 0.5),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0x10, 0x13, 0x19)),
                );
                // Magnifier glyph
                painter.text(
                    egui::pos2(header_rect.left() + 16.0, header_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "\u{1F50D}",
                    egui::FontId::proportional(15.0),
                    theme::COLOR_ACCENT,
                );

                // Place the text edit inside the header rect.
                let edit_rect = egui::Rect::from_min_size(
                    egui::pos2(header_rect.left() + 38.0, header_rect.top() + 8.0),
                    egui::vec2(palette_width - 76.0, header_h - 16.0),
                );
                let edit_resp = ui.put(
                    edit_rect,
                    egui::TextEdit::singleline(&mut gui.command_palette.query)
                        .hint_text("Type a command, view, or tool…")
                        .font(egui::FontId::proportional(14.0))
                        .frame(false)
                        .text_color(egui::Color32::from_rgb(220, 226, 235)),
                );
                if gui.command_palette.request_focus {
                    edit_resp.request_focus();
                    gui.command_palette.request_focus = false;
                }
                // Match-count chip on the right edge of the header.
                let chip_text = if scored.is_empty() {
                    "no matches".to_string()
                } else {
                    format!(
                        "{} match{}",
                        scored.len(),
                        if scored.len() == 1 { "" } else { "es" }
                    )
                };
                let chip_color = if scored.is_empty() {
                    egui::Color32::from_rgb(180, 110, 110)
                } else {
                    theme::COLOR_ACCENT
                };
                let painter = ui.painter();
                painter.text(
                    egui::pos2(header_rect.right() - 14.0, header_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    chip_text,
                    egui::FontId::proportional(10.5),
                    chip_color,
                );

                // -- Results list --
                if scored.is_empty() {
                    ui.add_space(28.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("No commands match \u{201C}")
                                .color(theme::COLOR_DIM)
                                .size(12.0),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "\u{201C}{}\u{201D}",
                                gui.command_palette.query
                            ))
                            .color(egui::Color32::from_rgb(200, 210, 225))
                            .size(13.0)
                            .italics(),
                        );
                    });
                    ui.add_space(20.0);
                } else {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .max_height(palette_height - header_h - 32.0)
                        .show(ui, |ui| {
                            ui.add_space(4.0);
                            for (visible_row, (_score, scored_entry)) in scored.iter().enumerate() {
                                let (label, category, shortcut, is_api) = match scored_entry {
                                    ScoredEntry::Gui(entry_idx) => {
                                        let entry = &entries[*entry_idx];
                                        (
                                            entry.label.to_string(),
                                            entry.category,
                                            entry.shortcut,
                                            false,
                                        )
                                    }
                                    ScoredEntry::Api(api_idx) => {
                                        let def = &api_command_defs()[*api_idx];
                                        (api_label(def.op), api_category(def.op), None, true)
                                    }
                                };
                                let selected = visible_row == gui.command_palette.selected;
                                let row_h = 34.0;
                                let (row_rect, row_resp) = ui.allocate_exact_size(
                                    egui::vec2(palette_width - 8.0, row_h),
                                    egui::Sense::click(),
                                );
                                // Inset the row so we get a 4px margin on each side.
                                let inset = row_rect.shrink2(egui::vec2(4.0, 1.0));
                                let painter = ui.painter();
                                let bg = if selected {
                                    theme::COLOR_ACCENT.gamma_multiply(0.18)
                                } else if row_resp.hovered() {
                                    egui::Color32::from_rgb(0x24, 0x29, 0x33)
                                } else {
                                    egui::Color32::TRANSPARENT
                                };
                                painter.rect_filled(inset, egui::CornerRadius::same(6), bg);
                                if selected {
                                    // Teal accent left bar.
                                    painter.rect_filled(
                                        egui::Rect::from_min_size(
                                            inset.min,
                                            egui::vec2(3.0, inset.height()),
                                        ),
                                        egui::CornerRadius {
                                            nw: 6,
                                            sw: 6,
                                            ne: 0,
                                            se: 0,
                                        },
                                        theme::COLOR_ACCENT,
                                    );
                                }

                                // Label
                                let label_color = if selected {
                                    egui::Color32::from_rgb(232, 238, 246)
                                } else {
                                    egui::Color32::from_rgb(210, 216, 226)
                                };
                                painter.text(
                                    egui::pos2(inset.left() + 14.0, inset.center().y),
                                    egui::Align2::LEFT_CENTER,
                                    label,
                                    egui::FontId::proportional(13.0),
                                    label_color,
                                );

                                // Category chip on the right side, before the shortcut.
                                let mut right_x = inset.right() - 8.0;
                                if let Some(sc) = shortcut {
                                    let font = egui::FontId::monospace(10.5);
                                    let g = painter.layout_no_wrap(
                                        sc.to_string(),
                                        font.clone(),
                                        theme::COLOR_DIM,
                                    );
                                    let pad = 6.0;
                                    let chip_w = g.size().x + pad * 2.0;
                                    let chip_rect = egui::Rect::from_min_size(
                                        egui::pos2(right_x - chip_w, inset.center().y - 8.0),
                                        egui::vec2(chip_w, 16.0),
                                    );
                                    painter.rect_filled(
                                        chip_rect,
                                        egui::CornerRadius::same(4),
                                        egui::Color32::from_rgb(0x14, 0x17, 0x1D),
                                    );
                                    painter.rect_stroke(
                                        chip_rect,
                                        egui::CornerRadius::same(4),
                                        egui::Stroke::new(
                                            0.6,
                                            egui::Color32::from_rgb(0x35, 0x3C, 0x48),
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                    painter.galley(
                                        egui::pos2(
                                            chip_rect.left() + pad,
                                            chip_rect.center().y - g.size().y * 0.5,
                                        ),
                                        g,
                                        theme::COLOR_DIM,
                                    );
                                    right_x = chip_rect.left() - 6.0;
                                }
                                // Category text right-aligned.
                                painter.text(
                                    egui::pos2(right_x, inset.center().y),
                                    egui::Align2::RIGHT_CENTER,
                                    category,
                                    egui::FontId::proportional(10.5),
                                    theme::COLOR_DIM,
                                );

                                if row_resp.clicked() {
                                    gui.command_palette.selected = visible_row;
                                    match scored_entry {
                                        ScoredEntry::Gui(_) => {
                                            to_dispatch = dispatch_for_entry(
                                                *scored_entry,
                                                &entries,
                                                &mut gui.command_palette,
                                            );
                                        }
                                        ScoredEntry::Api(api_idx) => {
                                            ensure_api_param_state(
                                                &api_command_defs()[*api_idx],
                                                &mut gui.command_palette,
                                            );
                                        }
                                    }
                                }
                                if is_api && row_resp.double_clicked() {
                                    if let ScoredEntry::Api(api_idx) = scored_entry {
                                        ensure_api_param_state(
                                            &api_command_defs()[*api_idx],
                                            &mut gui.command_palette,
                                        );
                                    }
                                    to_dispatch = dispatch_for_entry(
                                        *scored_entry,
                                        &entries,
                                        &mut gui.command_palette,
                                    );
                                }
                            }
                            ui.add_space(4.0);
                        });
                    if let Some(def) = selected_api {
                        ui.add_space(4.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("Parameters")
                                    .color(theme::COLOR_DIM)
                                    .size(10.5),
                            );
                            ui.label(
                                egui::RichText::new(def.op)
                                    .monospace()
                                    .color(theme::COLOR_ACCENT)
                                    .size(10.5),
                            );
                        });
                        ui.add(
                            egui::TextEdit::multiline(&mut gui.command_palette.api_param_json)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(palette_width - 18.0)
                                .desired_rows(5),
                        );
                        if let Some(err) = &gui.command_palette.api_param_error {
                            ui.label(
                                egui::RichText::new(err)
                                    .color(egui::Color32::from_rgb(220, 100, 95))
                                    .size(10.5),
                            );
                        }
                    }
                }

                // -- Footer hints --
                let footer_h = 26.0;
                let footer_rect = ui.allocate_space(egui::vec2(palette_width, footer_h)).1;
                let painter = ui.painter();
                painter.rect_filled(
                    footer_rect,
                    egui::CornerRadius {
                        nw: 0,
                        ne: 0,
                        sw: 10,
                        se: 10,
                    },
                    egui::Color32::from_rgb(0x16, 0x19, 0x20),
                );
                painter.line_segment(
                    [
                        egui::pos2(footer_rect.left() + 8.0, footer_rect.top() + 0.5),
                        egui::pos2(footer_rect.right() - 8.0, footer_rect.top() + 0.5),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(0x10, 0x13, 0x19)),
                );
                painter.text(
                    egui::pos2(footer_rect.left() + 14.0, footer_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    "\u{2191}\u{2193} navigate    \u{21B5} run    Esc close",
                    egui::FontId::proportional(10.5),
                    theme::COLOR_DIM,
                );
                painter.text(
                    egui::pos2(footer_rect.right() - 14.0, footer_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    "Ctrl+K",
                    egui::FontId::monospace(10.0),
                    egui::Color32::from_rgb(120, 130, 145),
                );
            });
        });

    // Enter dispatches the highlighted entry.
    if enter_pressed && !scored.is_empty() {
        let (_, entry) = scored[gui.command_palette.selected];
        to_dispatch = dispatch_for_entry(entry, &entries, &mut gui.command_palette);
    }

    if let Some(dispatch) = to_dispatch {
        match dispatch {
            PendingDispatch::Gui(run) => run(gui),
            PendingDispatch::Api(command) => {
                gui.actions.push(GuiAction::ExecuteApiCommand(command))
            }
        }
        if gui.command_palette.api_param_error.is_none() {
            gui.command_palette.close();
        }
    }
}

#[doc(hidden)]
pub(crate) fn api_catalog_ops_for_test() -> Vec<&'static str> {
    api_command_defs().iter().map(|def| def.op).collect()
}

#[doc(hidden)]
pub(crate) fn palette_match_labels_for_test(query: &str) -> Vec<String> {
    let entries = catalogue();
    let mut scored = scored_entries(query, &entries);
    scored.truncate(12);
    scored
        .into_iter()
        .map(|(_, entry)| match entry {
            ScoredEntry::Gui(idx) => entries[idx].label.to_string(),
            ScoredEntry::Api(idx) => api_label(api_command_defs()[idx].op),
        })
        .collect()
}

#[doc(hidden)]
pub(crate) fn build_api_command_for_test(op: &str, json: &str) -> Result<Command, String> {
    build_api_command(op, json)
}

#[doc(hidden)]
pub(crate) fn shortcut_labels_for_test() -> [&'static str; 2] {
    ["Ctrl+K", "Ctrl+Shift+P"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_all() {
        assert_eq!(fuzzy_score("", "anything"), Some(0));
    }

    #[test]
    fn out_of_order_does_not_match() {
        assert!(fuzzy_score("xy", "yx").is_none());
    }

    #[test]
    fn subsequence_matches() {
        assert!(fuzzy_score("vfr", "View: Front").is_some());
    }

    #[test]
    fn contiguous_beats_scattered() {
        let contiguous = fuzzy_score("box", "box").unwrap();
        let scattered = fuzzy_score("box", "back of xenon").unwrap();
        assert!(
            contiguous > scattered,
            "contiguous score {contiguous} should beat scattered {scattered}"
        );
    }

    #[test]
    fn catalogue_is_nonempty() {
        assert!(!catalogue().is_empty());
    }

    #[test]
    fn toggle_open_close() {
        let mut s = CommandPaletteState::new();
        assert!(!s.open);
        s.toggle();
        assert!(s.open);
        assert!(s.request_focus);
        s.toggle();
        assert!(!s.open);
    }
}
