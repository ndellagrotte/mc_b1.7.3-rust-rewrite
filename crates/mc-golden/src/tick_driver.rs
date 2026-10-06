//! The `kind: "tick"` driver: runs a [`TickScenario`] on an [`mc_sim::world::World`]
//! and emits the trace the Java oracle emits (tests/golden/README.md "Tier B/C/S
//! scenarios"; oracle: `tools/oracle/java/harness/TickScenario.java`,
//! `HarnessWorld.java`, `Dumps.java`).
//!
//! Use it as the drive closure of [`crate::run`]:
//!
//! ```no_run
//! mc_golden::run("tests/golden/entity/flat-item-drop", mc_golden::tick_driver::drive).unwrap();
//! ```
//!
//! # Sequence (mirrors `TickScenario.run`)
//!
//! 1. Validate the scenario as the oracle does (`ticks >= 1`, op ticks in
//!    range, `difficulty` 0..=3, dumps have exactly one of `every`/`t`).
//! 2. Build the world (`HarnessWorld.create`): `World::new` with
//!    [`EmptyChunkProvider`] for `"world": "empty"`, or the dimension's
//!    generator (`WorldProvider::get_chunk_provider(seed)`, WP-1.4; population
//!    is still a recorded stub, WP-1.7) for `"world": "generated"`, then
//!    [`World::make_deterministic`] (`makeDeterministic` + `resetStatics`).
//!    `"save_format": "mcregion"` puts a [`SaveOldDir`] on `world/` of an
//!    in-memory [`mc_sim::save::Storage`] (the oracle's temp directory), see
//!    "The `save` op".
//! 3. The implicit preload (`TickScenario.implicitPreload`) and its
//!    `preload` line.
//! 4. For each tick `t`: the ops with this `t` in file order (a stable sort
//!    by `t`, as the oracle's `List.sort`), each emitting its `op` line;
//!    `HarnessWorld.runTick`; `world.updatingLighting()` once; then the dumps
//!    due at `t` in scenario order.
//!
//! `runTick` (HarnessWorld.java, Minecraft.java:1133-1174) as implemented in
//! `run_tick`: with a player, `joinPlayerCounter` and every 30th tick
//! `joinEntityInSurroundings(player)`; `difficultySetting = difficulty`;
//! `field_27172_i` countdown; `updateEntities`;
//! `setAllowedMobSpawns(difficulty > 0, true)`, or `(false, false)` for
//! `"mob_spawning": "none"`; `tick`; with a player,
//! `randomDisplayUpdates(floor(posX), floor(posY), floor(posZ))`. The
//! frame-start pool resets (`AxisAlignedBB.clearBoundingBoxPool`,
//! `Vec3D.initialize`) have no observable state in the port.
//!
//! # Ops and dumps
//!
//! | Op / dump | mc-sim call |
//! |---|---|
//! | `set_block` | [`World::set_block_and_metadata_with_notify`] → `ok` |
//! | `set_block_silent` | [`World::set_block_and_metadata`] → `ok` |
//! | `fill` | `set_block_and_metadata` over the box, x outer, z middle, y inner → `changed` |
//! | `set_time` | [`World::set_world_time`] |
//! | `set_weather` | the given fields of [`World::world_info`] (`rain_time`, `raining`, `thunder_time`, `thundering`) |
//! | `preload` | [`World::get_chunk_from_chunk_coords`] over the square, dz outer, dx inner, each followed by draining [`World::updating_lighting`] → `loaded` |
//! | `player` | [`new_entity_player_sp`] (the pure `EntityPlayer` constructor chain; its update is recorded as not transcribed, WP-2.5), `setLocationAndAngles`, [`World::spawn_entity_in_world`] → `id`, `joined` |
//! | `spawn` | `construct` (the kind's constructor; `Item`: [`new_entity_item`] and `delayBeforeCanPickup = pickup_delay` (10); the harness-defined `Egg`, `Fireball`, `LightningBolt`, `FishingHook`, a `Painting` with `direction` and a `Minecart` with `type`), `setLocationAndAngles` unless the constructor took the position, then [`World::spawn_entity_in_world`] (`LightningBolt`: [`World::add_weather_effect`]) → `id`, `kind`, `joined` |
//! | `set_motion` / `set_pos` / `set_rotation` | fields of the loaded entity / [`EntityBase::set_position`](mc_sim::entity::EntityBase::set_position) / fields |
//! | `attack` | `attack`: `target.attackEntityFrom(attacker or null, damage)` through [`mc_sim::entity::dispatch::attack_entity_from`] (target out of its slot, attacker a copy) → `target_id`, `via` `"direct"`, `result` |
//! | `interact` | `TickScenario.interact` in `interact`: the stack into slot `currentItem`, `EntityPlayer.useCurrentItemOnEntity` ([`dispatch::interact`] with the player and the target both in hand; when false, `ItemStack.useItemOnEntity` on a living target is a recorded gap, WP-2.6), then `sendUseItem` ([`send_use_item`]) → `id`, `result`, `used`, `stack` |
//! | `explode` | [`World::create_explosion`] (no exploder), or [`World::new_explosion`] with `flaming` → `destroyed` (the size of `destroyedBlockPositions`) |
//! | `use_item` | `TickScenario.useItem` in `use_item`: the stack into slot `currentItem`, `PlayerController.sendPlaceBlock` (block dispatch `block_activated`, else `ItemStack.useItem`), `swingItem`, the size-0 removal, `sendUseItem` → `placed`, `used`, `stack`. Recorded gaps: `Item.onItemUse` (taken as false) and `Item.onItemRightClick` (taken as returning the stack unchanged), item behaviour WP-2.6; the `blockActivated` overrides the block dispatch records (WP-1.6) |
//! | `chunk_digest` | [`World::chunk_arrays`] → [`TraceLine::chunk_digest_tick`] (+ [`TraceLine::chunk_full_tick`]) |
//! | `block` / `block_box` | [`World::block_dump`] |
//! | `scheduled_ticks` | [`World::scheduled_ticks`] |
//! | `entity_state` | [`mc_sim::golden::entity_state_fields`] in `loadedEntityList` order |
//! | `world_time` / `rng_state` | [`World::get_world_time`] / [`World::rng_state`] |
//! | `weather` | [`World::world_info`] timers and flags, the `World` rain/thunder strengths |
//! | `ray_trace` | [`World::ray_trace_between`] (`rayTraceBlocks_do` on fresh vectors) |
//! | `power` | [`World::is_block_getting_powered`], then [`World::is_block_indirectly_getting_powered`] |
//! | `save` | [`World::try_save_world`]`(true)`, then [`emit_tier_s`] on the in-memory save (masked) |
//!
//! Not supported yet, each an error naming the op and the missing Java code:
//! `spawn` of the mobs other than `Pig`, `Sheep`, `Cow`, `Chicken`, `Wolf`,
//! `Minecart` and `Boat` (their `(World)` constructors), `attack` with the player as the attacker
//! (`EntityPlayer.attackTargetEntityWithCurrentItem`, WP-2.5). `use_item` runs, but every item's `onItemUse` /
//! `onItemRightClick` is a recorded gap (see the table), so a fixture whose
//! item does something diverges at the first block or inventory dump after
//! the op.
//!
//! # Entity kind names
//!
//! `kind` in `spawn` lines and `entity` lines is the Java class simple name
//! ([`EntityKindData::java_class`](mc_sim::entity::EntityKindData::java_class)).
//! The oracle's player is its own subclass `HarnessPlayer` of
//! `EntityPlayer` (tools/oracle/java/harness/HarnessPlayer.java:27); the
//! port's player is `EntityPlayerSP`, reported as `HarnessPlayer`
//! ([`trace_kind`]).
//!
//! # The `save` op (Tier S)
//!
//! `World.saveWorld(true, null)` through the world's [`SaveOldDir`] (session
//! lock, `level.dat` with `version` 19132 and the player, every chunk with
//! `needsSaving(true)` through `McRegionChunkLoader`), then `TierS.emitFiles`
//! with the masked `level.dat` ([`emit_tier_s`]). The save code's clock is
//! [`crate::world_read_driver::HARNESS_CLOCK_MILLIS`] (`LastPlayed` and the region timestamps are not
//! compared).
//!
//! One class difference is bridged, as [`trace_kind`] bridges the name: the
//! oracle's `HarnessPlayer` extends `EntityPlayer`, whose `writeEntityToNBT`
//! has no `Score` (that is `EntityPlayerSP`'s, EntityPlayerSP.java:98-101),
//! while the port's player is `EntityPlayerSP`. [`HarnessSaveHandler`]
//! removes `Score` from the player compound before `level.dat` is written.

