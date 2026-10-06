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
//!    [`EmptyChunkProvider`] for `"world": "empty"`, then
//!    [`World::make_deterministic`] (`makeDeterministic` + `resetStatics`).
//!    `"world": "generated"` and `"save_format": "mcregion"` are errors: the
//!    terrain generator (WP-1.4/1.7) and McRegion saving (WP-1.9) are not
//!    transcribed yet.
//! 3. The implicit preload (`TickScenario.implicitPreload`) and its
//!    `preload` line.
//! 4. For each tick `t`: the ops with this `t` in file order (a stable sort
//!    by `t`, as the oracle's `List.sort`), each emitting its `op` line;
//!    `HarnessWorld.runTick`; `world.updatingLighting()` once; then the dumps
//!    due at `t` in scenario order.
//!
//! `runTick` (HarnessWorld.java, Minecraft.java:1133-1174) as implemented in
//! `run_tick`: `difficultySetting = difficulty`; `field_27172_i`
//! countdown; `updateEntities`; `setAllowedMobSpawns(difficulty > 0, true)`;
//! `tick`. The player-only steps (`joinPlayerCounter` /
//! `joinEntityInSurroundings`, `randomDisplayUpdates`) never run, because the
//! `player` op is not supported yet (below); the frame-start pool resets
//! (`AxisAlignedBB.clearBoundingBoxPool`, `Vec3D.initialize`) have no
//! observable state in the port.
//!
//! # Ops and dumps
//!
//! | Op / dump | mc-sim call |
//! |---|---|
//! | `set_block` | [`World::set_block_and_metadata_with_notify`] → `ok` |
//! | `set_block_silent` | [`World::set_block_and_metadata`] → `ok` |
//! | `fill` | `set_block_and_metadata` over the box, x outer, z middle, y inner → `changed` |
//! | `set_time` | [`World::set_world_time`] |
//! | `preload` | [`World::get_chunk_from_chunk_coords`] over the square, dz outer, dx inner, each followed by draining [`World::updating_lighting`] → `loaded` |
//! | `spawn` `Item` | [`new_entity_item`], `delayBeforeCanPickup = pickup_delay` (10), [`World::spawn_entity_in_world`] → `id`, `kind`, `joined` |
//! | `set_motion` / `set_pos` / `set_rotation` | fields of the loaded entity / [`EntityBase::set_position`](mc_sim::entity::EntityBase::set_position) / fields |
//! | `chunk_digest` | [`World::chunk_arrays`] → [`TraceLine::chunk_digest_tick`] (+ [`TraceLine::chunk_full_tick`]) |
//! | `block` / `block_box` | [`World::block_dump`] |
//! | `scheduled_ticks` | [`World::scheduled_ticks`] |
//! | `entity_state` | [`mc_sim::golden::entity_state_fields`] in `loadedEntityList` order |
//! | `world_time` / `rng_state` | [`World::get_world_time`] / [`World::rng_state`] |
//!
//! Not supported yet, each an error naming the op and the missing Java code:
//! `player` (`EntityPlayerSP`/`HarnessPlayer` construction, WP-2.5),
//! `spawn` of any kind but `Item` (the `Entity<kind>(World)` constructors),
//! `use_item` (`PlayerController`, items), `attack` (`attackEntityFrom` of
//! living entities, the player), `save` (Tier S, WP-1.9).
//!
//! # Entity kind names
//!
//! `kind` in `spawn` lines and `entity` lines is the Java class simple name
//! ([`EntityKindData::java_class`](mc_sim::entity::EntityKindData::java_class)).
//! The oracle's player is its own subclass `HarnessPlayer` of
//! `EntityPlayerSP`, so the port's `EntityPlayerSP` is reported as
//! `HarnessPlayer` ([`trace_kind`]).

use mc_sim::entity::item::new_entity_item;
use mc_sim::golden::{StateValue, entity_state_fields};
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension as SimDimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};
use serde_json::{Map, Value};

use crate::scenario::{
    Dimension, Dump, DumpKind, EntityWhich, Op, OpKind, TickScenario, WorldSource,
};
use crate::trace::{ChunkArrays, EntityLine, ScheduledTick};
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
    let mut world = build_world(sc)?;

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
            apply(&mut world, ops[next], w)?;
            next += 1;
        }
        run_tick(&mut world, sc.difficulty);
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

