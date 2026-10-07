# Gate 2 review notes (Phase 2, WP-2.1 … WP-2.7 and WP-2.G)

Written 2026-10-06 by the Gate 2 reviewer for the owner's review. PLAN_v4 §5
defines Gate 2 as: "a headless scripted session (generate a world, walk, dig,
place, craft, fight, sleep, save, reload) matches the oracle trace; `testWorld`
ticks for 6,000 ticks with periodic digests equal to the oracle's".

Phase 2 was integrated on one branch, `phase-2`. It was forked from the Gate 1
commit `66c6d09` ("Phase 1 gate: docs/GATE1_REVIEW.md") and its tip was `c415e67`
before this document. It holds 86 linear "WP-x.y step N" / "WP-x.y merge"
commits, rebased and fast-forwarded from the per-WP branches `wp-2.1`, `wp-2.2`,
`wp-2.3a/b/c`, `wp-2.4a/b`, `wp-2.5a/b`, `wp-2.6`, `wp-2.7` and `wp-2.G`. All of
those branches still exist. Nothing is pushed.

Size of Phase 2: 317 files, +94,739 / −1,162 lines. `mc-sim` is now 62.9k lines
of source and 13.6k lines of tests (Gate 1: 40.1k and 7.1k).

**Verdict.** Gate 2 is not green. Its second criterion passes and its first
fails on one recorded gap.

- **`testWorld` 6,000 ticks: pass.** `tick/testworld-6000` matches the oracle on
  all 86 lines (§3.2).
- **The scripted session: fails, `#[ignore]`d.** `tick/gate2-session` covers
  every step of the gate: walk, dig, place, craft, fight, sleep, save and reload.
  Lines 1–744 of its 3,237 match. Line 745 (t=78) differs because the player
  does not pick up the dirt it dug. `EntityItem.onCollideWithPlayer` still
  records `InventoryPlayer.addItemStackToInventory` instead of calling it, and
  no Phase 2 WP owned that call.

**Experiment (not committed).** I wired that one call into
`entity::item::on_collide_with_player` (11 lines, §3.1). With it, the whole
fixture matches: all 3,237 lines, including the reload and the final save.
Every other test in the workspace still passes.

Every Phase 1 fixture that Gate 1 deferred to Phase 2 now passes. All other
218 fixtures pass. The owner should decide whether to accept Gate 2 as it
stands or after a small follow-up that closes the pickup gaps (§8 question 1).

## 1. Harness acceptance and gate commands

Run in the main checkout at `c415e67`. Nothing needed fixing, so this gate made
no fix commits.

| Check | Result | Evidence |
|---|---|---|
| `cargo test --workspace` | pass | 652 passed, 0 failed, 1 ignored (`tick_gate2_session`) over 51 test binaries and 5 doc-test runs; `golden_tick` 185 passed, 1 ignored |
| `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` | clean | exit 0 |
| `cargo fmt --all --check` | clean | exit 0 |
| `cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown` | pass | exit 0 (D9) |
| `sh tools/oracle/regen.sh` | no drift | exit 0; `git status --short tests/golden` was empty afterwards |
| §4.4 item 1: `tools/oracle/check-reproducible.sh` | pass | "3 runs byte-identical (221 files)" |
| §4.4 item 2: `tools/oracle/check-sensitivity.sh` | pass | 219/219 fixtures `ok`: all 202 seeded fixtures change, all 17 unseeded ones stay identical. Rust side: `golden_tick::flat_item_drop_with_perturbed_seed_fails`, `golden_worldgen::overworld_with_perturbed_seed_fails`, `golden_block_table::block_table_comparison_is_sensitive` |
| §4.4 item 3: decoded first difference | pass | e.g. `tick/gate2-session: golden mismatch at line 745 (t=78, e="entity"), field state.inventory` with both values printed (§3.1) |
| §4.4 item 4: `testWorld` | pass | `golden_save::testworld_chunk_digest_and_chest` and `save_testworld` (unchanged since Gate 1) |
| **Gate 2: `testWorld` 6,000 ticks** | **pass** | §3.2 |
| **Gate 2: scripted session** | **fail (ignored)** | §3.1 |
| Ignored fixtures still fail as their reasons say | confirmed | `cargo test -p mc-sim --test golden_tick -- --ignored tick_gate2_session` fails at line 745, as the reason states |

The oracle still prints `harness: warning: chunk (1,0) already loaded before
0,0; the "generated" stage is not pure generator output` for the `*-42-spread`
worldgen fixtures. This is expected, as at Gate 1.

## 2. Fixtures

There are 219 fixture directories under `tests/golden` (Gate 1 had 131). A Rust
test runs every one of them, so no fixture lacks a test. The appendix lists
each fixture with its test.

| Tier | Fixtures | Pass | Ignored | Rust tests |
|---|---|---|---|---|
| A | 32 (rng 3, math 3, hash 6, nbt 5, block 1, item 1, recipe 1, stat 1, worldgen 11) | 32 | 0 | mc-core `golden_rng`, `golden_hash`, `golden_sine_table`, `math_sweep_report`; mc-nbt `golden_nbt`; mc-sim `golden_block_table`, `golden_item_table`, `golden_recipe_table` (new), `golden_stat_table` (new), `golden_worldgen` |
| B | 90 (light 4, tick 86) | 90 | 0 | mc-sim `golden_tick` |
| C | 89 (entity 62, tick 27) | 89 | 0 | mc-sim `golden_tick` |
| S | 8 (save 3; tick 5: `dungeon-chest-save`, `chest-container-clicks`, `furnace-dispenser-clicks`, `reload-roundtrip`, `gate2-session`) | 7 | 1 | mc-sim `golden_save` (the 3 `save/*`), `golden_tick` (the 5 tick ones, listed in `golden_save::COVERED_IN_GOLDEN_TICK`) |
| **Total** | **219** | **218** | **1** | |

Two Tier C fixtures, `entity/sheep-shear` and `entity/wolf-pack-retaliates`,
also have a `save` op and emit Tier S lines. They are listed in
`COVERED_IN_GOLDEN_TICK` too.

These coverage checks fail when a fixture directory has no test:
- `golden_tick::every_tick_and_entity_fixture_has_a_test`
- `golden_worldgen::every_worldgen_fixture_has_a_test`
- `golden_save::every_tier_s_fixture_has_a_test`
- `golden_nbt::every_nbt_fixture_is_covered`
- `golden_hash::all_hash_fixtures_listed`
- `golden_rng::all_rng_fixtures_listed`

The `math`, `block`, `item`, `recipe` and `stat` areas still have no such check
(Gate 1 nit N1, now five areas).

### 2.1 Gate 1's deferred fixtures

All seven fixtures that Gate 1 §2.1 moved to the Phase 2 gate pass, and so do
the four Tier C fixtures from Gate 1 §2.2.

