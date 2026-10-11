# Running the game

`mc_b173` is a Rust rewrite of Minecraft Beta 1.7.3. **It contains nothing of
Mojang's.** It reads the textures and the `lang/` files from your own copy of the
Beta 1.7.3 client jar (`minecraft.jar`), and sounds and music from a `resources/`
directory, at run time (PLAN_v4 D8). You have to supply both.

## 1. Get the files

**`minecraft.jar`.** This is the Beta 1.7.3 client jar, about 1.5 MB. It must
contain `terrain.png` (the game checks this at start-up). Some ways to get it:

- From a launcher that installs Beta 1.7.3. In Prism Launcher, create a b1.7.3
  instance and start it once. The jar is then in the launcher's data folder
  under `libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar`.
  Next to the game it may keep that name if it is the only jar there that
  contains `terrain.png` (section 2); anywhere else rename it to
  `minecraft.jar` or point `--jar` at it.
- From an old `.minecraft/bin/minecraft.jar` that you kept.

A modded jar works if `terrain.png`, `gui/`, `lang/` and the other resources
are where vanilla puts them. Texture packs are not supported (deferred, PLAN_v4 D3).

**`resources/`.** The game reads only these folders: `sound/`, `newsound/`
(effects), `music/`, `newmusic/` (music) and `streaming/` (records), with
`.ogg` and `.mus` files. Old launchers downloaded them into
`.minecraft/resources/`. A Prism Launcher b1.7.3 instance has them in
`<instance>/minecraft/resources/` after its first start. An empty directory
works (the game is then silent); a missing one is an error.

## 2. Put them next to the game

The simplest layout is one folder with the game and the two files:

```text
Games/b1.7.3/
  mc_b173_<ver>_x86_64.AppImage   or the unpacked archive's mc_b173
  minecraft.jar                   your Beta 1.7.3 client jar
  resources/                      sound/, newsound/, music/, newmusic/, streaming/
```

Then start the game with no arguments. The AppImage (Linux, x86_64):

```sh
chmod +x mc_b173_<ver>_x86_64.AppImage
./mc_b173_<ver>_x86_64.AppImage
```

On a system without FUSE 2 or 3 support the AppImage cannot mount itself;
run it with `./mc_b173_<ver>_x86_64.AppImage --appimage-extract-and-run`.

From the unpacked archive: `./mc_b173`.

"Next to the game" means the directory the `.AppImage` file is really in, or
for the unpacked archive the directory of the `mc_b173` executable. A symlink
to either does not count: the directory of the file it points to does.
A tool such as AppImageLauncher that moves the file into its own folder
changes which directory that is, so put the two files where the AppImage
ended up.

The jar is found by the name `minecraft.jar`. If there is none, the one
`*.jar` file there that contains `terrain.png` is used, so the launcher's
`minecraft-b1.7.3-client.jar` works without renaming. If several jars there
contain `terrain.png`, none of them is used, the game says so on stderr

```text
mc_b173: several jars with terrain.png in /home/you/Games/b1.7.3: /home/you/Games/b1.7.3/a.jar and /home/you/Games/b1.7.3/b.jar; using none (rename yours to minecraft.jar or pass --jar)
```

and looks in the data directory instead (section 3). The sounds must be in a
directory named exactly `resources`.

Only the jar and `resources/` are found next to the game. Options, saves,
screenshots and statistics still go to the data directory (section 3) or to
`--data-dir`.

## 3. Or put them in the data directory

The game keeps its files in its own data directory and never touches your real
`.minecraft`:

| System | Data directory |
|---|---|
| Linux | `$XDG_DATA_HOME/mc_b173`, else `~/.local/share/mc_b173` |
| macOS | `~/Library/Application Support/mc_b173` |
| Windows | `%APPDATA%\mc_b173` |

Inside it, as in vanilla: `options.txt`, `saves/`, `screenshots/`, `stats/`.
Put the two files next to them:

```text
<data dir>/
  minecraft.jar        your Beta 1.7.3 client jar
  resources/           sound/, newsound/, music/, newmusic/, streaming/
  options.txt  saves/  screenshots/  stats/     (the game makes these)
```

Then start the game with no arguments. On Linux, from the unpacked release
archive or with the AppImage (section 2 for `chmod +x` and the no-FUSE flag):

```sh
./mc_b173
./mc_b173_<ver>_x86_64.AppImage
```

At start-up the game prints on stderr the two paths it chose, for example:

```text
mc_b173: minecraft.jar: /home/you/.local/share/mc_b173/minecraft.jar
mc_b173: resources: /home/you/.local/share/mc_b173/resources
```

If a file is missing, the game stops and prints the path it tried, for
example:

```text
mc_b173: cannot read minecraft.jar at /home/you/.local/share/mc_b173/minecraft.jar (No such file or directory (os error 2)); give your own Beta 1.7.3 client jar with --jar <path>, or without --jar put it next to the AppImage or the mc_b173 executable, or copy it to <data dir>/minecraft.jar (docs/RUNNING.md)
```

The message for a missing `resources/` directory reads the same way.

## 4. Or point at them

You can leave the files where they are and give their paths instead:

```sh
mc_b173 --jar ~/Games/b1.7.3/minecraft.jar --resources ~/Games/b1.7.3/resources
```

| Flag | Meaning | Default |
|---|---|---|
| `--data-dir DIR` | options, `saves/`, `screenshots/`, `stats/` | the table above |
| `--jar FILE` | your Beta 1.7.3 `minecraft.jar` | `minecraft.jar` next to the AppImage/executable, else `<data dir>/minecraft.jar` |
| `--resources DIR` | the `resources/` directory | `resources/` next to the AppImage/executable, else `<data dir>/resources` |
| `--username NAME` | the player name (names the stats file) | vanilla's `Player<millis % 1000>` |
| `--world NAME` | open `saves/NAME`, or create it, skipping the title screen | title screen |
| `--seed SEED` | the new world's seed (with `--world`) | random |
| `--import DIR` | copy a vanilla Beta 1.7.3 save into `saves/`, then open it. A world with a block or item id Beta 1.7.3 does not have is refused, and nothing is copied (PLAN_v4 D5) | — |
| `--vsync on\|off` | `on` waits for the display's refresh (no tearing, the frame rate capped at the display's). `off`, the default, runs as the original game does (below) | `off` |
| `--script FILE` | the scripted-input dev mode (`crates/mc-client/README.md`) | — |

Without `--jar`/`--resources`, each file is looked for in this order: next to
the game (section 2), then in the data directory (`--data-dir` when it is
given, else the default of section 3), then the development fallbacks (below).
So with nothing next to the game, `mc_b173 --data-dir /tmp/mc` looks for
`/tmp/mc/minecraft.jar` and `/tmp/mc/resources`. Explicit paths always win and
are used as given (a relative one is relative to the current directory). The
two start-up lines (`mc_b173: minecraft.jar: <path>`, `mc_b173: resources:
<path>`) show which paths were chosen.

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

A build from this repository falls back to the developer's files when neither
the game's directory nor the data directory has them:

- the jar of the vanilla reference checkout,
  `.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar` in the
  workspace the binary was built from (`docs/REFERENCES.md`);
- the `resources/` of the Prism Launcher (Flatpak) instance named `b1.7.3`,
  `~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/minecraft/resources`.

These paths are used only when they exist, so a packaged build on another
machine never depends on them.

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
