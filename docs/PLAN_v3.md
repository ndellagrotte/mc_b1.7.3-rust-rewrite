# PLAN_v3 — Rust/Bevy rewrite of Minecraft Beta 1.7.3 (master plan)

**Status:** v3, 2026-10-05. Supersedes `PLAN_v2.md`, `PLAN_v1.md`,
`RUST_REWRITE_BLUEPRINT.md` and `docs/ADR-007-golden-harness.md` wherever they
conflict. Agents follow this document.

**Goal:** a native Rust + Bevy game with **vanilla Minecraft Beta 1.7.3
singleplayer parity**, proven by fixtures generated from the vanilla Java code.
Then a browser build.

**What changed from v2.** The repository was restructured and now contains, under
`.reference/`, the reconstructed vanilla b1.7.3 source (client and server), the
web port that was previously the only Java here, the MinecraftOSS mashup, and a
reference McRegion world. A real b1.7.3 client is installed in Prism Launcher.
This removes the biggest weakness of v2: the spec is no longer inferred from a
modified port, it is the vanilla source, and it compiles and runs headless on
this machine (§1). Consequences:

| Area | v2 | v3 |
|---|---|---|
| Spec and transcription source | Web port's Java, corrected by a deviation ledger | **Vanilla client source** in `.reference/mc_b1.7.3_release`. The web port is no longer read by transcribers |
| Oracle | Two oracles (vanilla if a jar appeared, port otherwise) and an agreement check | **One oracle**, built from the vanilla source with `javac`. Verified working headless |
| Deviation audit / ledger | Core deliverable of Phase 0 | Dropped. Done once here (§1.3) to justify the switch; no longer needed |
| Saves | McRegion, details "from the vanilla reference when available" | McRegion, transcribed from `RegionFile`, `McRegionChunkLoader`, `SaveOldDir`; reference world loads headless today |
| Assets | Web port's `resources/` | Textures and language files from the user's own `minecraft.jar`; sounds and music from the Prism instance's `resources/` directory |
| Visual reference (Tier D) | Web build, or a real client "if the owner can run one" | The Prism instance (Java 8, LWJGL 2.9.4) and, optionally, the vanilla client run from source via MCP |
| Repository layout | `rust/` subdirectory | Cargo workspace at the repository root; `.reference/` is read-only input |

Contents: 1 What was verified in the new layout · 2 Decisions · 3 Architecture ·
4 Oracle and fixtures · 5 Phases and work packages · 6 Agent protocol ·
7 Risks · 8 Owner items

---

## 1. What was verified in the new layout

### 1.1 Repository

```
Beta-1.7.3-main/                      ← the rewrite project (currently: plans, docs/, .reference/)
├─ PLAN_v1.md, PLAN_v2.md, PLAN_v3.md, RUST_REWRITE_BLUEPRINT.md
├─ docs/ADR-007-golden-harness.md
└─ .reference/                        ← read-only inputs, never edited by agents
   ├─ mc_b1.7.3_release/1.7.3-LTS/    ← vanilla, MCP-deobfuscated (github.com/jacobo-mc/mc_b1.7.3_release)
   │   ├─ src/minecraft/              678 files, 66,925 LOC (client)
   │   ├─ src/minecraft_server/       444 files, 41,777 LOC (server; Phase 6 material)
   │   ├─ jars/bin/                   minecraft.jar, minecraft_server.jar, lwjgl 2 jars + Linux natives
   │   ├─ conf/                       MCP mappings (4,482 fields, 3,509 methods), patches
   │   └─ runtime/                    MCP scripts (decompile / recompile / startclient), need Java 8
   ├─ mc_b1.7.3-web-port/             the TeaVM port audited in PLAN_v1 (632 logic files, 60.5k LOC)
   ├─ 2010-rust-rewrite-mashup-main/  MinecraftOSS crates + Bevy 0.19 terrain renderer (D4)
   └─ mc_b1.7.3-world/testWorld/      McRegion world, seed 123, 4 region files, 3.6 MB
Prism instance: ~/.var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher/instances/b1.7.3/
   minecraft/saves/testWorld          identical to .reference/mc_b1.7.3-world/testWorld
   minecraft/resources/               music, newmusic, newsound, sound, sound3, streaming (audio assets)
   Java: jre-legacy 1.8.0_202; LWJGL 2.9.4-nightly-20150209
```

The project is still not a git repository and Rust is still not installed.

### 1.2 The vanilla source compiles and runs headless (tested)

- `javac` on the installed JDK 25 compiles all 678 client files with zero
  errors against `jars/bin/{lwjgl,lwjgl_util,jinput,minecraft}.jar`
  (`minecraft.jar` supplies the bundled `paulscode` sound classes).
