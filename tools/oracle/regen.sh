#!/usr/bin/env sh
# Regenerates the golden fixtures from the Java oracle (PLAN_v4 §4.1).
#   1. build.sh: Math.random() rewrite + javac into build/oracle
#   2. harness.Main: tests/golden/*/*/scenario.json -> build/golden-out
#   3. copy expected.jsonl and sidecars into tests/golden (generated files in
#      each scenario directory are replaced; scenario.json is never touched)
#   4. print `git status --short tests/golden` so drift is visible
# Nobody edits expected.jsonl by hand; run this instead.
#
# SOAK (env.sh): unset or 0 regenerates every fixture except the long-run soak
# set tests/golden/soak/* (the default); SOAK=only regenerates the
# soak set alone (about 2 min of oracle time); SOAK=1 both. Fixtures outside
# the selected set are not touched.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
. "$ROOT/tools/oracle/env.sh"
OUT="$ROOT/build/golden-out"

sh "$ROOT/tools/oracle/build.sh"

rm -rf "$OUT"
oracle_run "$OUT"

IDS=$(golden_ids)
cd "$ROOT/tests/golden"
for dir in $IDS; do
    [ -f "$OUT/$dir/expected.jsonl" ] || { echo "regen: no output for $dir" >&2; exit 1; }
    find "$dir" -maxdepth 1 -type f ! -name scenario.json -exec rm -f {} +
    cp "$OUT/$dir"/* "$dir"/
done

echo "regen: fixtures copied into tests/golden"
git -C "$ROOT" status --short tests/golden
