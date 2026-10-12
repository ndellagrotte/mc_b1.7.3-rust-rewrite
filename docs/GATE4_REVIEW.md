# Gate 4 review notes (Phase 4, WP-4.1 … WP-4.9)

Written 2026-10-10 by the Gate 4 reviewer for the owner's review. PLAN_v4 §5
defines Gate 4 as "vanilla parity on native" (WP-4.5, the export, may slip
past the gate; it did not).

Phase 4 was integrated on one branch, `phase-4`, forked from the Gate 3
commit `d52ac6b`. Its tip was `a2e73c3` (WP-4.9 step 13) before this
document. It holds 63 linear "WP-x.y step N" / "WP-x.y merge" commits over
`d52ac6b`: wave E (WP-4.2, 3.9, 4.6, 4.4, 4.5, 4.3, 4.1) and wave F (WP-4.8,
4.7, 4.9, merged in that order). Nothing is pushed.

Size of Phase 4: 166 files, +43,055 / −1,789 lines. By area: `mc-client`
49 files (+6,352 / −1,016), `mc-sim` 51 files (+2,859 / −697), `mc-golden`
10 files (+2,781 / −20), `mc-app` 1 file, `mc-core` 1 file (a coverage
test), `tools/` 6 files, `docs/` 7 files (+3,094), `tests/golden` 35 files
(17 new fixtures, ~27.8k lines of oracle output, mostly the five soaks).

**Verdict: Gate 4 passes.** Every criterion was exercised on this tree and
held. The original b1.7.3 client was launched again (§3) and compared side by
side on 12 stations; every difference found is one the checklist already
classifies as run-to-run noise that vanilla shows against itself
(PARITY_CHECKLIST §9). The open items (§8) are documented divergences the
owner has to accept or schedule, and paths that were not exercised; none was
seen to break parity in play.

| # | Criterion | Result | Evidence |
|---|---|---|---|
| G4-1 | Every block rendered (PARITY_CHECKLIST §3: ids 1-96 and their metadata variants, 42 views) | **pass** | Checklist verdict "match" on every view; re-checked here on `g01`, `g14`, `g22`, `g25` and `nether` against the real client: the ground, the blocks, the HUD and the hand differ by 0.5-1.4 mean (§3) |
| G4-2 | Every screen (§5 of the checklist: 21 screens) | **pass** | Checklist "match"/"identical"; P2 (flat achievement icons) closed by WP-3.9, re-checked here: achievements screen 1.2 mean against vanilla (§3); Game Over re-checked (§2.2, §3); every Rust-side screen of the Gate 3 path, import, export and the chest minecart seen again (§2) |
| G4-3 | Every mob (§4 of the checklist) | **pass** | Checklist "match"; P1 (third-person held items) closed by WP-3.9, re-checked here: the zombie pigman's gold sword in `zoo1` and the player's sword in F5 against vanilla (§3); P3 (Game Over draws no entities) closed by WP-4.9, re-checked (§2.2, §3) |
| G4-4 | Every sound trigger (§6 of the checklist: 33 stations) | **pass** | Checklist "match" on every station; P7 (`portal.travel` after the load) closed by WP-4.9, re-checked here on a walked portal round trip: `portal.trigger` → 4.08 s → `portal.travel` → 20 ms → "Toggling dimension!!" (§2.3); the `death` station's calls (landing `random.hurt`, `mob.ghast.moan` volume 10 / range 160, `portal.portal`) equal vanilla's (§3) |
| G4-5 | Gate 3 path again (title → create → play → save → reload; import `testWorld`; D5 refusal) | **pass** | §2.1 |
| G4-6 | Dead-player path (die, Game Over draws entities and the hand, quit, reopen on Game Over, respawn) | **pass** | §2.2 (P3 and P4 closed) |
| G4-7 | Portal round trip by walking | **pass** | §2.3 |
| G4-8 | Export of a Rust-created world, then import of that export | **pass** | §2.4; the same export also opened in the vanilla client at the exported position |
| G4-9 | Chest minecart | **pass** | §2.5 |
| G4-10 | F3 memory lines | **pass** | §2.6; same two lines as vanilla, real values (P5 closed) |
| G4-11 | Frame rate, Balanced vs Power saver | **pass** | §4.2: Balanced uncapped (dev 220-310 fps, dist 260-388), Power saver 41 fps, as vanilla's EntityRenderer.java:364-416 (P6 implemented with the vanilla default) |
| G4-12 | 20 quick quits | **pass** | §2.7: 20 of 20 exit 0, every save written, no core dump |
| G4-13 | Side-by-side spot check against the real client (10 stations required) | **pass** | §3: 12 stations, vanilla launched with the §1.1 recipe |
| G4-14 | Performance vs the oracle and the vanilla client | **pass** | §4: the simulation runs the soaks at 2,198-2,801 ticks/s (oracle ~2,300-2,400); the client at 2-4× vanilla's frame rate on this machine |
| G4-15 | Packaging | **pass** | §5: `dist` build, the Linux bundle packed as `release.yaml` does, run from the data-dir layout of `docs/RUNNING.md` |
| G4-16 | Soak | **pass** | §1: the 5 soaks pass (36 s parallel; serial ticks/s in §4.1); `SOAK=only regen.sh` changes nothing |
| G4-17 | Gate commands (build, dist build, tests, clippy, fmt, wasm32, regen ×2, reproducibility) | **pass** | §1 |

Screenshots, dumps, logs and the scripts that made them are under
`build/screens/gate4/` (git-ignored; paths below are relative to it). The
data directories of the scripted runs are under the session scratchpad
(`…/scratchpad/gate4/d1` … `d8`, `dq`, `dfar*`). The harness is WP-4.9's
rebuilt one (PARITY_CHECKLIST §13.4), copied to `harness/`, with two changes
of this review (§3).