- Singleplayer constructs plain `World` (`Minecraft.java:1199`); the tick order
  recorded in PLAN_v2 §3.3 is unchanged in vanilla (`Minecraft.java:1160-1165`,
  `World.java:1745-1780`).
- `StringTranslate` loads language files through the class loader, so **no GL
  stub is needed**; `Block`/`Item`/`StatList` static initialisation runs
  headless with `minecraft.jar` on the classpath for `/lang/*.lang`.
- Headless probe, seed 123: `ChunkProviderGenerate.provideChunk` for three
  chunks gives identical CRCs across two independent worlds; a world ticks 100
  times without a player.
- **The reference world loads through vanilla's own save code.**
  `new World(new SaveOldDir(savesDir, "testWorld", false), …)` reports seed 123,
  time 1751, spawn 21,64,−64; chunk (0,−4) comes back populated from
  `region/r.0.-1.mcr`; a `TileEntityChest` at **9,64,−61** holds 2 planks (id 5)
  and 4 sticks (id 280), with the workbench at 9,64,−62. (`world_info.txt` says
  9,65,−60 / 9,65,−61; those are the player's F3 coordinates, off by one.)
  Note: `SaveHandler` is the pre-McRegion handler and silently generates fresh
  chunks; `SaveOldDir` (returned by `SaveConverterMcRegion.getSaveLoader`) is the
  one that reads region files. Opening a world rewrites `level.dat` and
  `session.lock`, so the oracle always works on a copy.
- Vanilla uses `java.util.Random` and a 65,536-entry sine table with factor
  `10430.378F` (`MathHelper.java:4-11`). Both v2 ledger entries DEV-001 and
  DEV-002 are therefore confirmed as web-port deviations.

### 1.3 Why the web port is dropped as a source

A normalised class-by-class diff (comments, whitespace and imports stripped)
between the web port and vanilla: 188 classes identical, 438 differ, 7
port-only, 51 vanilla-only. Most differences are decompiler style (ternaries,
field initialisers, boxing) and the port's **older MCP names** — vanilla has
`headRotationCourse`, `rayTraceBlocks_do_do`, `setEntityState` where the port
has `field_25048_b`, `func_28105_a`, `func_9425_a` — but real behaviour changes
are mixed in and cannot be told apart textually. The decisive test:
**the same seed generates different terrain.** Seed 123, chunks (−1..1, −4):

```
vanilla  4fb6e2d2 92443bf2 86323733
web port 152adba4 d8f3dd13 63a93c5a
```

So the port is not a usable spec for vanilla parity, and the vanilla tree is
better named, complete (server included), and runnable. Everything PLAN_v1
found about the port (singleplayer on `World`, the GL-reachability problem, the
custom `Random`, the save layout) remains true of the port and is now simply
irrelevant. The port is kept in `.reference/` only as a worked example of a
browser build of this game (Phase 5).

### 1.4 Carry-overs that still hold

- Design findings from PLAN_v1 §1.1: Bevy query order is not insertion order
  (R5); Java overrides mostly call `super` (R6); `World.positionsToUpdate` is a
  `HashSet` iterated for random-tick chunk order (R14); 14 gameplay sites call
  `Math.random()` directly (R15, re-check against vanilla in WP-0.4); entities
  need chunks within ±32 blocks to tick; random ticks need a player.
- MinecraftOSS facts and the reuse table (PLAN_v2 D4), with the licence now
  confirmed by the owner.
- Mapping names in the vanilla tree are MCP-lineage, and the LTS author notes
  some were invented ("* = I made this name up" in `docs/LTS-Changelog.log`).
  Names are fine for Rust identifiers; citations use file and line, never the
  name alone.

---

## 2. Decisions

**D1 — Simulation is a plain-Rust core; Bevy is the shell.** (owner, confirmed)
`World`, chunks, blocks and entities live in ordinary Rust structs in crates with
no Bevy dependency. Bevy owns the window, input, rendering, UI, audio and assets,
and drives the simulation from one system. Reasons: R5 and R6; Java call chains
re-enter freely inside one stack frame; entity-list order is gameplay-visible.
From the original ADR-001 only rule 5 (closed `EntityKind` enum with match
tables) and rule 6 (models as data tables) survive.

**D2 — The spec is the vanilla client source in
`.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft`.** (owner, confirmed)
Transcribers read only that tree. Citations are `// src: EntityWolf.java:86`
and resolve against it. Vanilla bugs are transcribed and flagged, never fixed.
Out of scope even though vanilla has it: anything depending on defunct online
services (login, skin and cloak downloads, resource downloads, stats sync).

**D3 — Scope order.** (1) Native singleplayer with vanilla parity. (2) Browser
build. (3) Later, each gated on an owner decision: multiplayer (the server tree
is now available for it), texture packs, sky dimension, pre-McRegion world
conversion. Agents that meet out-of-scope code cite it, skip it and tag the
task `deferred`.

**D4 — MinecraftOSS code may be copied, with attribution.** (owner, confirmed)

| Source (under `.reference/2010-rust-rewrite-mashup-main/`) | Use | Notes |
|---|---|---|
| `third_party/minecraftoss/core/src/random.rs` — `LegacyRandom` incl. `next_gaussian` | Adapt into `mc_core::java_random` | Drop the Xoroshiro and positional halves. Must pass fixtures generated from `java.util.Random` |
| `…/core/src/nbt.rs` (372 lines) | Adapt into `mc-nbt` | Reject `IntArray`/`LongArray` on read, never write them (b1.7.3 has tags 0–10). Compounds are `BTreeMap`, so write order differs from Java; compare parsed trees (§4.3) |
| `…/core/src/anvil.rs` (186 lines) | Adapt into the McRegion container | Same sector/header layout as `.mcr`. Rewrites whole region files; verify against `RegionFile.java` semantics, revisit incremental writes in Phase 4 |
| Workspace lint settings (`excessive_precision`, `approx_constant` allowed) | Copy | Keeps transcribed float literals intact |
| `crates/minecraft_terrain/src/{mesh,sections,lighting,texture_mips}.rs` | Scaffolding for WP-3.3 | Vertex layout, section meshing jobs, light-to-shader plumbing. Which faces, textures and light values to emit come from `RenderBlocks.java` |
| Trace-binary / scenario-replay layout | Pattern | Informs `mc-golden` |
| `generator/`, `entities/`, `world/level/`, `player/` | **Do not copy** | Minecraft 26.3 behaviour |

Rules: each adapted file carries a header
(`// Adapted from MinecraftOSS <path> @ 4013a68, Apache-2.0; modified`); the
repository gets a `NOTICE` entry, the Apache-2.0 text, and a credits section in
`README.md` as the maintainer asked. Copied code is presumed 26.3-specific until
the beta fixtures pass.

**D5 — The native save format is vanilla McRegion.** (owner, confirmed)
Transcribed from `RegionFile`, `RegionFileCache`, `McRegionChunkLoader`,
`ChunkLoader` (chunk NBT), `SaveOldDir`, `SaveFormatOld`, `WorldInfo`.
Import (priority) copies a vanilla save folder into the saves directory and
opens it; unknown block or item ids are refused with a message (§8). Export is
planned, low priority: an "Export" action plus validation in the Prism client
(WP-4.5). Not supported: the web port's save layout and EPK archives; pre-McRegion
worlds (deferred, D3).

**D6 — Toolchain.** Stable Rust pinned in `rust-toolchain.toml`. Bevy 0.19, the
version the copyable scaffolding targets; the orchestrator may pick a newer
stable release at kickoff if that scaffolding ports cleanly, and it is then not
upgraded again before Gate 4. The oracle builds with `javac` from the installed
JDK via a shell script; Java 8 from the Prism instance is available for MCP's
own scripts if the from-source client is ever needed.

**D7 — Rendering parity is visual, not structural.** The Java renderer is
fixed-function immediate-mode GL. Game-data parts are transcribed (faces,
texture indices, colours, light values, model boxes); the drawing machinery is
written for Bevy.

**D8 — Mojang material stays local and out of git.** `.reference/` is
git-ignored; `docs/REFERENCES.md` records what goes there and where it came
from. Textures (`terrain.png`, `gui/`, `mob/`, `font/`, …) and `lang/` files are
read at runtime from the user's own `minecraft.jar` (path configurable, default
the one in `.reference/…/jars/bin/`); sound and music from a `resources/`
directory (default: the Prism instance's). Nothing of Mojang's is embedded in
the binary or committed.

**D9 — Browser readiness from day one.** (owner, confirmed) `mc-core`,
`mc-nbt` and `mc-sim` use no `std::fs`, `std::thread`,
`std::time::{Instant,SystemTime}` or environment access. Storage goes through a
`Storage` trait (byte-blob read, write, list, rename, delete); time is passed
in. Enforced by clippy `disallowed-types`/`disallowed-methods` and a CI job
running `cargo check --target wasm32-unknown-unknown` on those crates.

---

## 3. Architecture

### 3.1 Repository and workspace layout

```
Cargo.toml                 workspace
rust-toolchain.toml, clippy.toml, .cargo/config.toml
crates/
  mc-core/  mc-nbt/  mc-sim/  mc-golden/  mc-client/  mc-app/
tools/oracle/              Java harness sources, build + run scripts (§4.1)
tests/golden/<area>/<name>/{scenario.json, expected.jsonl}
docs/                      ADRs, mappings.md, REFERENCES.md, AUDIT notes
.reference/                git-ignored inputs (§1.1)
```

| Crate | Depends on | Owns | Vanilla source |
|---|---|---|---|
| `mc-core` | — | `JavaRandom`, `MathHelper` (65,536-entry sine table), AABB, `Vec3D`, ray-trace result, `JavaHashSet`/`JavaHashMap` order emulation | `MathHelper`, `AxisAlignedBB`, `Vec3D`, `MovingObjectPosition`; RNG adapted per D4 |
| `mc-nbt` | — | NBT tags 0–10, big-endian codec, gzip and zlib wrappers | adapted per D4; checked against `NBT*`, `CompressedStreamTools` |
| `mc-sim` | core, nbt | Everything that ticks: block and item registries and behaviour, world, chunks, lighting, worldgen, tile entities, entities, AI, inventory, crafting, stats; McRegion and `level.dat` codecs over `Storage` | all remaining non-render, non-GUI, non-net classes |
| `mc-golden` | sim | Scenario runner and trace writer; dev-dependency only | — |
| `mc-client` | sim, bevy | Bevy plugins: tick driver, native `Storage`, jar/resources asset loading, mesher, entity and sky rendering, particles, GUI, audio, input, settings | `Render*`, `Model*`, `Gui*`, `EntityRenderer`, `RenderGlobal`, `EffectRenderer`, `SoundManager`, `GameSettings`, `TexturePackDefault` (asset paths only) |
| `mc-app` | client | Binary; thin | `Minecraft.java` |

`mc-sim` is one crate on purpose: blocks call the world, the world calls blocks,
entities call both, items place blocks and spawn entities. Modules inside it
(`block/`, `item/`, `world/`, `gen/`, `entity/`, `inventory/`, `save/`) are the
ownership units for agents. `gen/` is the clean piece to split out if compile
times hurt.

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
  WP-0.5 and used everywhere.
- **Side effects for the client** (sounds, particles, block-changed
  notifications) go through an `IWorldAccess`-equivalent event queue that
  `mc-client` drains each frame. The simulation never calls into Bevy.
- **Single-threaded and synchronous**, like the Java. Background work (chunk
  meshing) belongs to the client and uses Bevy's task pool, which degrades to
  one thread in the browser.

### 3.3 Tick order (vanilla `Minecraft.runTick` → `World.updateEntities`, `World.tick`)

```
per game tick, when not paused:
  GUI / input handling, player controller update
  entityRenderer.updateRenderer, renderGlobal.updateClouds      (client state)
  world.updateEntities():
      weather effects → loadedEntityList in order (removals in place)
      → loadedTileEntityList → pending tile-entity adds
  world.tick():
      updateWeather → sleep check → SpawnerAnimals.performSpawning
      → chunk unload → skylight recalculation → worldTime + 1, autosave
      → TickUpdates (scheduled) → updateBlocksAndPlayCaveSounds (random ticks, 80/chunk)
  world.randomDisplayUpdates(player position)                    (client particles)
  effectRenderer.updateEffects()
```

The tick driver is a port of `Timer.java` (20 TPS, at most 10 ticks per frame,
partial tick for interpolation) run from a single Bevy system in `Update`, with
the clock value passed in by the client.

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
5. `Math.sin/cos/atan2/pow/exp/log` go through `mc_core::math`. WP-0.3 decides
   the implementation from a bit-comparison sweep against the oracle JVM; the
   sine table itself is a fixture dumped from the oracle, not recomputed.
6. Vanilla bugs are transcribed and flagged, never fixed.

### 3.5 Client notes

- **Assets:** `mc-client` opens `minecraft.jar` as a zip at startup and reads
  textures and `lang/` from it; `resources/` supplies `.ogg` sounds and music.
  Both paths are settings with the defaults from D8. Missing files produce a
  clear startup error naming the expected path.
- **Terrain:** 16×16×16 sections as in `WorldRenderer`, meshed on Bevy's task
  pool from chunk snapshots, drawn with a custom unlit material (vertex colour ×
  brightness table, alpha test, nearest filtering, distance fog) in two passes.
  Scaffolding from `minecraft_terrain` (D4).
- **GUI:** Java screens draw in scaled pixel coordinates with immediate-mode
  calls (`drawTexturedModalRect`, `drawString`). Default approach is a small 2D
  quad batcher with that same API, so `Gui*` classes transcribe nearly line for
  line. WP-3.6 begins with a short spike confirming this against `bevy_ui`.
- **Animated textures** (`TextureFX` subclasses) are procedural and are
  transcribed, writing into the atlas.
- **Audio:** `bevy_audio`; sound names and volume/pitch arithmetic transcribed
  from the call sites; `SoundManager`'s random music timing ported.
- **World import** is a client screen that copies a chosen folder through the
  native `Storage` after validating `level.dat` and block ids.

---

## 4. Oracle and fixtures

### 4.1 Build recipe (verified)

```
V=.reference/mc_b1.7.3_release/1.7.3-LTS
CP="$V/jars/bin/lwjgl.jar:$V/jars/bin/lwjgl_util.jar:$V/jars/bin/jinput.jar:$V/jars/bin/minecraft.jar"
javac -nowarn -proc:none -encoding UTF-8 -cp "$CP" -d build/oracle \
      $(find build/oracle-src tools/oracle/java -name '*.java')
java -cp "build/oracle:$CP" harness.Main tests/golden build/golden-out
```

`build/oracle-src` is a generated copy of `src/minecraft` with the single
scripted rewrite from §4.2 applied. `minecraft.jar` must stay last on the
classpath so compiled vanilla classes shadow the obfuscated originals while its
`lang/` and `paulscode` contents remain reachable. No GL stub is required.

### 4.2 Harness rules

- Worlds are `World` with `SaveHandlerMP` (no disk) or `SaveOldDir` on a
  **copy** of a save (opening a world rewrites `level.dat` and `session.lock`).
  Never `SaveHandler` (pre-McRegion; generates instead of loading) and never
  `WorldClient`.
- The oracle compiles from a generated copy in which a checked-in script
  rewrites `Math.random()` to `HarnessMath.random()` (a seeded stream) and
  nothing else. No hand edits to vanilla code; the script's diff is reviewed
  once. WP-0.4 re-counts the call sites in the vanilla tree.
- After constructing a world, the harness seeds `World.rand` and the
  random-tick LCG state; every entity's `rand` is seeded from
  `(scenario seed, entityId)` on construction. `Entity.nextEntityID` is reset
  per scenario.
- Entities are constructed directly (`new EntityPig(world)`), not through
  `EntityList`. Scenarios that need ticking preload chunks within 32 blocks;
  scenarios that need random ticks or mob spawning add a
  `HarnessPlayer extends EntityPlayer`.
- Save fixtures: the oracle writes McRegion worlds to a temp directory through
  `SaveOldDir`; those plus `testWorld` are the import corpus.

### 4.3 Tiers

| Tier | Covers | Compared by |
|---|---|---|
| A | RNG sequences (generated from `java.util.Random` directly), sine table dump, `Math.*` sweep, NBT primitives, terrain generation and population per chunk, biome arrays | exact bytes / digest |
| B | Lighting, scheduled and random ticks, fluids, fire, plants, redstone, pistons, rails, tile entities, explosions | per-tick digests and block dumps, exact |
| C | Entity physics, collisions, AI, projectiles, vehicles, item entities, damage, inventory and crafting results | per-tick field traces, floats as bit patterns, exact |
| S | Saves: read oracle-written worlds and `testWorld`; write, re-read, compare; cross-read (Rust writes, oracle reads, and the reverse) | parsed NBT trees and chunk arrays equal. Not raw bytes: Java writes compound keys in `HashMap` order and gzip/zlib output differs by implementation |
| D | Rendering, GUI, audio, feel | side-by-side against the Prism client on `testWorld` and fixed seeds; human-reviewed. Optional later: the client run from source via MCP with scripted camera positions |

Traces use ADR-007 §5's formats: one scenario file drives both sides, floats
serialise as IEEE-754 bit patterns, digests are over canonical byte arrays.
Fixtures are checked in under `tests/golden/`; ordinary CI runs only the Rust
side. A `golden-regen` job rebuilds them from the oracle on demand; drift
without a harness change is a determinism bug, investigated, never
force-pushed.

### 4.4 Harness acceptance

1. Tier A regenerates byte-identically across three runs.
2. Sensitivity check: perturbing one RNG seed bit, or one block in a scenario,
   fails the matching fixture.
3. A failing comparison prints the first differing tick and field with both
   values decoded.
4. `testWorld` loads in the Rust port with chunk digests equal to what the
   oracle reads from the same files, and the chest at 9,64,−61 holds 2 planks
   and 4 sticks.

---

## 5. Phases and work packages

LOC figures are vanilla Java lines to transcribe. "∥" marks packages that can
run concurrently once their dependencies are merged. Packages above roughly
1.5k LOC are split by the listed sub-units, one agent task each.

Subsystem sizes in the vanilla client (675 files, 65.2k LOC in `src/`, plus
`Minecraft.java` 1,553): rendering 78 files / 12.5k (`RenderBlocks.java` 3,380);
blocks 82 / 8.8k; entities 36 classes / 8.4k; world core 26 / 5.7k; worldgen
48 / 5.0k; GUI 50 / 4.9k; networking 72 / 4.6k (deferred); items, crafting,
inventory 69 / 3.6k; NBT and saves 27 / 1.9k; JSON library 47 / 1.8k (replaced
by `serde_json`); particles 18 / 1.1k; stats and achievements 14 / 1.0k; core
math 4 / 0.6k; sound 5 / 0.3k.

### Phase 0 — Foundations (serial except where marked; Phase 1 starts after 0.5)

| WP | Deliverable |
|---|---|
| 0.1 | `git init`; `.gitignore` (`.reference/`, `build/`, `target/`); `docs/REFERENCES.md`; install rustup; workspace at the root with lint settings per D4; clippy disallow-lists and wasm `cargo check` job per D9; CI for `cargo test` and `clippy`; `NOTICE`, Apache-2.0 text and README credits per D4 |
| 0.2 | `tools/oracle`: build script (§4.1), `Math.random` rewrite script, `harness.Main`, Tier A dumps; `mc-golden` runner; fixtures for RNG (from `java.util.Random`), sine table, `Math.*` sweep, three terrain seeds × three chunks × overworld and nether |
| 0.3 | `mc-core` and `mc-nbt` adapted from MinecraftOSS and green against 0.2; §3.4 rule 5 decided |
| 0.4 ∥ | Tier B/C/S scaffolding: seeding protocol, `HarnessPlayer`, chunk preload, scenario ops (`set_block`, `spawn`, `set_motion`, `use_item`, `save`, …), `SaveOldDir` temp-world writer, `Math.random()` site census in the vanilla tree |
| 0.5 | `mc-sim` skeleton, owner-reviewed: `World`, `Chunk`, block table and dispatch, entity struct layout and the take/put-back pattern, event queue, `Storage` trait, module layout, one block and one entity implemented end to end as the pattern to copy; `docs/mappings.md` started |

**Gate 0:** §4.4 items 1–3 pass; owner approves the 0.5 skeleton.

### Phase 1 — World without entities

| WP | Vanilla source | LOC | Depends | Gate |
|---|---|---|---|---|
| 1.1 Block data table | `Block.java` static init, `Material*`, `StepSound*` | ~0.9k | 0.5 | oracle dump of every block's fields |
| 1.2 Item data table ∥ | `Item.java` static init, tool/armour enums | ~0.6k | 0.5 | oracle dump of every item's fields |
| 1.3 Chunk storage and lighting | `Chunk`, `NibbleArray`, `MetadataChunkBlock`, light paths in `World` | ~1.5k | 0.5 | B: light arrays after scripted edits |
| 1.4 Noise and terrain shape ∥ | `NoiseGenerator*`, `WorldChunkManager*`, `BiomeGen*`, `ChunkProviderGenerate/Hell.provideChunk`, `MapGenCaves*` | ~2.3k | 0.3 | A: unpopulated chunk digests |
| 1.5 World block access and ticks | remainder of `World` block get/set, neighbour notification, scheduled ticks, random ticks, weather, time | ~1.8k | 1.1, 1.3 | B: tick traces |
| 1.6 Block behaviour ∥ (seven tasks) | fluids · fire, plants, crops, leaves, snow/ice · redstone wire, torches, repeaters, buttons, levers, plates · pistons · rails (`RailLogic`) · containers, doors, beds, signs and other interactables · simple remainder | ~7.5k | 1.5 | B: one scenario set per group |
| 1.7 Population features ∥ | `WorldGen*`, `populate` in both providers | ~2.4k | 1.5, 1.6 simple blocks | A: populated chunk digests |
| 1.8 Tile entities and explosions | `TileEntity*` (7 logic kinds), `Explosion` | ~1.0k | 1.5 | B |
| 1.9 Saves (two tasks) | container: `RegionFile`, `RegionFileCache` (adapted from `anvil.rs`) · `McRegionChunkLoader`, `ChunkLoader` chunk NBT, `SaveOldDir`, `SaveFormatOld`, `WorldInfo`, `level.dat`, `session.lock`, `ChunkProvider*` caching | ~1.9k | 1.3, 1.8 | S: `testWorld` and oracle-written worlds load; write/re-read and cross-read equality |

**Gate 1:** all Tier A, B and S fixtures green; §4.4 item 4 passes.

### Phase 2 — Entities and player

| WP | Vanilla source | LOC | Depends | Gate |
|---|---|---|---|---|
| 2.1 Base entity physics | `Entity.java` (`moveEntity`, water/lava/fire, riding) | ~1.0k | 1.5 | C: falling, stepping, collision traces |
| 2.2 Living base and pathfinding | `EntityLiving`, `EntityCreature`, `Path*`, `Pathfinder` | ~1.5k | 2.1 | C |
| 2.3 Mobs ∥ (three tasks) | animals and wolf · monsters, slime, ghast · squid, spawn rules (`SpawnerAnimals`) | ~2.6k | 2.2 | C per mob |
| 2.4 Non-living entities ∥ (two tasks) | item, falling sand, TNT, painting, projectiles, fishing hook, lightning · minecart and boat | ~2.8k | 2.1 | C |
| 2.5 Player, inventory, crafting | `EntityPlayer`, `EntityPlayerSP` (non-client parts), `InventoryPlayer`, `Container*`, `Slot*`, `CraftingManager`, `Recipes*`, `FurnaceRecipes`, `PlayerControllerSP` | ~3.6k | 2.2, 1.2 | C: scripted player; all 151 recipes against oracle dump |
| 2.6 Item behaviour ∥ | `Item*` use/attack logic, `ItemMap`, `MapData` | ~2.4k | 2.5 | C |
| 2.7 Stats and achievements ∥ | `Stat*`, `Achievement*` (local only; `StatsSyncher` deferred per D2) | ~1.0k | 2.5 | list dump |

**Gate 2:** a headless scripted session (generate a world, walk, dig, place,
craft, fight, sleep, save, reload) matches the oracle trace; `testWorld` ticks
for 6,000 ticks with periodic digests equal to the oracle's.

### Phase 3 — Bevy client (3.1–3.3 may start after Gate 0 against the skeleton)

| WP | Vanilla source | LOC | Depends |
|---|---|---|---|
| 3.1 App shell | `Minecraft.java` loop, `Timer`, input mapping, `GameSettings`, camera from `EntityRenderer`, native `Storage`, jar and `resources/` asset access | ~2.2k | 0.5 |
| 3.2 Texture atlas and animated textures | `RenderEngine`, `TextureFX` family, `TexturePackDefault` paths | ~1.3k | 3.1 |
| 3.3 Terrain mesher (four tasks) | scaffolding from `minecraft_terrain`; `RenderBlocks` by render type: cubes and smooth lighting · fluids · plants, torches, fire, crops · redstone, rails, doors, stairs, pistons, bed and the rest; plus `WorldRenderer` sectioning | ~4.0k | 3.2, 1.3 |
| 3.4 Sky, clouds, weather, fog, selection box | `RenderGlobal`, parts of `EntityRenderer` | ~1.8k | 3.1 |
| 3.5 Entity models and renderers ∥ (two tasks) | `Model*`, `Render*`, tile-entity renderers | ~3.4k | 3.2, Phase 2 types |
| 3.6 GUI ∥ (three tasks) | font and widgets · in-game HUD, chat, inventory and container screens · menus, world select/create/**import**, options, achievements | ~4.9k | 3.1, 2.5, 1.9 |
| 3.7 Particles, hand and item rendering | `EntityFX` family, `EffectRenderer`, `ItemRenderer` | ~1.6k | 3.2 |
| 3.8 Audio | `SoundManager`, `SoundPool`, sound call sites surfaced as events | ~0.3k | 3.1 |

**Gate 3:** the game is playable from the title screen through world creation
or import of `testWorld`, play, save and reload.

### Phase 4 — Vanilla parity and hardening

| WP | Deliverable |
|---|---|
| 4.1 | Side-by-side checklist against the Prism client: every block rendered, every screen, every mob, every sound trigger, on `testWorld` and fixed seeds |
| 4.2 | Long-run soak: periodic world digests against the oracle over tens of thousands of ticks on several seeds |
| 4.3 | Performance pass (chunk meshing, lighting, incremental region writes), packaging |
| 4.4 | Deferred-tag review: every `deferred` citation is still out of scope under D3, or scheduled |
| 4.5 | **World export (low priority):** "Export" action; worlds written by the Rust game opened in the Prism client and checked |

**Gate 4 = vanilla parity on native.** WP-4.5 may slip past the gate.

### Phase 5 — Browser build

Compile `mc-client`/`mc-app` to wasm; browser `Storage` (IndexedDB or OPFS)
behind the same trait; asset loading over HTTP from a user-supplied jar and
resources (never hosted by us); audio start on first gesture; single-threaded
meshing budget; download size. The web port in `.reference/` is the worked
example of the same game in a browser. The simulation crates already compile
for the target (D9), so this phase is client work.

### Phase 6 — Later, each gated on an owner decision

Multiplayer (client protocol 4.6k LOC; the vanilla server tree is available for
a test server or an integrated one), texture packs, sky dimension, pre-McRegion
world conversion (`SaveConverterMcRegion`).

---

## 6. Agent protocol

**Roles.** An *orchestrator* session holds this plan, issues work packages,
tracks the dependency table and merges. *Transcriber* agents each take one task
in its own git worktree and branch. A *reviewer* agent, given only the vanilla
source and the diff (not the transcriber's reasoning), checks the diff against
the Java. The owner approves Gate reviews and anything touching the 0.5
skeleton's interfaces.

**Task brief** (the orchestrator fills this for every task):

```
Task:        WP id and name
Spec files:  paths under .reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft
             (the agent reads these in full before writing)
Rust target: module paths it may create or edit; everything else is read-only
May copy:    MinecraftOSS files from the D4 table, if any
Depends on:  merged interfaces it may call
Fixtures:    scenarios that must pass; scenarios it must add
Out of scope: deferred code it will meet (cite and skip)
Rules:       §3.4, D9 and the Definition of Done below
```

**Definition of Done.**
1. `cargo build`, `cargo clippy` and `cargo test -p <crate>` pass, including
   the wasm check for simulation crates.
2. Named fixtures pass; new behaviour has a new scenario, or the report says
   which behaviour is unverified and why.
3. Every non-obvious constant and ordering carries a `// src:` citation into
   the vanilla tree.
4. Each renamed `field_NNN` / `func_NNN` is recorded in `docs/mappings.md` with
   the usage that justified the name.
5. No edits outside the task's Rust target, and never inside `.reference/`. A
   needed interface change is reported to the orchestrator instead of made.
6. Copied code carries its origin header and only comes from the D4 table.
7. Report lists vanilla bugs transcribed, deferred code skipped, and anything
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
| JVM `Math.sin`/`cos`/`pow` intrinsics differ from Rust in the last bit | WP-0.2 sweep decides early; the sine table is a dumped fixture; fallbacks are running the oracle with intrinsics disabled, or a `libm`-based implementation matched to it |
| `HashSet` iteration order must be reproduced (R14) | `JavaHashSet` in `mc-core`, fixture with a known insertion sequence dumped from the oracle |
| 26.3 behaviour leaks in through copied code | D4 "do not copy" list; reviewer checks origin headers; beta fixtures are the only acceptance |
| Region writing corrupts a world | Import copies, never edits in place; Tier S write/re-read and cross-read; atomic write-then-rename in the native `Storage`; WP-4.5 validation in the Prism client |
| Simulation picks up a file, thread or clock dependency that blocks Phase 5 | D9 lint lists and the wasm CI job from WP-0.1 |
| Interface churn in the `mc-sim` skeleton invalidates parallel work | Gate 0 owner review; one worked block and entity before fan-out |
| Rendering has no exact oracle | Transcribe data paths exactly; human side-by-side checklist in Phase 4 against the real client |
| `RenderBlocks` is one 3.4k-line file with shared state | Split by render type with the shared helpers landed first in the cubes task |
| Agents "improve" quirks | §3.4 rule 6, reviewer checks against Java |
| Agents read the web port by mistake (same class names, different behaviour) | Briefs give absolute paths; reviewer rejects citations outside the vanilla tree; the terrain-digest difference in §1.3 is the standing reminder |
| Decompiled Mojang code and assets in the tree | D8: `.reference/` git-ignored, nothing embedded, repository private |

---

## 8. Owner items

No design decisions are open. Remaining inputs:

| # | Item | Needed for |
|---|---|---|
| 1 | Rust installed (`rustup`) and approval to `git init` this directory with `.reference/` ignored | WP-0.1; nothing builds until then |
| 2 | Credit text and link for MinecraftOSS as the maintainer wants it shown | README in WP-0.1. Default: "MinecraftOSS (commit 4013a68), via github.com/chasmlol/2010-rust-rewrite-mashup, Apache-2.0" |
| 3 | Policy for imported worlds with block ids newer than b1.7.3: refuse (default) or load with unknown blocks as air | WP-1.9 / 3.6 |
| 4 | Whether `.reference/mc_b1.7.3_release` should be updated from its upstream or frozen at the current checkout (recommended: frozen; record the commit in `docs/REFERENCES.md`) | Stable citations |
