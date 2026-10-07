#!/usr/bin/env sh
# Harness acceptance, PLAN_v4 §4.4 item 1: Tier A regenerates byte-identically
# across three runs. Needs a built oracle (tools/oracle/build.sh).
# SOAK (env.sh) selects the scenarios as for regen.sh; SOAK=only compares two
# runs of the soak set (tests/golden/soak, about 2.5 min each) instead of three.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
. "$ROOT/tools/oracle/env.sh"
TMP=$(mktemp -d "${TMPDIR:-/tmp}/oracle-repro.XXXXXX")
trap 'rm -rf "$TMP"' EXIT INT TERM

RUNS="1 2 3"
[ "${SOAK:-0}" = only ] && RUNS="1 2"
for run in $RUNS; do
    oracle_run "$TMP/run$run" > /dev/null
done

status=0
for run in $RUNS; do
    [ "$run" = 1 ] && continue
    if ! diff -r "$TMP/run1" "$TMP/run$run" > /dev/null; then
        echo "check-reproducible: run $run differs from run 1:" >&2
        diff -rq "$TMP/run1" "$TMP/run$run" >&2 || true
        status=1
    fi
done
[ "$status" -eq 0 ] && echo "check-reproducible: $(echo $RUNS | wc -w | tr -d ' ') runs byte-identical ($(find "$TMP/run1" -type f | wc -l | tr -d ' ') files)"
exit "$status"
