//! File I/O and tessellation for the CAD kernel.
//!
//! Supports reading and writing STL (ASCII & binary), Wavefront OBJ, SVG, and
//! JSON formats, plus face/solid tessellation into indexed triangle meshes.

pub mod gltf;
pub mod iges;
pub mod json;
pub mod native;
pub mod obj;
pub mod step;
pub mod stl;
pub mod svg;
pub mod tessellate;

pub use gltf::{export_gltf, write_gltf};
pub use iges::{IgesEntity, IgesEntityType, IgesWriter, read_iges_lines, read_iges_points};
pub use json::{export_json, import_json, model_from_json, model_to_json, read_json, write_json};
pub use native::{CADK_EXTENSION, load_project, save_project};
pub use obj::{export_obj, import_obj, read_obj, write_obj};
pub use step::{
    ParsedStepEntity, StepEntity, StepWriter, export_step_mesh, parse_step_entities,
    read_step_points,
};
pub use stl::{
    export_stl_ascii, export_stl_binary, import_stl, read_stl_ascii, read_stl_binary,
    write_stl_ascii, write_stl_binary,
};
pub use svg::{SvgDocument, SvgElement, SvgStyle, profile_to_svg};
pub use tessellate::{Mesh, Triangle, tessellate_face, tessellate_solid};

#[cfg(test)]
mod thread_safety_tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn io_types_are_send_sync() {
        assert_send_sync::<crate::Mesh>();
        assert_send_sync::<crate::Triangle>();
        assert_send_sync::<crate::StepWriter>();
        assert_send_sync::<crate::IgesWriter>();
    }
}
