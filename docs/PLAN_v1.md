# PLAN_v1 — Rust/Bevy rewrite of Beta 1.7.3 (master plan)

**Status:** v1, 2026-10-05. Supersedes `RUST_REWRITE_BLUEPRINT.md` and
`docs/ADR-007-golden-harness.md` wherever they conflict. Those two files stay in
the tree as input material; agents follow this document.

**Goal:** a Rust + Bevy game that behaves like the Java game in this repository
in singleplayer, proven by fixtures generated from the Java code.

Contents: 1 Audit of prior research · 2 Decisions · 3 Architecture ·
4 Golden oracle · 5 Phases and work packages · 6 Agent protocol · 7 Risks ·
8 Open decisions for the owner

---

## 1. Audit of the prior research

Every claim below was checked against the source in this checkout. The harness
claims were tested by compiling the Java tree and running a headless probe
(recipe in §4.1).

### 1.1 Refuted — these change the plan

| # | Blueprint / ADR-007 claim | What the source shows | Consequence |
|---|---|---|---|
| R1 | Singleplayer runs on `WorldClient extends World`; the harness should build a `WorldClient` | Singleplayer constructs plain `World` (`Minecraft.java:952, 989, 1000`). `WorldClient` is built only by `NetClientHandler` (`:59, :474`) and overrides `updateBlocksAndPlayCaveSounds()` to empty, `scheduleBlockUpdate()` to empty and `TickUpdates()` to `return false` (`WorldClient.java:73-81`) | A `WorldClient` harness would record a world with no block ticks at all. Oracle uses `World`. `WorldClient` is multiplayer-only and deferred |
| R2 | Gameplay logic has no GL/JSO reachability, so it runs on a plain JVM with `src/lwjgl/java` on the classpath | `Block.<clinit>` → `Item` → `StatList` → `StatCollector` → `StringTranslate` → `GL11.getResourceAsStream` (`StringTranslate.java:14`). With the LWJGL backend this dies in `GL11_1.<clinit>` with `ClassNotFoundException: org.lwjgl.system.FunctionProvider` on the first chunk request | Oracle needs a headless platform stub. A one-method stub `GL11` placed first on the classpath is enough for worldgen and ticking (tested) |
| R3 | RNG is `java.util.Random`; copy MinecraftOSS `LegacyRandom` 1:1 | The game uses its own `net.minecraft.src.Random` (no file imports `java.util.Random`). Its `setSeed` sets `haveNextNextGaussian = true` (`Random.java:25`), so the first `nextGaussian()` after any seeding returns `0.0`. Measured, seed 42: port `0.0`, JDK `0.27686040089698144`. `nextInt` matches the JDK. `LegacyRandom` also exposes no `next_gaussian` | Write `JavaRandom` from `Random.java`, quirk included (decision D2). 13 gameplay/render files call `nextGaussian` |
| R4 | `FixedUpdate` order: input → random block ticks → scheduled ticks → tile entities → entities → player → time | Actual order in `Minecraft.runTick` (`:887-927`): `updateEntities()` (weather effects → entity list → tile entities) and then `World.tick()` (weather → sleep check → `SpawnerAnimals` → chunk unload → skylight → time and autosave → scheduled ticks → random ticks) → `randomDisplayUpdates` → particles. `runGameLoop` and `updateEntitiesAndBlocks` do not exist | Tick order is §3.3, transcribed from source |
| R5 | `bevy_ecs` iteration order mirrors the Java entity list because we control insertion | Bevy query iteration follows archetype/table storage; it is not insertion order across archetypes and despawn swap-removes rows | With R6, this is why the simulation is not built on ECS (decision D1) |
| R6 | ADR-001 Rule 1: a Java override runs *instead of* the base system; wolf is the worked example | `EntityWolf.updatePlayerActionState`, `onLivingUpdate` and `onUpdate` all begin with `super.…()` (`EntityWolf.java:86, 115, 142`). Most overrides in the tree extend rather than replace | The `Without<Wolf>` exclusion pattern would drop base behaviour. Superseded by D1 |
| R7 | Wolf example constants: body 0.6×0.8, dataWatcher bits 0x02/0x04/0x08, collar at byte 20 | `setSize(0.8F, 0.8F)`; flags on byte 16 are sitting `1`, angry `2`, tamed `4`; index 17 is owner string, 18 is health; there is no collar (`EntityWolf.java:17-27, 412-452`) | The "pattern to copy" was not read from source. Discard it |
| R8 | `field_9365_p` is `moveStrafing` (`EntityLiving.java:8`); `0.98` constant at `EntityLiving.java:142` | `field_9365_p` (line 7) is a separate field initialised to `Math.random() * 12398.0F`; `moveStrafing` is already named at line 50. The `0.98` sites are lines 570, 676, 677 | Example mapping is invented. `docs/mappings.md` entries must come from reading usage |
| R9 | `src/lwjgl/java` and `net/PeytonPlayz585` are TeaVM shims | `src/teavm/java` (33 files, 12.9k LOC) is the TeaVM/WebGL backend and the one `build.gradle` compiles. `src/lwjgl/java` (60 files, 23.8k LOC) is a desktop LWJGL backend plus vendored sound libraries, wired by `.classpath` and `debugRuntime.launch`. Still nothing to port from either | Oracle compiles against `src/lwjgl/java`; never ship or port it |
| R10 | Round-trip real `level.dat`/chunk files from `jars/` | No `.dat`, `.mcr` or chunk file exists anywhere in the repo. `jars/` holds only jars and natives | Save fixtures must be generated by the oracle |
| R11 | Save format is vanilla (`.dat` chunk codec, `level.dat`) | Eaglercraft-specific: `level.dat` is written **uncompressed** (`EaglerSaveHandler.java:50-60`), chunks are gzip NBT in flat files named by a custom 10-character base-36 scheme under `c0/` (overworld) and `c1/` (nether) (`ChunkLoader.java:17-33`), on a virtual filesystem | Not compatible with vanilla saves. See D5 |
| R12 | ~59 non-FX entity classes; `EntityFX` + 20 subclasses; slime under `EntityMob` | 38 non-FX entity classes (58 `Entity*.java` minus 17 FX, `EntityRenderer`, `EntityList`, `EntitySorter`); `EntityFX` + 16 subclasses; `EntitySlime extends EntityLiving implements IMob`; `EntityPigZombie extends EntityZombie`; `EntityClientPlayerMP extends EntityPlayerSP` | Entity work is about a third smaller than budgeted |
| R13 | `EntityList.createEntityByName` for harness spawning | Method does not exist. `createEntityInWorld` routes through `SpawnerAnimals.newInstance`, which reads `Minecraft.getMinecraft().theWorld` (`SpawnerAnimals.java:329` and siblings) and throws headless | Oracle constructs entities directly (`new EntityPig(world)`) |
| R14 | Only ordering hazard is `Chunk.chunkTileEntityMap`; tile-entity tick order depends on it | Tile entities tick from `loadedTileEntityList`, an `ArrayList`. The map's order matters only at chunk load/save (`Chunk.java:455`, `ChunkLoader.java:123`). The hazard the ADR missed: `World.positionsToUpdate` is a `HashSet<ChunkCoordIntPair>` iterated at `World.java:1884` to pick the chunk order for random ticks, which share `World.rand` | Rust must reproduce Java `HashSet` iteration order for that set (§3.4) |
| R15 | Reflective `setSeed` on `Random` fields makes the Java side deterministic | 14 gameplay call sites use `Math.random()` directly (`EntityLiving.java:37,62,64,65,378,379,385`, `EntityItem.java:9,17,18,20`, `EntityTNTPrimed.java:17`, plus two particle sites), and `new Random()` itself seeds from `Math.random()` | Oracle is built from a mechanically patched source copy (§4.2) |

