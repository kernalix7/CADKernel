//! egui-based UI panels for the CAD application.

mod context_menu;
mod dialogs;
mod menu;
mod overlays;
mod properties;
mod report;
mod sketch_ui;
mod status_bar;
pub(crate) mod task_panel;
pub(crate) mod theme;
mod toolbar;
mod tree;
mod view_cube;

use crate::nav::NavConfig;
use crate::render::{Camera, DisplayMode, GridConfig, StandardView};
use cadkernel_io::Mesh;
use cadkernel_modeling::MassProperties;
use cadkernel_sketch::{Constraint, Sketch, WorkPlane};
use cadkernel_topology::{
    BRepModel, EdgeData, FaceData, Handle, ShellData, SolidData, VertexData,
};
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Workbench system
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Workbench {
    Part,
    PartDesign,
    Sketcher,
    Mesh,
    TechDraw,
    Assembly,
    Draft,
    Surface,
    Fem,
}

impl Workbench {
    pub const ALL: &[Workbench] = &[
        Workbench::Part,
        Workbench::PartDesign,
        Workbench::Sketcher,
        Workbench::Mesh,
        Workbench::TechDraw,
        Workbench::Assembly,
        Workbench::Draft,
        Workbench::Surface,
        Workbench::Fem,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Part => "\u{2B22} Part",
            Self::PartDesign => "\u{2699} PartDesign",
            Self::Sketcher => "\u{270F} Sketcher",
            Self::Mesh => "\u{25A6} Mesh",
            Self::TechDraw => "\u{1F4D0} TechDraw",
            Self::Assembly => "\u{1F527} Assembly",
            Self::Draft => "\u{2712} Draft",
            Self::Surface => "\u{223F} Surface",
            Self::Fem => "\u{2206} FEM",
        }
    }
}

// ---------------------------------------------------------------------------
// Sketch editing mode
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SketchTool {
    Select,
    Line,
    Rectangle,
    Circle,
    Arc,
    Point,
    Ellipse,
    Polyline,
    Slot,
    BSpline,
    Polygon { sides: u32 },
}

/// Kind of dimension constraint being edited in the popup.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum DimensionKind {
    Distance,
    Radius,
    Angle,
    Length,
    HDistance,
    VDistance,
    Diameter,
}

/// State for the dimension input popup shown after clicking a dimensional constraint button.
pub(crate) struct DimensionPopup {
    pub kind: DimensionKind,
    pub value: f64,
    pub just_opened: bool,
    /// If editing an existing constraint, its index.
    pub edit_constraint_index: Option<usize>,
}

/// Reference to a specific sketch entity for selection/editing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SketchEntityRef {
    Point(usize),
    Line(usize),
    Arc(usize),
    Circle(usize),
    Ellipse(usize),
    BSpline(usize),
}

/// Full sketch state snapshot for undo/redo.
#[derive(Clone)]
pub(crate) struct SketchSnapshot {
    pub sketch: Sketch,
}

pub(crate) struct SketchMode {
    pub sketch: Sketch,
    pub plane: WorkPlane,
    pub tool: SketchTool,
    pub pending_point: Option<(f64, f64)>,
    pub extrude_distance: f64,
    pub construction_mode: bool,
    pub show_grid: bool,
    pub snap_enabled: bool,
    pub show_constraints: bool,
    pub polyline_points: Vec<(f64, f64)>,
    pub undo_stack: Vec<SketchSnapshot>,
    pub redo_stack: Vec<SketchSnapshot>,
    pub selected_entities: Vec<SketchEntityRef>,
    pub hovered_entity: Option<SketchEntityRef>,
    pub drag_points: Vec<usize>,
    pub drag_origin: Option<(f64, f64)>,
    pub drag_started: bool,
    /// Box selection: start screen coordinates
    pub box_select_start: Option<(f64, f64)>,
    /// Box selection: current screen coordinates
    pub box_select_end: Option<(f64, f64)>,
    pub show_context_menu: bool,
    pub constraint_residuals: Vec<f64>,
    pub solver_converged: bool,
    pub grid_spacing: f64,
    /// Clipboard: copied points as (dx, dy) offsets relative to centroid.
    pub clipboard_points: Vec<(f64, f64)>,
    /// Clipboard: copied lines as (start_idx, end_idx) into clipboard_points.
    pub clipboard_lines: Vec<(usize, usize)>,
    /// Validation issues from last check.
    pub validation_issues: Vec<cadkernel_sketch::SketchValidationIssue>,
}

impl SketchMode {
    pub fn new(plane: WorkPlane) -> Self {
        Self {
            sketch: Sketch::new(),
            plane,
            tool: SketchTool::Line,
            pending_point: None,
            extrude_distance: 10.0,
            construction_mode: false,
            show_grid: true,
            snap_enabled: true,
            show_constraints: true,
            polyline_points: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            selected_entities: Vec::new(),
            hovered_entity: None,
            drag_points: Vec::new(),
            drag_origin: None,
            drag_started: false,
            box_select_start: None,
            box_select_end: None,
            show_context_menu: false,
            constraint_residuals: Vec::new(),
            solver_converged: true,
            grid_spacing: 1.0,
            clipboard_points: Vec::new(),
            clipboard_lines: Vec::new(),
            validation_issues: Vec::new(),
        }
    }

    /// Save a full clone of the current sketch state before a sketch operation.
    pub fn save_snapshot(&mut self) {
        self.undo_stack.push(SketchSnapshot { sketch: self.sketch.clone() });
        self.redo_stack.clear();
    }

    /// Undo: restore the previous sketch state.
    pub fn undo(&mut self) -> bool {
        if let Some(snap) = self.undo_stack.pop() {
            self.redo_stack.push(SketchSnapshot { sketch: self.sketch.clone() });
            self.sketch = snap.sketch;
            self.pending_point = None;
            self.selected_entities.clear();
            true
        } else {
            false
        }
    }

    /// Redo: restore the next sketch state.
    pub fn redo(&mut self) -> bool {
        if let Some(snap) = self.redo_stack.pop() {
            self.undo_stack.push(SketchSnapshot { sketch: self.sketch.clone() });
            self.sketch = snap.sketch;
            self.selected_entities.clear();
            true
        } else {
            false
        }
    }

    /// Compute degrees of freedom: 2 * points - constraint_dofs.
    pub fn degrees_of_freedom(&self) -> i32 {
        let pt_dofs = (self.sketch.points.len() * 2) as i32;
        let mut c_dofs = 0i32;
        for c in &self.sketch.constraints {
            c_dofs += match c {
                Constraint::Coincident(..) => 2,
                Constraint::Horizontal(_) | Constraint::Vertical(_) => 1,
                Constraint::Parallel(..) | Constraint::Perpendicular(..) => 1,
                Constraint::PointOnLine(..) => 1,
                Constraint::PointOnCircle(..) => 1,
                Constraint::Symmetric(..) => 2,
                Constraint::Distance(..) => 1,
                Constraint::Angle(..) => 1,
                Constraint::Radius(..) => 1,
                Constraint::Length(..) => 1,
                Constraint::Fixed(..) => 2,
                Constraint::Tangent(..) => 1,
                Constraint::EqualLength(..) => 1,
                Constraint::Midpoint(..) => 2,
                Constraint::Collinear(..) => 2,
                Constraint::EqualRadius(..) => 1,
                Constraint::Concentric(..) => 2,
                Constraint::Diameter(..) => 1,
                Constraint::Block(..) => 2,
                Constraint::HorizontalDistance(..) => 1,
                Constraint::VerticalDistance(..) => 1,
                Constraint::PointOnObject(..) => 1,
                Constraint::Refraction { .. } => 1,
            };
        }
        pt_dofs - c_dofs
    }

    /// Recompute per-constraint residuals, solver status, and validation issues.
    pub fn update_constraint_status(&mut self) {
        if self.sketch.constraints.is_empty() {
            self.constraint_residuals.clear();
            self.solver_converged = true;
        } else {
            self.constraint_residuals = cadkernel_sketch::constraint_residuals(&self.sketch);
            self.solver_converged = self.constraint_residuals.iter().all(|r| *r < 1e-6);
        }
        let validation = cadkernel_sketch::validate_sketch(&self.sketch, 0.01);
        self.validation_issues = validation.issues;
    }
}

// ---------------------------------------------------------------------------
// Entity selection
// ---------------------------------------------------------------------------

/// Selection mode determines what gets picked in the 3D viewport.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum SelectionMode {
    Solid,
    Face,
    Edge,
    Vertex,
}

/// Transform gizmo mode for the selected object.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub(crate) enum GizmoMode {
    None,
    Translate,
    Rotate,
    Scale,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub(crate) enum SelectedEntity {
    Solid(Handle<SolidData>),
    Shell(Handle<ShellData>),
    Face(Handle<FaceData>),
    Edge(Handle<EdgeData>),
    Vertex(Handle<VertexData>),
}

// ---------------------------------------------------------------------------
// Report / log levels
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReportLevel {
    Info,
    Warning,
    Error,
}

// ---------------------------------------------------------------------------
// Toast notifications
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ToastLevel {
    Success,
    Info,
    Warning,
    Error,
}

/// A floating toast notification with auto-dismiss.
#[derive(Clone)]
pub(crate) struct Toast {
    pub level: ToastLevel,
    pub message: String,
    /// When this toast was created.
    pub created_at: std::time::Instant,
}

// ---------------------------------------------------------------------------
// Actions emitted by the GUI
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MirrorPlane {
    XY,
    XZ,
    YZ,
}

