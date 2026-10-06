# PLAN_v2 — Rust/Bevy rewrite of Beta 1.7.3 (master plan)

**Status:** v2, 2026-10-05. Supersedes `PLAN_v1.md`, `RUST_REWRITE_BLUEPRINT.md`
and `docs/ADR-007-golden-harness.md` wherever they conflict. Agents follow this
document.

**Goal:** a native Rust + Bevy game with **vanilla Minecraft Beta 1.7.3
singleplayer parity**, proven by fixtures generated from Java. Once that is
reached, a browser build like the original port.

**What changed from v1** (owner decisions, 2026-10-05):

| # | Decision | Effect on the plan |
|---|---|---|
| 1 | Plain-Rust simulation, Bevy as shell | Confirmed; no longer open |
| 2 | Vanilla parity ranks above web-port parity | New spec hierarchy, a vanilla reference tree, a deviation audit and a two-oracle setup (§2 D2, §4) |
| 3 | No web-port world import. Vanilla world import is a priority; export is planned, low priority | Native save format is vanilla McRegion; the port's save layout and EPK are dropped (D5, WP-1.9) |
| 4 | MinecraftOSS code is Apache-2.0 and may be copied; credit in the README | Reuse list and attribution rules (D4); RNG, NBT and region container are adapted instead of written fresh |
| 5 | Native first; simulation free of direct file and thread access; CI checks it compiles for the browser; web build after vanilla parity | Guardrails in WP-0.1 and WP-0.6; web build is Phase 5 |

Contents: 1 Audit of prior research · 2 Decisions · 3 Architecture ·
4 Reference and oracle · 5 Phases and work packages · 6 Agent protocol ·
7 Risks · 8 What is needed from the owner

---

## 1. Audit of the prior research

Every claim below was checked against the source in this checkout. The harness
claims were tested by compiling the Java tree and running a headless probe
(recipe in §4.2).

### 1.1 Refuted — these change the plan

| # | Blueprint / ADR-007 claim | What the source shows | Consequence |
|---|---|---|---|
| R1 | Singleplayer runs on `WorldClient extends World`; the harness should build a `WorldClient` | Singleplayer constructs plain `World` (`Minecraft.java:952, 989, 1000`). `WorldClient` is built only by `NetClientHandler` (`:59, :474`) and overrides `updateBlocksAndPlayCaveSounds()` to empty, `scheduleBlockUpdate()` to empty and `TickUpdates()` to `return false` (`WorldClient.java:73-81`) | A `WorldClient` harness would record a world with no block ticks at all. Oracle uses `World`. `WorldClient` is multiplayer-only and deferred |
| R2 | Gameplay logic has no GL/JSO reachability, so it runs on a plain JVM with `src/lwjgl/java` on the classpath | `Block.<clinit>` → `Item` → `StatList` → `StatCollector` → `StringTranslate` → `GL11.getResourceAsStream` (`StringTranslate.java:14`). With the LWJGL backend this dies in `GL11_1.<clinit>` with `ClassNotFoundException: org.lwjgl.system.FunctionProvider` on the first chunk request | Oracle needs a headless platform stub. A one-method stub `GL11` placed first on the classpath is enough for worldgen and ticking (tested) |
| R3 | RNG is `java.util.Random`; copy MinecraftOSS `LegacyRandom` 1:1 | The game uses its own `net.minecraft.src.Random` (no file imports `java.util.Random`). Its `setSeed` sets `haveNextNextGaussian = true` (`Random.java:25`), so the first `nextGaussian()` after any seeding returns `0.0`. Measured, seed 42: port `0.0`, JDK `0.27686040089698144`. `nextInt` matches the JDK. | Vanilla used `java.util.Random`, so `JavaRandom` follows the JDK; the port's quirk is ledger entry DEV-001 (D2). 13 gameplay/render files call `nextGaussian` |
| R4 | `FixedUpdate` order: input → random block ticks → scheduled ticks → tile entities → entities → player → time | Actual order in `Minecraft.runTick` (`:887-927`): `updateEntities()` (weather effects → entity list → tile entities) and then `World.tick()` (weather → sleep check → `SpawnerAnimals` → chunk unload → skylight → time and autosave → scheduled ticks → random ticks) → `randomDisplayUpdates` → particles. `runGameLoop` and `updateEntitiesAndBlocks` do not exist | Tick order is §3.3, transcribed from source |
| R5 | `bevy_ecs` iteration order mirrors the Java entity list because we control insertion | Bevy query iteration follows archetype/table storage; it is not insertion order across archetypes and despawn swap-removes rows | With R6, this is why the simulation is not built on ECS (D1) |
| R6 | ADR-001 Rule 1: a Java override runs *instead of* the base system; wolf is the worked example | `EntityWolf.updatePlayerActionState`, `onLivingUpdate` and `onUpdate` all begin with `super.…()` (`EntityWolf.java:86, 115, 142`). Most overrides in the tree extend rather than replace | The `Without<Wolf>` exclusion pattern would drop base behaviour. See D1 |
| R7 | Wolf example constants: body 0.6×0.8, dataWatcher bits 0x02/0x04/0x08, collar at byte 20 | `setSize(0.8F, 0.8F)`; flags on byte 16 are sitting `1`, angry `2`, tamed `4`; index 17 is owner string, 18 is health; there is no collar (`EntityWolf.java:17-27, 412-452`) | The "pattern to copy" was not read from source. Discard it |
| R8 | `field_9365_p` is `moveStrafing` (`EntityLiving.java:8`); `0.98` constant at `EntityLiving.java:142` | `field_9365_p` (line 7) is a separate field initialised to `Math.random() * 12398.0F`; `moveStrafing` is already named at line 50. The `0.98` sites are lines 570, 676, 677 | Example mapping is invented. `docs/mappings.md` entries must come from reading usage |
| R9 | `src/lwjgl/java` and `net/PeytonPlayz585` are TeaVM shims | `src/teavm/java` (33 files, 12.9k LOC) is the TeaVM/WebGL backend and the one `build.gradle` compiles. `src/lwjgl/java` (60 files, 23.8k LOC) is a desktop LWJGL backend plus vendored sound libraries, wired by `.classpath` and `debugRuntime.launch`. Still nothing to port from either | Oracle P compiles against `src/lwjgl/java`; never ship or port it |
| R10 | Round-trip real `level.dat`/chunk files from `jars/` | No `.dat`, `.mcr` or chunk file exists anywhere in the repo. `jars/` holds only jars and natives | Save fixtures must be generated by the oracle |
| R11 | Save format is vanilla (`.dat` chunk codec, `level.dat`) | Eaglercraft-specific: `level.dat` is written **uncompressed** (`EaglerSaveHandler.java:50-60`), chunks are gzip NBT in flat files named by a custom 10-character base-36 scheme under `c0/` (overworld) and `c1/` (nether) (`ChunkLoader.java:17-33`), on a virtual filesystem | Not compatible with vanilla saves, and not carried over. See D5 |
| R12 | ~59 non-FX entity classes; `EntityFX` + 20 subclasses; slime under `EntityMob` | 38 non-FX entity classes (58 `Entity*.java` minus 17 FX, `EntityRenderer`, `EntityList`, `EntitySorter`); `EntityFX` + 16 subclasses; `EntitySlime extends EntityLiving implements IMob`; `EntityPigZombie extends EntityZombie`; `EntityClientPlayerMP extends EntityPlayerSP` | Entity work is about a third smaller than budgeted |
| R13 | `EntityList.createEntityByName` for harness spawning | Method does not exist. `createEntityInWorld` routes through `SpawnerAnimals.newInstance`, which reads `Minecraft.getMinecraft().theWorld` (`SpawnerAnimals.java:329` and siblings) and throws headless | Oracle constructs entities directly (`new EntityPig(world)`). Ledgered as DEV-003 |
| R14 | Only ordering hazard is `Chunk.chunkTileEntityMap`; tile-entity tick order depends on it | Tile entities tick from `loadedTileEntityList`, an `ArrayList`. The map's order matters only at chunk load/save (`Chunk.java:455`, `ChunkLoader.java:123`). The hazard the ADR missed: `World.positionsToUpdate` is a `HashSet<ChunkCoordIntPair>` iterated at `World.java:1884` to pick the chunk order for random ticks, which share `World.rand` | Rust must reproduce Java `HashSet` iteration order for that set (§3.4) |
| R15 | Reflective `setSeed` on `Random` fields makes the Java side deterministic | 14 gameplay call sites use `Math.random()` directly (`EntityLiving.java:37,62,64,65,378,379,385`, `EntityItem.java:9,17,18,20`, `EntityTNTPrimed.java:17`, plus two particle sites), and `new Random()` itself seeds from `Math.random()` | Oracle is built from a mechanically patched source copy (§4.3) |

