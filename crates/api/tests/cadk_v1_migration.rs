//! Wave 1 sub-phase 5a - `.cadk` V1 -> V2 placeholder migration guard.
//!
//! Locks the contract that the V2 variant added in Wave 1 does not
//! disturb V1 byte semantics. The placeholder V1 -> V2 migrator is a
//! pure no-op until Wave 3 fills it with real Body persistence migration,
//! and this test pins that behaviour from the public surface.

use cadkernel_api::cadk::{self, SchemaVersion};
use cadkernel_api::{Command, SolidId};

fn r1_log() -> Vec<Command> {
    vec![
        Command::CreateBox {
            dx: 50.0,
            dy: 30.0,
            dz: 10.0,
        },
        Command::CreateCylinder {
            radius: 3.0,
            height: 12.0,
        },
        Command::BooleanSubtract {
            lhs: SolidId(0),
            rhs: SolidId(1),
        },
    ]
}

#[test]
fn v1_bytes_round_trip_through_migrate_byte_identically() {
    let bytes = cadk::encode(&r1_log()).expect("encode");
    let migrated = cadk::migrate_to_current(&bytes).expect("migrate");
    assert_eq!(
        migrated, bytes,
        "V1 migration must be byte-identical in the Wave 1 no-op path"
    );
}

#[test]
fn schema_version_v2_is_known() {
    assert_eq!(SchemaVersion::from_u32(2), Some(SchemaVersion::V2));
    assert_eq!(SchemaVersion::V2.as_u32(), 2);
}

#[test]
fn current_schema_version_stays_v1_in_wave1() {
    assert_eq!(SchemaVersion::current(), SchemaVersion::V1);
}