### 1.2 Verified — carry forward

- 632 files / 60,555 LOC in `net/minecraft/src`; plus `Minecraft.java` (1,247).
- No server classes exist. Singleplayer is one in-process `World`; Phase 1 needs no protocol code. 59 `Packet*.java` confirmed; networking totals 69 files / 4.5k LOC, all deferrable.
- Texture-pack classes exist (4 `TexturePack*`, 2 GUI) and are safely deferrable.
- Registry sizes: 96 blocks, 106 items (blueprint said ~90 / ~100), 31 `GuiScreen`/`GuiContainer` subclasses (~30).
- Random ticks: 80 picks per chunk per tick (`World.java:1944`), 19×19 chunks around each player. The "216" in the blueprint matches nothing.
- `Chunk.func_997_a` seed formula (`Chunk.java:593`) and the unseeded `World.rand` / `field_9437_g` / `Entity.rand` sites are as ADR-007 lists them.
- `World.worldAccesses` is a clean no-op seam for the oracle.
- Golden-fixture design in ADR-007 §5 (one scenario file, floats as IEEE-754 bit patterns, digests over canonical byte arrays, fixtures checked in) is sound and adopted.
- MinecraftOSS facts: five crates, README states commit `4013a68`; LOC figures match (core 8.0k, generator 31.2k, world 15.1k, player 16.1k + 5.1k bins, entities 35.2k + 20.2k bins); engine crates have no Bevy dependency; workspace uses Bevy 0.19; lints allow `excessive_precision` and `approx_constant`; its wolf is 0.6×0.85; the catalog JSON artifacts it loads are absent; `generator/` is inapplicable to beta. The mashup `NOTICE` describes it as the fork author's own project but there is no licence file for it, so its licence is unverified (D4).
- "No runnable desktop client" holds in practice, for a different reason than stated: `jars/lwjgl.jar` is LWJGL 2 (2013) while `lwjgl-jemalloc.jar` needs LWJGL 3's `org.lwjgl.system`, which is missing.

### 1.3 New findings the research missed

- **The whole Java tree compiles with plain `javac` on the installed JDK 25** (`src/main/java` + `src/lwjgl/java` against `jars/*.jar`, 813 classes, no errors). Gradle is not needed for the oracle.
- **Headless worldgen works and is reproducible.** Seed 12345, three chunks, two independent `World` instances: identical CRCs both times.
- **This port deviates from vanilla b1.7.3 in gameplay-visible ways.** Besides R3: `MathHelper` uses a 4,096-entry sine table with factor `651.8986F` (`MathHelper.java:4-11`); vanilla, as far as I recall, uses 65,536 entries. That feeds every `moveFlying` and look-vector calculation. There are likely more. The spec question is D2.
- **Entities do not tick unless chunks exist within ±32 blocks** (`World.java:1286`), and random ticks happen only around entries in `playerEntities`. In the probe a pig spawned in mid-air did not move in 100 ticks. Oracle scenarios need preloaded chunks and a headless player.
- **Sky dimension is unreachable in this build.** `WorldProviderSky` exists, but `usePortal` only switches between −1 and 0 and the save handler only has `c0`/`c1`.
- **Environment gaps:** no `cargo`/`rustc` on this machine, the project is not a git repository, and the Gradle wrapper pins 8.5, which predates JDK 25.
- **Subsystem sizes** (drives §5): rendering 69 files / 11.2k LOC (`RenderBlocks.java` alone 3,376); blocks 83 / 8.1k; entities 38 classes / 8.3k; world core 26 / 5.6k; GUI 52 / 5.0k; worldgen 48 / 4.9k; items, crafting and inventory 69 / 3.6k; stats and achievements 61 / 2.7k (of which the bundled JSON parser is 47 / 1.7k and is replaced by `serde_json`); NBT and save 20 / 1.2k; particles 18 / 1.1k.