#[allow(dead_code)]
pub(crate) enum GuiAction {
    NewModel,
    OpenFile(PathBuf),
    SaveFile(PathBuf),
    ClearRecentFiles,
    ImportFile(PathBuf),
    ExportStl(PathBuf),
    ExportStlWithOptions { path: PathBuf, binary: bool, scale: f64 },
    ExportObj(PathBuf),
    ExportGltf(PathBuf),
    ExportStep(PathBuf),
    ExportIges(PathBuf),
    ExportDxf(PathBuf),
    ExportPly(PathBuf),
    Export3mf(PathBuf),
    ExportBrep(PathBuf),
    CreateBox {
        width: f64,
        height: f64,
        depth: f64,
    },
    CreateCylinder {
        radius: f64,
        height: f64,
    },
    CreateSphere {
        radius: f64,
    },
    CreateCone {
        base_radius: f64,
        top_radius: f64,
        height: f64,
    },
    CreateTorus {
        major_radius: f64,
        minor_radius: f64,
    },
    CreateTube {
        outer_radius: f64,
        inner_radius: f64,
        height: f64,
    },
    CreatePrism {
        radius: f64,
        height: f64,
        sides: usize,
    },
    CreateWedge {
        dx: f64,
        dy: f64,
        dz: f64,
        dx2: f64,
        dy2: f64,
    },
    CreateEllipsoid {
        rx: f64,
        ry: f64,
        rz: f64,
    },
    CreateHelix {
        radius: f64,
        pitch: f64,
        turns: f64,
        tube_radius: f64,
    },
    ResetCamera,
    FitAll,
    /// Frame the camera on a single object's bounding box (no scene-wide fit).
    /// Dispatched by double-clicking an object row in the scene tree.
    FocusObject(crate::scene::ObjectId),
    ToggleProjection,
    SetGizmoMode(GizmoMode),
    SetDisplayMode(DisplayMode),
    SetStandardView(StandardView),
    SetCameraYawPitch(f32, f32),
    ScreenOrbit(f32, f32),
    RollDelta(f32),
    ToggleGrid,
    Undo,
    Redo,
    BooleanUnionWith {
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
    },
    BooleanSubtractWith {
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
    },
    BooleanIntersectWith {
        width: f64,
        height: f64,
        depth: f64,
        offset: [f64; 3],
    },
    StatusMessage(String),
    MirrorSolid(MirrorPlane),
    ScaleSolid {
        factor: f64,
    },
    ShellSolid {
        thickness: f64,
    },
    FilletAllEdges {
        radius: f64,
    },
    ChamferAllEdges {
        distance: f64,
    },
    LinearPattern {
        count: usize,
        spacing: f64,
        axis: u8,
    },
    EnterSketch(WorkPlane),
    SketchOnSelectedFace,
    SketchClick(f64, f64),
    SetSketchTool(SketchTool),
    SketchConstrainHorizontal,
    SketchConstrainVertical,
    SketchConstrainLength(f64),
    // Sketch constraint expansion
    SketchConstrainParallel,
    SketchConstrainPerpendicular,
    SketchConstrainCoincident,
    SketchConstrainTangent,
    SketchConstrainEqual,
    SketchConstrainSymmetric,
    SketchConstrainFixed,
    SketchConstrainBlock,
    SketchConstrainDistance(f64),
    SketchConstrainAngle(f64),
    SketchConstrainRadius(f64),
    SketchConstrainDiameter(f64),
    SketchConstrainHDistance(f64),
    SketchConstrainVDistance(f64),
    // Sketch tools
    SketchFilletCorner { radius: f64 },
    SketchChamferCorner { distance: f64 },
    SketchTrimEdge,
    SketchSplitEdge,
    SketchExtendEdge,
    SketchMirrorGeometry,
    SketchExternalProjection,
    SketchCarbonCopy,
    SketchCopySelection,
    SketchPasteSelection(f64, f64),
    SketchMergePoints,
    // Sketch B-spline
    SketchConvertToBSpline,
    SketchIncreaseDegree,
    SketchDecreaseDegree,
    SketchInsertKnot,
    // Sketch toggles
    ToggleSketchConstruction,
    ToggleSketchGrid,
    ToggleSketchSnap,
    ToggleSketchConstraintsVisible,
    CloseSketch,
    CancelSketch,
    EditSketch,
    TechDrawAddView(cadkernel_io::ProjectionDir),
    TechDrawThreeView,
    TechDrawExportSvg(PathBuf),
    TechDrawClear,
    MeshDecimate(f64),
    MeshSubdivide,
    MeshFlipNormals,
    MeshFillHoles,
    MeshSmooth {
        iterations: usize,
        factor: f64,
    },
    MeshHarmonizeNormals,
    MeshCheckWatertight,
    MeshRemesh {
        target_edge_len: f64,
    },
    MeshRepair,
    MeasureSolid,
    CheckGeometry,
    SelectAll,
    DeselectAll,
    DeleteSelected,
    // Scene management
    SelectObject(crate::scene::ObjectId),
    ToggleVisibility(crate::scene::ObjectId),
    RemoveObject(crate::scene::ObjectId),
    DuplicateObject(crate::scene::ObjectId),
    RenameObject(crate::scene::ObjectId, String),
    ShowAll,
    HideAll,
    // Parametric rebuild + color + task preview
    RebuildObject { id: crate::scene::ObjectId, params: crate::scene::CreationParams },
    SetObjectColor { id: crate::scene::ObjectId, color: [f32; 4] },
    TaskPreviewUpdate(task_panel::ActiveTask),
    // Transform operations
    MoveObject { id: crate::scene::ObjectId, dx: f64, dy: f64, dz: f64 },
    RotateObject { id: crate::scene::ObjectId, axis: u8, angle_deg: f64 },
    ScaleObjectUniform { id: crate::scene::ObjectId, factor: f64 },
    // Multi-select
    ToggleSelect(crate::scene::ObjectId),
    SetSelectionMode(SelectionMode),
    // Loop/ring selection
    SelectEdgeLoop,
    SelectEdgeRing,
    SelectFaceLoop,
    // Scene boolean (pick two objects)
    BooleanSceneUnion,
    BooleanSceneSubtract,
    BooleanSceneIntersect,

    // -- Part: Join operations --
    FaceFromWires,
    ConnectShapes,
    EmbedShapes,
    CutoutShapes,
    // -- Part: Compound operations --
    ExplodeCompound,
    CompoundFilter,
    BooleanFragments,
    SliceToCompound,
    // -- Part: Convert operations --
    PointsFromShape,
    ConvertToSolid,
    AutoDefeaturing { threshold: f64 },
    TransformedCopy { dx: f64, dy: f64, dz: f64 },
    ProjectCurvesOnSurface,
    CoonsPatch,

    // -- PartDesign: Feature operations --
    PadSketch { depth: f64, symmetric: bool },
    PocketSketch { depth: f64, through_all: bool },
    GrooveSketch { angle: f64 },
    HoleSketch { radius: f64, depth: f64 },
    CountersunkHoleSketch { radius: f64, depth: f64, countersink_angle: f64 },
    AdditiveLoft,
    AdditivePipe,
    SubtractiveLoft,
    SubtractivePipe,
    CreateSprocket { teeth: u32, roller_diameter: f64, pitch: f64, bore: f64 },
    CreateShaftDesign { segments: Vec<(f64, f64)> },
    CreateInvoluteGear { teeth: u32, module_val: f64, pressure_angle: f64 },
    ShapeBinder,
    SuppressFeature,
    SetTip,
    MoveFeatureUp,
    MoveFeatureDown,

    // -- Assembly workbench --
    CreateAssembly,
    InsertComponent,
    SolveAssembly,
    ExplodedView { factor: f64 },
    BillOfMaterials,
    DOFAnalysis,
    AddAssemblyJoint(AssemblyJointType),
    ToggleAssemblyComponentVisibility(usize),
    CommitAssemblyJoint,

    // -- Draft workbench --
    DraftLine,
    DraftWire,
    DraftCircle,
    DraftArc,
    DraftEllipse,
    DraftRectangle,
    DraftPolygon,
    DraftBSpline,
    DraftBezier,
    DraftPoint,
    DraftFacebinder,
    DraftHatch,
    DraftMove,
    DraftRotate,
    DraftScale,
    DraftMirror,
    DraftOffset,
    DraftTrim,
    DraftStretch,
    DraftClone,
    DraftArrayRect,
    DraftArrayPolar,
    DraftArrayPath,
    DraftArrayPoint,
    DraftDimension,
    DraftLabel,
    DraftText,
    DraftUpgrade,
    DraftDowngrade,
    DraftWireToBSpline,
    DraftToSketch,
    SetDraftLayer(String),
    ToggleDraftSnap(String),

    // -- Surface workbench --
    SurfaceFilling,
    SurfaceBoundary,
    SurfaceSections,
    SurfaceExtend,
    SurfaceBlend,
    SurfacePipe,
    SurfaceCoons,

    // -- FEM workbench --
    CreateFemAnalysis,
    SetFemMaterial(String),
    OpenMaterialPicker,
    CommitMaterialPicker,
    OpenBcEditor(BcKind),
    CommitBcEditor,
    GenTetMesh { element_size: f64 },
    GenHexMesh { nx: u32, ny: u32, nz: u32 },
    AddFemConstraint(FemConstraintType),
    SolveStatic,
    SolveModal { modes: usize },
    SolveThermal,
    SolveBuckling { modes: usize },
    SolveNonlinear,
    ShowStress,
    ShowDisplacement,
    ShowVonMises,
    FemSummary,
    FemReport,

    // -- TechDraw expanded --
    TechDrawNewPage,
    TechDrawFromTemplate,
    TechDrawRedraw,
    TechDrawSectionView,
    TechDrawDetailView,
    TechDrawBrokenView,
    TechDrawDimLinear,
    TechDrawDimRadius,
    TechDrawDimDiameter,
    TechDrawDimAngle,
    TechDrawDimArcLen,
    TechDrawDimArea,
    TechDrawText,
    TechDrawRichText,
    TechDrawBalloon,
    TechDrawLeader,
    TechDrawWeld,
    TechDrawSurfFinish,
    TechDrawCenterFace,
    TechDrawCenterLines,
    TechDrawCenterPoints,
    TechDrawBoltCircle,
    TechDrawExportDxf(PathBuf),
    TechDrawExportPdf(PathBuf),

