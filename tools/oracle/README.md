# Java oracle (PLAN_v4 §4, ADR-007)

Compiles the vanilla b1.7.3 client source from the git-ignored reference tree
(`.reference/mc_b1.7.3_release/1.7.3-LTS`) together with a small harness, and
writes the golden fixtures under `tests/golden/` (format: `tests/golden/README.md`).
Needs a JDK ≥ 17 (`javac`/`java` on `PATH`); the harness sources are Java 17.

| File | Purpose |
|---|---|
| `build.sh` | `rewrite-math-random.sh`, then `javac` of `build/oracle-src` + `java/` into `build/oracle` (§4.1) |
| `rewrite-math-random.sh` | Copies `src/minecraft` to `build/oracle-src`, rewrites `Math.random()` |
| `regen.sh` | `build.sh`, runs the harness into `build/golden-out`, copies the results into `tests/golden`, prints `git status --short tests/golden` |
| `check-reproducible.sh` | Three harness runs, byte-compared (§4.4 item 1) |
| `check-sensitivity.sh` | One run with every seed XOR 1; every seeded fixture must change, every unseeded one must not; only `sine_table`, `nbt_write`, `nbt_read`, `java_hash_order`, `world_read`, `block_table`, `item_table`, `recipe_table`, `stat_table` may lack a seed (§4.4 item 2) |
| `env.sh` | Shared classpath (`minecraft.jar` last), the JVM math flags (§3.4 rule 5) and the `oracle_run` helper, sourced by the scripts |
| `census.md` | Every `Math.random()` and `new Random(` site in the vanilla tree, entity-constructor draws, clock reads, static state: what the harness does about each (WP-0.4) |
| `java/harness/` | `Main` (driver, tier dispatch), `TierA` (`java_random`, `sine_table`, `math_sweep`, `terrain`), `BlockTable` (`block_table`), `ItemTable` (`item_table`), `RecipeTable` (`recipe_table`), `StatTable` (`stat_table`), `Nbt` (`nbt_write`, `nbt_read`), `HashOrder` (`java_hash_order`), `TickScenario` (kind `tick`: ops, tick loop), `Dumps` (Tier B/C dumps, incl. `ray_trace`, `power`, `inventory` and `map_data`), `TierS` (`save` op, `world_read`), `HarnessWorld` (world construction, seeding protocol, static resets), `HarnessWorldAccess` (the `IWorldAccess` with the entity seeding hook), `HarnessPlayer`, `EmptyChunkProvider` (`"world": "empty"`), `Rng` (reads `java.util.Random` state), `Reflect`, `Num`, `Blake3`, `Json`, `Hex`, `Trace`, `HarnessMath` |

Tier A kinds (specified in `tests/golden/README.md`): `java_random`, `sine_table`,
`math_sweep` (optional `"strict": true` for `StrictMath`), `terrain` (WP-0.2);
`nbt_write`, `nbt_read`, `java_hash_order` (WP-0.3); `block_table` (WP-1.1), `item_table` (WP-1.2), `recipe_table` (WP-2.5), `stat_table` (WP-2.7). Tier B/C/S kinds (WP-0.4):
`tick` (a world ticked under ops and dumps; tiers B, C, S) and `world_read` (S).

## Use

```sh
tools/oracle/regen.sh               # build + regenerate tests/golden (about 7 s)
tools/oracle/check-reproducible.sh  # needs a built oracle
tools/oracle/check-sensitivity.sh   # compares against the committed fixtures
```

Scripts are POSIX `sh` and work from any directory. Direct use:

```sh
java $JAVA_MATH_FLAGS -cp "build/oracle:$CP" harness.Main tests/golden build/golden-out [area/name ...]
java -cp "build/oracle:$CP" harness.Main --selftest       # BLAKE3 test vectors
java $JAVA_MATH_FLAGS -Dharness.perturbSeed=1 -cp ... harness.Main ...  # XOR every scenario seed
```

Every run checks BLAKE3 against the official test vectors first. Never edit an
`expected.jsonl` by hand. Drift after a regen without a harness change is a
determinism bug: investigate it, do not commit it.

## The one rewrite of vanilla code

`rewrite-math-random.sh` replaces every `Math.random()` with
`harness.HarnessMath.random()` and nothing else (58 occurrences on 49 lines;
the script fails if the count is 0 or the rewrite is incomplete).
`HarnessMath` is one `java.util.Random` stream, reseeded from the scenario seed
(0 when there is none) at the start of every scenario. No other vanilla file is
edited; the copy is regenerated on every build.

## Determinism overrides

