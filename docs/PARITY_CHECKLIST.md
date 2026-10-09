# Vanilla parity checklist (WP-4.1)

Written 2026-10-09 for the owner by the WP-4.1 transcriber, revised the
same day after review (§12 lists what the review changed). PLAN_v4 §5
defines WP-4.1 as "a side-by-side checklist against the Prism client: every
block rendered, every screen, every mob, every sound trigger, on `testWorld`
and fixed seeds". Branch `wp-4.1`, off `phase-4` before WP-4.2 (`d52ac6b`, the
Gate 3 tip; nothing here depends on WP-4.2).

**The real client was driven.** The original obfuscated b1.7.3 `minecraft.jar`
ran with the Prism instance's Java 8 (1.8.0_202) on a nested X server, was
driven through its own menus with `xdotool`, and took its own F2 screenshots.
Every item below has a vanilla screenshot or a vanilla sound log next to the
Rust one, except the few marked *owner to verify* (§10).

**Verdict.** The Rust client matches vanilla almost everywhere it was
compared: 137 screenshot pairs (the world views, the menu and in-game GUI
tours, the loading screen); the options, video settings, controls, rename,
delete and create-world screens byte for byte (mean difference 0.0); the
fog colour and density of all four render distances to the byte at the
sampled points; the container screens, the inventory and the sign editor
within the noise of animated textures; and 33 sound stations with the
same sound names, volumes and pitch ranges (the pitches are random in
both). **8 defects and 2 to confirm** (§2) were found, 2 of them the open
Gate 3 defects D1 and D2; none breaks play. The most visible is P3: on the
Game Over screen Rust draws neither the hand nor any entity or tile
entity. Most remaining differences are run-to-run noise that vanilla shows
against itself (random ticks, mob AI, cloud offset, how far chunk loading
has got at the moment of the screenshot); §9 lists them so they are not
mistaken for bugs.

Contents: 1 Method · 2 Defects · 3 Blocks · 4 Mobs and entities ·
5 Screens · 6 Sounds · 7 testWorld, nether, night · 8 Render distance, F3,
frame rate, options, folders · 9 Differences that are not Rust bugs ·
10 Not verified / owner to verify · 11 Reproducing · 12 Review changes

---

## 1. Method

### 1.1 The two clients

| | Vanilla | Rust |
|---|---|---|
| Program | `.reference/…/jars/bin/minecraft.jar` (the original obfuscated b1.7.3 jar), LWJGL 2 jars and natives from the same directory | `target/debug/mc_b173` at `d52ac6b` (dev profile) for every pair; `target/release/mc_b173` (same commit) for the repeat runs of §2 P10 and §8.3 |
| Runtime | Prism's Java 8: `~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/java/jre-legacy/bin/java` | — |
| Data | a fake home, `-Duser.home=<scratch>/vhome` (so `Minecraft.getMinecraftDir` is `<scratch>/vhome/.minecraft`, Minecraft.java:351-388; nothing touched `~/.minecraft` or the Prism instance); the Prism instance's `resources/` copied into it | `--data-dir build/screens/wp41/rdata` (and `rdata_menu`, `rdata_exp`) |
| Display | a nested rootful Xwayland, `Xwayland :47 -geometry 854x480` (NVIDIA GLX), so the vanilla window can hold focus and the pointer without touching the desktop | a second nested Xwayland `:48`, X11 backend (`env -u WAYLAND_DISPLAY DISPLAY=:48`) |
| Window | 854 × 480 (Minecraft.java:1488), GUI scale auto = 2 | 854 × 480, GUI scale 2 |
| Input | `xdotool` on `:47` (clicks at window pixels = 2 × GUI pixels; keys held 120 ms because LWJGL's `Keyboard.isKeyDown` is polled once a frame) | `--script` (README "Scripted-input dev mode") |
| Screenshot | the game's own F2 (`ScreenShotHelper`, the back buffer); `ffmpeg -f x11grab` for the loading screen and state checks | the `screenshot` action |
| Options | defaults (one fresh `options.txt`, §8.4) | defaults |
| Player name | `parity` (`Minecraft.main` arguments `parity -`) | `--username parity` |

The vanilla client is started through `ParityLaunch`
(`build/screens/wp41/scripts/java/ParityLaunch.java`), which does exactly what
`Minecraft.startMainThread` does (Minecraft.java:1482-1510) with two test
changes, both mirroring the Rust script mode:

1. `Minecraft.sndManager` (obfuscated `B`) is replaced, before the game
   thread starts, by a subclass of `SoundManager` (`yo`) that writes every
   `playSound`, `playSoundFX` and `playStreaming` call (name, position,
   volume, pitch, world time) to a log and then calls the real method.
   `startGame` initialises it as it would the original.
2. Once `startGame` has made the `MouseHelper` (`C`), it is replaced by a
   subclass whose `mouseXYChange` drains and drops LWJGL's deltas (the Rust
   script mode also ignores real input). Without it, pointer motion on the
   nested server turned the camera after the world loaded (seen in the
   first batch: rotated paintings and zoo views).

Because these classes live in the jar's unnamed package, the JVM refuses
them next to the jar's signed classes; `ParityLaunch` therefore runs on a
local copy of `minecraft.jar` with `META-INF/MANIFEST.MF`, `MOJANG_C.SF`
and `MOJANG_C.DSA` removed. The class files are unchanged. The copy lives
in the scratch directory and is never committed (D8). The first batch ran
the unmodified `net.minecraft.client.Minecraft` main class; its block views
agree with the harness runs (its paintings and zoo views were turned by the
pointer, point 2).

### 1.2 The worlds

`cargo run --release -p mc-golden --example parity_world -- build/screens/wp41/worlds`
(committed, `crates/mc-golden/examples/parity_world.rs`) writes, through the
simulation's own save path (`World::try_save_world` on a `SaveOldDir`):

- `parity`: a flat 256 × 256 area (chunks −8..7 on both axes, saved and
  marked populated; seed 123 beyond it), noon, no rain or thunder for
  500,000 ticks. It holds 27 galleries of 8 cells (every block id 1-96
  except 36, the moving piston, and the metadata variants named in §3),
  signs with text, 3 painting walls with all 25 motives, 3 rows of fenced,
  roofed stalls with every mob variant, a giant, a ghast in a glass box and
  a squid in a glass tank, dropped items, minecarts and boats, 5 double
  chests with every block item and item, GUI stations and 24 sound
  stations. The player wears diamond armour and holds a sword, pickaxe,
  bow, torches, signs, cobblestone, a bucket, record 13 and TNT.
- `parity_nether`: the seed's nether around (0, 0), generated and saved by
  the simulation, the player in dimension −1. Chunks −3..3 are loaded and
  only −3..2 saved, so every saved chunk was populated (a chunk is
  populated once its +x, +z and +x+z neighbours exist, ChunkProvider.java:50-66).
- `parity2` (added after review): the same flat ground with the variants
  the first pass lacked: 10 more galleries (`g28`-`g37`, §3), three more
  stall rows (`zoo4`-`zoo6`) and a creeper (§4), and 8 more sound stations
  (§6). A separate world, so the screenshots taken on `parity` stay valid.
- `views.json`: 103 camera points (79 in `parity`, 22 in `parity2`, 2 in
  `parity_nether`).

