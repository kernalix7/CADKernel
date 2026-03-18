//! Geometry and topology validation utilities.

use cadkernel_topology::{BRepModel, Handle, SolidData};

/// Result of a geometry check.
#[derive(Debug)]
pub struct CheckResult {
    pub is_valid: bool,
    pub issues: Vec<String>,
}

/// Checks a solid for topological validity.
///
/// Verifies:
/// 1. Solid has at least one shell
/// 2. Each shell has at least one face
/// 3. Each face has at least one loop
/// 4. Each loop has at least 3 half-edges
/// 5. Vertex coordinates are finite (not NaN/Inf)
/// 6. No degenerate faces (zero-area estimated by vertex count)
pub fn check_geometry(model: &BRepModel, solid: Handle<SolidData>) -> CheckResult {
    let mut issues = Vec::new();

    let sd = match model.solids.get(solid) {
        Some(s) => s,
        None => {
            issues.push("solid handle is invalid".into());
            return CheckResult {
                is_valid: false,
                issues,
            };
        }
    };

    // 1. Check solid has at least one shell
    if sd.shells.is_empty() {
        issues.push("solid has no shells".into());
    }

    for (si, &shell_h) in sd.shells.iter().enumerate() {
        let sh = match model.shells.get(shell_h) {
            Some(s) => s,
            None => {
                issues.push(format!("shell {si} handle is invalid"));
                continue;
            }
        };

        // 2. Check each shell has at least one face
        if sh.faces.is_empty() {
            issues.push(format!("shell {si} has no faces"));
        }

        for (fi, &face_h) in sh.faces.iter().enumerate() {
            let fd = match model.faces.get(face_h) {
                Some(f) => f,
                None => {
                    issues.push(format!("shell {si}, face {fi} handle is invalid"));
                    continue;
                }
            };

            // 3. Check each face has a loop
            let ld = match model.loops.get(fd.outer_loop) {
                Some(l) => l,
                None => {
                    issues.push(format!("shell {si}, face {fi} loop handle is invalid"));
                    continue;
                }
            };

            // 4. Check each loop has at least 3 half-edges
            let hes = model.loop_half_edges(ld.half_edge);
            if hes.len() < 3 {
                issues.push(format!(
                    "shell {si}, face {fi} loop has only {} half-edges (minimum 3)",
                    hes.len()
                ));
            }

            // 5. Check vertex coordinates are finite
            for &he_h in &hes {
                if let Some(he) = model.half_edges.get(he_h) {
                    if let Some(vd) = model.vertices.get(he.origin) {
                        let p = vd.point;
                        if !p.x.is_finite() || !p.y.is_finite() || !p.z.is_finite() {
                            issues.push(format!(
                                "shell {si}, face {fi} has non-finite vertex ({}, {}, {})",
                                p.x, p.y, p.z
                            ));
                        }
                    }
                }
            }
        }
    }

    CheckResult {
        is_valid: issues.is_empty(),
        issues,
    }
}

/// Check if a solid is watertight (all edges shared by exactly 2 faces).
///
/// Counts the number of faces referencing each edge. A manifold solid should
/// have every edge shared by exactly two faces (one forward, one backward
/// half-edge).
pub fn check_watertight(model: &BRepModel, solid: Handle<SolidData>) -> bool {
    use std::collections::HashMap;

    let sd = match model.solids.get(solid) {
        Some(s) => s,
        None => return false,
    };

    // Count how many half-edges reference each edge
    let mut edge_face_count: HashMap<u32, u32> = HashMap::new();

    for &shell_h in &sd.shells {
        let sh = match model.shells.get(shell_h) {
            Some(s) => s,
            None => return false,
        };

        for &face_h in &sh.faces {
            let fd = match model.faces.get(face_h) {
                Some(f) => f,
                None => return false,
            };
            let ld = match model.loops.get(fd.outer_loop) {
                Some(l) => l,
                None => return false,
            };
            let hes = model.loop_half_edges(ld.half_edge);
            for &he_h in &hes {
                if let Some(he) = model.half_edges.get(he_h) {
                    if let Some(edge) = he.edge {
                        *edge_face_count.entry(edge.index()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    // Every edge must be referenced exactly 2 times for a closed manifold
    !edge_face_count.is_empty() && edge_face_count.values().all(|&count| count == 2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;

    #[test]
    fn test_valid_box_passes_check() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let result = check_geometry(&model, r.solid);
        assert!(result.is_valid, "box should be valid: {:?}", result.issues);
    }

    #[test]
    fn test_invalid_handle_fails_check() {
        let model = BRepModel::new();
        let fake_handle = Handle::<SolidData>::from_raw_parts(9999, 0);
        let result = check_geometry(&model, fake_handle);
        assert!(!result.is_valid);
        assert!(!result.issues.is_empty());
    }

    #[test]
    fn test_watertight_box() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        assert!(check_watertight(&model, r.solid));
    }
}
