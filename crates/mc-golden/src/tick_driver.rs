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
//!    "The `save` op". `"world": "load"` (`TickScenario.load`) copies the
//!    save directory `dir` (relative to the fixture directory, which only
//!    [`drive_in`] knows) into such a storage and opens it with
//!    [`World::new_from_save`], then `makeDeterministic` with the scenario
//!    seed.
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
//! | `player` | [`new_entity_player_sp`] (the `EntityPlayer` constructor chain), username `"Player"` (HarnessPlayer's), the `inventory` stacks ([`InventoryPlayer::set_inventory_slot_contents`](mc_sim::inventory::InventoryPlayer::set_inventory_slot_contents)), `setLocationAndAngles`, [`World::spawn_entity_in_world`] → `id`, `joined` |
//! | `input` | the player's [`PlayerSpData::movement_input`](mc_sim::entity::PlayerSpData) = [`MovementInput::from_keys`]; `hotbar` → `inventory.currentItem` |
//! | `dig` | `dig`: [`PlayerControllerSp::update_controller`], on a press or every 5 ticks held [`swing_item`] + [`PlayerControllerSp::click_block`], then [`PlayerControllerSp::send_block_removing`] → `x`, `y`, `z`, `clicked`, `progress`, `broken`; a tick without `dig` releases ([`PlayerControllerSp::reset_block_removing`]) |
//! | `sleep` | [`block_dispatch::block_activated`] as the player → `result`, `status` ([`PlayerSpData::last_sleep_status`](mc_sim::entity::PlayerSpData)) |
//! | `spawn` | `construct` (the kind's constructor; `Item`: [`new_entity_item`] and `delayBeforeCanPickup = pickup_delay` (10); the harness-defined `Egg`, `Fireball`, `LightningBolt`, `FishingHook`, a `Painting` with `direction` and a `Minecart` with `type`), `setLocationAndAngles` unless the constructor took the position, then [`World::spawn_entity_in_world`] (`LightningBolt`: [`World::add_weather_effect`]) → `id`, `kind`, `joined` |
//! | `set_motion` / `set_pos` / `set_rotation` | fields of the loaded entity / [`EntityBase::set_position`](mc_sim::entity::EntityBase::set_position) / fields |
//! | `attack` | `attack`: the player as attacker: [`swing_item`], [`player_controller::attack_entity`] (both out of their slots) → `target_id`, `via` `"player"`; otherwise `target.attackEntityFrom(attacker or null, damage)` through [`mc_sim::entity::dispatch::attack_entity_from`] (target out of its slot, attacker a copy) → `target_id`, `via` `"direct"`, `result` |
//! | `interact` | `TickScenario.interact` in `interact`: the stack into slot `currentItem`, `EntityPlayer.useCurrentItemOnEntity` ([`use_current_item_on_entity`] with the player and the target both in hand; when false, `ItemStack.useItemOnEntity` on a living target: `Item.saddleEntity`, item behaviour WP-2.6), then `sendUseItem` ([`send_use_item`]) → `id`, `result`, `used`, `stack` |
//! | `explode` | [`World::create_explosion`] (no exploder), or [`World::new_explosion`] with `flaming` → `destroyed` (the size of `destroyedBlockPositions`) |
//! | `use_item` | `TickScenario.useItem` in `use_item`: the stack into slot `currentItem`, `PlayerController.sendPlaceBlock` (block dispatch `block_activated`, else `ItemStack.useItem`), `swingItem`, the size-0 removal, `sendUseItem` → `placed`, `used`, `stack`. `Item.onItemUse` / `Item.onItemRightClick` are the item behaviour dispatch ([`mc_sim::item::behaviour`], WP-2.6) |
//! | `craft` | `craft`: [`CraftingManager::find_matching_recipe`] over a new [`InventoryCrafting`] (3×3 or 2×2) filled in slot order → `result` |
//! | `slot_click` | `open_container` (the player's `ContainerPlayer`, built by the `player` op and held in the harness, or the last opened / a new workbench, chest (on [`chest_inventory`]), furnace or dispenser container), then [`Container::slot_click`] with the player in hand → `slot`, `result`, `stack`, `held` |
//! | `inventory` | `inventory_line`: the player's `InventoryPlayer` and its `ContainerPlayer` grid and result |
//! | `chunk_digest` | [`World::chunk_arrays`] → [`TraceLine::chunk_digest_tick`] (+ [`TraceLine::chunk_full_tick`]) |
//! | `block` / `block_box` | [`World::block_dump`] |
//! | `scheduled_ticks` | [`World::scheduled_ticks`] |
//! | `entity_state` | [`mc_sim::golden::entity_state_fields`] in `loadedEntityList` order |
//! | `world_time` / `rng_state` | [`World::get_world_time`] / [`World::rng_state`] |
//! | `weather` | [`World::world_info`] timers and flags, the `World` rain/thunder strengths |
//! | `player_state` | the player's [`PlayerData`](mc_sim::entity::PlayerData) fields, box and offsets (`player_state_line`) |
//! | `map_data` | `map_data_line`: the `MapData` `map_<id>` loaded in [`World::field_28108_z`] (no file read; WP-2.6) |
//! | `ray_trace` | [`World::ray_trace_between`] (`rayTraceBlocks_do` on fresh vectors) |
//! | `power` | [`World::is_block_getting_powered`], then [`World::is_block_indirectly_getting_powered`] |
//! | `save` | [`World::try_save_world`]`(true)`, then [`emit_tier_s`] on the in-memory save (masked) |
//! | `reload` | `reload`: [`World::try_save_world`]`(true)`, [`World::new_from_save`] on a new [`SaveOldDir`] over the same store, [`World::set_chunk_generator`] (empty worlds), [`World::adopt_clock_seeded_state`], [`World::adopt_statics`], the old [`World::loaded_chunk_coords`] loaded again, a new player through [`World::spawn_player_with_loaded_chunks`] → `chunks`, `entities`, `tile_entities`, `player` |
//!
//! Not supported yet, each an error naming the op and the missing Java code:
//! `spawn` of the mobs other than `Pig`, `Sheep`, `Cow`, `Chicken`, `Wolf`,
//! `Minecart` and `Boat` (their `(World)` constructors); an op that needs
//! the player after it left `loadedEntityList` (the oracle keeps the
//! `HarnessPlayer` object).
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
use mc_sim::crafting::CraftingManager;
use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::player::{
    MovementInput, new_entity_player_sp, swing_item, use_current_item_on_entity,
};
use mc_sim::golden::{StateValue, entity_state_fields};
use mc_sim::inventory::InventoryRef;
use mc_sim::inventory::container::chest::new_container_chest;
use mc_sim::inventory::container::dispenser::new_container_dispenser;
use mc_sim::inventory::container::furnace::new_container_furnace;
use mc_sim::inventory::container::player::new_container_player;
use mc_sim::inventory::container::workbench::new_container_workbench;
use mc_sim::inventory::container::{Container, ContainerKind};
use mc_sim::inventory::crafting::InventoryCrafting;
use mc_sim::inventory::large_chest::chest_inventory;
use mc_sim::item::ItemStack;
use mc_sim::player_controller::{self, PlayerControllerSp, send_place_block, send_use_item};
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

