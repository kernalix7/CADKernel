#!/usr/bin/env bash
# bench_threshold.sh — fail if a Criterion benchmark's mean exceeds a budget.
#
# Usage:
#   bench_threshold.sh <bench_id> [budget_ns]
#
# <bench_id>   : the benchmark name as it appears under target/criterion/
# <budget_ns>  : nanosecond ceiling (integer). Optional for known gates.
#
# Exits 0 when mean ≤ budget_ns, non-zero otherwise.
# Requires: python3 (for JSON parsing; available on all CI runners)
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
    echo "Usage: $0 <bench_id> [budget_ns]" >&2
    exit 2
fi

BENCH_ID="$1"
if [[ $# -eq 2 ]]; then
    BUDGET_NS="$2"
else
    case "$BENCH_ID" in
        r3_repaint_1080p) BUDGET_NS=33300000 ;;
        *)
            echo "ERROR: no built-in budget for bench '${BENCH_ID}'." >&2
            echo "Usage: $0 <bench_id> [budget_ns]" >&2
            exit 2
            ;;
    esac
fi

ESTIMATES_FILE="target/criterion/${BENCH_ID}/new/estimates.json"

if [[ ! -f "$ESTIMATES_FILE" ]]; then
    echo "ERROR: estimates file not found: $ESTIMATES_FILE" >&2
    echo "  Run 'cargo bench' first so Criterion writes results to target/criterion/." >&2
    exit 2
fi

MEAN_NS=$(python3 -c "
import json, sys
with open('${ESTIMATES_FILE}') as f:
    d = json.load(f)
mean = d['mean']['point_estimate']
print(int(mean))
")

echo "bench '${BENCH_ID}': mean=${MEAN_NS} ns, budget=${BUDGET_NS} ns"

if python3 -c "import sys; sys.exit(0 if int('${MEAN_NS}') <= int('${BUDGET_NS}') else 1)"; then
    echo "PASS: mean is within budget."
    exit 0
else
    OVER=$(python3 -c "print(int('${MEAN_NS}') - int('${BUDGET_NS}'))")
    echo "FAIL: mean exceeds budget by ${OVER} ns." >&2
    exit 1
fi
