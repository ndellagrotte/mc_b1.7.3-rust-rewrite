# Running the game

`mc_b173` is a Rust rewrite of Minecraft Beta 1.7.3. **It contains nothing of
Mojang's.** It reads the textures and the `lang/` files from your own copy of the
Beta 1.7.3 client jar, and sounds and music from a `resources/` directory, at
run time (PLAN_v4 D8). Both come from a Prism Launcher b1.7.3 instance.

## 1. Set up Prism Launcher

1. Install [Prism Launcher](https://prismlauncher.org/) (the native package or
   the Flatpak).
2. Import the instance that ships with this repository:
   `b1.7.3-mmc-instance/b1.7.3.zip` (Prism: *Add Instance → Import → Browse*).
   It is a plain MultiMC/Prism instance export, Minecraft **b1.7.3** named
   `b1.7.3`, nothing else; its folder is then `.../PrismLauncher/instances/b1.7.3`.
   Creating an instance of b1.7.3 by hand and naming it `b1.7.3` works the same.
3. **Launch it from Prism at least once.** Prism downloads the client jar and the
   sounds on the first launch; you can quit as soon as the title screen shows.

Afterwards the files the game reads are:

```text
<Prism data dir>/
  libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar   the client jar
  instances/b1.7.3/
    instance.cfg
    minecraft/resources/        sound/, newsound/, music/, newmusic/, streaming/
```

(Older instances use `.minecraft/` instead of `minecraft/`; both work.) The game
reads only these folders in `resources/`: `sound/`, `newsound/` (effects),
`music/`, `newmusic/` (music) and `streaming/` (records). Texture packs are not
supported (deferred, PLAN_v4 D3).

## 2. Start the game

With the instance named `b1.7.3` in Prism's default data directory, start the
game with no arguments. These places are tried, in order:

| System | Instance looked for |
|---|---|
| Linux | `$XDG_DATA_HOME/PrismLauncher/instances/b1.7.3` (else `~/.local/share/...`), then the Flatpak's `~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3` |
| macOS | `~/Library/Application Support/PrismLauncher/instances/b1.7.3` |
| Windows | `%APPDATA%\PrismLauncher\instances\b1.7.3` |

From the unpacked release tarball (Linux): `./mc_b173`.

If the instance has another name, or Prism keeps its data elsewhere, give the
instance directory:

```sh
mc_b173 --prism-instance ~/Games/PrismLauncher/instances/my-beta
```

The client jar is always looked for two levels up from the instance, under
`libraries/` (Prism's own layout; a custom instance folder in Prism's settings
is not supported).

At start-up the game prints on stderr the data directory (with " (portable)"
when it is the `data/` folder beside the game), the instance and the two paths
it chose,
for example:

```text
mc_b173: data dir: /home/you/.local/share/mc_b173
mc_b173: prism instance: /home/you/.local/share/PrismLauncher/instances/b1.7.3
mc_b173: minecraft.jar: /home/you/.local/share/PrismLauncher/libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar
mc_b173: resources: /home/you/.local/share/PrismLauncher/instances/b1.7.3/minecraft/resources
```

If something is missing, the game stops and says what. No instance found:

```text
mc_b173: no Prism Launcher b1.7.3 instance found (tried ...); create a b1.7.3 instance in Prism Launcher, launch it once, and pass --prism-instance <.../PrismLauncher/instances/b1.7.3> if it is elsewhere (docs/RUNNING.md)
```

The instance exists but has never been launched:

```text
mc_b173: found the Prism instance at /home/you/.local/share/PrismLauncher/instances/b1.7.3 but not the client jar at /home/you/.local/share/PrismLauncher/libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar; launch the b1.7.3 instance from Prism Launcher at least once so it downloads them, then try again
```

A directory that is not a Prism instance (no `instance.cfg`) is refused too.

### Portable builds

Portable zips are named `mc_b173_<ver>_<target>.zip`: `linux-x86_64` and
`linux-aarch64` (an AppImage inside), `windows-x86_64` and `windows-aarch64`
(an `.exe`), `macos-aarch64` and `macos-x86_64` (a `.app`; not produced yet, it
needs an Apple SDK). Each holds one folder with the game, an empty `data/`
tree, `NOTICE`, `LICENSE`, this file and `credits/`.

Unzip anywhere and run the AppImage, `.exe` or `.app` inside the folder. A
folder named `data` next to the game (next to the `.AppImage`, `mc_b173.exe` or
the `.app`) is used as the data directory; `--data-dir` wins over it. Delete or
rename `data/` to fall back to the platform folder (section 3). Prism Launcher
with the b1.7.3 instance is still required, as above.