### 1.2 Verified — carry forward

- 632 files / 60,555 LOC in `net/minecraft/src`; plus `Minecraft.java` (1,247).
- No server classes exist. Singleplayer is one in-process `World`; Phase 1 needs no protocol code. 59 `Packet*.java` confirmed; networking totals 69 files / 4.5k LOC, all deferrable.
- Texture-pack classes exist (4 `TexturePack*`, 2 GUI) and are safely deferrable.
- Registry sizes: 96 blocks, 106 items (blueprint said ~90 / ~100), 31 `GuiScreen`/`GuiContainer` subclasses (~30).
- Random ticks: 80 picks per chunk per tick (`World.java:1944`), 19×19 chunks around each player. The "216" in the blueprint matches nothing.
- `Chunk.func_997_a` seed formula (`Chunk.java:593`) and the unseeded `World.rand` / `field_9437_g` / `Entity.rand` sites are as ADR-007 lists them.
- `World.worldAccesses` is a clean no-op seam for the oracle.
- Golden-fixture design in ADR-007 §5 (one scenario file, floats as IEEE-754 bit patterns, digests over canonical byte arrays, fixtures checked in) is sound and adopted.
- MinecraftOSS facts: five crates, README states commit `4013a68`; LOC figures match (core 8.0k, generator 31.2k, world 15.1k, player 16.1k + 5.1k bins, entities 35.2k + 20.2k bins); engine crates have no Bevy dependency; workspace uses Bevy 0.19; lints allow `excessive_precision` and `approx_constant`; its wolf is 0.6×0.85; the catalog JSON artifacts it loads are absent; `generator/` is inapplicable to beta. The mashup `NOTICE` describes it as the fork author's own project but there is no licence file for it, so the tree itself does not state its licence; the owner has since confirmed Apache-2.0 with the maintainer (D4).
- "No runnable desktop client" holds in practice, for a different reason than stated: `jars/lwjgl.jar` is LWJGL 2 (2013) while `lwjgl-jemalloc.jar` needs LWJGL 3's `org.lwjgl.system`, which is missing.

### 1.3 New findings the research missed

