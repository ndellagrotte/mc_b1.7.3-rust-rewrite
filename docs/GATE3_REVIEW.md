# Gate 3 review notes (Phase 3, WP-3.1 … WP-3.8)

Written 2026-10-07 by the Gate 3 reviewer for the owner's review. PLAN_v4 §5
defines Gate 3 as: "the game is playable from the title screen through world
creation or import of `testWorld`, play, save and reload".

Phase 3 was integrated on one branch, `phase-3`. It was forked from the
Phase 2 tip `a8f3841` (WP-2.H step 6), and its tip was `49cd40d` before this
document. It holds 61 linear "WP-x.y step N" / "WP-x.y merge" commits, rebased
and fast-forwarded from the per-WP branches (`wp-3.1`, `wp-3.1b`, `wp-3.2`,
`wp-3.3a/b/c/d`, `wp-3.4`, `wp-3.5a/b`, `wp-3.6a/b/c`, `wp-3.7a/b`,
`wp-3.8`). Nothing is pushed.

Size of Phase 3: 213 files, +68,888 / −181 lines. `mc-client` is new: 61.5k
lines of Rust (source and unit tests). `mc-sim` changed in 19 files
(+2,189 / −168, §5) and is now 64.8k lines.

**Verdict: Gate 3 passes.** Every step of the gate was driven through the real
menus by the `--script` dev mode on a fresh data directory and checked against
dumps, the saved files and screenshots (§2):

| # | Criterion | Result | Evidence |
|---|---|---|---|
| G1 | Title screen → Singleplayer → Create New World (`gate3`, seed 123) → in game | **pass** | `s1/01`–`05`; `d05_ingame.json`: dir `gate3`, seed 123 |
| G2 | Play: walk, look around, dig a block with the left button held, open the inventory | **pass** | `s1/06`–`11`; block (−60, 66, −240) broken (mouse-over before/after), dirt picked up |
| G3 | Escape → Game menu → Save and quit to title → Singleplayer → select `gate3` → Play: same world dir, same position (±0.01), dug block still air | **pass** | `s1/12`–`15`; position identical (Δ = 0), world time identical, region file: (−60, 66, −240) = 0 |
| G4 | Import… `testWorld` → in game at the saved player position; the chest at (9, 64, −61) opens and shows 2 planks + 4 sticks | **pass** | `s2/01`–`04`; position equals `level.dat` exactly |
| G5 | Walk to and look at water, a torch, a door/rail/stairs, a mob; `set_time 18000` | **pass (with a note)** | `s2/05`–`11`, `s2b/02`; `testWorld` has no torch, door, rail or stairs, so they were placed with the `set_block` dev action |
| G6 | Quit, reopen `testWorld` from the world list, dump | **pass** | `s2b/01`–`03`; position, rotation and world time equal the save |
| G7 | Import… a folder that fails D5 validation → the refusal dialog names the problem; nothing is copied | **pass** | `s3/01`, `s3/03`; `saves/` never created |
| G8 | Gate commands: tests, clippy, fmt, wasm32 check, oracle regen without drift | **pass** | §1 |

Screenshots are under `build/screens/gate3/` (git-ignored), one folder per
scenario; the scripts that made them, the bad-world maker and two small
readers (McRegion/NBT, PNG crop) are in `build/screens/gate3/scripts/`. All
paths below are relative to `build/screens/gate3/`.

In all nine runs the game logged no panic and no error or warning of its
own. The only `ERROR` lines are the script's "no enabled button" (the first
scenario 1 attempt, O1, and the menu run, §2.4), and the only `WARN` lines
are the two expected import refusals. The defects
found (§3) are visual gaps already recorded by the integrators (third-person
held items) and documentation nits. None blocks the gate.

## 1. Gate commands

Run in the main checkout at `49cd40d` on 2026-10-07 (Arch Linux, kernel
7.2.8, AMD Ryzen 7 7700X, NVIDIA RTX 3080, Wayland). Nothing needed fixing; this
review made no code changes.

| Check | Result | Evidence |
|---|---|---|
| `cargo build -p mc-app` | pass | binary `target/debug/mc_b173` |
| `cargo test --workspace` | pass | 1,113 passed, 0 failed, 0 ignored over 57 test binaries (incl. doc tests) |
| `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` | clean | exit 0 |
| `cargo fmt --all --check` | clean | exit 0 |
| `cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown` | pass | exit 0 (D9) |
| `sh tools/oracle/regen.sh` | no drift | exit 0; `git status --short tests/golden` empty afterwards (221 scenarios) |

