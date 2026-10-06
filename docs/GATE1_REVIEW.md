# Gate 1 review notes (Phase 1, WP-1.1 … WP-1.9)

Written 2026-10-06 by the Gate 1 reviewer for the owner's review (PLAN_v4 §5:
"Gate 1: all Tier A, B and S fixtures green; §4.4 item 4 passes"). Phase 1 was
integrated on one branch, `phase-1` (forked from `wp-0.5` at `a094a54`, tip
`6ebd8c9` before this document): 77 linear "WP-x.y step N" / "WP-x.y merge"
commits rebased and fast-forwarded from the per-WP branches `wp-1.1` …
`wp-1.9b` and `wp-int-a`, all still present. Nothing is pushed.
Size of Phase 1: 335 files, +71,956 / −669 lines; `mc-sim` is now 40.1k lines of
source plus 7.1k lines of tests.

**Verdict.** Not green in the literal sense: 6 Tier B fixtures and 1 Tier S
fixture are `#[ignore]`d. Every one of them stops at Java code that PLAN_v4 puts
in Phase 2 (entity constructors, `Item.onItemUse`, the player's update). No
Phase 1 code is known to diverge. All Tier A fixtures and the other Tier B and
S fixtures pass, and §4.4 item 4 passes; it was also checked beyond the
fixture, on all 784 chunks of `testWorld` (§3). The owner should decide
whether Gate 1 is accepted with these seven fixtures moved to the Phase 2
gate (question 1 in §8).

## 1. Harness acceptance and gate commands

Run in the main checkout at `6ebd8c9`. Nothing needed fixing, so this gate made
no fix commits.

| Check | Result | Evidence |
|---|---|---|
| `cargo test --workspace` | pass | 402 passed, 0 failed, 11 ignored (33 test binaries, 5 doc-test runs) |
| `cargo clippy --workspace --all-targets --all-features -- -Dwarnings` | clean | exit 0 |
| `cargo fmt --all --check` | clean | exit 0 |
| `cargo check -p mc-core -p mc-nbt -p mc-sim --target wasm32-unknown-unknown` | pass | exit 0 (D9) |
| `sh tools/oracle/regen.sh` | no drift | exit 0; `git status --short tests/golden` empty afterwards |
| §4.4 item 1: `tools/oracle/check-reproducible.sh` | pass | "3 runs byte-identical (133 files)" |
| §4.4 item 2: `tools/oracle/check-sensitivity.sh` | pass | 131/131 fixtures `ok`: every seeded scenario changes, every unseeded one stays identical; Rust side: `golden_tick::flat_item_drop_with_perturbed_seed_fails`, `golden_worldgen::overworld_with_perturbed_seed_fails` |
| §4.4 item 3: decoded first difference | pass | `crates/mc-golden/tests/runner.rs`; e.g. `save/tile-entities: golden mismatch at line 18 (t=5, e="level_dat"), field root.value.Data.value.Player.value.Fire.value` |
| §4.4 item 4: `testWorld` | **pass** | §3 below |
| Ignored fixtures still fail as their reasons say | confirmed | `cargo test --no-fail-fast -- --ignored`: all 11 fail at the line their reason names (§2.2) |

The oracle prints `harness: warning: chunk (1,0) already loaded before 0,0; the
"generated" stage is not pure generator output` for the `*-42-spread`
worldgen fixtures. This comes from TierA.java:199 and is expected: those
fixtures generate neighbouring chunks, so population has already loaded some
of them. The warning is not a failure.

## 2. Fixtures

131 fixture directories under `tests/golden`. Every one is run by a Rust test,
so no fixture lacks a test. The appendix lists each fixture with its test.

| Tier | Fixtures | Pass | Ignored | Rust tests |
|---|---|---|---|---|
| A | 30 (rng 3, math 3, hash 6, nbt 5, block 1, item 1, worldgen 11) | 30 | 0 | mc-core `golden_rng`, `golden_hash`, `golden_sine_table`, `math_sweep_report`; mc-nbt `golden_nbt`; mc-sim `golden_block_table`, `golden_item_table`, `golden_worldgen` (every stage compared; `SKIPPED_STAGES` is empty since WP-1.7) |
| B | 82 (light 4, tick 78) | 76 | 6 | mc-sim `golden_tick` |
| S | 4 (save 3, `tick/dungeon-chest-save`) | 3 | 1 | mc-sim `golden_save`, `golden_tick::tick_dungeon_chest_save` |
| C (Phase 2 gate) | 15 (entity 6, tick 9) | 11 | 4 | mc-sim `golden_tick` |
| **Total** | **131** | **120** | **11** | |

Coverage checks that fail when a fixture directory has no test:
`golden_tick::every_tick_and_entity_fixture_has_a_test` (tick, entity, light),
`golden_worldgen::every_worldgen_fixture_has_a_test`,
`golden_save::every_tier_s_fixture_has_a_test` (it accepts
`tick/dungeon-chest-save` through `COVERED_IN_GOLDEN_TICK`, so that fixture is
run only by golden_tick), `golden_nbt::every_nbt_fixture_is_covered`,
`golden_hash::all_hash_fixtures_listed`, `golden_rng::all_rng_fixtures_listed`.
The `math`, `block` and `item` areas have no such check, because each holds
fixed, named fixtures. A new fixture added there would go unnoticed until
someone wrote its test (nit N1).

### 2.1 Tier B and S fixtures not green (all blocked on Phase 2)

| Fixture | Tier | First difference (re-run at this gate) | Missing Java code (owner WP) |
|---|---|---|---|
| `tick/block-item-placement` | B | line 7, t=1 `op use_item` field `placed` | `ItemBlock` / `ItemReed` / `ItemDoor` / `ItemBed` / `ItemSlab.onItemUse` (WP-2.6) |
| `tick/redstone-item-placement` | B | line 8, t=1 `op use_item` field `placed` | `ItemBlock` / `ItemReed` / `ItemRedstone.onItemUse` (WP-2.6) |
| `tick/bonemeal-sapling` | B | line 7, t=1 `op use_item` field `placed` | `ItemDye.onItemUse` (WP-2.6) |
| `tick/mob-spawner-spawn` | B | line 25, t=20 `rng_state` field `math_random` | `EntityPig(World)` constructor via `EntityList.createEntityInWorld` (WP-2.3) |
| `tick/tnt-by-fire` | B | line 170, t=79 `rng_state` field `math_random` | `EntityTNTPrimed` (WP-2.4) |
| `tick/sapling-with-player` | B | line 4, t=99 `chunk_digest` (-2,-4) field `blocks` | `SpawnerAnimals` mob constructors (WP-2.3), then the player's update (WP-2.5) |
| `save/tile-entities` | S | line 18, t=5 `level_dat` `Data.Player.Fire` (and `Motion`, `OnGround`) | the player's update: `Entity.onEntityUpdate` / `EntityLiving` / `EntityPlayer.onUpdate` (WP-2.2, WP-2.5). Every other line matches (`save_tile_entities_matches_except_the_player`) |

### 2.2 Tier C fixtures not green (Phase 2 gate, listed for completeness)

| Fixture | Where it stops | Missing |
|---|---|---|
| `entity/arrow-flight` | t=0 `op spawn kind Arrow`: "the EntityArrow(World) constructor is not transcribed" | WP-2.4 |
| `entity/flat-pig-falls` | t=0 `op spawn kind Pig` | `EntityPig` (WP-2.3), wander AI (WP-2.2) |
| `entity/pig-attacked` | t=0 `op spawn kind Pig` | WP-2.3, then attack code (WP-2.2, WP-2.5) |
| `entity/pig-falls` | t=0 `op spawn kind Pig` | WP-2.3, water physics (WP-2.1) |

The other 11 Tier C fixtures pass. They cover item entities, falling sand,
pressure plates, torches, frames, fences and similar, and run on the WP-0.5
`EntityItem` / `EntityFallingSand`.

## 3. §4.4 item 4: `testWorld`

**Test.** `crates/mc-sim/tests/golden_save.rs::testworld_chunk_digest_and_chest`
(WP-1.9b). It opens `tests/golden/save/testworld/world` through `SaveOldDir` on
a `MemoryStorage` and asserts the following:

- it loads chunk (0, −4) through the world's own `ChunkProvider` and
  `McRegionChunkLoader`;
- the chunk's `blocks`, `data`, `skylight`, `blocklight` and `heightmap`
  digests equal the oracle's `"stage":"loaded"` line;
- `chest_slots(9, 64, −61) == [(0, ItemStack(5, 2, 0)), (1, ItemStack(280, 4, 0))]`,
  that is, 2 planks in slot 0 and 4 sticks in slot 1;
- the chest is in `loadedTileEntityList`, and `chunk_load_errors() == 0`.

`golden_save::save_testworld` replays the whole `save/testworld` fixture
(`level_dat`, `world_info`, `region_header`, `chunk_nbt`, `chunk_digest`,
`tile_entity`).

```
$ cargo test -p mc-sim --test golden_save -- --nocapture testworld cross_read save_testworld
test testworld_chunk_digest_and_chest ... ok
test save_testworld ... ok
test cross_read_rust_written_world_by_vanilla ... ok
test cross_read_vanilla_written_world_by_rust ... ok
...
oracle  {"t":0,"e":"chunk_digest","x":0,"z":-4,"stage":"loaded","blocks":"2f78deee3a94…a055","data":"e653fc481…298c","skylight":"d02cbfbe…655e","blocklight":"d0a5f66b…8c54","heightmap":"5b85e0f0…de5b"}
Rust    target/golden/save/testworld/actual.jsonl: the same five digests; the sorted-key JSON of every line is identical to expected.jsonl
oracle  {"t":0,"e":"tile_entity","id":"Chest","x":9,"y":64,"z":-61,"items":[{"slot":0,"id":5,"count":2,"damage":0},{"slot":1,"id":280,"count":4,"damage":0}],…}
```

The cross-read tests ran: `build/oracle/harness/WorldReadDir.class` exists,
and no "skipped" line was printed. Without a built oracle they return early
and still report `ok` (question 7).

**Stronger check made at this gate (not committed).** The committed fixture
holds only `region/r.0.-1.mcr` and compares one chunk, while §4.4 says "chunk
digests" (plural). To cover that wording, I copied the owner's full
`.reference/mc_b1.7.3-world/testWorld` (4 regions) into the scratchpad. I gave
it a `world_read` scenario that lists all 784 chunks present in the four
region headers, ran the oracle on it, and replayed it through
`world_read_driver::drive_in` with a temporary test, which I deleted
afterwards. **All 1,627 lines match**: 784 `chunk_digest`, 784 `chunk_nbt`,
4 `region_header`, 53 `tile_entity`, `level_dat` and `world_info`. Question 6
asks whether this check should become the committed fixture.

## 4. Recorded gaps left in the code

### 4.1 Block dispatch (`crates/mc-sim/src/block/dispatch.rs`)

**None.** `cargo test -p mc-sim --test block_dispatch_coverage -- --nocapture`
prints `0 overrides recorded as not transcribed, 306 transcribed in dispatch, 13
data methods`. The vanilla override set has 514 `(class, method)` rows
(`tests/data/block_overrides.txt`). Two of them are dead in vanilla:
`BlockFluid.updateTick` and `BlockLeavesBase.isOpaqueCube`. Every `if let
Some(what) = what { untranscribed(..) }` tail in dispatch.rs is now
unreachable: each match ends in `_ => None`. The merge commits of WP-1.6d,
1.6e and 1.8 already dropped this tail for 18 methods. The rest can go the
same way (nit N2).

### 4.2 `World::skipped_systems` / `World::unimplemented_hits` strings, by Phase 2 owner

These strings come from a grep of `skip(` / `note_unimplemented(` in
`crates/mc-sim/src` and `crates/mc-golden/src`. "From tick" means the string
is reachable from `World.tick` or from the harness `runTick`
(`updateEntities`, `tick`, `randomDisplayUpdates`) without a player action.

| String(s) | Site | From tick | Owner |
|---|---|---|---|
| `SpawnerAnimals.performSleepSpawning` (skipped_systems) | tick.rs:47 | yes (all players asleep, hostile spawning on) | WP-2.3 (with Pathfinder 2.2, beds 2.5) |
| `EntityPlayer.wakeUpPlayer (World.wakeUpAllPlayers)` | tick.rs:128 | yes | WP-2.5 |
| `Entity{Spider,Zombie,Skeleton,Creeper,Slime,Ghast,PigZombie,Sheep,Pig,Chicken,Cow,Wolf,Squid}.<init> (SpawnerAnimals.performSpawning)` | spawner.rs:225 | yes | WP-2.3 |
| `EntityLightningBolt.<init> (World.updateBlocksAndPlayCaveSounds)` | tick.rs:349 | yes (thunder) | WP-2.4 |
| `EntityTNTPrimed.<init>` | block/tnt.rs:137 | yes (fire, redstone, explosions reach TNT) | WP-2.4 |
| `EntityArrow.<init>`, `EntityEgg.<init>`, `EntitySnowball.<init>` | block/dispenser.rs:127-135 | yes (dispenser scheduled tick) | WP-2.4 |
| `EntityList.createEntityInWorld` | tile_entity/mob_spawner.rs:200 | yes (`updateEntities` → tile entities) | WP-2.3 |
| `EntityLiving.onUpdate`, `EntityLiving.onEntityUpdate` | entity/living.rs:216, :227 | yes (`updateEntities`) | WP-2.2 |
| `EntityMob/Creeper/PigZombie/Slime/Ghast/Wolf.onUpdate` | entity/dispatch.rs:27-47 | yes | WP-2.3 |
| `EntityPlayer.onUpdate`, `EntityPlayer.updateRidden`, `EntityPlayer.getEyeHeight`, `EntityPlayer.getYOffset`, `EntityPlayerSP.moveEntity`, `EntityPlayerSP.isSneaking` | entity/dispatch.rs | yes (any scenario with a player) | WP-2.5 |
| `Entity{Arrow,Snowball,Egg,Fireball,Fish,Painting,TNTPrimed,LightningBolt}.onUpdate` | entity/dispatch.rs:54-63 | yes, once constructed | WP-2.4 (task 1) |
| `Entity{Minecart,Boat}.onUpdate`, `.getBoundingBox`, `.getCollisionBox`, `.getMountedYOffset`, `.canBePushed`, `EntityBoat.updateRiderPosition` | entity/dispatch.rs, entity/base.rs:1063 | yes, once constructed | WP-2.4 (task 2) |
| `*.attackEntityFrom` (Fireball, Painting, Minecart, Boat, Mob, PigZombie, Sheep, Wolf, Player), `*.setEntityDead` (Minecart, Slime, Player), `*.fall` (Flying, Pig, Chicken, Player), `*.canBeCollidedWith`, `EntityWolf.getEyeHeight`, `EntitySpider.getMountedYOffset`, `EntityPainting.moveEntity` | entity/dispatch.rs | yes, once those kinds exist | WP-2.2 / 2.3 / 2.4 / 2.5 by class |
| `EntityList.createEntityFromNBT: the entity class's (World) constructor …` | save/entity_list.rs:161 | chunk loads | WP-2.3 / WP-2.4 |
| `EntityPlayer.readEntityFromNBT: bedChunkCoordinates and wakeUpPlayer…` | entity/nbt.rs:332 | world load | WP-2.5 |
| `Entity*.writeEntityToNBT`, `Entity*.readEntityFromNBT` | entity/nbt.rs:127, :291 | saves | Phase 2 per class |
| `EntityPlayer.addStat` | block/leaves.rs, snow.rs, dispatch.rs:2262 (`harvestBlock`) | no (player digging) | WP-2.7 |
| `EntityPlayer.sleepInBedAt`, `EntityPlayer.bedChunkCoordinates` | block/bed.rs | no (`blockActivated`) | WP-2.5 |
| `EntityPlayer.setInPortal` | block/portal.rs:185 | yes (entity collision) | WP-2.5 |
| `EntityLiving.heal heartsLife` | block/cake.rs:129 | no (`blockActivated`) | WP-2.2 |
| `EntityPlayerSP.displayGUIChest/Furnace/Dispenser` | chest.rs, furnace.rs, dispenser.rs | no | WP-2.5 (containers), GUI WP-3.6 |
| `ColorizerFoliage.getFoliageColor`, `ColorizerGrass.getGrassColor` | leaves.rs:123, grass.rs:88 | no (render colour) | Phase 3 (WP-3.3) |
| `Item.onItemUse`, `Item.onItemRightClick` | mc-golden tick_driver.rs:828, :856 | `use_item` op | WP-2.6 |
| `TileEntity*.writeToNBT/readFromNBT/updateEntity` | tile_entity/mod.rs | no: `TileEntityKindData::Untranscribed` is never constructed | dead, can go (nit N3) |
| `World.addLoadedEntities (chunk-stored entities)` (skipped_systems) | world/entities.rs:185 | no longer expected: since WP-1.9b, chunk entities join `loadedEntityList` before this call (chunk_provider.rs:280-288) | invariant check; stale doc (nit N4) |
| "… over a taken-out entity/player", "Entity ref to an entity no longer in loadedEntityList", "Chunk entity list holds an entity not in loadedEntityList", "EntityItem.writeEntityToNBT with a null item", "… randomDisplayUpdates around a player no longer in loadedEntityList", "Entity.mountEntity(null): mount out of its slot" | world/entities.rs, explosion.rs, pressure_plate.rs, chunk_loader.rs, tick_driver.rs:414, base.rs:1096 | defensive | not work items: guards for the take/put-back pattern and Java NPE paths |

No fixture run reports which of these strings it reached: the tick driver does
not emit `unimplemented_hits`. Question 8 proposes adding that.

## 5. Decisions the orchestrator took during Phase 1 (confirm or overturn)

1. **One integration branch.** All Phase 1 work landed on `phase-1`. Each WP ran
   in its own worktree (`.worktrees/wp-x.y`, branch `wp-x.y`), with up to seven
   in parallel in WP-1.6, and was then rebased onto `phase-1` and
   fast-forwarded. An integrator agent fixed merge fallout in "WP-x.y merge"
   commits: lifting ignores, removing dead dispatch tails, and the
   `COVERED_IN_GOLDEN_TICK` list. The owner therefore reviews `phase-1` as a
   whole, not each WP before the next starts, which is what
   `HANDOFF_WP-0.1.md`'s workflow assumed. The per-WP branches are kept. Phase 1
   also started while the questions in `docs/GATE0_REVIEW.md` were still
   unanswered: the owner's "do phase 1 and 2" was read as Gate 0 approval.
2. **Oracle harness growth.** The `Math.random()` rewrite script
   (`tools/oracle/rewrite-math-random.sh`) is unchanged; it is still the only
   rewrite of vanilla code. The Java harness grew by 942 lines:
   - new Tier A kinds `block_table` (BlockTable.java, a fresh class loader) and
     `item_table` (ItemTable.java), reflection dumps of every block and item;
   - `"world": "generated"`;
   - the ops `player`, `use_item` (the `blockActivated` branch of
     `clickMouse(1)`), `set_weather`, `explode` and `save` with
     `"save_format": "mcregion"`;
   - the dumps `ray_trace`, `power` and `weather`;
   - `harness.WorldReadDir` (cross-reads) and `harness.RegionCheck` (vanilla
     `RegionFile` reads Rust-written regions).

   PLAN_v4 §4.2 is silent on these; they are recorded in
   `tests/golden/README.md` and `tools/oracle/README.md`.
3. **The `mob_spawning` scenario knob.** With `"none"`, the harness tick calls
   `setAllowedMobSpawns(false, false)` instead of `(difficulty > 0, true)`, so
   `SpawnerAnimals.performSpawning` returns before it draws anything. 71
   scenarios set it (70 in `tick/`, plus `save/tile-entities`). That covers
   every fixture with a player except `tick/spawning-draws`,
   `tick/sapling-with-player`, `tick/bonemeal-sapling` and
   `entity/pig-attacked`, so these fixtures do not depend on the mob
   constructors. It departs from
   `Minecraft.runTick` (Minecraft.java:1163), and both engines apply it the
   same way. `tick/spawning-draws` runs with `"default"` and pins every
   spawner draw up to the constructors.
4. **The `explode` op and singleton seeding.** `explode` runs
   `World.createExplosion` (or, with `flaming`, the body of `newExplosion`)
   with the explosion's `ExplosionRNG` seeded `mix(seed, −6)`. The port seeds
   *every* explosion's Random this way: TNT, beds and the op. Vanilla seeds it
   from the clock (Explosion.java:11). Other singletons:
   - `Item.itemRand` and the chest, furnace and dispenser blocks' Randoms are
     `mix(seed, −1…−5)`;
   - each `TileEntityDispenser`'s own Random is `mix(seed, −7)`, in the port
     only (the oracle has no hook, census §2);
   - `randomDisplayUpdates`' per-call `new Random()` (World.java:1999) is one
     world-owned stream, `World::display_random`.

   Question 5 asks how the shipping game seeds these.
5. **The minimal player (WP-1.5).** The port builds a player with the pure
   `EntityPlayer(World)` / `EntityPlayerSP` constructor chain
   (`entity/player.rs`). This is enough for `playerEntities`, random ticks,
   spawner chunk selection, `joinEntityInSurroundings` and
   `randomDisplayUpdates`, plus `blockStrength`, `canHarvestBlock` and
   `swingItem`. The player's update is a recorded gap (`EntityPlayer.onUpdate`,
   `EntityLiving.onUpdate`). Implied gaps:
   - any fixture whose oracle player moves, falls or burns diverges once that
     shows in a dump (`save/tile-entities`, `tick/sapling-with-player`);
   - `addStat`, sleeping, portals, healing from cake and the GUIs are
     recorded (§4.2);
   - the port's player is `EntityPlayerSP` but is reported as the oracle's
     `HarnessPlayer`;
   - the harness save handler drops `EntityPlayerSP`'s `Score` from the
     `level.dat` player (EntityPlayerSP.java:98-101), so the result compares
     equal to the oracle's `EntityPlayer`.
