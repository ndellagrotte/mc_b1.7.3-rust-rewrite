# tools/portable

`build.sh` builds portable zips of the game on one Linux machine, for six targets:

| Target | Rust target | Game file |
|---|---|---|
| `linux-x86_64` | `x86_64-unknown-linux-gnu` (glibc 2.31) | `mc_b173.AppImage` |
| `linux-aarch64` | `aarch64-unknown-linux-gnu` (glibc 2.31) | `mc_b173.AppImage` |
| `windows-x86_64` | `x86_64-pc-windows-gnu` | `mc_b173.exe` |
| `windows-aarch64` | `aarch64-pc-windows-gnullvm` | `mc_b173.exe` |
| `macos-aarch64` | `aarch64-apple-darwin` | `mc_b173.app` (unverified, see below) |
| `macos-x86_64` | `x86_64-apple-darwin` | `mc_b173.app` (unverified, see below) |

Each zip holds one folder:

```
mc_b173_<ver>_<target>.zip
└─ mc_b173_<ver>_<target>/
   ├─ mc_b173.AppImage | mc_b173.exe | mc_b173.app/
   ├─ data/{saves/,screenshots/,stats/,exports/,README.txt}
   ├─ NOTICE  LICENSE  RUNNING.md
   └─ credits/
```

`data/` beside the game is the game's data directory (portable mode, `crates/mc-client/src/paths.rs`);
`--data-dir` still overrides it. There is no auto-update: to upgrade, unzip the new build and copy
`data/` into it by hand. `data/README.txt` (from `data-README.txt`) tells players the same. The game
still needs a Prism Launcher b1.7.3 instance for `minecraft.jar` and `resources/`; the repository's
`b1.7.3-mmc-instance/b1.7.3.zip` is an importable one (docs/RUNNING.md).

## Prerequisites

The script installs nothing. `build.sh --check` prints an OK/MISSING/WARN/SKIP table with the command
for each missing piece. It puts `~/.cargo/bin` first on PATH itself, so rustup's cargo (1.99.0, from
`rust-toolchain.toml`) wins over a distro `/usr/bin/cargo`. On Fedora:

```
sudo dnf install rustup zig squashfs-tools zip unzip binutils file curl
rustup-init -y --default-toolchain none     # once, if rustup is new
export PATH="$HOME/.cargo/bin:$PATH"
rustup toolchain install 1.99.0 --profile minimal -c rustfmt,clippy
cargo install --locked cargo-zigbuild@0.23.4
rustup target add --toolchain 1.99.0 x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu \
    x86_64-pc-windows-gnu aarch64-pc-windows-gnullvm
sudo dnf install mingw64-binutils gcc       # optional: Explorer icon in the x86_64 exe
```

zig 0.15 or 0.16 is expected (other versions only warn). The targets are not listed in
`rust-toolchain.toml` on purpose, so CI does not install them.

zig's linker opens every object file at once (over 3,000 with `--low-mem`), so the script raises the
soft open-files limit to the hard limit itself. If `--check` warns about the limit, the hard limit is
also low; raise it with `ulimit -n 8192` or a `/etc/security/limits.d` entry.

## Usage

```
export PATH="$HOME/.cargo/bin:$PATH"
tools/portable/build.sh --check
tools/portable/build.sh linux-aarch64 windows-x86_64
tools/portable/build.sh            # all six; macOS is skipped without an SDK
```

The four targets this host can produce, one after another (about 9 minutes each once the
dependencies are cached, on an 8 GB machine with thin LTO):

```
tools/portable/build.sh --low-mem linux-aarch64 linux-x86_64 windows-x86_64 windows-aarch64
```

Options: `--low-mem` (thin LTO, 16 codegen units; the binary is a little bigger and slower than with
the dist profile's fat LTO, but the final link needs far less memory), `--clean-after` (remove
`target/<triple>` after each verified zip), `-j N` (cargo jobs, default 2). Targets always build one at
a time, in the table's order whatever the argument order (default: all six). Exit status: 0 when every selected
target was built (macOS SKIPs count as success), 1 when one failed, 2 on a usage or prerequisite error.