---

## 2. Decisions

Each has a default that agents follow. Items marked **owner** are listed again in §8.

**D1 — Simulation is a plain-Rust core; Bevy is the shell. (owner)**
`World`, chunks, blocks and entities live in ordinary Rust structs in crates with
no Bevy dependency. Bevy owns the window, input, rendering, UI, audio and assets,
and drives the simulation from one system. This replaces ADR-001 rules 1, 2, 3
and 7. Reasons: R5 and R6; Java call chains re-enter freely (entity → `setBlock`
→ neighbour notification → spawns an entity) inside one stack frame, which ECS
systems cannot express without exclusive world access anyway; entity-list order
is gameplay-visible; and the only comparable prior art (MinecraftOSS) made the
same choice. ADR-001 rules 5 (one closed `EntityKind` enum with match tables)
and 6 (models as data tables) are kept.

**D2 — The spec is this repository's Java, quirks included. (owner)**
The oracle can only execute this code, so fixtures encode this port's behaviour.
Agents transcribe port deviations (R3, the sine table) and record each one in
`docs/DEVIATIONS.md` with the vanilla behaviour where known. Switching any of
them to vanilla is a later, per-item owner decision, behind a named constant.

**D3 — Phase 1 is singleplayer on native desktop.** Multiplayer (`Packet*`,
`Net*`, `WorldClient`, MP player classes), texture-pack selection, the sky
dimension, EPK world import/export and a wasm build are out of scope until
Phase 1 exits. Agents that meet such code cite it, skip it and tag the task
`deferred`.

**D4 — MinecraftOSS is reference-only.** No code is copied until the owner
confirms its licence. The cost is small: the reusable parts were the RNG (which
must be rewritten anyway, R3) and an NBT codec for a format with more tag types
than beta has. Its trace-binary layout and lint settings are ideas, not code,
and are adopted. The rest of `2010-rust-rewrite-mashup-main/` is an unrelated
game and stays outside the new workspace.

**D5 — Saves use this port's NBT schema on the real filesystem. (owner)**
Same directory layout and file contents as R11, so save/load can be checked
against oracle-written bytes after decompression. Importing vanilla or browser
saves is out of scope.

**D6 — Toolchain.** Stable Rust pinned in `rust-toolchain.toml`. Bevy pinned to
the newest stable release at kickoff (the reference workspace runs 0.19) and not
upgraded during Phase 1–4. Oracle runs on the installed JDK via a shell script.

**D7 — Rendering parity is visual, not structural.** The Java renderer is
fixed-function immediate-mode GL. Game-data parts are transcribed (which faces,
which texture index, which colour and light value, model boxes); the drawing
machinery is written for Bevy.

**D8 — Assets are read from `resources/` at runtime** and never embedded in the
binary or committed to a public repository; they and the decompiled Java are
Mojang's.

---

## 3. Architecture

### 3.1 Workspace

New Cargo workspace in `rust/` at the repository root.

| Crate | Depends on | Owns | Java source |
|---|---|---|---|
| `mc-core` | — | `JavaRandom`, `MathHelper` (sine table loaded from a fixture-checked constant), AABB, `Vec3D`, ray-trace result, `JavaHashSet` order emulation | `Random`, `MathHelper`, `AxisAlignedBB`, `Vec3D`, `MovingObjectPosition` |
| `mc-nbt` | — | NBT tags, big-endian codec, gzip wrapper | `NBT*`, `CompressedStreamTools` |
| `mc-sim` | core, nbt | Everything that ticks: block and item registries and behaviour, world, chunks, lighting, worldgen, tile entities, entities, AI, inventory, crafting, save/load, stats | all remaining non-render, non-GUI, non-net classes |
| `mc-golden` | sim | Scenario runner and trace writer; dev-dependency only | — |
| `mc-client` | sim, bevy | Bevy plugins: tick driver, mesher, entity and sky rendering, particles, GUI, audio, input, settings | `Render*`, `Model*`, `Gui*`, `EntityRenderer`, `RenderGlobal`, `EffectRenderer`, `SoundManager`, `GameSettings` |
| `mc-app` | client | Binary; thin | `Minecraft.java` |

`mc-sim` is one crate on purpose. Blocks call the world, the world calls blocks,
entities call both, and items place blocks and spawn entities; the blueprint's
seven-crate split would need cycles or trait indirection at every one of those
calls. Module boundaries inside `mc-sim` (`block/`, `item/`, `world/`, `gen/`,
`entity/`, `inventory/`, `save/`) give agents the same ownership units without
that cost. If compile times hurt, `gen/` is the clean piece to split out later.