In all runs the game logged no panic and no error of its own. The only
`WARN` lines are the two expected D5 refusals and winit's "error setting
XSETTINGS" on the nested X server.

## 1. Gate commands

Run in the main checkout at `a2e73c3` on 2026-10-10 (Arch Linux, kernel
7.2.8, AMD Ryzen 7 7700X, NVIDIA RTX 3080 driver 615.71, Wayland). This
review made no code changes.

| Check | Result | Evidence (`logs/`) |
|---|---|---|
| `cargo build -p mc-app` | pass | `build.log`; binary copied to `bin/mc_b173_dev` for every run below |
| `cargo build --profile dist -p mc-app` | pass, 9 min 34 s | `dist.log`; `target/dist/mc_b173` 75.3 MB |
| `cargo test --workspace` | pass | `test.log`: **1,206 passed, 0 failed, 7 ignored** over 61 test binaries (incl. doc tests). The 7 ignored are the 5 soaks and the 2 oracle cross-reads (`#[ignore]` since WP-4.8, Gate 1 N6) |
| `cargo test -p mc-golden --test golden_soak -- --ignored` | pass | `soak.log`: 5 passed in 36.4 s; `soak_serial.log` (one at a time, §4.1) |
| `cargo test -p mc-sim --test golden_save -- --ignored cross_read` | pass | `crossread.log`: both cross-reads (vanilla reads a Rust-written world, Rust a vanilla-written one) pass against `build/oracle` |
| `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` | clean | `clippy.log` |
| `cargo fmt --all --check` | clean | `fmt.log` |
| `cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown` | pass (D9) | `wasm.log` |
| `sh tools/oracle/regen.sh` | no drift | `regen.log`; `git status --short tests/golden` empty (`regen_status.txt`) |
| `SOAK=only sh tools/oracle/regen.sh` | no drift | `regen_soak.log`, `regen_soak_status.txt` empty |
| `sh tools/oracle/check-reproducible.sh` | pass | `repro.log`: "3 runs byte-identical (235 files)" |

Test counts by binary (largest first): `mc_client` unit tests 475 (Gate 3:
431), `golden_tick` 200 (188), `mc_sim` unit tests 88, `mc_core` 29,
`mc_nbt` 26, `golden_save` 24 (+2 ignored), `mc_golden` 21, then the
`mc-sim` integration suites (`stats` 15, `animals` 15,
`inventory_containers` 14, `block_simple` 14, …) and the new ones
(`gap_hygiene`, `parity_remainder`, `singleplayer_gaps`). Gate 3 ended at
1,113.

Fixtures: **238** `scenario.json` under `tests/golden` = 233 default + 5 soak
(Gate 3: 221). The 17 new ones: the 5 soaks (WP-4.2);
`tick/cake-heal-window`, `entity/pig-lightning-pigman`,
`entity/wolf-defends-owner`, `entity/minecart-chest-container`,
`entity/skeleton-arrow-after-death` (WP-4.6); `tick/bed-nether-player-near`,
`entity/boat-broken-by-rider`, `entity/boat-shot-by-rider`,
`entity/minecart-broken-by-rider`, `entity/minecart-shot-by-rider`,
`entity/fishing-rod-dead-bobber` (WP-4.8); `tick/dead-player-save-reload`
(WP-4.7). Every one comes from the oracle and passes; none is ignored.

## 2. The exercised paths

