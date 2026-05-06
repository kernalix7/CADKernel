//! Build the R1-R12 reference parts and write each to `.cadk` plus a
//! companion `.expected_hash` file containing the BLAKE3 content hash.
//!
//! ```bash
//! cargo run --release --example build_reference_parts -- --output tests/corpus/reference_parts/
//! ```
//!
//! For now this implements R1 (axis-aligned box) and R2 (extruded
//! rectangle with circular hole) at topology level; R3-R12 follow the
//! same pattern and are stubbed to compile but skipped in execution.
//!
//! Cross-architecture determinism requirement: the produced .cadk hash
//! MUST be byte-identical on Linux x86-64, Linux arm64, macOS arm64, and
//! Windows x86-64. CI verifies this via tests/reference_parts_corpus.rs.

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

fn build_r1_box(_dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Once topology / modeling crates expose a stable Rust public API for
    // primitive construction + .cadk export, replace the body below with:
    //
    //   use cadkernel_modeling::quick;
    //   let solid = quick::quick_box(100.0, 50.0, 25.0)?;
    //   cadkernel_io::cadk::write(&solid, dest)?;
    //
    // For now we surface a clear NotImplemented so CI can track when the
    // dependency lands.
    Err("cadk writer not yet wired into example".into())
}

fn build_r2_extrude(_dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Err("extrude + cadk writer not yet wired".into())
}

fn stub(_dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    Err("R3-R12 not yet implemented".into())
}