Applied by `HarnessWorld.makeDeterministic` (Tier A and B/C/S alike) right after `new World(new SaveHandlerMP(),
"harness", provider, seed)`, all derived from the scenario seed:

- `world.rand.setSeed(seed)` (World.java:82, `new Random()`).
- `field_9437_g = new Random(seed).nextInt()` (World.java:75, clock-seeded
  random-tick LCG state), set by reflection.
- `soundCounter = <that same Random>.nextInt(12000)` (World.java:90, drawn from
  the unseeded `rand`), set by reflection.

`lockTimestamp` (World.java:80, wall clock) is left alone: it is only copied into
`World(World, WorldProvider)` (World.java:134) and never read. Tier A traces contain
no entity state (b1.7.3 has no spawning during generation). Block and Item statics
initialise normally. Tier B/C/S add the seeding protocol below.

`java.lang.Math` results: every script runs the JVM with `JAVA_MATH_FLAGS` from
`env.sh` (HotSpot's transcendental intrinsics disabled, PLAN_v4 §3.4 rule 5,
decided in WP-0.3), so `Math` equals `StrictMath` (fdlibm) on every JDK and CPU and
`math/math-sweep` equals `math/math-sweep-strict`. Use the same flags when running
`harness.Main` by hand. The fixtures were generated on OpenJDK 25, x86-64. Measured on
that JVM (WP-0.3, PLAN_v4 §3.4 rule 5): `-Xint`, C1-only and
C2-only (`-Xcomp -XX:-TieredCompilation`) give the same `Math` bits as the default
(the interpreter and both compilers use the same stubs, so warm-up does not change
results); with `-XX:+UnlockDiagnosticVMOptions
-XX:DisableIntrinsic=_dsin,_dcos,_dtan,_datan2,_dpow,_dexp,_dlog,_dlog10`
`Math` equals `StrictMath` (`math/math-sweep-strict`) on every input, which is why
those flags were adopted.

`java_hash_order` fixtures record the oracle JDK's `HashSet` order. `HashOrder`
cross-checks it against a model of the JDK 8+ table and refuses scenarios whose
order would depend on `System.identityHashCode` (equal-hash keys in a tree bin);
`regen.sh` prints the model's final capacity and tree-bin count per scenario.

`nbt_read` input files (e.g. `tests/golden/nbt/testworld-level-dat/level.dat`)
are re-emitted unchanged by the oracle, so `regen.sh`'s replace-all-generated-files
step keeps them.

## CI

The `golden-regen` job in `.github/workflows/ci.yml` (manual, Temurin 21) calls
`regen.sh`. It only does real work on a runner that has the git-ignored
reference tree; elsewhere it reports and exits 0. Ordinary CI runs only the
Rust side against the committed fixtures.

## `block_table` (WP-1.1)

`BlockTable` opens a `URLClassLoader` over the harness's own class path whose
parent is the platform loader, so `net.minecraft.src.*` (and `harness.HarnessMath`,
which the rewritten vanilla code calls) are loaded and statically initialised afresh
in it: the dump never sees singleton state that an earlier scenario in the same JVM
mutated (`BlockFlowing` scratch fields, bounds, `ignoreUpdates`, …) and does not
depend on the scenario order or on `-Dharness.perturbSeed`. Everything vanilla is
reached by reflection (`Class.forName(..., true, loader)`, `getDeclaredFields`,
`getDeclaredMethod` up the class chain with `setAccessible`). The static initialiser
prints `16 achievements` / `151 recipes` to stdout (AchievementList.java:49,
CraftingManager.java:82);
that is not part of the trace. Line format and reduction rules:
`tests/golden/README.md`, section `kind: "block_table"`.

## Tier B/C/S (WP-0.4)

Scenario format, op and dump semantics and the tick loop are specified in
`tests/golden/README.md` ("Tier B/C/S scenarios"); this section is the oracle side.

### Seeding protocol (PLAN_v4 §4.2, ADR-007 §4.3)

Per scenario, before the world exists (`HarnessWorld.resetStatics`, `TickScenario.run`):

1. `HarnessMath.seed(seed)`: the `Math.random()` stream (58 rewritten sites, census §1).
2. `Entity.nextEntityID = 0`, `NextTickListEntry.nextTickEntryID = 0` (reflection); the
   static `RegionFileCache` closed and cleared; transient statics back to their initial
   values (census §6).
3. Unseeded singleton Randoms reseeded with `mix(seed, k)`, where
   `mix(s, k) = s ^ (k * 0x9E3779B97F4A7C15L)` (wrapping): `Item.itemRand` k = −1,
   `Block.chest` −2, `Block.stoneOvenIdle` −3, `Block.stoneOvenActive` −4,
   `Block.dispenser` −5 (census §2).

