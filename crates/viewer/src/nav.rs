//! Mouse navigation configuration with multiple CAD-style presets.
//!
//! Default: FreeCAD Gesture. Switchable at runtime via the Settings dialog.

use crate::gui::theme::{ThemeMode, UiDensity};
use serde::{Deserialize, Serialize};

/// Unit system for dimension display.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum UnitSystem {
    Millimeters,
    Centimeters,
    Meters,
    Inches,
    Feet,
}

impl UnitSystem {
    pub const ALL: &[UnitSystem] = &[
        UnitSystem::Millimeters,
        UnitSystem::Centimeters,
        UnitSystem::Meters,
        UnitSystem::Inches,
        UnitSystem::Feet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Millimeters => "mm",
            Self::Centimeters => "cm",
            Self::Meters => "m",
            Self::Inches => "in",
            Self::Feet => "ft",
        }
    }

    pub fn long_label(self) -> &'static str {
        match self {
            Self::Millimeters => "Millimeters (mm)",
            Self::Centimeters => "Centimeters (cm)",
            Self::Meters => "Meters (m)",
            Self::Inches => "Inches (in)",
            Self::Feet => "Feet (ft)",
        }
    }
}

/// A saved camera view bookmark.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ViewBookmark {
    pub name: String,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub distance: f32,
    pub target: [f32; 3],
}

/// Background gradient preset.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BgPreset {
    Dark,
    Medium,
    Light,
    Blueprint,
    Custom,
}

impl BgPreset {
    pub const ALL: &[BgPreset] = &[
        BgPreset::Dark,
        BgPreset::Medium,
        BgPreset::Light,
        BgPreset::Blueprint,
        BgPreset::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Medium => "Medium",
            Self::Light => "Light",
            Self::Blueprint => "Blueprint",
            Self::Custom => "Custom",
        }
    }
}

/// Navigation action resolved from mouse + modifier state.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavAction {
    None,
    Orbit,
    Pan,
    Zoom,
}

/// Preset navigation styles modelled after popular CAD programs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum NavStyle {
    /// FreeCAD default CAD navigation (renamed from FreeCADGesture for serde compat)
    #[serde(alias = "CAD")]
    FreeCADGesture,
    Gesture,
    Blender,
    Maya,
    SolidWorks,
    Inventor,
    OpenCascade,
    OpenSCAD,
    Revit,
    SiemensNX,
    TinkerCAD,
    Touchpad,
}

/// Orbit computation style (how mouse movement maps to 3D rotation).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum OrbitStyle {
    Turntable,
    Trackball,
    FreeTurntable,
    TrackballClassic,
    RoundedArcball,
}

impl OrbitStyle {
    pub const ALL: &[OrbitStyle] = &[
        OrbitStyle::Turntable,
        OrbitStyle::Trackball,
        OrbitStyle::FreeTurntable,
        OrbitStyle::TrackballClassic,
        OrbitStyle::RoundedArcball,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Turntable => "Turntable",
            Self::Trackball => "Trackball",
            Self::FreeTurntable => "Free Turntable",
            Self::TrackballClassic => "Trackball Classic",
            Self::RoundedArcball => "Rounded Arcball",
        }
    }
}

/// Where 3D rotation is centered.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum RotationMode {
    WindowCenter,
    DragAtCursor,
    ObjectCenter,
}

impl RotationMode {
    pub const ALL: &[RotationMode] = &[
        RotationMode::WindowCenter,
        RotationMode::DragAtCursor,
        RotationMode::ObjectCenter,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::WindowCenter => "Window center",
            Self::DragAtCursor => "Drag at cursor",
            Self::ObjectCenter => "Object center",
        }
    }
}