    // -- I/O expanded --
    ImportSvg(PathBuf),
    ImportGltf(PathBuf),
    Import3mf(PathBuf),
    ImportDae(PathBuf),
    ExportSvg(PathBuf),
    ExportDae(PathBuf),

    // -- Viewport overlays --
    ToggleOrigin,
    ToggleGrid3d,
    ToggleMeasurement,
    AddMeasurementPoint([f32; 3]),
    ClearMeasurement,

    // -- Section plane --
    ToggleSectionPlane,

    // -- View bookmarks --
    SaveBookmark(String),
    RestoreBookmark(usize),
    DeleteBookmark(usize),

    // -- Object grouping --
    CreateGroup(String),
    GroupSelected(u32),
    UngroupObject(crate::scene::ObjectId),
    ToggleGroupVisibility(u32),
    DeleteGroup(u32),

    // -- Theme / density --
    ThemeToggle,
    DensityChange(theme::UiDensity),

    // -- Report --
    ClearReport,

    // -- Scripting / Plugins / MCP --
    ExecuteLuaCode(String),
    ExecuteLuaFile(String),
    ClearLuaConsole,
    TogglePluginManager,
    StartMcpServer,
    StopMcpServer,
    InitPlugins,
}

// -- Assembly joint types --
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum AssemblyJointType {
    Grounded,
    Fixed,
    Revolute,
    Cylindrical,
    Slider,
    Ball,
    Distance,
    Angle,
    Parallel,
    Perpendicular,
    Gear,
    Rack,
    Screw,
    Belt,
}

impl AssemblyJointType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Grounded => "Grounded",
            Self::Fixed => "Fixed",
            Self::Revolute => "Revolute",
            Self::Cylindrical => "Cylindrical",
            Self::Slider => "Slider",
            Self::Ball => "Ball",
            Self::Distance => "Distance",
            Self::Angle => "Angle",
            Self::Parallel => "Parallel",
            Self::Perpendicular => "Perpendicular",
            Self::Gear => "Gear",
            Self::Rack => "Rack",
            Self::Screw => "Screw",
            Self::Belt => "Belt",
        }
    }

    /// Minimum number of components required in the assembly to open a
    /// joint editor of this type. Grounded attaches to a single component;
    /// all other joints require two distinct components.
    pub fn min_components(self) -> usize {
        match self {
            Self::Grounded => 1,
            _ => 2,
        }
    }
}

// -- FEM constraint types --
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FemConstraintType {
    Fixed,
    Force,
    Pressure,
    Displacement,
    Gravity,
    Spring,
}

// -- FEM material picker / BC editor (Phase O-a) --
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MaterialPreset {
    Steel, Aluminum, Titanium, Copper, Concrete, CastIron, Custom,
}

impl MaterialPreset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Steel => "Steel", Self::Aluminum => "Aluminum",
            Self::Titanium => "Titanium", Self::Copper => "Copper",
            Self::Concrete => "Concrete", Self::CastIron => "Cast Iron",
            Self::Custom => "Custom",
        }
    }
    /// Short description (E, ν, ρ).
    pub fn description(self) -> &'static str {
        match self {
            Self::Steel => "E=210 GPa, \u{03BD}=0.30, \u{03C1}=7850 kg/m\u{00B3}",
            Self::Aluminum => "E=70 GPa, \u{03BD}=0.33, \u{03C1}=2700 kg/m\u{00B3}",
            Self::Titanium => "E=114 GPa, \u{03BD}=0.34, \u{03C1}=4430 kg/m\u{00B3}",
            Self::Copper => "E=117 GPa, \u{03BD}=0.34, \u{03C1}=8960 kg/m\u{00B3}",
            Self::Concrete => "E=30 GPa, \u{03BD}=0.20, \u{03C1}=2400 kg/m\u{00B3}",
            Self::CastIron => "E=170 GPa, \u{03BD}=0.26, \u{03C1}=7200 kg/m\u{00B3}",
            Self::Custom => "User-specified properties",
        }
    }
    /// Build the matching `FemMaterial` for this preset. Custom falls back to
    /// steel; real custom path uses [`material_from_preset`].
    pub fn to_material(self) -> cadkernel_modeling::FemMaterial {
        use cadkernel_modeling::FemMaterial;
        match self {
            Self::Steel => FemMaterial::steel(), Self::Aluminum => FemMaterial::aluminum(),
            Self::Titanium => FemMaterial::titanium(), Self::Copper => FemMaterial::copper(),
            Self::Concrete => FemMaterial::concrete(), Self::CastIron => FemMaterial::cast_iron(),
            Self::Custom => FemMaterial::steel(),
        }
    }
}

/// Build a `FemMaterial` from a preset + user-provided custom values. For
/// non-Custom presets the custom values are ignored. Invalid Custom inputs
/// fall back to steel.
pub(crate) fn material_from_preset(p: MaterialPreset, e: f64, nu: f64, rho: f64)
    -> cadkernel_modeling::FemMaterial
{
    use cadkernel_modeling::FemMaterial;
    match p {
        MaterialPreset::Custom => FemMaterial::custom(e, nu, rho)
            .unwrap_or_else(|_| FemMaterial::steel()),
        _ => p.to_material(),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MaterialPickerState {
    pub selected: MaterialPreset,
    pub youngs_modulus: f64,
    pub poisson_ratio: f64,
    pub density: f64,
}
impl MaterialPickerState {
    pub fn new() -> Self {
        Self { selected: MaterialPreset::Steel, youngs_modulus: 210.0e9,
            poisson_ratio: 0.3, density: 7850.0 }
    }
}

// Boundary-condition kinds exposed by the viewer's BC editor.
//
// Phase O-a shipped FixedNode + Force. Phase O-b extends to 12 scalar /
// single-node / Vec3-only variants. The four remaining kernel-side variants
// (`TieConstraint`, `RigidBody`, `ContactConstraint`, `SectionPrint`) are
// **deferred** here — they require a multi-node-set picker UX and a separate
// session.
//
// Field overload: `BcEditorState` carries a single `vec3_x/y/z` triple that is
// re-labelled by the dialog per `BcKind` (force / displacement / acceleration
// / load / axis / direction / gravity / force-density). The dialog's
// visibility gate guarantees only the relevant inputs are shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BcKind {
    FixedNode,
    Force,
    Pressure,
    Displacement,
    Gravity,
    DistributedLoad,
    Spring,
    CentrifugalLoad,
    SelfWeight,
    SpringConstraint,
    BodyLoad,
    InitialTemperature,
}

/// Which of the BC editor's input groups a given `BcKind` consumes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct BcInputs {
    pub node: bool,
    pub element: bool,
    pub vec3: bool,
    pub scalar: bool,
}