/// Drives a `kind: "tick"` scenario (module docs). Other kinds are an error,
/// and so is `"world": "load"`, which needs the fixture directory
/// ([`drive_in`]).
pub fn drive(scenario: &Scenario, w: &mut TraceWriter) -> Result<(), GoldenError> {
    drive_with(scenario, w, None)
}

/// The drive closure for a fixture directory (relative to the workspace
/// root, as [`crate::fixture_dir`] gives it): [`drive`], plus
/// `"world": "load"`, whose `dir` is relative to that directory.
pub fn drive_in(
    fixture: impl Into<std::path::PathBuf>,
) -> impl FnOnce(&Scenario, &mut TraceWriter) -> Result<(), GoldenError> {
    let fixture = fixture.into();
    move |scenario, w| drive_with(scenario, w, Some(&fixture))
}

fn drive_with(
    scenario: &Scenario,
    w: &mut TraceWriter,
    fixture: Option<&std::path::Path>,
) -> Result<(), GoldenError> {
    let ScenarioKind::Tick(sc) = &scenario.kind else {
        return Err(GoldenError::other(format!(
            "tick_driver: scenario {:?} is not kind \"tick\"",
            scenario.name
        )));
    };
    validate(sc)?;
    let (mut world, save_store) = match sc.world {
        WorldSource::Load => load_world(sc, fixture)?,
        _ => build_world(sc)?,
    };
    let mut h = Harness {
        save_store,
        seed: sc.seed,
        empty_generator: sc.world == WorldSource::Empty,
        player: None,
        join_player_counter: 0,
        difficulty: sc.difficulty,
        mob_spawning: sc.mob_spawning.unwrap_or_default(),
        player_pos: None,
        player_container: None,
        open_container: None,
        controller: None,
        last_dig: None,
        mouse_ticks_ran: 0,
        dig_this_tick: false,
    };

    let driven = run_ticks(sc, &mut world, &mut h, w);
    // The gaps the scenario reached, also when it failed part-way
    // (crate::gaps; `reload` carries the counters over to the new world).
    w.set_gaps(crate::Gaps::of(&world));
    driven
}