use mc_sim::block::dispatch as block_dispatch;
use mc_sim::entity::dispatch;
use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::player::{new_entity_player_sp, swing_item};
use mc_sim::golden::{StateValue, entity_state_fields};
use mc_sim::item::ItemStack;
use mc_sim::save::{ChunkLoader, NoSave, SaveError, SaveHandler, SaveOldDir, SharedSaveStore, Tag};
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::info::WorldInfo;
use mc_sim::world::provider::{Dimension as SimDimension, WorldProvider};
use mc_sim::world::ray_trace::EnumMovingObjectType;
use mc_sim::world::{World, WorldEnvironment};
use serde_json::{Map, Value};

use crate::scenario::{
    Dimension, Dump, DumpKind, EntityWhich, MobSpawning, Op, OpKind, TickScenario, WorldSource,
};
use crate::trace::{ChunkArrays, EntityLine, ScheduledTick};
use crate::world_read_driver::{WORLD_DIR, emit_tier_s, harness_store};
use crate::{GoldenError, Scenario, ScenarioKind, TraceLine, TraceWriter, blake3_hex};

/// Drives a `kind: "tick"` scenario (module docs). Other kinds are an error.
pub fn drive(scenario: &Scenario, w: &mut TraceWriter) -> Result<(), GoldenError> {
    let ScenarioKind::Tick(sc) = &scenario.kind else {
        return Err(GoldenError::other(format!(
            "tick_driver: scenario {:?} is not kind \"tick\"",
            scenario.name
        )));
    };
    validate(sc)?;
    let (mut world, save_store) = build_world(sc)?;
    let mut h = Harness {
        save_store,
        player: None,
        join_player_counter: 0,
        difficulty: sc.difficulty,
        mob_spawning: sc.mob_spawning.unwrap_or_default(),
        player_pos: None,
    };

    implicit_preload(&mut world, &sc.ops, w);

    // Ops grouped by t, file order within a tick (TickScenario.java: a stable
    // sort by t, then consumed in order).
    let mut ops: Vec<&Op> = sc.ops.iter().collect();
    ops.sort_by_key(|op| op.t);
    let mut next = 0;
    for t in 0..sc.ticks {
        // Frame start (Minecraft.java:519-520): AxisAlignedBB.clearBoundingBoxPool()
        // and Vec3D.initialize() reset object pools, which the port does not have.
        while next < ops.len() && ops[next].t == t {
            apply(&mut world, &mut h, ops[next], w)?;
            next += 1;
        }
        run_tick(&mut world, &mut h);
        // Once per frame after the tick, Minecraft.java:553 (at most 500 updates).
        world.updating_lighting();
        for d in &sc.dumps {
            if d.due_at(t) {
                dump(&mut world, d, t, w)?;
            }
        }
    }
    Ok(())
}

