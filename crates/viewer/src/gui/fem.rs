//! FEM workbench types: material picker and boundary-condition editor state.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. The types
//! here are used by `ActiveDialog` (declared in `gui/mod.rs`) and consumed by
//! the dialog renderers in `gui/dialogs.rs`.

/// Actions specific to the FEM workbench, dispatched through
/// `GuiAction::Fem(FemAction)`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum FemAction {
    CreateAnalysis,
    SetMaterial(String),
    OpenMaterialPicker,
    CommitMaterialPicker,
    OpenBcEditor(BcKind),
    CommitBcEditor,
    GenTetMesh { element_size: f64 },
    GenHexMesh { nx: u32, ny: u32, nz: u32 },
    AddConstraint(super::FemConstraintType),
    SolveStatic,
    SolveModal { modes: usize },
    SolveThermal,
    SolveBuckling { modes: usize },
    SolveNonlinear,
    ShowStress,
    ShowDisplacement,
    ShowVonMises,
    Summary,
    Report,
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
// Phase O-a — material picker / BC editor unit tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod fem_picker_tests {
    use super::*;
    use crate::gui::GuiState;
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