- **The whole Java tree compiles with plain `javac` on the installed JDK 25** (`src/main/java` + `src/lwjgl/java` against `jars/*.jar`, 813 classes, no errors). Gradle is not needed for the oracle.
- **Headless worldgen works and is reproducible.** Seed 12345, three chunks, two independent `World` instances: identical CRCs both times.
- **This port deviates from vanilla b1.7.3 in gameplay-visible ways.** Besides R3: `MathHelper` uses a 4,096-entry sine table with factor `651.8986F` (`MathHelper.java:4-11`); vanilla, as far as I recall, uses 65,536 entries. That feeds every `moveFlying` and look-vector calculation. There are likely more; finding them is WP-0.2.
- **Entities do not tick unless chunks exist within ±32 blocks** (`World.java:1286`), and random ticks happen only around entries in `playerEntities`. In the probe a pig spawned in mid-air did not move in 100 ticks. Oracle scenarios need preloaded chunks and a headless player.
- **Sky dimension is unreachable in this build.** `WorldProviderSky` exists, but `usePortal` only switches between −1 and 0 and the save handler only has `c0`/`c1`.
- **Environment gaps:** no `cargo`/`rustc` on this machine, the project is not a git repository, and the Gradle wrapper pins 8.5, which predates JDK 25.
- **Subsystem sizes** (drives §5): rendering 69 files / 11.2k LOC (`RenderBlocks.java` alone 3,376); blocks 83 / 8.1k; entities 38 classes / 8.3k; world core 26 / 5.6k; GUI 52 / 5.0k; worldgen 48 / 4.9k; items, crafting and inventory 69 / 3.6k; stats and achievements 61 / 2.7k (of which the bundled JSON parser is 47 / 1.7k and is replaced by `serde_json`); NBT and save 20 / 1.2k; particles 18 / 1.1k.

---

---

## 2. Decisions

All five v1 open decisions are settled. D1–D5 record the owner's choices.

**D1 — Simulation is a plain-Rust core; Bevy is the shell.**
`World`, chunks, blocks and entities live in ordinary Rust structs in crates with
no Bevy dependency. Bevy owns the window, input, rendering, UI, audio and assets,
and drives the simulation from one system. Reasons: R5 and R6; Java call chains
re-enter freely (entity → `setBlock` → neighbour notification → spawns an
entity) inside one stack frame; entity-list order is gameplay-visible. From the
earlier ADR-001, only rule 5 (one closed `EntityKind` enum with match tables)
and rule 6 (models as data tables) survive.

**D2 — The spec is vanilla Beta 1.7.3.**
Where this repository's Java differs from vanilla in singleplayer behaviour,
vanilla wins. Evidence for "what vanilla does" is ranked:

1. A **vanilla reference tree**: the b1.7.3 client decompiled from the owner's
   own jar (§4.1). Authoritative.
2. The port's Java, for every class the deviation audit shows is unchanged from
   vanilla. This is expected to be the large majority and is what agents read
   day to day.
3. Recalled or documented vanilla behaviour. Usable only as a lead; any ledger
   entry resting on it is marked `unconfirmed` until level 1 confirms it.

Every difference is an entry in `docs/DEVIATIONS.md` (id, file and lines, port
behaviour, vanilla behaviour, evidence level, oracle patch). Known so far:

| Id | Deviation | Vanilla behaviour | Evidence |
|---|---|---|---|
| DEV-001 | `Random.setSeed` leaves the gaussian flag set; first `nextGaussian()` returns `0.0` | `java.util.Random` semantics | Confirmed against the JDK (R3) |
| DEV-002 | `MathHelper` sine table has 4,096 entries, factor `651.8986F` | 65,536 entries, factor `10430.378F` | **Unconfirmed** (recalled) |
| DEV-003 | `SpawnerAnimals.newInstance` builds mobs with `Minecraft.getMinecraft().theWorld` instead of the world passed in | Reflection on the class with the given world | Likely; affects only which world object is used |
| DEV-004 | Save format (R11), `Eagler*` classes, EPK import/export, `GuiCreateOrImport` | McRegion saves, no EPK | Structural; see D5 |
| DEV-005 | Music list includes a `newMusic` set; sound and resource loading replaced | To be established by the audit | Unconfirmed |

Out of scope even though vanilla has it: anything that depends on defunct
online services (login, skin and cloak downloads, resource download).

**D3 — Scope order.** (1) Native singleplayer with vanilla parity. (2) Browser
build. (3) Everything else: multiplayer (`Packet*`, `Net*`, `WorldClient`, MP
player classes), texture-pack selection, the sky dimension, pre-b1.3 world
conversion. Agents that meet out-of-scope code cite it, skip it and tag the
task `deferred`.

**D4 — MinecraftOSS code may be copied, with attribution.**
Licence confirmed by the owner with its maintainer as Apache-2.0; the
maintainer asks for credit in the project README.

| Source (under `2010-rust-rewrite-mashup-main/`) | Use | Notes |
|---|---|---|
| `third_party/minecraftoss/core/src/random.rs` — `LegacyRandom` and its `next_gaussian` | Adapt into `mc_core::java_random` | Drop the Xoroshiro and positional-factory halves. Must pass JDK-generated fixtures |
| `…/core/src/nbt.rs` (372 lines) | Adapt into `mc-nbt` | Supports `IntArray`/`LongArray`, which b1.7.3 lacks: the beta codec must reject them on read and never write them. Compounds are `BTreeMap`, so key order on write differs from Java; see §4.4 |
| `…/core/src/anvil.rs` (186 lines) | Adapt into the McRegion container | Same sector/header layout as `.mcr`. It rewrites whole region files; acceptable to start, revisit in Phase 4 |
| Workspace lint settings (`excessive_precision`, `approx_constant` allowed) | Copy | Keeps transcribed float literals intact |
| `crates/minecraft_terrain/src/{mesh,sections,lighting,texture_mips}.rs` | Scaffolding for WP-3.3 | Vertex layout, section meshing jobs, light-to-shader plumbing. Which faces, textures and light values to emit still come from `RenderBlocks.java` |
| Trace-binary / scenario-replay layout | Pattern | Informs `mc-golden` |
| `generator/`, `entities/`, `world/level/`, `player/` | **Do not copy** | They implement Minecraft 26.3. Plausible-looking but wrong-era behaviour is the main contamination risk |

