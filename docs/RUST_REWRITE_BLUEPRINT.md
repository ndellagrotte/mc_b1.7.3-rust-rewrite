# Rust/Bevy Rewrite Blueprint — Minecraft Beta 1.7.3

Session-zero artifacts for the agentic rewrite. Two parts:

- **Part I** — workspace & crate layout
- **Part II** — ADR-001: entity archetypes

Plus Part III (verification contract) and Part IV (queued ADRs).

Source of truth is the Java codebase in `src/main/java/net/minecraft/src` (~60.5k LOC,
632 files). The TeaVM/LWJGL shims (`src/lwjgl/java`, `net/PeytonPlayz585`) are
**dead weight** — never port them; Bevy/wgpu replaces all of it.

---

## Part 0 — Agent contract (read before touching any module)

These rules override your defaults/habits. They exist because the goal is
**behavioral parity**, not idiomatic Rust.

1. **Verbatim constants.** Magic numbers are sacred. Transcribe them exactly with
   a source citation comment:
   `// src: net/minecraft/src/EntityLiving.java:142`
   Never round, never "fix", never replace `0.9800000190734863D` with `0.98`.
2. **Field/method mapping.** Java field → Rust field with the same name unless it
   is an unmapped `field_9365_p`-style name. For unmapped names, infer a semantic
   name and record the mapping in `docs/mappings.md`:
   `moveStrafing <- field_9365_p` (EntityLiving.java:8). Never drop one silently.
3. **Types.** Java `double` → `f64`, `float` → `f32`, `int` → `i32`,
   `long` → `i64`. No implicit narrowing, no `f32` where Java used `f64` —
   physics parity depends on it.
4. **Determinism.**
   - No `HashMap` iteration where order can affect outcomes. Use `BTreeMap`,
     `IndexMap`, or `Vec` + linear scan. (`bevy_ecs` iteration order is fine —
     it mirrors the Java entity-list order we control via insertion.)
   - All gameplay randomness goes through `mc_core::java_random::JavaRandom`
     (exact 48-bit LCG port of `java.util.Random`). Never `rand::random()`.
   - Transcendentals (`sin`, `cos`, `atan2`) go through `mc_core::math` wrappers
     so parity deviations are caught in one place.
5. **No speculative cleanup.** If Java code looks like a bug (it often is —
   beta mechanics depend on several), transcribe the bug. Flag it in the PR
   description; fixing it is a human decision.
6. **One module per task.** Don't refactor across crate boundaries while
   transcribing. If a cross-cutting change is needed, write it up as a new ADR.
7. **Definition of Done** for any transcription task (see Part III): compiles,
   `cargo test -p <crate>` green, golden test added or updated, Java source
   lines cited for every non-obvious constant.
8. **Scope gates.** Phase 1 = singleplayer to vanilla parity, nothing else.
   The following are **out of scope until Phase 1 exits**:

   | Deferred subsystem (Phase 2) | Java source | Notes |
   |---|---|---|
   | Multiplayer: wire protocol, sockets, handshake, MP UI, remote players | `Packet*.java` (59), `NetClientHandler`, `NetHandler`, `NetworkManager`, `ThreadConnectToServer`, `GuiMultiplayer`, `EntityOtherPlayerMP`/`EntityClientPlayerMP` | This port has **no server classes** (no `WorldServer`/`NetServerHandler`) — SP is client-authoritative, so Phase 1 needs zero protocol code. ADR-004 owns the Phase-2 seam |
   | Texture/resource pack selection & custom pack loading | `GuiTexturePacks`, `GuiTexturePackSlot`, `TexturePackBase/Custom/Default/List` | The default atlas from `resources/` **is** in scope |

   Discovering deferred code during transcription = cite it, skip it, tag the PR
   `phase-2`. Never "quickly add" networking or texture-pack stubs.

---

## Part I — Workspace & crate layout

### Principles