### 3.2 Simulation shape

- `World` owns chunks, the entity list (`Vec` in Java list order, with stable
  ids), tile-entity list, scheduled-tick set, and its RNGs. Fields keep Java
  names where the Java has real names.
- **Blocks:** a static table indexed by block id holding the data fields
  (material, hardness, light, texture index, bounds) plus a `BlockKind` enum;
  behaviour is free functions `fn update_tick(world: &mut World, x, y, z, rng)`
  dispatched by `match` on kind. No `Box<dyn Block>`.
- **Entities:** `struct Entity { base: EntityBase, kind: EntityKindData }` where
  `EntityKindData` is an enum with one variant per concrete class, and layered
  data structs (`LivingData`, `CreatureData`, …) are embedded by the variants
  that inherit them. A Java method becomes a function per class level;
  `super.onUpdate()` becomes an explicit call to the parent level's function.
  This keeps the Java call order line-for-line.
- **Borrowing:** entity ticks take the entity out of its slot
  (`Option::take`), run with `&mut World`, and put it back, so entity code can
  query and mutate the world and other entities as Java does. This pattern is
  defined once in WP-0.5 and used everywhere.
- **Side effects for the client** (sounds, particles, block-changed
  notifications) go through an `IWorldAccess`-equivalent event queue that
  `mc-client` drains each frame. The simulation never calls into Bevy.

### 3.3 Tick order (from `Minecraft.runTick` and `World.tick`)

```
per game tick, when not paused:
  GUI / input handling, player controller update
  entityRenderer.updateRenderer, renderGlobal.updateClouds      (client state)
  world.updateEntities():
      weather effects → loadedEntityList in order (removals in place)
      → loadedTileEntityList → pending tile-entity adds
  world.tick():
      updateWeather → sleep check → SpawnerAnimals.performSpawning
      → chunk unload → skylight recalculation → worldTime + 1, autosave every 40
      → TickUpdates (scheduled) → updateBlocksAndPlayCaveSounds (random ticks)
  world.randomDisplayUpdates(player position)                    (client particles)
  effectRenderer.updateEffects()
```

The tick driver is a port of `Timer.java` (20 TPS, at most 10 ticks per frame,
partial tick for interpolation) run from a single Bevy system in `Update`, not
Bevy's `FixedUpdate`, so catch-up and interpolation behave as in Java.

### 3.4 Determinism rules

1. Java `double` → `f64`, `float` → `f32`, `int` → `i32` with wrapping
   arithmetic, `long` → `i64` wrapping. Casts follow Java semantics; Rust `as`
   already saturates float→int the way Java does.
2. Constants are transcribed exactly, with a `// src: File.java:line` comment.
3. All randomness goes through `JavaRandom`. Each Java `Math.random()` site maps
   to a named draw from one `MathRandom` stream owned by the world (§4.2).
4. Collections: `ArrayList` → `Vec`, `TreeSet` → `BTreeSet` with the Java
   comparator, `HashMap` used only for lookup → any map. Where Java *iterates* a
   hash collection in gameplay (`World.positionsToUpdate`,
   `Chunk.chunkTileEntityMap`), use `mc_core::JavaHashSet`/`JavaHashMap`, which
   reproduce bucket order from the Java `hashCode`. Any new case found is
   reported, not silently replaced.
5. `Math.sin/cos/atan2/pow/exp/log` (87 call sites) go through `mc_core::math`.
   WP-0.3 decides the implementation from a bit-comparison sweep against the
   oracle JVM.
6. Bugs in the Java are transcribed and flagged, never fixed.

### 3.5 Client notes

- **Terrain:** 16×16×16 sections as in `WorldRenderer`, meshed on Bevy's task
  pool from chunk snapshots, drawn with a custom unlit material (vertex colour ×
  brightness table, alpha test, nearest filtering, distance fog) in two passes.
- **GUI:** Java screens draw in scaled pixel coordinates with immediate-mode
  calls (`drawTexturedModalRect`, `drawString`). Default approach is a small 2D
  quad batcher with that same API, so `Gui*` classes transcribe nearly line for
  line. WP-4.6 begins with a short spike confirming this against `bevy_ui`.
- **Animated textures** (`TextureFX` subclasses: water, lava, fire, portal,
  compass, clock) are procedural and are transcribed, writing into the atlas.
- **Audio:** `bevy_audio` playing the `.ogg` files in `resources/`; sound names
  and volume/pitch arithmetic transcribed from the call sites.

---

## 4. Golden oracle

### 4.1 Verified build recipe

```
javac -nowarn -proc:none -encoding UTF-8 -cp "jars/*" -d build/oracle \
      $(find src/main/java src/lwjgl/java -name '*.java')
# stub: package net.PeytonPlayz585.opengl; class GL11 with only
#   public static InputStream getResourceAsStream(String p)  → reads resources/<p>
java -cp build/oracle-stub:build/oracle harness.Main …
```

The stub directory precedes the real classes, so `GL11` resolves to the stub and
no LWJGL class is ever loaded. With this, `new World(new SaveHandlerMP(),
"name", seed)`, `ChunkProviderGenerate.provideChunk`, `updateEntities()` and
`tick()` all run headless.

### 4.2 Oracle rules (replacing ADR-007 §2, §4.2, §4.3)

