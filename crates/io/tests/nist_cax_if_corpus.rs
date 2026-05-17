//! NIST CAx-IF-style STEP corpus round-trip checks for v1.0 Gate #17.
//!
//! Official CAx-IF exchange rounds are not bundled here. The fixture directory
//! uses license-clear NIST EDM and STEPcode public corpus files that exercise
//! the same STEP B-Rep and assembly shapes required by this gate.

use std::path::{Path, PathBuf};

use cadkernel_io::tessellate::tessellate_solid;
use cadkernel_io::{export_step, import_step};
use cadkernel_math::Point3;
use cadkernel_topology::BRepModel;

const MIN_FACE_AREA_JACCARD: f64 = 0.999;
const MAX_VERTEX_DELTA: f64 = 1.0e-6;

#[derive(Debug, Clone, Copy)]
struct FixtureCase {
    name: &'static str,
    file_name: &'static str,
    required_markers: &'static [&'static str],
}

const FIXTURES: &[FixtureCase] = &[
    FixtureCase {
        name: "planar_cylindrical_ap203",
        file_name: "caxif_planar_cylindrical_ap203.step",
        required_markers: &["PLANE", "CYLINDRICAL_SURFACE"],
    },
    FixtureCase {
        name: "interacting_pockets_ap203",
        file_name: "caxif_interacting_pockets_ap203.stp",
        required_markers: &["PLANE", "CYLINDRICAL_SURFACE"],
    },
    FixtureCase {
        name: "conical_clevis_ap203",
        file_name: "caxif_conical_clevis_ap203.stp",
        required_markers: &["CONICAL_SURFACE", "CYLINDRICAL_SURFACE"],
    },
    FixtureCase {
        name: "bspline_bracket_ap203",
        file_name: "caxif_bspline_bracket_ap203.stp",
        required_markers: &["B_SPLINE_CURVE_WITH_KNOTS", "CYLINDRICAL_SURFACE"],
    },
    FixtureCase {
        name: "assembly_ap203",
        file_name: "caxif_assembly_ap203.stp",
        required_markers: &[
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
            "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
        ],
    },
    FixtureCase {
        name: "trimmed_nurbs_assembly_ap214",
        file_name: "caxif_trimmed_nurbs_assembly_ap214.stp",
        required_markers: &[
            "B_SPLINE_SURFACE_WITH_KNOTS",
            "B_SPLINE_CURVE_WITH_KNOTS",
            "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
        ],
    },
    FixtureCase {
        name: "toroidal_ap214",
        file_name: "caxif_toroidal_ap214.stp",
        required_markers: &["TOROIDAL_SURFACE", "CYLINDRICAL_SURFACE"],
    },
    FixtureCase {
        name: "vendor_cone_ap214",
        file_name: "caxif_vendor_cone_ap214.stp",
        required_markers: &["CONICAL_SURFACE", "CYLINDRICAL_SURFACE"],
    },
];

#[derive(Debug, Clone, Copy)]
struct RoundTripMetrics {
    face_area_jaccard: f64,
    source_face_area: f64,
    imported_face_area: f64,
    max_vertex_delta: f64,
    source_vertex_count: usize,
    imported_vertex_count: usize,
    source_face_count: usize,
    imported_face_count: usize,
    source_solid_count: usize,
    imported_solid_count: usize,
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("nist")
}

fn fixture_path(file_name: &str) -> PathBuf {
    fixture_root().join(file_name)
}

fn metadata_path(file_name: &str) -> PathBuf {
    fixture_root().join(format!("{file_name}.md"))
}