6. **Worldgen block ids as `u8`.** Java stores generator ids as `byte`
   (`(byte)Block.x.blockID`). `worldgen::ids` narrows the single source
   `block::ids` to `u8` (WP-1.4 int step 3). The bit pattern is the same as
   Java's signed byte; every generator id is < 128 (the largest is 88).
7. **In-memory `RegionFile` (WP-1.9a).** `Storage` is a whole-blob interface,
   so a `RegionFile` holds the whole region in memory and writes it back after
   every change. A chunk save rewrites up to a few MB; PLAN_v4 Phase 4 (WP-4.3)
   revisits incremental writes. Decompression is eager, so damage *after* the
   end of the NBT document (e.g. a bad Adler-32) is an error in the port, while
   Java never reads it.
8. **Tier S entity NBT (WP-1.9b).** `EntityList` is a data table. Only
   `EntityItem` and `EntityFallingSand` can be constructed.
   - Every other saved id consumes an entity id and, for `EntityLiving`
     subclasses, makes the four `Math.random()` draws of `EntityLiving`'s
     constructor, so ids and the stream stay aligned. The gap is recorded.
   - The port keeps the stored compound and, on the next chunk save, writes
     it back unchanged at the end of the chunk's `Entities` list. Vanilla
     would write the entity's current state in its own order; the port's
     behaviour is marked "Not vanilla" at chunk_loader.rs:124.
   - `"Mob"` (abstract `EntityLiving`) is skipped and dropped, as in vanilla.
   - `EntityPlayer.readEntityFromNBT`'s bed and wake-up part is recorded
     (WP-2.5).
