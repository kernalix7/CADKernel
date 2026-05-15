//! Reference part builders exposed for benchmarks and fuzzers.
//!
//! These mirror the R1-R12 fixtures produced by
//! `examples/build_reference_parts.rs` but return the encoded `.cadk`
//! bytes directly instead of writing to disk, so callers (Criterion
//! benches, integration tests, future fuzz harnesses) can avoid the
//! filesystem round-trip.
//!
//! R1-R3 are implemented as deterministic public-API command scripts. R4-R12
//! land alongside the corresponding parity gates.
//! The example binary delegates to these helpers for its file-output
//! path so the on-disk corpus and the in-memory bytes stay bit-identical.

use crate::{ApiResult, Command, Session, SolidId};

/// R1 — small L-bracket command script.
pub fn r1_commands() -> Vec<Command> {
    vec![
        Command::CreateBox {
            dx: 80.0,
            dy: 40.0,
            dz: 10.0,
        },
        Command::CreateBox {
            dx: 10.0,
            dy: 40.0,
            dz: 40.0,
        },
        Command::Translate {
            id: SolidId(1),
            dx: 0.0,
            dy: 0.0,
            dz: 8.0,
        },
        Command::BooleanUnion {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 14.0,
        },
        Command::Translate {
            id: SolidId(3),
            dx: 20.0,
            dy: 10.0,
            dz: -2.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(2),
            rhs: SolidId(3),
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 14.0,
        },
        Command::Translate {
            id: SolidId(5),
            dx: 60.0,
            dy: 10.0,
            dz: -2.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(4),
            rhs: SolidId(5),
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 14.0,
        },
        Command::Translate {
            id: SolidId(7),
            dx: 20.0,
            dy: 30.0,
            dz: -2.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(6),
            rhs: SolidId(7),
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 14.0,
        },
        Command::Translate {
            id: SolidId(9),
            dx: 60.0,
            dy: 30.0,
            dz: -2.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(8),
            rhs: SolidId(9),
        },
    ]
}

/// R2 — compact housing script with a bored cylindrical shell.
pub fn r2_commands() -> Vec<Command> {
    vec![
        Command::CreateCylinder {
            radius: 24.0,
            height: 20.0,
        },
        Command::CreateCylinder {
            radius: 12.0,
            height: 24.0,
        },
        Command::Translate {
            id: SolidId(1),
            dx: 0.0,
            dy: 0.0,
            dz: -2.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
        Command::CreateCylinder {
            radius: 5.0,
            height: 8.0,
        },
        Command::Translate {
            id: SolidId(3),
            dx: 16.0,
            dy: 0.0,
            dz: 18.0,
        },
        Command::BooleanUnion {
            lhs: SolidId(2),
            rhs: SolidId(3),
        },
        Command::CreateCylinder {
            radius: 5.0,
            height: 8.0,
        },
        Command::Translate {
            id: SolidId(5),
            dx: -16.0,
            dy: 0.0,
            dz: 18.0,
        },
        Command::BooleanUnion {
            lhs: SolidId(4),
            rhs: SolidId(5),
        },
    ]
}

/// R3 — gearbox stub: gear-like torus plus central hub.
pub fn r3_commands() -> Vec<Command> {
    vec![
        Command::CreateTorus {
            major_radius: 18.0,
            minor_radius: 3.0,
        },
        Command::CreateCylinder {
            radius: 8.0,
            height: 6.0,
        },
        Command::Translate {
            id: SolidId(1),
            dx: 0.0,
            dy: 0.0,
            dz: -3.0,
        },
        Command::BooleanUnion {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
    ]
}

fn session_from_commands(commands: &[Command]) -> ApiResult<Session> {
    let mut session = Session::new();
    for command in commands {
        session.execute(command.clone())?;
    }
    Ok(session)
}

/// Build the R1 reference part and return its `.cadk`-encoded bytes.
pub fn r1_bytes() -> ApiResult<Vec<u8>> {
    session_from_commands(&r1_commands())?.save_cadk()
}

/// Build the R2 reference part and return its `.cadk`-encoded bytes.
pub fn r2_bytes() -> ApiResult<Vec<u8>> {
    session_from_commands(&r2_commands())?.save_cadk()
}

/// Build the R3 reference part and return its `.cadk`-encoded bytes.
pub fn r3_bytes() -> ApiResult<Vec<u8>> {
    session_from_commands(&r3_commands())?.save_cadk()
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
        assert_eq!(session.log().len(), r1_commands().len());
    }

    #[test]
    fn r2_bytes_round_trips_to_one_solid() {
        let bytes = r2_bytes().expect("build R2");
        assert!(!bytes.is_empty(), "r2 bytes must be non-empty");
        let session = Session::load_cadk(&bytes).expect("load R2");
        assert_eq!(session.document().solid_count(), 1);
        assert_eq!(session.log().len(), r2_commands().len());
    }

    #[test]
    fn r3_bytes_round_trips_to_one_solid() {
        let bytes = r3_bytes().expect("build R3");
        assert!(!bytes.is_empty(), "r3 bytes must be non-empty");
        let session = Session::load_cadk(&bytes).expect("load R3");
        assert_eq!(session.document().solid_count(), 1);
        assert_eq!(session.log().len(), r3_commands().len());
    }

    #[test]
    fn r1_bytes_are_deterministic() {
        let a = r1_bytes().expect("build R1 first");
        let b = r1_bytes().expect("build R1 second");
        assert_eq!(a, b, "r1_bytes must be deterministic across calls");
    }
}
