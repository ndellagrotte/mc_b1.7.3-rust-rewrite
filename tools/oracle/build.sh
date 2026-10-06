#!/usr/bin/env sh
# Builds the Java oracle (PLAN_v4 §4.1): the vanilla client source with the
# Math.random() rewrite applied (rewrite-math-random.sh -> build/oracle-src),
# plus the harness (tools/oracle/java), compiled into build/oracle.
# Runnable from anywhere; paths resolve from the repository root.
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
. "$ROOT/tools/oracle/env.sh"   # V and CP (minecraft.jar last)
cd "$ROOT"

sh tools/oracle/rewrite-math-random.sh

rm -rf build/oracle
mkdir -p build/oracle
# -nowarn hides the vanilla tree's deprecation noise (Applet, Thread.stop).
# shellcheck disable=SC2046 # word splitting of the file list is intended
javac -nowarn -proc:none -encoding UTF-8 -cp "$CP" -d build/oracle \
      $(find build/oracle-src tools/oracle/java -name '*.java')
echo "build: compiled oracle into build/oracle"
