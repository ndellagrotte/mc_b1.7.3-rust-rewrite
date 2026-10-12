#!/usr/bin/env bash
# Builds the portable zips (tools/portable/README.md): one per target, each a
# folder mc_b173_<ver>_<target>/ holding the game (AppImage, exe or .app), an
# empty data/ tree (saves, screenshots, stats, exports), NOTICE, LICENSE,
# RUNNING.md and credits/.
#
#   tools/portable/build.sh [--check] [--low-mem] [--clean-after] [-j N] [TARGET...|all]
#
# Targets are built one at a time with rustup's cargo 1.99.0 and
# cargo-zigbuild. Linux links against a focal (glibc 2.31) sysroot made from
# pinned Ubuntu debs; the AppImage is the pinned type2 runtime followed by a
# squashfs image. macOS needs MC_MACOS_SDK and is skipped without it.
# The script installs nothing; it only downloads the pinned runtimes and debs
# into dist/.cache. Outputs: dist/mc_b173_<ver>_<target>.zip, dist/<target>.log.
# Exit: 0 all selected targets built (SKIPs allowed), 1 a target failed,
# 2 usage or prerequisite error. Runnable from anywhere.
set -euo pipefail
shopt -s inherit_errexit

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
HERE=$ROOT/tools/portable
DIST=$ROOT/dist
CACHE=$DIST/.cache
WORK=$DIST/.work
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"   # rustup's cargo before /usr/bin/cargo
# zig's linker opens every object file at once (3000+ with thin LTO); the
# usual soft limit of 1024 open files fails it with ProcessFdQuotaExceeded.
ulimit -S -n "$(ulimit -H -n)" 2>/dev/null || true
FD_MIN=8192

TOOLCHAIN=1.99.0
GLIBC=2.31
JOBS=2
LOWMEM=0
CLEAN_AFTER=0
CHECK_ONLY=0
ORDER="linux-aarch64 linux-x86_64 windows-x86_64 windows-aarch64 macos-aarch64 macos-x86_64"

# Pins. Bump a pin by downloading the file and recomputing its sha256 (for
# the debs, cross-check the SHA256 field in dists/focal-updates/main/
# binary-{amd64,arm64}/Packages.xz). A mismatch is fatal.
RUNTIME_TAG=20251108
RUNTIME_SHA_aarch64=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444
RUNTIME_SHA_x86_64=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d
UA=http://archive.ubuntu.com/ubuntu/pool/main
UP=http://ports.ubuntu.com/ubuntu-ports/pool/main
DEBS=(
    "amd64|$UA/a/alsa-lib/libasound2_1.2.2-2.1ubuntu2.5_amd64.deb|5ea02b4590b210bb2d4a180007184c10e8941d0ee1f025361950b0d0084578a6"
    "amd64|$UA/a/alsa-lib/libasound2-dev_1.2.2-2.1ubuntu2.5_amd64.deb|f10cbb3ccdab80e162e64255e3dfe1e04b02fe1d59d082a7a3d760f0623da9dc"
    "amd64|$UA/s/systemd/libudev1_245.4-4ubuntu3.24_amd64.deb|f2dc7d6771a2f26b93d1f17cdf3011a2b43b1adfd3df716e396a64c6890a4e3a"
    "amd64|$UA/s/systemd/libudev-dev_245.4-4ubuntu3.24_amd64.deb|93dcaa323bc0a5610659dad534ed14df17a91bb7a383ce54ec079b5dcbf5cd3a"
    "arm64|$UP/a/alsa-lib/libasound2_1.2.2-2.1ubuntu2.5_arm64.deb|3cf3900c1b9e5aa90c340099b15f04d8f93c078cd69acf1856ea65fea42b003e"
    "arm64|$UP/a/alsa-lib/libasound2-dev_1.2.2-2.1ubuntu2.5_arm64.deb|df662c7a6f0d7f3fbaafa6ed629ebc35f5eef6c56af577589b2a53292cd80b43"
    "arm64|$UP/s/systemd/libudev1_245.4-4ubuntu3.24_arm64.deb|64e5cb9b70622038d57d2aa898cc4903e127f050e62f84abecf6c6f7b65e86b8"
    "arm64|$UP/s/systemd/libudev-dev_245.4-4ubuntu3.24_arm64.deb|6bf400c522fcc87d9b6aedc03385cf463be9633be22db5749e61d01240eb0240"
)
# Shared libraries a Linux binary may need; anything else fails verify_linux.
NEEDED_OK="libasound.so.2 libudev.so.1 libc.so.6 libm.so.6 libdl.so.2 libpthread.so.0 librt.so.1 libgcc_s.so.1 ld-linux-x86-64.so.2 ld-linux-aarch64.so.1"

