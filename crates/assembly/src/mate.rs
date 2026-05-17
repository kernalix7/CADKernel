use cadkernel_core::{KernelError, KernelResult};
use serde::{Deserialize, Serialize};

use crate::ComponentId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaceRef {
    pub component: ComponentId,
    pub point: [f64; 3],
    pub normal: [f64; 3],
}

impl FaceRef {
    pub fn new(component: ComponentId, point: [f64; 3], normal: [f64; 3]) -> Self {
        Self {
            component,
            point,
            normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeRef {
    pub component: ComponentId,
    pub point: [f64; 3],
    pub direction: [f64; 3],
}

impl EdgeRef {
    pub fn new(component: ComponentId, point: [f64; 3], direction: [f64; 3]) -> Self {
        Self {
            component,
            point,
            direction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisRef {
    pub component: ComponentId,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
}

impl AxisRef {
    pub fn new(component: ComponentId, origin: [f64; 3], direction: [f64; 3]) -> Self {
        Self {
            component,
            origin,
            direction,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurveRef {
    pub component: ComponentId,
    pub control_points: Vec<[f64; 3]>,
}

impl CurveRef {
    pub fn new(component: ComponentId, control_points: Vec<[f64; 3]>) -> Self {
        Self {
            component,
            control_points,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mate", rename_all = "snake_case")]
pub enum Mate {
    Coincident {
        face_a: FaceRef,
        face_b: FaceRef,
    },
    Concentric {
        axis_a: AxisRef,
        axis_b: AxisRef,
    },
    Parallel {
        face_a: FaceRef,
        face_b: FaceRef,
    },
    Perpendicular {
        face_a: FaceRef,
        face_b: FaceRef,
    },
    Tangent {
        face_a: FaceRef,
        edge_b: EdgeRef,
    },
    Distance {
        face_a: FaceRef,
        face_b: FaceRef,
        distance: f64,
    },
    Angle {
        face_a: FaceRef,
        face_b: FaceRef,
        angle_rad: f64,
    },
    Lock {
        component: ComponentId,
    },
    Slider {
        axis: AxisRef,
        range_min: f64,
        range_max: f64,
    },
    Hinge {
        axis: AxisRef,
        range_min: f64,
        range_max: f64,
    },
    Gear {
        axis_a: AxisRef,
        axis_b: AxisRef,
        ratio: f64,
    },
    Rack {
        axis: AxisRef,
        pitch: f64,
    },
    Universal {
        axis_a: AxisRef,
        axis_b: AxisRef,
    },
    Cam {
        profile_face: FaceRef,
        follower: EdgeRef,
    },
    Screw {
        axis: AxisRef,
        pitch: f64,
    },
    Path {
        component: ComponentId,
        path_curve: CurveRef,
    },
}

impl Mate {
    pub fn component_ids(&self) -> Vec<ComponentId> {
        match self {
            Self::Coincident { face_a, face_b }
            | Self::Parallel { face_a, face_b }
            | Self::Perpendicular { face_a, face_b }
            | Self::Distance { face_a, face_b, .. }
            | Self::Angle { face_a, face_b, .. } => dedupe2(face_a.component, face_b.component),
            Self::Concentric { axis_a, axis_b }
            | Self::Gear { axis_a, axis_b, .. }
            | Self::Universal { axis_a, axis_b } => dedupe2(axis_a.component, axis_b.component),
            Self::Tangent { face_a, edge_b } => dedupe2(face_a.component, edge_b.component),
            Self::Lock { component } | Self::Path { component, .. } => vec![*component],
            Self::Slider { axis, .. }
            | Self::Hinge { axis, .. }
            | Self::Rack { axis, .. }
            | Self::Screw { axis, .. } => vec![axis.component],
            Self::Cam {
                profile_face,
                follower,
            } => dedupe2(profile_face.component, follower.component),
        }
    }

    pub fn constraint_count(&self) -> i32 {
        match self {
            Self::Coincident { .. } => 3,
            Self::Concentric { .. } => 4,
            Self::Parallel { .. } => 2,
            Self::Perpendicular { .. } => 1,
            Self::Tangent { .. } => 1,
            Self::Distance { .. } => 1,
            Self::Angle { .. } => 1,
            Self::Lock { .. } => 6,
            Self::Slider { .. } => 5,
            Self::Hinge { .. } => 5,
            Self::Gear { .. } => 1,
            Self::Rack { .. } => 1,
            Self::Universal { .. } => 4,
            Self::Cam { .. } => 2,
            Self::Screw { .. } => 5,
            Self::Path { .. } => 5,
        }
    }

    pub fn is_single_dof_motion(&self) -> bool {
        matches!(
            self,
            Self::Slider { .. }
                | Self::Hinge { .. }
                | Self::Rack { .. }
                | Self::Screw { .. }
                | Self::Path { .. }
        )
    }

    pub fn validate(&self) -> KernelResult<()> {
        match self {
            Self::Coincident { face_a, face_b }
            | Self::Parallel { face_a, face_b }
            | Self::Perpendicular { face_a, face_b } => {
                validate_face(face_a)?;
                validate_face(face_b)
            }
            Self::Distance {
                face_a,
                face_b,
                distance,
            } => {
                validate_face(face_a)?;
                validate_face(face_b)?;
                validate_nonnegative(*distance, "distance")
            }
            Self::Angle {
                face_a,
                face_b,
                angle_rad,
            } => {
                validate_face(face_a)?;
                validate_face(face_b)?;
                validate_finite(*angle_rad, "angle_rad")
            }
            Self::Concentric { axis_a, axis_b }
            | Self::Gear { axis_a, axis_b, .. }
            | Self::Universal { axis_a, axis_b } => {
                validate_axis(axis_a)?;
                validate_axis(axis_b)?;
                if let Self::Gear { ratio, .. } = self {
                    validate_nonzero(*ratio, "ratio")?;
                }
                Ok(())
            }
            Self::Tangent { face_a, edge_b } => {
                validate_face(face_a)?;
                validate_edge(edge_b)
            }
            Self::Lock { .. } => Ok(()),
            Self::Slider {
                axis,
                range_min,
                range_max,
            }
            | Self::Hinge {
                axis,
                range_min,
                range_max,
            } => {
                validate_axis(axis)?;
                validate_range(*range_min, *range_max)
            }
            Self::Rack { axis, pitch } | Self::Screw { axis, pitch } => {
                validate_axis(axis)?;
                validate_nonzero(*pitch, "pitch")
            }
            Self::Cam {
                profile_face,
                follower,
            } => {
                validate_face(profile_face)?;
                validate_edge(follower)
            }
            Self::Path {
                component: _,
                path_curve,
            } => validate_curve(path_curve),
        }
    }
}

fn dedupe2(a: ComponentId, b: ComponentId) -> Vec<ComponentId> {
    if a == b { vec![a] } else { vec![a, b] }
}

fn validate_face(face: &FaceRef) -> KernelResult<()> {
    validate_vec3(face.point, "face point")?;
    validate_direction(face.normal, "face normal")
}

fn validate_edge(edge: &EdgeRef) -> KernelResult<()> {
    validate_vec3(edge.point, "edge point")?;
    validate_direction(edge.direction, "edge direction")
}

fn validate_axis(axis: &AxisRef) -> KernelResult<()> {
    validate_vec3(axis.origin, "axis origin")?;
    validate_direction(axis.direction, "axis direction")
}

fn validate_curve(curve: &CurveRef) -> KernelResult<()> {
    if curve.control_points.len() < 2 {
        return Err(KernelError::InvalidArgument(
            "path curve requires at least two control points".into(),
        ));
    }
    for point in &curve.control_points {
        validate_vec3(*point, "path control point")?;
    }
    Ok(())
}

fn validate_vec3(value: [f64; 3], label: &str) -> KernelResult<()> {
    for component in value {
        validate_finite(component, label)?;
    }
    Ok(())
}

fn validate_direction(value: [f64; 3], label: &str) -> KernelResult<()> {
    validate_vec3(value, label)?;
    let len2 = value[0] * value[0] + value[1] * value[1] + value[2] * value[2];
    if len2 <= 1.0e-24 {
        return Err(KernelError::InvalidArgument(format!(
            "{label} must be non-zero"
        )));
    }
    Ok(())
}

fn validate_range(min: f64, max: f64) -> KernelResult<()> {
    validate_finite(min, "range_min")?;
    validate_finite(max, "range_max")?;
    if min > max {
        return Err(KernelError::InvalidArgument(
            "range_min must be <= range_max".into(),
        ));
    }
    Ok(())
}

fn validate_nonnegative(value: f64, label: &str) -> KernelResult<()> {
    validate_finite(value, label)?;
    if value < 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "{label} must be >= 0"
        )));
    }
    Ok(())
}

fn validate_nonzero(value: f64, label: &str) -> KernelResult<()> {
    validate_finite(value, label)?;
    if value.abs() <= 1.0e-12 {
        return Err(KernelError::InvalidArgument(format!(
            "{label} must be non-zero"
        )));
    }
    Ok(())
}

fn validate_finite(value: f64, label: &str) -> KernelResult<()> {
    if !value.is_finite() {
        return Err(KernelError::InvalidArgument(format!(
            "{label} must be finite"
        )));
    }
    Ok(())
}