- Worlds are `World`, never `WorldClient`.
- The oracle compiles from a **generated copy** of `src/main/java` produced by a
  checked-in script that rewrites `Math.random()` to `HarnessMath.random()` (a
  seeded stream) and nothing else. No hand edits to game code; the script's diff
  is reviewed once.
- After constructing a world, the harness seeds `World.rand` and
  `field_9437_g`; every entity's `rand` is seeded from `(scenario seed, entityId)`
  on construction. `Entity.nextEntityID` is reset per scenario.
- Entities are constructed directly. Scenarios that need ticking preload chunks
  within 32 blocks; scenarios that need random ticks or mob spawning add a
  `HarnessPlayer extends EntityPlayer`. Mob-spawning scenarios also need the
  `Minecraft` singleton stood up or `SpawnerAnimals.newInstance` avoided; that
  is investigated in WP-0.4 and may reduce spawning to a narrower fixture.
- Save fixtures are produced by the oracle with a save handler writing to a
  temp directory through the desktop `fileutils.File`.

### 4.3 Tiers

| Tier | Covers | Compared by |
|---|---|---|
| A | RNG sequences, sine table, math sweep, NBT bytes, terrain generation and population per chunk, biome arrays | exact bytes / digest |
| B | Lighting, scheduled and random ticks, fluids, fire, plants, redstone, pistons, rails, tile entities, explosions, save round-trip | per-tick digests and block dumps, exact |
| C | Entity physics, collisions, AI, projectiles, vehicles, item entities, damage, inventory and crafting results | per-tick field traces, floats as bit patterns, exact |
| D | Rendering, GUI, audio, feel | side-by-side against the prebuilt `web/` build; screenshots reviewed by a human, with automation optional and late |

Traces use ADR-007 §5's formats. Fixtures are checked in under `rust/tests/golden/`;
ordinary CI runs only the Rust side.

### 4.4 Harness acceptance

1. Tier A regenerates byte-identically across three runs.
2. Sensitivity check: perturbing one RNG seed bit, or one block in a scenario,
   fails the matching fixture.
3. A failing comparison prints the first differing tick and field with both
   values decoded.

---

## 5. Phases and work packages

LOC figures are Java lines to transcribe. "∥" marks packages that can run
concurrently once their dependencies are merged. Packages above roughly 1.5k LOC
are split by the listed sub-units, one agent task each.

### Phase 0 — Foundations (serial; nothing else starts until 0.5 is merged)

| WP | Deliverable |
|---|---|
| 0.1 | `git init`, `.gitignore` (build outputs, `web/`, oracle output), install rustup, create `rust/` workspace with lint config, `scripts/oracle-build.sh` implementing §4.1, CI running `cargo test` and `clippy` |
| 0.2 | Oracle harness Tier A plus `mc-golden` runner; fixtures for RNG (including the gaussian quirk), sine table, `Math.*` sweep, three terrain seeds |
| 0.3 | `mc-core` and `mc-nbt` complete and green against 0.2; decision recorded for §3.4 rule 5 |
| 0.4 | Oracle Tier B/C scaffolding: `Math.random` rewrite script, seeding protocol, `HarnessPlayer`, chunk preload, scenario ops (`set_block`, `spawn`, `set_motion`, `use_item`, …), spawning investigation |
| 0.5 | `mc-sim` skeleton, owner-reviewed: `World`, `Chunk`, block table and dispatch, entity struct layout and the take/put-back pattern, event queue, module layout, one block and one entity implemented end to end as the pattern to copy; `docs/mappings.md` and `docs/DEVIATIONS.md` started |

**Gate 0:** harness acceptance (§4.4) passes; owner approves the 0.5 skeleton.

### Phase 1 — World without entities

| WP | Java source | LOC | Depends | Gate |
|---|---|---|---|---|
| 1.1 Block data table | `Block.java` static init, `Material*`, `StepSound*` | ~0.9k | 0.5 | oracle dump of every block's fields |
| 1.2 Item data table ∥ | `Item.java` static init, tool/armour enums | ~0.6k | 0.5 | oracle dump of every item's fields |
| 1.3 Chunk storage and lighting | `Chunk`, `NibbleArray`, `MetadataChunkBlock`, light paths in `World` | ~1.5k | 0.5 | B: light arrays after scripted edits |
| 1.4 Noise and terrain shape ∥ | `NoiseGenerator*`, `WorldChunkManager*`, `BiomeGen*`, `ChunkProviderGenerate/Hell.provideChunk`, `MapGenCaves*` | ~2.3k | 0.3 | A: unpopulated chunk digests |
| 1.5 World block access and ticks | remainder of `World` block get/set, neighbour notification, scheduled ticks, random ticks, weather, time | ~1.8k | 1.1, 1.3 | B: tick traces |
| 1.6 Block behaviour ∥ (seven tasks) | fluids · fire, plants, crops, leaves, snow/ice · redstone wire, torches, repeaters, buttons, levers, plates · pistons · rails (`RailLogic`) · containers, doors, beds, signs and other interactables · simple remainder | ~7.2k | 1.5 | B: one scenario set per group |
| 1.7 Population features ∥ | `WorldGen*`, `populate` in both providers | ~2.4k | 1.5, 1.6 simple blocks | A: populated chunk digests |
| 1.8 Tile entities and explosions | `TileEntity*` (7 logic kinds), `Explosion` | ~1.0k | 1.5 | B |
| 1.9 Save and load | `ChunkLoader`, `EaglerSave*`, `WorldInfo`, `ChunkProvider*` caching | ~1.0k | 1.3, 1.8 | round-trip against oracle-written saves |

