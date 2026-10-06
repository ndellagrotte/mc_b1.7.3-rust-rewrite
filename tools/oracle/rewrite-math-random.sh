#!/usr/bin/env sh
# Generates build/oracle-src: a copy of the vanilla client source with the one
# scripted rewrite allowed by PLAN_v4 §4.2 applied:
#
#     Math.random()  ->  harness.HarnessMath.random()
#
# and nothing else. HarnessMath draws from a single seeded java.util.Random, so
# every vanilla Math.random() call site becomes deterministic.
#
# Census of the vanilla tree (src/minecraft), taken for WP-0.2:
#     grep -rn 'Math.random()'  -> 49 lines
#     grep -rno 'Math.random()' -> 58 occurrences
# Every occurrence is a bare `Math.random()` (preceded by a space, `(` or a
# cast's `)`), never `StrictMath.random()` or `java.lang.Math.random()`.
# If the count printed below changes, re-review the call sites.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
V="$ROOT/.reference/mc_b1.7.3_release/1.7.3-LTS"
SRC="$V/src/minecraft"
OUT="$ROOT/build/oracle-src"

[ -d "$SRC" ] || { echo "rewrite-math-random: missing $SRC" >&2; exit 1; }

rm -rf "$OUT"
mkdir -p "$OUT"
cp -R "$SRC/." "$OUT/"

count=$(grep -rho 'Math\.random()' "$OUT" --include='*.java' | wc -l | tr -d ' ')
if [ "$count" -eq 0 ]; then
    echo "rewrite-math-random: no Math.random() occurrences found" >&2
    exit 1
fi

grep -rl 'Math\.random()' "$OUT" --include='*.java' | while IFS= read -r f; do
    sed 's/Math\.random()/harness.HarnessMath.random()/g' "$f" > "$f.tmp"
    mv "$f.tmp" "$f"
done

# Every remaining `Math.random()` must now be the tail of a HarnessMath call.
total=$(grep -rho 'Math\.random()' "$OUT" --include='*.java' | wc -l | tr -d ' ')
done_count=$(grep -rho 'harness\.HarnessMath\.random()' "$OUT" --include='*.java' | wc -l | tr -d ' ')
if [ "$done_count" -ne "$count" ] || [ "$total" -ne "$count" ]; then
    echo "rewrite-math-random: expected $count rewrites, found $done_count of $total" >&2
    exit 1
fi
echo "rewrite-math-random: replaced $count Math.random() occurrences"