/// The kind name a trace reports for an mc-sim class name: the port's local
/// player `EntityPlayerSP` is the oracle's `HarnessPlayer`; every other class
/// keeps its name.
pub fn trace_kind(java_class: &'static str) -> &'static str {
    match java_class {
        "EntityPlayerSP" => "HarnessPlayer",
        other => other,
    }
}

/// `EntityList`'s name table (EntityList.java:83-107): `spawn` kind → Java
/// class. Used to name the class in "not transcribed" errors.
const ENTITY_LIST: [(&str, &str); 24] = [
    ("Arrow", "EntityArrow"),
    ("Snowball", "EntitySnowball"),
    ("Item", "EntityItem"),
    ("Painting", "EntityPainting"),
    ("Mob", "EntityLiving"),
    ("Monster", "EntityMob"),
    ("Creeper", "EntityCreeper"),
    ("Skeleton", "EntitySkeleton"),
    ("Spider", "EntitySpider"),
    ("Giant", "EntityGiantZombie"),
    ("Zombie", "EntityZombie"),
    ("Slime", "EntitySlime"),
    ("Ghast", "EntityGhast"),
    ("PigZombie", "EntityPigZombie"),
    ("Pig", "EntityPig"),
    ("Sheep", "EntitySheep"),
    ("Cow", "EntityCow"),
    ("Chicken", "EntityChicken"),
    ("Squid", "EntitySquid"),
    ("Wolf", "EntityWolf"),
    ("PrimedTnt", "EntityTNTPrimed"),
    ("FallingSand", "EntityFallingSand"),
    ("Minecart", "EntityMinecart"),
    ("Boat", "EntityBoat"),
];

// ---- setup ----------------------------------------------------------------------

/// The oracle's argument checks (TickScenario.run).
fn validate(sc: &TickScenario) -> Result<(), GoldenError> {
    if sc.ticks < 1 {
        return Err(GoldenError::other("\"ticks\" must be >= 1"));
    }
    if !(0..=3).contains(&sc.difficulty) {
        return Err(GoldenError::other("difficulty must be 0..3"));
    }
    let save_format = sc.save_format.as_deref().unwrap_or("none");
    for op in &sc.ops {
        if op.t >= sc.ticks {
            return Err(GoldenError::other(format!(
                "op t={} outside 0..{}",
                op.t,
                sc.ticks - 1
            )));
        }
        if matches!(op.kind, OpKind::Save { .. }) && save_format != "mcregion" {
            return Err(GoldenError::other(
                "op save needs \"save_format\": \"mcregion\"",
            ));
        }
    }
    for d in &sc.dumps {
        if d.every.is_some() == d.t.is_some() {
            return Err(GoldenError::other(
                "dump needs exactly one of \"every\" and \"t\"",
            ));
        }
        if d.every == Some(0) {
            return Err(GoldenError::other("\"every\" must be >= 1"));
        }
    }
    Ok(())
}

/// The oracle's `SaveOldDir` as its `HarnessPlayer` meets it (module docs,
/// "The `save` op"): the player compound loses `EntityPlayerSP`'s `Score`.
#[derive(Debug)]
pub struct HarnessSaveHandler(pub SaveOldDir);

impl SaveHandler for HarnessSaveHandler {
    fn load_world_info(&mut self) -> Option<WorldInfo> {
        self.0.load_world_info()
    }
    fn check_session_lock(&mut self) -> Result<(), SaveError> {
        self.0.check_session_lock()
    }
    fn get_chunk_loader(&mut self, provider: &WorldProvider) -> Option<Box<dyn ChunkLoader>> {
        self.0.get_chunk_loader(provider)
    }
    fn save_world_info_and_player(
        &mut self,
        info: &mut WorldInfo,
        mut player: Option<Tag>,
    ) -> Result<(), SaveError> {
        if let Some(Tag::Compound(m)) = player.as_mut() {
            m.remove("Score");
        }
        self.0.save_world_info_and_player(info, player)
    }
    fn save_world_info(&mut self, info: &WorldInfo) -> Result<(), SaveError> {
        self.0.save_world_info(info)
    }
}

/// `HarnessWorld.create`: `new World(handler, "harness", provider, seed)`,
/// then `makeDeterministic` / `resetStatics`. With `"save_format":
/// "mcregion"`, also the shared store holding the save.
fn build_world(sc: &TickScenario) -> Result<(World, Option<SharedSaveStore>), GoldenError> {
    let dimension = match sc.dimension {
        Dimension::Overworld => SimDimension::Surface,
        Dimension::Nether => SimDimension::Hell,
    };
    let provider = WorldProvider::new(dimension);
    let generator = match sc.world {
        WorldSource::Empty => ChunkGenerator::Empty(EmptyChunkProvider),
        // `World.getChunkProvider` (World.java:210-213): the provider's
        // generator for the world seed. Population is still a recorded stub
        // (WP-1.7), see `ChunkGenerator::populate`.
        WorldSource::Generated => provider.get_chunk_provider(sc.seed),
    };
    let (handler, store): (Box<dyn SaveHandler>, Option<SharedSaveStore>) =
        match sc.save_format.as_deref().unwrap_or("none") {
            "none" => (Box::new(NoSave), None),
            "mcregion" => {
                // `new SaveOldDir(tempRoot, "world", true)` (HarnessWorld.create)
                let store = harness_store(mc_sim::save::MemoryStorage::default());
                let dir = SaveOldDir::new(store.clone(), "", WORLD_DIR)
                    .map_err(|e| GoldenError::other(e.to_string()))?;
                (Box::new(HarnessSaveHandler(dir)), Some(store))
            }
            other => return Err(GoldenError::other(format!("unknown save_format {other}"))),
        };
    let mut world = World::new(
        handler,
        "harness",
        provider,
        sc.seed,
        generator,
        // Overwritten by make_deterministic; vanilla draws these unseeded.
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    world.make_deterministic(sc.seed);
    Ok((world, store))
}

/// `MathHelper.floor_double`.
fn floor_double(v: f64) -> i32 {
    let i = v as i32;
    if v < i as f64 { i - 1 } else { i }
}

/// The `[x, z]` positions an op names (`TickScenario.positions`).
fn positions(op: &OpKind) -> Vec<[f64; 2]> {
    match *op {
        OpKind::SetBlock { x, z, .. }
        | OpKind::SetBlockSilent { x, z, .. }
        | OpKind::UseItem { x, z, .. } => vec![[x as f64, z as f64]],
        OpKind::Player { x, z, .. } => vec![[x, z]],
        OpKind::Fill { x0, z0, x1, z1, .. } => {
            vec![[x0 as f64, z0 as f64], [x1 as f64, z1 as f64]]
        }
        OpKind::Spawn { pos, .. } | OpKind::SetPos { pos, .. } => vec![[pos[0], pos[2]]],
        _ => Vec::new(),
    }
}

/// Before tick 0: the r = 2 chunk square around every position an op names,
/// in op file order (`TickScenario.implicitPreload`).
fn implicit_preload(world: &mut World, ops: &[Op], w: &mut TraceWriter) {
    let mut loaded = 0u64;
    for op in ops {
        for [x, z] in positions(&op.kind) {
            loaded += preload_square(world, floor_double(x) >> 4, floor_double(z) >> 4, 2);
        }
    }
    w.push(TraceLine::preload_implicit(loaded));
}

/// `getChunkFromChunkCoords` over the square, dz outer, dx inner, draining
/// the light queue after each chunk (Minecraft.java:1366-1373). Returns how
/// many chunks were not loaded before.
fn preload_square(world: &mut World, cx: i32, cz: i32, r: i32) -> u64 {
    let mut loaded = 0;
    for dz in -r..=r {
        for dx in -r..=r {
            if !world.chunk_exists(cx + dx, cz + dz) {
                loaded += 1;
            }
            world.get_chunk_from_chunk_coords(cx + dx, cz + dz);
            while world.updating_lighting() {}
        }
    }
    loaded
}

/// The harness's per-scenario state besides the world (`HarnessWorld`'s
/// fields).
struct Harness {
    /// The save's storage (`HarnessWorld.saveRoot`), for `"save_format":
    /// "mcregion"`.
    save_store: Option<SharedSaveStore>,
    /// `HarnessWorld.player`: the entity id of the `player` op's player.
    player: Option<i32>,
    /// `HarnessWorld.joinPlayerCounter` (Minecraft.joinPlayerCounter).
    join_player_counter: i32,
    /// The scenario's `difficulty`.
    difficulty: i32,
    /// The scenario's `mob_spawning`.
    mob_spawning: MobSpawning,
    /// The player's position when `run_tick` last found it in
    /// `loadedEntityList`: the oracle keeps the `HarnessPlayer` object after
    /// it leaves the list and still centres `randomDisplayUpdates` on it.
    player_pos: Option<(f64, f64, f64)>,
}

/// `HarnessWorld.runTick` (module docs).
// src: HarnessWorld.java runTick; Minecraft.java:1133-1174
fn run_tick(world: &mut World, h: &mut Harness) {
    if let Some(pid) = h.player {
        // src: Minecraft.java:1134-1139
        h.join_player_counter += 1;
        if h.join_player_counter == 30 {
            h.join_player_counter = 0;
            world.join_entity_in_surroundings(pid);
        }
    }
    world.difficulty_setting = h.difficulty; // src: Minecraft.java:1142
    if world.field_27172_i > 0 {
        world.field_27172_i -= 1; // src: Minecraft.java:1155-1157
    }
    world.update_entities(); // src: Minecraft.java:1159
    match h.mob_spawning {
        // src: Minecraft.java:1163
        MobSpawning::Default => world.set_allowed_mob_spawns(h.difficulty > 0, true),
        MobSpawning::None => world.set_allowed_mob_spawns(false, false),
    }
    world.tick(); // src: Minecraft.java:1164
    if let Some(pid) = h.player {
        // src: Minecraft.java:1167-1169
        match world.entity(pid) {
            Some(p) => h.player_pos = Some((p.base.pos_x, p.base.pos_y, p.base.pos_z)),
            None => {
                // The oracle reads the removed player's fields, which may have
                // moved during the update that removed it; the port only has
                // the position from the end of the previous tick.
                world.note_unimplemented(
                    "HarnessWorld.runTick randomDisplayUpdates around a player no longer in loadedEntityList",
                );
            }
        }
        if let Some((x, y, z)) = h.player_pos {
            world.random_display_updates(floor_double(x), floor_double(y), floor_double(z));
        }
    }
}

// ---- ops ------------------------------------------------------------------------

fn unsupported(t: u32, op: &str, why: &str) -> GoldenError {
    GoldenError::other(format!("t={t}: op {op} not supported yet: {why}"))
}

/// One op and its `op` line (`TickScenario.apply`).
fn apply(
    world: &mut World,
    h: &mut Harness,
    op: &Op,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
    let t = op.t;
    let line = |name: &str| TraceLine::op(t as i64, name);
    let out = match op.kind {
        OpKind::SetBlock { x, y, z, id, meta } => {
            // src: World.java:461-468
            line("set_block").with(
                "ok",
                world.set_block_and_metadata_with_notify(x, y, z, id, meta),
            )
        }
        OpKind::SetBlockSilent { x, y, z, id, meta } => {
            // src: World.java:370-384
            line("set_block_silent").with("ok", world.set_block_and_metadata(x, y, z, id, meta))
        }
        OpKind::Fill {
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
            id,
            meta,
        } => {
            if x1 < x0 || y1 < y0 || z1 < z0 {
                return Err(GoldenError::other(format!("t={t}: fill: max < min")));
            }
            let mut changed = 0u64;
            for x in x0..=x1 {
                for z in z0..=z1 {
                    for y in y0..=y1 {
                        if world.set_block_and_metadata(x, y, z, id, meta) {
                            changed += 1;
                        }
                    }
                }
            }
            line("fill").with("changed", changed)
        }
        OpKind::SetTime { time } => {
            world.set_world_time(time); // src: World.java:2274-2276
            line("set_time")
        }
        OpKind::SetWeather {
            rain_time,
            raining,
            thunder_time,
            thundering,
        } => {
            // src: WorldInfo.java:195-221 (plain setters)
            let info = &mut world.world_info;
            if let Some(v) = rain_time {
                info.rain_time = v;
            }
            if let Some(v) = raining {
                info.raining = v;
            }
            if let Some(v) = thunder_time {
                info.thunder_time = v;
            }
            if let Some(v) = thundering {
                info.thundering = v;
            }
            line("set_weather")
        }
        OpKind::Preload { cx, cz, radius } => {
            line("preload").with("loaded", preload_square(world, cx, cz, radius))
        }
        OpKind::Player {
            x,
            y,
            z,
            yaw,
            pitch,
        } => {
            if h.player.is_some() {
                return Err(GoldenError::other(format!("t={t}: only one player op")));
            }
            // `new HarnessPlayer(world)`: the pure EntityPlayer constructor
            // chain (mc_sim::entity::player); its `rand` is seeded with
            // mix(seed, id) at construction (`seedAfterConstruction`).
            let mut e = new_entity_player_sp(world);
            // `place`: setLocationAndAngles (Entity.java:669-676), yaw/pitch
            // defaulting to the constructor's.
            let yaw = yaw.unwrap_or(e.base.rotation_yaw);
            let pitch = pitch.unwrap_or(e.base.rotation_pitch);
            e.base.set_location_and_angles(x, y, z, yaw, pitch);
            let eid = e.id();
            let joined = world.spawn_entity_in_world(e); // src: World.java:896-918
            h.player = Some(eid);
            line("player").with("id", eid).with("joined", joined)
        }
        OpKind::Spawn {
            ref kind,
            pos,
            yaw,
            pitch,
            id,
            item,
            pickup_delay,
            direction,
            minecart_type,
        } => {
            let mut e = construct(
                world,
                h,
                t,
                kind,
                pos,
                item,
                pickup_delay,
                direction,
                minecart_type,
            )?;
            if placed_by_op(kind, direction, minecart_type) {
                // `place`: setLocationAndAngles (Entity.java:669-676), yaw/pitch
                // defaulting to the constructor's.
                let yaw = yaw.unwrap_or(e.base.rotation_yaw);
                let pitch = pitch.unwrap_or(e.base.rotation_pitch);
                e.base
                    .set_location_and_angles(pos[0], pos[1], pos[2], yaw, pitch);
            }
            let eid = e.id();
            if let Some(want) = id
                && want != eid
            {
                return Err(GoldenError::other(format!(
                    "t={t}: spawn: entity got id {eid}, scenario says {want}"
                )));
            }
            let kind_name = trace_kind(e.kind.java_class());
            let joined = if kind == "LightningBolt" {
                world.add_weather_effect(e) // src: World.java:891-894
            } else {
                world.spawn_entity_in_world(e) // src: World.java:896-918
            };
            line("spawn")
                .with("id", eid)
                .with("kind", kind_name)
                .with("joined", joined)
        }
        OpKind::SetMotion { id, motion } => {
            let e = loaded_entity(world, t, "set_motion", id)?;
            e.base.motion_x = motion[0];
            e.base.motion_y = motion[1];
            e.base.motion_z = motion[2];
            line("set_motion").with("id", id)
        }
        OpKind::SetPos { id, pos } => {
            let e = loaded_entity(world, t, "set_pos", id)?;
            e.base.set_position(pos[0], pos[1], pos[2]); // src: Entity.java:163-170
            line("set_pos").with("id", id)
        }
        OpKind::SetRotation { id, yaw, pitch } => {
            let e = loaded_entity(world, t, "set_rotation", id)?;
            e.base.rotation_yaw = yaw;
            e.base.rotation_pitch = pitch;
            line("set_rotation").with("id", id)
        }
        OpKind::UseItem {
            item_id,
            count,
            meta,
            x,
            y,
            z,
            side,
        } => {
            let Some(pid) = h.player else {
                return Err(GoldenError::other(format!(
                    "t={t}: use_item needs a player op first"
                )));
            };
            let r = world.with_entity_mut(pid, |world, p| {
                use_item(world, p, item_id.map(|id| (id, count, meta)), x, y, z, side)
            });
            let Some((placed, used, stack)) = r else {
                return Err(unsupported(
                    t,
                    "use_item",
                    "the player is no longer in loadedEntityList (the oracle keeps using its \
                     HarnessPlayer object)",
                ));
            };
            line("use_item")
                .with("placed", placed)
                .with("used", used)
                .with("stack", stack_json(stack))
        }
        OpKind::Attack {
            target_id,
            attacker_id,
            damage,
        } => attack(world, h, t, target_id, attacker_id, damage)?,
        OpKind::Interact {
            id,
            item_id,
            count,
            meta,
        } => {
            let Some(pid) = h.player else {
                return Err(GoldenError::other(format!(
                    "t={t}: interact needs a player op first"
                )));
            };
            loaded_entity(world, t, "interact", id)?;
            if id == pid {
                return Err(unsupported(
                    t,
                    "interact",
                    "the target is the player itself (both would have to be in hand)",
                ));
            }
            let r = world.with_entity_mut(pid, |world, p| {
                interact(world, p, id, item_id.map(|i| (i, count, meta)))
            });
            let Some((result, used, stack)) = r else {
                return Err(unsupported(
                    t,
                    "interact",
                    "the player is no longer in loadedEntityList (the oracle keeps using its \
                     HarnessPlayer object)",
                ));
            };
            line("interact")
                .with("id", id)
                .with("result", result)
                .with("used", used)
                .with("stack", stack_json(stack))
        }
        OpKind::Explode {
            x,
            y,
            z,
            size,
            flaming,
        } => {
            // src: World.java:1518-1528 (TickScenario.explode: the flaming case
            // seeds ExplosionRNG with mix(seed, -6), as Explosion::new does)
            let e = if flaming {
                world.new_explosion(None, x, y, z, size, true)
            } else {
                world.create_explosion(None, x, y, z, size)
            };
            line("explode").with("destroyed", e.destroyed_block_positions.len() as u64)
        }
        OpKind::Save { ref chunks } => {
            let Some(store) = h.save_store.clone() else {
                return Err(GoldenError::other(
                    "op save needs \"save_format\": \"mcregion\"",
                ));
            };
            // src: TierS.save: World.saveWorld(true, null) (World.java:278-291)
            world
                .try_save_world(true)
                .map_err(|e| GoldenError::other(format!("t={t}: save: {e}")))?;
            w.push(line("save"));
            return emit_tier_s(
                t as i64,
                &store,
                WORLD_DIR,
                chunks.as_deref(),
                true,
                world,
                w,
            );
        }
    };
    w.push(out);
    Ok(())
}

/// `TickScenario.attack`: the target (and the attacker, if given) found in
/// `loadedEntityList` by id. Attacker = the player: `swingItem`, then
/// `attackTargetEntityWithCurrentItem` (Minecraft.java:802-814) — not
/// transcribed yet (WP-2.5), an error. Otherwise
/// `target.attackEntityFrom(attacker or null, damage)` through the entity
/// dispatch, with the target taken out of its slot and the attacker passed
/// as a copy (as `EntityArrow` passes its shooter, `entity::arrow`) → `via`
/// `"direct"`, `result`.
fn attack(
    world: &mut World,
    h: &Harness,
    t: u32,
    target_id: i32,
    attacker_id: Option<i32>,
    damage: Option<i32>,
) -> Result<TraceLine, GoldenError> {
    loaded_entity(world, t, "attack", target_id)?;
    let attacker = match attacker_id {
        Some(a) => Some(loaded_entity(world, t, "attack", a)?.clone()),
        None => None,
    };
    let line = TraceLine::op(t as i64, "attack").with("target_id", target_id);
    if attacker.is_some() && attacker_id == h.player {
        return Err(unsupported(
            t,
            "attack",
            "attacker = the player: EntityPlayer.attackTargetEntityWithCurrentItem is not \
             transcribed (WP-2.5)",
        ));
    }
    let Some(damage) = damage else {
        return Err(GoldenError::other(format!(
            "t={t}: attack: \"damage\" is required unless the attacker is the player"
        )));
    };
    // src: Entity.java:746-749, EntityLiving.java:307-365 (TickScenario.attack)
    let result = world
        .with_entity_mut(target_id, |w, target| {
            mc_sim::entity::dispatch::attack_entity_from(target, w, attacker.as_ref(), damage)
        })
        .ok_or_else(|| unsupported(t, "attack", "the target is taken out of its slot"))?;
    Ok(line.with("via", "direct").with("result", result))
}

/// Whether the `spawn` op places the entity with `setLocationAndAngles`
/// (`TickScenario.placedByOp`): every kind but `Item`, `LightningBolt`,
/// `FishingHook`, a `Painting` with a `direction` and a `Minecart` with a
/// `type`, whose constructors take the position.
fn placed_by_op(kind: &str, direction: Option<i32>, minecart_type: Option<i32>) -> bool {
    match kind {
        "Item" | "LightningBolt" | "FishingHook" => false,
        "Painting" => direction.is_none(),
        "Minecart" => minecart_type.is_none(),
        _ => true,
    }
}

/// `TickScenario.construct`: the entity a `spawn` op makes, its `rand`
/// seeded with `mix(seed, id)` at construction (`seedAfterConstruction`;
/// the port's `EntityBase::new` does it, so the oracle's replays of
/// constructor draws are the port's constructor draws).
///
/// - `Item`: `EntityItem(world, x, y, z, stack)` (EntityItem.java:11-21, as
///   Block.java:355-364 drops it), `delayBeforeCanPickup = pickup_delay`.
/// - `Painting` with `direction`: `EntityPainting(world, floor(x), floor(y),
///   floor(z), direction)` (EntityPainting.java:22-45, its art draw on the
///   seeded `rand`).
/// - Harness-defined names (not in `EntityList`): `Egg` (`EntityEgg(World)`),
///   `Fireball` (`EntityFireball(World)`), `LightningBolt`
///   (`EntityLightningBolt(world, x, y, z)`, joined with `addWeatherEffect`;
///   refused at difficulty 2 and more, where its constructor places fire
///   with the oracle's unseeded `rand`), `FishingHook` (`EntityFish(world,
///   player)`, needs the `player` op; sets the player's `fishEntity`).
/// - `Minecart` with `type`: `EntityMinecart(world, x, y, z, type)`
///   (EntityMinecart.java:57-67, as ItemMinecart.java:16; no draws).
/// - Any other `EntityList` name: its `(World)` constructor; the mobs not
///   transcribed yet are an error naming the class.
#[allow(clippy::too_many_arguments)]
fn construct(
    world: &mut World,
    h: &Harness,
    t: u32,
    kind: &str,
    pos: [f64; 3],
    item: Option<crate::scenario::ItemSpec>,
    pickup_delay: Option<i32>,
    direction: Option<i32>,
    minecart_type: Option<i32>,
) -> Result<mc_sim::entity::Entity, GoldenError> {
    use mc_sim::entity::{
        arrow, boat, chicken, cow, egg, falling_sand, fireball, fish, lightning_bolt, minecart,
        painting, pig, sheep, snowball, tnt_primed, wolf,
    };
    let e = match kind {
        "Item" => {
            let Some(item) = item else {
                return Err(GoldenError::other(format!(
                    "t={t}: spawn Item needs \"item\""
                )));
            };
            let mut e = new_entity_item(
                world,
                pos[0],
                pos[1],
                pos[2],
                ItemStack::new(item.id, item.count, item.damage),
            );
            if let Some(d) = e.kind.item_mut() {
                d.delay_before_can_pickup = pickup_delay.unwrap_or(10);
            }
            e
        }
        "Egg" => egg::new_entity_egg_empty(world),
        "Fireball" => fireball::new_entity_fireball_empty(world),
        "LightningBolt" => {
            if h.difficulty >= 2 {
                return Err(GoldenError::other(format!(
                    "t={t}: spawn LightningBolt needs difficulty < 2 (its constructor places \
                     fire with the oracle's unseeded rand, tools/oracle/census.md §3)"
                )));
            }
            lightning_bolt::new_entity_lightning_bolt(world, pos[0], pos[1], pos[2])
        }
        "FishingHook" => {
            let Some(pid) = h.player else {
                return Err(GoldenError::other(format!(
                    "t={t}: spawn FishingHook needs a player op first"
                )));
            };
            world
                .with_entity_mut(pid, fish::new_entity_fish)
                .ok_or_else(|| {
                    unsupported(
                        t,
                        "spawn",
                        "FishingHook: the player is no longer in loadedEntityList",
                    )
                })?
        }
        "Painting" if direction.is_some() => painting::new_entity_painting(
            world,
            floor_double(pos[0]),
            floor_double(pos[1]),
            floor_double(pos[2]),
            direction.unwrap_or(0),
        ),
        "Minecart" if minecart_type.is_some() => {
            minecart::new_entity_minecart(world, pos[0], pos[1], pos[2], minecart_type.unwrap_or(0))
        }
        _ => {
            let class = ENTITY_LIST
                .iter()
                .find(|(name, _)| *name == kind)
                .map(|(_, class)| *class)
                .ok_or_else(|| {
                    GoldenError::other(format!(
                        "t={t}: unknown entity kind {kind} (EntityList names)"
                    ))
                })?;
            match class {
                "EntityArrow" => arrow::new_entity_arrow_empty(world),
                "EntitySnowball" => snowball::new_entity_snowball_empty(world),
                "EntityPainting" => painting::new_entity_painting_empty(world),
                "EntityTNTPrimed" => tnt_primed::new_entity_tnt_primed_empty(world),
                "EntityFallingSand" => falling_sand::new_entity_falling_sand_empty(world),
                "EntityPig" => pig::new_entity_pig(world),
                "EntitySheep" => sheep::new_entity_sheep(world),
                "EntityCow" => cow::new_entity_cow(world),
                "EntityChicken" => chicken::new_entity_chicken(world),
                "EntityWolf" => wolf::new_entity_wolf(world),
                "EntityMinecart" => minecart::new_entity_minecart_empty(world),
                "EntityBoat" => boat::new_entity_boat_empty(world),
                _ => {
                    return Err(unsupported(
                        t,
                        "spawn",
                        &format!("kind {kind}: the {class}(World) constructor is not transcribed"),
                    ));
                }
            }
        }
    };
    Ok(e)
}

/// `TickScenario.useItem`: a right click on face `side` of block `(x, y, z)`
/// as the player, with the hit given instead of a ray trace (the TILE branch
/// of `Minecraft.clickMouse(1)`, Minecraft.java:819-850, as the oracle
/// performs it). Returns `(placed, used, stack in slot currentItem)`.
///
/// Order, as the oracle's:
/// 1. `stack` (item id, count default 1, damage default 0) replaces hotbar
///    slot `inventory.currentItem`.
/// 2. `placed = PlayerController.sendPlaceBlock` ([`send_place_block`]) with
///    the slot's stack (`getCurrentItem`); if placed, `swingItem`
///    ([`swing_item`], EntityPlayer.java:483-486).
/// 3. With a stack: a stack left at size 0 is removed from the slot; if
///    nothing was placed, `used = sendUseItem` ([`send_use_item`]) on the
///    slot's stack, if any.
///
/// Stack identity: Java passes the slot's `ItemStack` object around, and
/// `Item.onItemUse` / `Item.onItemRightClick` mutate it in place (an
/// `ItemBlock` placement decrements `stackSize`). Stacks are values here, so
/// the held stack is passed as the hotbar *slot* that holds it
/// (`Option<usize>`, `None` for Java's `null`): [`item_stack_use_item`] runs
/// on the stack in that slot and the mutation lands in the slot, and the
/// size-0 test reads the slot, which is Java's `held` because nothing in
/// sendPlaceBlock stores another stack into the slot (no b1.7.3
/// `blockActivated` or `onItemUse` assigns `mainInventory`). Java's identity
/// test in `sendUseItem` (`var5 != var3`) is the explicit
/// [`RightClickResult`] instead. WP-2.6 transcribes `onItemUse` and
/// `onItemRightClick` against these signatures (`&mut ItemStack`), never on
/// a copy.
///
/// Recorded gaps (WP-2.6, item behaviour): `Item.onItemUse` (treated as
/// returning false, so `ItemStack.useItem` returns false and adds no stat)
/// and `Item.onItemRightClick` (treated as returning the same stack
/// unchanged, so `sendUseItem` returns false). `blockActivated` goes through
/// the block dispatch, whose untranscribed overrides record themselves.
fn use_item(
    world: &mut World,
    p: &mut mc_sim::entity::Entity,
    stack: Option<(i32, Option<i32>, Option<i32>)>,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> (bool, bool, Option<ItemStack>) {
    if let Some((id, count, meta)) = stack {
        let inv = &mut player_data(p).inventory;
        // src: ItemStack.java:33-38 (new ItemStack(id, count, damage))
        inv.main_inventory[inv.current_item as usize] =
            Some(ItemStack::new(id, count.unwrap_or(1), meta.unwrap_or(0)));
    }
    // `new PlayerControllerSP(null)`: its constructor only stores `mc`
    // (PlayerControllerSP.java), and sendPlaceBlock / sendUseItem are
    // PlayerController's (not overridden), which never read it.
    let mut not_placed = true;
    // held = inv.getCurrentItem(), as the slot holding it.
    let held = current_item_slot(p);
    let placed = send_place_block(world, p, held, x, y, z, side);
    if placed {
        not_placed = false;
        swing_item(p);
    }
    let mut used = false;
    if let Some(slot) = held {
        // `held.stackSize == 0`: the slot still holds `held` (use_item docs).
        let inv = &mut player_data(p).inventory;
        if inv.main_inventory[slot].is_some_and(|s| s.stack_size == 0) {
            inv.main_inventory[inv.current_item as usize] = None;
        }
        if not_placed {
            // again = inv.getCurrentItem(); again != null && sendUseItem(...)
            used = current_item_slot(p).is_some_and(|again| send_use_item(world, p, again));
        }
    }
    let current = player_data(p).inventory.get_current_item().copied();
    (placed, used, current)
}

/// `TickScenario.interact`: a right click on entity `target` as the player
/// `p` (in hand), the ENTITY branch of `Minecraft.clickMouse(1)`
/// (Minecraft.java:800-817, :846-850). Returns `(result, used, stack in slot
/// currentItem)`.
///
/// 1. `stack` (item id, count default 1, damage default 0) replaces hotbar
///    slot `inventory.currentItem`.
/// 2. `PlayerController.interactWithEntity` (PlayerController.java:88-90) =
///    `EntityPlayer.useCurrentItemOnEntity` (EntityPlayer.java:457-469):
///    `result = target.interact(player)` ([`dispatch::interact`], the target
///    taken out of its slot too); when false, the held stack's
///    `useItemOnEntity` on a living target (`Item.saddleEntity`, item
///    behaviour WP-2.6: recorded, taken as doing nothing, so no stack
///    reaches size 0 there).
/// 3. `used = sendUseItem` ([`send_use_item`]) on the slot's stack, if any:
///    the entity branch leaves `clickMouse`'s `var2` true.
fn interact(
    world: &mut World,
    p: &mut mc_sim::entity::Entity,
    target: i32,
    stack: Option<(i32, Option<i32>, Option<i32>)>,
) -> (bool, bool, Option<ItemStack>) {
    if let Some((id, count, meta)) = stack {
        let inv = &mut player_data(p).inventory;
        // src: ItemStack.java:33-38 (new ItemStack(id, count, damage))
        inv.main_inventory[inv.current_item as usize] =
            Some(ItemStack::new(id, count.unwrap_or(1), meta.unwrap_or(0)));
    }
    let result = world
        .with_entity_mut(target, |w, e| {
            let r = dispatch::interact(e, w, p);
            if !r && current_item_slot(p).is_some() && e.kind.living().is_some() {
                // src: EntityPlayer.java:459-466, ItemStack.java:170-172
                w.note_unimplemented("Item.saddleEntity");
            }
            r
        })
        .expect("the interact target was found in its slot");
    let used = current_item_slot(p).is_some_and(|again| send_use_item(world, p, again));
    let current = player_data(p).inventory.get_current_item().copied();
    (result, used, current)
}

/// The `EntityPlayer` level of the `use_item` player.
fn player_data(p: &mut mc_sim::entity::Entity) -> &mut mc_sim::entity::PlayerData {
    p.kind
        .player_mut()
        .expect("the player op's entity is an EntityPlayer")
}

/// `inventory.getCurrentItem()` (InventoryPlayer.java:15-17) as the slot that
/// holds the stack: `Some(currentItem)` when it returns a stack, `None` for
/// `null`.
fn current_item_slot(p: &mut mc_sim::entity::Entity) -> Option<usize> {
    let inv = &player_data(p).inventory;
    inv.get_current_item().map(|_| inv.current_item as usize)
}

/// `PlayerController.sendPlaceBlock(player, world, stack, x, y, z, side)`:
/// the block's `blockActivated`, else `stack.useItem(...)`. `stack` is the
/// slot of `mainInventory` holding Java's `var3` (`None` for `null`; see
/// [`use_item`]).
// src: PlayerController.java:75-82
fn send_place_block(
    world: &mut World,
    p: &mut mc_sim::entity::Entity,
    stack: Option<usize>,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let var8 = world.get_block_id(x, y, z);
    if var8 > 0 && block_dispatch::block_activated(world, var8, x, y, z, p) {
        true
    } else {
        match stack {
            None => false,
            Some(slot) => {
                // The stack is moved out of its slot while the item runs, so
                // that it and the player can both be borrowed mutably, and
                // put back afterwards: whatever onItemUse does to it lands
                // in the slot, as on Java's shared object.
                let mut s = player_data(p).inventory.main_inventory[slot]
                    .take()
                    .expect("sendPlaceBlock's stack is the slot's stack");
                let r = item_stack_use_item(&mut s, world, p, x, y, z, side);
                player_data(p).inventory.main_inventory[slot] = Some(s);
                r
            }
        }
    }
}

/// `ItemStack.useItem(player, world, x, y, z, side)`:
/// `getItem().onItemUse(...)`, and on success
/// `player.addStat(StatList.objectUseStats[itemID], 1)`. `Item.onItemUse` is
/// item behaviour (WP-2.6): recorded and treated as false (docs of
/// [`use_item`]), so no stat is added. While it runs, `stack` is out of the
/// player's inventory (the slot is empty; [`send_place_block`]).
// src: ItemStack.java:58-65
fn item_stack_use_item(
    stack: &mut ItemStack,
    world: &mut World,
    p: &mut mc_sim::entity::Entity,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    let _ = (stack, p, x, y, z, side);
    world.note_unimplemented("Item.onItemUse");
    false
}

/// What `ItemStack.useItemRightClick` returned, standing in for Java's
/// identity test `var5 != var3` in `sendUseItem`.
enum RightClickResult {
    /// `var3` itself (possibly mutated in place, in the slot).
    Same,
    /// Another object (`ItemBucket`, `ItemSoup`, ...). Never `null` in
    /// b1.7.3: no `onItemRightClick` override returns `null`.
    #[expect(
        dead_code,
        reason = "built by the onItemRightClick overrides (WP-2.6); read by send_use_item"
    )]
    Replaced(ItemStack),
}

/// `ItemStack.useItemRightClick(world, player)`:
/// `getItem().onItemRightClick(this, world, player)`. Item behaviour
/// (WP-2.6): recorded and taken to return `this` unchanged.
// src: ItemStack.java:71-73
fn item_stack_use_item_right_click(
    stack: &mut ItemStack,
    world: &mut World,
    p: &mut mc_sim::entity::Entity,
) -> RightClickResult {
    let _ = (stack, p);
    world.note_unimplemented("Item.onItemRightClick");
    RightClickResult::Same
}

/// `PlayerController.sendUseItem(player, world, stack)`: `useItemRightClick`;
/// a changed stack (another object, or another size) replaces the slot and
/// counts as used. `stack` is the slot holding Java's `var3` (see
/// [`use_item`]).
// src: PlayerController.java:47-61
fn send_use_item(world: &mut World, p: &mut mc_sim::entity::Entity, stack: usize) -> bool {
    let mut var3 = player_data(p).inventory.main_inventory[stack]
        .take()
        .expect("sendUseItem's stack is the slot's stack");
    let var4 = var3.stack_size;
    let r = item_stack_use_item_right_click(&mut var3, world, p);
    let inv = &mut player_data(p).inventory;
    // var3 goes back into its slot first: Java's var3 never left it.
    inv.main_inventory[stack] = Some(var3);
    let var5 = match r {
        RightClickResult::Same if var3.stack_size == var4 => return false,
        RightClickResult::Same => var3,
        RightClickResult::Replaced(s) => s,
    };
    let cur = inv.current_item as usize;
    inv.main_inventory[cur] = Some(var5);
    if var5.stack_size == 0 {
        inv.main_inventory[cur] = None;
    }
    true
}

/// `Dumps.stack`: `{id, count, damage}` or `null`.
fn stack_json(s: Option<ItemStack>) -> Value {
    match s {
        None => Value::Null,
        Some(s) => serde_json::json!({
            "id": s.item_id,
            "count": s.stack_size,
            "damage": s.item_damage,
        }),
    }
}

/// The entity `id` in `loadedEntityList` (`TickScenario.entity`).
fn loaded_entity<'w>(
    world: &'w mut World,
    t: u32,
    op: &str,
    id: i32,
) -> Result<&'w mut mc_sim::entity::Entity, GoldenError> {
    world.entity_mut(id).ok_or_else(|| {
        GoldenError::other(format!(
            "t={t}: {op}: no entity with id {id} in loadedEntityList"
        ))
    })
}