log() { echo "build: $*"; }
warn() { echo "build: warning: $*" >&2; }
die() { echo "build: $*" >&2; exit 2; }
fail() { echo "build: FAIL: $*" >&2; exit 1; }

# fetch URL SHA DEST: download once, verify the pin, reuse the cached copy.
fetch() {
    local url=$1 sha=$2 dest=$3
    [ -n "$sha" ] || die "no sha256 pin for $url"
    if [ -f "$dest" ] && echo "$sha  $dest" | sha256sum -c --quiet >/dev/null 2>&1; then
        return 0
    fi
    mkdir -p "$(dirname "$dest")"
    log "fetch $url"
    curl -fL --retry 3 --connect-timeout 20 -sS -o "$dest.part" "$url"
    echo "$sha  $dest.part" | sha256sum -c --quiet \
        || { rm -f "$dest.part"; die "checksum mismatch: $url"; }
    mv "$dest.part" "$dest"
}

usage() {
    cat <<EOF
usage: tools/portable/build.sh [--check] [--low-mem] [--clean-after] [-j N] [TARGET...|all]

Builds dist/mc_b173_<ver>_<target>.zip for each TARGET (default: all), one at a time.
Targets: $ORDER

  --check        print the prerequisite table and exit (0 ready, 1 something missing)
  --low-mem      thin LTO and 16 codegen units instead of fat LTO (less rustc memory)
  --clean-after  remove target/<triple> after each verified zip (saves disk)
  -j N           cargo jobs (default 2)

Env: MC_MACOS_SDK  an extracted MacOSX*.sdk; without it the macOS targets are skipped.
Logs: dist/<target>.log. See tools/portable/README.md.
EOF
}

version() {
    local v
    v=$(sed -n '/^\[workspace\.package\]/,/^\[/{s/^version *= *"\([^"]*\)".*/\1/p}' "$ROOT/Cargo.toml")
    [[ $v =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || die "cannot read [workspace.package] version from Cargo.toml (got '$v')"
    echo "$v"
}

# target_vars T: TRIPLE, ZTARGET (what zigbuild gets), OS, DEBARCH, MA, APPARCH, EXE.
target_vars() {
    DEBARCH= MA= APPARCH= EXE=mc_b173
    case $1 in
        linux-x86_64) TRIPLE=x86_64-unknown-linux-gnu OS=linux DEBARCH=amd64 MA=x86_64-linux-gnu APPARCH=x86_64 ;;
        linux-aarch64) TRIPLE=aarch64-unknown-linux-gnu OS=linux DEBARCH=arm64 MA=aarch64-linux-gnu APPARCH=aarch64 ;;
        windows-x86_64) TRIPLE=x86_64-pc-windows-gnu OS=windows EXE=mc_b173.exe ;;
        windows-aarch64) TRIPLE=aarch64-pc-windows-gnullvm OS=windows EXE=mc_b173.exe ;;
        macos-aarch64) TRIPLE=aarch64-apple-darwin OS=macos ;;
        macos-x86_64) TRIPLE=x86_64-apple-darwin OS=macos ;;
        *) die "unknown target '$1' (targets: $ORDER)" ;;
    esac
    ZTARGET=$TRIPLE
    if [ "$OS" = linux ]; then ZTARGET=$TRIPLE.$GLIBC; fi
}

