#!/usr/bin/env bash
# Smoke test for the mc_b173 AppImage: smoke_appimage.sh <appimage>.
# 1. --help prints the usage and exits 0.
# 2. a minecraft.jar candidate and resources/ beside the .AppImage are found.
# 3. two candidate jars beside it are ambiguous: none is used.
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
    XDG_DATA_HOME="$s/xdg" timeout 60 "$s/game.AppImage" >"$s/log" 2>&1 || true
}
fail() {
    echo "----- log -----"
    cat "$s/log"
    echo "---------------"
    echo "smoke: $*" >&2
    exit 1
}

# 2. lookup beside the AppImage
cp "$img" "$s/game.AppImage"
mkdir "$s/resources"
make_jar "$s/b173-client.jar"
run_game
grep -qxF "mc_b173: minecraft.jar: $s/b173-client.jar" "$s/log" || fail "jar beside the AppImage not found"
grep -qxF "mc_b173: resources: $s/resources" "$s/log" || fail "resources/ beside the AppImage not found"
echo "smoke: lookup beside the AppImage OK"

# 3. ambiguity
make_jar "$s/second.jar"
run_game
grep -qF "several jars" "$s/log" || fail "two jars did not report 'several jars'"
if grep -qF "minecraft.jar: $s/b173-client.jar" "$s/log"; then
    fail "an ambiguous jar was used"
fi
echo "smoke: ambiguous jars OK"

echo "smoke OK"