1. **Logic vs. presentation split is the load-bearing wall.** Game-logic crates
   build headless, test headless, and depend on **`bevy_ecs` at most** — never
   full `bevy`, `bevy_render`, `bevy_asset`, or `bevy_ui`. This is what makes
   golden tests (fixed seed → terrain hash, scripted input → state trace) cheap.
2. **Crate boundaries follow Java cohesion**, roughly one crate per subsystem
   cluster, sized so an agent can hold one crate's context comfortably.
3. **The binary crate is thin.** All wiring (plugins, schedules, states) lives in
   `mc-app`; logic crates expose plain systems/functions.

### Crates

| Crate | Bevy deps | Owns | Java source (translate from) |
|---|---|---|---|
| `mc-core` | none | `JavaRandom`, AABB, vec/math wrappers, `Direction`/facing, `Facing`→interpolated-value helpers, game constants, `Tick` type | `MathHelper`, `Vec3`, `AxisAlignedBB`, `Random` (reimplement) |
| `mc-nbt` | none | NBT read/write (byte-exact; wrap `fastnbt` or port) | `NBTBase` family, `NBTReadWriteLock` shim dropped |
| `mc-format` | none | Chunk `.dat` save codec, `level.dat`, player data | `CompressedStreamTools`, `WorldLoader`, save/load in `World`/`Chunk` |
| `mc-blocks` | none | Block registry (~90), materials, block traits/behavior fns, light opacity/emission tables | `Block.java` + all `Block*.java` |
| `mc-items` | none | Item registry (~100), stack, tools, food, recipes (`CraftingManager`, `FurnaceRecipes`) | `Item*.java`, `CraftingManager`, `InventoryCrafting` |
| `mc-worldgen` | none | Noise generators, biomes, chunk providers (overworld/sky/hell), structure/feature placers, `SpawnerAnimals` spawn rules | `NoiseGenerator*`, `BiomeGen*`, `ChunkProvider*`, `MapGen*`, feature classes |
| `mc-world` | `bevy_ecs` | `World`, `Chunk` storage, block get/set + re-light, scheduled & random block ticks, tile entities, entity-list order, explosion | `World`, `WorldClient`, `Chunk`, `ChunkCoordinates`, `TileEntity*`, `Explosion` |
| `mc-entity` | `bevy_ecs` | All entity archetypes, shared physics (`moveEntity` port), AI, damage/health, projectiles, minecarts/boats, NBT entity save | `Entity*.java` (~59 real classes), `EntityList` |
| `mc-protocol` | none | Beta protocol packet codecs + client handshake/state machine — **Phase 2 (MP gate), do not create yet** | `Packet*.java`, `NetClientHandler`, `NetworkManager`, `ThreadConnectToServer` |
| `mc-render` | `bevy_render` + `bevy_ecs` | Chunk mesher (from `RenderBlocks`), entity models (from `ModelBase` data), sky/clouds/weather, particles, first-person hand/item, fog, HUD render prep | `RenderBlocks`, `RenderGlobal`, `WorldRenderer`, `Render*.java`, `Model*.java`, `Tessellator` (replaced) |
| `mc-ui` | `bevy_ui` | Screens (~30 `GuiScreen`), font, menus, input mapping, i18n (`StringTranslate`) | `Gui*.java`, `FontRenderer`, keybinds |
| `mc-audio` | `bevy_audio` | Sound triggers, `SoundManager` port, music sequencing | `SoundManager`, `SoundPool` (drop paulscode/mp3spi — codec handled by bevy) |
| `mc-app` | full `bevy` | Binary. Plugin wiring, tick schedule, game states (menu/world), Phase-2 MP gate, wasm target shim | `Minecraft.java`, `EntityRenderer` (camera), main loop |

### Dependency graph

```
mc-core ──────────────┐
   │                  │
   ▼                  ▼
mc-nbt            mc-blocks ──► mc-items
   │                  │            │
   ▼                  ▼            ▼
mc-format        mc-worldgen ◄── (recipes)
                    │             
                    ▼             
                mc-world ◄──── mc-entity
                    │             │
                    ▼             ▼
              mc-protocol    mc-render ─► mc-ui ─► mc-audio
                    └─────────┬───────────┘
                              ▼
                           mc-app
```

