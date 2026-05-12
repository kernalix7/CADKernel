//! `cadk-inspect` — inspect a `.cadk` container without modifying it.
//!
//! A3 deliverable: a small read-only diagnostic CLI that prints the
//! header, manifest, and per-blob CRC verification status. Useful for
//! debugging file format issues and as a sanity check after autosave.
//!
//! Usage:
//!
//! ```text
//! cadk-inspect <path-to-file.cadk> [--verbose] [--quick]
//! ```
//!
//! Flags:
//!   --verbose / -v   also print the decoded command list
//!   --quick / -q     skip document/thumbnail CRC checks (header + manifest only)
//!
//! Exit codes:
//!   0  file looks healthy (magic + schema + all checked CRCs OK)
//!   1  I/O or argument error
//!   2  container is malformed or fails any integrity check
//!
//! A3.0.5 (2026-05-13) rebuilt this binary on top of the new
//! [`cadk::inspect_path`] / [`cadk::CadkSummary`] surface. The default
//! contract is unchanged (document blob CRC still verified via `decode`);
//! `--quick` is the new opt-in fast path that uses inspect only.

use std::path::Path;
use std::process::ExitCode;

use cadkernel_api::cadk::{self, CadkFlags, CadkSummary};

fn main() -> ExitCode {
    let mut path: Option<String> = None;
    let mut verbose = false;
    let mut quick = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--verbose" | "-v" => verbose = true,
            "--quick" | "-q" => quick = true,
            "--help" | "-h" => {
                println!("usage: cadk-inspect <path-to-file.cadk> [--verbose] [--quick]");
                return ExitCode::from(0);
            }
            other if other.starts_with('-') => {
                eprintln!("error: unknown flag {other}");
                return ExitCode::from(1);
            }
            other => {
                if path.is_some() {
                    eprintln!("error: multiple paths supplied");
                    return ExitCode::from(1);
                }
                path = Some(other.to_string());
            }
        }
    }
    let Some(path) = path else {
        eprintln!("usage: cadk-inspect <path-to-file.cadk> [--verbose] [--quick]");
        return ExitCode::from(1);
    };

    match inspect(Path::new(&path), verbose, quick) {
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

fn print_flag_breakdown(flags: u32) {
    if flags & CadkFlags::MANIFEST_COMPRESSED != 0 {
        println!("                  + MANIFEST_COMPRESSED");
    }
    if flags & CadkFlags::SIGNED != 0 {
        println!("                  + SIGNED");
    }
    if flags & CadkFlags::HAS_THUMBNAIL != 0 {
        println!("                  + HAS_THUMBNAIL");
    }
    if flags & CadkFlags::DOCUMENT_COMPRESSED != 0 {
        println!("                  + DOCUMENT_COMPRESSED");
    }
    let unknown = flags & !CadkFlags::KNOWN;
    if unknown != 0 {
        println!("                  + unknown bits 0x{unknown:08x}");
    }
}

fn print_summary(path: &Path, file_size: usize, summary: &CadkSummary) {
    println!("file:           {}", path.display());
    println!("size:           {file_size} bytes");
    println!("magic:          \"CADK\"  (OK)");
    println!("schema:         {}", summary.schema_version);
    println!("flags:          0x{:08x}", summary.flags);
    print_flag_breakdown(summary.flags);
    println!("blobs:          {}", summary.blob_count);
    print!("document:       {} bytes", summary.document_length);
    if summary.document_compressed() {
        println!(" (zstd)");
    } else {
        println!();
    }
    match summary.thumbnail_length {
        Some(len) => println!("thumbnail:      present, {len} bytes  (per manifest)"),
        None => println!("thumbnail:      absent"),
    }
}

fn inspect(path: &Path, verbose: bool, quick: bool) -> Result<(), InspectError> {
    // Read the file once; reuse the bytes for both inspect (cheap) and
    // — unless --quick — decode (full CRC check).
    let bytes = std::fs::read(path)
        .map_err(|e| InspectError::Io(format!("read {}: {e}", path.display())))?;

    let summary = cadk::inspect(&bytes)
        .map_err(|e| InspectError::Format(format!("inspect failed: {e}")))?;
    print_summary(path, bytes.len(), &summary);

    if quick {
        println!("status:         OK (header + manifest verified, --quick)");
        if verbose {
            // --verbose needs decoded commands; --quick suppresses decode.
            // Surface the conflict explicitly rather than silently dropping
            // the command listing.
            eprintln!("note: --verbose is a no-op when combined with --quick");
        }
        return Ok(());
    }

    // Full path: verify the document blob CRC by decoding it. This also
    // exercises the zstd path when DOCUMENT_COMPRESSED is set.
    let commands = cadk::decode(&bytes)
        .map_err(|e| InspectError::Format(format!("decode failed: {e}")))?;

    // Thumbnail CRC check is independent of the document blob.
    let thumb = cadk::decode_thumbnail(&bytes)
        .map_err(|e| InspectError::Format(format!("thumbnail check failed: {e}")))?;
    if let Some(thumb) = &thumb {
        println!("thumbnail crc:  OK ({} bytes recovered)", thumb.len());
    }

    println!("commands:       {}", commands.len());

    if verbose {
        println!();
        println!("--- blobs ---");
        for (i, blob) in summary.blobs.iter().enumerate() {
            println!(
                "[{i:>4}] {:?}  name={:?}  length={} bytes",
                blob.kind, blob.name, blob.length
            );
        }

        println!();
        println!("--- commands ---");
        for (i, cmd) in commands.iter().enumerate() {
            println!("[{i:>4}] {cmd:?}");
        }
    }

    println!("status:         OK (all CRCs verified)");
    Ok(())
}
