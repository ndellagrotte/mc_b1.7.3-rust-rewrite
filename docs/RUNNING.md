# Running the game

`mc_b173` is a Rust rewrite of Minecraft Beta 1.7.3. **It contains nothing of
Mojang's.** It reads the textures and the `lang/` files from your own copy of the
Beta 1.7.3 client jar, and sounds and music from a `resources/` directory, at
run time (PLAN_v4 D8). Both come from a Prism Launcher b1.7.3 instance.

## 1. Set up Prism Launcher

1. Install [Prism Launcher](https://prismlauncher.org/) (the native package or
   the Flatpak).
2. Create an instance of Minecraft **b1.7.3** and leave its name as `b1.7.3`
   (its folder is then `.../PrismLauncher/instances/b1.7.3`).
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

The AppImage (Linux, x86_64):

```sh
chmod +x mc_b173_<ver>_x86_64.AppImage
./mc_b173_<ver>_x86_64.AppImage
```

On a system without FUSE 2 or 3 support the AppImage cannot mount itself;
run it with `./mc_b173_<ver>_x86_64.AppImage --appimage-extract-and-run`.

From the unpacked archive: `./mc_b173`.

If the instance has another name, or Prism keeps its data elsewhere, give the
instance directory:

```sh
mc_b173 --prism-instance ~/Games/PrismLauncher/instances/my-beta
```

The client jar is always looked for two levels up from the instance, under
`libraries/` (Prism's own layout; a custom instance folder in Prism's settings
is not supported).

At start-up the game prints on stderr the instance and the two paths it chose,
for example:

```text
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

## 3. The data directory

The game keeps its own files in its own data directory and never touches your
real `.minecraft` or the Prism instance's saves:

| System | Data directory |
|---|---|
| Linux | `$XDG_DATA_HOME/mc_b173`, else `~/.local/share/mc_b173` |
| macOS | `~/Library/Application Support/mc_b173` |
| Windows | `%APPDATA%\mc_b173` |

Inside it, as in vanilla: `options.txt`, `saves/`, `screenshots/`, `stats/`.

## 4. Command line

| Flag | Meaning | Default |
|---|---|---|
| `--data-dir DIR` | options, `saves/`, `screenshots/`, `stats/` | the table above |
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
files only), `credits/`, `NOTICE`, `LICENSE` and this file. For Linux the
release has both `mc_b173_<ver>_linux.tar.gz` and an x86_64 AppImage,
`mc_b173_<ver>_x86_64.AppImage`, which holds the binary, the icon, a `.desktop`
entry, `credits/`, `NOTICE`, `LICENSE` and this file. Neither contains anything
of Mojang's. Both are built on Ubuntu 22.04, so they need glibc 2.35 or newer;
ALSA (`libasound.so.2`), `libudev.so.1`, `libwayland-client.so.0` and the GPU
driver come from the system. The browser build is Phase 5.

## Saves on disk

Worlds are vanilla McRegion saves (`saves/<name>/level.dat`, `region/r.X.Z.mcr`)
and open in the vanilla client. While the game runs, saves are written by a
background thread. A region file is updated by rewriting only the sectors that
changed, through a small journal file next to it (`.mc_journal-r.X.Z.mcr`).
If the game or the computer stops in the middle of a write, the next start
finishes the update from the journal, so every file is either the old or the new
version (`crates/mc-client/src/storage.rs`). Quit through the game menu (or close
the window, which does the same) so the last save is written.