# row STATUS ITEM [HINT]: one line of the prerequisite table.
row() {
    if [ -n "${3:-}" ]; then printf '  %-8s %-46s %s\n' "$1" "$2" "$3"; else printf '  %-8s %s\n' "$1" "$2"; fi
    if [ "$1" = MISSING ]; then MISSING=1; fi
}

check_common() {
    local t missing="" kb zv fd
    if command -v rustup >/dev/null; then row OK rustup
    else row MISSING rustup "install rustup (https://rustup.rs or 'sudo dnf install rustup && rustup-init -y --default-toolchain none')"; fi
    if (cd "$ROOT" && cargo --version 2>/dev/null) | grep -q "^cargo $TOOLCHAIN"; then row OK "cargo $TOOLCHAIN (rustup)"
    else row MISSING "cargo $TOOLCHAIN (rustup)" "rustup toolchain install $TOOLCHAIN --profile minimal -c rustfmt,clippy"; fi
    if grep -q "^channel = \"$TOOLCHAIN\"" "$ROOT/rust-toolchain.toml" && ! grep -q '^targets' "$ROOT/rust-toolchain.toml"; then
        row OK rust-toolchain.toml
    else row MISSING rust-toolchain.toml "must pin channel \"$TOOLCHAIN\" and list no targets"; fi
    if command -v zig >/dev/null; then
        zv=$(zig version)
        case $zv in 0.15.*|0.16.*) row OK "zig $zv" ;; *) row WARN "zig $zv" "tested with 0.15/0.16" ;; esac
    else row MISSING zig "sudo dnf install zig"; fi
    if (cd "$ROOT" && cargo-zigbuild --version >/dev/null 2>&1); then row OK cargo-zigbuild
    else row MISSING cargo-zigbuild "cargo install --locked cargo-zigbuild@0.23.4"; fi
    for t in curl sha256sum ar tar xz zip unzip zipinfo file objdump mksquashfs od install realpath; do
        command -v "$t" >/dev/null || missing="$missing $t"
    done
    if [ -z "$missing" ]; then row OK "host tools"
    else row MISSING "host tools:$missing" "sudo dnf install curl coreutils binutils tar xz zip unzip file squashfs-tools"; fi
    fd=$(ulimit -S -n)
    if [ "$fd" != unlimited ] && [ "$fd" -lt "$FD_MIN" ]; then
        row WARN "open files limit $fd" "zig's linker needs more (ulimit -n $FD_MIN, or raise the hard limit in /etc/security/limits.d)"
    fi
    kb=$(awk '/^MemAvailable:/{print $2}' /proc/meminfo)
    if [ "${kb:-0}" -lt $((3 * 1024 * 1024)) ]; then row WARN "memory: $((kb / 1024)) MiB available" "close other programs or use --low-mem"; fi
    kb=$(df -Pk "$ROOT" | awk 'NR==2{print $4}')
    if [ "$kb" -lt $((15 * 1024 * 1024)) ]; then row WARN "disk: $((kb / 1024 / 1024)) GiB free" "use --clean-after"; fi
}

# check_target T: 0 ready, 1 missing, 2 skip (macOS without an SDK).
check_target() {
    target_vars "$1"
    if [ "$OS" = macos ] && [ ! -f "${MC_MACOS_SDK:-/nonexistent}/SDKSettings.json" ]; then
        row SKIP "$1" "not supported on this host without an Apple SDK (set MC_MACOS_SDK to an extracted MacOSX*.sdk; see tools/portable/README.md)"
        return 2
    fi
    if ! rustup target list --installed --toolchain "$TOOLCHAIN" 2>/dev/null | grep -qx "$TRIPLE"; then
        row MISSING "$1 ($TRIPLE)" "rustup target add --toolchain $TOOLCHAIN $TRIPLE"
        return 1
    fi
    row OK "$1 ($TRIPLE)"
    case $1 in
        windows-x86_64)
            if ! command -v x86_64-w64-mingw32-windres >/dev/null; then
                row WARN "$1 icon" "no Explorer icon in the exe (optional: sudo dnf install mingw64-binutils gcc)"
            elif ! command -v gcc >/dev/null; then
                row MISSING "$1 icon" "x86_64-w64-mingw32-windres needs a C preprocessor: sudo dnf install gcc"
            fi ;;
        windows-aarch64) row INFO "$1 icon" "the exe has no Explorer icon (window icon unaffected)" ;;
        macos-*) command -v rcodesign >/dev/null \
            || row WARN "$1 signing" "rcodesign missing, the .app stays unsigned (cargo install --locked apple-codesign)" ;;
    esac
    return 0
}