Rules for copied code: each file carries a header naming its origin
(`// Adapted from MinecraftOSS <path> @ 4013a68, Apache-2.0; modified`), the
repository gets a `NOTICE` entry and a copy of the Apache-2.0 text, and the
root `README.md` gets a credits section. Copied code is presumed 26.3-specific
until the beta fixtures pass; upstream's own tests prove nothing here.

**D5 — The native save format is vanilla b1.7.3 McRegion.**
Worlds are stored exactly as vanilla stores them: gzipped `level.dat` and
`region/r.<x>.<z>.mcr` files, with the nether under `DIM-1/`. Consequences:

- **Import (priority):** "Import world" copies a vanilla save folder into the
  game's saves directory and opens it. No conversion step, so nothing to drift.
- **Export (planned, low priority):** a save folder is already in vanilla
  layout. The remaining work is an "Export" action and, mainly, proving in a
  real b1.7.3 client that worlds we wrote open correctly (WP-4.5).
- **Not supported:** the port's `c0/`/`c1/` layout, uncompressed `level.dat`
  and EPK archives. `Eagler*` save classes are not transcribed.
- The port's chunk NBT uses vanilla tag names (`Level`, `Blocks`, `Data`,
  `SkyLight`, `BlockLight`, `HeightMap`, …), so `ChunkLoader.java`'s chunk
  encode/decode is still a valid source for chunk contents. The exact
  `level.dat` wrapping and region details come from the vanilla reference.
- Imported worlds containing block or item ids b1.7.3 does not define (worlds
  touched by later versions) are refused with a message by default (§8).
- Pre-b1.3 worlds (one file per chunk, which vanilla b1.7.3 converts on open)
  are deferred.

**D6 — Toolchain.** Stable Rust pinned in `rust-toolchain.toml`. Bevy 0.19, the
version the copyable scaffolding targets; the orchestrator may pick a newer
stable release at kickoff if that scaffolding ports cleanly, and it is then not
upgraded again before Gate 4. Oracles build with `javac` via shell scripts.

**D7 — Rendering parity is visual, not structural.** The Java renderer is
fixed-function immediate-mode GL. Game-data parts are transcribed (which faces,
which texture index, which colour and light value, model boxes); the drawing
machinery is written for Bevy.

**D8 — Mojang material stays local.** Assets are read from `resources/` at
runtime, never embedded in the binary. The vanilla jar and its decompiled tree
live in a git-ignored `reference/` directory and are never committed. Keep the
repository private while it contains decompiled Java.

**D9 — Browser readiness from day one.** `mc-core`, `mc-nbt` and `mc-sim` use no
`std::fs`, `std::thread`, `std::time::Instant`/`SystemTime`, or environment
access. Storage goes through a `Storage` trait (read, write, list, rename,
delete on byte blobs) implemented natively by the client; time is passed in.
Enforced by clippy `disallowed-types`/`disallowed-methods` and a CI job running
`cargo check --target wasm32-unknown-unknown` on those three crates.

---

## 3. Architecture

### 3.1 Workspace

New Cargo workspace in `rust/` at the repository root.

| Crate | Depends on | Owns | Source |
|---|---|---|---|
| `mc-core` | — | `JavaRandom`, `MathHelper`, AABB, `Vec3D`, ray-trace result, `JavaHashSet`/`JavaHashMap` order emulation | `MathHelper`, `AxisAlignedBB`, `Vec3D`, `MovingObjectPosition`; RNG adapted from MinecraftOSS |
| `mc-nbt` | — | NBT tags, big-endian codec, gzip and zlib wrappers | adapted from MinecraftOSS; checked against `NBT*`, `CompressedStreamTools` |
| `mc-sim` | core, nbt | Everything that ticks: block and item registries and behaviour, world, chunks, lighting, worldgen, tile entities, entities, AI, inventory, crafting, stats; McRegion and `level.dat` codecs over the `Storage` trait | all remaining non-render, non-GUI, non-net classes |
| `mc-golden` | sim | Scenario runner and trace writer; dev-dependency only | — |
| `mc-client` | sim, bevy | Bevy plugins: tick driver, native `Storage`, mesher, entity and sky rendering, particles, GUI, audio, input, settings | `Render*`, `Model*`, `Gui*`, `EntityRenderer`, `RenderGlobal`, `EffectRenderer`, `SoundManager`, `GameSettings` |
| `mc-app` | client | Binary; thin | `Minecraft.java` |

`mc-sim` is one crate on purpose. Blocks call the world, the world calls blocks,
entities call both, and items place blocks and spawn entities; splitting them
would need cycles or trait indirection at every one of those calls. Modules
inside `mc-sim` (`block/`, `item/`, `world/`, `gen/`, `entity/`, `inventory/`,
`save/`) give agents the same ownership units. If compile times hurt, `gen/` is
the clean piece to split out later.

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
- **Borrowing:** entity ticks take the entity out of its slot
  (`Option::take`), run with `&mut World`, and put it back, so entity code can
  query and mutate the world and other entities as Java does. Defined once in
  WP-0.6 and used everywhere.
- **Side effects for the client** (sounds, particles, block-changed
  notifications) go through an `IWorldAccess`-equivalent event queue that
  `mc-client` drains each frame. The simulation never calls into Bevy.
- **Single-threaded and synchronous**, like the Java. Background work (chunk
  meshing) belongs to the client and uses Bevy's task pool, which already
  degrades to one thread in the browser.

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

