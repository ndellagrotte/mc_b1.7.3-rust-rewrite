#!/usr/bin/env bash
# Builds an x86_64 AppImage of mc_b173: make_appimage.sh <binary> <output>.
# The AppDir holds only the binary, the desktop entry, our own icon and the
# project's NOTICE, LICENSE, docs/RUNNING.md and credits/. Nothing
# Mojang-derived enters: the player's minecraft.jar and resources/ are found
# next to the .AppImage file at run time (docs/RUNNING.md).
# Needs APPIMAGETOOL (appimagetool-x86_64.AppImage) and APPIMAGE_RUNTIME
# (the type2 runtime-x86_64 file).
set -euo pipefail

if [ "$#" -ne 2 ]; then
    echo "usage: $0 <binary> <output.AppImage>" >&2
    exit 2
fi
bin=$1
out=$2
root=$(cd "$(dirname "$0")/../.." && pwd)

die() { echo "make_appimage: $*" >&2; exit 1; }
[ -n "${APPIMAGETOOL:-}" ] || die "set APPIMAGETOOL to the appimagetool-x86_64.AppImage path"
[ -n "${APPIMAGE_RUNTIME:-}" ] || die "set APPIMAGE_RUNTIME to the type2 runtime-x86_64 path"
[ -f "$APPIMAGETOOL" ] || die "APPIMAGETOOL not found: $APPIMAGETOOL"
[ -x "$APPIMAGETOOL" ] || die "APPIMAGETOOL is not executable: $APPIMAGETOOL"
[ -f "$APPIMAGE_RUNTIME" ] || die "APPIMAGE_RUNTIME not found: $APPIMAGE_RUNTIME"
[ -f "$bin" ] || die "binary not found: $bin"

# Resolve the output to an absolute path (its directory must exist).
out_dir=$(cd "$(dirname "$out")" && pwd) || die "output directory missing: $(dirname "$out")"
out="$out_dir/$(basename "$out")"

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
appdir="$work/AppDir"
mkdir "$appdir" "$work/tmp"

icon="$root/build/macos/AppIcon.iconset/icon_256x256.png"
desktop="$root/build/linux/mc_b173.desktop"

install -Dm755 "$bin" "$appdir/usr/bin/mc_b173"
ln -s usr/bin/mc_b173 "$appdir/AppRun"

install -Dm644 "$desktop" "$appdir/mc_b173.desktop"
install -Dm644 "$desktop" "$appdir/usr/share/applications/mc_b173.desktop"

install -Dm644 "$icon" "$appdir/mc_b173.png"
install -Dm644 "$icon" "$appdir/usr/share/icons/hicolor/256x256/apps/mc_b173.png"
ln -s mc_b173.png "$appdir/.DirIcon"

doc="$appdir/usr/share/doc/mc_b173"
mkdir -p "$doc"
cp "$root/NOTICE" "$root/LICENSE" "$root/docs/RUNNING.md" "$doc/"
cp -r "$root/credits" "$doc/credits"

# TMPDIR: the extracted appimagetool lands in $work, not in /tmp for good.
TMPDIR="$work/tmp" ARCH=x86_64 APPIMAGE_EXTRACT_AND_RUN=1 "$APPIMAGETOOL" --no-appstream \
    --runtime-file "$APPIMAGE_RUNTIME" "$appdir" "$out"

# A killed appimagetool leaves the runtime area zeroed: check the ELF magic.
[ "$(head -c 4 "$out" | od -An -tx1 | tr -d ' \n')" = 7f454c46 ] || die "$out has no runtime (appimagetool did not finish)"

echo "AppImage: $out ($(du -h "$out" | cut -f1))"