Every run below is `bin/mc_b173_dev --data-dir <fresh dir> --script
scripts/<s>.json` on the desktop's Wayland (`WAYLAND_DISPLAY=wayland-0
DISPLAY=:0`), window 854 × 480, GUI scale 2, default options. The scripts
are generated by `scripts/gen.py`. Positions are `posX, posY (eye), posZ` as
`dump` prints them. Every screenshot named here was looked at.

### 2.1 The Gate 3 path again (`s1`, `s2`, `s2b`, `s3`)

- **Create, play, save, reload** (`s1/01`-`14`, dumps `s1/d*.json`): title
  (stone logo, splash, Multiplayer disabled per D3) → Singleplayer → Create
  New World, name `gate4`, seed `123` → in game after 200 ticks
  (`05_ingame.png`: a desert shore with dead bushes, cactus, water, 3D
  clouds, the sun, the hand, ten hearts and the "Press 'E'" hint). 30 ticks
  of `key.forward`, a mouse turn, then looking straight down with the left
  button held 40 ticks: the sand at (−91, 63, 260) and (−91, 62, 260)
  broke with the crack overlay (`08_digging.png`), the mouse-over moved
  from (−91, 63, 260) to (−91, 61, 260) and the two sand items fell into
  the hole (`09_dug.png`). Inventory (`10`, "Achievement get! Taking
  Inventory"), Escape twice → Game menu with "Saving level.." (`11`) → Save
  and quit to title → Singleplayer (`13`: "gate4 (10/10/26 5:42 AM, 1.61
  MB)") → `click_slot 0` → Play Selected World. On arrival: dir `gate4`,
  world time 311 = before the save, position (−90.5, 65.62000000476837,
  260.02422883197863), **Δ = 0** on every axis, yaw 0 / pitch 90 equal,
  mouse-over (−91, 61, 260). `14_reloaded.png` shows the hole and both
  dropped items. On disk (`scripts/mcr.py`): (−91, 62..65, 260) air,
  (−91, 61, 260) sand.
- **Import `testWorld` through the UI** (`s2/01`-`04`): Singleplayer →
  Import... → the path typed → Import. One frame in: dir `testWorld`, seed
  123, world time 1751 = `level.dat` `Time`, position
  (9.52309703383433, 66.62000000476837, −60.519650742587615) = `Player.Pos`
  exactly, yaw −264.29 (= −624.29 % 360, as Gate 3 explained). A right
  click opens the chest with **2 planks and 4 sticks** (`03_chest.png`).
  `set_pos` to the save's animals: a cow and a chicken (`04_mobs.png`).
- **Reopen from the list** (`s2b`): world time 1882 and position
  (−15.300000011920929, 64.62000000476837, −53.69999998807907) equal to
  the quit save; `02_reopened.png` matches `s2/04`.
- **D5 refusal** (`s3`, worlds from `scripts/make_bad_world.py`):
  `BadBlocks` → "World refused: Block id 200 is not a Beta 1.7.3 block.
  First found in chunk (0, −4) (Blocks at local x 5, y 70, z 5). The world
  was not imported; nothing was copied." (`01`); `NoLevelDat` → "Cannot read
  level.dat: missing" (`02`). The data dir holds only `options.txt` and
  `stats/` afterwards.

### 2.2 The dead player (`s4`, `s4b`): P3 and P4

A Rust-created world (`death4`, seed 123). Five stacks given (sword,
cobblestone, torches, a map, logs), a stone pillar set under (1.5, 1.5) and
the player put at y 121.62 with no armour. After the fall: health −32, the
Game Over screen.

- `s4/02_game_over.png` (60 ticks after the death, so after `deathTime` 20
  took the player out of `loadedEntityList`): "Game over!", "Score: &e0"
  (vanilla's literal), Respawn, Title menu; behind the gradient the water,
  a squid, the pig spawned next to the body, and **the arm** (lower right).
  Before WP-4.9 this view drew the terrain only (P3).
- `quit` (dev `changeWorld1(null)`), then `level.dat` (`harness/nbt.py`):
  `Player` present with **`Health` −32, `DeathTime` 21**, an empty
  inventory, the death position. This is vanilla's save (PARITY_CHECKLIST
  P4); Rust used to write no `Player` (closed by WP-4.7).
- Reopen (`s4b`): the world opens **on the Game Over screen** (dump: screen
  `GameOver`, health −32; `01_reopened.png` with the red-tinted stone and
  the arm). Respawn → in game at the spawn (56.5, 65.62, 45.5), health 20,
  the same entity id 241 (Minecraft.java:1451, :1468). `level.dat`
  afterwards: `Health` 20, `DeathTime` 0.

The vanilla side of this path is the `death` station of §3.

### 2.3 A portal round trip on foot (`s5`, `MC_SOUND_LOG=1`)

World `portal4` (seed 123). An obsidian floor and a 4 × 5 frame were set 3
blocks south of the player with `set_block`, the player was given flint and
steel (259) and **lit the portal with a right click** on the frame's floor
(`01_portal_lit.png`: the animated portal, the frame, the held flint and
steel). Then:

1. `key.forward` 12 ticks: the player stops inside the portal
   (`02_in_portal.png`: the purple overlay; dump `in_portal` true,
   `time_in_portal` 0.25). `portal.trigger` at 12.968, `portal.travel` at
   17.040 (+4.07 s, 80 ticks at 0.0125), "Toggling dimension!!" at 17.060:
   the travel sound starts before the world change, as EntityPlayerSP.java:58-59
   (P7 closed).
2. Nether arrival at (10.5, 68.62, −9.0) in the portal the Teleporter built
   (`03_nether_arrival.png`).
3. `key.back` 8 ticks then `key.left` 8 ticks: out of the portal
   (`04_nether_out.png`: lava falls, netherrack, the portal at the right;
   dump `time_until_portal` 0 after 30 ticks out).
4. `key.right` 8, `key.forward` 8: back in (`in_portal` true). `portal.trigger`
   26.641, `portal.travel` 30.740 (+4.10 s), toggle at once.
5. **Overworld arrival at (87.0, 67.62, −15.5), in the portal that was lit in
   step 0** (the Teleporter found it within 128 blocks of x × 8). Dimension 0.
   `05_overworld_again.png`: inside the portal block (its faces fill the view,
   as in vanilla, §2.4).

### 2.4 Export, then import of the export (`s6`, `s6b`, vanilla `exp_portal4`)

`portal4` (§2.3, a Rust-created world with a nether) exported through the
world list: Singleplayer → row 0 → Export... (`02_export_screen.png`, the
default folder `<data-dir>/exports`) → Export → "World exported: 11 files, 8
region files and 1565 chunks were read back and match the world. The player
is at 87.0, 67.6, −15.5." (`03_exported.png`) → Done. The export has
`level.dat`, `level.dat_old`, `session.lock`, `region/` and `DIM-1/region/`
(4 region files each).

A fresh data dir then imported that folder through Import... (`s6b`): one
frame in, dir `portal4`, world time 533, position (87.0, 67.62000000476837,
−15.5), dimension 0, all equal to the export's `level.dat`.

The same export, opened in the **vanilla client** (`vanilla/exp_portal4.png`,
`vsaved/exp_portal4_30`): listed as `portal4`, the player stands in the
portal holding the flint and steel, the view as Rust's; vanilla's own save
keeps the position (87.0, 67.62000000476837, −15.5) (only `Time` moved on,
533 → 671). Vanilla does not teleport a player loaded inside a portal either
(PARITY_CHECKLIST §9.4).

### 2.5 The chest minecart (`s7`, `s7b`)

World `cart4`; a stone pad set around the player, a chest minecart spawned 2
blocks ahead (`spawn_entity Minecart`, type 1), 32 cobblestone and 7 planks
given. A right click on the cart (mouse-over `Entity`) opens `GuiChest`
titled **"Minecart"** with 3 rows (`02_opened.png`). Clicks: the cobblestone
into cart slot 0, a right click with the planks puts one into slot 26
(`03_moved.png`; the "6" drawn there is the cursor stack under the pointer).
Escape drops the 6 planks left on the cursor (`onCraftGuiClosed`; the
inventory dump has no cursor and no planks). Reopened: 32 cobblestone in
slot 0 and 1 plank in slot 26 (`05_reopened.png`). After `quit` and a new
process (`s7b`): the same contents (`02_cart_after_reload.png`). Gate 3
recorded that the cart could not be opened; WP-4.6 closed it.

### 2.6 F3 (`s8`, `s8dist`, `rust/f3_g01.png`)

The right column now has vanilla's two lines: `Used memory: 7% (564MB) of
7796MB` / `Allocated memory: 12% (941MB)` (dev, `s8/dbg01_balanced.json`;
`dump_debug` writes the column since WP-4.9). Vanilla on the same view:
`Used memory: 6% (470MB) of 6932MB` / `Allocated memory: 33% (2307MB)`
(`vanilla/f3_g01_idle.png`); Rust: `4% (335MB) of 7796MB` / `9% (714MB)`
(`rust/f3_g01.png`). Same layout, lines and graph; the values are the
process's (WP-4.9's documented stand-in for the JVM heap: max =
`MemTotal/4`). The long fps line runs into "Used memory" in both. The
four `[rust]` lines are the project's (WP-4.3). `gaps` in every
`dump_debug` of this review is empty (`skipped_systems` {},
`unimplemented_hits` {}).

### 2.7 Twenty quick quits (`logs/qq/`, `logs/qq_results.txt`)

20 runs of `--import testWorld` with a script that quits 5 frames after the
world is in (the case that segfaulted 4 of 6 times in PERF.md §7): **20 of
20 exit 0**, every run's `saves/testWorld/level.dat` written, no panic in
any log, `coredumpctl list --since <start>`: "No coredumps found" (fixed by
WP-4.9's synchronous pipeline compilation).

### 2.8 Far from the origin (`far/`, extra)

PERF.md §7 records the `f32` world-space placement as designed, not built.
A world was set at x = 1,000,008 (a glass pad at y 107, `scripts/far.json`)
and opened in both clients (`far/far_rust_reopen.png`,
`vanilla/far_vanilla.png`): the same terrain, glass and water, no visible
displacement in a still frame (the differences are the pig's pose and the
clouds). Motion jitter was not measured: frame-to-frame pixel changes while
sneaking are larger far than near (`scripts/framesteps.py`: 23-74k vs
11-42k changed pixels per frame pair) but view bobbing and the animated
water confound the figure. Left as an open item (§8, O5).

## 3. Side by side with the original client

**The vanilla client was launched** with the PARITY_CHECKLIST §1.1 recipe as
WP-4.9 rebuilt it (§13.4): the obfuscated b1.7.3 `minecraft.jar` (the
signature-stripped local copy, never committed) through `ParityLaunch` on
Prism's Java 8, a fake home `build/screens/gate4/vhome`, on a nested
Xwayland `:47`, driven with `xdotool`; Rust on a second nested Xwayland
`:48` (`harness/rrun.sh`, dev binary). Both opened the same bytes (the
`parity` worlds of `crates/mc-golden/examples/parity_world.rs`, one copy
per view by `harness/mkview.py`), the screenshot about 6 s / 120 ticks after
the world appeared.

Two harness changes of this review:

1. `vrun.py` grabbed vanilla with `ffmpeg -f x11grab`. On this machine the
   first grab after a pause caught frames without the GUI pass and without
   entities (no HUD, no hand, no mobs in `zoo1`), while a grab 0.3 s later
   had them (`vanilla/g01_try0..3.png`). The stations now use the game's own
   **F2** screenshot (`grab_f2`: `ScreenShotHelper`, the back buffer), as
   WP-4.1's harness did.
2. Rust's data dir got the vanilla home's `stats/stats_parity_unsent.dat`,
   so neither client shows the "Press 'E'" hint (§9.5 of the checklist).
   Rust read vanilla's stats file and dropped the hint, which also checks
   WP-4.7's stats loading.

Diff = mean of the per-pixel maximum channel difference / share of pixels
over 24 (`harness/png.py`), whole frame and per region (`scripts/regdiff.py`:
sky = rows 0-89, mid = 90-199, ground = rows 200-409 left of the hand, HUD,
hand). `cmp/<v>_sbs.png` holds each pair (vanilla left, half scale);
`cmp/stats.txt` the figures.

| # | Station | Whole | Ground / HUD / hand | Verdict |
|---|---|---|---|---|
| 1 | `g01` stone, grass, dirt, cobblestone, planks, bedrock, sand, gravel | 5.8 / 6 % | 0.63 / 0.56 / 1.37 | match. Sky and horizon: clouds (offset counts ticks since start) and the generated hill beyond the flat area (loading order, §9.8). In one later vanilla run the dirt block had turned to grass (a random tick, §9.1) |
| 2 | `g14` torches, redstone torches, levers, button | 5.7 / 5 % | 0.60 / 0.54 / 1.34 | match (same; pigs wander, §9.2) |
| 3 | `g22` chest, double chest, furnaces, dispenser, note block, jukebox | 4.1 / 4 % | 0.62 / 0.51 / 1.28 | match |
| 4 | `g25` beds, spinning spawner, locked chest, portal | 5.1 / 6 % | 0.69 / 0.54 / 1.36 | match. The locked chest stood in vanilla's capture and had vanished in Rust's: it vanishes on a random tick in both (§9.1) |
| 5 | `zoo1` slimes, spider, zombie pigman, powered creeper, creeper, skeleton | 6.2 / 10 % | 3.80 / 0.54 / 1.47 | match: **the pigman holds its gold sword in both** (P1 closed), the powered creeper's armour pass in both; poses are each AI's |
| 6 | `zoo2` pigs, cow, chicken, sheep colours | 9.2 / 14 % | 4.98 / 0.56 / 2.42 | match (positions are AI, §9.2) |
| 7 | `things` dropped items, minecarts, boats | 6.7 / 6 % | 0.63 / 0.50 / 1.28 | match. Vanilla's horizon showed farther generated terrain at 6 s: Rust on the 60 Hz nested display loads more slowly (PARITY_CHECKLIST §13.2) |
| 8 | `nether` netherrack, lava, fire, glowstone, fog | **0.6** / 0.3 % | 1.40 / 0.31 / 0.24 | match |
| 9 | `death` Game Over over the dropped inventory | 13.4 / 17 % | — | match: both draw the arm and dropped items under the gradient. How many items lie in view is random in both (`cmp/death_reps.png`: one vanilla and three Rust runs; a second vanilla run, `vanilla/death_v2.png`, shows as few as Rust's). The saves agree: 24 items within 32 blocks, 11-14 within 4, 21 within 8, the player at (42.5, 64.1, 108.3) with `Health` −17 in both (`scripts/items_near.py` on `vsaved/death_28`, `death_29`, `rdata/saves/death`). Sounds: `random.hurt` at (42.5, 64.0, 108.3), `mob.ghast.moan` from (59, 66, 70) with vanilla's volume 10 (Rust logs the range 160), `portal.portal` from (48.5, *, 110.5) in both |
| 10 | `f3` on `g01` | 14.7 / 13 % | — | match: same lines and layout (§2.6); the fps and the frame graph differ (§4.2) |
| 11 | Achievements screen (pause menu → Achievements on `g01`) | **1.2** / 0.1 % | — | match: the log and the workbench drawn as lit 3D blocks in both (P2 closed) |
| 12 | Third person (F5 on `g01`) | 6.2 / 7 % | 1.02 / 0.46 / — | match: the diamond-armoured player from behind **with the sword in hand in both** (`cmp/tp_hstack.png`; P1 closed) |
| + | The Rust export of §2.4 opened in vanilla | — | — | opens at the exported position (§2.4) |

Every near-scene region (ground, HUD, hand) is within 0.5-1.5 mean on the
static views; the mob views carry the AI's motion. The whole-frame figures
are higher than WP-4.1's 2-4 because the clouds and the far terrain are in
the frame and differ with the time since start and the loading order; the
regions show the near scene matches.

Not re-driven here (the checklist's stored verdicts stand, all "match" after
wave F): the remaining 30 block views, the menu screens (5 byte-identical in
WP-4.1), the 33 sound stations, `testWorld` views, fog per render distance,
`options.txt` and the folder layout.

## 4. Performance

### 4.1 Simulation vs the oracle

`cargo test -p mc-golden --test golden_soak -- --ignored --test-threads=1
--nocapture` (dev profile, opt-level 1; `logs/soak_serial.log`):

| Soak (ticks) | Rust, this review | Oracle (tests/golden/README.md, JVM start included) |
|---|---|---|
| `nether-8675309` (36,000) | 2,310 ticks/s | ~2,300-2,400 |
| `seed-123` (72,000) | 2,669 | |
| `seed-8675309` (48,000) | 2,801 | 19-20 s ≈ 2,400-2,500 |
| `seed--7612233452384654969` (48,000) | 2,198 | |
| `testworld` (48,000) | 2,536 | |

On par with the oracle in the test profile (PERF.md §3.3: `dist` 2,008-3,050).

### 4.2 Client vs the vanilla client

F3's first line, `g01`-like scenes, same machine:

| Client | Display | Balanced | Power saver | Max FPS |
|---|---|---|---|---|
| vanilla b1.7.3 | nested Xwayland `:47` (GL, not paced) | 104 fps this session (`vanilla/f3_g01_idle.png`); 101 with a build running; 242 in WP-4.1's session | — (40 by EntityRenderer.java:369-411) | — |
| Rust dev | Wayland, `s8` (seed 123, new world) | 220, 310 after loading | **41** | 368 |
| Rust `dist` | Wayland, `s8dist` (packaged bundle) | 260, 388 after loading | **41** | 424 |
| Rust dev | nested Xwayland `:48` | 60 (the nested server presents Vulkan at 60 Hz, PARITY_CHECKLIST §13) | | |

Balanced and Max FPS run uncapped and Power saver holds about 40 fps, as in
vanilla; `--vsync on` remains the override. `dist` reached the game 1,271
ms after start on a new seed-123 world (PERF.md: 1,143-1,375).

## 5. Packaging

`target/dist/mc_b173` (75.3 MB, stripped) was packed as
`.github/workflows/release.yaml`'s Linux job does (`mc_b173`, `assets/`
(README only), `credits/`, `NOTICE`, `LICENSE`, `RUNNING.md`),
unpacked into an empty folder (`pkg/unpacked`) and run with `--data-dir
../data`, where the data dir holds `minecraft.jar` and `resources/` as
`docs/RUNNING.md` §2 says (no `--jar`/`--resources`). It created world
`dist4` (seed 123), played the `s8` script (F3, every performance option)
and saved (`level.dat`, `level.dat_old`, `session.lock`, `region/`).
`--jar ../nope.jar` stops with "mc_b173: cannot read minecraft.jar at
../nope.jar (No such file or directory (os error 2)); give your own Beta
1.7.3 client jar with --jar <path>, … (docs/RUNNING.md)". Nothing from the
jar or `resources/` is in the bundle. Windows and macOS bundles were not
built here (no runners).

## 6. What each Phase 4 WP delivered

From `git log d52ac6b..a2e73c3`. Every WP ends with review-fix steps.

| WP | Commits | Delivered |
|---|---|---|
| 4.2 Soak | `a62b460`…`70cdb29` (5) | The `SOAK` modes of the oracle scripts (default excludes the soaks, `SOAK=1` all, `SOAK=only`), five soak fixtures (36,000-72,000 ticks; autosaves, a reload, the nether, `testWorld`, fenced animal pens), `golden_soak.rs`, the README "Soak" section and CI |
| 3.9 Gate 3 defects | `3063b99`…`3250228` (6) | D1: third-person held items (`HeldItemHook` filled and inserted); D2: 3D block icons on the achievements and statistics screens; `click_slot`, `give`/`give_item` merged; the import refusal's log names the real file (O4); README and `mappings.md` nits |
| 4.6 Sim gaps | `3a6d370`…`c49549f` (5) | Cake heals through `EntityLiving.heal`; pig → zombie pigman on lightning; `onDeath`'s kill hooks; `alertWolves`' tamed branch; the chest minecart's `IInventory` (`InventoryRef::Entity`); removed entities a field still names are kept (`World::removed_entities`); 5 fixtures; `docs/WP4.6_SIM_GAPS.md` |
| 4.4 Deferred review | `27add9b`…`431a560` (3) | `docs/DEFERRED_REVIEW.md`: every `deferred` line checked, the 97 gap sites classified, the PLAN_v5 WP table, the 34-item owner backlog, the D8 audit; one tag text fixed |
| 4.5 Export | `637fe24`…`36fcb1e` (5) | `export.rs` (validate, copy, re-read, compare, publish), Export... on the world list, the export screens, the `export_world` dev action, `docs/EXPORT.md` (opened in the original client, cross-read by the vanilla loaders: 2,267 chunks, 0 differing) |
| 4.3 Performance and packaging | `75aac07`…`bd2769a` (7) + merge `177d207` | `ChunkProvider.chunkMap` as a hashed map, the random-tick loop and `positionsToUpdate` fixture-neutrally faster (3-5×); `perf` counters, `--vsync`, write-behind saves with incremental journaled region writes; `flate2` on `zlib-rs`; `--jar`/`--resources` from the data dir, `docs/RUNNING.md`, the release workflow; `docs/PERF.md`. The merge makes the export wait for the write-behind queue |
| 4.1 Parity checklist | `0d8c05e`…`d222a15` (4) | The `parity_world` example (3 worlds, 103 views), `docs/PARITY_CHECKLIST.md`: 137 screenshot pairs and 33 sound stations against the original client; defects P1-P10 |
| 4.8 Gap hygiene | `42acc5d`…`07c3cd0` (10) | Per-fixture `gaps.json` and `dump_debug`'s `gaps`; dead code and render-only arms out of `mc-sim` (gap sites 96 → 76); `EntitySlot::class_chain`; honest coverage checks (`every_<area>_fixture_has_a_test`) and `#[ignore]`d cross-reads; citation nits. **Beyond its behaviour-neutral brief** (flagged by the WP and the integrator): four reachable gaps fixed with 6 oracle fixtures (the nether bed's explosion hits the clicking player; a boat or minecart broken by its rider, by hand or its own arrow; the fishing rod on a dead bobber) |
| 4.7 Small parity remainder | `9f2708b`…`2f19710` (3) + merge `ff43be6` | P4: the dead player kept for `playerEntities`, saved in `level.dat`, the world reopens on Game Over, respawn moves the dead object and the fields naming it to a fresh id; `EntityFireball.getCollisionBorderSize` = 1; `StatsSyncher` as local file I/O (stats autosave every 100 ticks, the received file merged), the wrong "D2" labels relabelled; `craftingInventory` and the container close inside `EntityPlayer.onUpdate`; fixture `tick/dead-player-save-reload`. The merge adds `class_chain` to the re-added player's slot |
| 4.9 Client parity fixes | `0695eaa`…`a2e73c3` (13) | P3 (the view entity when dead), P5 (F3 memory from `/proc`), P6 (no vsync by default, the Power saver sleep), P7 (`portal.travel` before the world change), P8 (synchronous pipeline compilation, which also fixes the exit segfault), P9 confirmed, P10 (`renderEndNanoTime` at the frame's end), no achievement window over the loading screen, pass 1 nearest first, the inventory preview through the entity renderers (hurt overlay, shadow, fire), centroid sampling in `gl_material.wgsl`, `screenshot_frames`, PARITY_CHECKLIST §13 |

## 7. Counts

| | Gate 3 (`d52ac6b`) | Gate 4 (`a2e73c3`) | Command |
|---|---|---|---|
| Workspace tests passed | 1,113 | **1,206** (+7 ignored: 5 soaks, 2 cross-reads) | `cargo test --workspace` |
| Fixtures | 221 | **238** (233 default + 5 soak) | `find tests/golden -name scenario.json \| wc -l` |
| `deferred` lines, `mc-client/src` + `mc-sim/src` (case-insensitive) | 60 | **54** | `grep -rni deferred crates/mc-client/src crates/mc-sim/src \| wc -l` |
| `deferred` lines, `crates/ tools/ docs/PLAN_v4.md` (DEFERRED_REVIEW §1's count) | 75 | **72** | `grep -rni deferred crates/ tools/ docs/PLAN_v4.md \| wc -l` |
| `note_unimplemented` call sites in `mc-sim` | 97 | **74** | `grep -rn 'note_unimplemented(' crates/mc-sim/src \| wc -l` minus the definition |

The `deferred` lines fell where WP-4.7 relabelled the stats "sync" (not an
online service, DEFERRED_REVIEW §2.3; no stats line is tagged deferred any
more) and WP-4.8 removed dead arms. The gap sites fell from 96 (`d222a15`)
to 76 with WP-4.8 and to 74 with WP-4.7 (the dead player in `saveLevel`;
the kill hook on a removed player). In normal play none is reached (`gaps`
empty in every dump of §2).

## 8. Open defects and items

None blocks the gate. D = defect (a vanilla difference), O = open or not
exercised, N = documentation nit.

- **D1. A screen opened by a click opens at the next tick hook.** The rest of
  that tick's input (a number key, Q, a second right click) still reaches
  the game, where vanilla routes it to the new screen (Minecraft.java:995-1126).
  README "Known deviations"; DEFERRED_REVIEW §3.2 proposed it for WP-4.7,
  which did not take it. Repro: a script that, in one tick, right-clicks a
  chest and presses `Q`: vanilla's `Q` reaches `GuiChest` (nothing dropped),
  the port drops the held item. Not exercised here. Owner: schedule it
  (`Game::process_input` sets `current_screen` after each controller call
  that raises a GUI) or accept.
- **D2. A portal with a workbench open drops the 3 × 3 grid once**; vanilla
  drops it twice (the second `EntityItem`s share the first ones' `ItemStack`s
  and end up empty). Accepted divergence of value-typed stacks unless the
  owner wants it emulated (DEFERRED_REVIEW Q31). Not exercised.
- **D3. Anaglyph 3D draws one eye** (vanilla draws both with `glColorMask`).
  Accepted by WP-4.9 (PERF.md §7); off by default. Owner to confirm.
- **O1. `f32` far-origin jitter** (PERF.md §7, designed, not built): a still
  frame at x = 1,000,008 matches vanilla (§2.8); motion was not measured.
  Repro: `scripts/jitter_far.json` vs `jitter_near.json`, then compare
  the frames by eye.
- **O2. Loading speed differs from vanilla's in both directions** (P10,
  PARITY_CHECKLIST §13.2): uncapped, Rust fills the 817-chunk area within
  6 s where vanilla needs 6-30 s; on a 60 Hz-paced display (or `--vsync on`)
  it is slower than vanilla. The population order follows the loading order,
  so worlds generated in the two clients differ in trees, fluids and gravel
  at chunk edges (§9.8: 559 of 817 chunks identical after 30 s). Not a
  simulation divergence; the owner may choose `mark_render_end`'s schedule
  for `--vsync on`.
- **O3. Not exercised by anyone** (PARITY_CHECKLIST §10, unchanged): vanilla
  portal travel timing driven to the end (Rust's 4.0-4.1 s matches the 80
  ticks of EntityPlayerSP.java and WP-4.9's vanilla x11grab, 3.99 s); the
  moving piston (block 36) mid-motion; statistics tabs with data;
  projectiles in flight, lightning, riding; a spider jockey; a wolf shaking;
  `ambient.cave.cave`, `mob.ghast.scream`, `mob.chickenplop`,
  `mob.wolf.shake`, a bobber's bite splash, music timing; audible output
  (both clients' logs only); long play of the renderer; the Windows and
  macOS bundles.
- **O4. The reused player id** (DEFERRED_REVIEW §3.1 E proposal 1) is now
  implemented by WP-4.7 (`World::move_removed_entity_to_fresh_id`), but the
  oracle has no respawn op, so it is unit-tested only (`parity_remainder.rs`).
- **O5. The bobber hooking its own angler** (DEFERRED_REVIEW §3.1 E proposal
  2): one `if` once a scenario reproduces it.
- **O6. The `x11grab` capture of the vanilla client can miss the GUI pass**
  (§3, harness only). WP-4.9's §13 vanilla evidence that used `grab` (the
  `death` and `f3` pairs) should be read with that in mind; this review's
  F2 re-takes agree with its verdicts.
- **N1. Commit hashes in two docs point at pre-rebase commits that are not
  on `phase-4`.** PARITY_CHECKLIST §13 cites the `wp-4.9` worktree hashes
  before the integrator's rebase: `f934b92` → `0695eaa`, `7611a9c` →
  `d5565d2`, `6461094` → `30f897b`, `b0da87b` → `41e80f3`, `1186282` →
  `97792fa`, `785a158` → `48039fa`, `7dc1954` → `3abba70`, `3a13a76` →
  `d8d9ad9`, `d199d18` → `762b05c`. PERF.md cites `1fbbd8f`, `5318c27`,
  `46cfe53`, `a0a010b` (→ `75aac07`, `0c321b9`, `46f6fdc`, `66a855a`),
  `0518d31` (the pre-rebase review fixes) and `6461094` (→ `30f897b`). The
  objects still exist (dangling, until `git gc`).
  Closed after the review by the Phase 4 docs commit (see git log).
- **N2. PARITY_CHECKLIST does not record P4's closure.** §2 P4 says "the fix
  is in `mc-sim`"; §13.1's P4 row says "not this WP". WP-4.7 closed it
  (README "Stats autosave and the dead player", §2.2 here).
  Closed after the review by the Phase 4 docs commit (see git log).
- **N3. DEFERRED_REVIEW §4 and §3.6 are as of wave E**: the wave E rows say
  "in progress", WP-4.7 "not scheduled", the Gate 4 row "after wave F".
  §3.2's container close-timing row is closed by WP-4.7.
  Closed after the review by the Phase 4 docs commit (see git log).
- **N4. The handoff's "47 `deferred` tags"** (docs/handoff/HANDOFF_PHASE4_REST.md §1,
  untracked) matches neither grep of §7; use §7's commands.

## 9. Interface changes to `mc-sim` in Phase 4 (for approval)

`git diff --stat d52ac6b..a2e73c3 -- crates/mc-sim`: 51 files. Every fixture
still matches (§1).

1. **WP-4.3**: `ChunkProvider::chunk_map` is a `ChunkMap =
   HashMap<i32, Box<Chunk>, BuildHasherDefault<ChunkKeyHasher>>` (was a
   `BTreeMap`); never iterated (PERF.md §3.2).
2. **WP-4.6**: `InventoryRef::Entity(entityId)` (the chest minecart);
   `World::removed_entities` / `removed_entity(_mut)` (a removed entity a
   loaded one still names); `entity::player::{kill_hooks,
   add_to_player_score, on_kill_entity, alert_wolves}`, `KillHook`.
3. **WP-4.8**: `EntitySlot::class_chain` (the Java class chain for class
   queries); `mc_golden::Gaps` and `fixture_names`; dead `Untranscribed`
   variants removed.
4. **WP-4.7**: `World::entity_object`, `with_entity_object_mut`,
   `move_removed_entity_to_fresh_id`; a removed player kept while in
   `playerEntities`; `StatsSyncher` and `StatFileWriter::{func_27178_d,
   func_27425_c, func_27180_b, func_27187_c}`, `World::stats_syncher`;
   `PlayerSpData::crafting_inventory` with `set_crafting_inventory`,
   `close_screen`, `close_screen_dropping`;
   `entity::dispatch::get_collision_border_size` returning 1.0 for fireballs.

`mc-core` gained one coverage test; `mc-nbt` is untouched.

## 10. Owner decisions pending

1. **Sign off Gate 4?** Every exercised criterion passes (table above).
2. **PERF.md §6** (WP-4.3; no answer recorded):
   1. `unsafe` prefetch in the random-tick loop (`mc-sim` is
      `#![forbid(unsafe_code)]`; ~1.3-1.6× on the soaks) — not done.
   2. The `dev` profile's opt-level 1 for workspace crates — left at 1.
   3. Accept `zlib-rs` as the `flate2` backend (in use).
   4. Accept the write-behind storage (in use; a crash loses milliseconds of
      queued writes, every file stays old or new).
3. **P6 default.** WP-4.9 implemented vanilla's: Balanced and Max FPS
   uncapped, Power saver ~40 fps, `--vsync off` the default and `--vsync on`
   the override. Confirm, or make `on` the default for a tear-free window;
   and whether the `updateRenderers` budget under vsync should start at
   `First` again (O2).
4. **D1-D3 above**: schedule or accept the click-opened screen routing (D1),
   the workbench double drop (D2, Q31), anaglyph's second eye (D3); build the
   floating origin of PERF.md §7 or not (O1).
5. **The DEFERRED_REVIEW §5 backlog** (34 items; 24 open at WP-4.4). Changed
   since: Q12 and Q13 done (WP-4.8); Q18, Q20, Q24, Q25 delivered (WP-4.6,
   3.9, 3.9, 4.3/4.9); Q34 acted on by WP-4.7 (the stats autosave and the
   received-file merge are implemented and the labels relabelled) but the D2
   text in PLAN_v4.md:97 still lists "stats sync" — amend it in PLAN_v5.
   Still open, among others: Q2 (`Math` == `StrictMath`), Q3/Q4 (gate
   sign-offs and the workflow), Q5-Q8 and Q28-Q29 (earlier interface and
   harness decisions), Q15-Q16, Q21 (the data dir and `--jar`/`--resources`
   defaults), Q22 (import/export by typed path), Q23 (default username),
   Q30 (commit trailers: the briefs ask for "Claude Fable 5.1", the commits
   carry "Claude Opus 5.5"), Q31-Q33.
6. **The PLAN_v5 table** (DEFERRED_REVIEW §4): accept the splits and added
   WPs (Phase 4 added WP-3.9, 4.6, 4.7, 4.8 and wave F's 4.9), and update the
   wave E rows (N3).
7. **`testWorld` licence** (Q10, since Gate 1): three committed files are
   byte copies of it (`tests/golden/nbt/testworld-level-dat/level.dat`,
   `tests/golden/save/testworld/world/{level.dat, region/r.0.-1.mcr}`), and
   `tick/testworld-6000` and `soak/testworld` load it. Q11 (commit the whole
   world) depends on it.
8. **The `Entity.rand` rewrite** (PLAN_v4 §8 item 3, Q1): still undecided.
   Hostile natural spawning (K10) and the constructor draws (K11) stay
   unit-tested only, and the soaks run at difficulty 0 with thunder held
   off. The port needs no change either way.
9. **WP-4.8 changed behaviour beyond its brief** (four oracle-verified gap
   fixes, §6): approve after the fact.
10. **Export default folder** (EXPORT.md §7 Q2): `<data-dir>/exports`, or the
    Prism instance's `saves/` when it exists.

## 11. Reproducing

`build/screens/gate4/` (git-ignored):

| Path | What |
|---|---|
| `scripts/gen.py` | Writes `s1`…`s8`, `quickquit` (`scripts/*.json`) |
| `scripts/mcr.py`, `harness/nbt.py`, `scripts/items_near.py` | Region, `level.dat` and dropped-item readers |
| `scripts/make_bad_world.py` | The D5 refusal worlds (`bad/`) |
| `scripts/regdiff.py`, `harness/png.py` | Per-region and whole-frame diffs, montages |
| `scripts/far*.json`, `scripts/jitter_*.json`, `scripts/framesteps.py` | §2.8 |
| `harness/` | WP-4.9's harness (`xw_start.sh`, `vanilla_launch.sh`, `vrun.py` with `grab_f2`, `rrun.sh`, `mkview.py`, `java/ParityLaunch`) |
| `worlds/` | `parity`, `parity2`, `parity_nether`, `views.json`, `exp_portal4`, `farx` |
| `vanilla/`, `rust/`, `cmp/`, `vsaved/`, `rdata/` | §3 |
| `pkg/` | §5 |
| `logs/` | §1 and every run's log |

```sh
G=build/screens/gate4
sh $G/harness/xw_start.sh 47 & sh $G/harness/xw_start.sh 48 &
sh $G/harness/vanilla_launch.sh $PWD/$G/vhome parity &
VHOME=$PWD/$G/vhome python3 $G/harness/vrun.py view g01          # F2 after 6 s
RBIN=$PWD/$G/bin/mc_b173_dev RDATA=$PWD/$G/rdata RDISPLAY=x11 \
  $G/harness/rrun.sh g01 $G/rust/scripts/g4_g01.json $G/rust/logs/g01.log
python3 $G/scripts/regdiff.py $G/vanilla/g01.png $G/rust/g01.png
```