ensure_runtime() {
    local sha_var=RUNTIME_SHA_$1
    fetch "https://github.com/AppImage/type2-runtime/releases/download/$RUNTIME_TAG/runtime-$1" \
        "${!sha_var}" "$CACHE/runtime-$RUNTIME_TAG-$1"
}

# ensure_sysroot DEBARCH MA: dist/.cache/sysroot-<arch> from the four debs.
ensure_sysroot() {
    local arch=$1 ma=$2 s=$CACHE/sysroot-$1 e a url sha deb d l t
    [ -f "$s/.done" ] && return 0
    rm -rf "$s" "$s.tmp"
    mkdir -p "$s.tmp"
    for e in "${DEBS[@]}"; do
        IFS='|' read -r a url sha <<<"$e"
        [ "$a" = "$arch" ] || continue
        deb=$CACHE/debs/$(basename "$url")
        fetch "$url" "$sha" "$deb"
        d=$CACHE/debs/x; rm -rf "$d"; mkdir -p "$d"
        (cd "$d" && ar x "$deb")
        tar -xf "$d"/data.tar.* -C "$s.tmp"
        rm -rf "$d"
    done
    # Insurance: the focal links are relative today; make any absolute one relative.
    find "$s.tmp" -type l -lname '/*' -print0 | while IFS= read -r -d '' l; do
        t=$(readlink "$l"); ln -sfn "$(realpath -s -m --relative-to="$(dirname "$l")" "$s.tmp$t")" "$l"
    done
    test -e "$s.tmp/usr/lib/$ma/libasound.so" || fail "sysroot: no libasound.so"
    test -e "$s.tmp/lib/$ma/libudev.so" || test -e "$s.tmp/usr/lib/$ma/libudev.so" || fail "sysroot: no libudev.so"
    test -e "$s.tmp/usr/lib/$ma/pkgconfig/alsa.pc" || fail "sysroot: no alsa.pc"
    test -e "$s.tmp/usr/lib/$ma/pkgconfig/libudev.pc" || fail "sysroot: no libudev.pc"
    mv "$s.tmp" "$s"
    touch "$s/.done"
}

# windres_wrapper: prints the path of a windres that preprocesses with the host gcc.
windres_wrapper() {
    local w=$CACHE/bin/x86_64-w64-mingw32-windres-hostcpp
    mkdir -p "$CACHE/bin"
    cat >"$w.tmp" <<'EOS'
#!/bin/sh
# tools/portable: windres with the host gcc as the .rc preprocessor.
exec x86_64-w64-mingw32-windres --preprocessor=gcc --preprocessor-arg=-E --preprocessor-arg=-xc --preprocessor-arg=-DRC_INVOKED "$@"
EOS
    chmod 755 "$w.tmp" && mv "$w.tmp" "$w"
    echo "$w"
}

