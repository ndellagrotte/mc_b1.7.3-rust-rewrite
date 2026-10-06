# Nondeterminism census of the vanilla tree (WP-0.4)

Scope: `.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft` (all sources are under
`net/minecraft/src` except `net/minecraft/client/Minecraft.java`, `MinecraftApplet.java` and
`net/minecraft/isom/IsomPreviewApplet.java`, which contain no `Math.random()` and no
`new Random(`). File names below are relative to `net/minecraft/src`. Taken with

```sh
grep -rn  'Math.random()' --include='*.java'   # 49 lines
grep -rno 'Math.random()' --include='*.java'   # 58 occurrences
grep -rn  'new Random('   --include='*.java'   # 47 sites
```

How the harness handles each class of site is the **seeding protocol**
(`tools/oracle/README.md`, implemented in `java/harness/HarnessWorld.java`). PLAN_v4 §3.4 rule 3
and §4.2, ADR-007 §4.3.

## 1. `Math.random()`: 58 occurrences on 49 lines

All 58 are rewritten to `harness.HarnessMath.random()` (`rewrite-math-random.sh`), one
`java.util.Random` stream reseeded from the scenario seed at the start of every scenario.

Class: **S** = the value lands in simulation state that traces see; **D** = drawn on a
simulation code path, but the value only reaches the client (particle position) or a field
nothing reads; it still advances the stream, so the Rust `MathRandom` must draw there too;
**C** = client-only code (particles, animated textures, RenderGlobal), never run by the harness.

| Site | × | Class | Purpose |
|---|---|---|---|
| BlockFluid.java:290 | 2 | D | `triggerLavaMixEffects`: x/z of 8 `largesmoke` particles when lava meets water |
| EntityBubbleFX.java:12, :13, :14, :15 | 4 | C | bubble particle motion and lifetime |
| EntityExplodeFX.java:6, :7, :8 | 3 | C | explosion particle motion |
| EntityFX.java:24, :25, :26 | 3 | C | particle base motion |
| EntityFX.java:27 | 2 | C | particle speed scale |
| EntityFlameFX.java:16 | 1 | C | flame particle lifetime |
| EntityItem.java:9 | 1 | D | `hoverStart` field initialiser (every EntityItem); read only by RenderItem.java:20-21 |
| EntityItem.java:17 | 1 | S | dropped item's `rotationYaw` (`EntityItem(World, x, y, z, ItemStack)`) |
| EntityItem.java:18, :20 | 2 | S | dropped item's initial `motionX`, `motionZ` |
| EntityLavaFX.java:15 | 1 | C | lava particle lifetime |
| EntityLiving.java:38 | 1 | D | `field_9325_U` initialiser; never read |
| EntityLiving.java:63 | 1 | D | `field_9363_r`; never read |
| EntityLiving.java:65 | 1 | D | `field_9365_p`; never read |
| EntityLiving.java:66 | 1 | S | initial `rotationYaw` of every living entity (incl. players) |
| EntityLiving.java:341, :342 | 4 | S | `attackEntityFrom`: knockback direction jitter while attacker and target overlap (loop) |
| EntityLiving.java:348 | 1 | S | `attackedAtYaw` (0 or 180) when hit without an attacker; read by EntityPlayer.java:229-230 (death knockback) and EntityRenderer (hurt camera) |
| EntityPortalFX.java:22, :24 | 2 | C | portal particle lifetime and texture |
| EntityRainFX.java:7, :15, :36 | 3 | C | rain splash particle |
| EntityReddustFX.java:19, :20, :21, :22, :26 | 5 | C | redstone particle colour and lifetime |
| EntitySmokeFX.java:18, :22 | 2 | C | smoke particle colour and lifetime |
| EntitySnowShovelFX.java:18, :22 | 2 | C | snow particle colour and lifetime |
| EntityTNTPrimed.java:17 | 1 | S | primed TNT's initial horizontal kick direction |
| ItemBucket.java:84 | 3 | D | x/y/z of 8 smoke particles when water is emptied in the Nether |
| RenderGlobal.java:1372 | 1 | C | aux SFX 1003: door open vs close sound |
| TextureCompassFX.java:58 | 1 | C | compass needle outside the overworld |
| TextureFlamesFX.java:33 | 4 | C | fire texture animation |
| TextureLavaFX.java:44, TextureLavaFlowFX.java:48 | 2 | C | lava texture animation |
| TextureWatchFX.java:39 | 1 | C | clock outside the overworld |
| TextureWaterFX.java:46, TextureWaterFlowFX.java:47 | 2 | C | water texture animation |

