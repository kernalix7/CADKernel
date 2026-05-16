use std::panic::{AssertUnwindSafe, catch_unwind};

use cadkernel_math::Point3;
use cadkernel_modeling::{
    BooleanOp, boolean_op, check_geometry, make_box, make_cone, make_cylinder, make_sphere,
};
use cadkernel_topology::{BRepModel, Handle, SolidData, ValidationSeverity};
use rand::{Rng, SeedableRng, rngs::StdRng};

const CI_PAIR_COUNT: usize = 100;
const GATE_PAIR_COUNT: usize = 1_000_000;
const GATE_RUNS: usize = 2;
const SEED: u64 = 42;

#[test]
fn boolean_pair_seeded_smoke_100() {
    run_seeded_boolean_pairs(CI_PAIR_COUNT, SEED);
}

#[test]
#[ignore = "1M-pair gate test is reserved for manual release verification"]
fn boolean_pair_one_million_two_consecutive_runs() {
    let pair_count = std::env::var("CADKERNEL_BOOLEAN_FUZZ_PAIRS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(GATE_PAIR_COUNT);
    let runs = std::env::var("CADKERNEL_BOOLEAN_FUZZ_RUNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(GATE_RUNS);

    for run in 0..runs {
        run_seeded_boolean_pairs(pair_count, SEED + run as u64);
    }
}

fn run_seeded_boolean_pairs(pair_count: usize, seed: u64) {
    let mut rng = StdRng::seed_from_u64(seed);
    for pair_index in 0..pair_count {
        let result = catch_unwind(AssertUnwindSafe(|| run_one_pair(&mut rng)));
        match result {
            Ok(Ok(())) => {}
            Ok(Err(message)) => {
                panic!("boolean fuzz pair {pair_index} failed with seed {seed}: {message}")
            }
            Err(_) => panic!("boolean fuzz pair {pair_index} panicked with seed {seed}"),
        }
    }
}

fn run_one_pair(rng: &mut StdRng) -> Result<(), String> {
    let (model_a, solid_a, model_b, solid_b) = make_random_pair(rng)?;
    let op = random_boolean_op(rng);

    let result = boolean_op(&model_a, solid_a, &model_b, solid_b, op)
        .map_err(|err| format!("boolean {op:?} returned error: {err:?}"))?;

    validate_boolean_result(&result)
}

fn make_random_pair(
    rng: &mut StdRng,
) -> Result<(BRepModel, Handle<SolidData>, BRepModel, Handle<SolidData>), String> {
    let center_a = random_point(rng);
    let center_b = Point3::new(
        center_a.x + 4.5 + rng.random_range(0.0..=1.0),
        center_a.y,
        center_a.z,
    );
    let (model_a, solid_a) = make_random_primitive_at(rng, center_a)?;
    let (model_b, solid_b) = make_random_primitive_at(rng, center_b)?;
    Ok((model_a, solid_a, model_b, solid_b))
}

fn make_random_primitive_at(
    rng: &mut StdRng,
    center: Point3,
) -> Result<(BRepModel, Handle<SolidData>), String> {
    let mut model = BRepModel::new();
    let solid = match rng.random_range(0..4) {
        0 => return make_box_at(rng, center),
        1 => {
            let height = random_extent(rng);
            make_cylinder(
                &mut model,
                Point3::new(center.x, center.y, center.z - height * 0.5),
                random_radius(rng),
                height,
                random_segments(rng),
            )
            .map_err(|err| format!("make_cylinder failed: {err:?}"))?
            .solid
        }
        2 => {
            make_sphere(
                &mut model,
                center,
                random_radius(rng),
                random_segments(rng),
                rng.random_range(3..=6),
            )
            .map_err(|err| format!("make_sphere failed: {err:?}"))?
            .solid
        }
        _ => {
            let height = random_extent(rng);
            make_cone(
                &mut model,
                Point3::new(center.x, center.y, center.z - height * 0.5),
                random_radius(rng),
                if rng.random_bool(0.2) {
                    0.0
                } else {
                    random_radius(rng)
                },
                height,
                random_segments(rng),
            )
            .map_err(|err| format!("make_cone failed: {err:?}"))?
            .solid
        }
    };

    Ok((model, solid))
}

fn make_box_at(rng: &mut StdRng, center: Point3) -> Result<(BRepModel, Handle<SolidData>), String> {
    let mut model = BRepModel::new();
    let width = random_extent(rng);
    let height = random_extent(rng);
    let depth = random_extent(rng);
    let origin = Point3::new(
        center.x - width * 0.5,
        center.y - height * 0.5,
        center.z - depth * 0.5,
    );
    let solid = make_box(&mut model, origin, width, height, depth)
        .map_err(|err| format!("make_box failed: {err:?}"))?
        .solid;
    Ok((model, solid))
}

fn validate_boolean_result(model: &BRepModel) -> Result<(), String> {
    let errors: Vec<_> = model
        .validate_detailed()
        .into_iter()
        .filter(|issue| issue.severity == ValidationSeverity::Error)
        .collect();
    if !errors.is_empty() {
        return Err(format!("validate_detailed found errors: {errors:?}"));
    }

    for (solid, _) in model.solids.iter() {
        let check = check_geometry(model, solid);
        if !check.is_valid {
            return Err(format!("check_geometry failed: {:?}", check.issues));
        }
    }

    Ok(())
}

fn random_boolean_op(rng: &mut StdRng) -> BooleanOp {
    match rng.random_range(0..3) {
        0 => BooleanOp::Union,
        1 => BooleanOp::Difference,
        _ => BooleanOp::Intersection,
    }
}

fn random_point(rng: &mut StdRng) -> Point3 {
    Point3::new(
        rng.random_range(-1.5..=1.5),
        rng.random_range(-1.5..=1.5),
        rng.random_range(-1.5..=1.5),
    )
}

fn random_extent(rng: &mut StdRng) -> f64 {
    rng.random_range(0.25..=2.0)
}

fn random_radius(rng: &mut StdRng) -> f64 {
    rng.random_range(0.15..=1.0)
}

fn random_segments(rng: &mut StdRng) -> usize {
    rng.random_range(6..=12)
}