The audit (WP-0.2) confirms this against the vanilla tree. The tick driver is a
port of `Timer.java` (20 TPS, at most 10 ticks per frame, partial tick for
interpolation) run from a single Bevy system in `Update`, with the clock value
passed in by the client.

### 3.4 Determinism rules

1. Java `double` → `f64`, `float` → `f32`, `int` → `i32` with wrapping
   arithmetic, `long` → `i64` wrapping. Casts follow Java semantics; Rust `as`
   already saturates float→int the way Java does.
2. Constants are transcribed exactly, with a `// src: File.java:line` comment,
   or `// vanilla: File.java:line` plus the `DEV-nnn` id where a deviation
   applies.
3. All randomness goes through `JavaRandom`. Each Java `Math.random()` site maps
   to a named draw from one `MathRandom` stream owned by the world (§4.3).
4. Collections: `ArrayList` → `Vec`, `TreeSet` → `BTreeSet` with the Java
   comparator, `HashMap` used only for lookup → any map. Where Java *iterates* a
   hash collection in gameplay (`World.positionsToUpdate`,
   `Chunk.chunkTileEntityMap`), use `mc_core::JavaHashSet`/`JavaHashMap`, which
   reproduce bucket order from the Java `hashCode`. Any new case found is
   reported, not silently replaced.
5. `Math.sin/cos/atan2/pow/exp/log` (87 call sites) go through `mc_core::math`.
   WP-0.3 decides the implementation from a bit-comparison sweep against the
   oracle JVM.
6. Vanilla bugs are transcribed and flagged, never fixed. Port bugs are not
   transcribed; they go in the ledger.

### 3.5 Client notes

- **Terrain:** 16×16×16 sections as in `WorldRenderer`, meshed on Bevy's task
  pool from chunk snapshots, drawn with a custom unlit material (vertex colour ×
  brightness table, alpha test, nearest filtering, distance fog) in two passes.
  Scaffolding from `minecraft_terrain` (D4).
- **GUI:** Java screens draw in scaled pixel coordinates with immediate-mode
  calls (`drawTexturedModalRect`, `drawString`). Default approach is a small 2D
  quad batcher with that same API, so `Gui*` classes transcribe nearly line for
  line. WP-3.6 begins with a short spike confirming this against `bevy_ui`.
- **Animated textures** (`TextureFX` subclasses: water, lava, fire, portal,
  compass, clock) are procedural and are transcribed, writing into the atlas.
- **Audio:** `bevy_audio` playing the `.ogg` files in `resources/`; sound names
  and volume/pitch arithmetic transcribed from the call sites.
- **World import** is a client screen that copies a chosen folder through the
  native `Storage` after validating `level.dat` and block ids.

---

## 4. Reference and oracle

### 4.1 Vanilla reference tree (new)

The port's identifiers (`field_9365_p`, `func_997_a`, `var1`) are Mod Coder
Pack names, so a b1.7.3 client decompiled with MCP-lineage tooling (RetroMCP
or equivalent) should line up with it class by class. That has not been tried
yet; WP-0.2 tries it.

- Input: the owner's own b1.7.3 client jar, placed in `reference/` (§8).
- Output: `reference/vanilla/src/…`, git-ignored, with a recorded tool version
  and jar hash so any machine reproduces the same tree.
- **Deviation audit:** normalise both trees (strip imports of shim packages,
  whitespace) and diff every class. Each logic class is labelled `identical`,
  `differs`, `port-only` or `vanilla-only` in `docs/AUDIT.md`; every `differs`
  hunk becomes a `DEV-nnn` ledger entry or is marked cosmetic. A style scan of
  the port for hand-written code (generics, comments, shim imports) flags only
  nine logic files and misses `SpawnerAnimals`, so it cannot replace the diff.

### 4.2 Two oracles

| | Oracle V | Oracle P |
|---|---|---|
| Built from | Vanilla reference tree | A generated copy of the port's `src/main/java` with one scripted patch per confirmed `DEV-nnn` entry |
| Availability | After WP-0.2 succeeds | Today (recipe below, verified) |
| Role | Authoritative expected output | Bootstrap, and permanent cross-check |

Both run the same harness sources and scenario files. Fixtures checked into the
repository come from Oracle V once it exists. CI's regeneration job runs both;
**any scenario where V and P disagree is either a known ledger entry whose
patch is wrong or an undiscovered deviation**, and blocks until explained. That
agreement check is what turns "we believe this is vanilla" into evidence.

If no vanilla jar is supplied, the project runs on Oracle P alone and vanilla
parity is limited to deviations someone notices; every ledger entry stays at
evidence level 3. This is the main reason §8 asks for the jar.

Oracle P recipe, as verified on the installed JDK 25:

```
javac -nowarn -proc:none -encoding UTF-8 -cp "jars/*" -d build/oracle-p \
      $(find build/port-src src/lwjgl/java -name '*.java')
# stub: package net.PeytonPlayz585.opengl; class GL11 with only
#   public static InputStream getResourceAsStream(String p)  → reads resources/<p>
java -cp build/oracle-stub:build/oracle-p harness.Main …
```

The stub directory precedes the real classes, so `GL11` resolves to the stub
and no LWJGL class is loaded. With it, `new World(new SaveHandlerMP(), "name",
seed)`, `ChunkProviderGenerate.provideChunk`, `updateEntities()` and `tick()`
run headless. Oracle V's recipe is established in WP-0.2; vanilla's
`StringTranslate` should not need the stub, but its client classes will need
LWJGL 2 on the compile classpath.

### 4.3 Harness rules

- Worlds are `World`, never `WorldClient`.
- Both oracles compile from generated source copies in which a checked-in
  script rewrites `Math.random()` to `HarnessMath.random()` (a seeded stream).
  No hand edits to game code; each script's diff is reviewed once.