(Rules: no cycles; `mc-world` ↔ `mc-entity` is the one allowed tight coupling —
see ADR-001 §7. Everything below `mc-render` is headless.)

### Java → Rust module map (quick lookup)

| Java cluster | Rust path |
|---|---|
| `NoiseGenerator*`, `ChunkProviderGenerate/Sky/Hell` | `mc-worldgen::{noise, provider, biome}` |
| `World`, `Chunk`, `WorldProvider*` | `mc-world::{world, chunk, provider}` |
| `Block*` | `mc-blocks::{registry, behavior::*}` |
| `Item*`, `CraftingManager` | `mc-items::{registry, crafting}` |
| `Entity*` (non-FX) | `mc-entity::{archetype, physics, ai, save}` |
| `EntityFX` + 20 FX subclasses | `mc-render::particles` (they are visuals, not entities) |
| `RenderBlocks`, `WorldRenderer`, `RenderGlobal` | `mc-render::{mesher, chunk, global}` |
| `Model*` (ModelBase subclasses) | `mc-render::model` (data tables, see ADR-001 §6) |
| `Gui*` | `mc-ui::screen::*` |
| `Packet*` | `mc-protocol::{packet, handler}` — **Phase 2 (MP gate)** |
| `MathHelper`, `AxisAlignedBB`, `Random` | `mc-core::{math, aabb, java_random}` |

### Tick architecture

- Bevy `FixedUpdate` at **`Duration::from_millis(50)`** = exactly 1 game tick
  (20 TPS). Everything gameplay-facing runs inside `FixedUpdate`; render-only
  interpolation runs on `Update`.
- Mirror the Java tick skeleton as named system sets in `mc-app`. Exact order
  must be transcribed from the client-authoritative path this port actually
  runs — `Minecraft.runGameLoop` → `WorldClient.tick()` →
  `World.updateEntitiesAndBlocks()`/`tick()` — do **not** invent an order, and
  do not assume vanilla's internal-server ordering (this port has none). The
  skeleton is:

```
FixedUpdate
 ├─ TickInput              (SP: from local input; MP: from NetClientHandler)
 ├─ TickBlocksRandom       (random block ticks, per-chunk 80/216 etc.)
 ├─ TickBlocksScheduled    (scheduled ticks: fluids, fire, ...)
 ├─ TickTileEntities       (furnaces, pistons moving, chests, signs)
 ├─ TickEntities           (entity list order = spawn/serialization order)
 ├─ TickPlayer
 ├─ TickWorldTime/weather
 └─ TickSaveAutoflush      (periodic)
```

- **Phase 1 is client-authoritative singleplayer** — this mirrors the port
  itself: it contains no server classes (no `WorldServer`, no
  `NetServerHandler`), and `WorldClient extends World` runs all gameplay logic
  directly. The Rust Phase-1 app therefore runs **one** `bevy_ecs::World`
  (logic vs. render split by system sets) with zero protocol code on the hot
  path. When multiplayer unlocks in Phase 2, ADR-004 decides the seam
  (loopback protocol vs. direct call boundary) without reshaping Phase-1 crates.

---

## Part II — ADR-001: Entity archetypes

**Status:** proposed (session-zero)
**Deciders:** human owner
**Scope:** all translation of `Entity*.java` into `mc-entity`

### 1. Context

Beta 1.7.3 entities are a deep single-inheritance hierarchy (~59 non-FX classes):

