# Shared settings for the oracle scripts; sourced, not executed.
# Expects ROOT (repository root) to be set by the caller.
V="$ROOT/.reference/mc_b1.7.3_release/1.7.3-LTS"
# minecraft.jar LAST: compiled vanilla classes shadow the obfuscated originals,
# its lang/ and paulscode contents stay reachable (PLAN_v4 §4.1).
CP="$V/jars/bin/lwjgl.jar:$V/jars/bin/lwjgl_util.jar:$V/jars/bin/jinput.jar:$V/jars/bin/minecraft.jar"
# PLAN_v4 §3.4 rule 5 decision (WP-0.3, 2026-10-06): the oracle runs with HotSpot's
# transcendental intrinsics disabled, so java.lang.Math.* produces exactly the
# StrictMath (fdlibm 5.3) results on every JDK and CPU. mc_core::jmath transcribes
# fdlibm; the real client (intrinsics on) differs by 1 ulp on ~3 % of sin/cos/pow
# inputs, which is accepted. See docs/PLAN_v4.md §3.4 rule 5.
JAVA_MATH_FLAGS="-XX:+UnlockDiagnosticVMOptions -XX:DisableIntrinsic=_dsin,_dcos,_dtan,_datan2,_dpow,_dexp,_dlog,_dlog10"
# SOAK selects the scenario set (tests/golden/README.md "Soak", WP-4.2):
#   unset or 0  every scenario except tests/golden/soak/* (the default)
#   1           every scenario, tests/golden/soak/* included
#   only        tests/golden/soak/* alone
# golden_ids prints the selected <area>/<name> ids, sorted, one per line.
golden_ids() {
    case "${SOAK:-0}" in
        0|1|only) ;;
        *) echo "SOAK must be 0, 1 or only, got \"$SOAK\"" >&2; return 1 ;;
    esac
    (
        cd "$ROOT/tests/golden" || exit 1
        for _s in */*/scenario.json; do
            _d=${_s%/scenario.json}
            case "${SOAK:-0}:$_d" in
                0:soak/*) ;;
                only:soak/*) echo "$_d" ;;
                only:*) ;;
                *) echo "$_d" ;;
            esac
        done
    )
}
# Runs the harness on the golden_ids scenarios: oracle_run <out-dir> [extra java options ...]
oracle_run() {
    _out=$1
    shift
    _ids=$(golden_ids) || return 1
    # harness.Main treats an empty id list as "every scenario" (java/harness/Main.java:47,60)
    [ -n "$_ids" ] || { echo "oracle_run: SOAK=${SOAK:-0} selects no scenarios" >&2; return 1; }
    # shellcheck disable=SC2086 # word splitting of the flags and of the id list is intended
    java $JAVA_MATH_FLAGS "$@" -cp "$ROOT/build/oracle:$CP" harness.Main "$ROOT/tests/golden" "$_out" $_ids
}