# target_env T: per-target environment; only ever called inside the target's subshell.
target_env() {
    local u s
    export CARGO_BUILD_JOBS=$JOBS
    unset PKG_CONFIG_PATH PKG_CONFIG_LIBDIR PKG_CONFIG_SYSROOT_DIR RC RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
    if [ "$LOWMEM" = 1 ]; then export CARGO_PROFILE_DIST_LTO=thin CARGO_PROFILE_DIST_CODEGEN_UNITS=16; fi
    case $OS in
        windows)
            # mingw64-binutils' windres preprocesses .rc files with
            # x86_64-w64-mingw32-gcc, which Fedora ships separately; the host
            # gcc is enough for icon.rc. embed-resource takes the resource
            # compiler from RC_<target> and accepts anything that answers
            # -V with "GNU windres". Without windres the icon is skipped.
            if [ "$TRIPLE" = x86_64-pc-windows-gnu ] && command -v x86_64-w64-mingw32-windres >/dev/null; then
                export "RC_${TRIPLE//-/_}=$(windres_wrapper)"
            fi ;;
        linux)
            u=${TRIPLE//-/_} s=$CACHE/sysroot-$DEBARCH
            export PKG_CONFIG_ALLOW_CROSS=1 "PKG_CONFIG_ALLOW_CROSS_$u=1" "PKG_CONFIG_SYSROOT_DIR_$u=$s" \
                "PKG_CONFIG_LIBDIR_$u=$s/usr/lib/$MA/pkgconfig:$s/usr/share/pkgconfig" ;;
        macos)
            # rustc would run an external `strip` for Apple targets.
            export SDKROOT=$MC_MACOS_SDK MACOSX_DEPLOYMENT_TARGET=11.0 CARGO_PROFILE_DIST_STRIP=false ;;
    esac
}

cargo_build() {
    log "cargo zigbuild --locked --profile dist -p mc-app --target $ZTARGET (jobs $JOBS)"
    (cd "$ROOT" && cargo zigbuild --locked --profile dist -p mc-app --target "$ZTARGET")
    BIN=$ROOT/target/$TRIPLE/dist/$EXE
    [ -f "$BIN" ] || fail "no binary at $BIN"
    if [ "$BIN" -nt "$WORK/$T/.start" ]; then log "built $BIN"
    else log "$BIN is up to date (cargo did not relink)"; fi
}

verify_linux() {
    local want max lib bad=""
    case $APPARCH in x86_64) want=elf64-x86-64 ;; *) want=elf64-littleaarch64 ;; esac
    objdump -f "$BIN" | grep -q "file format $want" || fail "$BIN is not $want"
    max=$(objdump -T "$BIN" | { grep -o 'GLIBC_[0-9.]*' || true; } | sed 's/GLIBC_//' | sort -uV | tail -1)
    log "max GLIBC symbol version: ${max:-none} (limit $GLIBC)"
    if [ -n "$max" ] && [ "$(printf '%s\n%s\n' "$max" "$GLIBC" | sort -V | tail -1)" != "$GLIBC" ]; then
        fail "needs GLIBC_$max, newer than $GLIBC"
    fi
    for lib in $(objdump -p "$BIN" | awk '$1 == "NEEDED" {print $2}'); do
        log "NEEDED $lib"
        case " $NEEDED_OK " in *" $lib "*) ;; *) bad="$bad $lib" ;; esac
    done
    [ -z "$bad" ] || fail "unexpected shared libraries:$bad"
    if [ "$T" = linux-aarch64 ] && [ "$(uname -m)" = aarch64 ]; then
        local out
        out=$("$BIN" --help 2>&1) || fail "$EXE --help exited non-zero: $out"
        grep -q "usage: mc_b173" <<<"$out" || fail "$EXE --help printed no usage line"
        log "$EXE --help: ok"
    fi
}