// ---- dumps ----------------------------------------------------------------------

fn not_loaded(what: &str, x: i32, y: i32, z: i32) -> GoldenError {
    GoldenError::other(format!(
        "{what} at {x},{y},{z}: y outside 0..127 or chunk not loaded (dumps never load chunks)"
    ))
}

/// One due dump (`TickScenario.dump`, `Dumps`).
fn dump(world: &mut World, d: &Dump, t: u32, w: &mut TraceWriter) -> Result<(), GoldenError> {
    let ti = t as i64;
    match d.what {
        DumpKind::ChunkDigest { x, z, full } => {
            let a = world.chunk_arrays(x, z).ok_or_else(|| {
                GoldenError::other(format!("chunk_digest {x},{z}: chunk not loaded"))
            })?;
            let arrays = ChunkArrays {
                blocks: a.blocks,
                data: a.data,
                skylight: a.skylight,
                blocklight: a.blocklight,
                heightmap: a.heightmap,
            };
            w.push(TraceLine::chunk_digest_tick(ti, x, z, &arrays));
            if full {
                w.push(TraceLine::chunk_full_tick(ti, x, z, &arrays));
            }
        }
        DumpKind::Block { x, y, z } => {
            let b = world
                .block_dump(x, y, z)
                .ok_or_else(|| not_loaded("block dump", x, y, z))?;
            w.push(TraceLine::block(
                ti,
                x,
                y,
                z,
                b.id,
                b.meta,
                b.skylight,
                b.blocklight,
            ));
        }
        DumpKind::BlockBox {
            x0,
            y0,
            z0,
            x1,
            y1,
            z1,
        } => {
            if x1 < x0 || y1 < y0 || z1 < z0 {
                return Err(GoldenError::other("block_box: max < min"));
            }
            let n = (x1 - x0 + 1) as i64 * (y1 - y0 + 1) as i64 * (z1 - z0 + 1) as i64;
            if n > 262144 {
                return Err(GoldenError::other("block_box larger than 262144 blocks"));
            }
            let n = n as usize;
            let (mut ids, mut metas, mut sky, mut light) = (
                Vec::with_capacity(n),
                Vec::with_capacity(n),
                Vec::with_capacity(n),
                Vec::with_capacity(n),
            );
            for x in x0..=x1 {
                for z in z0..=z1 {
                    for y in y0..=y1 {
                        let b = world
                            .block_dump(x, y, z)
                            .ok_or_else(|| not_loaded("block_box", x, y, z))?;
                        // Java `(byte)` casts: the low 8 bits.
                        ids.push(b.id as u8);
                        metas.push(b.meta as u8);
                        sky.push(b.skylight as u8);
                        light.push(b.blocklight as u8);
                    }
                }
            }
            w.push(TraceLine::blocks_full(
                ti,
                [x0, y0, z0],
                [x1, y1, z1],
                &ids,
                &metas,
                &sky,
                &light,
            ));
        }
        DumpKind::ScheduledTicks { full } => {
            let entries: Vec<ScheduledTick> = world
                .scheduled_ticks()
                .map(|e| ScheduledTick {
                    x: e.x_coord,
                    y: e.y_coord,
                    z: e.z_coord,
                    block_id: e.block_id,
                    time: e.scheduled_time,
                })
                .collect();
            w.push(TraceLine::scheduled_ticks(ti, &entries, full));
        }
        DumpKind::EntityState { ref which } => entity_state(world, which, ti, w)?,
        DumpKind::WorldTime => w.push(TraceLine::world_time(ti, world.get_world_time())),
        DumpKind::RayTrace {
            from,
            to,
            stop_on_liquid,
        } => {
            require_chunks_loaded(
                world,
                [floor_double(from[0]), floor_double(from[2])],
                [floor_double(to[0]), floor_double(to[2])],
                "ray_trace",
            )?;
            // `World.rayTraceBlocks_do` on fresh vectors (Dumps.rayTrace).
            let m = world.ray_trace_between(from, to, stop_on_liquid);
            let hit = match m {
                None => Value::Null,
                Some(m) => serde_json::json!({
                    "type": match m.type_of_hit {
                        EnumMovingObjectType::Tile => "TILE",
                        EnumMovingObjectType::Entity => "ENTITY",
                    },
                    "x": m.block_x,
                    "y": m.block_y,
                    "z": m.block_z,
                    "side": m.side_hit,
                    "hit_vec": vec3_bits(m.hit_vec.x_coord, m.hit_vec.y_coord, m.hit_vec.z_coord),
                    "entity": m.entity_hit,
                }),
            };
            w.push(
                TraceLine::new(ti, "ray_trace")
                    .with("from", vec3_bits(from[0], from[1], from[2]))
                    .with("to", vec3_bits(to[0], to[1], to[2]))
                    .with("stop_on_liquid", stop_on_liquid)
                    .with("hit", hit),
            );
        }
        DumpKind::Power { x, y, z } => {
            require_chunks_loaded(world, [x - 2, z - 2], [x + 2, z + 2], "power")?;
            // Java evaluates the two in this order (Dumps.power).
            let powered = world.is_block_getting_powered(x, y, z);
            let indirectly = world.is_block_indirectly_getting_powered(x, y, z);
            w.push(
                TraceLine::new(ti, "power")
                    .with("x", x)
                    .with("y", y)
                    .with("z", z)
                    .with("powered", powered)
                    .with("indirectly_powered", indirectly),
            );
        }
        DumpKind::Weather => {
            // Dumps.weather: WorldInfo's timers and flags, World's strengths.
            let i = &world.world_info;
            w.push(
                TraceLine::new(ti, "weather")
                    .with("rain_time", i.rain_time)
                    .with("raining", i.raining)
                    .with("thunder_time", i.thunder_time)
                    .with("thundering", i.thundering)
                    .with_f32("prev_rain_strength", world.prev_raining_strength)
                    .with_f32("rain_strength", world.raining_strength)
                    .with_f32("prev_thunder_strength", world.prev_thundering_strength)
                    .with_f32("thunder_strength", world.thundering_strength),
            );
        }
        DumpKind::RngState => {
            let r = world.rng_state();
            w.push(TraceLine::rng_state(ti, r.world_rand, r.lcg, r.math_random));
        }
    }
    Ok(())
}