| Fixture | Closed by |
|---|---|
| `tick/block-item-placement`, `tick/redstone-item-placement`, `tick/bonemeal-sapling` | WP-2.6 step 2 (item behaviour dispatch) |
| `tick/mob-spawner-spawn` | WP-2.3c (EntityList constructors, the spawner loop) |
| `tick/tnt-by-fire` | WP-2.4a (`EntityTNTPrimed`) |
| `tick/sapling-with-player` | WP-2.3a/c and WP-2.5a (the spawner's mobs, the player's update) |
| `save/tile-entities` | WP-2.5a (the player's update) |
| `entity/arrow-flight`, `entity/flat-pig-falls`, `entity/pig-attacked`, `entity/pig-falls` | WP-2.4a, WP-2.2, WP-2.5a |

Phase 2 also ignored some fixtures for a while and then un-ignored them:
- `tick/testworld-6000` (WP-2.5a)
- `tick/natural-spawn-pigs`, `tick/sleep-spawning` (WP-2.3c)
- `entity/wolf-begs-bone` (WP-2.3a)

All four pass.

### 2.2 The one ignored fixture

| Fixture | Tier | `#[ignore]` reason (golden_tick.rs:254, verbatim) |
|---|---|---|
| `tick/gate2-session` | S | "needs EntityItem.onCollideWithPlayer's pickup (a recorded gap: entity::item::on_collide_with_player records InventoryPlayer.addItemStackToInventory instead of calling it; WP-2.6's item behaviour is merged but cannot be reached past it): first diff line 745 (t=78 entity 0 state.inventory: the oracle's player picks up the dug dirt); lines 1-744 match (walk, dig, shovel wear, the parked zombie)" |

## 3. The two Gate 2 criteria

### 3.1 The headless scripted session: `tick/gate2-session` (fail)

**What the scenario does** (`tests/golden/tick/gate2-session/scenario.json`;
Tier S; 411 ticks).
- **World:** seed 123 generated terrain, difficulty 1, natural spawning off,
  saved through `SaveOldDir`.
- **Walk:** forward for 60 ticks.
- **Dig:** grass with a stone shovel (`dig` ops), then walk into the hole to
  pick up the dirt.
- **Place:** dirt with `use_item`.
- **Craft:**
  - logs → planks → sticks → crafting table by 17 `slot_click`s in the 2×2
    grid;
  - three `craft` ops;
  - the crafting table placed.
- **Fight:** a zombie spawned far away at t=0, moved next to the player by
  `set_pos`, and killed with three sword attacks.
- **Eat:** bread.
- **Sleep:** night set, the bed placed with `ItemBed.onItemUse`, then `sleep`
  (status `OK`). `World.tick` skips to the morning and wakes the player.
- **Save, reload:** `save` at t=310, then the `reload` op: 429 chunks are loaded
  again and the player gets a new id, 2045.
- **After the reload:** 100 more ticks and a final `save`.
- **Dumps:**
  - every tick: `rng_state` and the player;
  - every 20 ticks: inventory, every entity, and the 3×3 chunk digests;
  - every 10 ticks: `player_state`;
  - block dumps at fixed ticks.

**Result.**

```
$ cargo test -p mc-sim --test golden_tick -- --ignored tick_gate2_session
tick/gate2-session: golden mismatch at line 745 (t=78, e="entity"), field `state.inventory`:
  expected: "f74e571a149a3cebd3a2236a9388dc72b5b7eb971c25aa649a98d15cd429b8d3"
  actual:   "e27ea4c29e9c8f7a52151a99b26b4c9545983d58936dd97fe24d568dbdc4f7d9"
test result: FAILED. 0 passed; 1 failed
```

Every other field of line 745 is equal: position, motion, `rand`, health,
`currentItem`. Only the inventory hash differs.

**System: item pickup** (`EntityItem.onCollideWithPlayer`, EntityItem.java:110-130).
`crates/mc-sim/src/entity/item.rs:196-215` reads `delayBeforeCanPickup` and then
records `"InventoryPlayer.addItemStackToInventory (EntityItem.onCollideWithPlayer)"`
instead of picking the item up, even though
`InventoryPlayer::add_item_stack_to_inventory` exists since WP-2.5b.

I logged the recorded gaps for every fixture, with a temporary hook in the tick
driver that I reverted afterwards. In this run the gap was hit 115 times, once
for each player–item collision after the pickup delay ran out. The same gap exists in
`EntityArrow.onCollideWithPlayer` (arrow.rs:510-527) and in
`EntitySnowball`/`EntityEgg.onCollideWithPlayer` (snowball.rs:332). No WP
brief owned the call: WP-2.4a left it to "WP-2.5", and WP-2.5 never wired it
back.

**Experiment (reverted, not committed).** The vanilla body without the two
achievement triggers. `onItemPickup` is only the client's particle; the
oracle's `HarnessPlayer` runs `Entity`'s empty body.

```rust
if d.delay_before_can_pickup == 0 {
    let mut stack = d.item.unwrap();
    let ok = player.kind.player_mut().unwrap().inventory.add_item_stack_to_inventory(&mut stack);
    data(e).item = Some(stack);
    if ok {
        let p = ((e.base.rand.next_float() - e.base.rand.next_float()) * 0.7 + 1.0) * 2.0;
        world.play_sound_at_entity(&e.base, "random.pop", 0.2, p);
        if stack.stack_size <= 0 { e.base.set_entity_dead(); }
    }
}
```

With this change, `cargo test -p mc-sim --test golden_tick -- --include-ignored`
gives 186 passed, 0 failed, so `tick/gate2-session` matches on all 3,237 lines.
`cargo test --workspace` still gives 652 passed, 0 failed. Nothing else in the
session diverges. The rest of the session already matches the oracle:
- placing (`ItemBlock`, `ItemBed`);
- crafting by `slot_click` and the `craft` op;
- the sword's `hitEntity` and wear;
- the zombie's death and drop roll;
- eating (`ItemFood`);
- sleeping through the night;
- both Tier S saves and the reload.

As the task rules require, this gate reports the failure and does not patch it.
The fix belongs in a WP with a reviewer: the snippet leaves out the
`mineWood`/`killCow` achievements, and the arrow and snowball sites need the
same change.

### 3.2 `testWorld` for 6,000 ticks: `tick/testworld-6000` (pass)

**What the scenario does.** It loads `tests/golden/save/testworld/world` with
the loading constructor (`"world": "load"`) and places the player at the spawn
point (21.5, 65, −63.5). It then runs 6,000 ticks with natural spawning off. The
run includes:
- random ticks over 19×19 chunks;
- scheduled ticks;
- the 24 saved mobs and items of `r.0.-1`;
- weather;
- autosaves every 40 ticks;
- `randomDisplayUpdates`;
- generation and population of the chunks missing from the one committed
  region.

Every 500 ticks it dumps the four chunks around spawn, `rng_state`, `world_time`
and entity 0.

```
$ cargo test -p mc-sim --test golden_tick -- --exact tick_testworld_6000
test result: ok. 1 passed; 0 failed; ... finished in 9.55s
```

`target/golden/tick/testworld-6000/actual.jsonl` equals `expected.jsonl` on every
line after sorting keys (86 lines: 48 `chunk_digest`, 12 `rng_state`, 12
`world_time`, 9 `entity`, 3 `entity_absent`, the preload line and the op line).
The final state agrees:

```
{"t":5999,"e":"rng_state","world_rand":70924232732405,"lcg":562145689,"math_random":250432830115496}
{"t":5999,"e":"world_time","time":7751}
```

**Finding R1: the fixture never dumps the player.** The scenario's comment says
"the player (id 0)", and its `entity_state` dump has `"which": [0]`. In this
world, though, the saved entities of the preloaded chunks take ids 0–3 first.
The op line reads `{"op":"player","id":4,"joined":true}`. Entity 0 is a saved
`EntityItem`: the `health` 5 seen in every entity line is `EntityItem.health`.
It despawns at age 6000, which is why `entity_absent` appears from t=4999 on.

**Stronger check made at this gate (not committed).**
1. I copied the scenario into the scratchpad and changed its dumps: the real
   player (`which: [4]`) and `player_state` every 100 ticks, `inventory` every
   500, and every entity every 1000.
2. I ran the oracle on the copy: 267 lines.
3. I replayed it through `tick_driver::drive_in` in a throwaway test, deleted
   afterwards. **All 267 lines match.**

The player stays alive at health 20 for all 6,000 ticks. Gate 2's second
criterion therefore holds for the player as well. Question 2 proposes fixing
the committed scenario.

## 4. Recorded gaps left in the code

### 4.1 Dispatch coverage tests

| Dispatch | Test output (`cargo test -p mc-sim --test <t> -- --nocapture`) | Override set |
|---|---|---|
| Block (`block/dispatch.rs`) | `0 overrides recorded as not transcribed, 306 transcribed in dispatch, 13 data methods` | `tests/data/block_overrides.txt` (512 overrides) |
| Entity (`entity/dispatch.rs`, `Entity` methods) | `19 entity overrides recorded as not transcribed, 115 transcribed in dispatch` | `entity_overrides.txt` (214) |
| Living (`entity/dispatch.rs`, `EntityLiving`/`EntityCreature` methods) | `0 living overrides recorded as not transcribed, 112 transcribed` | `living_overrides.txt` (116) |
| Item behaviour (`item/behaviour/mod.rs`) | no `Item*` gap from any entry point. 31 behaviour overrides are transcribed (`onItemUse` 13, `onItemRightClick` 8, `hitEntity` 3, `onBlockDestroyed` 3, `saddleEntity` 2, `onUpdate`, `onCreated`); the other 33 are data methods. The probes reached one non-item record, `EntityPlayerSP.displayGUIEditSign` | `item_overrides.txt` (64) |

The 19 entity pairs (printed by a temporary `eprintln!` in
`entity_dispatch_coverage`, reverted):

| Pairs | Count | Vanilla caller | Class | Owner |
|---|---|---|---|---|
| `getShadowSize`: Arrow, Egg, FallingSand, Fireball, Fish, Snowball, TNTPrimed | 7 | `Render.java` | render only | Phase 3 (WP-3.5) |
| `isInRangeToRenderDist`: Egg, Fireball, Fish, Snowball; `isInRangeToRenderVec3D`: LightningBolt | 5 | `Entity.isInRangeToRenderVec3D`, `RenderGlobal` | render only | Phase 3 (WP-3.4/3.5) |
| `getCollisionBorderSize`: Fireball | 1 | `EntityRenderer.getMouseOver` (picking) | client only | Phase 3 (WP-3.1) |
| `setVelocity`: Arrow, Egg, Fish, Snowball; `setPositionAndRotation2`: Fish | 5 | `NetClientHandler` only | networking, deferred (D2) | none (D2) |
| `onStruckByLightning`: **EntityPig** | 1 | `EntityLightningBolt.onUpdate` | **real gap**: the pig dies but no `EntityPigZombie` takes its place, although that class exists since WP-2.3b (pig.rs:95-104) | follow-up (§8 question 1) |

So 18 of the 19 are client or network methods, and one is a real gap that
`World.tick` can reach (a thunderstorm bolt striking a pig).

### 4.2 `World::skipped_systems` / `World::unimplemented_hits` strings

These come from a grep of `skip(` / `note_unimplemented(` in `crates/mc-sim/src`
and `crates/mc-golden/src` (113 call sites). "From tick" means the string can be
reached from `World.tick`, `World.updateEntities` or the harness `runTick` with
no player action. "Hit by" lists the fixtures that reached the string in the
logged run (§3.1).

**Real gaps** (simulation-visible; none blocks a passing fixture):

| String | Site | From tick | Hit by | Note |
|---|---|---|---|---|
| `InventoryPlayer.addItemStackToInventory (EntityItem.onCollideWithPlayer)` | entity/item.rs:210 | yes | `tick/gate2-session` (115) | blocks Gate 2 (§3.1) |
| `InventoryPlayer.addItemStackToInventory (EntityArrow.onCollideWithPlayer)` | entity/arrow.rs:523 | yes | — | the arrow is never picked up |
| `InventoryPlayer.addItemStackToInventory (EntitySnowball/EntityEgg.onCollideWithPlayer)` | entity/snowball.rs:332 | yes | — | vanilla gives an arrow here (transcribed vanilla bug) |
| `EntityPig.onStruckByLightning` | entity/pig.rs:102 | yes (thunder) | — | §4.1 |
| `EntityWolf.isWolfTamed (EntityPlayer.alertWolves)` | entity/player.rs:725, :742 | yes (a mob hurts the player) | — | every wolf is treated as untamed, so a tamed wolf never defends its owner. The wolf exists since WP-2.3a |
| `EntityPlayer.addToPlayerScore`, `EntityPlayer.onKillEntity` | entity/living.rs:772, :778 | yes | `entity/player-attack-pig`, `tick/gate2-session` | `scoreValue` is 0 for every b1.7.3 mob, so only the `mobKills`/`playerKills` stats and the `killEnemy` achievement are lost (stat file only). `dispatch::add_to_player_score` and `player::on_kill_entity` exist; `onDeath`'s shared-reference attacker is not wired to them |
| `EntityPlayer.setInPortal` | block/portal.rs:185 | yes (entity collision) | — | **wiring gap**: `entity::dispatch::set_in_portal` → `player::set_in_portal` exists since WP-2.5a, but `BlockPortal.onEntityCollidedWithBlock` still records. A player in a portal never gets `inPortal`/`timeUntilPortal` (visible in `player_state`) |
| `EntityLiving.heal heartsLife` | block/cake.rs:129 | no (`blockActivated`) | — | cake uses a private copy of `heal` that skips `heartsLife = heartsHalvesLife / 2`. `living::heal` exists. Probably unobservable: the value 10 never exceeds the `> heartsHalvesLife / 2` test |
| `Minecraft.usePortal` | entity/player.rs:1318 | only with `client_additions` | — | nether travel (`Minecraft.usePortal`, `Teleporter.java`) is not transcribed and no WP names it (question 7) |

**Client only (Phase 3):**

| String | Site | Owner |
|---|---|---|
| `EntityPlayerSP.displayGUIChest/Furnace/Dispenser/EditSign` | block/chest.rs:201, furnace.rs:132, dispenser.rs:85, minecart.rs:1018, item/behaviour/block_items.rs:302 | WP-3.6 (the oracle's `EntityPlayer` bodies are empty, so traces agree) |
| `ColorizerGrass.getGrassColor`, `ColorizerFoliage.getFoliageColor` | block/grass.rs:88, leaves.rs:131 | WP-3.3 |
| the 18 entity render and network arms of §4.1 | entity/dispatch.rs:662-830 | WP-3.x / D2 |

**Defensive** (not work items): guards for the take-out/put-back entity pattern
and for Java exceptions. The strings:
- `World.getEntitiesWithinAABB[ExcludingEntity] over a taken-out entity`
- `Chunk entity list holds an entity not in loadedEntityList`
- `Entity ref to an entity no longer in loadedEntityList`
- the `… out of its slot` / `… not in its slot` records in base.rs, boat.rs,
  minecart.rs, wolf.rs, creature.rs, mob.rs, spider.rs, skeleton.rs, ghast.rs,
  arrow.rs, fish.rs, lightning_bolt.rs, living.rs and player.rs
- `… taken-out player` in tick.rs, spawner.rs, world/entities.rs and
  world/mod.rs
- `… (Java NullPointerException)` in painting.rs, fish.rs, nbt.rs and player.rs
- `TileEntityMobSpawner.updateEntity: ClassCastException …` and `… did not join`
- `ChunkLoader.storeChunkInCompound: a section lists an entity not in loadedEntityList`
- `Container over a tile entity no longer in the world`
- `EntityFish.catchFish: bobber …`
- `HarnessWorld.runTick randomDisplayUpdates around a player no longer in loadedEntityList`

Two of these were hit by passing fixtures:
- `World.getEntitiesWithinAABB over a taken-out entity`: 7 fixtures plus
  `gate2-session`, e.g. `entity/spider-leap` (14 hits) and
  `tick/sword-pickaxe-durability` (8);
- `… ExcludingEntity over a taken-out entity`: 4 fixtures plus
  `gate2-session` (`tick/bed-nether`, `tick/block-item-placement`,
  `entity/pigzombie-anger`, `tick/place-stairs-slabs-doors-beds-signs`).

See R5.

**Dead or unreachable** (Gate 1 nits, still open):
- `TileEntity*.writeToNBT/readFromNBT/updateEntity` (tile_entity/mod.rs:200, :269,
  :297): `Untranscribed` is never constructed (N3).
- `World.addLoadedEntities (chunk-stored entities)` (world/entities.rs:185, the
  only `skip(` left): an invariant check, and its doc is stale (N4).
- `EntityList` `FromNbt::Gap` and `construct_gap` (save/entity_list.rs:281-301)
  with the "preserved" list in chunk_loader.rs:124. `missing_constructors()` is
  empty and asserted empty (`golden_save::stored_entities_construct_and_mob_is_skipped`),
  so Gate 1's K6 path cannot run.
- `Entity*.readEntityFromNBT` (entity/nbt.rs:458, the `other =>` arm): every
  saved kind has its own arm. Probably unreachable; not verified.
- Five `untranscribed(world, what)` tails in block/dispatch.rs (lines 1090,
  1160, 1199, 1556, 1979) after all-`None` matches (N2).

### 4.3 What the logged run reached

Across all 186 `golden_tick` fixtures (gate2-session included) and the
`golden_save` fixtures, the only recorded strings reached were these:
- the pickup (gate2-session);
- `addToPlayerScore`/`onKillEntity`;
- `displayGUIChest` and `displayGUIEditSign`;
- the two taken-out-entity guards.

`skipped_systems` stayed empty everywhere. No passing fixture reaches a real
gap that changes its trace.

## 5. Decisions the orchestrator took during Phase 2 (confirm or overturn)

1. **The workflow.**
   - All Phase 2 work landed on `phase-2`. Each WP ran in its own worktree and
     branch.
   - WP-2.3 was split into a, b and c, WP-2.4 into a and b, and WP-2.5 into a
     and b.
   - The gate got its own package, **WP-2.G**, which PLAN_v4 §5 does not list:
     the `reload` op, the reload fixtures and `tick/gate2-session`.
   - Merge fallout went into "WP-x.y merge" commits.
   - Phase 2 started with Gate 1's questions unanswered in the tree.
2. **Oracle harness growth.** `tools/oracle/rewrite-math-random.sh` is
   unchanged, so `Math.random()` is still the only rewrite of vanilla code. The
   Java harness grew by +1,150 / −24 lines in 7 files:
   - new Tier A kinds `recipe_table` (RecipeTable.java) and `stat_table`
     (StatTable.java: a fresh class loader with an identity translation table,
     so stat names are not localised);
   - new ops `interact`, `input`, `dig`, `sleep`, the player branch of
     `attack`, `craft`, `slot_click` and `reload`;
   - the `player` op's `inventory`, and `"world": "load"`;
   - new dumps `player_state`, `inventory` and `map_data`;
   - `check-sensitivity.sh` lists the two new unseeded kinds.

   Notes on the new ops:
   - `dig` cannot use the vanilla `PlayerControllerSP`, which reaches the
     world through `mc.theWorld`. `HarnessWorld.Controller` mirrors it line for
     line, leaving out only the dig step sound.
   - `use_item`, `interact` and `attack` run the vanilla controller code with
     the op's block or entity in place of the ray trace.
   - `slot_click` composes the chest inventory as `BlockChest.blockActivated`
     does, copied into the harness because the last step,
     `displayGUIChest`, is empty for `EntityPlayer`.

   Everything is recorded in `tests/golden/README.md` and
   `tools/oracle/README.md`.
3. **The `reload` protocol** (WP-2.G; README "Reload").
   - The old world is saved with `saveWorld(true, null)` and dropped, then the
     save is opened again with the loading constructor.
   - Carried over: the `World.rand` object, `field_9437_g` and `soundCounter`.
     These are clock-seeded in vanilla.
   - No static is reset: `nextEntityID`, `nextTickEntryID`, the `Math.random()`
     stream and the singleton Randoms go on.
   - The old `chunkList` is loaded again in order, with the light queue drained
     after each chunk.
   - A new `HarnessPlayer` joins through `spawnPlayerWithLoadedChunks`.
   - **Harness choice:** the old world's provider is passed instead of `null`.
     With `null`, vanilla takes the dimension from the `Player` compound's
     `Dimension`, and the harness player never sets `Entity.dimension`, so a
     nether save would reopen as the surface (`tick/reload-nether` pins this).
     In the real game a nether player's compound says −1, so this choice only
     covers a harness shortcut.
   - Scheduled ticks are not saved by b1.7.3 and are dropped on both sides
     (`tick/reload-roundtrip`).
4. **`HarnessPlayer` and the port's `client_additions` flag.** The oracle's
   player extends `EntityPlayer`, because `EntityPlayerSP` needs a
   `Minecraft`. It is given:
   - `MovementInput`, applied where `EntityPlayerSP` applies it;
   - `isSneaking`;
   - the username `"Player"`, which `EntityPlayer.onDeath` dereferences
     (EntityPlayer.java:223).

   **Not modelled:**
   - the rest of `EntityPlayerSP.onLivingUpdate`: the portal timer and
     `Minecraft.usePortal`, the sneak `ySize` dip, and `pushOutOfBlocks`, which
     moves the player;
   - `addStat`, which is `EntityPlayer`'s no-op.

   The port has these behind `PlayerSpData::client_additions`. The golden
   driver leaves the flag off, so these `EntityPlayerSP` parts are checked by
   unit tests only (`crates/mc-sim/tests/player.rs`).
5. **Statistics on the player** (WP-2.7). `Minecraft.statFileWriter` is carried
   as `PlayerData::stat_file_writer`, and `entity::player::add_stat` runs
   `EntityPlayerSP.addStat`.
   - The port always counts stats, while the oracle's `HarnessPlayer` counts
     none. No dump reads them, so the traces agree.
   - Stat behaviour is checked by `crates/mc-sim/tests/stats.rs`, and the
     registries by `stat/stat-table`.
   - Merge consequence: `add_stat_if_player` and the `ItemTool`/`ItemSword`/
     `ItemShears` functions now take `&mut Entity`, and
     `ItemStack::damage_item_by` adds the break stat.
6. **Spawn kinds beyond `EntityList`'s `(World)` constructors** (WP-2.4a/b,
   2.3b).
   - Harness-defined `Egg`, `Fireball` and `LightningBolt`. The bolt goes
     through `addWeatherEffect`, so it never appears in `entity_state`; its
     shape draws are replayed; it is refused at `difficulty` ≥ 2, where the
     constructor places fire with the unseeded `rand`.
   - Harness-defined `FishingHook`: `EntityFish(world, player)`, with the
     heading Gaussians replayed.
   - `Painting` with `direction`: the art draw is replayed.
   - `Minecart` with `type`.
   - Slime size and jump delay are replayed.

   In each case the oracle replays the constructor's draws on the seeded stream
   so that it matches the port, whose `rand` is seeded at construction.
7. **Item and map state as world fields** (WP-2.6).
   - `Item.itemRand` → `World::item_rand`, seeded `mix(seed, −1)`.
   - `MapStorage` → `World::field_28108_z`.
   - `ISaveHandler.getMapFile` → `SaveHandler::{has_map_files, read_map_file, write_map_file}`.
     These lost their default bodies, so a wrapper cannot drop them.
   - `HarnessSaveHandler` forwards them to `SaveOldDir`.
   - The `map_data` dump reads `MapStorage.loadedDataMap` by reflection, so no
     map file is read.
8. **Skeleton interface changes** (§6: the owner approves anything touching the
   0.5 skeleton's interfaces):
   - the `SaveHandler` map-file methods (7);
   - `World::adopt_statics`, `adopt_clock_seeded_state`, `set_chunk_generator`,
     `loaded_chunk_coords`, `loaded_chunk_count` (reload);
   - `dispatch::get_can_spawn_here` and `dispatch::is_in_water` now take
     `&mut Entity` (the squid's `isInWater` pushes it, a transcribed vanilla
     bug);
   - `EntityList`'s `CONSTRUCTORS` table and `new_instance`;
   - `PlayerData::stat_file_writer`;
   - `PlayerSpData::client_additions` and `last_sleep_status` (harness-only
     state, marked "Not vanilla state").
9. **Work outside a brief.** `EntityPlayer.getEyeHeight` landed in WP-2.3b
   (`b9b14f4`, a separate commit "so the orchestrator can take it or leave it to
   WP-2.5"), because the mobs' sight and aim read it. It was kept.
10. **How `tick/gate2-session` was built.**
    - Difficulty 1 with `"mob_spawning": "none"`.
    - The zombie is spawned at t=0, 41 blocks away and out of target range, so
      its id does not depend on the terrain and the flipped-seed sensitivity
      run still works. `set_pos` then brings it to the player at t=110.
    - The Tier S lines cover only chunks (−2, −4) and (−2, −3), which hold
      every edit. The rest is compared by `chunk_digest` every 20 ticks.
    - The night is skipped by `World.tick`'s all-asleep path, not by
      `set_time`.
11. **Sensitivity workarounds.** The wolf bone fixtures interact at ticks where
    the taming roll fails under both seed 123 and the flipped seed 122
    (`05b7a23`). At the time, a taming threw `NullPointerException` in the
    oracle because `HarnessPlayer` had no username. The username was added in
    WP-2.5a.
12. **`testworld-6000` runs with natural spawning off.** Its saved mobs tick,
    but `SpawnerAnimals.performSpawning` never runs in the long run.
13. **Commit trailers.** The briefs asked for `Co-Authored-By: Claude Fable
    5.1`. `66c6d09..c415e67` has 85 Opus 5.5 trailers and 1 Fable 5.1 trailer
    (`33214b7`, WP-2.3a step 8). The merge agents used Opus 5.5, the model
    actually running, and so does this document's commit.

## 6. Known divergences from vanilla (deliberate or recorded)

Status of Gate 1's list:

| # | Divergence | Status at Gate 2 |
|---|---|---|
| K1 | `SpawnerAnimals` mob constructors not made | **closed** (WP-2.3c). spawner.rs's "Known divergence" module doc is now stale (R3) |
| K2 | `TileEntityMobSpawner` stops at `createEntityInWorld` | **closed** (WP-2.3c) |
| K3 | `randomDisplayUpdates`' `new Random()` is one seeded `display_random` stream | by design (D9) |
| K4 | Clock-seeded Randoms seeded instead (`ExplosionRNG` `mix(seed, −6)`, dispenser `mix(seed, −7)`) | unchanged; reload adds the carried-over `World.rand`/`field_9437_g`/`soundCounter` (§5.3) |
| K5 | `EntityLightningBolt` not constructed | **closed** (WP-2.4a); its draws are unseeded in the oracle (K11) |
| K6 | Unconstructible saved entities kept and written back unchanged | **unreachable** (every constructor exists); dead code (R4) |
| K7 | Eager region decompression | accepted |
| K8 | `use_item`: `onItemUse` taken as false | **closed** (WP-2.6) |
| K9 | Oracle `Math` == `StrictMath` | owner decision pending (PLAN_v4 §8 item 4) |
| K10 | Hostile spawning unseeded in the oracle; harness `difficulty` 0 | **open**, PLAN_v4 §8 item 3 (question 3) |

New in Phase 2:

| # | Divergence or unverified area | Where | Effect |
|---|---|---|---|
| K11 | **Irreproducible constructor draws** (census §3). Draws that happen in a constructor before the oracle's join hook seeds `Entity.rand`; the port draws them on its construction-seeded stream, so the values cannot be compared. Fixtures avoid tracing them. Skeleton arrows are reproducible: `setArrowHeading` runs again after the join (skeleton.rs:15-20) | the sites below | behaviour unverified by fixtures, checked by unit tests only |
| K12 | `EntityPlayerSP` client parts off in every golden run (§5.4) | entity/player.rs `client_additions` | `pushOutOfBlocks`, the sneak `ySize` and the portal timer are unverified against the oracle |
| K13 | Stats counted by the port, never by the oracle (§5.5) | stats/, `PlayerData::stat_file_writer` | stat values are unverified against vanilla, except the registries |
| K14 | Reload keeps the old world's provider (§5.3) | tick_driver.rs:1066, HarnessWorld.reload | harness only |
| K15 | The real gaps of §4.2 | §4.2 | the pickups (Gate 2), pig→pigman, wolf defence, portal entry, kill stats |

The K11 sites:
- bow, dispenser and player-thrown arrow heading jitter (EntityArrow.java:55-57);
- snowball, egg and bobber heading jitter;
- the ghast fireball's acceleration jitter (ghast.rs:17-22);
- `ItemPainting`'s art choice;
- the jump delay of slimes made by a split (slime.rs:22-32);
- the lightning bolt's shape, lifetime and fire (tick.rs:344);
- the `getCanSpawnHere` rolls of `EntityMob`, `EntityGhast` and `EntitySlime`.

**Vanilla bugs transcribed.** There are 61 `vanilla bug` markers in `crates/`,
up from 34 at Gate 1. The ones Phase 2 added cover:
- **Inventory and crafting:**
  - the whole held stack goes to `onPickupFromSlot` / `onCrafting`;
  - a partly taken crafting output loses its rest;
  - `mergeItemStack`'s copy;
  - the cow replaces a whole bucket stack.
- **Mobs:**
  - the ghast's stale waypoint distance and its fireball's unmoved box;
  - the slime's health reset to size² on load, and its split needing health
    exactly 0;
  - `EntityMob`'s reference comparison with a null attacker;
  - the pigman's anger that never decays;
  - the wolf's unchecked `attackTime`;
  - the squid's `isInWater`, a query with a side effect.
- **Projectiles and items:**
  - the snowball's pickup giving an arrow;
  - the arrow's unset `xTile`/`yTile`/`zTile`;
  - the peaceful heal's `ticksExisted % 20 * 12`;
  - the hoe's always-true parenthesisation;
  - the `lavaStill, lavaStill` bucket pair;
  - `MapData`'s shifting removal;
  - a crafted map keeping damage 0;
  - the negated pitch settle;
  - the minecart collision term `var12 * var1.prevPosX`.

## 7. Reviewer findings (none changes a passing trace)

- **R1. `tick/testworld-6000` dumps an item, not the player** (§3.2). The
  scenario comment is wrong ("the player (id 0)"; the player is id 4), and no
  player state is compared in the long run. A variant that dumps the player
  matches the oracle (267 lines).
- **R2. Wiring gaps left behind by Phase 1 call sites.** Four Phase 1 call
  sites still record a gap although the Phase 2 function they need exists:
  - `BlockPortal.onEntityCollidedWithBlock` records `EntityPlayer.setInPortal`;
    `dispatch::set_in_portal` exists;
  - `BlockCake`'s private `heal` records `heartsLife`; `living::heal` exists;
  - `EntityLiving.onDeath` records `addToPlayerScore`/`onKillEntity`; both
    functions exist;
  - `EntityPig.onStruckByLightning` records the conversion; `EntityPigZombie`
    exists.

  Together with the three pickups (§3.1), these are a small, mechanical
  follow-up.
- **R3. Stale docs.**
  - spawner.rs module docs ("Recorded gaps", "Known divergence": no
    constructor is missing any more);
  - entity/item.rs module docs and `on_collide_with_player`'s doc ("needs the
    inventory (WP-2.5)");
  - arrow.rs:510-513 ("The inventory call is WP-2.5");
  - living.rs:768-770 ("EntityPlayer's is WP-2.5");
  - pig.rs:12-13 (the `flyPig` achievement is done; `EntityPigZombie` "is
    WP-2.3");
  - portal.rs:182 and cake.rs:127 ("Phase 2");
  - docs/mappings.md rows 235-236 (`add_stat` takes `&'static str`, "empty
    body"; flagged by the WP-2.7 merge report);
  - world/entities.rs:177-179 (N4).
- **R4. Dead code.** `FromNbt::Gap` / `construct_gap` / chunk_loader's preserved
  list (K6), `TileEntityKindData::Untranscribed` (N3), five unreachable
  `untranscribed` tails in block/dispatch.rs (N2), and probably the
  `readEntityFromNBT` fallthrough.
- **R5. The taken-out-entity guard cannot tell a harmless skip from a wrong
  one.**
  - Mechanism: when an AABB query meets an entity that is out of its slot and
    not the one in hand, `World::get_entities_within_aabb` leaves it out and
    records it (world/entities.rs:695-736). Java would test that entity's
    class and box like any other.
  - Where it hit: 11 passing fixtures reach this record, and none of them
    changed a traced value. For example, in `tick/bed-nether` the bed
    explosion's box contains the player, who is out of its slot while it
    activates the bed. The player is about 12 blocks away, beyond the
    explosion's `distance / (2 * size) <= 1` test (Explosion.java:77), so vanilla would not have hurt
    it either.
  - Risk: a case where the skipped entity does match would diverge, with only
    this counter as the sign.
  - Proposal: keep the class name and box of entities that are taken out, so
    the query can still test them, or pass the caller's held entities.
- **R6. Coverage checks.** The `recipe` and `stat` areas join `math`, `block`
  and `item` without an "every fixture has a test" check (N1).
- **R7. Per-fixture gap reporting (Gate 1 N7/Q8)** is still missing. This
  review added it temporarily (§3.1) and it found R2 and R5. Making it
  permanent, as a `gaps` file next to `actual.jsonl`, is cheap.

## 8. Open owner questions

1. **Gate 2 now or after a follow-up?** The reviewer recommends a short
   follow-up WP before Gate 2 is signed off:
   - transcribe `onCollideWithPlayer` for `EntityItem`, `EntityArrow` and
     `EntitySnowball`/`EntityEgg`, with the `mineWood`/`killCow`
     achievements;
   - wire the R2 sites (setInPortal, cake heal, `onDeath`'s score and kill
     hooks, pig → pigman);
   - add the `EntityPlayer.alertWolves` tamed-wolf branch;
   - un-ignore `tick/gate2-session`.

   The pickup alone makes the whole session match (§3.1). The alternative is
   to accept Gate 2 with `gate2-session` ignored on a gap that is known and
   one call deep.
2. **Fix `tick/testworld-6000`'s dumps** (R1): dump `which: [4]` plus
   `player_state` and correct the comment. That means a regen of its
   `expected.jsonl`, which changes a committed fixture, so it needs the owner's
   approval. The variant already matches.
3. **PLAN_v4 §8 item 3 (`Entity.rand` rewrite) is still undecided, and Phase 2
   finished without it.**
   - Not reproducible today: hostile natural spawning (K10) and every
     constructor draw in K11.
   - Approving `Entity.java:96` → `HarnessMath.entityRandom(this.entityId)`
     would make all of them comparable. The port already seeds `Entity.rand`
     with `mix(seed, id)` at construction, so no port change is expected.
   - Without the rewrite, these behaviours stay checked by unit tests only,
     until the Phase 4 side-by-side check.
4. Confirm the **reload protocol** (§5.3), including the kept provider (K14).
5. Confirm the **`HarnessPlayer` model** (§5.4, K12). Should a later WP add an
   oracle path for the `EntityPlayerSP` parts that matter to the simulation,
   such as `pushOutOfBlocks`?
6. Approve the **skeleton interface changes** of §5.8.
7. **Nether travel.** `Minecraft.usePortal` and `Teleporter.java` (about 250
   LOC) are not transcribed and no WP lists them. Should they go to WP-3.1
   (`Minecraft.java`) or to Phase 4? Gate 3's "play, save and reload" does not
   need them.
8. Gate 1's questions 2–11 and Gate 0's questions have no recorded answer in
   the tree. Question 5 (seeding the clock-seeded Randoms in the shipping game)
   and question 9 (the licence note on the committed `testWorld` save) are
   still the open ones that matter for Phase 3.
9. Make per-fixture gap reporting (R7) permanent?
10. Accept the WP-2.G addition to PLAN_v4 §5 and the remaining WP splits
    (§5.1)?

## Appendix: every fixture

Status as of `c415e67`, from `cargo test --workspace`. The ignored test was
also run with `--ignored` and fails as described in §3.1.

| Fixture | Tier | Status | Rust test |
|---|---|---|---|
| `block/block-table` | A | pass | `golden_block_table.rs::block_table_matches_oracle` |
| `entity/arrow-flight` | C | pass | `golden_tick.rs::entity_arrow_flight` |
| `entity/arrow-in-ground-pickup` | C | pass | `golden_tick.rs::entity_arrow_in_ground_pickup` |
| `entity/boat-on-water` | C | pass | `golden_tick.rs::entity_boat_on_water` |
| `entity/boat-player-ride` | C | pass | `golden_tick.rs::entity_boat_player_ride` |
| `entity/chicken-eggs` | C | pass | `golden_tick.rs::entity_chicken_eggs` |
| `entity/chicken-fall-hatch` | C | pass | `golden_tick.rs::entity_chicken_fall_hatch` |
| `entity/cow-milk` | C | pass | `golden_tick.rs::entity_cow_milk` |
| `entity/creeper-fuse-explode` | C | pass | `golden_tick.rs::entity_creeper_fuse_explode` |
| `entity/falling-sand-on-cactus` | C | pass | `golden_tick.rs::entity_falling_sand_on_cactus` |
| `entity/fireball-explode` | C | pass | `golden_tick.rs::entity_fireball_explode` |
| `entity/fishing-hook` | C | pass | `golden_tick.rs::entity_fishing_hook` |
| `entity/flat-item-drop` | C | pass | `golden_tick.rs::entity_flat_item_drop` |
| `entity/flat-pig-falls` | C | pass | `golden_tick.rs::entity_flat_pig_falls` |
| `entity/ghast-waypoints` | C | pass | `golden_tick.rs::entity_ghast_waypoints` |
| `entity/item-collision-pair` | C | pass | `golden_tick.rs::entity_item_collision_pair` |
| `entity/item-drop` | C | pass | `golden_tick.rs::entity_item_drop` |
| `entity/item-in-fire` | C | pass | `golden_tick.rs::entity_item_in_fire` |
| `entity/item-in-lava` | C | pass | `golden_tick.rs::entity_item_in_lava` |
| `entity/item-in-water` | C | pass | `golden_tick.rs::entity_item_in_water` |
| `entity/item-in-web` | C | pass | `golden_tick.rs::entity_item_in_web` |
| `entity/item-out-of-world` | C | pass | `golden_tick.rs::entity_item_out_of_world` |
| `entity/item-steps-up` | C | pass | `golden_tick.rs::entity_item_steps_up` |
| `entity/lightning-strike` | C | pass | `golden_tick.rs::entity_lightning_strike` |
| `entity/minecart-break-drops` | C | pass | `golden_tick.rs::entity_minecart_break_drops` |
| `entity/minecart-derail-collide` | C | pass | `golden_tick.rs::entity_minecart_derail_collide` |
| `entity/minecart-detector-rail` | C | pass | `golden_tick.rs::entity_minecart_detector_rail` |
| `entity/minecart-furnace-push` | C | pass | `golden_tick.rs::entity_minecart_furnace_push` |
| `entity/minecart-player-ride` | C | pass | `golden_tick.rs::entity_minecart_player_ride` |
| `entity/minecart-powered-brake` | C | pass | `golden_tick.rs::entity_minecart_powered_brake` |
| `entity/minecart-rail-loop` | C | pass | `golden_tick.rs::entity_minecart_rail_loop` |
| `entity/painting-art` | C | pass | `golden_tick.rs::entity_painting_art` |
| `entity/pig-attacked` | C | pass | `golden_tick.rs::entity_pig_attacked` |
| `entity/pig-burning` | C | pass | `golden_tick.rs::entity_pig_burning` |
| `entity/pig-drowning` | C | pass | `golden_tick.rs::entity_pig_drowning` |
| `entity/pig-fall-damage` | C | pass | `golden_tick.rs::entity_pig_fall_damage` |
| `entity/pig-falls` | C | pass | `golden_tick.rs::entity_pig_falls` |
| `entity/pig-knockback` | C | pass | `golden_tick.rs::entity_pig_knockback` |
| `entity/pig-ladder` | C | pass | `golden_tick.rs::entity_pig_ladder` |
| `entity/pig-wander` | C | pass | `golden_tick.rs::entity_pig_wander` |
| `entity/pigzombie-anger` | C | pass | `golden_tick.rs::entity_pigzombie_anger` |
| `entity/player-attack-pig` | C | pass | `golden_tick.rs::entity_player_attack_pig` |
| `entity/player-death-drops` | C | pass | `golden_tick.rs::entity_player_death_drops` |
| `entity/player-dig-stone` | C | pass | `golden_tick.rs::entity_player_dig_stone` |
| `entity/player-fall-damage` | C | pass | `golden_tick.rs::entity_player_fall_damage` |
| `entity/player-fall-damage-armour` | C | pass | `golden_tick.rs::entity_player_fall_damage_armour` |
| `entity/player-sleep` | C | pass | `golden_tick.rs::entity_player_sleep` |
| `entity/player-walk-jump` | C | pass | `golden_tick.rs::entity_player_walk_jump` |
| `entity/sheep-shear` | C | pass | `golden_tick.rs::entity_sheep_shear` |
| `entity/skeleton-shoot` | C | pass | `golden_tick.rs::entity_skeleton_shoot` |
| `entity/skeleton-sunburn` | C | pass | `golden_tick.rs::entity_skeleton_sunburn` |
| `entity/slime-split` | C | pass | `golden_tick.rs::entity_slime_split` |
| `entity/snowball-egg-flight` | C | pass | `golden_tick.rs::entity_snowball_egg_flight` |
| `entity/spider-leap` | C | pass | `golden_tick.rs::entity_spider_leap` |
| `entity/squid-flowing-water` | C | pass | `golden_tick.rs::entity_squid_flowing_water` |
| `entity/squid-swim` | C | pass | `golden_tick.rs::entity_squid_swim` |
| `entity/tnt-explode` | C | pass | `golden_tick.rs::entity_tnt_explode` |
| `entity/wolf-begs-bone` | C | pass | `golden_tick.rs::entity_wolf_begs_bone` |
| `entity/wolf-bone-refused` | C | pass | `golden_tick.rs::entity_wolf_bone_refused` |
| `entity/wolf-hunts-sheep` | C | pass | `golden_tick.rs::entity_wolf_hunts_sheep` |
| `entity/wolf-pack-retaliates` | C | pass | `golden_tick.rs::entity_wolf_pack_retaliates` |
| `entity/wolf-rain-shake` | C | pass | `golden_tick.rs::entity_wolf_rain_shake` |
| `entity/zombie-chase-player` | C | pass | `golden_tick.rs::entity_zombie_chase_player` |
| `hash/chunkcoordinates` | A | pass | `mc-core golden_hash.rs::chunkcoordinates` |
| `hash/chunkcoordintpair-active-chunks` | A | pass | `mc-core golden_hash.rs::chunkcoordintpair_active_chunks` |
| `hash/chunkcoordintpair-spiral` | A | pass | `mc-core golden_hash.rs::chunkcoordintpair_spiral` |
| `hash/chunkposition-explosion` | A | pass | `mc-core golden_hash.rs::chunkposition_explosion` |
| `hash/integer-chunk-keys` | A | pass | `mc-core golden_hash.rs::integer_chunk_keys` |
| `hash/string-names` | A | pass | `mc-core golden_hash.rs::string_names` |
| `item/item-table` | A | pass | `golden_item_table.rs::item_item_table` |
| `light/blocklight-glowstone` | B | pass | `golden_tick.rs::light_blocklight_glowstone` |
| `light/chunk-border` | B | pass | `golden_tick.rs::light_chunk_border` |
| `light/occlusion` | B | pass | `golden_tick.rs::light_occlusion` |
| `light/skylight-column` | B | pass | `golden_tick.rs::light_skylight_column` |
| `math/math-sweep` | A | pass | `mc-core math_sweep_report.rs::golden_math_sweep_strict_eq` |
| `math/math-sweep-strict` | A | pass | `mc-core math_sweep_report.rs::golden_math_sweep_strict_fixture_eq` |
| `math/sine-table` | A | pass | `mc-core golden_sine_table.rs::golden_sine_table` |
| `nbt/empty-root` | A | pass | `mc-nbt golden_nbt.rs::nbt_empty_root` |
| `nbt/level-dat-shape` | A | pass | `mc-nbt golden_nbt.rs::nbt_level_dat_shape` |
| `nbt/nan-payloads` | A | pass | `mc-nbt golden_nbt.rs::nbt_nan_payloads` |
| `nbt/primitives` | A | pass | `mc-nbt golden_nbt.rs::nbt_primitives` |
| `nbt/testworld-level-dat` | A | pass | `mc-nbt golden_nbt.rs::nbt_testworld_level_dat` |
| `recipe/recipe-table` | A | pass | `golden_recipe_table.rs::recipe_recipe_table` |
| `rng/java-random--4172144991579068934` | A | pass | `mc-core golden_rng.rs::java_random_neg_4172144991579068934` |
| `rng/java-random-0` | A | pass | `mc-core golden_rng.rs::java_random_0` |
| `rng/java-random-123` | A | pass | `mc-core golden_rng.rs::java_random_123` |
| `save/roundtrip-seed-123` | S | pass | `golden_save.rs::save_roundtrip_seed_123` |
| `save/testworld` | S | pass | `golden_save.rs::save_testworld (+ testworld_chunk_digest_and_chest)` |
| `save/tile-entities` | S | pass | `golden_save.rs::save_tile_entities` |
| `stat/stat-table` | A | pass | `golden_stat_table.rs::stat_stat_table` |
| `tick/activate-workbench-stairs` | B | pass | `golden_tick.rs::tick_activate_workbench_stairs` |
| `tick/bed-nether` | B | pass | `golden_tick.rs::tick_bed_nether` |
| `tick/bed-place-and-break` | B | pass | `golden_tick.rs::tick_bed_place_and_break` |
| `tick/block-item-placement` | B | pass | `golden_tick.rs::tick_block_item_placement` |
| `tick/bonemeal-sapling` | B | pass | `golden_tick.rs::tick_bonemeal_sapling` |
| `tick/bow-shoot` | C | pass | `golden_tick.rs::tick_bow_shoot` |
| `tick/bucket-fill-empty` | B | pass | `golden_tick.rs::tick_bucket_fill_empty` |
| `tick/bucket-nether-fizz` | B | pass | `golden_tick.rs::tick_bucket_nether_fizz` |
| `tick/button-lever` | B | pass | `golden_tick.rs::tick_button_lever` |
| `tick/cactus-reed-growth` | B | pass | `golden_tick.rs::tick_cactus_reed_growth` |
| `tick/cake-eat` | B | pass | `golden_tick.rs::tick_cake_eat` |
| `tick/chest-container-clicks` | S | pass | `golden_tick.rs::tick_chest_container_clicks` |
| `tick/chest-double-and-break` | B | pass | `golden_tick.rs::tick_chest_double_and_break` |
| `tick/craft-ops` | C | pass | `golden_tick.rs::tick_craft_ops` |
| `tick/crops-growth` | B | pass | `golden_tick.rs::tick_crops_growth` |
| `tick/detector-rail-idle` | C | pass | `golden_tick.rs::tick_detector_rail_idle` |
| `tick/detector-rail-random-tick` | B | pass | `golden_tick.rs::tick_detector_rail_random_tick` |
| `tick/dispenser-dispense-item` | B | pass | `golden_tick.rs::tick_dispenser_dispense_item` |
| `tick/dispenser-idle` | B | pass | `golden_tick.rs::tick_dispenser_idle` |
| `tick/door-iron-powered` | B | pass | `golden_tick.rs::tick_door_iron_powered` |
| `tick/door-support-removed` | B | pass | `golden_tick.rs::tick_door_support_removed` |
| `tick/door-toggle` | B | pass | `golden_tick.rs::tick_door_toggle` |
| `tick/door-upper-powered` | B | pass | `golden_tick.rs::tick_door_upper_powered` |
| `tick/dungeon-chest-save` | S | pass | `golden_tick.rs::tick_dungeon_chest_save` |
| `tick/dye-bonemeal-crops` | C | pass | `golden_tick.rs::tick_dye_bonemeal_crops` |
| `tick/empty-world-20` | B | pass | `golden_tick.rs::tick_empty_world_20` |
| `tick/explosion-crater` | B | pass | `golden_tick.rs::tick_explosion_crater` |
| `tick/explosion-fire` | B | pass | `golden_tick.rs::tick_explosion_fire` |
| `tick/fence-stairs-step-bounds` | C | pass | `golden_tick.rs::tick_fence_stairs_step_bounds` |
| `tick/fire-burnout-stone` | B | pass | `golden_tick.rs::tick_fire_burnout_stone` |
| `tick/fire-farmland-rain` | B | pass | `golden_tick.rs::tick_fire_farmland_rain` |
| `tick/fire-rain-spread` | B | pass | `golden_tick.rs::tick_fire_rain_spread` |
| `tick/fire-spread-planks` | B | pass | `golden_tick.rs::tick_fire_spread_planks` |
| `tick/fishing-rod-reel` | C | pass | `golden_tick.rs::tick_fishing_rod_reel` |
| `tick/flat-flowing-water` | B | pass | `golden_tick.rs::tick_flat_flowing_water` |
| `tick/flat-lava-fire` | B | pass | `golden_tick.rs::tick_flat_lava_fire` |
| `tick/flat-lava-flow` | B | pass | `golden_tick.rs::tick_flat_lava_flow` |
| `tick/flat-lava-flow-nether` | B | pass | `golden_tick.rs::tick_flat_lava_flow_nether` |
| `tick/flat-lava-water-fizz` | B | pass | `golden_tick.rs::tick_flat_lava_water_fizz` |
| `tick/flat-sand-falls` | B | pass | `golden_tick.rs::tick_flat_sand_falls` |
| `tick/flat-torch-light` | B | pass | `golden_tick.rs::tick_flat_torch_light` |
| `tick/flat-water-blockers` | B | pass | `golden_tick.rs::tick_flat_water_blockers` |
| `tick/flat-water-drain` | B | pass | `golden_tick.rs::tick_flat_water_drain` |
| `tick/flat-water-flow` | B | pass | `golden_tick.rs::tick_flat_water_flow` |
| `tick/flat-water-item-push` | C | pass | `golden_tick.rs::tick_flat_water_item_push` |
| `tick/flat-water-lava-mix` | B | pass | `golden_tick.rs::tick_flat_water_lava_mix` |
| `tick/flat-water-spread-holes` | B | pass | `golden_tick.rs::tick_flat_water_spread_holes` |
| `tick/flint-steel-tnt-portal` | B | pass | `golden_tick.rs::tick_flint_steel_tnt_portal` |
| `tick/food-eat-heal` | C | pass | `golden_tick.rs::tick_food_eat_heal` |
| `tick/furnace-dispenser-clicks` | S | pass | `golden_tick.rs::tick_furnace_dispenser_clicks` |
| `tick/furnace-idle` | B | pass | `golden_tick.rs::tick_furnace_idle` |
| `tick/gate2-session` | S | ignored | `golden_tick.rs::tick_gate2_session` |
| `tick/grass-spread` | B | pass | `golden_tick.rs::tick_grass_spread` |
| `tick/hoe-seeds-crops` | B | pass | `golden_tick.rs::tick_hoe_seeds_crops` |
| `tick/inventory-slot-clicks` | C | pass | `golden_tick.rs::tick_inventory_slot_clicks` |
| `tick/ladder-placement` | B | pass | `golden_tick.rs::tick_ladder_placement` |
| `tick/leaves-decay` | B | pass | `golden_tick.rs::tick_leaves_decay` |
| `tick/locked-chest-vanishes` | B | pass | `golden_tick.rs::tick_locked_chest_vanishes` |
| `tick/map-craft` | C | pass | `golden_tick.rs::tick_map_craft` |
| `tick/map-create-update` | C | pass | `golden_tick.rs::tick_map_create_update` |
| `tick/minecart-boat-painting-place` | C | pass | `golden_tick.rs::tick_minecart_boat_painting_place` |
| `tick/mob-spawner-cap` | C | pass | `golden_tick.rs::tick_mob_spawner_cap` |
| `tick/mob-spawner-delay` | B | pass | `golden_tick.rs::tick_mob_spawner_delay` |
| `tick/mob-spawner-spawn` | B | pass | `golden_tick.rs::tick_mob_spawner_spawn` |
| `tick/mob-spawner-spawn-grass` | C | pass | `golden_tick.rs::tick_mob_spawner_spawn_grass` |
| `tick/natural-spawn-pigs` | C | pass | `golden_tick.rs::tick_natural_spawn_pigs` |
| `tick/natural-spawn-squid` | C | pass | `golden_tick.rs::tick_natural_spawn_squid` |
| `tick/note-jukebox-activate` | B | pass | `golden_tick.rs::tick_note_jukebox_activate` |
| `tick/piston-immovable` | B | pass | `golden_tick.rs::tick_piston_immovable` |
| `tick/piston-moving-block-tick` | B | pass | `golden_tick.rs::tick_piston_moving_block_tick` |
| `tick/piston-orphan-blocks` | B | pass | `golden_tick.rs::tick_piston_orphan_blocks` |
| `tick/piston-push-limit` | B | pass | `golden_tick.rs::tick_piston_push_limit` |
| `tick/piston-push-stone` | B | pass | `golden_tick.rs::tick_piston_push_stone` |
| `tick/piston-retract-entity` | B | pass | `golden_tick.rs::tick_piston_retract_entity` |
| `tick/piston-retract-unpowered` | B | pass | `golden_tick.rs::tick_piston_retract_unpowered` |
| `tick/place-stairs-slabs-doors-beds-signs` | B | pass | `golden_tick.rs::tick_place_stairs_slabs_doors_beds_signs` |
| `tick/portal-display` | B | pass | `golden_tick.rs::tick_portal_display` |
| `tick/portal-frame` | C | pass | `golden_tick.rs::tick_portal_frame` |
| `tick/portal-ignite` | B | pass | `golden_tick.rs::tick_portal_ignite` |
| `tick/powered-rail-chain` | B | pass | `golden_tick.rs::tick_powered_rail_chain` |
| `tick/powered-rail-detector` | B | pass | `golden_tick.rs::tick_powered_rail_detector` |
| `tick/powered-rail-lever` | B | pass | `golden_tick.rs::tick_powered_rail_lever` |
| `tick/pressure-plate` | C | pass | `golden_tick.rs::tick_pressure_plate` |
| `tick/rail-remove-support` | C | pass | `golden_tick.rs::tick_rail_remove_support` |
| `tick/rail-shapes` | B | pass | `golden_tick.rs::tick_rail_shapes` |
| `tick/rail-slope-bounds` | B | pass | `golden_tick.rs::tick_rail_slope_bounds` |
| `tick/random-ticks-stone-platform` | B | pass | `golden_tick.rs::tick_random_ticks_stone_platform` |
| `tick/ray-trace-platform` | B | pass | `golden_tick.rs::tick_ray_trace_platform` |
| `tick/record-jukebox` | C | pass | `golden_tick.rs::tick_record_jukebox` |
| `tick/redstone-item-placement` | B | pass | `golden_tick.rs::tick_redstone_item_placement` |
| `tick/redstone-ore-touch` | B | pass | `golden_tick.rs::tick_redstone_ore_touch` |
| `tick/redstone-removal` | C | pass | `golden_tick.rs::tick_redstone_removal` |
| `tick/redstone-torch-inverter` | B | pass | `golden_tick.rs::tick_redstone_torch_inverter` |
| `tick/redstone-wire-line` | B | pass | `golden_tick.rs::tick_redstone_wire_line` |
| `tick/reload-generated` | B | pass | `golden_tick.rs::tick_reload_generated` |
| `tick/reload-nether` | B | pass | `golden_tick.rs::tick_reload_nether` |
| `tick/reload-roundtrip` | S | pass | `golden_tick.rs::tick_reload_roundtrip` |
| `tick/repeater-delays` | B | pass | `golden_tick.rs::tick_repeater_delays` |
| `tick/saddle-pig-ride` | C | pass | `golden_tick.rs::tick_saddle_pig_ride` |
| `tick/sapling-grow` | B | pass | `golden_tick.rs::tick_sapling_grow` |
| `tick/sapling-with-player` | B | pass | `golden_tick.rs::tick_sapling_with_player` |
| `tick/sign-pops-off` | B | pass | `golden_tick.rs::tick_sign_pops_off` |
| `tick/sleep-spawning` | C | pass | `golden_tick.rs::tick_sleep_spawning` |
| `tick/snow-ice` | B | pass | `golden_tick.rs::tick_snow_ice` |
| `tick/soul-sand-web-item` | C | pass | `golden_tick.rs::tick_soul_sand_web_item` |
| `tick/spawning-draws` | B | pass | `golden_tick.rs::tick_spawning_draws` |
| `tick/sponge-removal-notifies` | B | pass | `golden_tick.rs::tick_sponge_removal_notifies` |
| `tick/sword-pickaxe-durability` | C | pass | `golden_tick.rs::tick_sword_pickaxe_durability` |
| `tick/testworld-6000` | B | pass | `golden_tick.rs::tick_testworld_6000` |
| `tick/tnt-by-fire` | B | pass | `golden_tick.rs::tick_tnt_by_fire` |
| `tick/torch-placement` | C | pass | `golden_tick.rs::tick_torch_placement` |
| `tick/trapdoor-toggle` | B | pass | `golden_tick.rs::tick_trapdoor_toggle` |
| `tick/use-item-noop` | B | pass | `golden_tick.rs::tick_use_item_noop` |
| `tick/water-flow` | B | pass | `golden_tick.rs::tick_water_flow` |
| `tick/weather-ice` | B | pass | `golden_tick.rs::tick_weather_ice` |
| `tick/weather-snow` | B | pass | `golden_tick.rs::tick_weather_snow` |
| `tick/weather-time` | B | pass | `golden_tick.rs::tick_weather_time` |
| `tick/workbench-crafting` | C | pass | `golden_tick.rs::tick_workbench_crafting` |
| `worldgen/nether-seed--4172144991579068934` | A | pass | `golden_worldgen.rs::nether_seed_neg_4172144991579068934` |
| `worldgen/nether-seed-123` | A | pass | `golden_worldgen.rs::nether_seed_123` |
| `worldgen/nether-seed-123-order-ab` | A | pass | `golden_worldgen.rs::nether_seed_123_order_ab` |
| `worldgen/nether-seed-123-order-ba` | A | pass | `golden_worldgen.rs::nether_seed_123_order_ba` |
| `worldgen/nether-seed-42-spread` | A | pass | `golden_worldgen.rs::nether_seed_42_spread` |
| `worldgen/nether-seed-8675309` | A | pass | `golden_worldgen.rs::nether_seed_8675309` |
| `worldgen/overworld-seed--4172144991579068934` | A | pass | `golden_worldgen.rs::overworld_seed_neg_4172144991579068934` |
| `worldgen/overworld-seed-123` | A | pass | `golden_worldgen.rs::overworld_seed_123` |
| `worldgen/overworld-seed-123-dungeon` | A | pass | `golden_worldgen.rs::overworld_seed_123_dungeon` |
| `worldgen/overworld-seed-42-spread` | A | pass | `golden_worldgen.rs::overworld_seed_42_spread` |
| `worldgen/overworld-seed-8675309` | A | pass | `golden_worldgen.rs::overworld_seed_8675309` |