9. **Save-path choices (WP-1.9b).**
   - `SaveConverterMcRegion` without conversion (PLAN_v4 D3): pre-McRegion
     worlds are refused, not converted.
   - The D5 import validation falls back to `level.dat_old` as `loadWorldInfo`
     does.
   - A stolen session lock (`MinecraftException`) ends `World.tick` before
     `setWorldTime` and reaches the client as `World::session_lock_error`.
   - The harness save clock is fixed (`HARNESS_CLOCK_MILLIS`); `LastPlayed`,
     `SizeOnDisk` and the region timestamps are masked or not compared, on both
     sides.
10. **Skeleton interface additions** (§6: the owner approves anything touching
    the 0.5 skeleton's interfaces).
    - The WP-1.6 prep block dispatch: one free function per overridable
      `Block` method, matching on `BlockKind`, with a `TRANSCRIBED` registry
      and a coverage test generated from the vanilla override set.
    - `RandomSource { WorldRand, ChunkProviderRand }`: the generator's own
      Random, shared with `provideChunk`, as population requires.
    - The spawner's eligible-chunk `JavaHashSet` is world-owned instead of a
      JVM static.
    - `World::skip` / `note_unimplemented` as the uniform gap recorder.
11. **Tier S test placement.** `tick/dungeon-chest-save` is a tick-area
    fixture with a `save` op. It runs only in `golden_tick.rs`, so two test
    binaries never write the same `actual.jsonl`.
12. **Commit trailers.** The briefs asked for `Co-Authored-By: Claude Fable
    5.1`. Agents wrote `Claude Opus 5.5`, the model that actually made the
    commits: 65 Opus and 12 Fable trailers in `a094a54..6ebd8c9`. This
    document's commit also uses Opus 5.5.

## 6. Known divergences from vanilla (deliberate, documented in code)

| # | Divergence | Where | Effect | Ends with |
|---|---|---|---|---|
| K1 | `SpawnerAnimals` mob constructors not made: no entity id, no `EntityLiving` `Math.random()` draws, the mob never joins (`getCanSpawnHere` counted as false); the yaw draw is kept | spawner.rs module docs ("Known divergence") | any scenario where the oracle constructs a mob diverges from that tick (`tick/sapling-with-player`) | WP-2.3 |
| K2 | `TileEntityMobSpawner` stops at `EntityList.createEntityInWorld` | tile_entity/mob_spawner.rs | diverges from the first spawn attempt (`tick/mob-spawner-spawn`) | WP-2.3 |
| K3 | `randomDisplayUpdates`' `new Random()` per call is one seeded `display_random` stream | world/mod.rs:274, tick.rs:428 | particles and sounds only. `tick/portal-display` pins that `World.rand` is not drawn and no block changes; the display draw order itself is unverified | by design (D9) |
| K4 | Clock-seeded Randoms seeded instead: every `ExplosionRNG` = `mix(seed, −6)`, `TileEntityDispenser`'s = `mix(seed, −7)` | explosion.rs, dispenser | the dispenser's Random only matters with ≥ 2 stacks (census row) | by design; question 5 |
| K5 | `EntityLightningBolt` not constructed: no id, no bolt draws, no fire at difficulty ≥ 2 | tick.rs:340-352 | thunderstorms after a strike | WP-2.4 |
| K6 | Unconstructible saved entities kept and re-written unchanged at the end of the list | save/entity_list.rs, chunk_loader.rs:124 | re-saved chunks differ from vanilla in entity order and state | Phase 2 constructors |
| K7 | Eager region decompression | save/region.rs docs | trailing damage is an error here, ignored by Java | accepted |
| K8 | `use_item`: `Item.onItemUse` taken as false, `onItemRightClick` returns the stack | mc-golden tick_driver.rs | item placement and bone meal fixtures (§2.1) | WP-2.6 |
| K9 | Oracle `Math` == `StrictMath` (Gate 0 decision 1) | tools/oracle/env.sh | 1 ulp from the real client on 1–4 % of trig/pow inputs | owner decision pending (PLAN_v4 §8 item 4) |
| K10 | Hostile spawning unseeded in the oracle (`Entity.rand` before join); harness `difficulty` 0 | census §3 | no hostile-spawn fixtures possible | PLAN_v4 §8 item 3, owner |

Vanilla bugs transcribed (34 `vanilla bug` markers in `crates/`), each pinned by
a fixture or unit test where one can observe it:
- SpawnerAnimals: the `nextInt(1)` draws and the in-loop running count.
- `BlockRedstoneOre.sparkle` tests `< 0.0D` on y.
- `BlockFire`: the missing return, and the rain test using the fire's z
  (BlockFire.java:97).
- The empty-body sponge.
- The trapdoor's power branch on its emptied cell.
- Retracting piston boxes moving away from their source.
- The bed explosion's unused midpoint.
- The second furnace tile entity left in `loadedTileEntityList`.
- Fluids: the dry-up fall of level 7.
- `BlockRail`: raw-meta slope bounds, and `RailLogic`'s `? true : true`.
- `Chunk` passing chunk-local coordinates to the render update.
- `rayTraceBlocks` moving the caller's start vector.
- `chunkMap` key aliasing.
- A flaming explosion leaving no fire.
- `faceToSide` indexed past its six entries.
- The `WorldChunkManager` `var4, var4` width.
- `McRegionChunkLoader` setting `xPos`/`zPos` on the root compound (the
  relocation bug).
- `RegionFile`'s padding loop zeroing the header, and the header sectors
  freed by a corrupt entry.
- Population: the nested-population `fallInstantly` reset, and the nether
  `hellRNG` load-order dependence (`worldgen/nether-seed-123-order-ab/-ba`).
- The dungeon chest slot overwrite.
- The cave carver off by one in y.

## 7. Reviewer findings (nits, none behavioural)

- **N1.** The `math`, `block` and `item` fixture areas have no "every fixture
  has a test" check (the other areas have one).
- **N2.** Each dispatch.rs entry point still ends with an unreachable
  `if let Some(what) = what { untranscribed(world, what) }` after an
  all-`None` match. The WP-1.6d/e and 1.8 merges removed it from 18 methods;
  the rest can go too.
- **N3.** `TileEntityKindData::Untranscribed` and its three recorded arms in
  tile_entity/mod.rs are never constructed ("none is left").
- **N4.** `World::add_loaded_entities` (world/entities.rs:177-186) still says
  chunk entities "arrive with the chunk loader, WP-1.9; ids already present
  are ignored until then". Since WP-1.9b they join `loadedEntityList` first,
  so the `skip` there is an invariant check, and the doc is stale.
- **N5.** `save/testworld` compares one chunk of one region (§3); the full
  world matches as well.
- **N6.** `cross_read_*` in golden_save.rs return early and report `ok` when
  `build/oracle` is missing. Ordinary CI runs only the Rust side, so there they
  always "pass" without checking anything.
- **N7.** The tick driver does not report which recorded gaps a fixture
  reached. A per-fixture `gaps` list next to `actual.jsonl` would show Phase 2
  where each ignored fixture stops beyond its first difference.

## 8. Open owner questions

1. **Gate 1 with exceptions?** Accept Gate 1 with the 6 Tier B fixtures and the
   1 Tier S fixture in §2.1 moved to the Phase 2 gate? Each stops only at a
   Phase 2 class.
2. Confirm the integration-branch workflow (§5.1) and that Phase 1 started with
   `docs/GATE0_REVIEW.md` unanswered. That document's questions are still open:
   the skeleton questions, leaves' fancy-graphics `isOpaqueCube`, and PLAN_v4 §8
   items 3 and 4.
3. Accept the oracle harness additions (§5.2) as within PLAN_v4 §4.2, given
   that the vanilla rewrite is still only `Math.random()`?
4. Keep the `mob_spawning: "none"` knob (§5.3) for Tier B isolation?
5. **Seeding outside the harness.** In the shipping game, what seeds the
   Randoms that vanilla seeds from the clock: `ExplosionRNG`,
   `TileEntityDispenser`'s Random, `randomDisplayUpdates`' Random, and
   `Entity.rand`? D9 keeps clocks out of `mc-sim`, so `mc-app` has to supply
   them. A proposal: per-world streams seeded at load from the session.
6. Make the full-`testWorld` check (784 chunks, 4 regions, 53 tile entities;
   it passes) the committed `save/testworld` fixture? That means committing
   the other three regions (3.6 MB in total) and depends on question 9.
7. Should the cross-read tests be `#[ignore = "needs build/oracle"]` (or fail
   in the `golden-regen` job) instead of passing silently without the oracle?
8. Add per-fixture gap reporting (N7) before Phase 2 starts?
9. **`testWorld` licensing note.** `docs/REFERENCES.md` describes
   `mc_b1.7.3-world/testWorld/` as "Mojang-generated save data; local only".
   But `tests/golden/save/testworld/world/{level.dat, region/r.0.-1.mcr}`
   (1.2 MB) has been committed since WP-0.4 (`6b8d703`), and
   `tests/golden/README.md` calls it "not Mojang content". Which is correct,
   and may the save stay in git?
10. Is the Tier S entity policy (§5.8, K6) acceptable until the Phase 2
    constructors land? It keeps unconstructible entities' stored compounds and
    writes them back at the end of the list.
11. **PLAN_v4 §8 item 3 (`Entity.rand` rewrite) now blocks WP-2.3.** Without
    it, no hostile-spawning fixture is reproducible. Decide before Phase 2
    wave C.

## Appendix: every fixture

Status as of `6ebd8c9`, from `cargo test --workspace` (ignored tests were also
run with `--ignored` and fail as described in §2).

| Fixture | Tier | Status | Rust test |
|---|---|---|---|
| `block/block-table` | A | pass | `golden_block_table.rs::block_table_matches_oracle` |
| `entity/arrow-flight` | C | ignored | `golden_tick.rs::entity_arrow_flight` |
| `entity/flat-item-drop` | C | pass | `golden_tick.rs::entity_flat_item_drop` |
| `entity/flat-pig-falls` | C | ignored | `golden_tick.rs::entity_flat_pig_falls` |
| `entity/item-drop` | C | pass | `golden_tick.rs::entity_item_drop` |
| `entity/pig-attacked` | C | ignored | `golden_tick.rs::entity_pig_attacked` |
| `entity/pig-falls` | C | ignored | `golden_tick.rs::entity_pig_falls` |
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
| `math/math-sweep-strict` | A | pass | `mc-core math_sweep_report.rs::golden_math_sweep_strict_fixture_eq` |
| `math/math-sweep` | A | pass | `mc-core math_sweep_report.rs::golden_math_sweep_strict_eq (+ math_sweep_report)` |
| `math/sine-table` | A | pass | `mc-core golden_sine_table.rs::golden_sine_table` |
| `nbt/empty-root` | A | pass | `mc-nbt golden_nbt.rs::nbt_empty_root` |
| `nbt/level-dat-shape` | A | pass | `mc-nbt golden_nbt.rs::nbt_level_dat_shape` |
| `nbt/nan-payloads` | A | pass | `mc-nbt golden_nbt.rs::nbt_nan_payloads` |
| `nbt/primitives` | A | pass | `mc-nbt golden_nbt.rs::nbt_primitives` |
| `nbt/testworld-level-dat` | A | pass | `mc-nbt golden_nbt.rs::nbt_testworld_level_dat` |
| `rng/java-random--4172144991579068934` | A | pass | `mc-core golden_rng.rs::java_random_neg_4172144991579068934` |
| `rng/java-random-0` | A | pass | `mc-core golden_rng.rs::java_random_0` |
| `rng/java-random-123` | A | pass | `mc-core golden_rng.rs::java_random_123` |
| `save/roundtrip-seed-123` | S | pass | `golden_save.rs::save_roundtrip_seed_123` |
| `save/testworld` | S | pass | `golden_save.rs::save_testworld (+ testworld_chunk_digest_and_chest)` |
| `save/tile-entities` | S | ignored | `golden_save.rs::save_tile_entities` |
| `tick/activate-workbench-stairs` | B | pass | `golden_tick.rs::tick_activate_workbench_stairs` |
| `tick/bed-nether` | B | pass | `golden_tick.rs::tick_bed_nether` |
| `tick/bed-place-and-break` | B | pass | `golden_tick.rs::tick_bed_place_and_break` |
| `tick/block-item-placement` | B | ignored | `golden_tick.rs::tick_block_item_placement` |
| `tick/bonemeal-sapling` | B | ignored | `golden_tick.rs::tick_bonemeal_sapling` |
| `tick/button-lever` | B | pass | `golden_tick.rs::tick_button_lever` |
| `tick/cactus-reed-growth` | B | pass | `golden_tick.rs::tick_cactus_reed_growth` |
| `tick/cake-eat` | B | pass | `golden_tick.rs::tick_cake_eat` |
| `tick/chest-double-and-break` | B | pass | `golden_tick.rs::tick_chest_double_and_break` |
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
| `tick/empty-world-20` | B | pass | `golden_tick.rs::tick_empty_world_20` |
| `tick/explosion-crater` | B | pass | `golden_tick.rs::tick_explosion_crater` |
| `tick/explosion-fire` | B | pass | `golden_tick.rs::tick_explosion_fire` |
| `tick/fence-stairs-step-bounds` | C | pass | `golden_tick.rs::tick_fence_stairs_step_bounds` |
| `tick/fire-burnout-stone` | B | pass | `golden_tick.rs::tick_fire_burnout_stone` |
| `tick/fire-farmland-rain` | B | pass | `golden_tick.rs::tick_fire_farmland_rain` |
| `tick/fire-rain-spread` | B | pass | `golden_tick.rs::tick_fire_rain_spread` |
| `tick/fire-spread-planks` | B | pass | `golden_tick.rs::tick_fire_spread_planks` |
| `tick/flat-flowing-water` | B | pass | `golden_tick.rs::tick_flat_flowing_water` |
| `tick/flat-lava-fire` | B | pass | `golden_tick.rs::tick_flat_lava_fire` |
| `tick/flat-lava-flow-nether` | B | pass | `golden_tick.rs::tick_flat_lava_flow_nether` |
| `tick/flat-lava-flow` | B | pass | `golden_tick.rs::tick_flat_lava_flow` |
| `tick/flat-lava-water-fizz` | B | pass | `golden_tick.rs::tick_flat_lava_water_fizz` |
| `tick/flat-sand-falls` | B | pass | `golden_tick.rs::tick_flat_sand_falls` |
| `tick/flat-torch-light` | B | pass | `golden_tick.rs::tick_flat_torch_light` |
| `tick/flat-water-blockers` | B | pass | `golden_tick.rs::tick_flat_water_blockers` |
| `tick/flat-water-drain` | B | pass | `golden_tick.rs::tick_flat_water_drain` |
| `tick/flat-water-flow` | B | pass | `golden_tick.rs::tick_flat_water_flow` |
| `tick/flat-water-item-push` | C | pass | `golden_tick.rs::tick_flat_water_item_push` |
| `tick/flat-water-lava-mix` | B | pass | `golden_tick.rs::tick_flat_water_lava_mix` |
| `tick/flat-water-spread-holes` | B | pass | `golden_tick.rs::tick_flat_water_spread_holes` |
| `tick/furnace-idle` | B | pass | `golden_tick.rs::tick_furnace_idle` |
| `tick/grass-spread` | B | pass | `golden_tick.rs::tick_grass_spread` |
| `tick/ladder-placement` | B | pass | `golden_tick.rs::tick_ladder_placement` |
| `tick/leaves-decay` | B | pass | `golden_tick.rs::tick_leaves_decay` |
| `tick/locked-chest-vanishes` | B | pass | `golden_tick.rs::tick_locked_chest_vanishes` |
| `tick/mob-spawner-delay` | B | pass | `golden_tick.rs::tick_mob_spawner_delay` |
| `tick/mob-spawner-spawn` | B | ignored | `golden_tick.rs::tick_mob_spawner_spawn` |
| `tick/note-jukebox-activate` | B | pass | `golden_tick.rs::tick_note_jukebox_activate` |
| `tick/piston-immovable` | B | pass | `golden_tick.rs::tick_piston_immovable` |
| `tick/piston-moving-block-tick` | B | pass | `golden_tick.rs::tick_piston_moving_block_tick` |
| `tick/piston-orphan-blocks` | B | pass | `golden_tick.rs::tick_piston_orphan_blocks` |
| `tick/piston-push-limit` | B | pass | `golden_tick.rs::tick_piston_push_limit` |
| `tick/piston-push-stone` | B | pass | `golden_tick.rs::tick_piston_push_stone` |
| `tick/piston-retract-entity` | B | pass | `golden_tick.rs::tick_piston_retract_entity` |
| `tick/piston-retract-unpowered` | B | pass | `golden_tick.rs::tick_piston_retract_unpowered` |
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
| `tick/redstone-item-placement` | B | ignored | `golden_tick.rs::tick_redstone_item_placement` |
| `tick/redstone-ore-touch` | B | pass | `golden_tick.rs::tick_redstone_ore_touch` |
| `tick/redstone-removal` | C | pass | `golden_tick.rs::tick_redstone_removal` |
| `tick/redstone-torch-inverter` | B | pass | `golden_tick.rs::tick_redstone_torch_inverter` |
| `tick/redstone-wire-line` | B | pass | `golden_tick.rs::tick_redstone_wire_line` |
| `tick/repeater-delays` | B | pass | `golden_tick.rs::tick_repeater_delays` |
| `tick/sapling-grow` | B | pass | `golden_tick.rs::tick_sapling_grow` |
| `tick/sapling-with-player` | B | ignored | `golden_tick.rs::tick_sapling_with_player` |
| `tick/sign-pops-off` | B | pass | `golden_tick.rs::tick_sign_pops_off` |
| `tick/snow-ice` | B | pass | `golden_tick.rs::tick_snow_ice` |
| `tick/soul-sand-web-item` | C | pass | `golden_tick.rs::tick_soul_sand_web_item` |
| `tick/spawning-draws` | B | pass | `golden_tick.rs::tick_spawning_draws` |
| `tick/sponge-removal-notifies` | B | pass | `golden_tick.rs::tick_sponge_removal_notifies` |
| `tick/tnt-by-fire` | B | ignored | `golden_tick.rs::tick_tnt_by_fire` |
| `tick/torch-placement` | C | pass | `golden_tick.rs::tick_torch_placement` |
| `tick/trapdoor-toggle` | B | pass | `golden_tick.rs::tick_trapdoor_toggle` |
| `tick/use-item-noop` | B | pass | `golden_tick.rs::tick_use_item_noop` |
| `tick/water-flow` | B | pass | `golden_tick.rs::tick_water_flow` |
| `tick/weather-ice` | B | pass | `golden_tick.rs::tick_weather_ice` |
| `tick/weather-snow` | B | pass | `golden_tick.rs::tick_weather_snow` |
| `tick/weather-time` | B | pass | `golden_tick.rs::tick_weather_time` |
| `worldgen/nether-seed--4172144991579068934` | A | pass | `golden_worldgen.rs::nether_seed_neg_4172144991579068934` |
| `worldgen/nether-seed-123-order-ab` | A | pass | `golden_worldgen.rs::nether_seed_123_order_ab` |
| `worldgen/nether-seed-123-order-ba` | A | pass | `golden_worldgen.rs::nether_seed_123_order_ba` |
| `worldgen/nether-seed-123` | A | pass | `golden_worldgen.rs::nether_seed_123` |
| `worldgen/nether-seed-42-spread` | A | pass | `golden_worldgen.rs::nether_seed_42_spread` |
| `worldgen/nether-seed-8675309` | A | pass | `golden_worldgen.rs::nether_seed_8675309` |
| `worldgen/overworld-seed--4172144991579068934` | A | pass | `golden_worldgen.rs::overworld_seed_neg_4172144991579068934` |
| `worldgen/overworld-seed-123-dungeon` | A | pass | `golden_worldgen.rs::overworld_seed_123_dungeon` |
| `worldgen/overworld-seed-123` | A | pass | `golden_worldgen.rs::overworld_seed_123` |
| `worldgen/overworld-seed-42-spread` | A | pass | `golden_worldgen.rs::overworld_seed_42_spread` |
| `worldgen/overworld-seed-8675309` | A | pass | `golden_worldgen.rs::overworld_seed_8675309` |