**Gate 1:** all Tier A fixtures and the Tier B set green.

### Phase 2 — Entities and player

| WP | Java source | LOC | Depends | Gate |
|---|---|---|---|---|
| 2.1 Base entity physics | `Entity.java` (`moveEntity`, water/lava/fire, riding) | ~1.1k | 1.5 | C: falling, stepping, collision traces |
| 2.2 Living base and pathfinding | `EntityLiving`, `EntityCreature`, `Path*`, `Pathfinder` | ~1.6k | 2.1 | C |
| 2.3 Mobs ∥ (three tasks) | animals and wolf · monsters, slime, ghast · squid, spawn rules (`SpawnerAnimals`) | ~2.6k | 2.2 | C per mob |
| 2.4 Non-living entities ∥ (two tasks) | item, falling sand, TNT, painting, projectiles, fishing hook, lightning · minecart and boat | ~2.8k | 2.1 | C |
| 2.5 Player, inventory, crafting | `EntityPlayer`, `InventoryPlayer`, `Container*`, `Slot*`, `CraftingManager`, `Recipes*`, `FurnaceRecipes`, `PlayerControllerSP` | ~3.5k | 2.2, 1.2 | C: scripted player; all 151 recipes against oracle dump |
| 2.6 Item behaviour ∥ | `Item*` use/attack logic, `ItemMap`, `MapData` | ~2.4k | 2.5 | C |
| 2.7 Stats and achievements ∥ | `Stat*`, `Achievement*` | ~1.0k | 2.5 | list dump |

**Gate 2:** a headless scripted session (generate a world, walk, dig, place,
craft, fight, sleep, save, reload) matches the oracle trace.

### Phase 3 — Bevy client (3.1–3.3 may start after Gate 0 against the skeleton)

| WP | Java source | LOC | Depends |
|---|---|---|---|
| 3.1 App shell | `Minecraft.java` loop, `Timer`, input mapping, `GameSettings`, camera from `EntityRenderer` | ~2.0k | 0.5 |
| 3.2 Texture atlas and animated textures | `RenderEngine`, `TextureFX` family | ~1.2k | 3.1 |
| 3.3 Terrain mesher (four tasks) | `RenderBlocks` by render type: cubes and smooth lighting · fluids · plants, torches, fire, crops · redstone, rails, doors, stairs, pistons, bed and the rest; plus `WorldRenderer` sectioning | ~3.9k | 3.2, 1.3 |
| 3.4 Sky, clouds, weather, fog, selection box | `RenderGlobal`, parts of `EntityRenderer` | ~1.7k | 3.1 |
| 3.5 Entity models and renderers ∥ (two tasks) | `Model*` (20), `Render*` (34), tile-entity renderers | ~3.2k | 3.2, Phase 2 types |
| 3.6 GUI ∥ (three tasks) | font and widgets · in-game HUD, chat, inventory and container screens · menus, world select and create, options, achievements | ~5.0k | 3.1, 2.5 |
| 3.7 Particles, hand and item rendering | `EntityFX` family, `EffectRenderer`, `ItemRenderer` | ~1.6k | 3.2 |
| 3.8 Audio | `SoundManager`, sound call sites surfaced as events | ~0.3k | 3.1 |

**Gate 3:** the game is playable from the title screen through world creation,
play, save and reload.

### Phase 4 — Parity and hardening

Side-by-side checklist against the `web/` build (every block rendered, every
screen, every mob), long-run soak comparing periodic world digests with the
oracle, performance pass (chunk meshing and lighting budgets), packaging.
Deviations found are fixed or entered in `docs/DEVIATIONS.md`.

### Phase 5 — Gated on owner decision

Multiplayer (4.5k LOC plus a protocol test server), texture packs, sky
dimension, wasm build, vanilla-behaviour switches from the deviation ledger.

---

## 6. Agent protocol