impl BcKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::FixedNode => "Fixed Node",
            Self::Force => "Force",
            Self::Pressure => "Pressure",
            Self::Displacement => "Displacement",
            Self::Gravity => "Gravity",
            Self::DistributedLoad => "Distributed Load",
            Self::Spring => "Spring",
            Self::CentrifugalLoad => "Centrifugal Load",
            Self::SelfWeight => "Self Weight",
            Self::SpringConstraint => "Spring Constraint",
            Self::BodyLoad => "Body Load",
            Self::InitialTemperature => "Initial Temperature",
        }
    }
    /// Visibility gate: which input groups are needed for each kind.
    pub fn inputs(self) -> BcInputs {
        let (n, e, v, s) = match self {
            Self::FixedNode          => (true,  false, false, false),
            Self::Force              => (true,  false, true,  false),
            Self::Pressure           => (false, true,  false, true),
            Self::Displacement       => (true,  false, true,  false),
            Self::Gravity            => (false, false, true,  false),
            Self::DistributedLoad    => (false, true,  true,  false),
            Self::Spring             => (true,  false, false, true),
            Self::CentrifugalLoad    => (false, false, true,  true),
            Self::SelfWeight         => (false, false, true,  false),
            Self::SpringConstraint   => (true,  false, true,  true),
            Self::BodyLoad           => (false, false, true,  false),
            Self::InitialTemperature => (true,  false, false, true),
        };
        BcInputs { node: n, element: e, vec3: v, scalar: s }
    }
    /// Label used for the Vec3 input group, when shown.
    pub fn vec3_label(self) -> &'static str {
        match self {
            Self::Force            => "Force (N)",
            Self::Displacement     => "Displacement (m)",
            Self::Gravity          => "Acceleration (m/s\u{00B2})",
            Self::DistributedLoad  => "Load (N/m\u{00B2})",
            Self::CentrifugalLoad  => "Axis",
            Self::SelfWeight       => "Gravity (m/s\u{00B2})",
            Self::SpringConstraint => "Direction",
            Self::BodyLoad         => "Force Density (N/m\u{00B3})",
            _ => "Vector",
        }
    }
    /// Label used for the scalar input, when shown.
    pub fn scalar_label(self) -> &'static str {
        match self {
            Self::Pressure           => "Pressure (Pa):",
            Self::Spring             => "Stiffness (N/m):",
            Self::CentrifugalLoad    => "Omega (rad/s):",
            Self::SpringConstraint   => "Stiffness (N/m):",
            Self::InitialTemperature => "Temperature (K):",
            _ => "Scalar:",
        }
    }
    /// Whether the legacy Force XYZ fields should be visible for this BC kind.
    /// Retained for back-compat with existing tests.
    #[cfg(test)]
    pub fn shows_force_fields(self) -> bool { self.inputs().vec3 }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct BcEditorState {
    pub bc_kind: BcKind,
    pub node_index: usize,
    pub element_index: usize,
    /// Re-labelled per `BcKind`: force / displacement / acceleration / load /
    /// axis / direction / gravity / force_density. See `BcKind::vec3_label`.
    pub vec3_x: f64,
    pub vec3_y: f64,
    pub vec3_z: f64,
    /// Re-labelled per `BcKind`: pressure / stiffness / omega / temperature.
    /// See `BcKind::scalar_label`.
    pub scalar_a: f64,
}
impl BcEditorState {
    pub fn new(bc_kind: BcKind) -> Self {
        Self {
            bc_kind,
            node_index: 0,
            element_index: 0,
            vec3_x: 0.0, vec3_y: 0.0, vec3_z: 0.0,
            scalar_a: 0.0,
        }
    }
    fn vec3(self) -> cadkernel_math::Vec3 {
        cadkernel_math::Vec3 { x: self.vec3_x, y: self.vec3_y, z: self.vec3_z }
    }
    /// Build the `BoundaryCondition` matching this editor state.
    pub fn to_boundary_condition(self) -> cadkernel_modeling::BoundaryCondition {
        use cadkernel_modeling::BoundaryCondition as BC;
        match self.bc_kind {
            BcKind::FixedNode => BC::FixedNode(self.node_index),
            BcKind::Force => BC::Force { node: self.node_index, force: self.vec3() },
            BcKind::Pressure => BC::Pressure {
                element: self.element_index, pressure: self.scalar_a,
            },
            BcKind::Displacement => BC::Displacement {
                node: self.node_index, displacement: self.vec3(),
            },
            BcKind::Gravity => BC::Gravity { acceleration: self.vec3() },
            BcKind::DistributedLoad => BC::DistributedLoad {
                element: self.element_index, load: self.vec3(),
            },
            BcKind::Spring => BC::Spring {
                node: self.node_index, stiffness: self.scalar_a,
            },
            BcKind::CentrifugalLoad => BC::CentrifugalLoad {
                axis: self.vec3(), omega: self.scalar_a,
            },
            BcKind::SelfWeight => BC::SelfWeight { gravity: self.vec3() },
            BcKind::SpringConstraint => BC::SpringConstraint {
                node_id: self.node_index,
                stiffness: self.scalar_a,
                direction: self.vec3(),
            },
            BcKind::BodyLoad => BC::BodyLoad { force_density: self.vec3() },
            BcKind::InitialTemperature => BC::InitialTemperature {
                node: self.node_index, temperature: self.scalar_a,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// GUI state persisted across frames
// ---------------------------------------------------------------------------

pub(crate) struct GuiState {
    pub show_model_tree: bool,
    pub tree_filter: String,
    pub show_properties: bool,
    pub property_tab: properties::PropertyTab,
    pub show_about: bool,
    pub show_settings: bool,
    pub show_create_box: bool,
    pub show_create_cylinder: bool,
    pub show_create_sphere: bool,
    pub show_create_cone: bool,
    pub show_create_torus: bool,
    pub show_create_tube: bool,
    pub show_create_prism: bool,
    pub show_create_wedge: bool,
    pub show_create_ellipsoid: bool,
    pub show_create_helix: bool,
    pub status_message: String,
    pub current_file: Option<String>,

    pub create_box_size: [f64; 3],
    pub create_cylinder_radius: f64,
    pub create_cylinder_height: f64,
    pub create_sphere_radius: f64,
    pub create_cone_base_radius: f64,
    pub create_cone_top_radius: f64,
    pub create_cone_height: f64,
    pub create_torus_major_radius: f64,
    pub create_torus_minor_radius: f64,
    pub create_tube_outer_radius: f64,
    pub create_tube_inner_radius: f64,
    pub create_tube_height: f64,
    pub create_prism_radius: f64,
    pub create_prism_height: f64,
    pub create_prism_sides: usize,
    pub create_wedge_dx: f64,
    pub create_wedge_dy: f64,
    pub create_wedge_dz: f64,
    pub create_wedge_dx2: f64,
    pub create_wedge_dy2: f64,
    pub create_ellipsoid_rx: f64,
    pub create_ellipsoid_ry: f64,
    pub create_ellipsoid_rz: f64,
    pub create_helix_radius: f64,
    pub create_helix_pitch: f64,
    pub create_helix_turns: f64,
    pub create_helix_tube_radius: f64,

    pub actions: Vec<GuiAction>,
    pub request_quit: bool,
    pub show_view_menu: bool,
    pub active_workbench: Workbench,

    pub sketch_mode: Option<SketchMode>,
    pub last_sketch: Option<(Sketch, WorkPlane)>,
    pub constraint_length_value: f64,
    pub constraint_distance_value: f64,
    pub constraint_angle_value: f64,
    pub constraint_radius_value: f64,
    pub sketch_fillet_radius: f64,
    pub sketch_chamfer_distance: f64,
    pub dimension_popup: Option<DimensionPopup>,

    pub techdraw_sheet: Option<cadkernel_io::DrawingSheet>,

    pub assembly: Option<cadkernel_modeling::Assembly>,

    pub fem_analysis: Option<cadkernel_modeling::AnalysisContainer>,

    // Boolean dialog state
    pub show_boolean_union: bool,
    pub show_boolean_subtract: bool,
    pub show_boolean_intersect: bool,
    pub bool_box_size: [f64; 3],
    pub bool_offset: [f64; 3],

    // Part operation dialog state
    pub show_mirror: bool,
    pub mirror_plane: MirrorPlane,
    pub show_scale: bool,
    pub scale_factor: f64,
    pub show_shell: bool,
    pub shell_thickness: f64,
    pub show_fillet: bool,
    pub fillet_radius: f64,
    pub show_chamfer: bool,
    pub chamfer_distance: f64,
    pub show_pattern: bool,
    pub pattern_count: usize,
    pub pattern_spacing: f64,
    pub pattern_axis: u8,

    // PartDesign dialog state
    pub show_pad: bool,
    pub pad_depth: f64,
    pub pad_symmetric: bool,
    pub show_pocket: bool,
    pub pocket_depth: f64,
    pub pocket_through_all: bool,
    pub show_groove: bool,
    pub groove_angle: f64,
    pub show_hole: bool,
    pub hole_radius: f64,
    pub hole_depth: f64,
    pub hole_countersink: bool,
    pub hole_countersink_angle: f64,
    pub show_sprocket: bool,
    pub sprocket_teeth: u32,
    pub sprocket_roller_diameter: f64,
    pub sprocket_pitch: f64,
    pub sprocket_bore: f64,
    pub show_shaft: bool,
    pub shaft_segments: Vec<(f64, f64)>,
    pub show_gear: bool,
    pub gear_teeth: u32,
    pub gear_module: f64,
    pub gear_pressure_angle: f64,
    pub show_defeaturing: bool,
    pub defeaturing_threshold: f64,

    // FEM dialog state
    pub show_fem_material: bool,
    pub fem_material_preset: String,
    pub show_fem_mesh: bool,
    pub fem_element_size: f64,
    pub show_fem_constraint: bool,
    pub fem_constraint_type: FemConstraintType,
    pub fem_constraint_value: f64,
    pub show_fem_solver: bool,
    pub fem_solver_tolerance: f64,
    pub fem_solver_max_iter: usize,
    // Phase O-a: material picker + BC editor
    pub material_picker_dialog: Option<MaterialPickerState>,
    pub bc_editor_dialog: Option<BcEditorState>,
    pub pending_fem_material: cadkernel_modeling::FemMaterial,
    // Export options dialog state
    pub show_export_options: bool,
    pub export_path: Option<PathBuf>,
    pub export_stl_binary: bool,
    pub export_scale: f64,

    // Assembly dialog state
    pub show_explode: bool,
    pub explode_factor: f64,
    pub show_bom_dialog: bool,
    pub bom_entries: Vec<cadkernel_modeling::BomEntry>,
    pub show_joint_editor: bool,
    pub joint_editor_type: Option<AssemblyJointType>,
    pub joint_editor_comp_a: usize,
    pub joint_editor_comp_b: usize,
    pub joint_editor_axis: [f32; 3],
    pub joint_editor_origin: [f32; 3],
    pub joint_editor_angle: f64,
    pub joint_editor_pitch: f64,
    pub joint_editor_ratio: f64,
    // Draft state
    #[allow(dead_code)]
    pub draft_active_layer: String,
    pub draft_snap_modes: [bool; 8],

    // Mesh dialog state
    pub show_mesh_smooth: bool,
    pub mesh_smooth_iters: usize,
    pub mesh_smooth_factor: f64,
    pub show_mesh_remesh: bool,
    pub mesh_remesh_edge_len: f64,

    // Entity selection (Block 1)
    pub selected_entities: Vec<SelectedEntity>,
    pub preselected_entity: Option<SelectedEntity>,
    pub preselected_object_id: Option<crate::scene::ObjectId>,
    pub selection_mode: SelectionMode,
    pub active_task: Option<task_panel::ActiveTask>,

    /// Egui pointer position in physical pixels (scale-correct, updated each frame).
    pub pointer_physical: Option<(f32, f32)>,

    // Report panel (Block 5)
    pub report_lines: Vec<(ReportLevel, String)>,
    pub show_report_panel: bool,
    pub bottom_tab: report::BottomTab,
    pub console_history: Vec<String>,
    pub console_input: String,
    pub recent_files: Vec<String>,
    /// Object being renamed in tree: (object_id, current_text)
    pub rename_edit: Option<(crate::scene::ObjectId, String)>,
    /// Splash screen frame counter (shown for first ~60 frames).
    pub splash_frames: u32,
    pub theme_applied: bool,
    pub show_viewport_context_menu: bool,
    pub show_shortcuts: bool,

    // Mouse world position for status bar (Block 4)
    pub mouse_world_pos: Option<[f64; 3]>,

    // Measurement overlay
    pub measurement_mode: bool,
    pub measurement_points: Vec<[f32; 3]>,

    // Snap visualization
    pub snap_highlight: Option<SnapHighlight>,

    // Display mode override (from properties panel)
    pub display_mode_override: Option<DisplayMode>,

    /// Current display mode (mirrored from viewport each frame for toolbar access).
    pub tb_display_mode: DisplayMode,

    /// Viewport rect (central area not claimed by panels), in physical pixels.
    pub viewport_rect: Option<[u32; 4]>,

    cached_props: Option<MassProperties>,
    cached_props_tri_count: usize,

    // Box selection (rubber band)
    pub box_select_start: Option<(f32, f32)>,
    pub box_select_end: Option<(f32, f32)>,

    // ViewCube drag rotation
    pub cube_dragging: bool,
    pub cube_drag_moved: bool,

    // Transform gizmo
    pub gizmo_mode: GizmoMode,
    /// Hovered gizmo axis: 0=X, 1=Y, 2=Z, None=no hover.
    pub gizmo_hover_axis: Option<u8>,

    // View bookmarks
    pub bookmark_name_input: String,
    /// Bookmark data mirrored from NavConfig: (name, yaw_deg, pitch_deg, distance).
    pub nav_bookmarks: Vec<(String, f32, f32, f32)>,

    // Object grouping
    pub group_name_input: String,
    /// Group info mirrored from Scene for menu display: (id, name, visible, member_count).
    pub scene_groups: Vec<(u32, String, bool, usize)>,

    // Lua console state
    pub lua_input: String,
    /// (input, output, is_error)
    pub lua_history: Vec<(String, String, bool)>,

    // Plugin manager
    pub show_plugin_manager: bool,

    // MCP server running flag
    pub mcp_running: bool,

    // Undo/Redo history for display
    pub history_entries: Vec<String>,
    pub future_entries: Vec<String>,

    // Toast notifications
    pub toasts: Vec<Toast>,

    // Toolbar context (set each frame before drawing)
    pub tb_can_undo: bool,
    pub tb_can_redo: bool,
    pub tb_has_selection: bool,
    pub tb_has_objects: bool,
    pub tb_in_sketch: bool,
    #[allow(dead_code)]
    pub tb_object_count: usize,
}

/// Snap visualization hint displayed in the viewport.
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) enum SnapHighlight {
    Vertex([f32; 3]),
    GridPoint([f32; 3]),
}

impl GuiState {
    pub fn new() -> Self {
        Self {
            show_model_tree: true,
            tree_filter: String::new(),
            show_properties: true,
            property_tab: properties::PropertyTab::Data,
            show_about: false,
            show_settings: false,
            show_create_box: false,
            show_create_cylinder: false,
            show_create_sphere: false,
            show_create_cone: false,
            show_create_torus: false,
            show_create_tube: false,
            show_create_prism: false,
            show_create_wedge: false,
            show_create_ellipsoid: false,
            show_create_helix: false,
            status_message: "Ready".into(),
            current_file: None,
            create_box_size: [10.0, 10.0, 10.0],
            create_cylinder_radius: 5.0,
            create_cylinder_height: 10.0,
            create_sphere_radius: 5.0,
            create_cone_base_radius: 5.0,
            create_cone_top_radius: 0.0,
            create_cone_height: 10.0,
            create_torus_major_radius: 5.0,
            create_torus_minor_radius: 1.5,
            create_tube_outer_radius: 5.0,
            create_tube_inner_radius: 3.0,
            create_tube_height: 10.0,
            create_prism_radius: 5.0,
            create_prism_height: 10.0,
            create_prism_sides: 6,
            create_wedge_dx: 10.0,
            create_wedge_dy: 10.0,
            create_wedge_dz: 10.0,
            create_wedge_dx2: 5.0,
            create_wedge_dy2: 5.0,
            create_ellipsoid_rx: 5.0,
            create_ellipsoid_ry: 3.0,
            create_ellipsoid_rz: 2.0,
            create_helix_radius: 5.0,
            create_helix_pitch: 3.0,
            create_helix_turns: 3.0,
            create_helix_tube_radius: 0.5,
            actions: Vec::new(),
            request_quit: false,
            show_view_menu: false,
            active_workbench: Workbench::Part,
            sketch_mode: None,
            last_sketch: None,
            constraint_length_value: 10.0,
            constraint_distance_value: 10.0,
            constraint_angle_value: 90.0,
            constraint_radius_value: 5.0,
            sketch_fillet_radius: 1.0,
            sketch_chamfer_distance: 1.0,
            dimension_popup: None,
            techdraw_sheet: None,
            assembly: None,
            fem_analysis: None,
            show_boolean_union: false,
            show_boolean_subtract: false,
            show_boolean_intersect: false,
            bool_box_size: [5.0, 5.0, 5.0],
            bool_offset: [5.0, 0.0, 0.0],
            show_mirror: false,
            mirror_plane: MirrorPlane::YZ,
            show_scale: false,
            scale_factor: 2.0,
            show_shell: false,
            shell_thickness: 1.0,
            show_fillet: false,
            fillet_radius: 1.0,
            show_chamfer: false,
            chamfer_distance: 1.0,
            show_pattern: false,
            pattern_count: 3,
            pattern_spacing: 15.0,
            pattern_axis: 0,
            show_pad: false,
            pad_depth: 10.0,
            pad_symmetric: false,
            show_pocket: false,
            pocket_depth: 5.0,
            pocket_through_all: false,
            show_groove: false,
            groove_angle: 360.0,
            show_hole: false,
            hole_radius: 2.0,
            hole_depth: 10.0,
            hole_countersink: false,
            hole_countersink_angle: 90.0,
            show_sprocket: false,
            sprocket_teeth: 18,
            sprocket_roller_diameter: 10.16,
            sprocket_pitch: 15.875,
            sprocket_bore: 5.0,
            show_shaft: false,
            shaft_segments: vec![(20.0, 10.0), (30.0, 15.0), (20.0, 10.0)],
            show_gear: false,
            gear_teeth: 20,
            gear_module: 2.0,
            gear_pressure_angle: 20.0,
            show_defeaturing: false,
            defeaturing_threshold: 1.0,
            show_fem_material: false,
            fem_material_preset: "Steel".into(),
            show_fem_mesh: false,
            fem_element_size: 1.0,
            show_fem_constraint: false,
            fem_constraint_type: FemConstraintType::Fixed,
            fem_constraint_value: 100.0,
            show_fem_solver: false,
            fem_solver_tolerance: 1e-6,
            fem_solver_max_iter: 1000,
            material_picker_dialog: None,
            bc_editor_dialog: None,
            pending_fem_material: cadkernel_modeling::FemMaterial::steel(),
            show_export_options: false,
            export_path: None,
            export_stl_binary: true,
            export_scale: 1.0,
            show_explode: false,
            explode_factor: 2.0,
            show_bom_dialog: false,
            bom_entries: Vec::new(),
            show_joint_editor: false,
            joint_editor_type: None,
            joint_editor_comp_a: 0,
            joint_editor_comp_b: 1,
            joint_editor_axis: [0.0, 0.0, 1.0],
            joint_editor_origin: [0.0, 0.0, 0.0],
            joint_editor_angle: 90.0,
            joint_editor_pitch: 1.0,
            joint_editor_ratio: 1.0,
            draft_active_layer: "Default".into(),
            draft_snap_modes: [true; 8],
            show_mesh_smooth: false,
            mesh_smooth_iters: 3,
            mesh_smooth_factor: 0.5,
            show_mesh_remesh: false,
            mesh_remesh_edge_len: 1.0,
            selected_entities: Vec::new(),
            preselected_entity: None,
            preselected_object_id: None,
            selection_mode: SelectionMode::Solid,
            active_task: None,
            pointer_physical: None,
            report_lines: Vec::new(),
            show_report_panel: true,
            bottom_tab: report::BottomTab::Report,
            console_history: Vec::new(),
            console_input: String::new(),
            recent_files: Vec::new(),
            rename_edit: None,
            splash_frames: 0,
            theme_applied: false,
            show_viewport_context_menu: false,
            show_shortcuts: false,
            mouse_world_pos: None,
            measurement_mode: false,
            measurement_points: Vec::new(),
            snap_highlight: None,
            display_mode_override: None,
            tb_display_mode: DisplayMode::Shading,
            viewport_rect: None,
            cached_props: None,
            cached_props_tri_count: 0,
            box_select_start: None,
            box_select_end: None,
            cube_dragging: false,
            cube_drag_moved: false,
            gizmo_mode: GizmoMode::Translate,
            gizmo_hover_axis: None,
            bookmark_name_input: String::new(),
            nav_bookmarks: Vec::<(String, f32, f32, f32)>::new(),
            group_name_input: String::new(),
            scene_groups: Vec::<(u32, String, bool, usize)>::new(),
            lua_input: String::new(),
            lua_history: Vec::new(),
            show_plugin_manager: false,
            mcp_running: false,
            history_entries: Vec::new(),
            future_entries: Vec::new(),
            toasts: Vec::new(),
            tb_can_undo: false,
            tb_can_redo: false,
            tb_has_selection: false,
            tb_has_objects: false,
            tb_in_sketch: false,
            tb_object_count: 0,
        }
    }

    pub fn invalidate_cache(&mut self) {
        self.cached_props = None;
        self.cached_props_tri_count = 0;
    }

    pub fn log(&mut self, level: ReportLevel, msg: impl Into<String>) {
        self.report_lines.push((level, msg.into()));
    }

    // ---- Assembly dispatcher helpers (testable without CadApp) ----

    /// Flip visibility of component `idx` in the current assembly.
    /// Returns true on success, false if no assembly or out-of-bounds.
    pub fn toggle_assembly_component_visibility(&mut self, idx: usize) -> bool {
        let asm = match self.assembly.as_mut() {
            Some(a) => a,
            None => return false,
        };
        let comp = match asm.components.get(idx) {
            Some(c) => c,
            None => return false,
        };
        let new_vis = !comp.visible;
        let cid = comp.id;
        asm.set_visible(cid, new_vis).is_ok()
    }

    /// Populate BOM entries from the current assembly. Returns true if an
    /// assembly exists and BOM was computed.
    pub fn populate_bom_entries(&mut self) -> bool {
        if let Some(asm) = self.assembly.as_ref() {
            self.bom_entries = asm.bill_of_materials();
            self.show_bom_dialog = true;
            true
        } else {
            false
        }
    }

    /// Open the joint editor for the given type. Returns false (no-op) if
    /// the current assembly has fewer components than the joint variant
    /// requires (1 for Grounded, 2 for every other type).
    pub fn open_joint_editor(&mut self, jtype: AssemblyJointType) -> bool {
        let n = self.assembly.as_ref().map(|a| a.num_components()).unwrap_or(0);
        if n < jtype.min_components() {
            return false;
        }
        self.joint_editor_type = Some(jtype);
        self.joint_editor_comp_a = 0;
        // When there's only one component (Grounded case), pin comp_b to 0.
        self.joint_editor_comp_b = if n >= 2 { 1 } else { 0 };
        self.show_joint_editor = true;
        true
    }

    /// Commit the currently-edited joint to `assembly.joints` based on the
    /// editor state. Returns true on success.
    pub fn commit_assembly_joint(&mut self) -> bool {
        use cadkernel_modeling::JointType;
        let jtype = match self.joint_editor_type {
            Some(t) => t,
            None => return false,
        };
        let asm = match self.assembly.as_mut() {
            Some(a) => a,
            None => return false,
        };
        let a = self.joint_editor_comp_a;
        let b = self.joint_editor_comp_b;
        let axis = cadkernel_math::Vec3 {
            x: self.joint_editor_axis[0] as f64,
            y: self.joint_editor_axis[1] as f64,
            z: self.joint_editor_axis[2] as f64,
        };
        let origin = cadkernel_math::Point3 {
            x: self.joint_editor_origin[0] as f64,
            y: self.joint_editor_origin[1] as f64,
            z: self.joint_editor_origin[2] as f64,
        };
        let joint = match jtype {
            AssemblyJointType::Grounded => JointType::Grounded,
            AssemblyJointType::Fixed => JointType::FixedJoint { component_a: a, component_b: b },
            AssemblyJointType::Revolute => JointType::Revolute {
                component_a: a, component_b: b, axis, origin,
            },
            AssemblyJointType::Cylindrical => JointType::Cylindrical {
                component_a: a, component_b: b, axis, origin,
            },
            AssemblyJointType::Slider => JointType::Slider {
                component_a: a, component_b: b, axis,
            },
            AssemblyJointType::Ball => JointType::BallJoint {
                component_a: a, component_b: b, center: origin,
            },
            AssemblyJointType::Distance => JointType::AngleJoint {
                component_a: a, component_b: b, angle: self.joint_editor_angle,
            },
            AssemblyJointType::Angle => JointType::AngleJoint {
                component_a: a, component_b: b, angle: self.joint_editor_angle,
            },
            AssemblyJointType::Parallel => JointType::ParallelAxes {
                component_a: a, component_b: b, axis_a: axis, axis_b: axis,
            },
            AssemblyJointType::Perpendicular => JointType::PerpendicularAxes {
                component_a: a, component_b: b, axis_a: axis, axis_b: axis,
            },
            AssemblyJointType::Gear => JointType::GearJoint {
                component_a: a, component_b: b, ratio: self.joint_editor_ratio,
            },
            AssemblyJointType::Rack => JointType::RackAndPinion {
                component_a: a, component_b: b, pitch_radius: self.joint_editor_pitch,
            },
            AssemblyJointType::Screw => JointType::ScrewJoint {
                component_a: a, component_b: b, axis, pitch: self.joint_editor_pitch,
            },
            AssemblyJointType::Belt => JointType::BeltJoint {
                component_a: a, component_b: b, ratio: self.joint_editor_ratio,
            },
        };
        asm.add_joint(joint);
        self.show_joint_editor = false;
        self.joint_editor_type = None;
        true
    }
}

// ---------------------------------------------------------------------------
// Top-level draw entry
// ---------------------------------------------------------------------------

pub(crate) struct ViewportInfo<'a> {
    pub camera: &'a Camera,
    pub display_mode: DisplayMode,
    pub grid_config: &'a GridConfig,
    pub show_grid: bool,
    pub fps: f32,
    pub show_fps: bool,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_ui(
    ctx: &egui::Context,
    gui: &mut GuiState,
    nav: &mut NavConfig,
    vp: &ViewportInfo<'_>,
    _model: &BRepModel,
    _mesh: &Option<Mesh>,
    scene: &crate::scene::Scene,
) {
    // Apply theme (mode-aware: re-applies on theme/density change)
    if !gui.theme_applied {
        theme::CadTheme::from_mode(nav.theme_mode)
            .with_density(nav.ui_density)
            .apply_to_egui(ctx);
        gui.theme_applied = true;
    }

    // Splash screen (first ~90 frames)
    if gui.splash_frames < 90 {
        gui.splash_frames += 1;
        let alpha = if gui.splash_frames < 60 { 1.0 } else { (90 - gui.splash_frames) as f32 / 30.0 };
        egui::Area::new(egui::Id::new("splash"))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(25, 28, 38, (alpha * 240.0) as u8))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(80, 120, 180, (alpha * 100.0) as u8)))
                    .corner_radius(16.0)
                    .inner_margin(48.0)
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("\u{2B22} CADKernel")
                                .size(36.0)
                                .color(egui::Color32::from_rgba_unmultiplied(200, 220, 255, (alpha * 255.0) as u8)));
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new("Open-source CAD Software")
                                .size(16.0)
                                .color(egui::Color32::from_rgba_unmultiplied(160, 180, 200, (alpha * 255.0) as u8)));
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("v0.1.0 — 9 crates, 15 I/O formats, NURBS kernel")
                                .size(11.0)
                                .color(egui::Color32::from_rgba_unmultiplied(120, 140, 160, (alpha * 255.0) as u8)));
                        });
                    });
            });
    }

    gui.tb_display_mode = vp.display_mode;
    menu::draw_menu_bar(ctx, gui, vp.camera, vp.display_mode);
    toolbar::draw_toolbar(ctx, gui);
    toolbar::draw_workbench_tabs(ctx, gui);
    toolbar::draw_context_toolbar(ctx, gui);
    overlays::draw_breadcrumb_bar(ctx, gui, scene);
    // FreeCAD-style ComboView: single left panel with tree (top) + properties/task (bottom)
    if gui.show_model_tree || gui.show_properties {
        egui::SidePanel::left("combo_view")
            .default_width(300.0)
            .width_range(220.0..=450.0)
            .frame(egui::Frame {
                fill: egui::Color32::from_rgb(37, 37, 38),
                inner_margin: egui::Margin::ZERO,
                stroke: egui::Stroke::new(1.0, egui::Color32::from_rgb(26, 28, 32)),
                ..egui::Frame::NONE
            })
            .show(ctx, |ui| {
                let avail = ui.available_height();
                let tree_height = avail * 0.45;

                // ---- Top: Model Tree ----
                if gui.show_model_tree {
                    // Panel header bar
                    if theme::draw_panel_header(ui, "Model", true) {
                        gui.show_model_tree = false;
                    }

                    egui::ScrollArea::vertical()
                        .id_salt("combo_tree")
                        .max_height(tree_height - 24.0)
                        .show(ui, |ui| {
                            ui.add_space(2.0);
                            // Inset content with horizontal padding
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(6, 0))
                                .show(ui, |ui| {
                                    tree::draw_model_tree_inline(ui, gui, scene);
                                });
                        });
                }

                // Separator
                theme::draw_separator(ui);

                // ---- Bottom: Task Panel OR Properties ----
                if gui.show_properties {
                    // Panel header bar
                    let props_title = if gui.active_task.is_some() { "Tasks" } else { "Properties" };
                    if theme::draw_panel_header(ui, props_title, true) {
                        gui.show_properties = false;
                    }

                    egui::ScrollArea::vertical()
                        .id_salt("combo_props")
                        .show(ui, |ui| {
                            ui.add_space(2.0);
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(6, 0))
                                .show(ui, |ui| {
                                    if !task_panel::draw_task_panel_inline(ui, gui) {
                                        properties::draw_properties_inline(ui, gui, scene);
                                    }
                                });
                        });
                }
            });
    }
    report::draw_report_panel(ctx, gui);
    status_bar::draw_status_bar(ctx, gui, vp, scene);
    overlays::draw_toast_overlay(ctx, gui);
    if gui.active_task.is_none() {
        dialogs::draw_create_dialogs(ctx, gui);
    }
    sketch_ui::draw_sketch_overlay(ctx, gui, vp.camera);
    if scene.is_empty() && gui.sketch_mode.is_none() && gui.active_task.is_none() {
        overlays::draw_welcome_screen(ctx, gui);
    }
    overlays::draw_techdraw_overlay(ctx, gui);
    dialogs::draw_about_dialog(ctx, gui);
    dialogs::draw_shortcuts_dialog(ctx, gui);
    dialogs::draw_settings(ctx, gui, nav);
    dialogs::draw_plugin_manager(ctx, gui);
    dialogs::draw_bom_dialog(ctx, gui);
    dialogs::draw_joint_editor_dialog(ctx, gui);
    dialogs::draw_material_picker_dialog(ctx, gui);
    dialogs::draw_bc_editor_dialog(ctx, gui);
    if nav.show_view_cube {
        view_cube::draw_view_cube(ctx, vp.camera, gui, nav);
    }
    if nav.show_axes_indicator {
        overlays::draw_axes_overlay(ctx, vp.camera, gui);
    }
    if nav.show_grid_3d {
        let avail = ctx.available_rect();
        overlays::draw_grid_3d_overlay(ctx, vp.camera, nav.grid_3d_spacing, avail);
    }
    overlays::draw_measurement_overlay(ctx, vp.camera, gui, nav);
    overlays::draw_snap_overlay(ctx, vp.camera, gui);
    overlays::draw_sketch_plane_preview(ctx, vp.camera, gui);
    overlays::draw_rubber_band(ctx, gui);
    overlays::draw_transform_gizmo(ctx, vp.camera, gui, scene);
    overlays::draw_selection_overlay(ctx, vp.camera, gui, scene, nav);
    if vp.show_grid {
        sketch_ui::draw_grid_scale_label(ctx, vp.grid_config);
    }

    // Viewport right-click context menu
    if ctx.input(|i| i.pointer.secondary_clicked()) {
        gui.show_viewport_context_menu = true;
    }
    if gui.show_viewport_context_menu {
        egui::Area::new(egui::Id::new("viewport_ctx"))
            .fixed_pos(ctx.input(|i| i.pointer.interact_pos().unwrap_or_default()))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    // Show face/edge menu when in sub-entity selection mode
                    if matches!(gui.selection_mode, SelectionMode::Face | SelectionMode::Edge) {
                        context_menu::face_edge_context_menu(ui, gui);
                        ui.separator();
                    }
                    context_menu::viewport_context_menu(ui, gui);
                    if ui.input(|i| i.pointer.primary_clicked())
                        || !ui.rect_contains_pointer(ui.max_rect())
                    {
                        gui.show_viewport_context_menu = false;
                    }
                });
            });
    }

    // Compute viewport rect (area not claimed by panels) for 3D scissor.
    // CentralPanel is NOT used — it would consume mouse events and block orbit/pick.
    let avail = ctx.available_rect();
    let ppp = ctx.pixels_per_point();
    gui.viewport_rect = Some([
        (avail.left() * ppp) as u32,
        (avail.top() * ppp) as u32,
        ((avail.width() * ppp) as u32).max(1),
        ((avail.height() * ppp) as u32).max(1),
    ]);

    // Store egui pointer position in physical pixels for picking.
    // Egui's pointer is always correct regardless of platform scaling quirks.
    gui.pointer_physical = ctx.input(|i| {
        i.pointer.latest_pos().map(|p| (p.x * ppp, p.y * ppp))
    });

}