`build/screens/wp41/scripts/mkviews.py` copies the base world once per view
and patches only `level.dat` (`Player.Pos`, `Rotation`, `Time`, and for a
few views the hotbar, `raining`, `thundering`, or no armour). `mktw.py`
does the same for `testWorld`. Both clients open **the same bytes**:
vanilla through Singleplayer → the world's row → Play Selected World, Rust
with `--world`. Each view's screenshot is taken about 6 s (120 ticks) after
the world appears. Two runs of the example write identical region files.

**Each vanilla run gets its own directory name.** Vanilla's
`RegionFileCache` is static: it keeps every region file it opened, keyed by
its path, until 256 are open (RegionFileCache.java:14-44); only deleting a
world from the list flushes it (`GuiSelectWorld.deleteWorld` →
`flushCache`, GuiSelectWorld.java:120, SaveConverterMcRegion.java:49-50).
The first harness copied every view to `saves/<view>`, so a second run of
the same view in one vanilla process read and wrote the first run's region
files (moved away and later unlinked), not the fresh copy: `vsaved/death`
and `vsaved/g01` came back with region files byte-identical to their input
while `level.dat` had moved on. `vbatch.py` now copies each run to
`saves/<view>_<n>` with a counter kept in the fake home (the world list
still shows `LevelName`, the view's name). Every vanilla view that had
run more than once in one process was taken again with the fixed harness:
`death`, `st_sign`, `st_bed`, `snd_lever`, `snd_button`, `snd_piston`,
`snd_dispenser`, `snd_tnt` (the in-game menu tour and the fog station ran
on a stale `g01` too; their screens do not depend on the chunks). §8.5's
vanilla region-size figures were taken with the old harness and may come
from stale files; the `level.dat` key comparison does not depend on them.
The Rust client is a new process per run and has no such cache.

### 1.3 Comparing

`scripts/pngtool.py` (no PIL) decodes both PNGs, writes the pair stacked
(`cmp/<view>_sbs.png`, vanilla on top) and a ×4 difference image
(`cmp/<view>_diff.png`), and reports the mean of the per-pixel maximum
channel difference and the share of pixels differing by more than 24.
`cmp/all_stats.txt` lists every pair. Static views with matching camera,
time and content come out at 2-4 mean (clouds, animated textures, the HUD
hint and 1-pixel edges of 3D item icons); screens with no world behind
them come out at 0.0-1.7. Every pair was also looked at. Sound logs are
compared per station by `scripts/sndcmp.py` (`cmp/sound_compare_full.txt`).

All paths below are relative to `build/screens/wp41/` (git-ignored). The
vanilla screenshot of view *v* is `vanilla/v.png`, the Rust one `rust/v.png`,
the stacked pair `cmp/v_sbs.png`.

---

## 2. Defects

Numbered P1-P10 (P9 and P10 are to confirm). P1 and P2 are Gate 3's D1
and D2, still open. Each has a Rust repro (the script files are in
`rust/scripts/`, the worlds made by §11's commands).

**P1. Third-person held items are not drawn (Gate 3 D1).** Vanilla's
skeletons hold a bow, zombie pigmen a gold sword, and the player in F5 the
item in hand. Rust draws none of them.
Evidence: `cmp/zoo1_sbs.png`, the zoomed crop `cmp/zoo13.png` (vanilla
skeleton with bow, pigman with sword; Rust empty-handed), `cmp/m20_third_person_sbs.png`
(vanilla player holds the sword in F5). Repro: `scripts/rbatch.sh
rust/scripts/view.json zoo1`; F5 in `rust/scripts/menu_ingame.json`.
Cause, as recorded at Gate 3: nothing inserts `render::held_item::HeldItemHook`.

**P2. Achievement icons draw blocks flat (Gate 3 D2).** Vanilla's
achievement map draws block icons (workbench, log, furnace) as lit 3D cubes;
Rust draws the flat tile. Evidence: `cmp/m12_achievements_sbs.png`.
Repro: `rust/scripts/menu_ingame.json` (Escape, Achievements).

**P3. A dead player's view draws no hand, no entities and no tile
entities.** On the Game Over screen vanilla still draws the world's
entities (the player's dropped inventory lies in front of the camera: a
log, a cake, …), the tile-entity specials and the arm (red under the
screen's gradient); Rust draws the terrain only. The dropped items exist
in Rust: `rdata/saves/death` holds 36 `Item` entities, vanilla's
`vsaved/death` 39 (`python3 scripts/ents.py rdata/saves/death`). Cause:
once `deathTime` passes 20 the player leaves `loadedEntityList`
(EntityLiving.java:155-158, World.java:1223-1232), but vanilla keeps
`mc.renderViewEntity` and draws from it: `RenderGlobal.renderEntities`
caches and positions from `mc.renderViewEntity` (RenderGlobal.java:253-266),
`renderWorld` passes its position to `renderEntities` (EntityRenderer.java:501) and
`renderHand` uses `thePlayer`. Rust looks the view entity up in the world:
`world.entity(view_id)` in `crates/mc-client/src/render/mod.rs:701-716`
returns before any entity or tile-entity pass when it is missing (the
comment there says this happens only between a portal's two halves; it is
also the whole time the Game Over screen is up), the tile-entity and
render-manager caches are filled through `world.with_entity_mut(view_id, …)`
(:736-739), and `item_renderer` draws the hand through
`world.with_entity_mut(pid, …)` (`crates/mc-client/src/item_renderer/mod.rs:562-566`).
Fix direction (wave F): resolve the view through `Game::render_view()`
(the world's entity, else the shell's `detached_player`, `game.rs` module
docs "The player") in all three places. Evidence: `cmp/death_sbs.png`
(vanilla above, taken again with the fixed harness, §1.2). Repro: view
`death` (the player starts 40 blocks up without armour),
`python3 scripts/stations.py rust death`.

**P4. A dead player is not saved.** After dying and leaving (vanilla:
"Title menu"; Rust: the `quit` action, both `changeWorld1(null)`), vanilla's
`level.dat` holds the dead player (`Data.Player` with `Health` −17,
`DeathTime` 21, an empty inventory: `World.playerEntities` keeps a player
that died through `Entity.setEntityDead`, only `World.setEntityDead`
removes it, World.java:934-945), so the world reopens on the Game Over
screen. Rust's `level.dat` has no `Player` compound: `World::save_level`
(`crates/mc-sim/src/world/mod.rs:2013-2030`) finds no entity for
`player_entities[0]` and records `note_unimplemented`; the world reopens
with a fresh player at the spawn point. Evidence: `vsaved/death/level.dat`
vs `rdata/saves/death/level.dat` (read with `scripts/nbt.py`). Repro: as P3,
then read `level.dat`. The fix is in `mc-sim` (WP-4.6 territory).

**P5. F3's memory lines read zero.** Rust shows `Used memory: 0% (0MB) of
0MB` and `Allocated memory: 0% (0MB)`; vanilla shows the JVM heap (`5%
(354MB) of 6932MB`). `DebugSources::{max,total,free}_memory` are never set
(`crates/mc-client/src/debug.rs:24-43`). There is no JVM; the process's
own memory would be the faithful stand-in. Evidence: `cmp/f3_right.png`
(vanilla above), `rust/m18b_f3_nopopup.png`. Repro: `rust/scripts/f3only.json`.

**P6. The frame rate is capped at the display's refresh rate.** With the
default "Performance: Balanced" vanilla runs uncapped (F3: 242 fps on this
machine, `cmp/m18_f3_sbs.png`); b1.7.3 only sleeps for "Power saver" (40
fps, EntityRenderer.java:369-411). Rust runs at 60 fps (vsync, Bevy's
default present mode). Owner decision: an uncapped present mode for
Balanced/Max (Gate 3 question 11 asks for the same for measurements).

**P7. `portal.travel` plays after the new dimension has loaded.** Standing
in a portal, Rust logs `Toggling dimension!!` and 2.2 s later
`portal.travel` (`rust/logs/snd_portal_stay.log`): the sound is a
`ClientSound` handed to kira in `PostUpdate`, after the frame that blocked
on the world change. Vanilla calls `playSoundFX("portal.travel")` before
`usePortal()` (EntityPlayerSP.java:58-59), so the sound starts before the
loading screen. Low severity. The vanilla side was not driven to a full
travel (§10). Repro: view `snd_portal_walk`, `python3 scripts/stations.py
rust snd_portal_stay` (walk in for 0.6 s, stay 10 s).

**P8. One frame of untextured fluids after loading.** The first Rust frame
after the loading screen draws still water and lava as flat blue and red
and the sky as a dark band (`rust/m22_loading_c.png`); vanilla's first
world frame has textured lava (`vanilla/m22_loading_d.png`). The animated
tiles appear to be blank until the first `updateDynamicTextures` tick;
vanilla's atlas starts with `terrain.png`'s still images. Cosmetic, one
frame. Repro: `rust/scripts/loading.json` with `g13` in `rdata_load/saves`.

**P9. The loading screen's progress bar was not seen in Rust.** Vanilla's
"Building terrain" shows the green 100 × 2 bar (`vanilla/m22_loading_b.png`);
the only Rust capture of that stage (`rust/m22_loading_a.png`) has none.
One capture each, so this may be timing; treat it as *to confirm* with a
script that screenshots every frame of a load.

**P10 (to confirm). Chunk loading and meshing progress at a fixed time
differs from vanilla's.** Both clients load, generate and populate chunks
as the chunk renderers ask for them (`WorldRenderer.updateRenderer` builds
a `ChunkCache`, which loads its chunks, ChunkCache.java:9-23), a few
renderers per frame within the frame budget (`RenderGlobal.updateRenderers`
in a loop until `renderEndNanoTime + 1/120 s` at Balanced,
EntityRenderer.java:365-378, :485-492). So how far the loaded area has
grown 6 s after loading depends on the frame rate (P6), the frame budget
and the build cost, and it differs between the clients and between the
Rust profiles. What that changes, in the `g05` view (one camera, saves
read with `scripts/chunks.py`, `cmp/fallingsand.txt`):

| Run | Chunks (populated) | FallingSand: recent / long falls |
|---|---|---|
| vanilla 6 s, two runs | 606 (549), 702 (645) | 0 / 117, 84 / 61 |
| Rust dev 6 s, four runs | 600 (543), 539 (484), 630 (574), 630 (574) | 561 / 78, 18 / 52, 323 / 42, 417 / 53 |
| Rust release 6 s, two runs | 734 (676), 737 (679) | 342 / 52, 265 / 82 |
| vanilla 30 s | 817 (760) | none |
| Rust release 30 s | 817 (760), the same chunks | none |
| Rust dev 30 s | 733 (675), a disc inside vanilla's square | none |

"Recent" falls have `motionY` above −1.0, under about 35 ticks into the
fall. Gravel placed by population next to a cave gets a scheduled tick
(`Chunk.setBlockID` calls `onBlockAdded`, Chunk.java:267, :302;
BlockSand.java:12-14) and falls 3 ticks later as an entity
(BlockSand.java:24-43; `fallInstantly` is set only inside `populate`,
ChunkProviderGenerate.java:311, :602), so whichever client is populating
more chunks in the last second before the save holds more young
FallingSand (the many sharing one `motionY` probably fell in the same
tick's batch of scheduled ticks). Most long falls in both clients are
gravel dropping into the void under the flat area's edge chunks (x = −8,
z = −8). At 30 s, release Rust
and vanilla have loaded and populated **the same 817 chunks**, hold no
FallingSand, and similar mobs (vanilla 84 hostile, 23 passive; Rust 80
and 25). 559 of the 817 chunks are block-for-block identical; the rest
differ in trees, fluid levels and gravel, the population-order effect of
§9.8. So this is not a simulation divergence (the golden fixtures hold
the population and tick-scheduling code to the oracle); it is a
difference in loading speed and order. The same cause probably explains
the one `g15` capture whose painting walls were not yet meshed (§3: 1 run
of 11). F3's "chunk updates" (renderers rebuilt in the last second,
Minecraft.java:611-612) 7 s into `g01` read 468 in vanilla at 242 fps,
1072 in Rust dev and 1775 in Rust release at 60 fps (`cmp/m18_f3_sbs.png`,
`rust/m18_f3_release.png`): Rust rebuilds more renderers per second than
vanilla, yet the dev build had loaded fewer chunks after 30 s. What to
confirm: whether the Rust renderer's choice and order of renderers to
rebuild (`terrain/mod.rs` around `update_renderers`, which also stops when
enough mesh jobs are in flight) matches vanilla's, given the same frame
rate, and whether the dev profile's slower loading matters for anyone but
these screenshots. Repro: `RBIN=./target/release/mc_b173` or the dev
binary with `scripts/rrun.sh g05 <script that waits 120 or 600 ticks and
quits>`, vanilla `scripts/vbatch.py --settle 6 g05` or `--settle 30`, then
`python3 scripts/chunks.py <save>`.

---

## 3. Blocks

Every block id 1-96 is placed except 36 (the moving piston exists only
during a piston's motion). 42 views (31 in `parity`, 11 in `parity2` for
the variants the review asked for); each pair was looked at in full and in
a crop of the gallery row (`cmp/gNN_row.png` and the stacked sheets
`cmp/g14_16.png`, `g17_19.png`, `g20_22.png`, `g23_25.png`, `g26_27.png`,
`g28_30.png`, `g31_33.png`, `g34_36.png`, `g36s_37.png`).
Every metadata variant the brief and the review named is now placed;
variants not shown are those that look like one shown (stairs and slabs
beyond the ones in g08/g20, crops on dry farmland).
Diff column: mean / % of pixels over 24 (clouds and the HUD hint included).

| View | Blocks (id:meta) | Diff | Verdict |
|---|---|---|---|
| g01 | stone, grass, dirt, cobblestone, planks, bedrock, sand, gravel | 3.4 / 2.7 | match |
| g02 | gold, iron, coal, lapis ore, lapis block, sandstone, gold block, iron block | 4.2 / 3.1 | match |
| g03 | bricks, TNT, bookshelf, mossy cobblestone, obsidian, diamond ore and block, workbench | 2.9 / 2.1 | match |
| g04 | redstone ore 73, glowing 74, ice, snow block, clay, netherrack, soul sand, glowstone | 3.4 / 2.5 | match |
| g05 | logs 17:0-2, leaves 18:0-2 (biome tint), sponge, glass | 3.1 / 2.7 | match |
| g06, g07 | wool 35:0-15 | 3.7 / 3.1 each | match |
| g08 | double slabs 43:0-3, slabs 44:0-3 | 4.2 / 3.8 | match |
| g09 | saplings 6:0-2, tall grass 31:0-2 (hashed offsets), dandelion, rose | 3.7 / 3.0 | match |
| g10 | mushrooms 39, 40, dead bush 32, cactus 81, reed 83 on grass by water, snow layer 78, web 30, fire 51 on netherrack | 5.3 / 3.5 | match (taken again after review with the reeds on grass and the snow layer moved away from the fire, both now stable: first pass 5.4 / 4.3 with reeds on sand, §9.1) |
| g11 | crops 59:0-7 on wet farmland | 4.1 / 3.7 | match |
| g12 | farmland 60:0/7, cake 92:0,1,3,5, snow layers 78:3,7 | 3.9 / 3.5 | match |
| g13 | water 9, flowing water 8:1,3,5,7, lava 11, flowing lava 10:2,4 | 5.0 / 5.2 | match (lone flowing blocks decay on random ticks in both, §9) |
| g14 | torches 50:1-5, redstone torches 76 lit and 75 unlit on a lever-powered block, levers 69:5/13, button 77:4 | 3.7 / 3.4 | match |
| g15 | redstone wire 55 at power 15/10/5/0, cross, corner; pressure plates 70:0/1, 72:0 | first run 20.1 / 12.3; re-take 3.4 / 2.6 | first run **differs**: in the 15:21 Rust capture the stone walls of the three painting galleries at z = 40 were not yet meshed 6 s after loading (sky at (150..420, 82..102) where vanilla has stone), so the paintings, which are entities, hung in the sky (`old/g15_old_sbs.png`). The re-take (`cmp/g15_sbs.png`) and 10 repeat runs (5 dev, 5 release, `rust/rep/`) all have the walls, and `cmp/g14_16.png` showed them too: an intermittent chunk-build delay, **P10**. The wire itself matches. |
| wire_close | the wire close up | 5.7 / 5.5 | match (brightness gradient identical) |
| g16 | repeaters 93 facing 0-3 and delays 1-4, powered 94 | 3.9 / 4.0 | match |
| g17 | rails 66:0-9 (straight, slopes, curves) | 3.6 / 3.6 | match |
| g18 | powered rails 27:0/1/8, detector rails 28:0/8, ladders 65:2-5, wall signs 68:2-5 | 3.4 / 3.5 | match |
| g19 | wooden doors 64 (four facings, open), iron doors 71 | 3.2 / 3.3 | match |
| g20 | stairs 53:0-3, 67:0-3 | 3.6 / 3.7 | match |
| g21 | fences 85 (post, line, corner, cross, two high) | 3.8 / 4.1 | match |
| g22 | chest, double chest, furnace 61:2, lit furnace 62:2, dispenser 23:2, note block 25, jukebox 84:1 | 3.6 / 3.8 | match (the other facings are in g36) |
| g23 | pumpkins 86:0-3, jack o'lanterns 91:0-3 | 3.5 / 3.5 | match |
| g24 | trapdoors 96:0-7 | 6.1 / 5.6 | match |
| g25 | beds 26 (four facings), mob spawner 52 (spinning pig, flames), locked chest 95, portal 90 | 4.3 / 4.1 | match (the locked chest vanishes on a random tick in both) |
| g26 | pistons 33 and sticky 29 retracted (up, north, east) and extended with heads 34 | 3.6 / 3.7 | match |
| g27, signs_close | sign posts 63 at 8 rotations with 4 lines of text | 3.8 / 3.8; close-up 15.5 (clouds) | match (glyphs identical) |
| paint1-3 | all 25 painting motives | 8.7-10.5 (clouds) | match |
| g28 (`parity2`) | stone buttons 77:1-4 on the four sides of a post, pressed 77:9-12; levers 69:1-4 off and 69:9-12 on | 2.6 / 2.0 | match |
| g29 | wall redstone torches 76:1-4 lit; 75:1-4 unlit on a post a lever on top powers; floor levers 69:6 and 69:14 | 2.8 / 2.2 | match |
| g30 | farmland 60:1-6, cake 92:2 and 92:4 | 3.2 / 2.6 | match |
| g31 | falling fluids: water 8:8 and lava 10:8 alone, a water column (9 over 8:8, 8:8) and a lava column | 4.8 / 4.8 | match (the lone falling lava had decayed in both, the lone falling water in vanilla only: random ticks, §9.1) |
| g32 | open doors in facings 1-3: wood 64:5-7, iron 71:5-7 | 2.9 / 2.3 | match |
| g33 | slopes of powered rails 27:2-5 and detector rails 28:2-5 | 3.2 / 2.7 | match |
| g34 | retracted pistons 33:0, 3, 4, 5 and sticky 29:0, 2, 3, 4 | 3.5 / 3.0 | match |
| g35 | extended pistons with heads, normal and sticky, facing down, south, west, east | 3.4 / 3.1 | match |
| g36, g36_south | furnace, lit furnace and dispenser in facings 3, 4, 5 (fronts of facing 3 from the south) | 3.6 / 3.3; 2.6 / 2.2 | match |
| g37 | sign posts 63 at the odd rotations 1-15 with text | 4.1 / 3.6 | match |
| g04/g13/g14/g25_night | the light sources at midnight | 0.5-1.8 (g25: 10.1, §9) | match |
| chest1-5 | every block item and item as GUI icons (3D cubes, flat icons, counts, damage bars) | 0.25-2.5 in the GUI rectangle | match; animated tiles (water, lava, fire, portal) differ in phase; 3D cube edges differ by one pixel (rasterisation) |

Not covered: the moving piston's tile-entity rendering during motion (the
piston stations end after the motion).

---

## 4. Mobs and entities

| View | Content | Verdict |
|---|---|---|
| zoo1 | zombie, skeleton, creeper, powered creeper (armour pass), zombie pigman, spider (eyes), slimes size 1, 2, 4 (outer cube) | match except **P1** (no bow, no gold sword) |
| zoo2 | pigs (one saddled), cow, chicken, sheep (white, sheared, red, blue, black) | match (models, textures, fleece colours) |
| zoo3 | wolves wild, tamed and sitting, angry (red eyes); sheep yellow and brown; pig, chicken, cow | match |
| zoo_big | giant (scale 6), ghast in a glass box, squid in a tank | match; the ghast fires through the glass in **both** clients (§9) |
| zoo1_night | the same at midnight | match (natural spawns differ) |
| things, items_close | dropped items (block cube, sword, apple, bow, wool, coal ×5, bucket, compass, torch), 3 minecarts (plain, chest, furnace), boats on water and land | match (bob phase differs) |
| m20_third_person | the player in F5 with full diamond armour | match except **P1** |
| m17_inventory | the inventory preview of the armoured player | match |
| death | the dying player, dropped inventory | **differs, P3**: vanilla draws the dropped items and the arm, Rust neither |
| tw_mobs | testWorld's chicken and cows | match |
| zoo4, zoo5 (`parity2`) | sheep in the ten colours zoo2/zoo3 lack (orange, magenta, light blue, lime, pink; grey, light grey, cyan, purple, green): all 16 fleece colours are now covered | match (`cmp/zoo4_6_rows.png`) |
| zoo6 | the generic `Monster` (EntityList id 49: a biped with the player skin), a zombie, a zombie pigman and an angry one (`Anger` 500) | match except **P1** (no gold swords); anger has no visible sign in b1.7.3 in either |
| creeper_swell | a creeper fenced in with the player: it swells and explodes | match: the creeper grows over its fuse in both, `random.fuse` (1.0, 0.5) then `random.explode` in both (vanilla 29 ticks apart by world time; Rust 1.1 s apart by wall clock, just after loading), the blast breaks the fence and leaves the player at 14 health in both saves (`cmp/creeper_vanilla_sheet.png`, `cmp/creeper_rust_sheet.png` from `scripts/vcreeper.py` and a Rust 10 fps capture, `cmp/creeper_flash.png` at the blast). The white flash phase was not caught at 10 fps |
| snd_fishing | the cast fishing hook and its line | match (`cmp/fishing_line.png`; a 1-pixel cyan mark beside the Rust hook, probably a splash particle, not checked further) |

Also seen in passing, in both: zombies and skeletons burning in daylight
(Gate 3), primed TNT with its flash (`rust/snd_tnt.png`), falling-sand
entities and items in lava (natural terrain). Not covered by a matched
screenshot: arrows stuck in blocks, snowballs, eggs, fireballs in flight,
lightning, a ridden pig or minecart, a spider jockey, a wolf shaking
water off (§10). A spider jockey cannot be put in a world file: b1.7.3
saves no rider links (`Entity.writeToNBT` has no `Riding`), so it exists
only from natural spawning (SpawnerAnimals.java:162-167) until the world
is reloaded.

---

## 5. Screens

| Screen | Pair | Diff | Verdict |
|---|---|---|---|
| Title | m01_title | 3.9 | match; the Multiplayer button is disabled in Rust (deferred, D3) and the splash is random in both |
| Options | m02_options | **0.0** | identical |
| Video settings | m03_video | **0.0** | identical |
| Controls | m04_controls | **0.0** | identical |
| Mods and Texture Packs | m05_texturepacks | 1.7 | match (default pack only, D3) |
| Select world (one world, selected) | m06_select, m07_select_selected | 1.2 | match; Rust adds the project's Import... button |
| Rename world | m08_rename | **0.0** | identical |
| Delete confirmation | m09_delete | **0.0** | identical |
| Create new world | m10_create | **0.0** | identical |
| Import (project-owned) | rust/m23_import.png | — | Rust only |
| Loading | m22_loading_a-d | — | match ("Loading level" / "Building terrain" / "Simulating world for a bit"), **P8**, **P9** |
| Pause menu | m11_pause | 2.4 | match |
| Achievements | m12_achievements | 1.5 | match except **P2** |
| Statistics: General / Blocks / Items | m13-m15 | 0.6 | match; the numbers are each client's own stats file (vanilla's had 90 sessions); Blocks and Items were empty in both |
| Options in game (Difficulty shown) | m16_options_ingame | 1.2 | match |
| Inventory | m17_inventory | 2.6 | match (achievement popup, preview, slots, counts) |
| Crafting table | st_workbench | 0.2 in the GUI | match |
| Furnace | st_furnace | 0.2 | match |
| Dispenser | st_dispenser | 0.2 | match |
| Chest, large chest | snd_chest_open, chest1-5 | 0.3-2.5 | match |
| Sign editor | st_sign | 0.4 (re-take) | match (text, cursor, Done) |
| Chat key (T) | m19_chat_key | 7.8 | match: `T` opens nothing in single player in both (Minecraft.java:1068) |
| Sleeping | st_bed | 3.9 (re-take) | match (in-bed camera at midnight, sleep fade) |
| Game over | death | 14.1 (re-take) | the screen matches ("Score: &e0" literal in both, vanilla bug kept); the world behind it **differs, P3** (no hand, entities or tile entities in Rust) |
| F3 | m18_f3 | 15.4 | layout matches; **P5**, **P6** (§8.2) |
| F1 (hide GUI) | m21_hide_gui | 3.2 | match (HUD and hand hidden) |

---

## 6. Sounds

33 sound stations (`scripts/stations.py`, the views `snd_*`: 25 in
`parity`, 7 added after review in `parity2` and `snd_fizz` in the nether;
the zoo, GUI, creeper and death runs add mob and GUI sounds), each run in
both clients with the same inputs. Rust logs with `MC_SOUND_LOG=1`
(`rust/logs/<station>.log`), vanilla through the harness
(`vanilla/sound.log` for the first pass, `vanilla/sound2.log` for the
review's re-takes and new stations, one `== <station>` section each; a
station run twice has two sections, which `sndcmp.py` adds up). Full
tables: `cmp/sound_compare_full.txt`, `cmp/sound_compare_fix.txt`.

Logging conventions differ, not the sounds: vanilla's log has the
arguments of the call (`note.harp` volume 3, ghast volume 10, `playSoundFX`
volume 1.0, a record as `streaming 13` with 1.0/1.0); Rust's has the
effective values (volume clamped to 1 with the range `16 × volume`;
`playSoundFX` × 0.25; a record at 0.5 × the sound option). Pitches drawn
from a `Random` differ run to run in both; the table compares the ranges.

| Trigger | Sounds in both clients (volume, pitch) | Verdict |
|---|---|---|
| Dig grass/dirt with a pickaxe | `step.grass`/`step.gravel` 0.25, 0.5 every 4 ticks; break 1.0, 0.8 | match |
| Place cobblestone | `step.stone` 1.0, 0.8 | match |
| Walk over grass, gravel, stone, wool | `step.grass`/`gravel`/`stone`/`cloth` 0.15, 1.0 | match (counts follow the distance walked) |
| Door | `random.door_open` or `door_close` (chosen by `Math.random()`, RenderGlobal) 1.0, 0.9-1.0 | match |
| Lever, button, pressure plate | `random.click` 0.3, 0.6 on / 0.5 off | match |
| Note block | `note.harp` 3.0, 0.53-0.60 rising | match |
| Bow; arrow hitting the ground | `random.bow` 1.0, 0.83-1.25; `random.drr` 1.0, 1.1-1.3 | match |
| Flint and steel | `fire.ignite` 1.0, 0.8-1.2; then `fire.fire` 1.0-2.0, 0.3-1.0 | match |
| TNT | `random.fuse` 1.0, 1.0; 80 ticks later `random.hurt` then `random.explode` 4.0, 0.56-0.84 in the same tick | match (Rust checked with a 25 s run, `rust/logs/snd_tnt_long.log`; the fire took 40 ticks to light the TNT in vanilla and 160 in Rust, random; the vanilla re-take exploded within its 10 s, the Rust 10 s run had not yet) |
| Record into jukebox | `playRecord("13")` → streaming at the block | match |
| Piston (via a lever) | `tile.piston.out` / `in` 0.5, 0.6-0.85 | match |
| Dispenser with arrows | `random.click` 0.3 (the lever), `random.bow` 1.0, 1.2 (aux 1002), then `random.drr` | match (vanilla's arrow landed within the run, Rust's after it) |
| Hit a pig | `mob.pig` 1.0, 0.8-1.2; `mob.pigdeath` 1.0, 0.9-1.0; sword `step.wood` 0.25, 0.5 on the fence | match (after review the camera looks down 17° so the ray clears the fence top and meets the pig; it dies in both) |
| Break glass | `step.stone` hits, `random.glass` 1.0, 0.8 | match |
| Walk into deep water | `random.splash` 0.06, 0.6-1.4 | match |
| Fall 15 blocks (armour) | landing `step.grass` 0.5, 0.75; `random.hurt` 1.0, 0.8-1.2 | match; both end at 9 hearts (`ceil(15 − 3)` = 12 damage, EntityLiving.java:440-444, × (25 − 20) / 25 = 2 for fresh full diamond armour, EntityPlayer.java:439-445, InventoryPlayer.java:284-305) |
| Fall 40 blocks (no armour) | `random.hurt`, Game Over | match |
| Rain | `ambient.weather.rain` 0.2, 1.0, about 7 a second (one per ~2.9 ticks: `random.nextInt(3) < rainSoundCounter++`, EntityRenderer.java:624-629) | match (vanilla 85 over 230 ticks, 7.4 a second; Rust 84 calls, 72 in range, over the 16.5 s the log spans, with an opening burst of 3 in one frame) |
| Pick up items | `random.pop` 0.2, 1.4-2.6 | match |
| Portal block nearby | `portal.portal` 1.0, 0.8-1.2 (random display tick) | match |
| Walk into a portal | `portal.trigger` (fx), once | match |
| Stay in a portal | `portal.trigger`, then `portal.travel` and the nether | Rust: travel after the load, **P7**; vanilla travel not driven (§10) |
| Start inside a portal | nothing: `timeUntilPortal` starts at 20 and `setInPortal` resets it to 10 every tick | match (vanilla quirk in both) |
| Open a chest | no sound (b1.7.3 has none) | match |
| Lava pool and fire on netherrack (`snd_lava`) | `fire.fire` 1.0-2.0, 0.3-1.0; lava makes **no sound** in b1.7.3, only `lava` particles (BlockFluid.randomDisplayTick, BlockFluid.java:209-221; there is no `liquid.lavapop` in this version) | match |
| Hit a zombie pigman (`snd_pigzombie`) | `mob.zombiepig.zpighurt`, then `mob.zombiepig.zpigangry` (vanilla logs the call's 2.0 and pitch × 1.8, EntityPigZombie.java:21; Rust the clamped 1.0) and `zpigdeath` | match |
| Slime in a pen with the player (`snd_slime`) | `mob.slime` 0.6 on its jumps, `mob.slimeattack` 1.0 (EntitySlime.java:117) and `random.hurt` | match |
| Throw snowballs, eggs (`snd_snowball`, `snd_egg`) | `random.bow` 0.5, 0.33-0.46, three each (ItemSnowball.java:11, ItemEgg.java:11) | match (hotbar counts 13 in both) |
| Cast a fishing rod (`snd_fishing`) | `random.bow` 0.5, 0.36-0.43 (ItemFishingRod.java:24) | match; no bite in 25 s in either (`random.splash` 0.25 from the bobber, EntityFish.java:284, is random) |
| Pour water by lava (`snd_lavamix`) | `random.fizz` 0.5, 2.2-3.2 as the lava hardens; `liquid.water` 0.75-1.0, 0.5-1.5 from the flowing water; `random.splash` from the bucket | match |
| Thunderstorm (`snd_thunder`, 45 s) | `ambient.weather.rain`; `ambient.weather.thunder` (vanilla logs the call's 10,000, Rust the clamped 1.0) 0.8-1.0 from lightning (EntityLightningBolt.java:39) | match (2 bolts in vanilla, 4 in Rust: random); Rust also logged the bolt's `random.explode` (2.0, range 32, :40); vanilla's bolts struck farther than 32 blocks |
| Empty a water bucket in the nether (`snd_fizz`) | `random.fizz` 0.5, 2.0-2.5, no water placed (ItemBucket.java:78-85) | match |
| Creeper next to the player (`creeper_swell`) | `random.fuse` 1.0, 0.5, then `random.explode` and `random.hurt` | match (§4) |
| Mob ambient and hurt | `mob.pig`, `mob.cow`, `mob.sheep`, `mob.chicken`, `mob.zombie`, `mob.skeleton`, `mob.spider`, `mob.slime`, `mob.zombiepig.zpig`, `mob.wolf.bark/growl/panting/whine`, `mob.creeper` (hurt only: creepers have no living sound), `mob.ghast.moan/charge/fireball` (volume 10, range 160) | match (names and volumes; timing is the AI's) |
| GUI button | `random.click` fx 1.0, 1.0 | match |
| Minecart, boat | none: b1.7.3 has no minecart or boat sounds (no call site) | match |
| Music | not compared (random timer) | §10 |

---

## 7. testWorld, nether, night

| View | Content | Verdict |
|---|---|---|
| tw_spawn | `testWorld` as saved: on the chest, looking down at it and the workbench | match |
| tw_lake | north over the beach and lake | match |
| tw_mobs | (−15.5, 64, −53.5): chicken, cows | match |
| tw_lake_night | the lake at midnight | match |
| nether | `parity_nether`: netherrack, lava, fire, glowstone, fog colour | match (0.7 / 0.3, taken again after review with every saved chunk populated, §1.2; the first pass also matched, with a pigman in vanilla only: natural spawn) |
| edge | the saved flat area's west edge: terrain each client generates and populates beside saved chunks | match; small population differences (a gravel vein) as §9 |

---

## 8. Render distance, F3, frame rate, options, folders

### 8.1 Render distance and fog

`fog` station: the `g01` view, then `key.fog` (F) three times; screenshots
`fog_far`, `fog_normal`, `fog_short`, `fog_tiny` (`cmp/fog_a.png`,
`cmp/fog_b.png`). Average colours of a stone face, the grass at y 300 and
the sky at y 30:

| Distance | Vanilla stone / sky | Rust stone / sky | Verdict |
|---|---|---|---|
| Far | (84, 78, 78) / (143, 201, 255) | (82, 76, 76) / (134, 196, 250) | match (sky differs by the clouds) |
| Normal | (84, 78, 78) / (170, 209, 255) | (84, 78, 77) / (169, 208, 254) | match |
| Short | (84, 78, 78) / (178, 212, 255) | (84, 78, 78) / (178, 212, 255) | identical |
| Tiny | (91, 87, 89) / (192, 216, 255) | (91, 87, 89) / (192, 216, 255) | identical |

The grass sample is (82, 110, 41) in all eight. The far fog band of
Far/Normal differs where the clouds and the terrain loaded by then differ.

### 8.2 F3

`cmp/m18_f3_sbs.png`. Same layout: the five left lines (version and fps,
`C:` sections, `E:` entities, `P:`/`T:`, `ServerChunkCache`), the position
block (x, y, z, f), the right column and the frame-time graph. Differences:
the memory numbers (**P5**); the fps (**P6**); the graph's bars sit at the
ring buffer's write position in both and look different only because
vanilla recorded about 400 frames while F3 was on and Rust 90; the `E:`
totals (vanilla 0/178, Rust 3/390; the release build 3/430,
`rust/m18_f3_release.png`) count every loaded entity, and 6 s after
loading Rust's held far more FallingSand than vanilla's because it was
populating more chunks at that moment (**P10**, where the 30 s state is
the same in both). Vanilla's long fps line runs into the right column
("…updatUsed memory"); Rust's shorter line does not.

### 8.3 Frame rate

On the same machine (RTX 3080, NVIDIA 615.71, nested Xwayland): vanilla 242
fps at Balanced (uncapped), Rust 60 fps (vsync) in both the dev and the
release build (`rust/m18_f3_release.png`). **P6**. The Rust dev profile
ran the stations at 20 ticks a second throughout.

**Release build.** Every pair above is the dev profile. After review the
release build (`cargo build --release -p mc-app`, same commit) ran the
`g15` view 5 times (all with the painting walls, `rust/rep/g15_rel*.png`,
and so did 5 dev runs), F3 on `g01`, and `g05` for 6 s twice and 30 s
once (**P10**): it loads chunks faster than the dev build (734 chunks
after 6 s against 630) and after 30 s holds the same 817 chunks as
vanilla. No visual difference between the profiles was seen in these
runs; the matched pairs were not all retaken in release.

### 8.4 Default `options.txt`

A fresh vanilla home and a fresh Rust data directory write the same file,
line for line (`vanilla/options.txt`, `rust/options.txt`): `music:1.0`,
`sound:1.0`, `invertYMouse:false`, `mouseSensitivity:0.5`, `viewDistance:0`,
`guiScale:0`, `bobView:true`, `anaglyph3d:false`, `advancedOpengl:false`,
`fpsLimit:1`, `difficulty:2`, `fancyGraphics:true`, `ao:true`,
`skin:Default`, `lastServer:`, and the ten `key_…` lines.

### 8.5 Folder layout

| Vanilla `.minecraft/` | Rust data directory | Note |
|---|---|---|
| `options.txt` | `options.txt` | same |
| `saves/<world>/{level.dat, level.dat_old, session.lock, region/, data/}` | `saves/<world>/{level.dat, level.dat_old, session.lock, region/}` | Rust writes no empty `data/` (`SaveHandler` makes it, SaveHandler.java:23-24; the `Storage` trait has no empty directories) (maps were not exercised) |
| `stats/stats_<name>_unsent.dat`, `.old` | same | same |
| `screenshots/` (on F2) | same | same |
| `texturepacks/` (made at start-up) | — | texture packs deferred (D3) |
| `resources/` (downloaded) | `--resources` elsewhere | D8 |

`level.dat` trees after a session have the same keys and types (diffed
with `scripts/nbt.py`); only the times and `SizeOnDisk` differ.
`SizeOnDisk` counts region growth up to the last `saveLevel` in both. The
live autosave growth matches (vanilla 37, 53, 94, 184 KB at 8, 15, 22,
29 s; Rust 45, 57, 90, 176 KB). The world list's "MB" differs after these
test sessions only because vanilla went through the pause menu, whose
`quickSaveWorld` saves everything, and the Rust script quit directly.

---

## 9. Differences that are not Rust bugs

Kept here so the next comparison does not report them again.

1. **Random ticks.** Each client seeds its world `Random` from the clock,
   so blocks that change on a random tick change at different moments:
   lone flowing water and lava decay (`g13`: 4 Rust trials, 1-3 of 5 gone
   after 125 ticks, vanilla 0 in one run; `g31`: the lone falling water
   gone in vanilla only), the locked chest vanishes (vanilla in `g21`,
   Rust in `g25`; it emits light, hence the bright pad in vanilla's
   `g25_night`), fire ages and spreads, glowing redstone ore dims. In the
   first pass the `g10` reeds stood on sand, which
   `BlockReed.canPlaceBlockAt` rejects (BlockReed.java:33-48), and its
   snow layer stood two blocks from the fire, lit above 11 so it melts
   (BlockSnow.java:71-77); both broke on random ticks at different moments
   in the two clients. After review the reeds stand on grass by water and
   the snow layer 5 blocks from the fire, and both stay in both clients.
2. **Mobs.** Positions, poses, jumps and natural spawns follow each
   client's AI randomness. Animals and monsters despawn beyond 128 blocks
   in both (EntityLiving.java:674).
3. **The ghast shoots through glass** in both clients
   (`rust/logs/zoo_big_long.log`, the vanilla log at the same time): the
   first fireball explodes on the box's wall at (58.4, 68.5, 66.0) in both.
4. **A player loaded inside a portal never triggers it** (§6), in both.
5. **Harness marks.** Vanilla in-game screenshots after the first one carry
   the chat line "Saved screenshot as …" (F2 adds it); the Rust `screenshot`
   action does not. The "Press 'E' to open your inventory" hint shows in
   Rust's default data directory and not in vanilla's (the vanilla `parity`
   stats file has the achievement from the in-game tour). The cloud offset
   counts ticks since the client started. Statistics are each client's own.
6. **Edges of 3D item icons** differ by one pixel in places (GL vs wgpu
   rasterisation), as do some block edges in the world.
7. **The dark stripe on the cliff** at the saved/generated border (`g01`,
   `g02`) is in both clients' screenshots (`cmp/g01_cliff.png`); a first
   vanilla run without it loaded the chunks in another order.
8. **Population at the saved area's edge** (a gravel vein in `edge`, trees
   in `g02`'s background) depends on the order chunks are generated, which
   follows the renderers' order in each client. After 30 s in `g05`, 559 of
   the 817 chunks both clients loaded are block-for-block identical; the
   rest differ in trees, fluid levels and gravel (P10).
9. **How far loading has got** 6 s after a world opens differs between the
   clients and between runs (P10): how many chunks are populated, so how
   many FallingSand and Item entities the save holds and the F3 `E:`
   total.

---

## 10. Not verified / owner to verify

Each with what to do in the Prism client (or `scripts/vanilla_launch.sh`).

- **Portal travel in vanilla.** The xdotool key hold walked through the
  portal both times. Prism: build a portal, step in, stop: `portal.trigger`
  at once, `portal.travel` about 4 s later, before the loading screen.
  Compare with Rust (P7).
- **Moving-piston rendering** (block 36, `TileEntityRendererPiston` during
  the 2-tick motion). `parity` view `snd_piston`: toggle the lever and watch
  the head slide in both clients.
- **Statistics with data**, the Blocks and Items tabs filled (empty in both
  test runs). Mine and craft a few things in each client, open Statistics.
- **Projectiles and effects** not screenshotted side by side: arrows stuck
  in blocks, snowballs and eggs in flight, ghast fireballs in flight, a
  lightning bolt (only its sounds were logged, `snd_thunder`), riding a
  pig, minecart or boat.
- **Mob states not placed**: a spider jockey (cannot be saved, §4: in
  either client wait for one at night in a cave-rich seed, or in Rust use
  `spawn_entity` twice and compare the pose with vanilla by eye); a wolf
  shaking water off (`EntityWolf`, EntityWolf.java:164-172: walk a tamed
  wolf into water and out again in both clients and compare the shake and
  `mob.wolf.shake`).
- **Sounds not triggered**: `ambient.cave.cave`, `mob.ghast.scream` and
  the other ghast and mob death sounds (pig and pigman deaths were
  logged), `mob.chickenplop` (a chicken lays every 6,000-12,000 ticks,
  EntityChicken.java:42-46: leave a chicken in view for 10 minutes),
  `mob.wolf.shake` (above), the bobber's `random.splash` on a bite (1 in
  500 a tick with the hook in water, 300 under open sky,
  EntityFish.java:276-284), music
  timing. `random.pop` from a snowball or egg (EntitySnowball.java:222-227,
  EntityEgg.java:235-240) cannot be triggered in play: it is the pick-up
  code copied from `EntityArrow`: it needs the projectile in the ground
  and thrown by that player, but `inGround` is set only by loading the
  entity from NBT and `thrower` is not saved, so it never runs (vanilla
  dead code; it would give the player an arrow). Lightning's
  `random.explode` (range 32) was logged in Rust only because vanilla's
  bolts struck farther away; nothing to verify. There is no
  `liquid.lavapop` in b1.7.3 (§6).
- **Audible output.** Both logs record the calls; nobody listened. The
  Rust logs carry no audio-device warning.
- **Long play.** Each view ran about 10 s. WP-4.2's soaks cover the
  simulation; nothing here covers rendering over hours.
- **Release build.** Only the runs named in §8.3 used it; the matched
  pairs are the dev profile.

---

## 11. Reproducing

Everything below `build/screens/wp41/` is untracked; the commands rebuild it.

```sh
W=build/screens/wp41
# worlds and per-view copies
cargo run --release -p mc-golden --example parity_world -- $W/worlds   # parity, parity2, parity_nether
python3 $W/scripts/mkviews.py $W/worlds          # all views (or name some)
python3 $W/scripts/mktw.py                       # testWorld views
cp -r $W/worlds/views/g01 $W/worlds/views/fog    # the render-distance station's world

# nested X servers (vanilla on :47, Rust on :48)
$W/scripts/xw_start.sh 47 &  $W/scripts/xw_start.sh 48 &

# vanilla: the unsigned jar copy for ParityLaunch (class files unchanged; never commit)
python3 -c "import zipfile,sys; zi=zipfile.ZipFile(sys.argv[1]); zo=zipfile.ZipFile(sys.argv[2],'w',zipfile.ZIP_DEFLATED); [zo.writestr(i, zi.read(i.filename)) for i in zi.infolist() if not (i.filename.startswith('META-INF/') and i.filename.endswith(('.SF','.DSA','.RSA','MANIFEST.MF')))]" .reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar /tmp/mc-unsigned.jar
javac --release 8 -nowarn -cp .reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/minecraft.jar:.reference/mc_b1.7.3_release/1.7.3-LTS/jars/bin/lwjgl.jar -d $W/scripts/java/classes $W/scripts/java/ParityLaunch.java
MCJAR_UNSIGNED=/tmp/mc-unsigned.jar $W/scripts/vanilla_launch.sh /tmp/vhome parity &

# vanilla runs (VHOME = the fake home)
export VHOME=/tmp/vhome
$W/scripts/vbatch.py g01 g02 …                   # plain views (F2 after 6 s)
$W/scripts/vmenu.py                              # menus from the title screen
$W/scripts/vbatch.py --settle 6 --actions "$(cat $W/scripts/vmenu_ingame.json)" g01
$W/scripts/vstations.py $(python3 $W/scripts/stations.py list)
$W/scripts/vcreeper.py                           # creeper_swell, x11grab at 10 fps from before the world opens

# Rust runs
$W/scripts/rbatch.sh $W/rust/scripts/view.json g01 g02 …
$W/scripts/rstations.sh $(python3 $W/scripts/stations.py list)
RDISPLAY=x11 RDATA=$W/rdata_menu $W/scripts/rrun.sh - build/screens/wp41/rust/scripts/menu_title.json
RDISPLAY=x11 RDATA=$W/rdata_menu $W/scripts/rrun.sh g01 build/screens/wp41/rust/scripts/menu_ingame.json
RBIN=./target/release/mc_b173 RDISPLAY=x11 RDATA=/tmp/r $W/scripts/rrun.sh g05 <wait-120-or-600-ticks-and-quit.json>   # P10
python3 $W/scripts/chunks.py $W/vsaved/g05 /tmp/r/saves/g05                                                # chunk and entity counts

# compare
$W/scripts/cmpall.sh g01 g02 …                   # cmp/<v>_sbs.png, <v>_diff.png, stats
python3 $W/scripts/sndcmp.py $W/vanilla/sound.log snd_dig snd_tnt …
```

`vx.sh` drives the vanilla window on `:47` (`click X Y`, `key K…`,
`keydown`/`keyup`, `type`, `shot NAME` = F2, `grab NAME` = x11grab).
`vbatch.py` opens a view through the menus, runs its actions, leaves through
the pause menu ("Save and quit to title", waiting for the title screen
before moving the save to `vsaved/`; each run is copied to its own
`saves/<view>_<n>` because of vanilla's static `RegionFileCache`, §1.2).
`stations.py` writes each station's
steps once, for both clients. `nbt.py` and `mcr.py` read and patch
`level.dat` and region files; `ents.py` counts saved entities. A vanilla
view takes about 16 s, a Rust one about 12 s.

---

## 12. Review changes

The review compared the first version of this document with the spec
tree, the saves, the logs and the screenshots. Each finding was checked
against the Java before it was applied.

| Finding | Outcome |
|---|---|
| The Game Over screen lacks entities and tile entities too, not only the hand | Applied: **P3** widened (RenderGlobal.java:253-266 draws from `mc.renderViewEntity`; Rust's `render/mod.rs:701-716` returns when the view entity has left the world). Confirmed on the re-taken `cmp/death_sbs.png` |
| The FallingSand / `E:` gap was explained without evidence | Applied: the explanation is gone; **P10** (to confirm) records the investigation. The suggested cause, ticks scheduled on generation or loading that vanilla does not schedule, was not borne out: at 30 s release Rust and vanilla hold the same 817 chunks and no FallingSand, so the gap is how far loading has got at 6 s. The pointer to `ChunkProviderLoadOrGenerate` does not apply: b1.7.3 worlds build a `ChunkProvider` (World.java:210-213), whose population rule is ChunkProvider.java:50-66 |
| The `g15` row quoted the close-up's figures and hid a missing wall | Applied: `g15` has its own row (first run differs, re-take and 10 repeats match), latency under **P10** |
| Repeated vanilla views reused stale region files (`RegionFileCache`) | Applied: `vbatch.py` uses a fresh directory per run; the affected views were taken again (§1.2) |
| `liquid.lavapop` does not exist in b1.7.3 | Applied: removed from §10; §6 has a lava row saying lava is silent (BlockFluid.java:209-221); the `snd_lava` description in `parity_world.rs` corrected |
| Rain "about 20 a second" | Applied: about 7 a second (EntityRenderer.java:624-629) |
| Fall with armour "6.5 hearts" | Applied: 9 hearts (12 × 5 / 25 = 2 damage) |
| `g22` description metadata | Applied: 61:2, 62:2, 23:2 (the other facings are now in `g36`) |
| Stale sign-rotation comment | Applied |
| `g10` reed on sand, snow layer by the fire | Applied: reed on grass by water (BlockReed.java:33-48), snow layer 5 blocks from the fire (BlockSnow.java:71-77); `g10` re-taken, both stable |
| `parity_nether` edge chunks never populated | Applied: chunks −3..3 loaded, −3..2 saved; `nether` re-taken |
| Coverage: metadata variants | Applied: `parity2` galleries `g28`-`g37` (§3) |
| Coverage: mob variants | Applied: all 16 sheep colours, `Monster`, an angry pigman, a swelling creeper (§4). A spider jockey cannot be saved in b1.7.3 and a wet wolf was not set up: §10 |
| Coverage: sound call sites | Applied: 8 new stations (§6) cover `mob.slimeattack`, `zpigangry`, `zpigdeath`, `mob.pigdeath`, the thrown-item and fishing-rod `random.bow`, `random.fizz` twice, `liquid.water`, thunder and lightning. Rejected in part: `random.pop` from snowballs and eggs cannot fire in b1.7.3 (§10). `mob.chickenplop`, `mob.wolf.shake` and a bobber's `random.splash` are in §10 |
| Coverage: dev profile only | Applied in part: release runs of `g15`, `g05` and F3 (§8.3) |