```
Entity
├─ EntityLiving              (health, hurt time, sounds, damage)
│  ├─ EntityCreature         (AI: wander/panic/attack, pathing)
│  │  ├─ EntityAnimal        (pig, cow, sheep, chicken, wolf)
│  │  ├─ EntityMob           (zombie, skeleton, spider, creeper,
│  │  │                       slime, pigzombie, giantzombie)
│  │  └─ EntityWaterMob      (squid)
│  ├─ EntityFlying           (ghast)
│  └─ EntityPlayer           (├ EntityPlayerSP — Phase 1;
│                             └ EntityClientPlayerMP, EntityOtherPlayerMP — MP-only, Phase 2)
├─ EntityMinecart            (5 variants via cargo type field)
├─ EntityBoat
├─ EntityItem / EntityPainting
├─ EntityArrow / EntitySnowball / EntityEgg / EntityFish
├─ EntityFireball / EntityWeatherEffect (└ EntityLightningBolt)
├─ EntityTNTPrimed / EntityFallingSand
└─ EntityFX + ~20 particle subclasses        → mc-render, NOT this ADR
```

Rust has no inheritance; ECS composition must replace: (a) inherited fields,
(b) virtual dispatch on ~15 overridable methods, (c) per-type registries
(NBT save/load, spawn eggs, network factory).

### 2. Decision

Six rules. Follow all of them; consistency across 59 classes matters more than
local elegance.

#### Rule 1 — Marker component per Java class; behavior via `With<>` queries

Each concrete Java class becomes a unit marker component named after the class.
"Inheritance" is expressed as system queries with marker bounds:

```rust
// EntityLiving.onLivingUpdate applies to every living thing:
fn living_update(q: Query<(&mut Velocity, &Health), Or<(With<Animal>, With<Mob>, With<WaterMob>, With<Flying>, With<Player>)>>) { ... }

// EntityWolf.onLivingUpdate REPLACES the EntityAnimal default for wolves:
fn wolf_living_update(q: Query<&mut WolfState, With<Wolf>>) { ... }
```

- Intermediate abstract classes (`EntityLiving`, `EntityCreature`, …) become
  **marker traits or marker components** applied alongside the concrete marker
  (e.g. every `Wolf` entity also gets `Animal`, `Creature`, `Living` markers).
  Spawn tables generate the full marker set; there is a compile-time-checked
  helper per archetype (Rule 5).
- **Override semantics:** a Java override of `onLivingUpdate`/`onUpdate`/
  `updatePlayerActionState` means the derived system runs **instead of** the
  base system for that marker. Implement by excluding the derived marker from
  the base system's query (`Without<Wolf>` etc.) or by disjoint system sets —
  never by "check the type inside one big system".

#### Rule 2 — Java fields → components, verbatim

Base `Entity` fields (Entity.java:7–55) become one shared component set every
archetype receives:

```rust
/// Direct transcription of net.minecraft/src/Entity.java fields.
#[derive(Component)]
pub struct EntityTransform {          // posX/Y/Z + prevs + lastTicks (f64)
    pub pos: Vec3d, pub prev_pos: Vec3d, pub last_tick_pos: Vec3d,
}
#[derive(Component)]
pub struct Velocity { pub motion: Vec3d }          // motionX/Y/Z (f64)
#[derive(Component)]
pub struct Facing { pub yaw: f32, pub pitch: f32, pub prev_yaw: f32, pub prev_pitch: f32 }
#[derive(Component)]
pub struct Body { pub width: f32, pub height: f32, pub y_offset: f32, pub step_height: f32 }
#[derive(Component)]
pub struct GroundState { pub on_ground: bool, pub collided_h: bool, pub collided_v: bool }
#[derive(Component)]
pub struct EntityMeta { pub id: EntityId, pub ticks_existed: i32, pub fire: i32,
                        pub is_dead: bool, pub been_attacked: bool /* … */ }
```

`EntityLiving` fields likewise → `Health { health: i32, hurt_time: i32, … }`,
`SwingProgress { … }`, etc. Unmapped names (`field_9365_p`) follow the
Part 0 §2 mapping rule.

#### Rule 3 — Physics is ONE system, shared by everything

