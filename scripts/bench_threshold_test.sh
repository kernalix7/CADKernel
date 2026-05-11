#!/usr/bin/env bash
# bench_threshold_test.sh — self-test for bench_threshold.sh
# Exercises pass, fail, and error paths using synthetic estimates.json fixtures.
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
THRESHOLD="$SCRIPT_DIR/bench_threshold.sh"

WORK="$(mktemp -d)"
# Save and restore the working directory around cd calls.
ORIG_DIR="$(pwd)"
cleanup() { cd "$ORIG_DIR"; rm -rf "$WORK"; }
trap cleanup EXIT

make_fixture() {
    local bench_id="$1"
    local mean_ns="$2"
    local dir="$WORK/target/criterion/${bench_id}/new"
    mkdir -p "$dir"
    python3 - <<PYEOF > "$dir/estimates.json"
import json
v = float($mean_ns)
d = {
    "mean":          {"point_estimate": v, "standard_error": 0.0, "confidence_interval": {"lower_bound": v, "upper_bound": v}},
    "median":        {"point_estimate": v, "standard_error": 0.0, "confidence_interval": {"lower_bound": v, "upper_bound": v}},
    "median_abs_dev":{"point_estimate": 0.0,"standard_error": 0.0,"confidence_interval": {"lower_bound": 0.0,"upper_bound": 0.0}},
    "slope":         {"point_estimate": v, "standard_error": 0.0, "confidence_interval": {"lower_bound": v, "upper_bound": v}},
    "std_dev":       {"point_estimate": 0.0,"standard_error": 0.0,"confidence_interval": {"lower_bound": 0.0,"upper_bound": 0.0}},
}
print(json.dumps(d))
PYEOF
}

PASS=0
FAIL=0

check() {
    local desc="$1"
    local expected="$2"
    shift 2
    local actual=0
    bash "$@" > /dev/null 2>&1 || actual=$?
    if [[ "$actual" -eq "$expected" ]]; then
        echo "  PASS: $desc"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $desc  (expected exit $expected, got $actual)"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== bench_threshold.sh self-test ==="

make_fixture "r1_exact" 200000000
make_fixture "r1_fast"  100000000
make_fixture "r1_slow"  300000000

cd "$WORK"

# Test 1: mean exactly at budget — pass
check "mean == budget → exit 0" 0 "$THRESHOLD" r1_exact 200000000

# Test 2: mean below budget — pass
check "mean < budget → exit 0"  0 "$THRESHOLD" r1_fast  250000000

# Test 3: mean above budget — fail
check "mean > budget → exit 1"  1 "$THRESHOLD" r1_slow  250000000

# Test 4: missing estimates file — usage error
check "missing bench → exit 2"  2 "$THRESHOLD" no_such_bench 250000000

# Test 5: wrong argument count — usage error (run from WORK to avoid cwd issues)
check "no args → exit 2"        2 "$THRESHOLD"

echo ""
echo "Results: ${PASS} passed, ${FAIL} failed"
[[ $FAIL -eq 0 ]]