There is no parallel mode. Running several `build.sh` processes at once does not help: they share
`target/`, and cargo serialises them on the build-directory lock while their output interleaves.

Outputs, all under the git-ignored `dist/`:

- `dist/mc_b173_<ver>_<target>.zip`, and `dist/<target>.log` with the full build output
- `dist/.work/<target>/`: staging; kept after a failure, removed after a success
- `dist/.cache/`: the pinned AppImage runtimes, the Ubuntu debs and the sysroots made from them

Cargo output goes to `target/<triple>/dist/`. There are no stamps: rerunning a target rebuilds its zip
(cargo itself skips the compile when nothing changed).

## What the script does and checks

- Linux: links against a focal (Ubuntu 20.04, glibc 2.31) sysroot built from eight pinned debs
  (`libasound2`, `libudev1` and their `-dev` packages for amd64 and arm64), through pkg-config's
  per-target variables. The binary must use no GLIBC symbol newer than 2.31 and may only need ALSA,
  libudev and glibc libraries; Wayland, X11, xkbcommon and Vulkan are loaded at run time. The AppImage
  is the pinned type2 runtime (`20251108`) followed by a zstd squashfs made with `mksquashfs`; no
  libraries are bundled. On an aarch64 host the linux-aarch64 zip is unpacked and run twice with a fake
  HOME: `--help` must work, and a plain run must report `data dir: .../data (portable)` beside the
  AppImage before exiting for the missing Prism instance. The x86_64 AppImage is only checked, not run.
- Windows: the exe must be a GUI-subsystem PE of the right machine type that imports only system DLLs
  (no libgcc, winpthread, libunwind or C++ runtime DLLs).
- Every zip is tested with `unzip -t` and must contain the `data/` tree, the docs and the game file
  (mode 0755 for the AppImage), and no `.jar`.

Every download is pinned by sha256 at the top of `build.sh`; a mismatch stops the build. To bump a pin,
download the new file and compute its sha256 (for the debs, cross-check the `SHA256` field in
`dists/focal-updates/main/binary-{amd64,arm64}/Packages.xz`).

## Memory, disk, time

The dist profile uses fat LTO with one codegen unit; rustc can peak at 3-5 GB on the final link. With
little free RAM, close other programs or use `--low-mem`. Each target needs about 3-5 GB under
`target/<triple>`; `--clean-after` removes it once the zip is verified. `--check` warns below 3 GiB of
available memory or 15 GiB of free disk. Expect 15-40 minutes per target on a cold build.

## Icons

The window icon is set at run time on every platform. The Explorer icon is embedded only in the
`windows-x86_64` exe, and only when `x86_64-w64-mingw32-windres` (mingw64-binutils) is installed;
the `windows-aarch64` exe never has one. The log says `icon: yes` or `icon: no`. windres preprocesses
`icon.rc` with `x86_64-w64-mingw32-gcc`, which Fedora ships as a separate package; the script instead
hands embed-resource a wrapper (`dist/.cache/bin/`) that uses the host `gcc`, so `mingw64-binutils`
and `gcc` are all it needs.

## macOS (not produced yet)

The macOS steps are wired but unverified. They run only when `MC_MACOS_SDK` points at an extracted
`MacOSX*.sdk` (with `SDKSettings.json`); without it both targets are reported as SKIP. The Apple SDK
is under Apple's licence, so it is neither downloaded nor included here; extract it from Xcode
yourself. Also needed: `rustup target add aarch64-apple-darwin x86_64-apple-darwin` and, for an
ad-hoc signature, `cargo install --locked apple-codesign` (`rcodesign`). The `.app` is built from
`build/macos/src/Game.app` with the workspace version in `Info.plist`.

A quarantined `.app` is run from a hidden read-only copy (App Translocation), so it would not see
`data/` beside it. Players run `xattr -dr com.apple.quarantine <unzipped folder>` first.