/// `Dumps.requireChunksLoaded`: every chunk of the block box spanned by the
/// two `[x, z]` corners is loaded.
fn require_chunks_loaded(
    world: &World,
    a: [i32; 2],
    b: [i32; 2],
    what: &str,
) -> Result<(), GoldenError> {
    for cx in (a[0].min(b[0]) >> 4)..=(a[0].max(b[0]) >> 4) {
        for cz in (a[1].min(b[1]) >> 4)..=(a[1].max(b[1]) >> 4) {
            if !world.chunk_exists(cx, cz) {
                return Err(GoldenError::other(format!(
                    "{what}: chunk {cx},{cz} not loaded (dumps never load chunks)"
                )));
            }
        }
    }
    Ok(())
}

/// `Dumps.vec`: three `f64` bit-pattern strings.
fn vec3_bits(x: f64, y: f64, z: f64) -> Value {
    Value::Array(vec![
        crate::Bits64::from(x).into(),
        crate::Bits64::from(y).into(),
        crate::Bits64::from(z).into(),
    ])
}

/// `Dumps.entityState`: `entity` lines in `loadedEntityList` order, filtered
/// to `which`, then `entity_absent` for each requested id not seen.
fn entity_state(
    world: &World,
    which: &EntityWhich,
    t: i64,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
    let mut seen = Vec::new();
    for (idx, slot) in world.loaded_entity_list.iter().enumerate() {
        let wanted = match which {
            EntityWhich::All => true,
            EntityWhich::Ids(ids) => ids.contains(&slot.entity_id),
        };
        if !wanted {
            continue;
        }
        let e = slot.entity.as_deref().ok_or_else(|| {
            GoldenError::other(format!(
                "entity_state: entity {} is taken out of its slot",
                slot.entity_id
            ))
        })?;
        w.push(entity_line(t, idx as u32, e)?);
        seen.push(slot.entity_id);
    }
    if let EntityWhich::Ids(ids) = which {
        for &id in ids {
            if !seen.contains(&id) {
                w.push(TraceLine::entity_absent(t, id));
            }
        }
    }
    Ok(())
}