fn read_fixture(case: FixtureCase) -> String {
    let path = fixture_path(case.file_name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read fixture {}: {e}", path.display()));
    if content.contains("ISO-10303-21") {
        return content;
    }

    let target = content.trim();
    if target.starts_with("../") {
        let target_path = path
            .parent()
            .expect("fixture path has parent directory")
            .join(target);
        return std::fs::read_to_string(&target_path).unwrap_or_else(|e| {
            panic!("read fixture symlink target {}: {e}", target_path.display())
        });
    }

    content
}

fn total_face_area(model: &BRepModel) -> f64 {
    let mut total = 0.0_f64;
    for (solid_h, _) in model.solids.iter() {
        let mesh = tessellate_solid(model, solid_h);
        for idx in &mesh.indices {
            let a = mesh.vertices[idx[0] as usize];
            let b = mesh.vertices[idx[1] as usize];
            let c = mesh.vertices[idx[2] as usize];
            let ab = b - a;
            let ac = c - a;
            total += 0.5 * ab.cross(ac).length();
        }
    }
    total
}

fn face_area_jaccard(source: f64, imported: f64) -> f64 {
    if source == 0.0 && imported == 0.0 {
        return 1.0;
    }
    let lo = source.min(imported);
    let hi = source.max(imported);
    if hi == 0.0 { 0.0 } else { lo / hi }
}

fn max_vertex_delta(lhs: &BRepModel, rhs: &BRepModel) -> f64 {
    if lhs.vertices.is_empty() && rhs.vertices.is_empty() {
        return 0.0;
    }
    if lhs.vertices.is_empty() || rhs.vertices.is_empty() {
        return f64::INFINITY;
    }

    let lhs_points: Vec<Point3> = lhs.vertices.iter().map(|(_, v)| v.point).collect();
    let rhs_points: Vec<Point3> = rhs.vertices.iter().map(|(_, v)| v.point).collect();
    max_nearest_distance(&lhs_points, &rhs_points)
        .max(max_nearest_distance(&rhs_points, &lhs_points))
}

fn max_nearest_distance(source: &[Point3], target: &[Point3]) -> f64 {
    source
        .iter()
        .map(|p| {
            target
                .iter()
                .map(|q| p.distance_to(*q))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn round_trip_metrics(content: &str) -> RoundTripMetrics {
    let source = import_step(content).expect("import source STEP fixture");
    let exported = export_step(&source).expect("export imported STEP fixture");
    let imported = import_step(&exported).expect("re-import exported STEP fixture");
    let source_face_area = total_face_area(&source);
    let imported_face_area = total_face_area(&imported);

    RoundTripMetrics {
        face_area_jaccard: face_area_jaccard(source_face_area, imported_face_area),
        source_face_area,
        imported_face_area,
        max_vertex_delta: max_vertex_delta(&source, &imported),
        source_vertex_count: source.vertices.len(),
        imported_vertex_count: imported.vertices.len(),
        source_face_count: source.faces.len(),
        imported_face_count: imported.faces.len(),
        source_solid_count: source.solids.len(),
        imported_solid_count: imported.solids.len(),
    }
}

fn assert_fixture_round_trip(case: FixtureCase) {
    let content = read_fixture(case);
    for marker in case.required_markers {
        assert!(
            content.contains(marker),
            "{}: fixture missing expected STEP marker {marker}",
            case.name
        );
    }

    let metrics = round_trip_metrics(&content);
    eprintln!(
        "{}: Jaccard={:.12}, area={:.12}->{:.12}, max_vertex_delta={:.12e}, vertices={}->{}, faces={}->{}, solids={}->{}",
        case.name,
        metrics.face_area_jaccard,
        metrics.source_face_area,
        metrics.imported_face_area,
        metrics.max_vertex_delta,
        metrics.source_vertex_count,
        metrics.imported_vertex_count,
        metrics.source_face_count,
        metrics.imported_face_count,
        metrics.source_solid_count,
        metrics.imported_solid_count
    );

    assert!(
        metrics.source_solid_count > 0,
        "{}: source STEP import produced no solids",
        case.name
    );
    assert_eq!(
        metrics.source_vertex_count, metrics.imported_vertex_count,
        "{}: vertex count changed",
        case.name
    );
    assert!(
        metrics.face_area_jaccard >= MIN_FACE_AREA_JACCARD,
        "{}: face-area Jaccard {} below {MIN_FACE_AREA_JACCARD}",
        case.name,
        metrics.face_area_jaccard
    );
    assert!(
        metrics.max_vertex_delta <= MAX_VERTEX_DELTA,
        "{}: max vertex delta {} above {MAX_VERTEX_DELTA}",
        case.name,
        metrics.max_vertex_delta
    );
}

fn assert_metadata(path: &Path) {
    let content =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        content.contains("Source:"),
        "{} missing Source attribution",
        path.display()
    );
    assert!(
        content.contains("License:"),
        "{} missing License attribution",
        path.display()
    );
}

#[test]
fn fixture_metadata_is_paired_with_each_step_file() {
    for case in FIXTURES {
        let step_path = fixture_path(case.file_name);
        assert!(
            step_path.exists(),
            "missing fixture {}",
            step_path.display()
        );
        assert_metadata(&metadata_path(case.file_name));
    }
}

#[test]
fn caxif_planar_cylindrical_ap203_round_trip() {
    assert_fixture_round_trip(FIXTURES[0]);
}

#[test]
fn caxif_interacting_pockets_ap203_round_trip() {
    assert_fixture_round_trip(FIXTURES[1]);
}

#[test]
fn caxif_conical_clevis_ap203_round_trip() {
    assert_fixture_round_trip(FIXTURES[2]);
}

#[test]
fn caxif_bspline_bracket_ap203_round_trip() {
    assert_fixture_round_trip(FIXTURES[3]);
}

#[test]
fn caxif_assembly_ap203_round_trip() {
    assert_fixture_round_trip(FIXTURES[4]);
}

#[test]
fn caxif_trimmed_nurbs_assembly_ap214_round_trip() {
    assert_fixture_round_trip(FIXTURES[5]);
}

#[test]
fn caxif_toroidal_ap214_round_trip() {
    assert_fixture_round_trip(FIXTURES[6]);
}

#[test]
fn caxif_vendor_cone_ap214_round_trip() {
    assert_fixture_round_trip(FIXTURES[7]);
}