After `new World(...)`: the determinism overrides above (`rand`, `field_9437_g`,
`soundCounter`), then `world.addWorldAccess(new HarnessWorldAccess(seed))`. Its
`obtainEntitySkin(entity)` is the entity hook: vanilla calls it from
`World.entityJoinedWorld` (World.java:915; b1.7.3 has no `spawnEntityInWorld`, this is it)
and from `World.addLoadedEntities` (World.java:2081, entities of a loading chunk). It sets
`entity.rand.setSeed(mix(seed, entityId))`, redraws the constructor's non-persisted
draws (`EntityChicken.timeUntilNextEgg`, `EntitySquid.rotationVelocity`) on that stream and
sets the seed again (`HarnessWorld.seedOnJoin`). Entities the harness constructs (`spawn`,
`player` ops) are additionally seeded right after construction (`EntitySlime`'s size and
jump delay are replayed then) and spawned with `entityJoinedWorld`, so the hook seeds them
too. All other `IWorldAccess` callbacks only count calls (`HarnessWorldAccess.calls`, not
traced: `playSound`/`spawnParticle` counts depend on the unseeded Random of
World.java:1999).

What stays unseeded (census §2-3): draws on `Entity.rand` before an entity joins, made by
vanilla code — hostile mob spawn checks (`EntityMob`, `EntityGhast`, `EntitySlime`
`getCanSpawnHere`), heading jitter of arrows/snowballs/eggs/fish/fireballs constructed by
mobs, players and dispensers, painting art, lightning bolts — plus
`TileEntityDispenser.dispenserRandom` and `Explosion.ExplosionRNG` (flaming explosions).
Hence the `difficulty` default of 0 and the warning for a player scenario that raises it
(not given with `"mob_spawning": "none"`, which turns natural spawning off altogether:
`HarnessWorld.runTick` calls `setAllowedMobSpawns(false, false)` instead of the client's
`(difficulty > 0, true)`, so `SpawnerAnimals.performSpawning` returns before any draw; a
Tier B isolation knob for scenarios about blocks). Closing these needs a second scripted
rewrite (Entity.java:96), proposed in census §3.

`SpawnerAnimals.eligibleChunksForSpawning` is a JVM-wide static `HashSet` that keeps its
table capacity between scenarios. It is not reset: with one player every call fills it with
the same 289 keys, the table is 512 wide from the first call on whether it grew there in this
scenario or an earlier one, and the iteration order of the keys is the same either way
(checked with `mc_core::JavaHashSet`, WP-1.5); `regen.sh` and subset runs agree.

`rng_state` dumps read `java.util.Random` state through its serial form (`Rng`, no
`--add-opens`; self-tested at start-up against `new Random(42)`).

### `HarnessPlayer`