`Entity.moveEntity` / collision stepping (Entity.java) is transcribed **once**
into `mc-entity::physics` and run for every entity with `Body` + `Transform` +
`Velocity`. Per-entity variation enters only through the components Java also
parameterized on (`stepHeight`, `width/height`, `noClip`,
`entityCollisionReduction`). **Never** copy collision code into an archetype.
Same for `fallDistance`/fire handling and the ridden/riding constraint solving.

#### Rule 4 — Per-kind data as enum components, not parallel booleans

Java instance flags/modes become explicit state enums. Example (the ADR's
canonical illustration — `EntityWolf`):

```rust
#[derive(Component)]
pub struct WolfState {
    pub looks_with_interest: bool,
    pub angry: bool,          // dataWatcher byte 16 & 0x02  // src: EntityWolf.java:22-27
    pub tamed: bool,          // dataWatcher byte 16 & 0x04
    pub sitting: bool,        // dataWatcher byte 16 & 0x08
    pub collar: u8,           // dataWatcher byte 20
    pub shaking: WolfShake,   // isWolfShaking + timeWolfIsShaking + prevTimeWolfIsShaking
}
```

`dataWatcher`-backed values stay grouped exactly as the dataWatcher lays them
out (byte 16 bitflags etc.) because the network layer (ADR-004) syncs those
bytes verbatim. AI locals (`field_25048_b`, `field_25054_g` — head-look
interpolators) become named fields with mappings.md entries.

#### Rule 5 — Spawn/save/render registration via one enum + match tables

ECS resists trait-object registries; use a single closed enum plus plain match
tables (mirrors `EntityList`):

```rust
pub enum EntityKind { Pig, Cow, Sheep, Chicken, Wolf, /* … */ }

// One row per kind: everything Java did with instanceof/class literals.
pub fn archetype(kind: EntityKind) -> EntityArchetype { match kind { … } } // marker set + components
pub fn save(e: &EntityRefForSave, kind: EntityKind, nbt: &mut NbtCompound) { … }
pub fn load(kind_from_id: &str, nbt: &NbtCompound, world: &mut World) -> Entity { … }
pub fn model(kind: EntityKind) -> ModelDef { … }   // see Rule 6
```

Adding a new entity = one enum variant + one arm in each table + its systems.
A `debug_check` test asserts every kind has all four arms (exhaustive matches
make the compiler enforce most of this anyway).

#### Rule 6 — Models are data, transcribed from `ModelBase` subclasses

`ModelBiped`, `ModelQuadruped`, etc. build box lists with texture offsets.
Port as declarative tables in `mc-render::model` (box: size, offset, uv,
rotation points) — no behavior. `RenderWolf` scale/offset constants live beside
the table. This keeps rendering parity auditable line-by-line against Java.

#### Rule 7 — The world↔entity seam