Test counts by binary (largest first): `mc_client` unit tests 431,
`golden_tick` 188, `mc_sim` unit tests 88, `mc_core` 29, `mc_nbt` 26,
`golden_save` 26, `mc_golden` 20, then the `mc-sim` integration suites
(`animals` 15, `inventory_containers` 14, `block_simple` 14, `region_file` 13,
`golden_worldgen` 13, `client_requests` 13 (new in WP-3.1b), …). Gate 2 ended at
662 tests in the four simulation crates.

Fixtures: 221 `scenario.json` under `tests/golden` (Gate 2: 220). The one new
fixture is `tick/portal-player-collide` (WP-3.1b, `BlockPortal.onEntityCollidedWithBlock`
→ `EntityPlayer.setInPortal`). Every fixture passes; none is ignored.
`check-reproducible.sh` / `check-sensitivity.sh` were not re-run: Phase 3 did
not touch the harness or the oracle tools.

## 2. The gate path, step by step

The binary was run as `WAYLAND_DISPLAY=wayland-0 DISPLAY=:0 mc_b173 --data-dir
<fresh dir> --script <file>` (no `--world`, so it starts on the title screen),
window 854 × 480, GUI scale 2 (427 × 240 scaled pixels). Positions are
`posX, posY (eye), posZ` as `dump` prints them.

### 2.1 Scenario 1: create, play, save and reload (`scripts/s1.json`, `s1/`)

1. **Title** (`01_title.png`). Dirt background, the stone MINECRAFT logo, a
   rotated yellow splash ("Woo, tigsource!"), Singleplayer, Multiplayer
   (disabled), Mods and Texture Packs, Options..., Quit Game,
   "Minecraft Beta 1.7.3" top left and the copyright line bottom right. This
   matches the Beta 1.7.3 title screen (no panorama before 1.8).
2. **Select World, empty** (`02_select_empty.png`). Vanilla layout plus the
   project's "Import..." button top right (D5).
3. **Create New World** (`03_create.png`). The name box was cleared with
   12 × `BACK`, `gate3` typed, Tab to the seed box, `123` typed. "Will be
   saved in: gate3" updates live. Then "Create New World".
4. **Loading** (`04_loading.png`): "Generating level / Building terrain"
   with the green progress bar.
5. **In game** after 200 ticks (`05_ingame.png`, `d05_ingame.json`):
   `ticks_ran` 225, world time 201, dir `gate3`, level name `gate3`, seed
   123, player at (−59.5, 67.62, −235.5). The view shows fancy leaves with
   biome tint, oak and birch logs, grass with side overlay, sand, 3D clouds,
   the hand, the hotbar, ten hearts and the "Press 'E' to open your
   inventory" hint.
6. **Walk and look** (`06`–`08`, `d06_walked.json`): 30 ticks of
   `key.forward` moved the player to z −238.70 (stopped against a step), then
   mouse motion turned the view right, left-up and down. Water, sand, trees
   and the sky all draw as expected.
7. **Dig** (`08_look_down.png` → `09_digging.png` → `10_dug.png`,
   `d08_dig_before.json`, `d10_dig_after.json`): looking at the grass block
   (−60, 66, −240) (top face), left button held for 22 ticks. The crack
   overlay and hit particles show while digging. The block breaks into
   particles and a dropped dirt item. Mouse-over moves from (−60, 66, −240)
   to the block below, (−60, 65, −240).
8. **Inventory** (`11_inventory.png`): E opens `GuiInventory`. It shows the
   player preview (holding the dirt), the 2 × 2 crafting grid, the dirt in
   hotbar slot 0 (picked up), hover highlight under the pointer, and the
   "Achievement get! Taking Inventory" popup.
9. **Game menu** (`12_game_menu.png`, `d12_before_save.json`): Escape closes
   the inventory, Escape opens "Game menu" (Back to game, Achievements,
   Statistics, Options..., Save and quit to title) with "Saving level.."
   blinking bottom left. Position (−59.5, 67.62000000476837,
   −238.69999998807907), yaw −180, pitch 60, world time 287.
