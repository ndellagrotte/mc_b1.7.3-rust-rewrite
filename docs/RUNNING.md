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
  under `libraries/com/mojang/minecraft/b1.7.3/minecraft-b1.7.3-client.jar`
  (rename it or point `--jar` at it).
- From an old `.minecraft/bin/minecraft.jar` that you kept.

A modded jar works if `terrain.png`, `gui/`, `lang/` and the other resources
are where vanilla puts them. Texture packs are not supported (deferred, PLAN_v4 D3).

**`resources/`.** The game reads only these folders: `sound/`, `newsound/`
(effects), `music/`, `newmusic/` (music) and `streaming/` (records), with
`.ogg` and `.mus` files. Old launchers downloaded them into
`.minecraft/resources/`. A Prism Launcher b1.7.3 instance has them in
`<instance>/minecraft/resources/` after its first start. An empty directory
works (the game is then silent); a missing one is an error.

## 2. Put them in the data directory

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
archive:

```sh
./mc_b173
```

If a file is missing, the game stops and prints the path it tried, for
example:

```text
mc_b173: cannot read minecraft.jar at /home/you/.local/share/mc_b173/minecraft.jar (No such file or directory (os error 2)); give your own Beta 1.7.3 client jar with --jar <path>, or without --jar copy it to <data dir>/minecraft.jar (docs/RUNNING.md)
```

## 3. Or point at them

You can leave the files where they are and give their paths instead:

```sh
mc_b173 --jar ~/Games/b1.7.3/minecraft.jar --resources ~/Games/b1.7.3/resources
```

| Flag | Meaning | Default |
|---|---|---|
| `--data-dir DIR` | options, `saves/`, `screenshots/`, `stats/` | the table above |
| `--jar FILE` | your Beta 1.7.3 `minecraft.jar` | `<data dir>/minecraft.jar` |
| `--resources DIR` | the `resources/` directory | `<data dir>/resources` |
| `--username NAME` | the player name (names the stats file) | vanilla's `Player<millis % 1000>` |
| `--world NAME` | open `saves/NAME`, or create it, skipping the title screen | title screen |
| `--seed SEED` | the new world's seed (with `--world`) | random |
| `--import DIR` | copy a vanilla Beta 1.7.3 save into `saves/`, then open it. A world with a block or item id Beta 1.7.3 does not have is refused, and nothing is copied (PLAN_v4 D5) | — |
| `--vsync on\|off` | `on` waits for the display's refresh (no tearing, the frame rate capped at the display's). `off`, the default, runs as the original game does (below) | `off` |
| `--script FILE` | the scripted-input dev mode (`crates/mc-client/README.md`) | — |

Without `--jar`/`--resources`, the defaults are taken from `--data-dir` when it
is given: `mc_b173 --data-dir /tmp/mc` looks for `/tmp/mc/minecraft.jar` and
`/tmp/mc/resources`. Explicit paths are used as given (a relative one is relative
to the current directory).

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

A build from this repository falls back to the developer's files when the data
directory has none:

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
files only), `credits/`, `NOTICE`, `LICENSE` and this file. The browser build
is Phase 5.

## Saves on disk

Worlds are vanilla McRegion saves (`saves/<name>/level.dat`, `region/r.X.Z.mcr`)
and open in the vanilla client. While the game runs, saves are written by a
background thread. A region file is updated by rewriting only the sectors that
changed, through a small journal file next to it (`.mc_journal-r.X.Z.mcr`).
If the game or the computer stops in the middle of a write, the next start
finishes the update from the journal, so every file is either the old or the new
version (`crates/mc-client/src/storage.rs`). Quit through the game menu (or close
the window, which does the same) so the last save is written.