verify_windows() {
    local want dlls
    case $TRIPLE in x86_64-*) want=pei-x86-64 ;; *) want=pei-aarch64-little ;; esac
    objdump -f "$BIN" | grep -q "file format $want" || fail "$BIN is not $want"
    # Read objdump's whole output first: grep -q closing the pipe early would
    # give objdump SIGPIPE and, under pipefail, a spurious failure.
    pe=$(objdump -p "$BIN")
    grep -q 'Subsystem.*Windows GUI' <<<"$pe" || fail "$BIN is not a Windows GUI program"
    dlls=$(sed -n 's/^[[:space:]]*DLL Name: //p' <<<"$pe")
    log "DLLs: $(echo $dlls)"
    if grep -Eiq '^(libgcc|libwinpthread|libstdc\+\+|libunwind|libc\+\+|vcruntime|msvcp)' <<<"$dlls"; then
        fail "imports a non-system DLL"
    fi
    if grep -q ' \.rsrc ' <<<"$(objdump -h "$BIN")"; then log "icon: yes"; else log "icon: no"; fi
}

# stage_common STAGE: the files every zip carries besides the game.
stage_common() {
    local st=$1
    mkdir -p "$st"/data/{saves,screenshots,stats,exports}
    install -m644 "$HERE/data-README.txt" "$st/data/README.txt"
    install -m644 "$ROOT/NOTICE" "$ROOT/LICENSE" "$st/"
    install -m644 "$ROOT/docs/RUNNING.md" "$st/RUNNING.md"
    cp -r "$ROOT/credits" "$st/credits"
    [ -z "$(find "$st" -name '*.jar')" ] || fail "a .jar ended up in the zip"
}

# make_appimage OUT: type2 runtime + squashfs of a fresh AppDir. No bundled libraries.
make_appimage() {
    local out=$1 a=$WORK/$T/AppDir sq=$WORK/$T/app.sqfs icon=$ROOT/build/macos/AppIcon.iconset/icon_256x256.png
    rm -rf "$a" "$sq"
    install -Dm755 "$BIN" "$a/usr/bin/mc_b173"
    ln -s usr/bin/mc_b173 "$a/AppRun"
    install -Dm644 "$HERE/mc_b173.desktop" "$a/mc_b173.desktop"
    install -Dm644 "$HERE/mc_b173.desktop" "$a/usr/share/applications/mc_b173.desktop"
    install -Dm644 "$icon" "$a/mc_b173.png"
    install -Dm644 "$icon" "$a/usr/share/icons/hicolor/256x256/apps/mc_b173.png"
    ln -s mc_b173.png "$a/.DirIcon"
    mkdir -p "$a/usr/share/doc/mc_b173"
    cp "$ROOT/NOTICE" "$ROOT/LICENSE" "$ROOT/docs/RUNNING.md" "$a/usr/share/doc/mc_b173/"
    cp -r "$ROOT/credits" "$a/usr/share/doc/mc_b173/credits"
    mksquashfs "$a" "$sq" -all-root -noappend -no-xattrs -comp zstd -quiet -no-progress
    cat "$CACHE/runtime-$RUNTIME_TAG-$APPARCH" "$sq" >"$out.part"
    [ "$(head -c4 "$out.part" | od -An -tx1 | tr -d ' \n')" = 7f454c46 ] || fail "AppImage has no ELF header"
    chmod 755 "$out.part"
    mv "$out.part" "$out"
    rm -rf "$a" "$sq"
    log "AppImage: $out ($(du -h "$out" | cut -f1))"
}

package_linux() {
    ensure_sysroot "$DEBARCH" "$MA"
    ensure_runtime "$APPARCH"
    target_env
    cargo_build
    verify_linux
    make_appimage "$STAGE/mc_b173.AppImage"
    stage_common "$STAGE"
}

package_windows() {
    target_env
    cargo_build
    verify_windows
    install -m755 "$BIN" "$STAGE/mc_b173.exe"
    stage_common "$STAGE"
}