10. **Save and quit to title** (`13_title_again.png`): back on the title (new
    splash), the achievement popup still sliding (it is drawn after every
    screen, as vanilla's).
11. **Reload** (`14_select_gate3.png`, `15_reloaded.png`,
    `d15_reloaded.json`): Singleplayer lists `gate3 (10/7/26 2:34 PM, 1.82
    MB)`. Click the row, then "Play Selected World". One frame after
    arrival: dir `gate3`, world time 287, position (−59.5,
    67.62000000476837, −238.69999998807907) (**Δ = 0** on every axis), yaw
    −180, pitch 60, mouse-over (−60, 65, −240): the ray passes through the dug
    cell. The screenshot shows the hole and the held dirt drawn as a 3D block.
12. **On disk** (read back with `scripts/mcr.py` after the final quit):
    block (−60, 66, −240) = 0 (air), (−60, 65, −240) = 3 (dirt);
    `level.dat` holds the same player position, rotation (−180, 60) and an
    inventory of one dirt in slot 0.

### 2.2 Scenario 2: import `testWorld`, play, quit, reopen (`scripts/s2.json`, `scripts/s2b.json`, `s2/`, `s2b/`)

1. **Import screen** (`s2/01_import_path.png`): Singleplayer → "Import..."
   opens "Import world" with a path box ("Folder of a Beta 1.7.3 save:"). The
   path of `.reference/mc_b1.7.3-world/testWorld` was typed, then "Import".
   The copy runs inside the click; the log says `imported … as
   saves/testWorld`.
2. **Arrival** (`s2/03_arrived.png`, `d03_arrived.json`, one frame in):
   dir `testWorld`, seed 123, world time 1751 (= `level.dat` `Time`),
   position (9.52309703383433, 66.62000000476837, −60.519650742587615)
   **exactly** `level.dat`'s `Player.Pos`, pitch 88.35. Yaw is −264.29
   where the file has −624.29: `Entity.setRotation` takes `yaw % 360`
   (Java's sign-keeping remainder), as vanilla does on load. The player
   stands on the chest looking down at its lid and the workbench.
3. **Chest** (`s2/04_chest.png`, `d04_chest.json`): a right click opens
   `GuiChest` ("Chest" / "Inventory") with **2 planks in slot 0 and 4 sticks
   in slot 1**, as the save's `TileEntities` entry says.
4. **Water** (`s2/05_water_north.png`, `06_water_close.png`): north across
   the beach to the lake; walked 25 ticks to z −65.70 and looked down at the
   shore. The water texture animates, and the selection outline is visible
   on the sand.
5. **Torch, rail, stairs, door** (`s2/07_torch_rail_stairs_door.png`):
   `testWorld` contains none of these. A scan of chunks x −6..6, z −10..2
   found only the chest, the workbench and water. They were placed with the
   `set_block` dev action: torch 50:5 at (11, 64, −60), rails 66:0 at
   (12, 64, −60/−59), oak stairs 53:0 at (11, 64, −62), and a door 64:0 / 64:8
   at (13, 64/65, −62) (`raw`). All draw with the right shape and
   orientation: the rail runs north–south (left–right when facing east), the
   torch has its flame and smoke, the stairs step and the door has its window.
6. **Night** (`s2/08_night_torch.png`, `09_night_sky.png`): `set_time
   18000`. The torch keeps full brightness. The sky shows stars and dark
   clouds, and the terrain falls to the night light level.
7. **Mob** (`s2/10_mob.png`, `11_f3.png`, `s2b/02_reopened.png`): teleported
   (`set_pos`) to (−15.5, 65.62, −53.5) near the save's chicken and cow. A
   chicken wanders into view (`11_f3.png`, with F3 on) and is fully
   drawn in `s2b/02_reopened.png`.
8. **Quit** (dev `quit`, which saves). `level.dat`: `Time` 6112, `Pos`
   (−15.5, 65.62000000476837, −53.5), `Rotation` (90, 15). The placed blocks
   and the chest's items are in the region file.
9. **Reopen from the list** (`s2b/01_select_testworld.png`,
   `d02_reopened.json`): the list shows `testWorld (10/7/26 2:37 PM, 3.49
   MB)`. Play, then one frame in: world time 6112, position and rotation equal
   to the save. `s2b/03_blocks_after_reload.png` shows the placed torch,
   rails, stairs and door again.

### 2.3 Scenario 3: D5 refusal (`scripts/s3.json`, `scripts/make_bad_world.py`, `s3/`)

`make_bad_world.py` makes two folders from `testWorld`:

- `BadBlocks`: a copy with block id 200 written into chunk (0, −4) at local
  (5, 70, 5) (`r.0.-1.mcr`, recompressed in place).
- `NoLevelDat`: an empty `region/` and a `readme.txt`.

Results:

- `BadBlocks` → "World refused": "Block id 200 is not a Beta 1.7.3 block.
  First found in chunk (0, −4) (Blocks at local x 5, y 70, z 5). The world was
  not imported; nothing was copied." (`s3/01_refused_block200.png`).
- Back → the path screen. Clear the path, `NoLevelDat` → "Cannot read
  level.dat: missing" (`s3/03_refused_no_level_dat.png`).
- Back, Cancel → the select-world list is empty (`s3/04_select_after.png`).
  The data dir holds only `options.txt` and `stats/`; there is no `saves/`.

The refusal paths are those of `mc_sim::save::import::validate` (block ids in
array order, then entities and tile entities; an unreadable `level.dat`/region
is refused as `Unreadable`) and `mc_client::import::import_world` (validate a
staged in-memory copy, then write).

### 2.4 Extra checks beyond the scripted gate

- **Block placing** (`scripts/s5.json`, `s5/02_placed.png`): a right click
  with dirt placed (−60, 66, −238) (mouse-over moved onto it, stack 5 → 4, the
  region file has id 3 there). An earlier attempt (`s4`) was refused because
  mobs had pushed the player into the target cell. That is correct: vanilla
  refuses to place a block inside the player.
- **Mobs and third person** (`scripts/s4.json`, `s4/01_mobs.png`,
  `02_third_person_sword.png`): the spawned zombie and skeleton burn in
  daylight, the pig draws, the diamond sword is held in first person, and F5
  shows the hurt-tinted player from behind. The skeleton holds no bow, and
  the third-person player shows no sword (§3, D1).
- **Nether** (`scripts/s6.json`, `s6/01_nether.png`): the dev `use_portal`
  action takes the player to dimension −1 at (1.0, 68.62, −21.5) inside a
  generated portal (`DIM-1/` is created). The portal overlay and the
  netherrack and lava behind it draw. A second `use_portal` went to the
  overworld and came straight back (§3, O2).
- **Menus** (`scripts/s7.json`, `s7/01_achievements.png`): the achievement
  map draws the unlockable "Taking Inventory" bright and the rest dark. Done
  returns to the game, as `GuiAchievements.actionPerformed` does in b1.7.3.
  The script then clicked Statistics, Done, Options... and Video Settings...;
  these were no longer on screen, so it logged "no enabled button" and
  `02`–`04` show the game. That is a script error, not a game bug.

## 3. Bugs and visual defects seen

None blocks the gate. D = defect, O = observation (not a bug, or a dev-tool
quirk), N = documentation nit.

- **D1. Third-person held items are not drawn.** Skeletons hold no bow, and
  the player in F5 holds nothing while the first-person view shows the
  sword. Repro: `scripts/s4.json` on a `gate3` world (spawn Skeleton/Zombie
  ahead, `give_item` 276 slot 0, F5); screenshots `s4/01_mobs.png`,
  `s4/02_third_person_sword.png`. This is the integration gap the WP-3.5b
  merge recorded: WP-3.7b's `item_renderer::render_item::HeldItemHook` is
  filled, but nothing inserts WP-3.5b's `render::held_item::HeldItemHook`
  that `RenderPlayer.renderSpecials` / `RenderBiped.renderEquippedItems`
  call. Also unbridged: the third-person `RenderPlayer` draw does not call
  `ItemRenderer::player_model_drawn` (`crates/mc-client/README.md`, "For
  WP-3.7b / Integration status").
- **D2. Achievement and statistics icons draw blocks flat.**
  `gui::menu::draw_item_into_gui` draws every icon as the 2D tile, where
  vanilla draws a lit 3D cube for blocks (`RenderBlocks.renderItemIn3d`).
  WP-3.6b's `gui::render_item::draw_item_into_gui` (lit 3D) exists but has no
  `colorize`/`clip` arguments (WP-3.6c merge note). Visible in
  `s7/01_achievements.png` (the log and workbench icons).
- **O1. The script cannot click a list row by name.** `GuiSlot` centres a
  short list in b1.7.3, so the first `gate3` row is at scaled y ≈ 92–124,
  not at the list top (36). The first scenario 1 run clicked y 50, selected
  nothing and then waited for `in_game` forever. A `click_slot` (row index)
  dev action would make menu scripts robust.
- **O2. The dev `use_portal` round trip re-enters the nether.** The second
  `use_portal` (from the nether, in its generated portal) toggled to the
  overworld and, about 40 ticks later, back to the nether (log: three
  "Toggling dimension!!"). The dev action calls `Minecraft.usePortal()`
  directly. It skips `EntityPlayerSP.onLivingUpdate`'s `timeUntilPortal = 10`,
  and `timeInPortal` had built up while the player stood in the nether portal,
  so the arrival portal sends the player back. Vanilla would do the same if
  `usePortal` were called this way. The real walk-into-a-portal path was not
  exercised at this gate.
- **O3. Achievements and statistics do not persist across launches by
  default.** Without `--username` the name is vanilla's offline
  `Player<millis % 1000>`, so each launch has a new `stats/stats_<name>_unsent.dat`
  and the "Press 'E'…" hint returns. This is vanilla behaviour (owner
  question 9).
- **O4. Import refusal log line names the staging path.** The `WARN` line
  says `cannot read import/level.dat: missing` (`import/` is the in-memory
  staging prefix). The dialog shows "Cannot read level.dat: missing". Cosmetic.
- **N1. README stale note.** The "Menus (WP-3.6c) / Achievements" bullet says
  `gui::render::flush_gui`'s `sort_bias: n` is outweighed by z differences.
  Since WP-3.6b it is `n × 4096` (`gui/render.rs:414`), which the WP-3.6b
  section describes.
- **N2. `docs/mappings.md` duplicates.** `RenderItem.field_27004_a` has two
  rows (lines 148 and 311) that disagree on whether the GUI changes it
  (GuiAchievements does). `BlockStep.field_22037_a` is listed twice (28 and
  175, from before Phase 3).
- **N3. Two overlapping dev actions.** `give` (WP-3.6b; inventory or armour)
  and `give_item` (WP-3.7b; inventory or a given main slot).

Things checked that are vanilla, not defects: the yaw on load (`% 360`); the
crosshair drawn under the pause menu; the achievement popup on the title
screen; Done on the achievements screen returning to the game; a big-tree
branch log (e.g. (−70, 72, −229) on seed 123) showing one face through fancy
leaves (`s1/05`, `s1/07`); the selection outline (1-pixel, 0.4 black,
confirmed by a crop of `s5/02_placed.png`).

## 4. Known gaps and deferred items

`grep -rn 'deferred' crates/mc-client/src | wc -l` → **28**, all D2 (online
services) or D3 (out of scope):

| Area | Count | Items |
|---|---|---|
| Shell (`game.rs`) | 7 | has-paid check and stats sync (D2); applet, multiplayer, snooper (D2/D3); `isMultiplayerWorld` never; pre-McRegion conversion (D3) |
| Texture packs | 6 | `textures/mod.rs` (1), `texture_packs.rs` (3), `assets.rs` (1), the `skin` option in `settings.rs` (1): default pack only (D3) |
| Skins and cloaks | 4 | `getTextureForDownloadableImage` and the `textureSetupComplete` reset (`textures/mod.rs`, 2), deadmau5's ears and the cloak (`render/player.rs`), the preview's skin (`gui/container/inventory.rs`) (D2) |
| Name tags | 2 | other players' name tags (`render/player.rs`, D3) |
| Multiplayer UI | 6 | the title's Multiplayer button and `GuiMultiplayer` (`main_menu.rs`, 3), `GuiSleepMP`'s button (`sleep.rs`, 2), the pause menu's `sendQuittingDisconnectingPacket` (`ingame_menu.rs`, 1) (D3) |
| Online services | 3 | login and session id (`session.rs`), resource downloads (`sound/mod.rs`, `assets.rs`) (D2) |

Recorded gaps that are not `deferred` tags (from the README and the merge
reports):

- **Rendering:** D1 and D2 above; no occlusion queries (`O: 0`, wgpu has
  none); `f32` world-space transforms (jitter far from the origin, no
  floating origin); pass-1 sections sorted by Bevy, not nearest first (shows
  only with fast graphics); `isChunkLit` scoped to the snapshot; the
  inventory preview has no hurt overlay, shadow or fire and duplicates
  `RenderPlayer` code that WP-3.5b now has; anaglyph's second eye not drawn;
  `cameraZoom` ignored.
- **GUI / input** (README "Known deviations", each needing a change outside
  WP-3.6b's files): container close timing (`isUsableByPlayer` runs at the
  start of tick N+1); a screen opened by a click opens at the next tick hook,
  so the rest of that tick's input goes to the game; the portal with an open
  workbench drops the grid once, where vanilla drops it twice; the chest
  minecart cannot be opened (`InventoryRef` has no entity variant).
- **Audio:** within a frame, `ClientEvent`, `ClientSound` and simulation
  sounds are three streams, not interleaved in call order. Panning
  approximates OpenAL's. Audio output was not checked at this gate (no
  listener).
- **Import:** a typed or pasted path; Bevy has no native file dialog (owner
  question 6).
- **Not exercised:** walking into a real nether portal; multiplayer (D3).

## 5. Interface changes to `mc-sim` in Phase 3

All 19 files are listed by `git diff --stat a8f3841..49cd40d -- crates/mc-sim`.
Every fixture still matches (§1). Please approve or overturn.

1. **WP-3.1 — `world/teleporter.rs`** (418 lines, `5f26aa1`):
   `Teleporter.java` as `Teleporter { new(seed) }` with `func_4107_a` (place
   the entity in a portal), `func_4106_b` (find the nearest portal within 128
   blocks) and `func_4108_c` (build one). It is used by the shell's
   `usePortal`. This answers Gate 2 question 7: nether travel went into
   WP-3.1, not Phase 4.
2. **WP-3.1b — the client requests and the `usePortal` split** (`692fd79`,
   `355ab2d`):
   - `world/client_requests.rs`: `ClientRequest` (the `EntityPlayerSP` →
     `Minecraft` calls: `DisplayGui(PlayerGui)`, `ChatMessage`,
     `PickupFx(PickupFx)`, `PlaySoundFx` (the portal sounds),
     `UsePortal { x, z }`, `CloseScreen`, `AchievementHint`), `ClientRequestQueue`, and a new
     field `World::client_requests`.
   - `entity/player.rs`: `display_gui`, `close_screen`, `add_chat_message`,
     `on_item_pickup` (raises the pickup FX), `is_client_player`,
     `use_portal_head` (the dimension toggle, `setEntityDead` and the ×8 / ÷8
     scaling run inside the player's update), `sp_on_living_update_after_portal`,
     and `set_in_portal`.
   - `block_activated` of chest, furnace, dispenser, workbench and bed;
     `block_items` (sign); `minecart`; `player_controller`: the GUI calls now
     raise requests instead of recording a gap.
   - `World::new_for_dimension(old, provider, save_handler, env)` is
     `World(World, WorldProvider)` (World.java:103-143). The save handler is
     passed in (`World::take_save_handler`) because a `Box` cannot be shared;
     the JVM statics move with `adopt_statics`, which now also moves
     `Item.itemRand`.
   - `World::seed_clock_randoms(ClockSeeds { math_random, item_rand,
     entity_rand, container_randoms })`: the shell seeds the `new Random()`
     streams from the clock once per opened save. Fixtures keep using
     `make_deterministic`. This is the shipping-game answer to Gate 1
     question 5.
3. **`block/portal.rs` — `setInPortal` wired** (`692fd79`):
   `on_entity_collided_with_block` now calls
   `entity::dispatch::set_in_portal`, where Phase 2 recorded
   `note_unimplemented("EntityPlayer.setInPortal")`. This was one of Gate 2's
   R2 sites. New fixture `tick/portal-player-collide`.
4. **WP-3.4 — `world/sky.rs`** (434 lines, `5f3d92e`): `getSkyColor`,
   `drawClouds`, `getFogColor`, `getStarBrightness`,
   `calcSunriseSunsetColors`, `func_4096_a`, `getCloudHeight`,
   `func_28112_c`. The module is private, with `pub` methods on
   `World`/`WorldProvider`. These are client-only reads; `get_sky_color`
   takes `&mut self` (the biome manager's caches).

No other `mc-sim` API changed. `mc-core` and `mc-nbt` are untouched.

## 6. Performance baseline

Dev-profile binary (the workspace's `opt-level` 1, dependencies at 3),
Wayland, vsync on, RTX 3080 / Ryzen 7 7700X, default options (render
distance Far, fancy graphics, smooth lighting on, performance Balanced), `testWorld` (`dump_debug`, `scripts/s2b.json`,
`s2b/d03_debug_*.json`, `d04_debug_walk.json`):

| Moment | F3 first line | `C:` sections (drawn/total), F (frustum-culled), E (empty) | Terrain |
|---|---|---|---|
| 200 ticks after reopening, standing | 60 fps, 20 chunk updates | C: 1105/5408, F: 1982, O: 0, E: 2321; pass 1 drawn 304 | 5,913 section rebuilds since load; meshing 3,051 ms + snapshots 762 ms in total (0.52 + 0.13 ms per rebuild); `terrain_frame_ms` 0.20; nothing queued |
| 100 ticks later | 60 fps, 9 chunk updates | same | 64 more rebuilds (animated neighbours, mobs' light); 0.20 ms |
| after walking 200 ticks south (25 blocks) | 60 fps, 439 chunk updates | C: 1045/5408, F: 2026, E: 2337 | 7,414 rebuilds; 16 in the sampled frame with 6 jobs in flight; `terrain_frame_ms` 3.32 |
| first import run, after `set_pos` jumps (`s2/d11_debug.json`) | 60 fps, 127 chunk updates | C: 1158/5408 | 9,752 rebuilds; meshing 5,559 ms, snapshots 1,565 ms in total |

The frame rate sits at the vsync cap throughout, so these numbers bound the
frame time from above (≤ 16.7 ms) and do not measure it. `dump_debug` has no
frame-time distribution. For WP-4.3 a baseline needs an uncapped present
mode and a frame-time histogram (owner question 11). `ServerChunkCache`
reached 867 loaded chunks after the walk.

## 7. What each WP delivered

From `git log a8f3841..49cd40d` (and the earlier `2009a09` range). Each WP
ends with a review-fixes step.

| WP | Commits | Delivered |
|---|---|---|
| 3.1 App shell | `5f26aa1`…`2009a09` (5) | `mc-sim` Teleporter; the Bevy shell in `mc-client`/`mc-app`: `Minecraft.java`'s frame and `runTick` with hook schedules, `Timer` (20 TPS, two-clock sync), `GameSettings`/`options.txt`, LWJGL-shaped input buffers, the camera half of `EntityRenderer` (`getMouseOver`, FOV, bobbing, hurt), native `Storage` (atomic write-then-rename), jar and `resources/` access, `StringTranslate`, F2 screenshots, F3 text, the import (D5) with refusal, `--script` dev mode, `usePortal`/`changeWorld`/`respawn` |
| 3.1b Client requests | `692fd79`, `ab7dfbb`, `355ab2d` | §5 item 2; the shell forwards the requests in call order as `ClientEvent`/`ClientSound` |
| 3.2 Textures | `0a66798`, `12b0ca7`, `6c8179d` | `RenderEngine` as the `Textures` resource (prefixes, missing texture, anaglyph), the terrain/items atlases, every `TextureFX` (lava, water, portal, compass, watch, flowing, fire), the colour maps, `dump_texture` |
| 3.3a Terrain core | `5b87950`…`6ce6f86` (7) | `RenderGlobal`'s section grid and rebuild policy, async meshing from main-thread snapshots, `renderStandardBlock` with and without AO, two passes, the frustum, crack overlay, `renderBlockOnInventory`, `set_option`/`dump_debug` |
| 3.3b Fluids | `1ca92b3`, `a0530d7` | `renderBlockFluids`, `getFlowDirection` on the snapshot, `set_block` |
| 3.3c Plants | `69f0eca`, `1625647` | crossed squares (tall-grass offsets and biome tint), torches, fire, crops, cactus |
| 3.3d Special blocks | `fd5dd1b`, `746ba56`, `82aaf2d` | redstone wire, door, ladder, rails, stairs, fence, lever, bed, repeater, piston base and head |
| 3.4 Sky | `5f3d92e`…`61793bd` (4) | `world/sky.rs`; sky, sunrise, sun, moon, stars, fast and fancy clouds, rain and snow, fog and clear colour, selection box, `GlMaterial` (GL arithmetic on the Java values), `set_time`/`set_rain`/`set_pos` |
| 3.5a Models | `9bf0cae`, `8a74d5e`, `f93f2aa` | every `Model*` as part tables and pose functions, shared part meshes, the lit `ModelMaterial` (with the `setNormal` byte-overflow bug kept), checked bit for bit against the Java models, `debug_model` |
| 3.5b Entity renderers | `19bdf6b`…`17b269b` (6) | `RenderManager` and every `Render*`, tile-entity renderers (sign, spawner, piston), `EntityGl` (GL state and CPU lighting), `renderEntities`, pickups, `spawn_entity`/`equip` |
| 3.6a GUI base | `b568554`, `519d3b4`, `6324be7` | `GuiGl` 2D batcher (the `bevy_ui` spike rejected), `FontRenderer`, widgets, `GuiSlot`, screen binding, the GUI cameras, `debug_gui`/`open_screen`/`mouse_to`/`type_text` |
| 3.6b In-game GUI | `96ed67b`…`4ed0119` (4) | HUD, chat, achievement popup, `RenderItem` GUI half, inventory, chest, furnace, crafting, dispenser, sign editing, sleep and game-over screens, F3 graph, `chat`/`dump_inventory`/`give` |
| 3.6c Menus | `f9ff6ff`, `2c4ea0d`, `49cd40d` | title, pause, options, video settings, controls, select/create/rename/delete world, the D5 import screen and refusal dialog, achievements map, statistics, texture packs (default only), loading screen, `click_button` |
| 3.7a Particles | `ad7322c`, `0a3ea9f`, `eac8e64` | `EffectRenderer` and every `EntityFX`, rain splashes, block-break debris tinting, `particle` |
| 3.7b Hand and items | `cd8deb0`…`6a72090` (6) | sRGB compositing for every camera (GL's byte-space blends), `ItemRenderer` (first-person hand, held items, maps, fire/block/water overlays), `give_item`/`set_fire` |
| 3.8 Audio | `4f2963f`…`708d3a6` (4) | sound pools, `.mus` decoding, the `playAuxSFX` table, `SoundManager` on `bevy_kira_audio` with streamed music, a paulscode/OpenAL model (attenuation, pitch clamp, panning), `aux_sfx` |

## 8. Open owner questions

1. **Sign off Gate 3?** Every scripted criterion passes. The open visual gaps
   (D1 third-person held items, D2 flat block icons) are recorded integration
   follow-ups. Should they be a short WP-3.5c / 3.6d before Phase 4, or part of
   WP-4.1's side-by-side checklist?
2. **PLAN_v4 §8 item 3, the `Entity.rand` rewrite, is still undecided** (Gate 2
   question 3). Hostile natural spawning (K10) and the constructor draws in
   K11 remain unit-tested only. Phase 3 changed nothing here; the shipping
   game now seeds the entity RNG from the clock (`seed_clock_randoms`).
3. **Nether travel landed in WP-3.1 / 3.1b** (Gate 2 question 7):
   `Teleporter.java`, the `usePortal` split, `World(World, WorldProvider)` and
   `setInPortal`. Please confirm. Only the dev `use_portal` was exercised
   (§2.4, O2). Should WP-4.1 cover building and walking through a real portal?
4. **`testWorld` licence** (Gate 1 question 9, still unanswered). The gate
   imports `.reference/mc_b1.7.3-world/testWorld`, which is never committed
   (D8), while the fixtures commit derived data (`golden_save`,
   `tick/testworld-6000`).
5. **The project-owned data directory default**: `$XDG_DATA_HOME/mc_b173`
   (macOS `~/Library/Application Support/mc_b173`, Windows
   `%APPDATA%\mc_b173`), never the real `.minecraft`. Confirm. Also confirm the
   `--jar` / `--resources` defaults (the workspace's `.reference` jar and the
   Prism Flatpak instance).
6. **The import screen is a typed path field** (Bevy has no native file
   dialog; `~/` is expanded; paste works through `arboard`). Accept it, or add
   a native dialog dependency (e.g. `rfd`) in Phase 4? Phase 5's browser build
   will need a different picker anyway.
7. **Approve the `mc-sim` interface changes of §5.** In particular: the
   save-handler hand-over in `new_for_dimension`, `ClientRequest` as the
   `EntityPlayerSP → Minecraft` channel, and `World::client_requests` being a
   public field.
8. Gate 2's other open items: the remaining R2 wiring sites (cake heal,
   `onDeath`'s score and kill hooks, pig → pigman) and the tamed-wolf branch
   of `alertWolves`. `setInPortal` is now done. Schedule them?
9. **Default username**: vanilla's random `Player<millis % 1000>` makes
   stats and achievements per launch (O3). Keep it, or persist a default
   name in `options.txt`/the data dir?
10. **Clean-ups**: merge the `give` / `give_item` dev actions (N3), reconcile
    the duplicate `mappings.md` rows (N2), fix the stale README note (N1),
    and add a `click_slot` dev action (O1). These are small enough for any
    Phase 4 WP.
11. **Performance measurement for WP-4.3**: add an uncapped present mode and
    a frame-time histogram to `dump_debug`, so the next baseline measures
    frame time and not just the 60 fps vsync cap (§6).
