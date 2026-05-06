//! Pre-built project templates for quick-start workflows.
//!
//! Each template function returns a [`BRepModel`] populated with one or more
//! solids. The [`template_list`] function returns metadata for all available
//! templates, suitable for populating a "New from Template" dialog.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::Point3;
use cadkernel_topology::BRepModel;

use crate::boolean::{BooleanOp, boolean_op};
use crate::gear::make_involute_gear;
use crate::primitives::{make_box, make_cylinder};

/// Category hint for template preview thumbnails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewType {
    Empty,
    Box,
    Assembly,
    Part,
    Gear,
}

/// Metadata for a single project template.
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    /// Short display name (e.g. "Empty Project").
    pub name: &'static str,
    /// One-line description shown in template picker.
    pub description: &'static str,
    /// Category hint for choosing a preview icon or thumbnail.
    pub preview_type: PreviewType,
}

/// Returns metadata for every available template.
///
/// The order matches the display order in the template picker dialog.
pub fn template_list() -> Vec<TemplateInfo> {
    vec![
        TemplateInfo {
            name: "Empty Project",
            description: "Blank model with no geometry",
            preview_type: PreviewType::Empty,
        },
        TemplateInfo {
            name: "Single Box",
            description: "One parametric box (width x height x depth)",
            preview_type: PreviewType::Box,
        },
        TemplateInfo {
            name: "Basic Assembly",
            description: "Box and cylinder placed side-by-side",
            preview_type: PreviewType::Assembly,
        },
        TemplateInfo {
            name: "Mechanical Part",
            description: "Box with a through-hole (boolean subtract)",
            preview_type: PreviewType::Part,
        },
        TemplateInfo {
            name: "Gear Demo",
            description: "Involute spur gear (20-degree pressure angle)",
            preview_type: PreviewType::Gear,
        },
    ]
}

/// Returns a completely empty model.
pub fn template_empty() -> BRepModel {
    BRepModel::new()
}

/// Returns a model containing a single axis-aligned box.
///
/// The box is placed at the origin with the given dimensions.
pub fn template_single_box(w: f64, h: f64, d: f64) -> KernelResult<BRepModel> {
    if w <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "width must be > 0, got {w}"
        )));
    }
    if h <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "height must be > 0, got {h}"
        )));
    }
    if d <= 0.0 {
        return Err(KernelError::InvalidArgument(format!(
            "depth must be > 0, got {d}"
        )));
    }
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, w, h, d)?;
    Ok(model)
}

/// Returns a model with two solids: a box and a cylinder placed beside it.
///
/// The box is 10x10x10 at the origin. The cylinder (radius 3, height 10)
/// sits at (15, 5, 0) so the two solids are separate but close together,
/// forming a minimal two-part assembly.
pub fn template_basic_assembly() -> KernelResult<BRepModel> {
    let mut model = BRepModel::new();
    make_box(&mut model, Point3::ORIGIN, 10.0, 10.0, 10.0)?;
    make_cylinder(&mut model, Point3::new(15.0, 5.0, 0.0), 3.0, 10.0, 64)?;
    Ok(model)
}

/// Returns a box with a cylindrical through-hole produced by boolean subtract.
///
/// The base block is 20x20x10. A cylinder of radius 4 and height 12 is
/// positioned so it punches completely through the top face, creating a
/// classic mechanical bracket shape.
pub fn template_mechanical_part() -> KernelResult<BRepModel> {
    // Base block
    let mut block_model = BRepModel::new();
    let block = make_box(&mut block_model, Point3::ORIGIN, 20.0, 20.0, 10.0)?;

    // Through-hole tool (cylinder taller than block to guarantee full cut)
    let mut tool_model = BRepModel::new();
    let tool = make_cylinder(
        &mut tool_model,
        Point3::new(10.0, 10.0, -1.0),
        4.0,
        12.0,
        64,
    )?;

    // Boolean subtract to create the hole
    let result = boolean_op(
        &block_model,
        block.solid,
        &tool_model,
        tool.solid,
        BooleanOp::Difference,
    )?;
    Ok(result)
}

