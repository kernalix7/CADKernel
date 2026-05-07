//! `cadk-inspect` — inspect a `.cadk` container without modifying it.
//!
//! A3 deliverable: a small read-only diagnostic CLI that prints the
//! header, manifest, and per-blob CRC verification status. Useful for
//! debugging file format issues and as a sanity check after autosave.
//!
//! Usage:
//!
//! ```text
//! cadk-inspect <path-to-file.cadk> [--verbose]
//! ```
//!
//! Exit codes:
//!   0  file looks healthy (magic + schema + all CRCs OK)
//!   1  I/O or argument error
//!   2  container is malformed or fails any integrity check

use std::path::Path;
use std::process::ExitCode;

use cadkernel_api::cadk::{self, CadkFlags, MAGIC};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: cadk-inspect <path-to-file.cadk> [--verbose]");
        return ExitCode::from(1);
    };
    let verbose = args.any(|a| a == "--verbose" || a == "-v");

    match inspect(Path::new(&path), verbose) {
        Ok(()) => ExitCode::from(0),
        Err(InspectError::Io(e)) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
        Err(InspectError::Format(msg)) => {
            eprintln!("error: {msg}");
            ExitCode::from(2)
        }
    }
}

enum InspectError {
    Io(String),
    Format(String),
}

fn inspect(path: &Path, verbose: bool) -> Result<(), InspectError> {
    let bytes = std::fs::read(path).map_err(|e| InspectError::Io(format!("read {}: {e}", path.display())))?;
    println!("file:           {}", path.display());
    println!("size:           {} bytes", bytes.len());

    if bytes.len() < MAGIC.len() {
        return Err(InspectError::Format("file is too small to contain magic".into()));
    }
    let magic = &bytes[..MAGIC.len()];
    let magic_match = magic == MAGIC;
    println!(
        "magic:          {:?}  ({})",
        std::str::from_utf8(magic).unwrap_or("<non-utf8>"),
        if magic_match { "OK" } else { "MISMATCH" }
    );
    if !magic_match {
        return Err(InspectError::Format("magic mismatch".into()));
    }

    // Decode the document blob (validates header + manifest CRC + blob CRC).
    let commands = match cadk::decode(&bytes) {
        Ok(c) => c,
        Err(e) => return Err(InspectError::Format(format!("decode failed: {e}"))),
    };
    println!("commands:       {}", commands.len());

    // Decode thumbnail (if present); reports flag + CRC status without
    // dumping the raw bytes.
    match cadk::decode_thumbnail(&bytes) {
        Ok(Some(thumb)) => println!("thumbnail:      present, {} bytes  (CRC OK)", thumb.len()),
        Ok(None) => println!("thumbnail:      absent"),
        Err(e) => return Err(InspectError::Format(format!("thumbnail check failed: {e}"))),
    }

    if verbose {
        // Reach into the header for the flag breakdown. We re-parse the
        // header bytes here using the public `decode` round-trip having
        // already validated everything; this keeps the binary lean
        // without exposing extra public API.
        let header_bytes = &bytes[MAGIC.len()..MAGIC.len() + 64];
        let flags = u32::from_le_bytes(header_bytes[4..8].try_into().unwrap());
        let schema_version = u32::from_le_bytes(header_bytes[0..4].try_into().unwrap());
        println!();
        println!("--- header ---");
        println!("schema_version: {schema_version}");
        println!("flags:          0x{flags:08x}");
        if flags & CadkFlags::MANIFEST_COMPRESSED != 0 {
            println!("                  + MANIFEST_COMPRESSED");
        }
        if flags & CadkFlags::SIGNED != 0 {
            println!("                  + SIGNED");
        }
        if flags & CadkFlags::HAS_THUMBNAIL != 0 {
            println!("                  + HAS_THUMBNAIL");
        }
        let unknown = flags & !CadkFlags::KNOWN;
        if unknown != 0 {
            println!("                  + unknown bits 0x{unknown:08x}");
        }

        println!();
        println!("--- commands ---");
        for (i, cmd) in commands.iter().enumerate() {
            println!("[{i:>4}] {cmd:?}");
        }
    }

    println!("status:         OK");
    Ok(())
}
