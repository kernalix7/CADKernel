//! Assembly workbench types and `GuiState` helpers.
//!
//! Extracted from `gui/mod.rs` as part of the viewer module split. The
//! `JointEditorState` here is consumed by `ActiveDialog` (declared in
//! `gui/mod.rs`) and rendered by `gui/dialogs.rs`. The `impl GuiState` block
//! at the bottom of this file extends the `GuiState` type with assembly-
//! specific methods; Rust supports impl blocks across multiple files.

use super::{ActiveDialog, GuiState};

/// Actions specific to the Assembly workbench, dispatched through
/// `GuiAction::Assembly(AssemblyAction)`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AssemblyAction {
    Create,
    InsertComponent,
    Solve,
    Explode { factor: f64 },
    BillOfMaterials,
    DofAnalysis,
    AddJoint(AssemblyJointType),
    ToggleComponentVisibility(usize),
    CommitJoint,
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

/// State for the joint editor modal.
#[derive(Clone, Debug)]
pub(crate) struct JointEditorState {
    pub joint_type: AssemblyJointType,
    pub comp_a: usize,
    pub comp_b: usize,
    pub axis: [f32; 3],
    pub origin: [f32; 3],
    pub angle: f64,
    pub pitch: f64,
    pub ratio: f64,
}