/// Returns a model containing an involute spur gear.
///
/// Parameters: module 2.0, 20 teeth, 20-degree pressure angle, face width 5.0.
pub fn template_gear_demo() -> KernelResult<BRepModel> {
    let mut model = BRepModel::new();
    let pressure_20deg = 20.0_f64.to_radians();
    make_involute_gear(&mut model, 2.0, 20, pressure_20deg, 5.0)?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{check_geometry, check_watertight};

    #[test]
    fn test_template_empty() {
        let model = template_empty();
        assert_eq!(model.solids.len(), 0);
        assert_eq!(model.faces.len(), 0);
    }

    #[test]
    fn test_template_single_box() {
        let model = template_single_box(5.0, 3.0, 2.0).unwrap();
        assert_eq!(model.solids.len(), 1);
        assert_eq!(model.faces.len(), 6);
        let (solid_h, _) = model.solids.iter().next().unwrap();
        let check = check_geometry(&model, solid_h);
        assert!(check.is_valid, "box should be valid: {:?}", check.issues);
        assert!(
            check_watertight(&model, solid_h),
            "box should be watertight"
        );
    }

    #[test]
    fn test_template_single_box_invalid_width() {
        let result = template_single_box(-1.0, 2.0, 3.0);
        assert!(result.is_err());
        let msg = result.err().unwrap().to_string();
        assert!(msg.contains("width"), "error should mention width: {msg}");
        assert!(
            msg.contains("-1"),
            "error should include the bad value: {msg}"
        );
    }

    #[test]
    fn test_template_single_box_invalid_height() {
        let result = template_single_box(1.0, 0.0, 3.0);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("height"));
    }

    #[test]
    fn test_template_single_box_invalid_depth() {
        let result = template_single_box(1.0, 2.0, -5.0);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("depth"));
    }

    #[test]
    fn test_template_basic_assembly() {
        let model = template_basic_assembly().unwrap();
        assert_eq!(model.solids.len(), 2, "assembly should have 2 solids");
        for (solid_h, _) in model.solids.iter() {
            let check = check_geometry(&model, solid_h);
            assert!(
                check.is_valid,
                "assembly solid should be valid: {:?}",
                check.issues
            );
        }
    }

    #[test]
    fn test_template_mechanical_part() {
        let model = template_mechanical_part().unwrap();
        assert!(
            !model.solids.is_empty(),
            "mechanical part should have at least one solid"
        );
        assert!(
            model.faces.len() > 6,
            "part with hole should have more than 6 faces, got {}",
            model.faces.len()
        );
    }

    #[test]
    fn test_template_gear_demo() {
        let model = template_gear_demo().unwrap();
        assert_eq!(model.solids.len(), 1, "gear should produce 1 solid");
        let (solid_h, _) = model.solids.iter().next().unwrap();
        let check = check_geometry(&model, solid_h);
        assert!(check.is_valid, "gear should be valid: {:?}", check.issues);
    }

    #[test]
    fn test_template_list_count() {
        let list = template_list();
        assert_eq!(list.len(), 5, "should have 5 templates");
    }

    #[test]
    fn test_template_list_names_unique() {
        let list = template_list();
        let mut names: Vec<&str> = list.iter().map(|t| t.name).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), 5, "template names must be unique");
    }

    #[test]
    fn test_template_list_preview_types() {
        let list = template_list();
        assert_eq!(list[0].preview_type, PreviewType::Empty);
        assert_eq!(list[1].preview_type, PreviewType::Box);
        assert_eq!(list[2].preview_type, PreviewType::Assembly);
        assert_eq!(list[3].preview_type, PreviewType::Part);
        assert_eq!(list[4].preview_type, PreviewType::Gear);
    }

    #[test]
    fn test_template_list_descriptions_nonempty() {
        let list = template_list();
        for t in &list {
            assert!(
                !t.description.is_empty(),
                "template {} has empty description",
                t.name
            );
        }
    }
}