# UNVERIFIED: reached only with MC_MACOS_SDK set (tools/portable/README.md).
package_macos() {
    local app=$STAGE/mc_b173.app/Contents src=$ROOT/build/macos/src/Game.app/Contents
    target_env
    cargo_build
    file "$BIN" | grep -q Mach-O || fail "$BIN is not a Mach-O binary"
    mkdir -p "$app/MacOS" "$app/Resources"
    install -m755 "$BIN" "$app/MacOS/mc_b173"
    cp "$src/Resources/AppIcon.icns" "$app/Resources/"
    sed '/<!-- Version -->/{n;s|<string>[^<]*</string>|<string>'"$VER"'</string>|;}' "$src/Info.plist" >"$app/Info.plist"
    grep -q "<string>$VER</string>" "$app/Info.plist" || fail "Info.plist: version not set"
    if command -v rcodesign >/dev/null; then rcodesign sign "$STAGE/mc_b173.app"
    else warn "rcodesign missing: mc_b173.app is unsigned"; fi
    stage_common "$STAGE"
}

zip_stage() {
    (cd "$WORK/$T/pkg" && zip -r -X -9 -q "$ZIP.part" "$PFX")   # no -D: keep the empty data/ dirs
    mv "$ZIP.part" "$ZIP"
}

verify_zip() {
    local list e payload
    unzip -tq "$ZIP" >/dev/null || fail "unzip -t failed for $ZIP"
    list=$(unzip -Z1 "$ZIP")
    case $OS in
        linux) payload=mc_b173.AppImage ;;
        windows) payload=mc_b173.exe ;;
        macos) payload=mc_b173.app/Contents/MacOS/mc_b173 ;;
    esac
    for e in data/saves/ data/screenshots/ data/stats/ data/exports/ data/README.txt \
        NOTICE LICENSE RUNNING.md credits/ "$payload"; do
        grep -qxF "$PFX/$e" <<<"$list" || fail "zip lacks $PFX/$e"
    done
    if [ "$OS" = linux ]; then
        zipinfo "$ZIP" "$PFX/mc_b173.AppImage" | grep -q '^-rwxr-xr-x' || fail "AppImage is not 0755 in the zip"
    fi
    if grep -q '\.jar$' <<<"$list"; then fail "zip contains a .jar"; fi
    log "zip ok: $(wc -l <<<"$list") entries"
}

# Runs the zipped AppImage with a fake HOME so a real Prism instance is never found.
smoke_linux_aarch64() {
    local w=$WORK/$T/smoke out rc
    rm -rf "$w"
    mkdir -p "$w/home/.local/share" "$w/tmp"
    w=$(realpath "$w")
    unzip -q "$ZIP" -d "$w"
    local run=(env -i "HOME=$w/home" "XDG_DATA_HOME=$w/home/.local/share" PATH=/usr/bin:/bin
        "TMPDIR=$w/tmp" APPIMAGE_EXTRACT_AND_RUN=1 "$w/$PFX/mc_b173.AppImage")
    out=$(cd "$w" && "${run[@]}" --help 2>&1) || fail "smoke: --help exited non-zero: $out"
    grep -q "usage: mc_b173" <<<"$out" || fail "smoke: --help printed no usage line"
    rc=0
    out=$(cd "$w" && "${run[@]}" 2>&1) || rc=$?
    echo "$out"
    [ "$rc" = 1 ] || fail "smoke: expected exit 1 without a Prism instance, got $rc"
    grep -qi "prism" <<<"$out" || fail "smoke: no Prism not-found message"
    grep -qF "mc_b173: data dir: $w/$PFX/data (portable)" <<<"$out" || fail "smoke: portable data dir not used"
    log "smoke: ok (portable data dir beside the AppImage)"
    rm -rf "$w"
}

# build_target T: runs in its own subshell; any failing command fails the target.
build_target() {
    T=$1
    target_vars "$T"
    PFX=mc_b173_${VER}_$T
    ZIP=$DIST/$PFX.zip
    STAGE=$WORK/$T/pkg/$PFX
    log "$T: $TRIPLE, version $VER, $(date -u +%Y-%m-%dT%H:%M:%SZ)"
    rm -rf "${WORK:?}/$T"
    mkdir -p "$WORK/$T/tmp" "$STAGE"
    touch "$WORK/$T/.start"
    export TMPDIR=$WORK/$T/tmp
    rm -f "$ZIP" "$ZIP.part"
    "package_$OS"
    zip_stage
    verify_zip
    if [ "$T" = linux-aarch64 ] && [ "$(uname -m)" = aarch64 ]; then smoke_linux_aarch64; fi
    if [ "$CLEAN_AFTER" = 1 ]; then log "removing target/$TRIPLE"; rm -rf "${ROOT:?}/target/$TRIPLE"; fi
    rm -rf "${WORK:?}/$T"
    log "$T: OK $ZIP"
}

