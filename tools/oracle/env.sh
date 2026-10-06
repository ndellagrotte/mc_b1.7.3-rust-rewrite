# Shared settings for the oracle scripts; sourced, not executed.
# Expects ROOT (repository root) to be set by the caller.
V="$ROOT/.reference/mc_b1.7.3_release/1.7.3-LTS"
# minecraft.jar LAST: compiled vanilla classes shadow the obfuscated originals,
# its lang/ and paulscode contents stay reachable (PLAN_v4 §4.1).
CP="$V/jars/bin/lwjgl.jar:$V/jars/bin/lwjgl_util.jar:$V/jars/bin/jinput.jar:$V/jars/bin/minecraft.jar"
# Runs the harness: oracle_run <out-dir> [extra java options ...]
oracle_run() {
    _out=$1
    shift
    java "$@" -cp "$ROOT/build/oracle:$CP" harness.Main "$ROOT/tests/golden" "$_out"
}
