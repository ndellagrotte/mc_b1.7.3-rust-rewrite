#!/usr/bin/env bash
# Smoke test for the mc_b173 AppImage: smoke_appimage.sh <appimage>.
# 1. --help prints the usage and exits 0.
# 2. a Prism b1.7.3 instance under $XDG_DATA_HOME/PrismLauncher is auto-detected.
# 3. --prism-instance finds the same instance given explicitly.
# 4. an instance without the client jar asks for a launch from Prism.
# SMOKE_ONLY_HELP=1 runs step 1 alone. Runs without FUSE (CI runners have none).
set -euo pipefail

if [ "$#" -ne 1 ]; then
    echo "usage: $0 <appimage>" >&2
    exit 2
fi
img=$(realpath "$1")
export APPIMAGE_EXTRACT_AND_RUN=1

s=$(realpath "$(mktemp -d)")
trap 'rm -rf "$s"' EXIT
# The runtime extracts into $TMPDIR/appimage_extracted_<digest> and leaves it
# there; keep it under $s so the cleanup takes it.
mkdir "$s/tmp"
export TMPDIR="$s/tmp"

# 1. help
help_out=$("$img" --help 2>&1) || { echo "$help_out"; echo "smoke: --help exited non-zero" >&2; exit 1; }
if ! grep -q '^usage: mc_b173 ' <<<"$help_out"; then
    echo "$help_out"
    echo "smoke: --help output lacks 'usage: mc_b173'" >&2
    exit 1
fi
echo "smoke: --help OK"
if [ "${SMOKE_ONLY_HELP:-}" = 1 ]; then
    echo "smoke OK (help only)"
    exit 0
fi

make_jar() {
    python3 -c 'import sys, zipfile; zipfile.ZipFile(sys.argv[1], "w").writestr("terrain.png", b"")' "$1"
}
run_game() {
    # No display on CI: the game exits non-zero after printing the paths.
    XDG_DATA_HOME="$s/xdg" timeout 60 "$img" "$@" >"$s/log" 2>&1 || true
}
fail() {
    echo "----- log -----"
    cat "$s/log"
    echo "---------------"
    echo "smoke: $*" >&2
    exit 1
}

# A launched Prism instance: instance.cfg, resources/ and the library jar.
prism="$s/xdg/PrismLauncher"
inst="$prism/instances/b1.7.3"
jar="$prism/libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar"
mkdir -p "$inst/minecraft/resources" "$(dirname "$jar")"
touch "$inst/instance.cfg"
make_jar "$jar"
check_paths() {
    grep -qxF "mc_b173: minecraft.jar: $jar" "$s/log" || fail "$1: client jar not found"
    grep -qxF "mc_b173: resources: $inst/minecraft/resources" "$s/log" || fail "$1: resources/ not found"
}

# 2. auto-detection
run_game
check_paths "auto-detect"
echo "smoke: Prism auto-detection OK"

# 3. explicit instance
run_game --prism-instance "$inst"
check_paths "--prism-instance"
echo "smoke: --prism-instance OK"

# 4. never launched: no jar
rm "$jar"
run_game
grep -qF "at least once" "$s/log" || fail "a missing jar did not ask to launch the instance"
echo "smoke: missing jar OK"

echo "smoke OK"
