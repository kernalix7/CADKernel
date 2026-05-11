//! Reference part builders exposed for benchmarks and fuzzers.
//!
//! These mirror the R1-R12 fixtures produced by
//! `examples/build_reference_parts.rs` but return the encoded `.cadk`
//! bytes directly instead of writing to disk, so callers (Criterion
//! benches, integration tests, future fuzz harnesses) can avoid the
//! filesystem round-trip.
//!
//! Today only R1 (axis-aligned box) and R2 (plate with through-hole) are
//! implemented. R3-R12 land alongside the corresponding parity gates.
//! The example binary delegates to these helpers for its file-output
//! path so the on-disk corpus and the in-memory bytes stay bit-identical.

use crate::{ApiResult, Command, Outcome, Session};

/// Build the R1 reference part (axis-aligned 100 × 50 × 25 box) and
/// return its `.cadk`-encoded bytes.
pub fn r1_bytes() -> ApiResult<Vec<u8>> {
    let mut session = Session::new();
    session.execute(Command::CreateBox {
        dx: 100.0,
        dy: 50.0,
        dz: 25.0,
    })?;
    session.save_cadk()
}

/// Build the R2 reference part (60 × 40 × 10 plate minus a Ø10
/// through-hole) and return its `.cadk`-encoded bytes.
pub fn r2_bytes() -> ApiResult<Vec<u8>> {
    let mut session = Session::new();
    let plate = match session.execute(Command::CreateBox {
        dx: 60.0,
        dy: 40.0,
        dz: 10.0,
    })? {
        Outcome::SolidCreated { id, .. } => id,
        other => {
            return Err(crate::ApiError::Kernel(format!(
                "R2: expected SolidCreated for plate, got {other:?}"
            )));
        }
    };
    let hole = match session.execute(Command::CreateCylinder {
        radius: 5.0,
        height: 10.0,
    })? {
        Outcome::SolidCreated { id, .. } => id,
        other => {
            return Err(crate::ApiError::Kernel(format!(
                "R2: expected SolidCreated for hole, got {other:?}"
            )));
        }
    };
    session.execute(Command::BooleanSubtract {
        lhs: plate,
        rhs: hole,
    })?;
    session.save_cadk()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r1_bytes_round_trips_to_one_solid() {
        let bytes = r1_bytes().expect("build R1");
        assert!(!bytes.is_empty(), "r1 bytes must be non-empty");
        let session = Session::load_cadk(&bytes).expect("load R1");
        assert_eq!(session.document().solid_count(), 1);
    }

    #[test]
    fn r2_bytes_round_trips_to_one_solid() {
        let bytes = r2_bytes().expect("build R2");
        assert!(!bytes.is_empty(), "r2 bytes must be non-empty");
        let session = Session::load_cadk(&bytes).expect("load R2");
        // R2 is plate − hole, so one boolean-result solid remains.
        assert_eq!(session.document().solid_count(), 1);
    }

    #[test]
    fn r1_bytes_are_deterministic() {
        let a = r1_bytes().expect("build R1 first");
        let b = r1_bytes().expect("build R1 second");
        assert_eq!(a, b, "r1_bytes must be deterministic across calls");
    }
}