**Roles.** An *orchestrator* session holds this plan, issues work packages,
tracks the dependency table and merges. *Transcriber* agents each take one task
in its own git worktree and branch. A *reviewer* agent, given only the Java
source and the diff (not the transcriber's reasoning), checks the diff against
the Java. The owner approves Gate reviews, §8 decisions and anything touching
the 0.5 skeleton's interfaces.

**Task brief** (the orchestrator fills this for every task):

```
Task:        WP id and name
Java files:  exact paths (the agent reads these in full before writing)
Rust target: module paths it may create or edit; everything else is read-only
Depends on:  merged interfaces it may call
Fixtures:    scenarios that must pass; scenarios it must add
Out of scope: deferred code it will meet (cite and skip)
Rules:       §3.4 and the Definition of Done below
```

**Definition of Done.**
1. `cargo build`, `cargo clippy` and `cargo test -p <crate>` pass.
2. Named fixtures pass; new behaviour has a new scenario, or the report says
   which behaviour is unverified and why.
3. Every non-obvious constant and ordering carries a `// src:` citation.
4. Each renamed `field_NNN` / `func_NNN` is recorded in `docs/mappings.md` with
   the usage that justified the name.
5. No edits outside the task's Rust target. A needed interface change is
   reported to the orchestrator instead of made.
6. Report lists Java bugs transcribed, deferred code skipped, and anything
   uncertain.

**Scenario authorship.** Transcribers write `scenario.json`; the orchestrator
runs the oracle to produce `expected.jsonl`. An agent never edits an expected
file to make its own test pass.

**Concurrency.** Width is bounded by the "∥" markers: at most about seven
transcribers during WP-1.6, fewer elsewhere. Interface changes to `World`,
`Chunk` or the entity layout are serialised through the orchestrator.

**When a fixture fails and the Rust matches the Java on reading:** suspect the
harness (unseeded randomness, hash order, float formatting) before the port,
and report it. Do not loosen the comparison.

---

## 7. Risks

| Risk | Mitigation |
|---|---|
| JVM `Math.sin`/`cos`/`pow` intrinsics differ from Rust in the last bit | WP-0.2 sweep decides early; fallbacks are running the oracle with intrinsics disabled, or a `libm`-based implementation matched to it |
| `HashSet` iteration order must be reproduced (R14) | `JavaHashSet` in `mc-core`, fixture with a known insertion sequence dumped from the oracle |
| Mob spawning depends on the `Minecraft` singleton headless (R13) | Investigated in WP-0.4; worst case spawning is verified by narrower fixtures and Phase 4 soak |
| Interface churn in the `mc-sim` skeleton invalidates parallel work | Gate 0 owner review; one worked block and entity before fan-out |
| Rendering has no exact oracle | Transcribe data paths exactly; human side-by-side checklist in Phase 4 |
| `RenderBlocks` is one 3.4k-line file with shared state | Split by render type with the shared helpers landed first in the cubes task |
| Agents "improve" quirks | §3.4 rule 6, reviewer checks against Java, deviations ledger |
| Decompiled Mojang code and assets | D8; keep the repository private |

---

## 8. Open decisions for the owner

Work proceeds on the defaults. Each entry says what is being chosen, what each
option buys and costs, and how expensive it is to change your mind later.

### 8.1 Where the simulation lives (D1)

**The choice.** Bevy is an ECS engine: the idiomatic way to build a game in it
is to make every game object a Bevy entity with components, and every behaviour
a system that queries them. The prior research proposed exactly that for
Minecraft's entities. The default here keeps the simulation (world, blocks,
entities, AI) in ordinary Rust structs and uses Bevy for everything the player
sees and hears.

| | Plain-Rust simulation (default) | ECS-native simulation |
|---|---|---|
| Transcription | Java method → Rust function, `super.x()` → a call to the parent's function. Agents can work line by line | Each class hierarchy has to be redesigned into components and systems before any code is written |
| Order-dependent behaviour | Entity list is a `Vec` in Java order | Bevy does not iterate in insertion order (R5); every tick would need an explicit sort by id |
| Re-entrant calls (entity places a block, which notifies a neighbour, which spawns an entity) | Works as in Java | Needs exclusive world access or deferred commands, which changes *when* things happen relative to Java |
| Verification | Runs headless in tests with no engine; traces compare directly to the oracle | Needs a Bevy `World` and schedule in every test |
| Bevy tooling for game objects (inspectors, third-party plugins, parallel systems) | Not available inside the simulation. The client still creates a Bevy entity per visible mob for rendering, so render-side tooling works | Available everywhere |
| Later features that are natural in ECS (mods, new mob types as data) | Added the Java way: new enum variant and functions | Easier |

**What "using Bevy" still means under the default:** window and input, the
terrain mesher and materials, mob and item rendering, sky and weather,
particles, all menus and HUD, audio, asset loading, and the frame loop.

**Cost of switching later.** High. The entity layer is 38 classes / 8.3k Java
lines plus the interfaces every block and item uses to touch entities. After
Gate 0 a switch means redoing Phase 2 and parts of Phase 1.

**Recommendation.** Keep the default. If ECS-native gameplay matters to you as
a goal in itself (for example as a base for a heavily modded game rather than a
faithful port), say so now, because it changes the project from transcription
to redesign and the oracle becomes a looser check.

### 8.2 Which game is the spec (D2)

**The choice.** This repository is not vanilla Beta 1.7.3. It is a browser port
that replaced parts of the Java runtime, and at least two replacements changed
gameplay:

| Deviation | Where | Effect in play |
|---|---|---|
| First `nextGaussian()` after seeding returns `0.0` | `Random.java:25` | Small: the first gaussian draw from each RNG object is not random. Used by item and projectile scatter, mob drops, minecarts; not by terrain generation |
| Sine table has 4,096 entries instead of (as far as I recall) 65,536 | `MathHelper.java:4-11` | Angles snap to about 0.09° steps instead of 0.005°. Feeds movement and look direction (`Entity`, `EntityLiving`, `EntityPlayer`) and also `MapGenCaves`, `MapGenCavesHell`, `WorldGenMinable` and `WorldGenClay`, so **caves and ore veins for a given seed probably differ from vanilla** |

There are likely others not yet found; nothing in the repo lets us detect them
automatically, because the only runnable reference is this port.

| Option | What you get | What it costs |
|---|---|---|
| Match the port (default) | Every behaviour is checkable against the oracle. Worlds look the same as the web version for the same seed | Known bugs are carried over; seeds will not match vanilla seed listings where caves and ores are concerned |
| Target vanilla for known deviations | Closer to the original game; vanilla seeds more likely to match | Each fix needs a matching scripted patch to the oracle's source copy so fixtures stay honest. Feasible for the two above (a few lines each). Deviations nobody has noticed stay in |
| Target vanilla fully | True b1.7.3 | Needs a vanilla b1.7.3 client jar as a second oracle, deobfuscated to line up with these class names. A separate sub-project |