// ---------------------------------------------------------------------------
// Assembly dispatcher helper tests (Phase N full)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod assembly_helper_tests {
    use super::*;
    use cadkernel_math::Point3;
    use cadkernel_modeling::{Assembly, JointType, make_box};
    use cadkernel_topology::BRepModel;

    fn make_assembly_with_n_named(parts: &[&str]) -> Assembly {
        let mut model = BRepModel::new();
        let b = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut asm = Assembly::new("T");
        for p in parts {
            asm.add_component(p, b.solid);
        }
        asm
    }

    #[test]
    fn toggle_component_visibility_flips_flag() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B"]));
        assert!(gui.assembly.as_ref().unwrap().components[0].visible);
        assert!(gui.toggle_assembly_component_visibility(0));
        assert!(!gui.assembly.as_ref().unwrap().components[0].visible);
        assert!(gui.toggle_assembly_component_visibility(0));
        assert!(gui.assembly.as_ref().unwrap().components[0].visible);
    }

    #[test]
    fn toggle_component_visibility_no_assembly_noop() {
        let mut gui = GuiState::new();
        assert!(!gui.toggle_assembly_component_visibility(0));
    }

    #[test]
    fn bill_of_materials_groups_by_name() {
        let mut gui = GuiState::new();
        // Two "Gear" + one "Shaft" → 2 entries.
        gui.assembly = Some(make_assembly_with_n_named(&["Gear", "Gear", "Shaft"]));
        assert!(gui.populate_bom_entries());
        assert!(gui.show_bom_dialog);
        assert_eq!(gui.bom_entries.len(), 2);
        let gear = gui.bom_entries.iter().find(|e| e.name == "Gear").unwrap();
        let shaft = gui.bom_entries.iter().find(|e| e.name == "Shaft").unwrap();
        assert_eq!(gear.quantity, 2);
        assert_eq!(shaft.quantity, 1);
    }

    #[test]
    fn bill_of_materials_no_assembly_returns_false() {
        let mut gui = GuiState::new();
        assert!(!gui.populate_bom_entries());
        assert!(!gui.show_bom_dialog);
    }

    #[test]
    fn open_joint_editor_requires_two_components() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["Only"]));
        assert!(!gui.open_joint_editor(AssemblyJointType::Revolute));
        assert!(!gui.show_joint_editor);
        assert!(gui.joint_editor_type.is_none());
    }

    #[test]
    fn open_joint_editor_ok_with_two_plus_components() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Revolute));
        assert!(gui.show_joint_editor);
        assert_eq!(gui.joint_editor_type, Some(AssemblyJointType::Revolute));
        assert_eq!(gui.joint_editor_comp_a, 0);
        assert_eq!(gui.joint_editor_comp_b, 1);
    }

    #[test]
    fn commit_revolute_joint_appends_one_revolute() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Revolute));
        gui.joint_editor_axis = [0.0, 0.0, 1.0];
        gui.joint_editor_origin = [1.0, 2.0, 3.0];
        assert!(gui.commit_assembly_joint());
        let joints = &gui.assembly.as_ref().unwrap().joints;
        assert_eq!(joints.len(), 1);
        assert!(matches!(joints[0], JointType::Revolute { .. }));
        assert!(!gui.show_joint_editor);
        assert!(gui.joint_editor_type.is_none());
    }

    #[test]
    fn open_grounded_joint_ok_with_one_component() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["Only"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Grounded));
        assert!(gui.show_joint_editor);
        assert_eq!(gui.joint_editor_type, Some(AssemblyJointType::Grounded));
    }

    #[test]
    fn commit_grounded_joint_appends_grounded() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Grounded));
        assert!(gui.commit_assembly_joint());
        let joints = &gui.assembly.as_ref().unwrap().joints;
        assert_eq!(joints.len(), 1);
        assert!(matches!(joints[0], JointType::Grounded));
    }

    #[test]
    fn commit_slider_joint_appends_slider() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Slider));
        assert!(gui.commit_assembly_joint());
        let joints = &gui.assembly.as_ref().unwrap().joints;
        assert_eq!(joints.len(), 1);
        assert!(matches!(joints[0], JointType::Slider { .. }));
    }

    #[test]
    fn tree_assembly_section_does_not_panic_with_assembly() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B", "C"]));
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |c| {
            egui::CentralPanel::default().show(c, |ui| {
                crate::gui::tree::draw_assembly_section(ui, &mut gui, None);
            });
        });
    }

    #[test]
    fn tree_assembly_section_noop_without_assembly() {
        let mut gui = GuiState::new();
        let ctx = egui::Context::default();
        let _ = ctx.run(egui::RawInput::default(), |c| {
            egui::CentralPanel::default().show(c, |ui| {
                crate::gui::tree::draw_assembly_section(ui, &mut gui, None);
            });
        });
    }

    #[test]
    fn joint_label_revolute_uses_arrow_notation() {
        use crate::gui::tree::joint_label;
        let j = JointType::Revolute {
            component_a: 0,
            component_b: 1,
            axis: cadkernel_math::Vec3 { x: 0.0, y: 0.0, z: 1.0 },
            origin: Point3::ORIGIN,
        };
        assert_eq!(joint_label(&j), "Revolute(0\u{2194}1)");
    }

    #[test]
    fn joint_label_grounded_has_no_components() {
        use crate::gui::tree::joint_label;
        assert_eq!(joint_label(&JointType::Grounded), "Grounded");
    }

    #[test]
    fn joint_label_fixed_and_gear() {
        use crate::gui::tree::joint_label;
        let f = JointType::FixedJoint { component_a: 2, component_b: 3 };
        assert_eq!(joint_label(&f), "FixedJoint(2\u{2194}3)");
        let g = JointType::GearJoint { component_a: 4, component_b: 5, ratio: 2.0 };
        assert_eq!(joint_label(&g), "Gear(4\u{2194}5)");
    }

    #[test]
    fn constraint_label_covers_all_variants() {
        use crate::gui::tree::constraint_label;
        use cadkernel_modeling::{AssemblyConstraint as C, ComponentId};
        let fixed = C::Fixed(ComponentId(7));
        assert_eq!(constraint_label(&fixed), "Fixed(comp 7)");
        let coin = C::Coincident {
            comp_a: ComponentId(0),
            comp_b: ComponentId(1),
            offset: 0.0,
        };
        assert_eq!(constraint_label(&coin), "Coincident(0,1)");
        let conc = C::Concentric { comp_a: ComponentId(2), comp_b: ComponentId(3) };
        assert_eq!(constraint_label(&conc), "Concentric(2,3)");
        let dist = C::Distance {
            comp_a: ComponentId(4),
            comp_b: ComponentId(5),
            distance: 10.0,
        };
        assert_eq!(constraint_label(&dist), "Distance(4,5)");
        let ang = C::Angle {
            comp_a: ComponentId(6),
            comp_b: ComponentId(8),
            angle: 90.0,
        };
        assert_eq!(constraint_label(&ang), "Angle(6,8)");
    }
}