- After constructing a world, the harness seeds `World.rand` and
  `field_9437_g`; every entity's `rand` is seeded from `(scenario seed, entityId)`
  on construction. `Entity.nextEntityID` is reset per scenario.
- Entities are constructed directly. Scenarios that need ticking preload chunks
  within 32 blocks (`World.java:1286`); scenarios that need random ticks or mob
  spawning add a `HarnessPlayer extends EntityPlayer`.
- Save fixtures: Oracle V writes real McRegion worlds to a temp directory;
  these are the import test corpus, alongside any real world the owner supplies.

### 4.4 Tiers

| Tier | Covers | Compared by |
|---|---|---|
| A | RNG sequences (also checked directly against `java.util.Random`), sine table, math sweep, NBT primitives, terrain generation and population per chunk, biome arrays | exact bytes / digest |
| B | Lighting, scheduled and random ticks, fluids, fire, plants, redstone, pistons, rails, tile entities, explosions | per-tick digests and block dumps, exact |
| C | Entity physics, collisions, AI, projectiles, vehicles, item entities, damage, inventory and crafting results | per-tick field traces, floats as bit patterns, exact |
| S | Saves: read oracle-written and real vanilla worlds; write, re-read, and compare | parsed NBT trees and chunk arrays equal. Not raw file bytes: Java writes compound keys in `HashMap` order and gzip output differs between implementations |
| D | Rendering, GUI, audio, feel | side-by-side against a real vanilla b1.7.3 client where the owner can run one, otherwise the `web/` build read with the ledger in hand; human-reviewed |

Traces use ADR-007 §5's formats (one scenario file, floats as IEEE-754 bit
patterns, digests over canonical byte arrays). Fixtures are checked in under
`rust/tests/golden/`; ordinary CI runs only the Rust side.

### 4.5 Harness acceptance

1. Tier A regenerates byte-identically across three runs, per oracle.
2. Sensitivity check: perturbing one RNG seed bit, or one block in a scenario,
   fails the matching fixture.
3. A failing comparison prints the first differing tick and field with both
   values decoded.
4. With both oracles present, the V/P agreement report is empty apart from
   ledgered deviations.

---

## 5. Phases and work packages

LOC figures are Java lines to transcribe. "∥" marks packages that can run
concurrently once their dependencies are merged. Packages above roughly 1.5k LOC
are split by the listed sub-units, one agent task each.

### Phase 0 — Foundations (serial except where marked; nothing in Phase 1 starts until 0.6 is merged)

| WP | Deliverable |
|---|---|
| 0.1 | `git init`; `.gitignore` (build outputs, `web/`, `reference/`, oracle output); install rustup; `rust/` workspace with lint settings copied per D4; clippy disallow-lists and wasm `cargo check` job per D9; CI for `cargo test` and `clippy`; `NOTICE`, Apache-2.0 text and README credits section per D4 |
| 0.2 ∥ | Vanilla reference tree, deviation audit (`docs/AUDIT.md`, `docs/DEVIATIONS.md`), Oracle V build recipe. Blocked on the jar (§8); everything else proceeds on Oracle P meanwhile |
| 0.3 | Harness Tier A for Oracle P, `mc-golden` runner, Oracle P patch scripts for confirmed deviations; fixtures for RNG, sine table, `Math.*` sweep, three terrain seeds |
| 0.4 | `mc-core` and `mc-nbt` adapted from MinecraftOSS and green against 0.3 and against JDK-generated RNG sequences; decision recorded for §3.4 rule 5 |
| 0.5 | Harness Tier B/C scaffolding: `Math.random` rewrite script, seeding protocol, `HarnessPlayer`, chunk preload, scenario ops (`set_block`, `spawn`, `set_motion`, `use_item`, …) |
| 0.6 | `mc-sim` skeleton, owner-reviewed: `World`, `Chunk`, block table and dispatch, entity struct layout and the take/put-back pattern, event queue, `Storage` trait, module layout, one block and one entity implemented end to end as the pattern to copy; `docs/mappings.md` started |

**Gate 0:** harness acceptance (§4.5) passes on the oracles available; owner
approves the 0.6 skeleton. If Oracle V arrives after Gate 0, its first act is
regenerating every fixture and reporting disagreements.

### Phase 1 — World without entities

| WP | Source | LOC | Depends | Gate |
|---|---|---|---|---|
| 1.1 Block data table | `Block.java` static init, `Material*`, `StepSound*` | ~0.9k | 0.6 | oracle dump of every block's fields |
| 1.2 Item data table ∥ | `Item.java` static init, tool/armour enums | ~0.6k | 0.6 | oracle dump of every item's fields |
| 1.3 Chunk storage and lighting | `Chunk`, `NibbleArray`, `MetadataChunkBlock`, light paths in `World` | ~1.5k | 0.6 | B: light arrays after scripted edits |
| 1.4 Noise and terrain shape ∥ | `NoiseGenerator*`, `WorldChunkManager*`, `BiomeGen*`, `ChunkProviderGenerate/Hell.provideChunk`, `MapGenCaves*` | ~2.3k | 0.4 | A: unpopulated chunk digests |
| 1.5 World block access and ticks | remainder of `World` block get/set, neighbour notification, scheduled ticks, random ticks, weather, time | ~1.8k | 1.1, 1.3 | B: tick traces |
| 1.6 Block behaviour ∥ (seven tasks) | fluids · fire, plants, crops, leaves, snow/ice · redstone wire, torches, repeaters, buttons, levers, plates · pistons · rails (`RailLogic`) · containers, doors, beds, signs and other interactables · simple remainder | ~7.2k | 1.5 | B: one scenario set per group |
| 1.7 Population features ∥ | `WorldGen*`, `populate` in both providers | ~2.4k | 1.5, 1.6 simple blocks | A: populated chunk digests |
| 1.8 Tile entities and explosions | `TileEntity*` (7 logic kinds), `Explosion` | ~1.0k | 1.5 | B |
| 1.9 Saves: McRegion and `level.dat` (two tasks) | region container adapted from MinecraftOSS `anvil.rs`; chunk encode/decode from `ChunkLoader.java`; `level.dat`, player data, session lock and region details from the vanilla reference; `WorldInfo`, `ChunkProvider*` caching | ~1.0k + container | 1.3, 1.8, 0.2 for the vanilla-only parts | S: read oracle-written and real vanilla worlds; write/re-read equality |