Totals: S 10, D 9, C 39 (58).

Consequences:

- In the real client every site, including the texture animations that run every frame,
  draws from one JVM-wide stream, so the values at S sites are not reproducible in vanilla at
  all. The harness defines the stream as "S and D sites, in simulation order"; the client
  never constructs particles or textures in the oracle (the harness `IWorldAccess` drops
  `spawnParticle`). The Rust port draws its world-owned `MathRandom` at exactly the S and D
  sites (a D draw whose value is discarded still advances the stream) and uses its own RNG
  for C sites.
- Particles are entities in b1.7.3 (`EntityFX extends Entity`), so in the real client every
  particle consumes an `Entity.nextEntityID`. The oracle has no particles; entity ids are
  sequential over simulation entities only. Nothing in single-player gameplay depends on the
  numeric id, but the harness's entity seeding (§3) does.

## 2. `new Random(`: 47 sites

| Site | Seeded? | Harness |
|---|---|---|
| World.java:75, :116, :162 | clock (`new Random().nextInt()`) | `field_9437_g` (random-tick LCG state) overwritten with `new Random(seed).nextInt()` after construction |
| World.java:82, :123, :169 | clock | `world.rand.setSeed(seed)` after construction (`soundCounter`, drawn from it in the constructor at :90, is overwritten with the second draw of `new Random(seed)`) |
| World.java:1999 | clock | `randomDisplayUpdates` local, passed to `Block.randomDisplayTick`, which only reads the world and emits particles/sounds (BlockFire, BlockFluid, BlockFurnace, BlockPortal, BlockRedstone*, BlockTorch, BlockStairs). Not seeded; affects only the (untraced) IWorldAccess call counts |
| Entity.java:96 | clock | `Entity.rand`, see §3 |
| Item.java:6 | clock, static | `Item.itemRand` reseeded per scenario with `mix(seed, -1)`. Gameplay use: bone meal on grass (ItemDye.java:53-64); else sound pitch |
| BlockChest.java:6 | clock, singleton | `Block.chest.random` reseeded per scenario, `mix(seed, -2)` (drop scatter when broken, :161-176) |
| BlockFurnace.java:6 | clock, two singletons | `Block.stoneOvenIdle` / `stoneOvenActive` `furnaceRand`, `mix(seed, -3)` / `mix(seed, -4)` (drop scatter, :159-174) |
| BlockDispenser.java:6 | clock, singleton | `Block.dispenser.random`, `mix(seed, -5)` (dispensed item velocity, :187-202) |
| TileEntityDispenser.java:7 | clock, per tile entity | **gap**: which slot fires (:44). No harness hook at tile-entity creation; dispenser scenarios are not reproducible |
| Explosion.java:11 | clock, per explosion | **gap**: only used for `isFlaming` fire placement (:118), i.e. ghast fireballs; TNT and creeper explosions do not draw it |
| Teleporter.java:6 | clock | portal placement orientation (:88); reached only through `Minecraft.usePortal` (client), which the harness never calls |
| MapGenBase.java:7 | clock, then `setSeed` | reseeded from the world seed before every use (MapGenBase.java:11, :17): deterministic |
| WorldGenBigTree.java:7 | clock, then `setSeed` | `setSeed(rand.nextLong())` in `generate` before use (:334): deterministic |
| NoiseGeneratorPerlin.java:12, NoiseGenerator2.java:15 | clock | no-argument delegating constructors; never called (`grep 'new NoiseGeneratorPerlin()'` / `NoiseGenerator2()` finds nothing) |
| ChunkProviderGenerate.java:32, ChunkProviderHell.java:28, ChunkProviderSky.java:32 | world seed | deterministic (Hell's population quirk: tests/golden/README.md "terrain") |
| MapGenCaves.java:15, MapGenCavesHell.java:15 | `rand.nextLong()` | deterministic |
| WorldChunkManager.java:18, :19, :20 | world seed × constant | deterministic |
| Chunk.java:599, EmptyChunk.java:121 | world seed + chunk coordinates | deterministic (`getRandomWithSeed`: slime chunks, EntitySlime.java:136) |
| RenderGlobal.java:122 (10842), ModelGhast.java:15 (1660), TexturePortalFX.java:11 (100), RenderLightningBolt.java:17, :27 (`boltVertex`) | constant | client rendering |
| EffectRenderer.java:12, EntityRenderer.java:43, RenderItem.java:8, RenderPainting.java:7, GuiIngame.java:13, GuiMainMenu.java:13, GuiAchievements.java:164, SoundManager.java:19, SoundPool.java:12, Particle.java:6 (GUI title particles), NetClientHandler.java:22 | clock | client only; never constructed by the harness |
| GuiCreateWorld.java:75, CanvasIsomPreview.java:109 | clock | random world seed for a new world (GUI, isometric previewer); not reached |

## 3. `Entity.rand` and draws before an entity joins the world

`Entity.rand = new Random()` (Entity.java:50, :96) is unseeded. Harness rule: every time
vanilla calls `IWorldAccess.obtainEntitySkin(entity)` — from `World.entityJoinedWorld`
(World.java:896-918; b1.7.3's "spawn entity in world", hook at :915) and from
`World.addLoadedEntities` (World.java:2077-2083, entities of a chunk being loaded) — the harness
`IWorldAccess` sets `rand.setSeed(mix(seed, entityId))` with
`mix(s, k) = s ^ (k * 0x9E3779B97F4A7C15)` (wrapping `long`), then redraws the constructor's
non-persisted draws (below), then sets the seed again. Weather effects
(`World.addWeatherEffect`, World.java:891; lightning) never pass through the hook.

Draws on `this.rand` that run before the entity joins (and are therefore unseeded unless the
harness intervenes), found by scanning every constructor, `entityInit`, field initialiser and
the methods they call in the 54 `Entity` subclasses:

| Site | Drawn | Harness |
|---|---|---|
| EntityChicken.java:17 | `timeUntilNextEgg` | redrawn by the join hook (not in NBT, so vanilla redraws it on every chunk reload too) |
| EntitySquid.java:23 | `rotationVelocity` | redrawn by the join hook (not in NBT) |
| EntitySlime.java:11-14 | size, `slimeJumpDelay` | the `spawn` op replays both right after construction; **gap** for slimes spawned by SpawnerAnimals (size is persisted, so the hook cannot redraw it) |
| EntityMob.java:73, :84 (`getCanSpawnHere`) | sky/block light thresholds | **gap**: called by SpawnerAnimals (SpawnerAnimals.java:130, :228) before `entityJoinedWorld`. Hostile spawning is not reproducible, hence `difficulty` defaults to 0 |
| EntityGhast.java:152, EntitySlime.java:136 (`getCanSpawnHere`) | 1/20, 1/10 chance | **gap**, as above (monster spawn type) |
| EntityArrow.java:55-57 (`setArrowHeading`, from the `(World, EntityLiving)` constructor and BlockDispenser.java:107-108) | heading jitter (Gaussian) | **gap** for skeleton, bow and dispenser arrows; the `spawn` op uses `EntityArrow(World)` (no draw) |
| EntitySnowball.java:60-62, EntityEgg.java:60-62 (`setThrowableHeading`), EntityFish.java:90-92 | heading jitter | **gap** (thrown by players/dispensers) |
| EntityFireball.java:52-54 | acceleration jitter | **gap** (ghast) |
| EntityPainting.java:41 | art choice (`(World, x, y, z, dir)` constructor) | **gap** (ItemPainting) |
| EntityLightningBolt.java:14-15, :25-27 | bolt shape, lifetime, fire placement | **gap**; never seeded at all (weather effect). Needs a thunderstorm and `rand.nextInt(100000) == 0` per chunk tick |

`Math.random()` draws in constructors (EntityLiving.java:38, :63, :65, :66, EntityItem.java:9,
:17-20, EntityTNTPrimed.java:17) are covered by the HarnessMath rewrite.

Recommendation for the orchestrator (not done; needs a PLAN_v4 §4.2 change): a second scripted
rewrite of the single statement `this.rand = new Random();` at Entity.java:96 into
`this.rand = harness.HarnessMath.entityRandom(this.entityId);` (the id is assigned at :76,
before it) would close every gap in this section, including hostile spawning.

## 4. Random-tick LCG

`World.field_9437_g` (World.java:25) is the state of a 32-bit LCG,
`g = g * 3 + 1013904223` (`field_9436_h` = 1013904223, World.java:26, :76, final, but the
update code uses the literal), stepped in
`updateBlocksAndPlayCaveSounds` (World.java:1868-1965) once per cave-sound probe (when
`soundCounter == 0`), once per lightning try that passes `rand.nextInt(100000) == 0`, once per
snow/ice try that passes `rand.nextInt(16) == 0`, and 80 times per chunk for random block ticks.
`g >> 2` gives x (`& 15`), z (`>> 8 & 15`), y (`>> 16 & 127`). Its initial value is
clock-seeded (World.java:75); the harness sets it to `new Random(seed).nextInt()`. The trace
`rng_state.lcg` reports it.

## 5. Clock reads

| Site | Reaches traces? | Handling |
|---|---|---|
| World.java:80, :121, :167 `lockTimestamp` | no | only copied into `World(World, WorldProvider)` (:134); never read |
| SaveHandler.java:17 `now` | no | written to and compared with `session.lock` (:33-60); a single handler per save directory |
| WorldInfo.java:96 `LastPlayed` | yes (level.dat) | masked in the Tier S `level_dat` line after a `save` op |
| RegionFile.java:232 chunk timestamps | yes (region header) | traced only as "non-zero exactly where a chunk is present" |
| RegionFile.java:31 `lastModified` | no | stored, never read |
| Timer.java:17-24, Minecraft.java (frame timing) | no | the harness runs exactly one frame per tick |

## 6. Static and singleton state reset per scenario

`Entity.nextEntityID` (Entity.java:7) and `NextTickListEntry.nextTickEntryID`
(NextTickListEntry.java:4) are set to 0; `RegionFileCache` (static, SoftReference-held
RegionFiles, RegionFileCache.java:14) is closed and cleared; the singleton Randoms of §2 are
reseeded; `BlockSand.fallInstantly` (BlockSand.java:6, set and cleared by every
`ChunkProvider*.populate`), `BlockFurnace.keepFurnaceInventory` (BlockFurnace.java:8),
`BlockRedstoneTorch.torchUpdates` (BlockRedstoneTorch.java:9, burnout history keyed by world
time), `TileEntityPiston.field_31018_m` (TileEntityPiston.java:15) and
`World.lightingUpdatesScheduled` (World.java:52) go back to their initial values. Other mutable
statics are caches or pools with no observable effect (`AxisAlignedBB`/`Vec3D` pools, cleared
at every frame start as Minecraft.java:519-520 does; `SpawnerAnimals.eligibleChunksForSpawning`,
cleared on use; `BiomeGenBase.biomeLookupTable`, a pure function) or client-only.

## 7. Hash-ordered iteration met on these paths

Already covered by Tier A `java_hash_order` (ADR-007 §6): `World.positionsToUpdate`
(random-tick chunk order, World.java:1868-1885), `SpawnerAnimals.eligibleChunksForSpawning`
(SpawnerAnimals.java:9, 17×17 `ChunkCoordIntPair` per player; same key class and pattern),
`Chunk.chunkTileEntityMap` (tile-entity tick order via `Chunk.onChunkLoad` →
`World.addTileEntity`, Chunk.java:459-467, and the `TileEntities` list order in saved chunks,
ChunkLoader.java:136-148). New ones found in this WP: the last two.
