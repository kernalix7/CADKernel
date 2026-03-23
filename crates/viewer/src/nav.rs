//! Mouse navigation configuration with multiple CAD-style presets.
//!
//! Default: FreeCAD Gesture. Switchable at runtime via the Settings dialog.

use serde::{Deserialize, Serialize};

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
    FreeCADGesture,
    Blender,
    SolidWorks,
    Inventor,
    OpenCascade,
}

impl NavStyle {
    pub const ALL: &[NavStyle] = &[
        NavStyle::FreeCADGesture,
        NavStyle::Blender,
        NavStyle::SolidWorks,
        NavStyle::Inventor,
        NavStyle::OpenCascade,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::FreeCADGesture => "FreeCAD Gesture",
            Self::Blender => "Blender",
            Self::SolidWorks => "SolidWorks",
            Self::Inventor => "Inventor / Fusion 360",
            Self::OpenCascade => "OpenCascade",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::FreeCADGesture => {
                "LMB: Orbit  |  RMB / MMB: Pan  |  Scroll: Zoom  |  Ctrl+RMB: Zoom drag"
            }
            Self::Blender => {
                "MMB: Orbit  |  Shift+MMB: Pan  |  Scroll: Zoom  |  Ctrl+MMB: Zoom drag"
            }
            Self::SolidWorks => {
                "MMB: Orbit  |  Ctrl+MMB: Pan  |  Scroll: Zoom  |  Shift+MMB: Zoom drag"
            }
            Self::Inventor => "Shift+MMB: Orbit  |  MMB: Pan  |  Scroll: Zoom",
            Self::OpenCascade => {
                "MMB: Orbit  |  Shift+MMB: Pan  |  Scroll: Zoom  |  Ctrl+MMB: Zoom drag"
            }
        }
    }
}

/// Runtime-configurable navigation parameters.
#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct NavConfig {
    pub style: NavStyle,
    pub orbit_sensitivity: f32,
    pub pan_sensitivity: f32,
    pub zoom_sensitivity: f32,
    pub invert_zoom: bool,
    /// When true, view-cube clicks and orbit buttons animate smoothly instead of
    /// snapping instantly.
    pub enable_view_animation: bool,
    /// Duration of a view animation in seconds (default 0.3 s).
    pub view_animation_duration: f32,

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

    // -- Lighting --
    pub enable_lighting: bool,
    pub light_intensity: f32,
    pub light_dir: [f32; 3],
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
            orbit_sensitivity: 0.005,
            pan_sensitivity: 0.002,
            zoom_sensitivity: 0.1,
            invert_zoom: false,
            enable_view_animation: true,
            view_animation_duration: 0.3,
            show_axes_indicator: true,
            show_fps: false,
            default_projection: crate::render::Projection::Perspective,
            show_view_cube: true,
            orbit_steps: 8,
            cube_size: 132.0,
            cube_opacity: 0.5,
            snap_to_nearest: true,
            cube_corner: 0, // TopRight
            enable_lighting: true,
            light_intensity: 1.0,
            light_dir: [0.5, 0.6, 0.8],
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
    ) -> NavAction {
        match self.style {
            NavStyle::FreeCADGesture => {
                if ctrl && right {
                    return NavAction::Zoom;
                }
                if left && !middle && !right {
                    return NavAction::Orbit;
                }
                if right || middle {
                    return NavAction::Pan;
                }
                NavAction::None
            }
            NavStyle::Blender => {
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
            NavStyle::SolidWorks => {
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
                if middle && shift {
                    return NavAction::Orbit;
                }
                if middle {
                    return NavAction::Pan;
                }
                NavAction::None
            }
            NavStyle::OpenCascade => {
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
        }
    }

    /// Compute the zoom factor for a single scroll step.
    pub fn scroll_zoom_factor(&self, scroll_delta: f32) -> f32 {
        let dir = if self.invert_zoom { -1.0 } else { 1.0 };
        1.0 - scroll_delta * self.zoom_sensitivity * dir
    }
}