**Gate 1:** all Tier A, B and S fixtures green. A real vanilla world loads
headless and its chunk digests match what the oracle reads from the same files.

### Phase 2 — Entities and player

| WP | Source | LOC | Depends | Gate |
|---|---|---|---|---|
| 2.1 Base entity physics | `Entity.java` (`moveEntity`, water/lava/fire, riding) | ~1.1k | 1.5 | C: falling, stepping, collision traces |
| 2.2 Living base and pathfinding | `EntityLiving`, `EntityCreature`, `Path*`, `Pathfinder` | ~1.6k | 2.1 | C |
| 2.3 Mobs ∥ (three tasks) | animals and wolf · monsters, slime, ghast · squid, spawn rules (`SpawnerAnimals`, with DEV-003) | ~2.6k | 2.2 | C per mob |
| 2.4 Non-living entities ∥ (two tasks) | item, falling sand, TNT, painting, projectiles, fishing hook, lightning · minecart and boat | ~2.8k | 2.1 | C |
| 2.5 Player, inventory, crafting | `EntityPlayer`, `InventoryPlayer`, `Container*`, `Slot*`, `CraftingManager`, `Recipes*`, `FurnaceRecipes`, `PlayerControllerSP` | ~3.5k | 2.2, 1.2 | C: scripted player; every recipe against oracle dump |
| 2.6 Item behaviour ∥ | `Item*` use/attack logic, `ItemMap`, `MapData` | ~2.4k | 2.5 | C |
| 2.7 Stats and achievements ∥ | `Stat*`, `Achievement*` | ~1.0k | 2.5 | list dump |

**Gate 2:** a headless scripted session (generate a world, walk, dig, place,
craft, fight, sleep, save, reload) matches the oracle trace; a vanilla world
with entities and tile entities imports and ticks without divergence.

### Phase 3 — Bevy client (3.1–3.3 may start after Gate 0 against the skeleton)

