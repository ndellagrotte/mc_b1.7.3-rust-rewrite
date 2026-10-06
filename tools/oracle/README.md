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
| `check-sensitivity.sh` | One run with every seed XOR 1; every seeded fixture must change, every unseeded one must not; only `sine_table`, `nbt_write`, `nbt_read`, `java_hash_order`, `world_read` may lack a seed (§4.4 item 2) |
| `env.sh` | Shared classpath (`minecraft.jar` last), the JVM math flags (§3.4 rule 5) and the `oracle_run` helper, sourced by the scripts |
| `census.md` | Every `Math.random()` and `new Random(` site in the vanilla tree, entity-constructor draws, clock reads, static state: what the harness does about each (WP-0.4) |
| `java/harness/` | `Main` (driver, tier dispatch), `TierA` (`java_random`, `sine_table`, `math_sweep`, `terrain`), `Nbt` (`nbt_write`, `nbt_read`), `HashOrder` (`java_hash_order`), `TickScenario` (kind `tick`: ops, tick loop), `Dumps` (Tier B/C dumps), `TierS` (`save` op, `world_read`), `HarnessWorld` (world construction, seeding protocol, static resets), `HarnessWorldAccess` (the `IWorldAccess` with the entity seeding hook), `HarnessPlayer`, `EmptyChunkProvider` (`"world": "empty"`), `Rng` (reads `java.util.Random` state), `Reflect`, `Num`, `Blake3`, `Json`, `Hex`, `Trace`, `HarnessMath` |

Tier A kinds (specified in `tests/golden/README.md`): `java_random`, `sine_table`,
`math_sweep` (optional `"strict": true` for `StrictMath`), `terrain` (WP-0.2);
`nbt_write`, `nbt_read`, `java_hash_order` (WP-0.3). Tier B/C/S kinds (WP-0.4):
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
Hence the `difficulty` default of 0 and the warning for a player scenario that raises it.
Closing these needs a second scripted rewrite (Entity.java:96), proposed in census §3.

`rng_state` dumps read `java.util.Random` state through its serial form (`Rng`, no
`--add-opens`; self-tested at start-up against `new Random(42)`).

### `HarnessPlayer`

`EntityPlayerSP` needs a `Minecraft` (constructor, `onLivingUpdate` reads
`mc.statFileWriter`, `mc.currentScreen`, `movementInput`), so `harness.HarnessPlayer`
extends `EntityPlayer` directly: the one abstract method, `func_6420_o`, is a no-op as in
EntityPlayerSP.java:169, and `updatePlayerActionState` zeroes strafe/forward/jump after
`EntityPlayer`'s swing update, as EntityPlayerSP.java:27-32 does with no keys pressed. Not
modelled: portals, `pushOutOfBlocks`, sneaking, statistics/achievements (EntityPlayer's
`addStat` is a no-op), the pickup animation. Vanilla paths that only run with a player in
`World.playerEntities`: random block ticks, snow/ice, lightning and cave sounds
(World.updateBlocksAndPlayCaveSounds, chunks within 9 of a player), mob spawning
(SpawnerAnimals.performSpawning, chunks within 8), sleeping, despawning and AI target
search (`getClosestPlayer`), item pickup, and the client's `World.randomDisplayUpdates`
(6000 `world.rand` draws per tick), which the tick loop runs for it. A player therefore
also loads (generates) the 19 × 19 chunks around it on the first tick.

`use_item` and `attack` drive the vanilla controller code without a `Minecraft`:
`new PlayerControllerSP(null)` (its constructor only stores `mc`; `sendPlaceBlock` and
`sendUseItem` never touch it) and `EntityPlayer.attackTargetEntityWithCurrentItem`. The
ray trace that picks the block or entity (`Minecraft.objectMouseOver`) is replaced by the
op's coordinates; left-click block breaking (`PlayerControllerSP.clickBlock`, which reads
`mc.theWorld`/`mc.thePlayer`) is not an op yet.

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
- `world_read` copies the committed save to a temp directory before opening it.
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