**Cost of switching later.** Low per item if done as planned: each deviation is
a named constant and a ledger entry, and flipping one means regenerating the
fixtures it touches. The sine-table size is the exception: it changes terrain,
so flipping it after players have worlds changes what generates at the edges
of those worlds.

**Recommendation.** Match the port through Gate 2, then decide the sine table
and the gaussian bug explicitly before anyone creates worlds they care about.

### 8.3 Save format (D5)

**The choice.** How worlds are stored on disk.

What the port does today: `level.dat` is plain (not gzipped) NBT; each chunk is
its own gzip-NBT file with a 10-character name, in `c0/` for the overworld and
`c1/` for the nether; everything sits in a virtual filesystem inside the
browser's IndexedDB. Worlds leave the browser only through the port's EPK
export (`ImportExport.java`, `EPKCompiler`/`EPKDecompiler`). Vanilla b1.7.3
uses region files (`.mcr`); no region code exists in this repository. The data
*inside* a chunk (`Level` compound with `Blocks`, `Data`, `SkyLight`,
`BlockLight`, `HeightMap`, entities, tile entities) uses the vanilla tag names,
so the formats differ in packaging, not content.

| Option | What you get | What it costs |
|---|---|---|
| Port layout on the real filesystem (default) | Save and load are checkable byte-for-byte (after gunzip) against files the oracle writes | One small file per chunk; a large world is tens of thousands of files. Not openable by vanilla tools |
| EPK import added | People with worlds in the web version can bring them over | Port `EPKDecompiler` (small); can be verified with an export from the web build |
| Vanilla McRegion read/write | Open real b1.7.3 worlds, use existing map tools | Written from the format documentation with no oracle; needs hand-made test worlds |
| Own container (single file or region-style) with the same chunk NBT | Fewer files, faster | Diverges from the oracle at the file level; chunk contents still checkable |

**Cost of switching later.** Low. Storage sits behind the `IChunkLoader` /
`ISaveHandler` seam the Java already has, and the chunk NBT is the same in
every option, so a converter or a second backend can be added at any time.

**Recommendation.** Default for Phase 1. Tell me if you or anyone else has web
worlds worth keeping; if so EPK import moves into Phase 4.

### 8.4 Reusing the bundled Rust project (D4)

**The choice.** `2010-rust-rewrite-mashup-main/` contains
`third_party/minecraftoss`, about 130k lines of Rust reimplementing *modern*
Minecraft (26.3), plus a Bevy 0.19 terrain renderer in
`crates/minecraft_terrain`. The question is whether agents may copy from it or
only read it.

**Licence status.** The workspace root is Apache-2.0. Its `NOTICE` describes
`minecraftoss` as the fork author's own project, copied from commit `4013a68`,
but that directory has no licence file and the upstream MinecraftOSS repository
was not checked. So the licence is probably Apache-2.0 and is not confirmed.

**What is actually reusable.** Less than the prior research suggested:

- The RNG cannot be copied (R3), and the NBT codec targets a format with tag
  types beta does not have; both are a few hundred lines to write fresh.
- Worldgen, entity AI, physics, redstone and recipes are for a different game
  version. Copying them risks importing modern behaviour that looks plausible.
- The useful parts are *examples*: how a Bevy 0.19 chunk mesher lays out
  vertices and feeds light to a shader (`mesh.rs`, `sections.rs`,
  `lighting.rs`), the trace-binary test layout, and the lint configuration.

| Option | Effect |
|---|---|
| Reference-only (default) | Agents may read it for technique; no code is copied. No licence exposure |
| Copy permitted | Saves some effort on the Bevy mesher scaffolding in WP-3.3. Requires the licence confirmed, attribution kept, and every copied piece checked against the Java |

**Cost of switching later.** None; it only widens what agents may do.

**Recommendation.** Keep the default unless you know the author or can point me
at the upstream licence. It matters mainly if this rewrite is ever published.

### 8.5 Target platform

**The choice.** The original is a browser game (the README links a hosted
build). Bevy can target native desktop and the browser (wasm), but the browser
adds constraints:

- No threads by default, so background chunk meshing and lighting would run on
  the main thread or need a worker setup.
- No filesystem; saves go to IndexedDB or OPFS through an async API.
- Audio cannot start before a user gesture; assets load over HTTP.
- Larger download and typically lower performance than native.

| Option | What you get | What it costs |
|---|---|---|
| Native desktop first (default) | Simplest path to a playable, fast build; ordinary files and threads | Not a drop-in replacement for the hosted web version until Phase 5 |
| Native first with wasm guardrails | Same, and the later web build is a port of the client only | Small now: `mc-sim` does no direct file or thread access (storage and background work go through traits), and CI runs `cargo check --target wasm32-unknown-unknown` on `mc-core`, `mc-nbt` and `mc-sim` |
| Web first | Replaces the hosted build directly | Every Phase 3 package carries the constraints above; slower iteration |

**Cost of switching later.** Low with the guardrails, moderate without them
(file and thread assumptions would have spread through the simulation).

**Recommendation.** Native first with the guardrails. They are cheap enough
that I would fold them into WP-0.1 and WP-0.5 unless you object; say the word
and I will update those rows.
