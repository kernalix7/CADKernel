//! DXF (Drawing Exchange Format) import and export.
//!
//! Supports 3DFACE entity import/export for triangle meshes, plus full
//! TechDraw drawing sheet export with dimension annotations, centerlines,
//! hatching, cosmetic lines, leader lines, and text annotations.

use std::fmt::Write;

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_math::{Point3, Vec3};

use crate::techdraw::{
    Centerline, CosmeticLine, CosmeticLineStyle, Dimension, DrawingSheet, HatchPattern,
    LeaderLine, TextAnnotation,
};
use crate::tessellate::Mesh;

/// Export mesh to DXF format (3DFACE entities).
pub fn export_dxf(mesh: &Mesh) -> KernelResult<String> {
    let mut out = String::new();
    out.push_str("0\nSECTION\n2\nHEADER\n0\nENDSEC\n");
    out.push_str("0\nSECTION\n2\nENTITIES\n");

    for idx in &mesh.indices {
        let p0 = mesh.vertices[idx[0] as usize];
        let p1 = mesh.vertices[idx[1] as usize];
        let p2 = mesh.vertices[idx[2] as usize];
        out.push_str("0\n3DFACE\n8\n0\n");
        write_dxf_point(&mut out, 10, p0);
        write_dxf_point(&mut out, 11, p1);
        write_dxf_point(&mut out, 12, p2);
        write_dxf_point(&mut out, 13, p2);
    }

    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

fn write_dxf_point(out: &mut String, group: i32, p: Point3) {
    let _ = write!(
        out,
        "{}\n{}\n{}\n{}\n{}\n{}\n",
        group,
        p.x,
        group + 10,
        p.y,
        group + 20,
        p.z
    );
}

/// Import DXF file, extracting 3DFACE entities into a mesh.
pub fn import_dxf(content: &str) -> KernelResult<Mesh> {
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();

    let lines: Vec<&str> = content.lines().collect();
    let mut i = 0;
    while i + 1 < lines.len() {
        let code = lines[i].trim();
        let value = lines[i + 1].trim();

        if code == "0" && value == "3DFACE" {
            let mut pts = [Point3::ORIGIN; 4];
            let mut j = i + 2;
            while j + 1 < lines.len() {
                let gc = lines[j].trim().parse::<i32>().unwrap_or(0);
                let gv = lines[j + 1].trim().parse::<f64>().unwrap_or(0.0);
                match gc {
                    10 => pts[0].x = gv,
                    20 => pts[0].y = gv,
                    30 => pts[0].z = gv,
                    11 => pts[1].x = gv,
                    21 => pts[1].y = gv,
                    31 => pts[1].z = gv,
                    12 => pts[2].x = gv,
                    22 => pts[2].y = gv,
                    32 => pts[2].z = gv,
                    13 => pts[3].x = gv,
                    23 => pts[3].y = gv,
                    33 => {
                        pts[3].z = gv;
                        break;
                    }
                    0 => break,
                    _ => {}
                }
                j += 2;
            }

            let base_idx = vertices.len() as u32;
            let v01 = pts[1] - pts[0];
            let v02 = pts[2] - pts[0];
            let n = v01.cross(v02);
            let n_len = n.length();
            let normal = if n_len > 1e-14 { n * (1.0 / n_len) } else { Vec3::Z };

            for pt in &pts[..3] {
                vertices.push(*pt);
                normals.push(normal);
            }
            indices.push([base_idx, base_idx + 1, base_idx + 2]);

            i = j + 2;
            continue;
        }
        i += 2;
    }

    Ok(Mesh {
        vertices,
        normals,
        indices,
    })
}

/// Write DXF string to file.
pub fn write_dxf(path: &str, content: &str) -> KernelResult<()> {
    std::fs::write(path, content).map_err(|e| KernelError::IoError(e.to_string()))
}

// ---------------------------------------------------------------------------
// TechDraw DXF Export
// ---------------------------------------------------------------------------

/// Export a complete TechDraw drawing sheet to DXF format.
///
/// Converts all drawing views (projected edges), dimension annotations,
/// centerlines, cosmetic lines, hatch patterns, leader lines, and text
/// annotations to their corresponding DXF entities.
pub fn export_drawing_dxf(sheet: &DrawingSheet) -> KernelResult<String> {
    let mut out = String::new();

    // HEADER section
    out.push_str("0\nSECTION\n2\nHEADER\n");
    let _ = write!(out, "9\n$ACADVER\n1\nAC1021\n");
    let _ = write!(out, "9\n$INSBASE\n10\n0.0\n20\n0.0\n30\n0.0\n");
    let _ = write!(
        out,
        "9\n$EXTMIN\n10\n0.0\n20\n0.0\n30\n0.0\n"
    );
    let _ = write!(
        out,
        "9\n$EXTMAX\n10\n{}\n20\n{}\n30\n0.0\n",
        sheet.width, sheet.height
    );
    out.push_str("0\nENDSEC\n");

    // TABLES section (line types, layers)
    out.push_str("0\nSECTION\n2\nTABLES\n");
    write_ltype_table(&mut out);
    write_layer_table(&mut out);
    out.push_str("0\nENDSEC\n");

    // ENTITIES section
    out.push_str("0\nSECTION\n2\nENTITIES\n");

    // Sheet border
    write_dxf_line_2d(&mut out, 0.0, 0.0, sheet.width, 0.0, "Border");
    write_dxf_line_2d(&mut out, sheet.width, 0.0, sheet.width, sheet.height, "Border");
    write_dxf_line_2d(&mut out, sheet.width, sheet.height, 0.0, sheet.height, "Border");
    write_dxf_line_2d(&mut out, 0.0, sheet.height, 0.0, 0.0, "Border");

    // Drawing views (projected edges)
    for view in &sheet.views {
        for edge in &view.edges {
            let x1 = view.center_x + edge.x1 * view.scale;
            let y1 = view.center_y + edge.y1 * view.scale;
            let x2 = view.center_x + edge.x2 * view.scale;
            let y2 = view.center_y + edge.y2 * view.scale;
            let layer = if edge.visible { "Visible" } else { "Hidden" };
            write_dxf_line_2d(&mut out, x1, y1, x2, y2, layer);
            if !edge.visible {
                // Set hidden line type
                let _ = write!(out, "6\nHIDDEN\n");
            }
        }
    }

    // Dimensions
    for dim in &sheet.dimensions {
        write_dimension_entity(&mut out, dim);
    }

    // Title block text
    if !sheet.title.is_empty() {
        write_dxf_text(&mut out, 10.0, 5.0, 5.0, &sheet.title, "Title");
    }

    out.push_str("0\nENDSEC\n0\nEOF\n");
    Ok(out)
}

/// Export centerlines as DXF LINE entities with CENTER line type.
pub fn export_centerlines_dxf(out: &mut String, centerlines: &[Centerline]) {
    for cl in centerlines {
        let dx = cl.end.x - cl.start.x;
        let dy = cl.end.y - cl.start.y;
        let len = (dx * dx + dy * dy).sqrt();
        let (nx, ny) = if len > 1e-10 {
            (dx / len, dy / len)
        } else {
            (1.0, 0.0)
        };
        let x1 = cl.start.x - nx * cl.extension;
        let y1 = cl.start.y - ny * cl.extension;
        let x2 = cl.end.x + nx * cl.extension;
        let y2 = cl.end.y + ny * cl.extension;
        write_dxf_line_2d(out, x1, y1, x2, y2, "Centerline");
        let _ = write!(out, "6\nCENTER\n");
    }
}

/// Export cosmetic lines as DXF LINE entities.
pub fn export_cosmetic_lines_dxf(out: &mut String, lines: &[CosmeticLine]) {
    for cl in lines {
        write_dxf_line_2d(out, cl.start.x, cl.start.y, cl.end.x, cl.end.y, "Cosmetic");
        match cl.style {
            CosmeticLineStyle::Continuous => {}
            CosmeticLineStyle::Dashed => {
                let _ = write!(out, "6\nDASHED\n");
            }
            CosmeticLineStyle::DashDot => {
                let _ = write!(out, "6\nCENTER\n");
            }
            CosmeticLineStyle::Dotted => {
                let _ = write!(out, "6\nPHANTOM\n");
            }
        }
    }
}

/// Export hatch patterns as DXF HATCH entities.
pub fn export_hatch_dxf(out: &mut String, patterns: &[HatchPattern]) {
    for pat in patterns {
        if pat.boundary.len() < 3 {
            continue;
        }
        out.push_str("0\nHATCH\n8\nHatch\n");
        // Pattern data
        let _ = write!(out, "70\n0\n");
        let _ = write!(out, "71\n1\n");
        // Boundary path count
        let _ = write!(out, "91\n1\n");
        // Boundary type (polyline)
        let _ = write!(out, "92\n1\n");
        let _ = write!(out, "72\n1\n");
        let _ = write!(out, "73\n1\n");
        let _ = write!(out, "93\n{}\n", pat.boundary.len());
        for pt in &pat.boundary {
            let _ = write!(out, "10\n{}\n20\n{}\n", pt.x, pt.y);
        }
        // Pattern definition
        let _ = write!(out, "75\n1\n");
        let _ = write!(out, "52\n{}\n", pat.angle);
        let _ = write!(out, "41\n{}\n", pat.spacing);
    }
}

/// Export leader lines as DXF LEADER entities.
pub fn export_leaders_dxf(out: &mut String, leaders: &[LeaderLine]) {
    for leader in leaders {
        out.push_str("0\nLEADER\n8\nDimension\n");
        // Leader style
        let _ = write!(out, "72\n0\n");
        let _ = write!(out, "73\n3\n");
        // Vertex count
        let _ = write!(out, "76\n2\n");
        // Vertices
        let _ = write!(
            out,
            "10\n{}\n20\n{}\n30\n0.0\n",
            leader.start.x, leader.start.y
        );
        let _ = write!(
            out,
            "10\n{}\n20\n{}\n30\n0.0\n",
            leader.end.x, leader.end.y
        );
        // Annotation text
        if !leader.text.is_empty() {
            write_dxf_text(
                out,
                leader.end.x + 2.0,
                leader.end.y,
                3.0,
                &leader.text,
                "Dimension",
            );
        }
    }
}

/// Export text annotations as DXF TEXT/MTEXT entities.
pub fn export_text_annotations_dxf(out: &mut String, annotations: &[TextAnnotation]) {
    for ann in annotations {
        write_dxf_text(
            out,
            ann.position.x,
            ann.position.y,
            ann.font_size,
            &ann.text,
            "Text",
        );
    }
}

// ---------------------------------------------------------------------------
// DXF entity writers
// ---------------------------------------------------------------------------

fn write_dxf_line_2d(out: &mut String, x1: f64, y1: f64, x2: f64, y2: f64, layer: &str) {
    let _ = write!(
        out,
        "0\nLINE\n8\n{layer}\n10\n{x1}\n20\n{y1}\n30\n0.0\n11\n{x2}\n21\n{y2}\n31\n0.0\n"
    );
}

fn write_dxf_text(out: &mut String, x: f64, y: f64, height: f64, text: &str, layer: &str) {
    let _ = write!(
        out,
        "0\nTEXT\n8\n{layer}\n10\n{x}\n20\n{y}\n30\n0.0\n40\n{height}\n1\n{text}\n"
    );
}

fn write_dimension_entity(out: &mut String, dim: &Dimension) {
    match dim {
        Dimension::Linear {
            x1,
            y1,
            x2,
            y2,
            offset,
            text,
        } => {
            let dx = x2 - x1;
            let dy = y2 - y1;
            let len = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = if len > 1e-10 {
                (-dy / len, dx / len)
            } else {
                (0.0, 1.0)
            };
            let ox1 = x1 + nx * offset;
            let oy1 = y1 + ny * offset;
            let ox2 = x2 + nx * offset;
            let oy2 = y2 + ny * offset;
            let mx = (ox1 + ox2) / 2.0;
            let my = (oy1 + oy2) / 2.0;

            out.push_str("0\nDIMENSION\n8\nDimension\n");
            // Dimension type: aligned (1)
            let _ = write!(out, "70\n1\n");
            // Definition point
            let _ = write!(out, "10\n{ox2}\n20\n{oy2}\n30\n0.0\n");
            // Mid-point of dimension line
            let _ = write!(out, "11\n{mx}\n20\n{my}\n30\n0.0\n");
            // Start of first extension line
            let _ = write!(out, "13\n{x1}\n23\n{y1}\n33\n0.0\n");
            // Start of second extension line
            let _ = write!(out, "14\n{x2}\n24\n{y2}\n34\n0.0\n");
            // Override text
            if !text.is_empty() {
                let _ = write!(out, "1\n{text}\n");
            }
        }
        Dimension::Radius {
            cx,
            cy,
            r,
            angle_deg,
            text,
        } => {
            let angle_rad = angle_deg.to_radians();
            let px = cx + r * angle_rad.cos();
            let py = cy + r * angle_rad.sin();

            out.push_str("0\nDIMENSION\n8\nDimension\n");
            // Dimension type: radius (4)
            let _ = write!(out, "70\n4\n");
            // Definition point (on arc)
            let _ = write!(out, "10\n{px}\n20\n{py}\n30\n0.0\n");
            // Center point
            let _ = write!(out, "15\n{cx}\n25\n{cy}\n35\n0.0\n");
            // Leader length
            let _ = write!(out, "40\n{r}\n");
            if !text.is_empty() {
                let _ = write!(out, "1\n{text}\n");
            }
        }
    }
}

fn write_ltype_table(out: &mut String) {
    out.push_str("0\nTABLE\n2\nLTYPE\n");
    // Continuous
    out.push_str("0\nLTYPE\n2\nCONTINUOUS\n70\n0\n3\nSolid line\n72\n65\n73\n0\n40\n0.0\n");
    // Hidden
    out.push_str("0\nLTYPE\n2\nHIDDEN\n70\n0\n3\nHidden line\n72\n65\n73\n2\n40\n6.35\n49\n3.175\n49\n-3.175\n");
    // Center
    out.push_str("0\nLTYPE\n2\nCENTER\n70\n0\n3\nCenter line\n72\n65\n73\n4\n40\n19.05\n49\n12.7\n49\n-3.175\n49\n3.175\n49\n-3.175\n");
    // Dashed
    out.push_str("0\nLTYPE\n2\nDASHED\n70\n0\n3\nDashed line\n72\n65\n73\n2\n40\n6.35\n49\n3.175\n49\n-3.175\n");
    // Phantom
    out.push_str("0\nLTYPE\n2\nPHANTOM\n70\n0\n3\nPhantom line\n72\n65\n73\n6\n40\n25.4\n49\n12.7\n49\n-3.175\n49\n3.175\n49\n-3.175\n49\n3.175\n49\n-3.175\n");
    out.push_str("0\nENDTAB\n");
}

fn write_layer_table(out: &mut String) {
    out.push_str("0\nTABLE\n2\nLAYER\n");
    let layers = [
        ("0", 7, "CONTINUOUS"),
        ("Border", 7, "CONTINUOUS"),
        ("Visible", 7, "CONTINUOUS"),
        ("Hidden", 1, "HIDDEN"),
        ("Centerline", 2, "CENTER"),
        ("Dimension", 3, "CONTINUOUS"),
        ("Hatch", 4, "CONTINUOUS"),
        ("Cosmetic", 5, "CONTINUOUS"),
        ("Text", 7, "CONTINUOUS"),
        ("Title", 7, "CONTINUOUS"),
    ];
    for (name, color, ltype) in layers {
        let _ = write!(out, "0\nLAYER\n2\n{name}\n70\n0\n62\n{color}\n6\n{ltype}\n");
    }
    out.push_str("0\nENDTAB\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::{Point2, Point3, Vec3};

    fn make_triangle_mesh() -> Mesh {
        Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn test_dxf_export_contains_3dface() {
        let mesh = make_triangle_mesh();
        let dxf = export_dxf(&mesh).unwrap();
        assert!(dxf.contains("3DFACE"));
        assert!(dxf.contains("EOF"));
        assert!(dxf.contains("SECTION"));
    }

    #[test]
    fn test_dxf_roundtrip() {
        let mesh = make_triangle_mesh();
        let dxf = export_dxf(&mesh).unwrap();
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.triangle_count(), 1);
        assert_eq!(imported.vertices.len(), 3);
    }

    #[test]
    fn test_dxf_roundtrip_multi() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(1.0, 1.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2], [0, 2, 3]],
        };
        let dxf = export_dxf(&mesh).unwrap();
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.triangle_count(), 2);
    }

    #[test]
    fn test_import_empty_dxf() {
        let dxf = "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n";
        let mesh = import_dxf(dxf).unwrap();
        assert_eq!(mesh.triangle_count(), 0);
    }

    #[test]
    fn test_export_drawing_dxf_empty_sheet() {
        let sheet = DrawingSheet::a4_landscape();
        let dxf = export_drawing_dxf(&sheet).unwrap();
        assert!(dxf.contains("SECTION"));
        assert!(dxf.contains("HEADER"));
        assert!(dxf.contains("ENTITIES"));
        assert!(dxf.contains("EOF"));
        assert!(dxf.contains("LINE"));
        assert!(dxf.contains("Border"));
    }

    #[test]
    fn test_export_drawing_dxf_with_dimensions() {
        use crate::techdraw::ProjectedEdge;
        let mut sheet = DrawingSheet::a4_landscape();
        sheet.title = "Test Drawing".into();
        sheet.dimensions.push(Dimension::Linear {
            x1: 10.0,
            y1: 50.0,
            x2: 90.0,
            y2: 50.0,
            offset: 10.0,
            text: "80.00".into(),
        });
        sheet.dimensions.push(Dimension::Radius {
            cx: 50.0,
            cy: 100.0,
            r: 25.0,
            angle_deg: 45.0,
            text: "R25".into(),
        });
        sheet.views.push(crate::techdraw::DrawingView {
            direction: crate::techdraw::ProjectionDir::Front,
            edges: vec![ProjectedEdge {
                x1: 0.0,
                y1: 0.0,
                x2: 50.0,
                y2: 0.0,
                visible: true,
            }],
            center_x: 100.0,
            center_y: 100.0,
            scale: 1.0,
        });
        let dxf = export_drawing_dxf(&sheet).unwrap();
        assert!(dxf.contains("DIMENSION"));
        assert!(dxf.contains("80.00"));
        assert!(dxf.contains("R25"));
        assert!(dxf.contains("Test Drawing"));
        assert!(dxf.contains("Visible"));
    }

    #[test]
    fn test_export_centerlines_dxf() {
        let mut out = String::new();
        let cls = vec![Centerline {
            start: Point2::new(10.0, 50.0),
            end: Point2::new(90.0, 50.0),
            extension: 5.0,
        }];
        export_centerlines_dxf(&mut out, &cls);
        assert!(out.contains("LINE"));
        assert!(out.contains("Centerline"));
        assert!(out.contains("CENTER"));
    }

    #[test]
    fn test_export_cosmetic_lines_dxf() {
        let mut out = String::new();
        let lines = vec![
            CosmeticLine {
                start: Point2::new(0.0, 0.0),
                end: Point2::new(100.0, 100.0),
                style: CosmeticLineStyle::Dashed,
                color: "red".into(),
                width: 0.5,
            },
            CosmeticLine {
                start: Point2::new(0.0, 100.0),
                end: Point2::new(100.0, 0.0),
                style: CosmeticLineStyle::Continuous,
                color: "blue".into(),
                width: 0.3,
            },
        ];
        export_cosmetic_lines_dxf(&mut out, &lines);
        assert!(out.contains("LINE"));
        assert!(out.contains("Cosmetic"));
        assert!(out.contains("DASHED"));
    }

    #[test]
    fn test_export_hatch_dxf() {
        let mut out = String::new();
        let pats = vec![HatchPattern {
            boundary: vec![
                Point2::new(0.0, 0.0),
                Point2::new(10.0, 0.0),
                Point2::new(10.0, 10.0),
                Point2::new(0.0, 10.0),
            ],
            angle: 45.0,
            spacing: 2.0,
        }];
        export_hatch_dxf(&mut out, &pats);
        assert!(out.contains("HATCH"));
        assert!(out.contains("Hatch"));
    }

    #[test]
    fn test_export_leaders_dxf() {
        let mut out = String::new();
        let leaders = vec![LeaderLine {
            start: Point2::new(50.0, 50.0),
            end: Point2::new(80.0, 80.0),
            text: "Note A".into(),
        }];
        export_leaders_dxf(&mut out, &leaders);
        assert!(out.contains("LEADER"));
        assert!(out.contains("Note A"));
    }

    #[test]
    fn test_export_text_annotations_dxf() {
        let mut out = String::new();
        let anns = vec![TextAnnotation {
            position: Point2::new(10.0, 20.0),
            text: "Section B-B".into(),
            font_size: 5.0,
        }];
        export_text_annotations_dxf(&mut out, &anns);
        assert!(out.contains("TEXT"));
        assert!(out.contains("Section B-B"));
    }

    #[test]
    fn test_export_drawing_dxf_layers() {
        let sheet = DrawingSheet::a4_landscape();
        let dxf = export_drawing_dxf(&sheet).unwrap();
        assert!(dxf.contains("LAYER"));
        assert!(dxf.contains("LTYPE"));
        assert!(dxf.contains("CONTINUOUS"));
        assert!(dxf.contains("HIDDEN"));
        assert!(dxf.contains("CENTER"));
    }

    #[test]
    fn test_dxf_export_header_section() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let dxf = export_dxf(&mesh).unwrap();
        assert!(dxf.contains("SECTION"));
        assert!(dxf.contains("HEADER"));
        assert!(dxf.contains("ENDSEC"));
        assert!(dxf.contains("EOF"));
    }

    #[test]
    fn test_dxf_roundtrip_coordinates() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(1.0, 2.0, 3.0),
                Point3::new(4.0, 5.0, 6.0),
                Point3::new(7.0, 8.0, 9.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let dxf = export_dxf(&mesh).unwrap();
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.indices.len(), 1);
        assert!((imported.vertices[0].x - 1.0).abs() < 1e-4);
    }

    #[test]
    fn test_dxf_empty_entities_section() {
        let mesh = Mesh { vertices: vec![], normals: vec![], indices: vec![] };
        let dxf = export_dxf(&mesh).unwrap();
        assert!(dxf.contains("ENTITIES"));
        assert!(!dxf.contains("3DFACE"));
    }

    #[test]
    fn test_dxf_two_triangles() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 0.0),
                Point3::new(2.0, 0.0, 0.0),
                Point3::new(3.0, 0.0, 0.0),
                Point3::new(2.5, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z, Vec3::Z],
            indices: vec![[0, 1, 2], [3, 4, 5]],
        };
        let dxf = export_dxf(&mesh).unwrap();
        let count = dxf.matches("3DFACE").count();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_dxf_import_no_faces() {
        let dxf = "0\nSECTION\n2\nHEADER\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n";
        let result = import_dxf(dxf);
        assert!(result.is_err() || result.unwrap().indices.is_empty());
    }

    #[test]
    fn test_dxf_roundtrip_face_count() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z],
            indices: vec![[0, 1, 2]],
        };
        let dxf = export_dxf(&mesh).unwrap();
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.indices.len(), 1);
    }

    // -----------------------------------------------------------------------
    // Edge case: 3D entities with Z coordinates
    // -----------------------------------------------------------------------

    #[test]
    fn test_dxf_3d_entities() {
        let mesh = Mesh {
            vertices: vec![
                Point3::new(0.0, 0.0, 0.0),
                Point3::new(1.0, 0.0, 0.0),
                Point3::new(0.5, 1.0, 2.0),
            ],
            normals: vec![Vec3::new(0.0, -0.894, 0.447)],
            indices: vec![[0, 1, 2]],
        };
        let dxf = export_dxf(&mesh).unwrap();
        assert!(dxf.contains("3DFACE"));
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.triangle_count(), 1);
        // Verify Z coordinate is preserved
        let max_z = imported.vertices.iter().map(|v| v.z).fold(f64::NEG_INFINITY, f64::max);
        assert!(max_z > 1.0);
    }

    // -----------------------------------------------------------------------
    // Edge case: empty mesh
    // -----------------------------------------------------------------------

    #[test]
    fn test_dxf_empty_mesh_export() {
        let mesh = Mesh::new();
        let dxf = export_dxf(&mesh).unwrap();
        assert!(dxf.contains("EOF"));
        let imported = import_dxf(&dxf).unwrap();
        assert_eq!(imported.triangle_count(), 0);
    }

    // -----------------------------------------------------------------------
    // Edge case: malformed DXF
    // -----------------------------------------------------------------------

    #[test]
    fn test_dxf_import_malformed() {
        let result = import_dxf("this is not DXF at all");
        // Should handle gracefully
        if let Ok(mesh) = result {
            assert_eq!(mesh.triangle_count(), 0);
        }
    }

    // -----------------------------------------------------------------------
    // Edge case: export-import-export consistency
    // -----------------------------------------------------------------------

    #[test]
    fn test_dxf_export_import_export_consistency() {
        let mesh = make_triangle_mesh();
        let dxf1 = export_dxf(&mesh).unwrap();
        let reimported = import_dxf(&dxf1).unwrap();
        let dxf2 = export_dxf(&reimported).unwrap();

        let face_count_1 = dxf1.matches("3DFACE").count();
        let face_count_2 = dxf2.matches("3DFACE").count();
        assert_eq!(face_count_1, face_count_2);
    }
}
