# World export (WP-4.5)

Written 2026-10-09 for the owner's review. PLAN_v4 §5, Phase 4, row 4.5
asks for an "Export" action, and for worlds written by the Rust game to be
opened in the Prism client and checked (D5: "Export is planned, low priority:
an 'Export' action plus validation in the Prism client").

Branch `wp-4.5`, forked from `d52ac6b` (the Gate 3 review). The code is in
`crates/mc-client/src/export.rs` and `crates/mc-client/src/gui/menu/export_world.rs`.
The select-world screen and the `export_world` dev action call it.

**Verdict: done.** Worlds the Rust game writes open in the original Beta
1.7.3 client, at the saved position, with what the Rust game put in them:

| # | Check | Result | Evidence |
|---|---|---|---|
| E1 | A world the Rust game created (seed 123): played for 482 ticks (a block dug with the mouse, one placed with the mouse, a gold wall, glowstone, a sign, a chest, a workbench and torches placed with `set_block`, items given), then exported in game | **pass** | `a1/03_before_export.png`, `a1/export_report.json`: 7 files, 4 region files, 733 chunks checked |
| E2 | E1 opened in the vanilla client (the reference `minecraft.jar`, Prism's Java 8) | **pass** | `vanilla/r01`–`r03`: listed as `rustworld`; the view matches the Rust screenshot (same blocks, same hotbar: 15 cobblestone, a diamond sword, 12 torches); F3 shows x 89.5, y 67.62000000476837, z 43.5, the Rust dump's position exactly |
| E3 | `testWorld` imported, played (walked, a diamond block, a torch and a sign placed, 3 diamonds given), exported in game | **pass** | `b/02_before_export.png`, `b/export_report.json`: 7 files, 4 region files, 784 chunks |
| E4 | E3 in the vanilla client | **pass** | `vanilla/t01`–`t04`: the same view as the Rust screenshot; F3 x 9.52309703383433, y 65.62000000476837, z −58.1552604391517 (Rust's to the last digit); the chest at (9, 64, −61) opens with **2 planks and 4 sticks** |
| E5 | A world exported with the player in the nether | **pass** | `n/export_report.json`: 11 files, 8 region files (4 in `DIM-1/region`), 750 chunks; `vanilla/n01_netherworld_f3.png`: vanilla starts in the nether at x 4.0, y 68.62000000476837, z −11.5, the exported position |
| E6 | Every exported chunk read by the vanilla loaders (`tools/oracle`'s `harness.WorldReadDir`, the Tier S cross-read) and compared with the Rust export's digests | **pass** | `oracle/summary.txt`: 733 + 784 + 397 + 353 = **2,267 chunks, 0 differing** (blocks, data, sky light, block light, height map); seed, time, name, version 19132 and the player match |
| E7 | Round trip: the vanilla client saved E1 and E3 ("Save and quit to title"); both imported back into the Rust game and exported again | **pass** | `d/*.json`: the Rust game arrives at the positions vanilla saved; both exports pass validation |
| E8 | The UI: Export... on the world list, the export screen, the success dialog, the refusal for an existing folder | **pass** | `a2/01`–`08` |
| E9 | Refusals: a block id 200, a truncated region file | **pass** | `c/bad_refused.png`, `c/trunc_refused.png`; no `exports/` folder is created |
| E10 | Workspace checks | **pass** | §5 |

Screenshots, dumps, reports and logs are under `build/screens/wp-4.5/`
(git-ignored). The scripts that made them are in `build/screens/wp-4.5/scripts/`,
and all paths below are relative to `build/screens/wp-4.5/`. The vanilla
screenshots are the vanilla client's own F2 captures, copied from the isolated
home. None of them is committed (D8).

## 1. What the export does

The native save format already is vanilla McRegion (D5). The export is
therefore a checked copy: nothing is converted or stripped. `session.lock` is
copied too, and vanilla rewrites it when it opens the world
(SaveHandler.java:41-51).

`export::export_world(saves_dir, name, target)`:

1. **Target.** `<target>/<world folder>` must not exist, or must be an
   empty folder. The export never overwrites or merges. A target inside the
   world folder is refused.
2. **Read.** The world folder is read into memory. The native storage's
   write temporaries (`.mc_tmp-*`) are skipped. A link to a directory is
   refused, because a link to a parent would never end.
3. **Validate**, on the in-memory copy:
   - D5's import scan (`mc_sim::save::import::validate`): `level.dat` (or
     `level.dat_old`, as `SaveHandler.loadWorldInfo` falls back) decodes,
     and every block and item id in it and in every chunk of `region/` and
     `DIM-1/region/` is one Beta 1.7.3 defines;
   - then the export's stricter checks: the save version is McRegion's
     19132 (pre-McRegion worlds stay deferred, D3), `SaveOldDir.loadWorldInfo`
     reads it, every chunk a region header lists reads back, and every such
     chunk loads into a `Chunk` through the simulation's `McRegionChunkLoader`.
     A truncated region file passes the import's scan, because vanilla's
     reader returns `null` for a chunk past the end and regenerates it. The
     export refuses it, because the exported world would silently lose those
     chunks.

   This yields the world's digest (`export::WorldDigest`). Per chunk it holds
   blake3 of the five arrays of the loaded `Chunk` (the Tier S `chunk_digest`
   with `"stage":"loaded"`, tests/golden/README.md) and blake3 of its stored
   NBT tree re-encoded in key order, so entities and tile entities count too.
   It also holds the `level.dat` tree's digest and every file's length and
   blake3.
4. **Copy** through `std::fs`, in the client and never in `mc-sim` (D9),
   into a hidden staging folder `<target>/.mc_export-<world>-<pid>`. Each
   file is synced to disk.
5. **Re-read** the staging folder from disk and compare it with the source
   (`export::compare`): the same files with the same bytes, and the same
   digest from the same loaders, chunk by chunk. A failure names the first
   differing file or chunk and array.
6. **Publish**: the staging folder is renamed to `<target>/<world>`. Any
   failure after step 4 removes the staging folder. A refused or failed
   export leaves nothing behind (E9, and the unit tests).

`export::export_from_game` is what the screen and the dev action call. When
the world being exported is the one open, it first saves that world
(`World.try_save_world(true)`, `saveWorld(true, null)`: `level.dat` with the
player, then every chunk). It then flushes the region-file cache
(`SaveConverterMcRegion.flushCache`) and runs `export_world` on
`<data-dir>/saves`. From the world list, the world is closed and its files
are complete.

**UI.** The select-world screen's new **Export...** button sits in the
title band's top left corner (4, 6, 70 × 20). It mirrors Import..., which is
top right, and it is enabled while a world is selected. It opens **Export
world**, which has the world's name, a folder box defaulting to
`<data-dir>/exports` (typed or pasted; `~/` is the home directory; a long
path shows its end), Export and Cancel. Success opens **World exported**:
the path (wrapped at `/`), the number of files, region files and chunks read
back, the player's position, and a hint to copy the folder into
`.minecraft/saves`. Done returns to the world list. A refusal or failure
opens **Export refused** or **Export failed**, which names the problem (the
first unknown block or item id and its chunk, the unreadable file and chunk,
the folder in the way, …) and says "Nothing was exported.". Back returns to
the export screen. Both dialogs are drawn like the import's refusal dialog.

**Dev action.** `{"action": "export_world", "name": <world folder>,
"path": <target folder>, "report": <json file, optional>}`. The report holds
`{"ok": true, "report": {path, files, bytes, regions, chunks, digest}}` (every
file's and chunk's blake3), or `{"ok": false, "error": …}`.

**Cost.** The export runs inside the click, as the import does. In the dev
build (opt-level 1) it took about 1.0 s for E1 in game (save, digest of 733
chunks, copy, re-read and second digest), 0.56 s for the 784 chunks of E3,
and 0.35 s from the world list for the 733 chunks of A2. That is one stalled
frame.

## 2. Verification in the vanilla client

### 2.1 Launch recipe

The vanilla client is the reference jar, started directly with Prism's
Java 8 and an isolated home. The real Prism instance and the user's
`.minecraft` are never touched. `Minecraft.main` takes the user name and
the session id (Minecraft.java:1515-1527: `args[0]` the name, `args[1]`
the session id, default `"-"`). `getAppDir` puts the data in
`<user.home>/.minecraft` (Minecraft.java:359-387).

```sh
VHOME=/tmp/wp45-vanilla-home                      # any empty folder
mkdir -p $VHOME/.minecraft/saves
cp -r ~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/minecraft/resources \
      $VHOME/.minecraft/                           # sounds; vanilla's resource download is dead
printf 'music:0.0\nsound:0.0\nviewDistance:1\nguiScale:2\n' > $VHOME/.minecraft/options.txt
cp -r <the exported world> $VHOME/.minecraft/saves/
VHOME=$VHOME sh build/screens/wp-4.5/scripts/launch_vanilla.sh
```

`launch_vanilla.sh` runs:

```sh
J=~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/java/jre-legacy/bin/java
B=.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin
cd "$VHOME"
env -u WAYLAND_DISPLAY DISPLAY=:0 HOME="$VHOME" "$J" -Xmx1G -Duser.home="$VHOME" \
  -Djava.library.path="$B" -cp "$B/minecraft.jar:$B/lwjgl.jar:$B/lwjgl_util.jar:$B/jinput.jar" \
  net.minecraft.client.Minecraft ExportCheck -
```

The log shows only the expected noise: "Failed to initialise controllers"
(jinput's plugin jar is not on the class path, and vanilla catches it), the
dead `s3.amazonaws.com/MinecraftResources/` and `login.minecraft.net`, and
two non-sound files of `resources/` it does not know. There is no chunk or
save error in any of the four runs.

**Driving it** (no person at the window). `xdotool` on XWayland works once
the window has the X input focus. `windowraise` and `windowfocus` the
vanilla frame, which is the window named `Minecraft` whose PID is the
java's. Move with `mousemove --window`, then `mousedown`/`mouseup` 1 with
about 0.1 s between them. `keydown`/`keyup` F2 makes vanilla's own
screenshot in `<VHOME>/.minecraft/screenshots/`. Window pixels at 854 × 480
(GUI scale 2): title → Singleplayer (427, 236); the only world row (400,
195); Play Selected World (267, 395); Back to game (427, 155); Save and quit
to title (427, 347). Focus loss pauses the game (vanilla opens the Game
menu). On this desktop other sessions' windows took the focus every few
seconds, so `scripts/vdo.sh` refocuses and retries before each sequence,
and a click after a refocus is sometimes needed twice. F3's coordinates are
drawn over the Game menu too, so a paused screenshot still shows the
position.

### 2.2 Results

- **E2, `rustworld`** (`vanilla/r01_select_rustworld.png`,
  `r02_rustworld_loaded.png`, `r03_rustworld_f3.png`). The list shows
  `rustworld (10/9/26 2:23 PM, 1.3 MB)`. Play loads it at once (no
  "Generating level"). The view is the Rust screenshot
  `a1/03_before_export.png` in every block: the 5 × 3 gold wall
  with glowstone on top, the sign "WP-4.5 / exported / from Rust / seed 123",
  the chest and workbench with a torch each, the cobblestone the mouse placed
  into the hole it dug (lower right), the grass bank, the trees and the
  beach. The hotbar shows 15 cobblestone, the diamond sword and 12 torches.
  F3: x 89.5, y 67.62000000476837, z 43.5. Then "Save and quit to title"
  wrote the world without complaint (E7).
- **E4, `testWorld`** (`t01`–`t04`). The player is where the Rust game
  saved it (F3 to the last digit). The view matches `b/02_before_export.png`:
  the diamond block with a torch and the sign "testWorld / exported / from
  Rust" behind the owner's chest, and 3 diamonds in hotbar slot 5. A right
  click opens the chest: 2 planks in slot 0 and 4 sticks in slot 1, as in
  the original save, after an import, a Rust save and an export.
- **E5, `netherworld`** (`n01_netherworld_f3.png`). The world was exported
  five ticks after the dev `use_portal`, so `level.dat` has `Dimension` −1.
  Vanilla opens it in the nether (netherrack, fires, the portal overlay) at
  x 4.0, y 68.62000000476837, z −11.5. An earlier export of the same seed
  had the player back in the overworld, standing in the arrival portal (the
  dev action's round trip, Gate 3 O2). Vanilla opened that one too, but this
  desktop's focus changes kept pausing it before the portal could fire. So
  the nether data is shown by E5 and by the cross-read (E6), not by walking
  through a portal.

### 2.3 Cross-read with the vanilla loaders (E6)

The vanilla client shows only what is near the player. Every chunk was
therefore also read by the vanilla save code: `tools/oracle`'s
`harness.WorldReadDir read <parent>` opens `<parent>/world` with
`World(SaveOldDir, "harness", 0, null)` and prints, per present chunk, the
`chunk_digest` of the `Chunk` that `McRegionChunkLoader.loadChunk` builds,
plus `world_info` and the tile entities (tests/golden/README.md, Tier S).
`scripts/xread.sh` runs it (oracle built by `tools/oracle/build.sh`).
`scripts/compare_xread.py` compares it with the Rust export report.

| Export | Dimension | Chunks (vanilla / Rust) | Differing | Notes |
|---|---|---|---|---|
| `rustworld` | 0 | 733 / 733 | 0 | 51 tile entities incl. the placed chest and sign |
| `testWorld` | 0 | 784 / 784 | 0 | 54 tile entities; `Chest` at (9, 64, −61) with (slot 0, id 5, 2) and (slot 1, id 280, 4) |
| `netherworld` | 0 | 397 / 397 | 0 | |
| `netherworld` | −1 | 353 / 353 | 0 | `DIM-1/region` read as the world's `region/`, next to the export's `level.dat` (`WorldReadDir` reads only `region/`) |

In every case the seed (123), the world time, the level name, the version
(19132) and the presence of the player match the report.

## 3. Rust-side smoke runs

All runs use `target/debug/mc_b173 --data-dir <fresh dir> --username Exporter
--script <file>`, window 854 × 480, GUI scale 2. No run logged a panic, or a
warning or error other than the expected refusals.

- **A1** (`scripts/a1.json`, `a1/`): `--world rustworld --seed 123`. Spawn
  (random spawn search) at (89.5, 67.62, 43.5). The structure was placed
  with `set_block`. With the look at pitch 60, the left button held 25 ticks
  dug the sand at (89, 65, 42) (`01_dug.png`; mouse-over (89, 65, 42) → (89,
  65, 41); the sand was picked up). Then the right button with cobblestone
  placed one there (stack 16 → 15; mouse-over back on (89, 65, 42)). At tick
  480 a screenshot was taken, then `export_world` ran
  (`export_report.json`), then `quit`.
- **A2** (`scripts/a2.json`, `a2/`), the UI on A1's data folder:
  `01` the list with Export... disabled (nothing selected); `02` the row
  selected and Export... enabled; `03` the export screen with the default
  folder (`…/data-a/exports`); `04` "World exported" with the path, "7
  files, 4 region files and 733 chunks were read back and match the world.
  The player is at 89.5, 67.6, 43.5."; `05` Done → the list; `06` a second
  export to the same folder → "Export refused: The folder … already exists
  and is not empty …  Nothing was exported."; `07` Back → the export
  screen; `08` Cancel → the list.
- **B** (`scripts/b.json`, `b/`): `--import testWorld`. On arrival the
  position equals `level.dat` (as at Gate 3). The player walked south 12
  ticks, a diamond block, a torch and a sign were placed, and 3 diamonds were
  given. At tick 300 `export_world` ran.
- **C** (`scripts/make_bad_worlds.py`, `scripts/c_*.json`, `c/`): two
  copies of the A1 export, each alone in a data folder, exported through the
  UI:
  - `BadBlocks` (id 200 at local (5, 70, 5) of chunk (0, 0)) → "Export
    refused: Block id 200 is not a Beta 1.7.3 block. First found in chunk
    (0, 0) (Blocks at local x 5, y 70, z 5). Nothing was exported."
  - `Truncated` (`region/r.0.0.mcr` cut before its last used sector) →
    "Export refused: Cannot read region/r.0.0.mcr: chunk (16, 13): listed in
    the header but unreadable (past the end of the file, or a bad length or
    version) Nothing was exported."

  In both runs the data folder has no `exports/` afterwards.
- **D** (`scripts/d_*.json`, `d/`): the vanilla-saved `rustworld` and
  `testWorld` (E7) imported with `--import`. The Rust game arrives at (89.5,
  67.62, 43.5) and at (9.523…, 65.62, −58.155…), and `export_world` passes.
- **N** (`scripts/n.json`, `n/`): `--world netherworld --seed 123`, dev
  `use_portal`, export five ticks after arrival.

## 4. Unit tests

13 new tests in `mc-client` (`export::tests`, `gui::menu::export_world::tests`,
`script::tests::parses_the_wp_4_5_export_action`), all in temporary folders:

- a good world (two overworld chunks, one nether chunk) exports. The copy
  has the source's bytes, nothing else is left in the target, the digest
  recomputed from the copy equals the report's, and the loaded `blocks`
  array of chunk (0, 0) hashes to the expected bytes;
- block id 200 in chunk (1, 0) → `Refused(UnknownBlock { id: 200, chunk (1, 0) })`,
  nothing written;
- a region file cut after the first chunk's sector: the import's
  `validate` accepts it (vanilla would regenerate the lost chunk), and the
  export refuses it with `Corrupt { region/r.0.0.mcr, "chunk (1, 0): listed
  in the header but unreadable …" }`;
- a missing `level.dat` → `Unreadable { level.dat, missing }`;
- a link to a directory inside the world → refused, nothing written;
- a non-empty target folder (a file in it, or a file in its place) is
  refused and left untouched. An empty target folder is used. A second
  export to the same place is refused. A target inside the world and a
  world name with `..` are refused;
- `compare` names the first difference: `level.dat`, a chunk's `blocks`,
  its `nbt`, a missing chunk;
- the reference `testWorld` when present (it is never committed): 100+
  chunks, byte-identical copy;
- the dialog texts (success with the wrapped path and the player position,
  each refusal kind), `wrap_path`, and the parsing of `export_world`.

## 5. Workspace checks

Run in the worktree after the last code change:

| Check | Result |
|---|---|
| `cargo build -p mc-app` | pass |
| `cargo test --workspace` | pass: 1,126 tests (1,113 at Gate 3 plus the 13 new ones in `mc-client`), 0 failed |
| `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` | clean |
| `cargo fmt --all --check` | clean |
| wasm32 check | not needed: `mc-core`, `mc-nbt` and `mc-sim` are untouched |

## 6. Changes outside the brief's file list

- `crates/mc-client/Cargo.toml` (and `Cargo.lock`): `blake3.workspace = true`.
  `blake3` was already a workspace dependency (mc-golden's digests). The
  export uses it so its chunk digests are the Tier S ones and can be
  compared with the oracle's lines directly (§2.3).
- `crates/mc-client/src/gui/menu/mod.rs`: `pub mod export_world;` and one
  row of the module table.
- `crates/mc-client/src/lib.rs`: `pub mod export;` and a doc line (additive,
  as the shared-file rule allows).

`mc-sim` is not changed. The export uses its public API: `import::validate`,
`SaveOldDir`, `McRegionChunkLoader`, `RegionFile`, `World::new_from_save`,
`golden::ChunkArrays`, `chunk_loader::discard`, `World::try_save_world`.
No `field_NNN`/`func_NNN` was touched, so `docs/mappings.md` is unchanged.

## 7. Known limits and owner questions

1. **How the Prism check was done.** I did not open the Prism launcher
   itself: its GUI is a Qt/Flatpak application that cannot be driven
   reliably from here. Its instance must also stay untouched. The same jar
   version (the reference `minecraft.jar`, which Prism's `b1.7.3` instance
   also runs), the same Java 8 and the same LWJGL 2 natives were launched
   directly (§2.1). To finish the check in Prism yourself, copy an export
   (e.g. `<data-dir>/exports/<world>`) into
   `~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/minecraft/saves/`,
   start the instance, and pick the world under Singleplayer. Use a world
   name that is not already there, and keep a backup of `saves/` if it
   matters.
2. **The default folder** is `<data-dir>/exports`. Should it default to the
   Prism instance's `saves/` instead, when that exists? It would save a copy
   step, but it would write into another program's folder.
3. **Typed path, no file dialog**: as for the import (Gate 3 question 6).
4. **The copy runs inside the click.** That is about 1 s for a 3 MB world in
   the dev build (§1). A large world would stall longer. A background task
   with a progress screen would be WP-4.3-style work.
5. **A stale staging folder**: if the process dies between steps 4 and 6, a
   hidden `.mc_export-<world>-<pid>` folder stays in the target. A later
   export from another process does not clean it up.
6. **Walking through a portal in vanilla** with the exported nether was not
   shown (§2.2, E5): this desktop's focus changes paused the game. The
   nether chunks were checked by the cross-read instead.