| WP | Source | LOC | Depends |
|---|---|---|---|
| 3.1 App shell | `Minecraft.java` loop, `Timer`, input mapping, `GameSettings`, camera from `EntityRenderer`, native `Storage` | ~2.0k | 0.6 |
| 3.2 Texture atlas and animated textures | `RenderEngine`, `TextureFX` family | ~1.2k | 3.1 |
| 3.3 Terrain mesher (four tasks) | scaffolding from `minecraft_terrain`; `RenderBlocks` by render type: cubes and smooth lighting · fluids · plants, torches, fire, crops · redstone, rails, doors, stairs, pistons, bed and the rest; plus `WorldRenderer` sectioning | ~3.9k | 3.2, 1.3 |
| 3.4 Sky, clouds, weather, fog, selection box | `RenderGlobal`, parts of `EntityRenderer` | ~1.7k | 3.1 |
| 3.5 Entity models and renderers ∥ (two tasks) | `Model*` (20), `Render*` (34), tile-entity renderers | ~3.2k | 3.2, Phase 2 types |
| 3.6 GUI ∥ (three tasks) | font and widgets · in-game HUD, chat, inventory and container screens · menus, world select/create/**import**, options, achievements | ~5.0k | 3.1, 2.5, 1.9 |
| 3.7 Particles, hand and item rendering | `EntityFX` family, `EffectRenderer`, `ItemRenderer` | ~1.6k | 3.2 |
| 3.8 Audio | `SoundManager`, sound call sites surfaced as events; vanilla music and sound lists per the audit | ~0.3k | 3.1 |

GUI classes the port rewrote (`GuiCreateOrImport` and whatever else the audit
labels `differs` or `port-only`) are taken from the vanilla reference.

**Gate 3:** the game is playable from the title screen through world creation
or import, play, save and reload.

### Phase 4 — Vanilla parity and hardening

| WP | Deliverable |
|---|---|
| 4.1 | Side-by-side checklist against the Tier D reference: every block rendered, every screen, every mob, every sound trigger |
| 4.2 | Long-run soak: periodic world digests against Oracle V over tens of thousands of ticks on several seeds |
| 4.3 | Ledger closure: every `DEV-nnn` entry confirmed at evidence level 1 and resolved to vanilla, or explicitly accepted by the owner |
| 4.4 | Performance pass (chunk meshing, lighting, region I/O incl. incremental region writes), packaging |
| 4.5 | **World export (low priority):** "Export" action; worlds written by the Rust game opened in a real vanilla b1.7.3 client and checked |

**Gate 4 = vanilla parity on native.** WP-4.5 may slip past the gate.

### Phase 5 — Browser build

Compile `mc-client`/`mc-app` to wasm; browser `Storage` (IndexedDB or OPFS)
behind the same trait; asset loading over HTTP; audio start on first gesture;
single-threaded meshing budget; download size. The simulation crates already
compile for the target (D9), so this phase is client work.

### Phase 6 — Later, each gated on an owner decision

Multiplayer (4.5k LOC plus a protocol test server), texture packs, sky
dimension, pre-b1.3 world conversion.

---

## 6. Agent protocol

**Roles.** An *orchestrator* session holds this plan, issues work packages,
tracks the dependency table and merges. *Transcriber* agents each take one task
in its own git worktree and branch. A *reviewer* agent, given only the spec
sources and the diff (not the transcriber's reasoning), checks the diff against
the Java. The owner approves Gate reviews and anything touching the 0.6
skeleton's interfaces.

**Task brief** (the orchestrator fills this for every task):

```
Task:        WP id and name
Spec files:  port Java paths for classes the audit marks identical;
             reference/vanilla paths and DEV ids for classes that differ
             (the agent reads these in full before writing)
Rust target: module paths it may create or edit; everything else is read-only
May copy:    MinecraftOSS files from the D4 table, if any
Depends on:  merged interfaces it may call
Fixtures:    scenarios that must pass; scenarios it must add
Out of scope: deferred code it will meet (cite and skip)
Rules:       §3.4, D9 and the Definition of Done below
```

Until the audit exists, briefs name port paths only and the task report must
list anything in those files that looks hand-written rather than decompiled.

**Definition of Done.**
1. `cargo build`, `cargo clippy` and `cargo test -p <crate>` pass, including
   the wasm check for simulation crates.
2. Named fixtures pass; new behaviour has a new scenario, or the report says
   which behaviour is unverified and why.
3. Every non-obvious constant and ordering carries a `// src:` or `// vanilla:`
   citation.
4. Each renamed `field_NNN` / `func_NNN` is recorded in `docs/mappings.md` with
   the usage that justified the name.
5. No edits outside the task's Rust target. A needed interface change is
   reported to the orchestrator instead of made.
6. Copied code carries its origin header and only comes from the D4 table.
7. Report lists vanilla bugs transcribed, suspected port deviations, deferred
   code skipped, and anything uncertain.

**Scenario authorship.** Transcribers write `scenario.json`; the orchestrator
runs the oracles to produce `expected.jsonl`. An agent never edits an expected
file to make its own test pass.

**Concurrency.** Width is bounded by the "∥" markers: at most about seven
transcribers during WP-1.6, fewer elsewhere. Interface changes to `World`,
`Chunk` or the entity layout are serialised through the orchestrator.

**When a fixture fails and the Rust matches the Java on reading:** suspect the
harness (unseeded randomness, hash order, float formatting) or an unledgered
deviation before the port, and report it. Do not loosen the comparison.

---

## 7. Risks

| Risk | Mitigation |
|---|---|
| No vanilla jar, or the decompiled tree does not line up with the port's names | Project still runs on Oracle P; vanilla claims stay `unconfirmed`. WP-0.2 is tried first so this is known before Phase 1 fans out |
| Fixtures generated from Oracle P before V exists encode port behaviour | Gate 0 rule: V regenerates everything on arrival; disagreements block merges in the affected area |
| A deviation changes terrain (DEV-002 feeds caves and ores) after terrain work is merged | WP-1.4 and 1.7 isolate the sine table behind `mc_core::math`; flipping it is a constant plus regenerated fixtures |
| 26.3 behaviour leaks in through copied code | D4 "do not copy" list; reviewer checks origin headers; beta fixtures are the only acceptance |
| Region writing corrupts a world | Import copies, never edits in place; Tier S write/re-read; WP-4.5 validation in the real client; atomic write-then-rename in the native `Storage` |
| JVM `Math.sin`/`cos`/`pow` intrinsics differ from Rust in the last bit | WP-0.3 sweep decides early; fallbacks are running the oracle with intrinsics disabled, or a `libm`-based implementation matched to it |
| `HashSet` iteration order must be reproduced (R14) | `JavaHashSet` in `mc-core`, fixture with a known insertion sequence dumped from the oracle |
| Simulation picks up a file, thread or clock dependency that blocks Phase 5 | D9 lint lists and the wasm CI job from WP-0.1 |
| Interface churn in the `mc-sim` skeleton invalidates parallel work | Gate 0 owner review; one worked block and entity before fan-out |
| Rendering has no exact oracle | Transcribe data paths exactly; human side-by-side checklist in Phase 4 |
| `RenderBlocks` is one 3.4k-line file with shared state | Split by render type with the shared helpers landed first in the cubes task |
| Agents "improve" quirks | §3.4 rule 6, reviewer checks against Java, deviations ledger |
| Decompiled Mojang code and assets | D8 |

---

## 8. What is needed from the owner

No design decisions are open. These inputs unblock or strengthen specific work:

| # | Item | Needed for | If absent |
|---|---|---|---|
| 1 | **A vanilla Beta 1.7.3 client jar** (from your own launcher install), dropped in `reference/` | WP-0.2: reference tree, audit, Oracle V. This is what makes "vanilla parity" provable | Work continues on Oracle P; deviations are found only by inspection |
| 2 | One or two **real b1.7.3 worlds** you don't mind using as test data | Tier S import corpus | Oracle-written worlds only, which exercise fewer edge cases |
| 3 | **Credit text and link** for MinecraftOSS as the maintainer wants it shown | README credits in WP-0.1 | I will credit "MinecraftOSS (commit 4013a68), via github.com/chasmlol/2010-rust-rewrite-mashup, Apache-2.0" and you can correct it |
| 4 | Whether you can **run a real b1.7.3 client** on this machine | Tier D reference and WP-4.5 export validation | The `web/` build stands in for visuals; export stays unvalidated |
| 5 | Policy for imported worlds with **block ids newer than b1.7.3**: refuse (default), or load with unknown blocks replaced by air | WP-1.9 / 3.6 | Refuse with a message |
| 6 | Rust installed (`rustup`), and approval to `git init` this directory | WP-0.1 | Nothing can be built |