/// One `entity` line from [`entity_state_fields`] (README field order).
fn entity_line(t: i64, idx: u32, e: &mc_sim::entity::Entity) -> Result<TraceLine, GoldenError> {
    let fields = entity_state_fields(e);
    let mut line = EntityLine::new(t, idx, e.id(), trace_kind(e.kind.java_class()));
    for (name, v) in fields {
        line = match (name, v) {
            // Set by EntityLine::new.
            ("id" | "kind", _) => line,
            (_, StateValue::F64(x)) => line.f64(name, x),
            (_, StateValue::F32(x)) => line.f32(name, x),
            (_, StateValue::Bool(b)) => line.bool(name, b),
            (_, StateValue::Int(i)) => line.int(name, i),
            ("state", StateValue::Object(o)) => line.state(state_object(o)?),
            (_, other) => {
                return Err(GoldenError::other(format!(
                    "entity line: unexpected value for {name}: {other:?}"
                )));
            }
        };
    }
    Ok(line.build())
}

/// The `state` object: floats as bit patterns, stacks as `{id,count,damage}`
/// or `null`, byte blobs as their BLAKE3 hex (`inventory`).
fn state_object(fields: Vec<(&'static str, StateValue)>) -> Result<Value, GoldenError> {
    let mut m = Map::new();
    for (name, v) in fields {
        m.insert(name.to_string(), state_value(v)?);
    }
    Ok(Value::Object(m))
}

fn state_value(v: StateValue) -> Result<Value, GoldenError> {
    Ok(match v {
        StateValue::F64(x) => crate::Bits64::from(x).into(),
        StateValue::F32(x) => crate::Bits32::from(x).into(),
        StateValue::Bool(b) => b.into(),
        StateValue::Int(i) => i.into(),
        StateValue::Str(s) => s.into(),
        StateValue::Stack(s) => stack_json(s),
        StateValue::Bytes(b) => blake3_hex(&b).into(),
        StateValue::Object(o) => state_object(o)?,
    })
}
