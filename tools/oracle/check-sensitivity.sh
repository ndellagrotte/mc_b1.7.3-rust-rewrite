#!/usr/bin/env sh
# Harness acceptance, PLAN_v4 §4.4 item 2: perturbing one RNG seed bit must
# change every seeded fixture. Runs the oracle with -Dharness.perturbSeed=1
# (every scenario seed XOR 1) and compares against the committed fixtures in
# tests/golden: each scenario with a "seed" must differ, each scenario without
# one (sine-table) must be identical. Needs a built oracle.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
. "$ROOT/tools/oracle/env.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/oracle-sens.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM

oracle_run "$TMP" -Dharness.perturbSeed=1 > /dev/null

cd "$ROOT/tests/golden"
status=0
for scenario in */*/scenario.json; do
    dir=$(dirname "$scenario")
    committed="$dir/expected.jsonl"
    [ -f "$committed" ] || { echo "check-sensitivity: $committed missing; run regen.sh" >&2; exit 1; }
    if grep -q '"seed"' "$scenario"; then
        if cmp -s "$committed" "$TMP/$dir/expected.jsonl"; then
            echo "check-sensitivity: FAIL $dir unchanged under a flipped seed bit" >&2
            status=1
        else
            echo "check-sensitivity: ok   $dir changes"
        fi
    else
        if cmp -s "$committed" "$TMP/$dir/expected.jsonl"; then
            echo "check-sensitivity: ok   $dir unseeded, identical"
        else
            echo "check-sensitivity: FAIL $dir has no seed but changed" >&2
            status=1
        fi
    fi
done
exit "$status"