impl NavStyle {
    pub const ALL: &[NavStyle] = &[
        NavStyle::FreeCADGesture,
        NavStyle::Gesture,
        NavStyle::Blender,
        NavStyle::Maya,
        NavStyle::SolidWorks,
        NavStyle::Inventor,
        NavStyle::OpenCascade,
        NavStyle::OpenSCAD,
        NavStyle::Revit,
        NavStyle::SiemensNX,
        NavStyle::TinkerCAD,
        NavStyle::Touchpad,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::FreeCADGesture => "CAD (default)",
            Self::Gesture => "Gesture",
            Self::Blender => "Blender",
            Self::Maya => "Maya",
            Self::SolidWorks => "SolidWorks",
            Self::Inventor => "OpenInventor",
            Self::OpenCascade => "OpenCascade",
            Self::OpenSCAD => "OpenSCAD",
            Self::Revit => "Revit",
            Self::SiemensNX => "Siemens NX",
            Self::TinkerCAD => "TinkerCAD",
            Self::Touchpad => "Touchpad",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::FreeCADGesture => {
                "LMB: Select  |  MMB: Orbit  |  Shift+MMB / RMB: Pan  |  Scroll: Zoom  |  Ctrl+RMB: Zoom drag"
            }
            Self::Gesture => {
                "LMB drag: Orbit  |  RMB: Pan  |  Scroll / Pinch: Zoom  |  Tap: Select"
            }
            Self::Blender => {
                "LMB: Select  |  MMB: Orbit  |  Shift+MMB: Pan  |  Scroll: Zoom"
            }
            Self::Maya => {
                "LMB: Select  |  Alt+LMB: Orbit  |  Alt+MMB: Pan  |  Alt+RMB / Scroll: Zoom"
            }
            Self::SolidWorks => {
                "LMB: Select  |  MMB: Orbit  |  Ctrl+MMB: Pan  |  Shift+MMB / Scroll: Zoom"
            }
            Self::Inventor => {
                "Ctrl+LMB: Select  |  LMB: Orbit  |  MMB: Pan  |  Scroll: Zoom"
            }
            Self::OpenCascade => {
                "LMB: Select  |  Ctrl+RMB: Orbit  |  Ctrl+MMB: Pan  |  Ctrl+LMB / Scroll: Zoom"
            }
            Self::OpenSCAD => {
                "LMB drag: Orbit  |  RMB: Pan  |  MMB / Shift+RMB / Scroll: Zoom"
            }
            Self::Revit => {
                "LMB: Select  |  Shift+MMB: Orbit  |  MMB: Pan  |  Scroll: Zoom"
            }
            Self::SiemensNX => {
                "LMB: Select  |  MMB: Orbit  |  MMB+RMB: Pan  |  Scroll: Zoom"
            }
            Self::TinkerCAD => {
                "LMB: Select  |  RMB: Orbit  |  MMB: Pan  |  Scroll: Zoom"
            }
            Self::Touchpad => {
                "LMB: Select  |  Alt+Move: Orbit  |  Shift+Move: Pan  |  Ctrl+Shift+Move / Scroll: Zoom"
            }
        }
    }
}

/// Runtime-configurable navigation parameters.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NavConfig {
    pub style: NavStyle,
    pub orbit_style: OrbitStyle,
    pub rotation_mode: RotationMode,
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub zoom_step: f32,
    pub zoom_at_cursor: bool,
    pub invert_zoom: bool,
    pub disable_touch_tilt: bool,
    /// When true, view-cube clicks and orbit buttons animate smoothly instead of
    /// snapping instantly.
    pub enable_view_animation: bool,
    /// Duration of a view animation in seconds (default 0.3 s).
    pub view_animation_duration: f32,
    pub enable_spinning: bool,
    pub show_rotation_center: bool,
    pub rotation_center_size: f32,

    // -- 3D View settings --
    pub show_axes_indicator: bool,
    pub show_fps: bool,
    pub default_projection: crate::render::Projection,

    // -- ViewCube settings --
    pub show_view_cube: bool,
    pub orbit_steps: u32,
    pub cube_size: f32,
    pub cube_opacity: f32,
    pub snap_to_nearest: bool,
    /// ViewCube corner: 0=TopRight, 1=TopLeft, 2=BottomLeft, 3=BottomRight
    pub cube_corner: u8,

    // -- Theme --
    pub theme_mode: ThemeMode,
    pub ui_density: UiDensity,

    // -- Viewport overlays --
    pub show_origin: bool,
    pub show_grid_3d: bool,
    pub grid_3d_spacing: f32,

    // -- Lighting --
    pub enable_lighting: bool,
    pub light_intensity: f32,
    pub light_dir: [f32; 3],

    // -- Units --
    pub unit_system: UnitSystem,
    pub decimal_places: u8,

    // -- Background --
    pub bg_preset: BgPreset,
    pub bg_custom_top: [f32; 3],
    pub bg_custom_bottom: [f32; 3],

    // -- Selection colors (RGB) --
    pub selection_color: [u8; 3],
    pub preselection_color: [u8; 3],

    // -- Tessellation --
    pub tessellation_segments: u32,

    // -- 3D snap --
    pub snap_to_grid_3d: bool,

    // -- Clip plane (section view) --
    pub clip_enabled: bool,
    pub clip_plane_normal: [f32; 3],
    pub clip_plane_offset: f32,

    // -- General --
    pub auto_save_enabled: bool,
    pub auto_save_interval_secs: u32,
    pub recent_files_max: usize,
    pub confirm_delete: bool,

    // -- View bookmarks --
    pub view_bookmarks: Vec<ViewBookmark>,
}

