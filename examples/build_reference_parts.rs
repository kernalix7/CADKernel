//! Build the R1-R12 reference parts and write each to `.cadk` plus a
//! companion `.expected_hash` file containing the FNV-1a content hash.
//!
//! ```bash
//! cargo run --release --example build_reference_parts -- --output tests/corpus/reference_parts/
//! ```
//!
//! Implemented today (Commercial CAD Roadmap v0.5 Gates #9 / #10):
//!   - R1: axis-aligned box (CreateBox)
//!   - R2: extruded rectangle with one circular through-hole
//!     (CreateBox + CreateCylinder + BooleanSubtract)
//!
//! R3-R12 follow the same pattern and are stubbed to compile but skipped
//! in execution. They land as later phases close the corresponding
//! roadmap gates (sketch profiles, fillet/chamfer, assembly, etc.).
//!
//! Cross-architecture determinism requirement: the produced `.cadk` hash
//! MUST be byte-identical on Linux x86-64, Linux arm64, macOS arm64, and
//! Windows x86-64. The codec encodes the command log only (no
//! timestamps, no kernel-side floating-point dependence on tessellation
//! seeds), so byte-identity is structural. CI verifies this via
//! `tests/reference_parts_corpus.rs`.

use cadkernel_api::{Command, Session};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut output_dir = PathBuf::from("tests/corpus/reference_parts");
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" | "-o" => {
                if let Some(p) = args.next() {
                    output_dir = PathBuf::from(p);
                }
            }
            "--help" | "-h" => {
                eprintln!("usage: build_reference_parts [--output DIR]");
                return ExitCode::SUCCESS;
            }
            other => {
                eprintln!("unknown arg: {other}");
                return ExitCode::FAILURE;
            }
        }
    }

    if let Err(e) = fs::create_dir_all(&output_dir) {
        eprintln!("cannot create {}: {e}", output_dir.display());
        return ExitCode::FAILURE;
    }

    type PartBuilder = fn(&Path) -> Result<(), Box<dyn std::error::Error>>;
    let parts: Vec<(&str, PartBuilder)> = vec![
        ("R1", build_r1_box),
        ("R2", build_r2_extrude),
        // R3-R12 stubbed:
        ("R3", stub),
        ("R4", stub),
        ("R5", stub),
        ("R6", stub),
        ("R7", stub),
        ("R8", stub),
        ("R9", stub),
        ("R10", stub),
        ("R11", stub),
        ("R12", stub),
    ];

    let mut had_error = false;
    for (id, builder) in parts {
        let dest = output_dir.join(format!("{id}.cadk"));
        match builder(&dest) {
            Ok(()) => println!("{id}: built  -> {}", dest.display()),
            Err(e) => {
                eprintln!("{id}: SKIPPED ({e})");
                // Stubs are expected to fail until implemented; not a hard error.
                if id == "R1" || id == "R2" {
                    had_error = true;
                }
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn build_r1_box(dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // R1 — axis-aligned box, 100 × 50 × 25.
    // Roadmap reference: §3 "R1 (bracket)" simplified to a single primitive.
    let mut session = Session::new();
    session.execute(Command::CreateBox {
        dx: 100.0,
        dy: 50.0,
        dz: 25.0,
    })?;
    write_cadk_with_hash(&session, dest)
}

fn build_r2_extrude(dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // R2 — base box 60 × 40 × 10 with one Ø10 through-hole at the
    // centre (a flat rectangular plate with a single mounting hole).
    // Drilled via CreateCylinder + BooleanSubtract — the smallest
    // composite that exercises the boolean splitter end-to-end through
    // the API surface.
    let mut session = Session::new();
    let plate = match session.execute(Command::CreateBox {
        dx: 60.0,
        dy: 40.0,
        dz: 10.0,
    })? {
        cadkernel_api::Outcome::SolidCreated { id, .. } => id,
        other => return Err(format!("R2: expected SolidCreated, got {other:?}").into()),
    };
    let hole = match session.execute(Command::CreateCylinder {
        radius: 5.0,
        height: 10.0,
    })? {
        cadkernel_api::Outcome::SolidCreated { id, .. } => id,
        other => return Err(format!("R2: expected SolidCreated, got {other:?}").into()),
    };
    session.execute(Command::BooleanSubtract {
        lhs: plate,
        rhs: hole,
    })?;
    write_cadk_with_hash(&session, dest)
}

fn stub(_dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Err("R3-R12 not yet implemented".into())
}

/// Encode the session's applied command log via the deterministic
/// `.cadk` codec, write it to `dest`, and write a sibling
/// `<dest>.expected_hash` file with the FNV-1a-64 hex digest.
fn write_cadk_with_hash(
    session: &Session,
    dest: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = session.save_cadk()?;
    fs::write(dest, &bytes)?;
    let digest = fnv1a_64(&bytes);
    let hash_path = dest.with_extension(format!(
        "{}.expected_hash",
        dest.extension().and_then(|s| s.to_str()).unwrap_or("cadk")
    ));
    fs::write(&hash_path, format!("{digest:016x}\n"))?;
    Ok(())
}

/// FNV-1a 64-bit. Used here only as a small dependency-free determinism
/// check for the corpus example. CI also verifies byte-equality, so the
/// digest is a convenience marker, not a cryptographic guarantee.
pub fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}