/// `HarnessWorld.create`: `new World(handler, "harness", provider, seed)`,
/// then `makeDeterministic` / `resetStatics`.
fn build_world(sc: &TickScenario) -> Result<World, GoldenError> {
    let generator = match sc.world {
        WorldSource::Empty => ChunkGenerator::Empty(EmptyChunkProvider),
        WorldSource::Generated => {
            return Err(GoldenError::other(
                "\"world\": \"generated\": worldgen not transcribed (Phase 1: \
                 ChunkProviderGenerate WP-1.4, ChunkProviderHell WP-1.7)",
            ));
        }
    };
    match sc.save_format.as_deref().unwrap_or("none") {
        "none" => {}
        "mcregion" => {
            return Err(GoldenError::other(
                "\"save_format\": \"mcregion\": SaveOldDir / McRegionChunkLoader not transcribed (WP-1.9)",
            ));
        }
        other => return Err(GoldenError::other(format!("unknown save_format {other}"))),
    }
    let dimension = match sc.dimension {
        Dimension::Overworld => SimDimension::Surface,
        Dimension::Nether => SimDimension::Hell,
    };
    let mut world = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(dimension),
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
    Ok(world)
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

/// `HarnessWorld.runTick` without a player (module docs).
// src: HarnessWorld.java runTick; Minecraft.java:1133-1174
fn run_tick(world: &mut World, difficulty: i32) {
    // No player: Minecraft.java:1134-1139 (joinPlayerCounter) does not run.
    world.difficulty_setting = difficulty; // src: Minecraft.java:1142
    if world.field_27172_i > 0 {
        world.field_27172_i -= 1; // src: Minecraft.java:1155-1157
    }
    world.update_entities(); // src: Minecraft.java:1159
    world.set_allowed_mob_spawns(difficulty > 0, true); // src: Minecraft.java:1163
    world.tick(); // src: Minecraft.java:1164
    // No player: randomDisplayUpdates (Minecraft.java:1167-1169) does not run.
}

// ---- ops ------------------------------------------------------------------------

fn unsupported(t: u32, op: &str, why: &str) -> GoldenError {
    GoldenError::other(format!("t={t}: op {op} not supported yet: {why}"))
}

/// One op and its `op` line (`TickScenario.apply`).
fn apply(world: &mut World, op: &Op, w: &mut TraceWriter) -> Result<(), GoldenError> {
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
        OpKind::Preload { cx, cz, radius } => {
            line("preload").with("loaded", preload_square(world, cx, cz, radius))
        }
        OpKind::Player { .. } => {
            return Err(unsupported(
                t,
                "player",
                "HarnessPlayer / EntityPlayerSP(World) construction and EntityPlayer are not \
                 transcribed (WP-2.5)",
            ));
        }
        OpKind::Spawn {
            ref kind,
            pos,
            id,
            item,
            pickup_delay,
            ..
        } => {
            if kind != "Item" {
                let class = ENTITY_LIST
                    .iter()
                    .find(|(name, _)| name == kind)
                    .map(|(_, class)| *class)
                    .ok_or_else(|| {
                        GoldenError::other(format!(
                            "t={t}: unknown entity kind {kind} (EntityList names)"
                        ))
                    })?;
                return Err(unsupported(
                    t,
                    "spawn",
                    &format!("kind {kind}: the {class}(World) constructor is not transcribed"),
                ));
            }
            let Some(item) = item else {
                return Err(GoldenError::other(format!(
                    "t={t}: spawn Item needs \"item\""
                )));
            };
            // src: EntityItem.java:11-21 (as Block.java:355-364 drops it);
            // no setLocationAndAngles for Item (TickScenario.apply).
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
            let eid = e.id();
            if let Some(want) = id
                && want != eid
            {
                return Err(GoldenError::other(format!(
                    "t={t}: spawn: entity got id {eid}, scenario says {want}"
                )));
            }
            let kind_name = trace_kind(e.kind.java_class());
            let joined = world.spawn_entity_in_world(e); // src: World.java:896-918
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
        OpKind::UseItem { .. } => {
            return Err(unsupported(
                t,
                "use_item",
                "needs the player and PlayerController.sendPlaceBlock / sendUseItem, \
                 Item.onItemUse (WP-2.5, item table WP-1.2)",
            ));
        }
        OpKind::Attack { .. } => {
            return Err(unsupported(
                t,
                "attack",
                "EntityPlayer.attackTargetEntityWithCurrentItem and \
                 EntityLiving.attackEntityFrom are not transcribed (WP-2.2, WP-2.5)",
            ));
        }
        OpKind::Save { .. } => {
            return Err(unsupported(
                t,
                "save",
                "World.saveWorld through SaveOldDir / McRegionChunkLoader and the Tier S \
                 dumps (WP-1.9)",
            ));
        }
    };
    w.push(out);
    Ok(())
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
        DumpKind::RngState => {
            let r = world.rng_state();
            w.push(TraceLine::rng_state(ti, r.world_rand, r.lcg, r.math_random));
        }
    }
    Ok(())
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
        StateValue::Stack(None) => Value::Null,
        StateValue::Stack(Some(s)) => serde_json::json!({
            "id": s.item_id,
            "count": s.stack_size,
            "damage": s.item_damage,
        }),
        StateValue::Bytes(b) => blake3_hex(&b).into(),
        StateValue::Object(o) => state_object(o)?,
    })
}