There is no auto-update. To upgrade, unzip the new build and copy the old
`data/` into it (or just `options.txt` and `saves/`).

- Linux: needs glibc 2.31 or newer and the system's ALSA, libudev, Wayland or
  X11 and GPU driver. Without FUSE, run `./mc_b173.AppImage --appimage-extract-and-run`.
  If unzip dropped the executable bit: `chmod +x mc_b173.AppImage`.
- Windows: the exe is unsigned; at the SmartScreen warning choose "More info",
  then "Run anyway".
- macOS: run `xattr -dr com.apple.quarantine <unzipped folder>` first, or macOS
  runs the app from a hidden copy that does not see `data/`.

They are built with `tools/portable/build.sh --check`, then
`tools/portable/build.sh [targets]` (see `tools/portable/README.md`).

## 3. The data directory

The game keeps its own files in its own data directory and never touches your
real `.minecraft` or the Prism instance's saves:

| System | Data directory |
|---|---|
| Linux | `$XDG_DATA_HOME/mc_b173`, else `~/.local/share/mc_b173` |
| macOS | `~/Library/Application Support/mc_b173` |
| Windows | `%APPDATA%\mc_b173` |

Inside it, as in vanilla: `options.txt`, `saves/`, `screenshots/`, `stats/`, `exports/`.

**Portable builds.** A folder named `data` next to the game is used instead of
the table above; `--data-dir` overrides both. The game prints
`mc_b173: data dir: <path>` on stderr at start, with ` (portable)` appended in
this mode. Any `data/` folder beside a bare binary is honoured too. There is no
auto-update: copy `data/` into the new build by hand.

## 4. Command line

| Flag | Meaning | Default |
|---|---|---|
| `--data-dir DIR` | options, `saves/`, `screenshots/`, `stats/`, `exports/` | `data/` next to the game when present, else the table above |
| `--prism-instance DIR` | your Prism Launcher b1.7.3 instance (the client jar and `resources/`) | the first instance found in section 2's table |
| `--username NAME` | the player name (names the stats file) | vanilla's `Player<millis % 1000>` |
| `--world NAME` | open `saves/NAME`, or create it, skipping the title screen | title screen |
| `--seed SEED` | the new world's seed (with `--world`) | random |
| `--import DIR` | copy a vanilla Beta 1.7.3 save into `saves/`, then open it. A world with a block or item id Beta 1.7.3 does not have is refused, and nothing is copied (PLAN_v4 D5) | — |
| `--vsync on\|off` | `on` waits for the display's refresh (no tearing, the frame rate capped at the display's). `off`, the default, runs as the original game does (below) | `off` |
| `--script FILE` | the scripted-input dev mode (`crates/mc-client/README.md`) | — |

A relative `--prism-instance` is relative to the current directory.

**Frame rate.** As in the original game, the "Performance" video option sets
the limit: "Balanced" (the default) and "Max FPS" run as fast as the machine
allows, and "Power saver" holds the game at about 40 frames a second. The
original never waits for the display; `--vsync on` does, for a smoother
picture without tearing. "Balanced" also gives the terrain builder a time budget
of 1/120 s per frame, as the original does, so with `--vsync on` new chunks
appear a little more slowly. A window that is covered or minimised may be
slowed by the desktop whatever the setting.

Worlds can also be imported in the game: Singleplayer, then "Import...", then
type or paste the path of the save folder.

## Development builds

Development builds find the jar and `resources/` the same way (section 2).

```sh
cargo run -p mc-app -- --world test --seed 123          # dev profile
cargo build --profile dist -p mc-app                    # the release build: target/dist/mc_b173
```

The `dist` profile (opt-level 3, LTO, one codegen unit, stripped) is what the
release workflow (`.github/workflows/release.yaml`, on a `v*` tag) builds for
Linux, Windows and macOS. Each archive holds the binary, `assets/` (project
files only), `credits/`, `NOTICE`, `LICENSE` and this file, and nothing of
Mojang's. On Linux, ALSA (`libasound.so.2`), `libudev.so.1`,
`libwayland-client.so.0` and the GPU driver come from the system. The six
portable zips (section "Portable builds") are built locally with
`tools/portable/build.sh`.

## Saves on disk

Worlds are vanilla McRegion saves (`saves/<name>/level.dat`, `region/r.X.Z.mcr`)
and open in the vanilla client. While the game runs, saves are written by a
background thread. A region file is updated by rewriting only the sectors that
changed, through a small journal file next to it (`.mc_journal-r.X.Z.mcr`).
If the game or the computer stops in the middle of a write, the next start
finishes the update from the journal, so every file is either the old or the new
version (`crates/mc-client/src/storage.rs`). Quit through the game menu (or close
the window, which does the same) so the last save is written.