impl Default for NavConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl NavConfig {
    pub fn new() -> Self {
        Self {
            style: NavStyle::FreeCADGesture,
            orbit_style: OrbitStyle::RoundedArcball,
            rotation_mode: RotationMode::WindowCenter,
            orbit_sensitivity: 0.005,
            pan_sensitivity: 0.002,
            zoom_sensitivity: 0.1,
            zoom_step: 0.2,
            zoom_at_cursor: true,
            invert_zoom: true,
            disable_touch_tilt: true,
            enable_view_animation: true,
            view_animation_duration: 0.5,
            enable_spinning: false,
            show_rotation_center: true,
            rotation_center_size: 5.0,
            show_axes_indicator: true,
            show_fps: false,
            default_projection: crate::render::Projection::Perspective,
            show_view_cube: true,
            orbit_steps: 8,
            cube_size: 68.0,
            cube_opacity: 0.5,
            snap_to_nearest: true,
            cube_corner: 0, // TopRight
            theme_mode: ThemeMode::Dark,
            ui_density: UiDensity::Normal,
            show_origin: true,
            show_grid_3d: true,
            grid_3d_spacing: 1.0,
            enable_lighting: true,
            light_intensity: 1.0,
            light_dir: [0.5, 0.6, 0.8],
            unit_system: UnitSystem::Millimeters,
            decimal_places: 2,
            bg_preset: BgPreset::Dark,
            bg_custom_top: [0.20, 0.22, 0.28],
            bg_custom_bottom: [0.08, 0.08, 0.10],
            selection_color: [60, 140, 255],
            preselection_color: [120, 200, 255],
            tessellation_segments: 64,
            snap_to_grid_3d: false,
            clip_enabled: false,
            clip_plane_normal: [0.0, 0.0, 1.0],
            clip_plane_offset: 0.0,
            auto_save_enabled: false,
            auto_save_interval_secs: 300,
            recent_files_max: 10,
            confirm_delete: true,
            view_bookmarks: Vec::new(),
        }
    }

    /// Snap a value to the 3D grid spacing, if snapping is enabled.
    pub fn snap_3d(&self, v: f64) -> f64 {
        if self.snap_to_grid_3d && self.grid_3d_spacing > 0.0 {
            let s = self.grid_3d_spacing as f64;
            (v / s).round() * s
        } else {
            v
        }
    }