// ---------------------------------------------------------------------------
// Phase O-a — material picker / BC editor unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod fem_picker_tests {
    use super::*;
    use cadkernel_modeling::BoundaryCondition;

    fn near(a: f64, b: f64) -> bool { (a - b).abs() <= 1e-6 * a.abs().max(b.abs()).max(1.0) }

    #[test]
    fn material_preset_all_six_match_fem_material_constructors() {
        for (preset, e, nu, rho) in [
            (MaterialPreset::Steel, 210.0e9, 0.30, 7850.0),
            (MaterialPreset::Aluminum, 70.0e9, 0.33, 2700.0),
            (MaterialPreset::Titanium, 114.0e9, 0.34, 4430.0),
            (MaterialPreset::Copper, 117.0e9, 0.34, 8960.0),
            (MaterialPreset::Concrete, 30.0e9, 0.20, 2400.0),
            (MaterialPreset::CastIron, 170.0e9, 0.26, 7200.0),
        ] {
            let m = preset.to_material();
            assert!(near(m.youngs_modulus, e) && near(m.poisson_ratio, nu) && near(m.density, rho),
                "{} mismatch", preset.label());
        }
    }

    #[test]
    fn material_from_preset_custom_uses_user_values_and_falls_back_on_invalid() {
        let m = material_from_preset(MaterialPreset::Custom, 123.0e9, 0.25, 5000.0);
        assert!(near(m.youngs_modulus, 123.0e9) && near(m.poisson_ratio, 0.25) && near(m.density, 5000.0));
        // Invalid (negative density) falls back to steel.
        let f = material_from_preset(MaterialPreset::Custom, 1.0e9, 0.3, -1.0);
        assert!(near(f.youngs_modulus, 210.0e9));
        // Named preset ignores custom values.
        let s = material_from_preset(MaterialPreset::Steel, 1.0, 0.1, 1.0);
        assert!(near(s.youngs_modulus, 210.0e9));
    }

    #[test]
    fn bc_kind_force_visibility_gate() {
        assert!(!BcKind::FixedNode.shows_force_fields());
        assert!(BcKind::Force.shows_force_fields());
    }

    #[test]
    fn bc_kind_inputs_gate_matches_kernel_field_set() {
        // (kind, node, element, vec3, scalar)
        let cases = [
            (BcKind::FixedNode,          true,  false, false, false),
            (BcKind::Force,              true,  false, true,  false),
            (BcKind::Pressure,           false, true,  false, true),
            (BcKind::Displacement,       true,  false, true,  false),
            (BcKind::Gravity,            false, false, true,  false),
            (BcKind::DistributedLoad,    false, true,  true,  false),
            (BcKind::Spring,             true,  false, false, true),
            (BcKind::CentrifugalLoad,    false, false, true,  true),
            (BcKind::SelfWeight,         false, false, true,  false),
            (BcKind::SpringConstraint,   true,  false, true,  true),
            (BcKind::BodyLoad,           false, false, true,  false),
            (BcKind::InitialTemperature, true,  false, false, true),
        ];
        for (k, n, e, v, s) in cases {
            let i = k.inputs();
            assert_eq!((i.node, i.element, i.vec3, i.scalar), (n, e, v, s),
                "{} inputs gate mismatch", k.label());
        }
    }

    #[test]
    fn bc_editor_state_to_fixed_node_uses_node_index_only() {
        let mut s = BcEditorState::new(BcKind::FixedNode);
        s.node_index = 42;
        s.vec3_x = 999.0; // ignored for FixedNode.
        match s.to_boundary_condition() {
            BoundaryCondition::FixedNode(n) => assert_eq!(n, 42),
            _ => panic!("expected FixedNode"),
        }
    }

    #[test]
    fn bc_editor_state_to_force_uses_xyz_components() {
        let mut s = BcEditorState::new(BcKind::Force);
        s.node_index = 7;
        s.vec3_x = 10.0; s.vec3_y = -20.0; s.vec3_z = 30.0;
        match s.to_boundary_condition() {
            BoundaryCondition::Force { node, force } => {
                assert_eq!(node, 7);
                assert!(near(force.x, 10.0) && near(force.y, -20.0) && near(force.z, 30.0));
            }
            _ => panic!("expected Force"),
        }
    }

    #[test]
    fn bc_editor_state_to_pressure_uses_element_and_scalar() {
        let mut s = BcEditorState::new(BcKind::Pressure);
        s.element_index = 5; s.scalar_a = 1.5e6;
        match s.to_boundary_condition() {
            BoundaryCondition::Pressure { element, pressure } => {
                assert_eq!(element, 5); assert!(near(pressure, 1.5e6));
            }
            _ => panic!("expected Pressure"),
        }
    }

    #[test]
    fn bc_editor_state_to_displacement_uses_node_and_vec3() {
        let mut s = BcEditorState::new(BcKind::Displacement);
        s.node_index = 3; s.vec3_x = 1.0; s.vec3_y = 2.0; s.vec3_z = 3.0;
        match s.to_boundary_condition() {
            BoundaryCondition::Displacement { node, displacement } => {
                assert_eq!(node, 3);
                assert!(near(displacement.x, 1.0) && near(displacement.y, 2.0)
                    && near(displacement.z, 3.0));
            }
            _ => panic!("expected Displacement"),
        }
    }

    #[test]
    fn bc_editor_state_to_gravity_uses_vec3_only() {
        let mut s = BcEditorState::new(BcKind::Gravity);
        s.vec3_x = 0.0; s.vec3_y = 0.0; s.vec3_z = -9.81;
        match s.to_boundary_condition() {
            BoundaryCondition::Gravity { acceleration } => {
                assert!(near(acceleration.z, -9.81));
            }
            _ => panic!("expected Gravity"),
        }
    }

    #[test]
    fn bc_editor_state_to_distributed_load_uses_element_and_vec3() {
        let mut s = BcEditorState::new(BcKind::DistributedLoad);
        s.element_index = 12; s.vec3_x = 100.0; s.vec3_y = 0.0; s.vec3_z = 50.0;
        match s.to_boundary_condition() {
            BoundaryCondition::DistributedLoad { element, load } => {
                assert_eq!(element, 12);
                assert!(near(load.x, 100.0) && near(load.z, 50.0));
            }
            _ => panic!("expected DistributedLoad"),
        }
    }

    #[test]
    fn bc_editor_state_to_spring_uses_node_and_scalar() {
        let mut s = BcEditorState::new(BcKind::Spring);
        s.node_index = 9; s.scalar_a = 1000.0;
        match s.to_boundary_condition() {
            BoundaryCondition::Spring { node, stiffness } => {
                assert_eq!(node, 9); assert!(near(stiffness, 1000.0));
            }
            _ => panic!("expected Spring"),
        }
    }

    #[test]
    fn bc_editor_state_to_centrifugal_uses_axis_vec3_and_omega_scalar() {
        let mut s = BcEditorState::new(BcKind::CentrifugalLoad);
        s.vec3_x = 0.0; s.vec3_y = 0.0; s.vec3_z = 1.0; s.scalar_a = 100.0;
        match s.to_boundary_condition() {
            BoundaryCondition::CentrifugalLoad { axis, omega } => {
                assert!(near(axis.z, 1.0) && near(omega, 100.0));
            }
            _ => panic!("expected CentrifugalLoad"),
        }
    }

    #[test]
    fn bc_editor_state_to_self_weight_uses_vec3_only() {
        let mut s = BcEditorState::new(BcKind::SelfWeight);
        s.vec3_z = -9.81;
        match s.to_boundary_condition() {
            BoundaryCondition::SelfWeight { gravity } => {
                assert!(near(gravity.z, -9.81));
            }
            _ => panic!("expected SelfWeight"),
        }
    }

    #[test]
    fn bc_editor_state_to_spring_constraint_uses_node_scalar_and_direction() {
        let mut s = BcEditorState::new(BcKind::SpringConstraint);
        s.node_index = 4; s.scalar_a = 500.0;
        s.vec3_x = 1.0; s.vec3_y = 0.0; s.vec3_z = 0.0;
        match s.to_boundary_condition() {
            BoundaryCondition::SpringConstraint { node_id, stiffness, direction } => {
                assert_eq!(node_id, 4);
                assert!(near(stiffness, 500.0) && near(direction.x, 1.0));
            }
            _ => panic!("expected SpringConstraint"),
        }
    }

    #[test]
    fn bc_editor_state_to_body_load_uses_vec3_only() {
        let mut s = BcEditorState::new(BcKind::BodyLoad);
        s.vec3_x = 0.0; s.vec3_y = 0.0; s.vec3_z = -1000.0;
        match s.to_boundary_condition() {
            BoundaryCondition::BodyLoad { force_density } => {
                assert!(near(force_density.z, -1000.0));
            }
            _ => panic!("expected BodyLoad"),
        }
    }

    #[test]
    fn bc_editor_state_to_initial_temperature_uses_node_and_scalar() {
        let mut s = BcEditorState::new(BcKind::InitialTemperature);
        s.node_index = 11; s.scalar_a = 293.15;
        match s.to_boundary_condition() {
            BoundaryCondition::InitialTemperature { node, temperature } => {
                assert_eq!(node, 11); assert!(near(temperature, 293.15));
            }
            _ => panic!("expected InitialTemperature"),
        }
    }

    #[test]
    fn pending_fem_material_default_is_steel() {
        let g = GuiState::new();
        assert!(near(g.pending_fem_material.youngs_modulus, 210.0e9));
        assert!(near(g.pending_fem_material.density, 7850.0));
    }

    #[test]
    fn pending_fem_material_clones_for_sticky_reuse() {
        let mut g = GuiState::new();
        g.pending_fem_material = cadkernel_modeling::FemMaterial::aluminum();
        let snapshot = g.pending_fem_material.clone();
        assert!(near(g.pending_fem_material.youngs_modulus, 70.0e9));
        assert!(near(snapshot.youngs_modulus, 70.0e9));
    }
}