/// The implicit preload, then every tick with its ops and dumps (the body of
/// [`drive_with`] once the world exists).
fn run_ticks(
    sc: &TickScenario,
    world: &mut World,
    h: &mut Harness,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
    implicit_preload(world, &sc.ops, w);

    // Ops grouped by t, file order within a tick (TickScenario.java: a stable
    // sort by t, then consumed in order).
    let mut ops: Vec<&Op> = sc.ops.iter().collect();
    ops.sort_by_key(|op| op.t);
    let mut next = 0;
    for t in 0..sc.ticks {
        // Frame start (Minecraft.java:519-520): AxisAlignedBB.clearBoundingBoxPool()
        // and Vec3D.initialize() reset object pools, which the port does not have.
        h.dig_this_tick = false;
        while next < ops.len() && ops[next].t == t {
            apply(world, h, ops[next], w)?;
            next += 1;
        }
        release_button(h);
        run_tick(world, h);
        // Once per frame after the tick, Minecraft.java:553 (at most 500 updates).
        world.updating_lighting();
        for d in &sc.dumps {
            if d.due_at(t) {
                dump(world, h, d, t, w)?;
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
        let name = match op.kind {
            OpKind::Save { .. } => Some("save"),
            OpKind::Reload {} => Some("reload"),
            _ => None,
        };
        if let Some(name) = name
            && save_format != "mcregion"
            && sc.world != WorldSource::Load
        {
            return Err(GoldenError::other(format!(
                "op {name} needs \"save_format\": \"mcregion\" or \"world\": \"load\""
            )));
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
    // The oracle's SaveOldDir inherits SaveHandler.getMapFile, never null
    // (SaveHandler.java:23-24, :167-169): idcounts.dat and data/map_N.dat
    // are read and written in "mcregion" and "world": "load" scenarios.
    fn has_map_files(&self) -> bool {
        self.0.has_map_files()
    }
    fn read_map_file(&mut self, name: &str) -> Result<Option<Vec<u8>>, SaveError> {
        self.0.read_map_file(name)
    }
    fn write_map_file(&mut self, name: &str, bytes: &[u8]) -> Result<(), SaveError> {
        self.0.write_map_file(name, bytes)
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
        // generator for the world seed.
        WorldSource::Generated => provider.get_chunk_provider(sc.seed),
        WorldSource::Load => unreachable!("load_world builds a loaded world"),
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

/// `TickScenario.load` (`"world": "load"`): the save directory `dir`
/// (relative to the fixture directory, inside the fixture tree) copied into
/// an in-memory storage as `world/…` (the oracle's temp copy), opened with the
/// loading constructor `World(SaveOldDir, "harness", seed, null)`
/// (`HarnessWorld.open`), then `makeDeterministic`/`resetStatics` with the
/// scenario seed. The store is the world's save (autosaves, `save` ops).
fn load_world(
    sc: &TickScenario,
    fixture: Option<&std::path::Path>,
) -> Result<(World, Option<SharedSaveStore>), GoldenError> {
    let Some(fixture) = fixture else {
        return Err(GoldenError::other(
            "\"world\": \"load\" needs the fixture directory (tick_driver::drive_in)",
        ));
    };
    if sc.save_format.is_some() {
        return Err(GoldenError::other(
            "\"world\": \"load\" takes no \"save_format\" (the loaded save is the world's)",
        ));
    }
    let sub = sc
        .dir
        .as_deref()
        .ok_or_else(|| GoldenError::other("\"world\": \"load\" needs \"dir\""))?;
    let src = load_dir_path(fixture, sub)?;
    let mut storage = mc_sim::save::MemoryStorage::default();
    for (rel, bytes) in crate::io::read_tree(&src)? {
        use mc_sim::save::Storage;
        storage
            .write(&mc_sim::save::path_join(WORLD_DIR, &rel), &bytes)
            .map_err(|e| GoldenError::other(e.to_string()))?;
    }
    let store = harness_store(storage);
    let dir = SaveOldDir::new(store.clone(), "", WORLD_DIR)
        .map_err(|e| GoldenError::other(e.to_string()))?;
    let mut world = World::new_from_save(
        Box::new(HarnessSaveHandler(dir)),
        "harness",
        sc.seed,
        None,
        // Overwritten by make_deterministic; vanilla draws these unseeded.
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    if world.is_new_world {
        return Err(GoldenError::other(format!("no level.dat in \"dir\" {sub}")));
    }
    world.make_deterministic(sc.seed);
    Ok((world, Some(store)))
}

/// `fixture/sub`, lexically normalised (`..` climbs), which must stay inside
/// the fixture tree (the fixture directory's grandparent, `tests/golden`)
/// and hold a `level.dat`.
fn load_dir_path(fixture: &std::path::Path, sub: &str) -> Result<std::path::PathBuf, GoldenError> {
    use std::path::Component;
    // Relative fixture paths are taken from the workspace root (crate::run).
    let fixture = if fixture.is_absolute() {
        fixture.to_path_buf()
    } else {
        crate::workspace_root().join(fixture)
    };
    let golden = fixture
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| GoldenError::other("the fixture directory has no fixture tree above it"))?
        .to_path_buf();
    let mut out = std::path::PathBuf::new();
    for c in fixture.join(sub).components() {
        match c {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    if !out.starts_with(&golden) {
        return Err(GoldenError::other(
            "\"dir\" must stay inside the fixture tree",
        ));
    }
    if !out.join("level.dat").is_file() {
        return Err(GoldenError::other(format!("no level.dat in \"dir\" {sub}")));
    }
    Ok(out)
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
    /// `HarnessWorld.seed`: the scenario seed (the `reload` op's loading
    /// constructor takes it).
    seed: i64,
    /// `HarnessWorld.emptyGenerator`: `"world": "empty"`, whose generator a
    /// `reload` swaps in again.
    empty_generator: bool,
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
    /// The player's `inventorySlots` (`new ContainerPlayer(inventory,
    /// !multiplayerWorld)`, EntityPlayer.java:42), built by the `player` op.
    /// Held here because the port's `PlayerData` does not hold
    /// `inventorySlots` (the [`mc_sim::entity::PlayerData`] docs).
    player_container: Option<Container>,
    /// `TickScenario.openContainer` / `openSpec`: the container the last
    /// `slot_click` with a spec other than `"player"` opened.
    open_container: Option<(String, Container)>,
    /// `HarnessWorld.controller`: the left button's `PlayerControllerSP`,
    /// created by the first `dig` op.
    controller: Option<PlayerControllerSp>,
    /// `HarnessWorld.lastDig`: the block the button was held on last tick.
    last_dig: Option<[i32; 3]>,
    /// `HarnessWorld.mouseTicksRan` (Minecraft.mouseTicksRan): the tick of
    /// the last left click.
    mouse_ticks_ran: u32,
    /// `HarnessWorld.digThisTick`.
    dig_this_tick: bool,
}

/// `TickScenario.releaseButton`: a tick without a `dig` op has the left
/// button up: `updateController()` (Minecraft.java:962-964) and
/// `func_6254_a(0, false)` → `resetBlockRemoving()` (Minecraft.java:779-797).
fn release_button(h: &mut Harness) {
    if let Some(c) = h.controller.as_mut()
        && !h.dig_this_tick
    {
        c.update_controller();
        c.reset_block_removing();
        h.last_dig = None;
    }
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
            ref inventory,
        } => {
            if h.player.is_some() {
                return Err(GoldenError::other(format!("t={t}: only one player op")));
            }
            // `new HarnessPlayer(world)`: the pure EntityPlayer constructor
            // chain (mc_sim::entity::player); its `rand` is seeded with
            // mix(seed, id) at construction (`seedAfterConstruction`).
            let mut e = new_entity_player_sp(world);
            // HarnessPlayer's username, as an EntityPlayerSP's session gives
            // one (EntityPlayerSP.java:20).
            player_data(&mut e).username = Some("Player".to_string());
            for s in inventory.iter().flatten() {
                if !(0..40).contains(&s.slot) {
                    return Err(GoldenError::other(format!(
                        "t={t}: inventory slot {} outside 0..39",
                        s.slot
                    )));
                }
                // src: InventoryPlayer.java:180-188 (setInventorySlotContents)
                player_data(&mut e).inventory.set_inventory_slot_contents(
                    s.slot,
                    Some(ItemStack::new(s.id, s.count, s.damage)),
                );
            }
            // `place`: setLocationAndAngles (Entity.java:669-676), yaw/pitch
            // defaulting to the constructor's.
            let yaw = yaw.unwrap_or(e.base.rotation_yaw);
            let pitch = pitch.unwrap_or(e.base.rotation_pitch);
            e.base.set_location_and_angles(x, y, z, yaw, pitch);
            let eid = e.id();
            // src: EntityPlayer.java:42 (in the constructor; no draws, no world access)
            h.player_container = Some(new_container_player(!world.multiplayer_world));
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
        OpKind::Input {
            forward,
            strafe,
            jump,
            sneak,
            hotbar,
        } => {
            let pid = require_player(h, t, "input")?;
            let p = loaded_entity(world, t, "input", pid)?;
            if let Some(slot) = hotbar {
                if !(0..9).contains(&slot) {
                    return Err(GoldenError::other(format!(
                        "t={t}: hotbar {slot} outside 0..8"
                    )));
                }
                // src: Minecraft.java:1073-1076 (the number keys)
                player_data(p).inventory.current_item = slot;
            }
            // TickScenario.input: MovementInputFromOptions.updatePlayerMoveState's
            // mapping (MovementInputFromOptions.java:50-76)
            let mi = MovementInput::from_keys(
                forward.unwrap_or(0.0),
                strafe.unwrap_or(0.0),
                jump,
                sneak,
            );
            match &mut p.kind {
                mc_sim::entity::EntityKindData::PlayerSp(_, _, d) => d.movement_input = mi,
                _ => unreachable!("the player op's entity is an EntityPlayerSP"),
            }
            line("input")
        }
        OpKind::Dig { x, y, z, side } => dig(world, h, t, x, y, z, side)?,
        OpKind::Sleep { x, y, z } => {
            let pid = require_player(h, t, "sleep")?;
            // TickScenario.sleep: Block.blockActivated as the player.
            let r = world.with_entity_mut(pid, |w, p| {
                if let mc_sim::entity::EntityKindData::PlayerSp(_, _, d) = &mut p.kind {
                    d.last_sleep_status = None;
                }
                let id = w.get_block_id(x, y, z);
                let result = id > 0 && block_dispatch::block_activated(w, id, x, y, z, p);
                let status = match &p.kind {
                    mc_sim::entity::EntityKindData::PlayerSp(_, _, d) => d.last_sleep_status,
                    _ => None,
                };
                (result, status)
            });
            let Some((result, status)) = r else {
                return Err(player_gone(t, "sleep"));
            };
            line("sleep").with("result", result).with(
                "status",
                status.map_or(Value::Null, |s| Value::from(s.name())),
            )
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
        OpKind::Craft { ref grid } => line("craft").with("result", stack_json(craft(t, grid)?)),
        OpKind::SlotClick {
            ref container,
            slot,
            button,
            shift,
        } => {
            let Some(pid) = h.player else {
                return Err(GoldenError::other(format!(
                    "t={t}: slot_click needs a player op first"
                )));
            };
            let c = open_container(world, h, t, container)?;
            if slot != -999 && (slot < 0 || slot as usize >= c.slots.len()) {
                return Err(GoldenError::other(format!(
                    "t={t}: slot_click: slot {slot} outside 0..{} and not -999",
                    c.slots.len() as i32 - 1
                )));
            }
            let r = world.with_entity_mut(pid, |world, p| {
                // src: PlayerController.java:96-98, Container.java:47-150
                let r = c.slot_click(world, p, slot, button, shift);
                let st = if slot == -999 {
                    None
                } else {
                    c.slot_get_stack(world, p, slot as usize)
                };
                let held = p
                    .kind
                    .player()
                    .expect("the player op's entity is an EntityPlayer")
                    .inventory
                    .get_item_stack();
                (r, st, held)
            });
            let Some((r, st, held)) = r else {
                return Err(unsupported(
                    t,
                    "slot_click",
                    "the player is no longer in loadedEntityList (the oracle keeps using its \
                     HarnessPlayer object)",
                ));
            };
            line("slot_click")
                .with("slot", slot)
                .with("result", stack_json(r))
                .with("stack", stack_json(st))
                .with("held", stack_json(held))
        }
        OpKind::Reload {} => reload(world, h, t)?,
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

/// `HarnessWorld.reload` (README "Reload"): save, drop the world, open the
/// save again with the loading constructor, carry the Java statics
/// ([`World::adopt_statics`]) and the harness-defined clock-seeded state
/// ([`World::adopt_clock_seeded_state`]) over, reload the old world's chunks
/// in `chunkList` order (each followed by draining the light queue), then,
/// if there was a player, a new one through
/// [`World::spawn_player_with_loaded_chunks`]. The dig controller, the last
/// dug block and the open container are dropped; the player's
/// `ContainerPlayer` is a new one (the constructor's).
// src: HarnessWorld.java reload; Minecraft.java:1268-1337 (changeWorld)
fn reload(world: &mut World, h: &mut Harness, t: u32) -> Result<TraceLine, GoldenError> {
    let Some(store) = h.save_store.clone() else {
        return Err(GoldenError::other(
            "op reload needs \"save_format\": \"mcregion\" or \"world\": \"load\"",
        ));
    };
    let coords = world.loaded_chunk_coords();
    // src: World.java:278-291 (saveWorld(true, null), as
    // Minecraft.changeWorld's saveWorldIndirectly does, Minecraft.java:1276)
    world
        .try_save_world(true)
        .map_err(|e| GoldenError::other(format!("t={t}: reload: save: {e}")))?;
    let dir = SaveOldDir::new(store, "", WORLD_DIR)
        .map_err(|e| GoldenError::other(format!("t={t}: reload: {e}")))?;
    // src: World.java:149-208, HarnessWorld.reload. Harness choice: the old
    // world's dimension is kept by passing its provider (World.java:184-185,
    // WorldProvider.java:100-108) rather than taking it from level.dat's
    // Player compound (World.java:186-190), which the harness player (whose
    // Entity.dimension is never set) or a world without a player would get
    // wrong, reading world/region instead of world/DIM-1/region
    // (SaveOldDir.java:11-20).
    let mut fresh = World::new_from_save(
        Box::new(HarnessSaveHandler(dir)),
        "harness",
        h.seed,
        Some(WorldProvider::new(world.world_provider.dimension)),
        // Overwritten by adopt_clock_seeded_state below.
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    if fresh.is_new_world {
        return Err(GoldenError::other(format!(
            "t={t}: reload: no level.dat in the save"
        )));
    }
    if h.empty_generator {
        fresh.set_chunk_generator(ChunkGenerator::Empty(EmptyChunkProvider));
    }
    fresh.adopt_clock_seeded_state(world);
    fresh.adopt_statics(world);
    // The old World object is dropped here.
    *world = fresh;
    h.controller = None;
    h.last_dig = None;
    h.open_container = None;
    for (x, z) in coords {
        world.get_chunk_from_chunk_coords(x, z);
        while world.updating_lighting() {}
    }
    let mut player = Value::Null;
    if h.player.is_some() {
        // As the `player` op builds it: the EntityPlayer constructor chain,
        // seeded at construction, username "Player", a new ContainerPlayer.
        let mut e = new_entity_player_sp(world);
        player_data(&mut e).username = Some("Player".to_string());
        let eid = e.id();
        h.player_container = Some(new_container_player(!world.multiplayer_world));
        world.spawn_player_with_loaded_chunks(e); // src: World.java:256-276
        h.player = Some(eid);
        player = Value::from(eid);
    }
    Ok(TraceLine::op(t as i64, "reload")
        .with("chunks", world.loaded_chunk_count() as u64)
        .with("entities", world.loaded_entity_list.len() as u64)
        .with("tile_entities", world.loaded_tile_entity_list.len() as u64)
        .with("player", player))
}

/// `TickScenario.craft`: `CraftingManager.findMatchingRecipe` over a new
/// 3×3 (9 entries) or 2×2 (4 entries) `InventoryCrafting`, entry `i` put
/// into slot `i` (`setInventorySlotContents`; the grid's event handler is a
/// container without slots, so nothing else runs).
// src: CraftingManager.java:160-169, InventoryCrafting.java:8-13, :58-61
fn craft(t: u32, grid: &[Option<[i32; 3]>]) -> Result<Option<ItemStack>, GoldenError> {
    let w = match grid.len() {
        9 => 3,
        4 => 2,
        _ => {
            return Err(GoldenError::other(format!(
                "t={t}: craft: \"grid\" needs 9 or 4 entries"
            )));
        }
    };
    let mut inv = InventoryCrafting::new(w, w);
    for (k, e) in grid.iter().enumerate() {
        if let Some([id, count, damage]) = *e {
            // src: ItemStack.java:33-38
            inv.set_inventory_slot_contents(k as i32, Some(ItemStack::new(id, count, damage)));
        }
    }
    Ok(CraftingManager::get_instance().find_matching_recipe(&inv))
}

/// `TickScenario.container`: the container a `slot_click` names (README
/// "Ops"): the player's own `ContainerPlayer`; the last opened container
/// when the spec repeats; otherwise a new `ContainerWorkbench`
/// (`"workbench"` at 0,0,0, or `"workbench at x,y,z"`), `ContainerChest` on
/// the inventory `BlockChest.blockActivated` builds
/// ([`mc_sim::inventory::large_chest::chest_inventory`]), `ContainerFurnace`
/// or `ContainerDispenser` on the tile entity at `x,y,z`, or
/// (`"minecart <id>"`, WP-4.6) `ContainerChest` over the chest minecart with
/// that id ([`InventoryRef::Entity`]), replacing the previous one without
/// `onCraftGuiClosed`.
fn open_container<'h>(
    world: &mut World,
    h: &'h mut Harness,
    t: u32,
    spec: &str,
) -> Result<&'h mut Container, GoldenError> {
    if spec == "player" {
        return Ok(h
            .player_container
            .as_mut()
            .expect("the player op built the player's container"));
    }
    if h.open_container.as_ref().is_some_and(|(s, _)| s == spec) {
        return Ok(&mut h.open_container.as_mut().expect("checked").1);
    }
    let bad = |what: &str| GoldenError::other(format!("t={t}: slot_click: {what}"));
    let cart = spec
        .strip_prefix("minecart ")
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()));
    let c = if spec == "workbench" {
        new_container_workbench(0, 0, 0)
    } else if let Some(digits) = cart {
        // TickScenario.java's CART regex: `minecart ([0-9]+)`, read with
        // `Long.parseLong`: an id past i32 names no entity.
        let id: i64 = digits
            .parse()
            .map_err(|_| bad(&format!("unknown container {spec:?}")))?;
        let no_cart = || {
            bad(&format!(
                "no chest minecart with id {id} in loadedEntityList"
            ))
        };
        let id = i32::try_from(id).map_err(|_| no_cart())?;
        let is_chest_cart = world.entity(id).is_some_and(|e| {
            matches!(&e.kind, mc_sim::entity::EntityKindData::Minecart(d) if d.minecart_type == 1)
        });
        if !is_chest_cart {
            return Err(no_cart());
        }
        new_container_chest(world, InventoryRef::Entity(id))
    } else {
        let (kind, pos) = spec
            .split_once(" at ")
            .ok_or_else(|| bad(&format!("unknown container {spec:?}")))?;
        // TickScenario.java's AT regex takes each coordinate as
        // `-?[0-9]+` only: refuse a leading '+' (or anything else
        // `str::parse` would accept) so both harnesses reject the same specs.
        let xyz: Vec<i32> = pos
            .split(',')
            .map(|v| {
                let digits = v.strip_prefix('-').unwrap_or(v);
                if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
                    v.parse::<i32>().ok()
                } else {
                    None
                }
            })
            .collect::<Option<_>>()
            .ok_or_else(|| bad(&format!("unknown container {spec:?}")))?;
        let [x, y, z] = xyz[..] else {
            return Err(bad(&format!("unknown container {spec:?}")));
        };
        let te_kind = |world: &mut World| {
            world.get_block_tile_entity(x, y, z).and_then(|r| {
                world
                    .tile_entities
                    .get(&r)
                    .map(|te| (r, te.kind.java_class()))
            })
        };
        match kind {
            "workbench" => new_container_workbench(x, y, z),
            "chest" => {
                if world.get_block_id(x, y, z) != mc_sim::block::ids::CHEST {
                    return Err(bad(&format!("no chest at {x},{y},{z}")));
                }
                let inv = chest_inventory(world, x, y, z)
                    .ok_or_else(|| bad(&format!("no chest tile entity at {x},{y},{z}")))?;
                new_container_chest(world, inv)
            }
            "furnace" => match te_kind(world) {
                Some((r, "TileEntityFurnace")) => new_container_furnace(r),
                _ => return Err(bad(&format!("no furnace at {x},{y},{z}"))),
            },
            "dispenser" => match te_kind(world) {
                Some((r, "TileEntityDispenser")) => new_container_dispenser(r),
                _ => return Err(bad(&format!("no dispenser at {x},{y},{z}"))),
            },
            _ => return Err(bad(&format!("unknown container {spec:?}"))),
        }
    };
    h.open_container = Some((spec.to_string(), c));
    Ok(&mut h.open_container.as_mut().expect("just set").1)
}

/// The `player` op's player, or the oracle's "needs a player op first".
fn require_player(h: &Harness, t: u32, op: &str) -> Result<i32, GoldenError> {
    h.player
        .ok_or_else(|| GoldenError::other(format!("t={t}: {op} needs a player op first")))
}

fn player_gone(t: u32, op: &str) -> GoldenError {
    unsupported(
        t,
        op,
        "the player is no longer in loadedEntityList (the oracle keeps using its \
         HarnessPlayer object)",
    )
}

/// `TickScenario.dig`: the left button held on block `(x, y, z)`, face
/// `side`, this tick: a press (no `dig` last tick, or another block) or a
/// held repeat 5 ticks after the last click is `clickMouse(0)`
/// (`swingItem`, `clickBlock`); then `sendBlockRemoving`
/// (Minecraft.java:779-830, :1003-1022, :1110-1113; README "Ops").
fn dig(
    world: &mut World,
    h: &mut Harness,
    t: u32,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> Result<TraceLine, GoldenError> {
    let pid = require_player(h, t, "dig")?;
    if h.dig_this_tick {
        return Err(GoldenError::other(format!("t={t}: one dig op per tick")));
    }
    h.dig_this_tick = true;
    let mut c = h.controller.take().unwrap_or_default();
    c.update_controller(); // src: Minecraft.java:962-964
    let press = h.last_dig != Some([x, y, z]);
    // `(float)(ticksRan - mouseTicksRan) >= timer.ticksPerSecond / 4.0F`
    let repeat = (t as i64 - h.mouse_ticks_ran as i64) as f32 >= 20.0 / 4.0;
    let r = world.with_entity_mut(pid, |w, p| {
        let mut clicked = false;
        let mut broken = false;
        if press || repeat {
            swing_item(p); // src: Minecraft.java:803-805
            broken = c.click_block(w, p, x, y, z, side);
            clicked = true;
        }
        broken |= c.send_block_removing(w, p, x, y, z, side);
        (clicked, broken)
    });
    let progress = c.cur_block_damage;
    h.controller = Some(c);
    let Some((clicked, broken)) = r else {
        return Err(player_gone(t, "dig"));
    };
    if clicked {
        h.mouse_ticks_ran = t;
    }
    h.last_dig = Some([x, y, z]);
    Ok(TraceLine::op(t as i64, "dig")
        .with("x", x)
        .with("y", y)
        .with("z", z)
        .with("clicked", clicked)
        .with_f32("progress", progress)
        .with("broken", broken))
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
    if attacker.is_some()
        && let Some(pid) = h.player
        && attacker_id == Some(pid)
    {
        if target_id == pid {
            return Err(unsupported(
                t,
                "attack",
                "the target is the player itself (both would have to be in hand)",
            ));
        }
        // Minecraft.clickMouse(0), ENTITY branch (Minecraft.java:802-814):
        // swingItem, then PlayerController.attackEntity
        // (PlayerController.java:92-94).
        world
            .with_entity_mut(pid, |w, p| {
                swing_item(p);
                w.with_entity_mut(target_id, |w, target| {
                    player_controller::attack_entity(w, p, target)
                })
            })
            .flatten()
            .ok_or_else(|| player_gone(t, "attack"))?;
        return Ok(line.with("via", "player"));
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
/// - Any other `EntityList` name: its `(World)` constructor from
///   `mc_sim::save::entity_list::CONSTRUCTORS` (an error naming the class
///   for one the port does not have yet). The animals (`Pig`, `Sheep`,
///   `Cow`, `Chicken`, `Wolf`, `Squid`) and the hostile mobs (`Monster`,
///   `Creeper`, `Skeleton`, `Spider`, `Giant`, `Zombie`, `Slime`, `Ghast`,
///   `PigZombie`) are WP-2.2's and WP-2.3's. The oracle replays the slime
///   constructor's size and jump-delay draws on the seeded `rand`
///   (`TickScenario.seedAfterConstruction`); the port's slime constructor
///   draws them from its construction-seeded `rand`, which is that replay.
///   `Mob` is the abstract `EntityLiving`.
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
    use mc_sim::entity::{egg, fireball, fish, lightning_bolt, minecart, painting};
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
            // The (World) constructors the port has, one table for every
            // caller (mc_sim::save::entity_list::CONSTRUCTORS).
            match mc_sim::save::entity_list::constructor(class) {
                Some(new) => new(world),
                None => {
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
/// (`Option<usize>`, `None` for Java's `null`):
/// [`ItemStack::use_item`] runs on the stack moved out of that slot
/// ([`send_place_block`]) and the mutation lands in the slot, and the
/// size-0 test reads the slot, which is Java's `held` because nothing in
/// sendPlaceBlock stores another stack into the slot (no b1.7.3
/// `blockActivated` or `onItemUse` assigns `mainInventory`). Java's identity
/// test in `sendUseItem` (`var5 != var3`) is the explicit
/// [`mc_sim::item::RightClickResult`] instead. `Item.onItemUse` and
/// `Item.onItemRightClick` are the item behaviour dispatch
/// ([`mc_sim::item::behaviour`], WP-2.6); `blockActivated` goes through the
/// block dispatch.
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
///    `EntityPlayer.useCurrentItemOnEntity` (EntityPlayer.java:457-469,
///    [`use_current_item_on_entity`], the target taken out of its slot
///    too): `result = target.interact(player)`; when false, the held
///    stack's `useItemOnEntity` on a living target (`Item.saddleEntity`:
///    the saddle on a pig, a dye on a sheep), and a stack at size 0 or less
///    leaves the hotbar.
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
        .with_entity_mut(target, |w, e| use_current_item_on_entity(p, w, e))
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

/// `Dumps.inventory`: the player's `InventoryPlayer` (`currentItem`, the 36
/// main and 4 armour slots, the cursor stack) and its `ContainerPlayer`'s
/// grid and result.
fn inventory_line(world: &World, h: &Harness, t: i64) -> Result<TraceLine, GoldenError> {
    let Some(pid) = h.player else {
        return Err(GoldenError::other("dump inventory needs a player op first"));
    };
    let Some(p) = world.entity(pid) else {
        return Err(GoldenError::other(format!(
            "t={t}: dump inventory: the player is no longer in loadedEntityList (the oracle keeps \
             using its HarnessPlayer object)"
        )));
    };
    let inv = &p
        .kind
        .player()
        .expect("the player op's entity is an EntityPlayer")
        .inventory;
    let list = |a: &[Option<ItemStack>]| Value::Array(a.iter().map(|s| stack_json(*s)).collect());
    let c = h
        .player_container
        .as_ref()
        .expect("the player op built the player's container");
    let ContainerKind::Player {
        craft_matrix,
        craft_result,
        ..
    } = &c.kind
    else {
        unreachable!("inventorySlots is a ContainerPlayer");
    };
    let grid: Vec<Option<ItemStack>> = (0..4).map(|i| craft_matrix.get_stack_in_slot(i)).collect();
    Ok(TraceLine::new(t, "inventory")
        .with("current_item", inv.current_item)
        .with("main", list(&inv.main_inventory))
        .with("armor", list(&inv.armor_inventory))
        .with("held", stack_json(inv.get_item_stack()))
        .with("craft_matrix", list(&grid))
        .with(
            "craft_result",
            stack_json(craft_result.get_stack_in_slot(0)),
        ))
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
fn dump(
    world: &mut World,
    h: &Harness,
    d: &Dump,
    t: u32,
    w: &mut TraceWriter,
) -> Result<(), GoldenError> {
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
        DumpKind::Inventory => w.push(inventory_line(world, h, ti)?),
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
        DumpKind::MapData { id } => w.push(map_data_line(world, ti, id)),
        DumpKind::PlayerState => {
            let pid = h
                .player
                .ok_or_else(|| GoldenError::other("player_state needs a player op first"))?;
            let e = world.entity(pid).ok_or_else(|| {
                GoldenError::other(format!(
                    "t={t}: player_state: the player is not in loadedEntityList (the oracle \
                     keeps its HarnessPlayer object)"
                ))
            })?;
            w.push(player_state_line(ti, e));
        }
    }
    Ok(())
}

/// `Dumps.mapData`: the `MapData` `"map_<id>"` as the world's `MapStorage`
/// holds it (`loadedDataMap.get`, no file read): `present`, and when present
/// BLAKE3 over its `colors` bytes, `scale`, `xCenter`, `zCenter`,
/// `dimension` and `step`.
fn map_data_line(world: &World, t: i64, id: i32) -> TraceLine {
    let name = mc_sim::item::behaviour::map::map_name(id);
    let l = TraceLine::new(t, "map_data").with("id", id);
    match world.field_28108_z.loaded(&name) {
        None => l.with("present", false),
        Some(d) => {
            let d = d.borrow();
            l.with("present", true)
                .with("colors", crate::blake3_hex(&d.colors))
                .with("scale", d.scale)
                .with("xCenter", d.x_center)
                .with("zCenter", d.z_center)
                .with("dimension", d.dimension)
                .with("step", d.step)
        }
    }
}

/// `Dumps.playerState`: the `EntityPlayer` state an `entity` line does not
/// carry.
fn player_state_line(t: i64, e: &mc_sim::entity::Entity) -> TraceLine {
    let mc_sim::entity::EntityKindData::PlayerSp(l, p, d) = &e.kind else {
        unreachable!("the player op's entity is an EntityPlayerSP")
    };
    let coords = |c: Option<[i32; 3]>| match c {
        None => Value::Null,
        Some(c) => serde_json::json!(c),
    };
    TraceLine::new(t, "player_state")
        .with("id", e.id())
        .with("sleeping", p.is_player_sleeping())
        .with("sleepTimer", p.get_sleep_timer())
        .with(
            "bed",
            coords(p.bed_chunk_coordinates.map(|c| [c.pos_x, c.pos_y, c.pos_z])),
        )
        .with(
            "spawn",
            coords(
                p.get_player_spawn_coordinate()
                    .map(|c| [c.pos_x, c.pos_y, c.pos_z]),
            ),
        )
        .with("isSwinging", p.is_swinging)
        .with("swingProgressInt", p.swing_progress_int)
        .with_f32("swingProgress", l.swing_progress)
        .with_f32("width", e.base.width)
        .with_f32("height", e.base.height)
        .with_f32("yOffset", e.base.y_offset)
        .with_f32("ySize", e.base.y_size)
        .with_f32("cameraYaw", p.camera_yaw)
        .with("score", p.score)
        .with("damageRemainder", p.damage_remainder)
        .with("timeUntilPortal", p.time_until_portal)
        .with("inPortal", p.in_portal)
        .with("sneaking", d.movement_input.sneak && !p.sleeping)
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