impl JointEditorState {
    /// Default editor state for a given joint type. `comp_b` is set to `1`
    /// when the assembly has \u{2265} 2 components; otherwise pinned to `0`
    /// (used for the Grounded variant).
    pub fn new(joint_type: AssemblyJointType, num_components: usize) -> Self {
        Self {
            joint_type,
            comp_a: 0,
            comp_b: if num_components >= 2 { 1 } else { 0 },
            axis: [0.0, 0.0, 1.0],
            origin: [0.0, 0.0, 0.0],
            angle: 90.0,
            pitch: 1.0,
            ratio: 1.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Assembly dispatcher helpers — extension `impl` block on `GuiState`.
//
// These methods are testable without `CadApp` and live here (rather than in
// `gui/mod.rs`) because they only touch assembly-specific state.
// ---------------------------------------------------------------------------

impl GuiState {
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

    /// Populate BOM entries from the current assembly and open the BOM
    /// dialog. Returns true if an assembly exists and BOM was computed.
    pub fn populate_bom_entries(&mut self) -> bool {
        if let Some(asm) = self.assembly.as_ref() {
            let entries = asm.bill_of_materials();
            self.active_dialog = Some(ActiveDialog::Bom(entries));
            true
        } else {
            false
        }
    }

    /// Open the joint editor for the given type. Returns false (no-op) if
    /// the current assembly has fewer components than the joint variant
    /// requires (1 for Grounded, 2 for every other type).
    pub fn open_joint_editor(&mut self, jtype: AssemblyJointType) -> bool {
        let n = self
            .assembly
            .as_ref()
            .map(|a| a.num_components())
            .unwrap_or(0);
        if n < jtype.min_components() {
            return false;
        }
        self.active_dialog = Some(ActiveDialog::JointEditor(JointEditorState::new(jtype, n)));
        true
    }

    /// Commit the currently-edited joint to `assembly.joints` based on the
    /// editor state. Returns true on success.
    pub fn commit_assembly_joint(&mut self) -> bool {
        use cadkernel_modeling::JointType;
        // Take the editor state out so we can re-borrow `self.assembly` mutably.
        let state = match self.active_dialog.take() {
            Some(ActiveDialog::JointEditor(s)) => s,
            other => {
                // Not a joint-editor dialog: restore and bail.
                self.active_dialog = other;
                return false;
            }
        };
        let asm = match self.assembly.as_mut() {
            Some(a) => a,
            None => return false,
        };
        let a = state.comp_a;
        let b = state.comp_b;
        let axis = cadkernel_math::Vec3 {
            x: state.axis[0] as f64,
            y: state.axis[1] as f64,
            z: state.axis[2] as f64,
        };
        let origin = cadkernel_math::Point3 {
            x: state.origin[0] as f64,
            y: state.origin[1] as f64,
            z: state.origin[2] as f64,
        };
        let joint = match state.joint_type {
            AssemblyJointType::Grounded => JointType::Grounded,
            AssemblyJointType::Fixed => JointType::FixedJoint {
                component_a: a,
                component_b: b,
            },
            AssemblyJointType::Revolute => JointType::Revolute {
                component_a: a,
                component_b: b,
                axis,
                origin,
            },
            AssemblyJointType::Cylindrical => JointType::Cylindrical {
                component_a: a,
                component_b: b,
                axis,
                origin,
            },
            AssemblyJointType::Slider => JointType::Slider {
                component_a: a,
                component_b: b,
                axis,
            },
            AssemblyJointType::Ball => JointType::BallJoint {
                component_a: a,
                component_b: b,
                center: origin,
            },
            AssemblyJointType::Distance => JointType::AngleJoint {
                component_a: a,
                component_b: b,
                angle: state.angle,
            },
            AssemblyJointType::Angle => JointType::AngleJoint {
                component_a: a,
                component_b: b,
                angle: state.angle,
            },
            AssemblyJointType::Parallel => JointType::ParallelAxes {
                component_a: a,
                component_b: b,
                axis_a: axis,
                axis_b: axis,
            },
            AssemblyJointType::Perpendicular => JointType::PerpendicularAxes {
                component_a: a,
                component_b: b,
                axis_a: axis,
                axis_b: axis,
            },
            AssemblyJointType::Gear => JointType::GearJoint {
                component_a: a,
                component_b: b,
                ratio: state.ratio,
            },
            AssemblyJointType::Rack => JointType::RackAndPinion {
                component_a: a,
                component_b: b,
                pitch_radius: state.pitch,
            },
            AssemblyJointType::Screw => JointType::ScrewJoint {
                component_a: a,
                component_b: b,
                axis,
                pitch: state.pitch,
            },
            AssemblyJointType::Belt => JointType::BeltJoint {
                component_a: a,
                component_b: b,
                ratio: state.ratio,
            },
        };
        asm.add_joint(joint);
        true
    }
}

// ---------------------------------------------------------------------------
// Assembly dispatcher helper tests (Phase N full)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod assembly_helper_tests {
    use super::*;
    use crate::gui::BcKind;
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
        let entries = match &gui.active_dialog {
            Some(ActiveDialog::Bom(e)) => e.clone(),
            other => panic!("expected Bom dialog, got {other:?}"),
        };
        assert_eq!(entries.len(), 2);
        let gear = entries.iter().find(|e| e.name == "Gear").unwrap();
        let shaft = entries.iter().find(|e| e.name == "Shaft").unwrap();
        assert_eq!(gear.quantity, 2);
        assert_eq!(shaft.quantity, 1);
    }

    #[test]
    fn bill_of_materials_no_assembly_returns_false() {
        let mut gui = GuiState::new();
        assert!(!gui.populate_bom_entries());
        assert!(gui.active_dialog.is_none());
    }

    #[test]
    fn open_joint_editor_requires_two_components() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["Only"]));
        assert!(!gui.open_joint_editor(AssemblyJointType::Revolute));
        assert!(gui.active_dialog.is_none());
    }

    #[test]
    fn open_joint_editor_ok_with_two_plus_components() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Revolute));
        let s = gui.joint_editor_state().expect("joint editor open");
        assert_eq!(s.joint_type, AssemblyJointType::Revolute);
        assert_eq!(s.comp_a, 0);
        assert_eq!(s.comp_b, 1);
    }

    #[test]
    fn commit_revolute_joint_appends_one_revolute() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["A", "B"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Revolute));
        {
            let s = gui.joint_editor_state_mut().expect("joint editor open");
            s.axis = [0.0, 0.0, 1.0];
            s.origin = [1.0, 2.0, 3.0];
        }
        assert!(gui.commit_assembly_joint());
        let joints = &gui.assembly.as_ref().unwrap().joints;
        assert_eq!(joints.len(), 1);
        assert!(matches!(joints[0], JointType::Revolute { .. }));
        assert!(gui.active_dialog.is_none());
    }

    #[test]
    fn open_grounded_joint_ok_with_one_component() {
        let mut gui = GuiState::new();
        gui.assembly = Some(make_assembly_with_n_named(&["Only"]));
        assert!(gui.open_joint_editor(AssemblyJointType::Grounded));
        let s = gui.joint_editor_state().expect("joint editor open");
        assert_eq!(s.joint_type, AssemblyJointType::Grounded);
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
            axis: cadkernel_math::Vec3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
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
        let f = JointType::FixedJoint {
            component_a: 2,
            component_b: 3,
        };
        assert_eq!(joint_label(&f), "FixedJoint(2\u{2194}3)");
        let g = JointType::GearJoint {
            component_a: 4,
            component_b: 5,
            ratio: 2.0,
        };
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
        let conc = C::Concentric {
            comp_a: ComponentId(2),
            comp_b: ComponentId(3),
        };
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

    /// Opening a second stateful dialog must replace the first — the
    /// `ActiveDialog` enum encodes "at most one open" by construction.
    #[test]
    fn active_dialog_is_mutually_exclusive_by_construction() {
        let mut gui = GuiState::new();
        assert!(gui.active_dialog.is_none());

        gui.open_material_picker();
        assert!(matches!(
            gui.active_dialog,
            Some(ActiveDialog::MaterialPicker(_))
        ));

        // Opening a different stateful dialog evicts the previous one.
        gui.open_bc_editor(BcKind::Force);
        assert!(matches!(gui.active_dialog, Some(ActiveDialog::BcEditor(_))));

        gui.close_active_dialog();
        assert!(gui.active_dialog.is_none());
    }
}