`EntityPlayerSP` needs a `Minecraft` (constructor, `onLivingUpdate` reads
`mc.statFileWriter`, `mc.currentScreen`, `movementInput`), so `harness.HarnessPlayer`
extends `EntityPlayer` directly: the one abstract method, `func_6420_o`, is a no-op as in
EntityPlayerSP.java:169. It has the `EntityPlayerSP` overrides that need no client: a
`MovementInput` (all zero until an `input` op sets it) applied after `EntityPlayer`'s
`updatePlayerActionState` as EntityPlayerSP.java:27-32 does, and `isSneaking()` =
`movementInput.sneak && !sleeping` (EntityPlayerSP.java:144-146). Its username is `"Player"`,
as a session gives an `EntityPlayerSP` one (EntityPlayerSP.java:20; `EntityPlayer.onDeath`
dereferences it, EntityPlayer.java:223). It keeps the last `sleepInBedAt` result for the
`sleep` op. Not modelled: the rest of `EntityPlayerSP.onLivingUpdate` (portal timing and
`Minecraft.usePortal`, the sneak `ySize` dip, `pushOutOfBlocks`), statistics/achievements
(EntityPlayer's `addStat` is a no-op), the pickup animation. The port has those
`EntityPlayerSP` parts behind `PlayerSpData::client_additions`, which the golden driver
leaves off, except the statistics: the port always counts them into the player's
`stat_file_writer` (WP-2.7), which no dump reads; `crates/mc-sim/tests/stats.rs` covers
them and `stat/stat-table` the registries. Vanilla paths that only run with a player in
`World.playerEntities`: random block ticks, snow/ice, lightning and cave sounds
(World.updateBlocksAndPlayCaveSounds, chunks within 9 of a player), mob spawning
(SpawnerAnimals.performSpawning, chunks within 8), sleeping, despawning and AI target
search (`getClosestPlayer`), item pickup, and the client's `World.randomDisplayUpdates`
(6000 `world.rand` draws per tick), which the tick loop runs for it. A player therefore
also loads (generates) the 19 × 19 chunks around it on the first tick. A player that dies
stays in `World.playerEntities` after it leaves `loadedEntityList` (only `World.setEntityDead`
removes it, World.java:945; the client's respawn calls that).

`use_item`, `interact` and `attack` drive the vanilla controller code without a
`Minecraft`: `new PlayerControllerSP(null)` (its constructor only stores `mc`;
`sendPlaceBlock` and `sendUseItem` never touch it), `EntityPlayer.useCurrentItemOnEntity`
(what `PlayerController.interactWithEntity` calls, inlined in `TickScenario.interact` to
report `entity.interact(player)`) and `EntityPlayer.attackTargetEntityWithCurrentItem`. The
ray trace that picks the block or entity (`Minecraft.objectMouseOver`) is replaced by the
op's coordinates or entity id. Left-click block breaking (`dig`) cannot use the vanilla
`PlayerControllerSP`: its `clickBlock`, `sendBlockRemoving` and `sendBlockRemoved` reach
the world and the player through `mc.theWorld` / `mc.thePlayer`, and `Minecraft.thePlayer`
is an `EntityPlayerSP`. `HarnessWorld.Controller` mirrors those methods (and
`PlayerController.sendBlockRemoved`, `resetBlockRemoving`, `updateController`) line for line
with the world and the player as fields, leaving out only the dig step sound
(`mc.sndManager.playSound`, no draws); the button logic around it (`clickMouse(0)` on a
press and every 5 ticks while held, `func_6254_a`) is `TickScenario.dig` /
`releaseButton`. The `sleep` op calls the bed's `blockActivated`.

### `"world": "empty"`

`EmptyChunkProvider` replaces the terrain generator inside the world's vanilla
`ChunkProvider` (private field `chunkProvider`, set by reflection right after the World
constructor, which loads no chunk), so chunk caching, population flags, saving and
unloading stay vanilla. Its chunks are all air with `generateSkylightMap()` applied, as
`ChunkProviderGenerate.provideChunk` finishes a chunk.

### Tier S notes

- `"save_format": "mcregion"` builds the world on `new SaveOldDir(<temp>, "world", true)`
  (`Files.createTempDirectory`); the directory is deleted after the scenario. `SaveOldDir`
  writes `session.lock` at construction and `level.dat` on every save, including the
  autosave `World.tick` runs every 40 ticks.
- `world_read` copies the committed save to a temp directory before opening it; so does a
  `tick` scenario with `"world": "load"` (`TickScenario.load`, `HarnessWorld.open` with the
  scenario's seed and difficulty), whose `dir` may climb out of the scenario directory but
  not out of `tests/golden`. The copy is the running world's save (autosaves, `save` ops)
  and is deleted afterwards.
- Read-back always goes through vanilla readers (`CompressedStreamTools`,
  `RegionFileCache.getChunkInputStream`, `McRegionChunkLoader.loadChunk`,
  `TileEntity.createAndLoadEntity`, `new WorldInfo(NBTTagCompound)`); raw bytes of
  `.mcr`/`level.dat` are never traced (compound order is HashMap order, zlib/gzip output is
  implementation specific). Region sector tables and lengths go into `dbg`.
- After a `save` op the oracle does not build `Chunk`s from the files: that constructs the
  saved entities (entity ids, `Math.random()` draws) and would perturb the running
  scenario. `world_read` does, for `chunk_digest` stage `loaded`.

### Determinism checks

`check-reproducible.sh` runs every scenario three times in one JVM each; the Tier B/C
scenarios with a player load 361 chunks and spawn animals and squid, which exercises the
hook. `check-sensitivity.sh` requires every `tick` scenario (all have a seed) to change
under a flipped seed bit; the empty-world scenarios do so through `rng_state` or entity
`rand` fields, since their blocks do not depend on the seed.

## `item_table` (WP-1.2)

`ItemTable` dumps vanilla static state, so it needs no world and no seed. It calls
`HarnessWorld.resetStatics(0)` first: that initialises `Block` before `Item` (the
order the game uses; `Item` first fails in `AchievementList`) and reseeds
`Item.itemRand`, which nothing in the dump draws from. Field values come from
reflection over each class in the item's chain (`getDeclaredFields`, non-static,
non-synthetic, sorted by name so the output is byte-stable); a field type without a
reduction rule (tests/golden/README.md) aborts the scenario instead of being
dropped. Every vanilla call is wrapped so an exception becomes
`{"exception":"<simple name>"}` in the trace; the dump calls no method with side
effects on shared state (`getStrVsBlock`/`canHarvestBlock` read block singletons,
`ItemStack` operations work on fresh stacks). The one exception is the
`block_texture` dump: the piston base face reads the `pistonBase` /
`pistonStickyBase` singletons' mutable bounds, which `resetStatics` does not
restore, so `ItemTable` calls `setBlockBoundsForItemRender()` on both first (full
cube, the static-init state) in case an earlier tick scenario in the same JVM left
a piston extended.

## Containers and crafting ops (WP-2.5)

`TickScenario` keeps the container a `slot_click` opened in a static field
(`openContainer`, with its spec string), reset at the start of every tick scenario; the
`"player"` spec always means the `HarnessPlayer`'s own `inventorySlots`. Switching to another
spec drops the previous container without `onCraftGuiClosed` (which would drop items
through `dropPlayerItem` and so draw from the player's `rand`): a scenario that wants a
workbench grid emptied clicks its stacks out first. The chest inventory is composed exactly
as `BlockChest.blockActivated` does (lines 198-212), copied into the harness because that
method's last step, `displayGUIChest`, is `EntityPlayer`'s empty body for the
`HarnessPlayer`. The `craft` op's grid uses the same slot-less `Container` as
`RecipeTable.grid`. Neither op draws from any `Random`; fixtures still dump `rng_state` so
`check-sensitivity.sh` sees the seed.

## Item behaviour and `map_data` (WP-2.6)

The `use_item`, `interact` and `attack` ops reach every `Item` behaviour override through the
vanilla controller code (above). `Item.itemRand` is reseeded per scenario (`mix(seed, -1)`,
seeding protocol); it draws the item sounds' pitch and bone meal's plants on grass. The
entities items construct draw in their constructors before the join hook can seed them
(census §3: the heading jitter of arrows, snowballs, eggs and bobbers thrown by the player,
`ItemPainting`'s art), so the item fixtures dump only what those draws cannot reach (the
inventory, the player, `rng_state`), or arrange for every outcome to look the same (a lone
block that only 1x1 paintings fit). The `map_data` dump (`Dumps.mapData`) reads the private
`MapStorage.loadedDataMap` of `World.field_28108_z` by reflection instead of calling
`World.loadItemData`, which would read `data/map_<id>.dat` and add what it finds to the
storage; `colors` goes into the trace as its BLAKE3. With `SaveHandlerMP` (`"save_format":
"none"`) `getMapFile` is null, so no map file is ever read or written.

## `recipe_table` (WP-2.5)

`RecipeTable` dumps `CraftingManager.getInstance().getRecipeList()` after
`HarnessWorld.resetStatics(0)` (Block before Item, as for `item_table`; the singleton
is immutable once built, so a tick scenario that built it earlier in the same JVM,
through a player's `ContainerPlayer`, changes nothing). The private fields of
`ShapedRecipes` (`recipeWidth`, `recipeHeight`, `recipeItems`) and `ShapelessRecipes`
(`recipeItems`) are read by reflection. The `findMatchingRecipe` probes use an
`InventoryCrafting` whose event handler is an anonymous `Container` with an empty slot
list (its `onCraftMatrixChanged` → `updateCraftingResults` does nothing); the tick
harness's `craft` op builds its grid the same way.

## `stat_table` (WP-2.7)

`StatTable` dumps `StatList` and `AchievementList` by reflection. Two things would make
the JVM's own copies unusable: their stat names are translated from the jar's `lang/`
files (which no fixture may hold, PLAN_v4 D8), and they are initialised by whichever
scenario first touched `Block`. So `StatTable` builds a new `URLClassLoader` over the
oracle's class path (parent: the platform loader, so nothing is shared), initialises
`StringTranslate` there, replaces its private `translateTable` with `KeyTable` (every key
translates to itself; a key whose en_US text contains `%` translates to `key(%1$s)`, so
`translateToLocalFormatted("stat.mineBlock", "tile.stone.name")` gives
`stat.mineBlock(tile.stone.name)`), and only then initialises `Block`, which runs
vanilla's static-initialiser chain (Block.java:689 → Item → Item.java:380 → StatList →
AchievementList; Block.java:705). `Locale.US` is the default locale while `Block`
initialises (restored after), so `StatBase`'s `DecimalFormat` uses `.` whatever the
machine's locale. The `IStatType` formats are called on the fresh loader's instances.
The `"16 achievements"` line `AchievementList` prints goes to stdout as on every first
initialisation.
