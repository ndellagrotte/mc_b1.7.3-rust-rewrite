#!/usr/bin/env sh
# Harness acceptance, PLAN_v4 §4.4 item 1: Tier A regenerates byte-identically
# across three runs. Needs a built oracle (tools/oracle/build.sh).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
. "$ROOT/tools/oracle/env.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/oracle-repro.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM

for run in 1 2 3; do
    oracle_run "$TMP/run$run" > /dev/null
done

status=0
for run in 2 3; do
    if ! diff -r "$TMP/run1" "$TMP/run$run" > /dev/null; then
        echo "check-reproducible: run $run differs from run 1:" >&2
        diff -rq "$TMP/run1" "$TMP/run$run" >&2 || true
        status=1
    fi
done
[ "$status" -eq 0 ] && echo "check-reproducible: 3 runs byte-identical ($(find "$TMP/run1" -type f | wc -l | tr -d ' ') files)"
exit "$status"