    /// Given button + modifier state, return which drag action to perform.
    pub fn resolve_drag(
        &self,
        left: bool,
        middle: bool,
        right: bool,
        shift: bool,
        ctrl: bool,
        alt: bool,
    ) -> NavAction {
        match self.style {
            NavStyle::FreeCADGesture => {
                // MMB: Orbit, Shift+MMB / RMB: Pan, Ctrl+RMB: Zoom
                if ctrl && right {
                    return NavAction::Zoom;
                }
                if middle && shift {
                    return NavAction::Pan;
                }
                if middle {
                    return NavAction::Orbit;
                }
                if right {
                    return NavAction::Pan;
                }
                NavAction::None
            }
            NavStyle::Gesture => {
                // LMB drag: Orbit, RMB: Pan (touchpad-friendly)
                if right {
                    return NavAction::Pan;
                }
                if left {
                    return NavAction::Orbit;
                }
                if middle {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::Blender => {
                // MMB: Orbit, Shift+MMB: Pan, Ctrl+MMB: Zoom
                if middle && ctrl {
                    return NavAction::Zoom;
                }
                if middle && shift {
                    return NavAction::Pan;
                }
                if middle {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::Maya => {
                // Alt+LMB: Orbit, Alt+MMB: Pan, Alt+RMB: Zoom
                if left && alt {
                    return NavAction::Orbit;
                }
                if middle && alt {
                    return NavAction::Pan;
                }
                if right && alt {
                    return NavAction::Zoom;
                }
                // Fallback: plain MMB = orbit
                if middle {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::SolidWorks => {
                // MMB: Orbit, Ctrl+MMB: Pan, Shift+MMB: Zoom
                if middle && shift {
                    return NavAction::Zoom;
                }
                if middle && ctrl {
                    return NavAction::Pan;
                }
                if middle {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::Inventor => {
                // LMB: Orbit (Ctrl+LMB = select in FreeCAD), MMB: Pan
                if middle {
                    return NavAction::Pan;
                }
                if left && !ctrl {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::OpenCascade => {
                // Ctrl+RMB: Orbit, Ctrl+MMB: Pan, Ctrl+LMB: Zoom
                if ctrl && left {
                    return NavAction::Zoom;
                }
                if ctrl && middle {
                    return NavAction::Pan;
                }
                if ctrl && right {
                    return NavAction::Orbit;
                }
                // Fallback: plain MMB orbit
                if middle {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::OpenSCAD => {
                // LMB drag: Orbit, RMB: Pan, MMB/Shift+RMB: Zoom
                if shift && right {
                    return NavAction::Zoom;
                }
                if middle {
                    return NavAction::Zoom;
                }
                if right {
                    return NavAction::Pan;
                }
                if left {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::Revit => {
                // Shift+MMB: Orbit, MMB: Pan, Ctrl+MMB: Zoom
                if middle && ctrl {
                    return NavAction::Zoom;
                }
                if middle && shift {
                    return NavAction::Orbit;
                }
                if middle {
                    return NavAction::Pan;
                }
                NavAction::None
            }
            NavStyle::SiemensNX => {
                // MMB: Orbit, MMB+RMB: Pan
                if middle && right {
                    return NavAction::Pan;
                }
                if middle {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::TinkerCAD => {
                // RMB: Orbit, MMB: Pan
                if middle {
                    return NavAction::Pan;
                }
                if right {
                    return NavAction::Orbit;
                }
                NavAction::None
            }
            NavStyle::Touchpad => {
                // Alt+Move: Orbit, Shift+Move: Pan, Ctrl+Shift+Move: Zoom
                // These use mouse-move with modifier keys (no buttons needed)
                if ctrl && shift && left {
                    return NavAction::Zoom;
                }
                if left && alt {
                    return NavAction::Orbit;
                }
                if left && shift {
                    return NavAction::Pan;
                }
                NavAction::None
            }
        }
    }

    /// Apply orbit rotation to camera yaw/pitch based on the active `OrbitStyle`.
    ///
    /// `dx`, `dy` are pixel deltas.  `sx`, `sy` are the current cursor position
    /// in screen space (needed for trackball mapping).  `sw`, `sh` are
    /// viewport width/height.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_orbit(
        &self,
        camera_yaw: &mut f32,
        camera_pitch: &mut f32,
        dx: f32,
        dy: f32,
        sx: f32,
        sy: f32,
        sw: f32,
        sh: f32,
    ) {
        let sens = self.orbit_sensitivity;
        match self.orbit_style {
            OrbitStyle::Turntable => {
                *camera_yaw -= dx * sens;
                *camera_pitch += dy * sens;
                let limit = std::f32::consts::FRAC_PI_2 - 0.01;
                *camera_pitch = camera_pitch.clamp(-limit, limit);
            }
            OrbitStyle::FreeTurntable => {
                *camera_yaw -= dx * sens;
                *camera_pitch += dy * sens;
            }
            OrbitStyle::Trackball | OrbitStyle::TrackballClassic | OrbitStyle::RoundedArcball => {
                // Virtual-sphere trackball: map cursor position to sphere,
                // compute rotation axis from cross product of old/new vectors.
                // For yaw/pitch cameras we project the result onto yaw & pitch.
                let half_w = sw * 0.5;
                let half_h = sh * 0.5;
                let radius = half_w.min(half_h);
                if radius < 1.0 {
                    // Degenerate viewport — fall back to turntable.
                    *camera_yaw -= dx * sens;
                    *camera_pitch += dy * sens;
                    return;
                }
                // Previous and current cursor on virtual sphere.
                let px = (sx - dx - half_w) / radius;
                let py = (sy - dy - half_h) / radius;
                let cx = (sx - half_w) / radius;
                let cy = (sy - half_h) / radius;
                let pz = (1.0 - px * px - py * py).max(0.0).sqrt();
                let cz = (1.0 - cx * cx - cy * cy).max(0.0).sqrt();
                // Rotation angle ≈ arc between the two sphere points.
                let dot = (px * cx + py * cy + pz * cz).clamp(-1.0, 1.0);
                let angle = dot.acos();
                if angle < 1e-6 {
                    return;
                }
                // Rotation axis in screen space: cross(prev, cur).
                let ax = py * cz - pz * cy;
                let ay = pz * cx - px * cz;
                // ax corresponds to pitch change, ay to yaw change.
                let len = (ax * ax + ay * ay).sqrt().max(1e-8);
                let scale = angle / len;
                *camera_yaw -= ay * scale;
                *camera_pitch += ax * scale;
                if self.orbit_style == OrbitStyle::Turntable {
                    let limit = std::f32::consts::FRAC_PI_2 - 0.01;
                    *camera_pitch = camera_pitch.clamp(-limit, limit);
                }
            }
        }
    }

    /// Compute the zoom factor for a single scroll step.
    pub fn scroll_zoom_factor(&self, scroll_delta: f32) -> f32 {
        let dir = if self.invert_zoom { -1.0 } else { 1.0 };
        1.0 - scroll_delta * self.zoom_step * dir
    }

    /// Compute the zoom factor for continuous drag zoom.
    pub fn drag_zoom_factor(&self, dy: f32) -> f32 {
        let dir = if self.invert_zoom { -1.0 } else { 1.0 };
        1.0 - dy * self.zoom_sensitivity * dir
    }
}