Java entities hold `public World worldObj`. Rust: **no** `World` handle inside
components. Entity systems take `&mut McWorld` (the `mc-world` game-world, not
bevy's) as a `Resource` plus `Commands`/events for spawn-despawn. Block queries
an entity needs (`isBlockNormalCube` etc.) go through accessor methods on that
resource. This is the allowed `mc-world` ↔ `mc-entity` coupling from Part I.

### 3. Worked example — wolf skeleton (the pattern to copy)

```rust
// spawn (from EntityList string "Wolf")
fn spawn_wolf(pos: Vec3d, cmds: &mut Commands) {
    cmds.spawn((
        EntityTransform::at(pos), Velocity::ZERO, Facing::default(),
        Body { width: 0.6, height: 0.8, y_offset: 0.0, step_height: 0.0 }, // src: EntityWolf ctor
        EntityMeta::new_autoid(), GroundState::default(),
        Health::mob(8),                                            // src: EntityWolf.entityInit
        Living, Creature, Animal, Wolf,                            // marker chain
        WolfState::default(),
    ));
}

// EntityAnimal.updateEntityActionState default — runs for wolves too…
fn animal_wander(q: Query<&mut MoveIntent, With<Animal>>) { … }

// …unless EntityWolf.updatePlayerActionState overrides:
fn wolf_action_state(
    q: Query<(&mut MoveIntent, &WolfState, &Transform), (With<Wolf>, Without<SittingOverrideNotRelevant>)>,
    player: Query<(&Transform, &HeldItem), With<Player>>,
) { /* tame-bone check, owner-follow, angry targeting — src: EntityWolf.java:85-113 */ }

// EntityWolf.onLivingUpdate — shaking dry, head-look (src: EntityWolf.java:114-140)
fn wolf_living_update(q: Query<(&mut WolfState, &Facing), With<Wolf>>) { … }
```

### 4. Forbidden patterns

- ❌ One mega-system with `match kind { … }` dispatch for behavior (allowed
  **only** in the Rule-5 registration tables).
- ❌ `Box<dyn EntityTrait>` object hierarchies — recreates inheritance, fights ECS.
- ❌ Duplicate physics/collision code anywhere outside `mc-entity::physics`.
- ❌ `HashMap` iteration touching gameplay order.
- ❌ Renaming or "improving" constants, dataWatcher layouts, or sound event names.
- ❌ Entities touching bevy `Transform` (render transform is derived in
  `mc-render` from `EntityTransform`, interpolated by alpha).

### 5. Consequences

- ~59 classes flatten into: 1 shared component set, ~40 state components,
  ~30 marker types, ~50 behavior systems, 4 registration tables.
- Entity-list iteration order (which Java preserves and beta mechanics observe,
  e.g. minecart collision order) is controlled by insertion order + a stable
  `EntityMeta.id` sort inside `TickEntities` where Java relied on list order.
- Network `dataWatcher` sync (ADR-004) maps 1:1 because Rule 4 kept layouts.

---

## Part III — Verification contract (per crate)

| Crate | Golden test |
|---|---|
| `mc-core` | `JavaRandom` vs Java-generated sequences; AABB/interp fixtures |
| `mc-worldgen` | fixed seeds → per-chunk block-array hash, overworld+sky+hell, ~100 chunks × 5 seeds |
| `mc-world` | scripted block set/get → light levels; tick order trace diff |
| `mc-entity` | scripted input scripts → per-tick `pos/health/state` trace diff vs Java harness output |
| `mc-format` | round-trip real `level.dat`/chunk files from this repo's `jars/` resources |
| `mc-protocol` | packet encode/decode fixtures; handshake against local vanilla b1.7.3 server — **Phase 2 (MP gate)** |
| `mc-render` | fixed-camera screenshots at fixed ticks; perceptual-hash diff vs Java build |

Golden fixtures are generated from the Java build via a small instrumented
harness (separate task; see ADR queue) and stored under `tests/golden/`.
**A transcription PR without a test is not reviewable** — if a subsystem lacks
a harness yet, say so in the PR and tag `needs-golden`.

### Definition of Done (every task)

1. `cargo build -p <crate>` + `cargo clippy -p <crate>` clean.
2. Unit/golden tests added; suite green.
3. Java source citations on all non-obvious constants and ordering.
4. `docs/mappings.md` updated for every renamed `field_NNN`/`func_NNN`.
5. No new crate-level deps without an ADR note.

---

## Part IV — Queued ADRs (write before starting the related module)

| ADR | Topic | Blocking |
|---|---|---|
| [ADR-007](docs/ADR-007-golden-harness.md) — **written** | Java-behavior golden harness (scenario → expected.jsonl from the Java build, exact-diff in CI) | all — build Tier A before any transcription |
| ADR-002 | Chunk storage & meshing pipeline (`RenderBlocks` → vertex buffers), chunk render sorting | `mc-render` |
| ADR-003 | Lighting engine port (sky/block flood fill + smooth lighting/AO) and relight-on-edit semantics | `mc-world` |
| ADR-004 | **Re-scoped (SP-first):** Phase-1 client-authoritative app seam + save/load boundaries. Wire protocol, `dataWatcher` sync, loopback-vs-direct — deferred to Phase-2 kickoff | `mc-app` (Phase 1); `mc-protocol` (Phase 2, not yet created) |
| ADR-005 | Save format & world directory layout (per-chunk gzip NBT) | `mc-format` |
| ADR-006 | Block behavior dispatch & update-graph ordering (redstone/pistons) | `mc-blocks`, `mc-world` |
| ADR-008 | Vendoring & version-delta policy for MinecraftOSS code (Part V) | `mc-core`, `mc-nbt` first |

---

## Part V — Prior-art reuse assessment: MinecraftOSS (Java 26.3 port)

Inspected: `2010-rust-rewrite-mashup-main/third_party/minecraftoss` (five
engine crates, copied from MinecraftOSS commit `4013a68`) plus its Bevy
integration `crates/minecraft_terrain` (46 files, bevy 0.19).

### V.1 What exists there

| Crate | Engine LOC | Trace-bin LOC | Notes |
|---|---|---|---|
| `core` | 8.0k | — | pos, registries, tags, chunk storage, **NBT codec**, **Java LCG**, light, entity NBT catalog |
| `generator` | 31.2k | — | 26.3 density-function worldgen, aquifers, carvers |
| `world` | 15.1k | — | chunk map, fluid/fire/piston/redstone/rail/random_tick level systems, physics (voxel-shape collide) |
| `player` | 16.1k | 5.1k | inventory, items, recipes, loot, placement + scenario-replay bins |
| `entities` | 35.2k | 20.2k | mobs + AI (goal system, navigation, pathing) + per-behavior check bins |

Architectural facts that matter to us:

- **Engine crates are Bevy-free** (deps: flate2, serde, md5, glam). The Bevy
  boundary lives entirely in `minecraft_terrain`. This independently validates
  Part I's logic/render split — including doing entities as plain structs, not
  ECS components.
- **~25k LOC (≈20%) is verification code**: `bin/check_*_trace.rs` /
  `*_trace.rs` binaries replaying `scenarios/*.json`. This is ADR-007's design
  built for real — adopt their binary-per-behavior layout in addition to
  scenario files.
- Their ground truth comes from a **Java-side harness that dumps artifacts**
  (e.g. `artifacts/entity-catalog/26.3.json`, loaded by
  `core/src/entity_data.rs`). Those artifacts aren't in this copy — we still
  must build our own (ADR-007), but the catalog-dump technique is directly
  applicable to Tier C entity defaults.
- Their discipline is ours, independently derived: workspace lints allow
  `excessive_precision`/`approx_constant` specifically to preserve transcribed
  f32 bit patterns; doc comments pin values to remapped-JAR sources
  (`Wolf.createAttributes`, `sized(0.6F, 0.85F)`). Part 0 rules 1–2 and the
  lint config should be copied wholesale.
- **License:** the mashup root is Apache-2.0 (IW4L + fork, per `NOTICE`);
  `third_party/minecraftoss` carries no separate LICENSE file in-tree. Treat
  as Apache-2.0 for reading/adapting, but **verify the upstream MinecraftOSS
  repository's license before vendoring wholesale** (ADR-008). NOTICE
  documents that no Minecraft code/assets ship in the repo — their game data
  is downloaded from Mojang at runtime, exactly like our situation differs
  (we hold decompiled source as spec, never distribute it).

### V.2 Disposition table (what we may copy, adapt, reference, or skip)

| MinecraftOSS asset | Verdict | Notes for b1.7.3 |
|---|---|---|
| `core/src/random.rs` — `LegacyRandom` | **copy 1:1** | Exact `java.util.Random` 48-bit LCG (`0x5deece66d`, mask `(1<<48)-1`, Marsaglia-polar gaussian cache). Beta uses this same RNG. Drop the Xoroshiro half; verify method coverage (`nextInt(bound)`, `nextFloat`, `nextGaussian`). This *is* `mc-core::java_random` |
| `core/src/nbt.rs` (+ `snbt.rs`) | **copy 1:1, verify** | Big-endian NBT with `write_gzip` — exactly beta's chunk-save codec. Add round-trip fixtures from real `.dat` files (ADR-007 Tier A) |
| Workspace lint config (`excessive_precision`, `approx_constant` allows) | **copy 1:1** | Part 0 rule 1, enforced mechanically |
| Trace-bin + scenario-replay layout | **copy the pattern** | `bin/check_<behavior>_trace.rs` replaying `scenarios/*.json`; merge into ADR-007 alongside scenario files |
| `core/src/light.rs` + `minecraft_terrain/src/lighting.rs` | **adapt** | Modern per-section light ≠ beta nibble-array sky/block, but the flood-fill structure and the `Vertex { sky_light, block_light }` shader-feeding pattern transfer. Beta's is simpler — Java source stays authoritative |
| `crates/minecraft_terrain/src/{mesh,sections,model}.rs` | **reference for ADR-002** | bytemuck `Vertex` layout, section-based meshing, per-block-variant model resolution, `texture_mips`. Meshing *semantics* still come from `RenderBlocks.java` |
| `entities/src/*.rs` structure & doc style | **pattern reference** | Per-type constants pinned to cited Java (`wolf.rs` is the exemplar) — matches ADR-001 Rule 4/6 style. But values are 26.3 (wolf `0.6×0.85` vs beta `0.6×0.8`); **never copy their constants** |
| Entities AI (`goals.rs`, `navigation.rs`, `path_search.rs`) | **adapt with care** | Goal/brain system is modern; beta's `updatePlayerActionState` is far simpler. A* pathing concepts transfer; transcribe beta's logic from our Java |
| `world/level/{fluid,fire,piston,redstone,rail}.rs` | **pattern reference** | Update-graph architecture informs ADR-006; semantics differ (26.3 has observers/comparators/copper; beta quirks are the spec) |
| `world/level/physics.rs` (voxel-shape collide) | **reference only** | 26.3 `VoxelShape` epsilon walk ≠ beta box-only `getCollidingBoundingBoxes` + `moveEntity` stepping. Transcribe beta's from `Entity.java` |
| `player` inventory/stack structures | **partial adapt** | Stack/inventory plumbing transfers; modern datapack recipes/loot ≠ beta hardcoded `CraftingManager` |
| `generator/*` (31.2k) | **not applicable** | Density-function worldgen is post-1.18 entirely. Beta needs its own `NoiseGenerator*` stack from our Java (~6k LOC). The single biggest non-reuse area |
| `core/{palette,anvil,chunk_nbt}.rs`, datapacks, advancements, POI, gossip, enchant | **not applicable** | Modern formats/systems with no beta counterpart |

### V.3 Version-delta rule (extends Part 0)

Any code lifted from MinecraftOSS is presumed **26.3-tainted** until proven
b1.7.3-correct:

1. Mark it `// oss: <file>` next to the required `// src: <b1.7.3 Java
   file:line>` citation. If the Java citation is missing, the port is not done.
2. Run the delta checklist before trusting it: block states vs id+meta ·
   worldgen algorithm · entity AI & attributes · hitbox/dimension constants ·
   chunk & save format · redstone component set · recipe/loot source ·
   dataWatcher vs entity NBT catalog.
3. Golden fixtures (ADR-007) are the arbiter — a copied module passes only if
   beta fixtures pass, never because upstream's did.

### V.4 Net effect on the plan

- **Real savings:** `mc-core` (RNG + NBT ≈ 2–3k battle-tested LOC),
  verification-infrastructure layout, lint discipline, and a working reference
  Bevy mesher for ADR-002. The engine-seam architecture is now
  *validated by prior art* rather than argued from theory.
- **Not saved:** the beta-specific 60% — worldgen, beta AI, beta physics
  constants, redstone/piston quirks, `RenderBlocks` meshing semantics, save
  format. Those still get transcribed from this repo's Java, under golden-test
  discipline.
- Risk flip: the biggest new risk is **silent 26.3 contamination** — modern
  code that looks right but encodes wrong-era behavior. V.3 exists to contain
  exactly that.