main() {
    local args=() sel=() t rc any_fail=0
    while [ $# -gt 0 ]; do
        case $1 in
            -h|--help) usage; exit 0 ;;
            --check) CHECK_ONLY=1 ;;
            --low-mem) LOWMEM=1 ;;
            --clean-after) CLEAN_AFTER=1 ;;
            -j) [ $# -ge 2 ] || die "-j needs a number"; JOBS=$2; shift ;;
            -j*) JOBS=${1#-j} ;;
            -*) usage >&2; die "unknown option $1" ;;
            *) args+=("$1") ;;
        esac
        shift
    done
    [[ $JOBS =~ ^[1-9][0-9]*$ ]] || die "-j needs a positive number (got '$JOBS')"
    if [ ${#args[@]} -eq 0 ]; then args=(all); fi
    for t in "${args[@]}"; do
        [ "$t" = all ] || case " $ORDER " in *" $t "*) ;; *) die "unknown target '$t' (targets: $ORDER)" ;; esac
    done
    for t in $ORDER; do   # build order, not argument order
        case " ${args[*]} " in *" all "*|*" $t "*) sel+=("$t") ;; esac
    done

    VER=$(version)
    MISSING=0
    declare -A STATUS=()
    echo "mc_b173 $VER portable build: prerequisites"
    check_common
    for t in "${sel[@]}"; do
        rc=0; check_target "$t" || rc=$?
        case $rc in 0) STATUS[$t]=READY ;; 2) STATUS[$t]=SKIP ;; *) STATUS[$t]=MISSING ;; esac
    done
    if [ "$CHECK_ONLY" = 1 ]; then
        if [ "$MISSING" = 0 ]; then echo "ready"; exit 0; fi
        echo "not ready: install what is MISSING (commands above)"; exit 1
    fi
    [ "$MISSING" = 0 ] || die "prerequisites missing (see the table above)"

    mkdir -p "$DIST" "$CACHE" "$WORK"
    for t in "${sel[@]}"; do
        [ "${STATUS[$t]}" = READY ] || continue
        echo "== $t (log: dist/$t.log)"
        set +e
        (
            set -eEuo pipefail
            trap 'echo "build: FAIL: line $LINENO: $BASH_COMMAND" >&2' ERR
            build_target "$t"
        ) 2>&1 | tee "$DIST/$t.log"
        rc=${PIPESTATUS[0]}
        set -e
        if [ "$rc" = 0 ]; then STATUS[$t]=OK
        else STATUS[$t]=FAIL; any_fail=1; rm -f "$DIST/mc_b173_${VER}_$t.zip.part"; fi
    done

    echo
    printf '%-16s %-5s %-6s %s\n' TARGET STATE SIZE "ZIP / LOG"
    for t in "${sel[@]}"; do
        local z=$DIST/mc_b173_${VER}_$t.zip size=-
        case ${STATUS[$t]} in
            OK) size=$(du -h "$z" | cut -f1); printf '%-16s %-5s %-6s %s\n' "$t" OK "$size" "${z#"$ROOT"/}" ;;
            FAIL) printf '%-16s %-5s %-6s %s\n' "$t" FAIL - "dist/$t.log (work dir kept: dist/.work/$t)" ;;
            *) printf '%-16s %-5s %-6s %s\n' "$t" SKIP - "no Apple SDK (MC_MACOS_SDK)" ;;
        esac
    done
    exit "$any_fail"
}

if [ "${BASH_SOURCE[0]}" = "$0" ]; then main "$@"; fi
