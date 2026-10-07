//! WP-3.1b: the local player's calls into `Minecraft` raised as
//! [`ClientRequest`]s (`world/client_requests.rs`), each at its call site
//! (BlockBed.java:63, :76; BlockChest.java:217; BlockFurnace.java:107;
//! BlockDispenser.java:79; BlockWorkbench.java:23; ItemSign.java:47;
//! EntityMinecart.java:774; EntityItem.java:123; EntityPlayerSP.java:35-87),
//! only for a player with `client_additions` (the harness player raises
//! none); the `usePortal` split of `EntityPlayerSP.onLivingUpdate`; and
//! `World(World, WorldProvider)` with the clock-seeded `Random`s.

use mc_core::java_random::JavaRandom;
use mc_sim::block::dispatch as bd;
use mc_sim::entity::item::new_entity_item;
use mc_sim::entity::minecart::new_entity_minecart;
use mc_sim::entity::player::{self, MovementInput, new_entity_player_sp};
use mc_sim::entity::{Entity, EntityKindData, PlayerSpData, dispatch, zombie};
use mc_sim::inventory::InventoryRef;
use mc_sim::item::ItemStack;
use mc_sim::save::NoSave;
use mc_sim::stats::achievement_list as al;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::client_requests::{ClientRequest, PlayerGui};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{ClockSeeds, World, WorldEnvironment};

const STONE: i32 = 1;
const DISPENSER: i32 = 23;
const BED: i32 = 26;
const CHEST: i32 = 54;
const WORKBENCH: i32 = 58;
const FURNACE: i32 = 61;
const SIGN_ITEM: i32 = 323;
const SIGN_POST: i32 = 63;

fn env(seed: i64) -> WorldEnvironment {
    WorldEnvironment {
        rand_seed: seed,
        field_9437_g: 0,
        lock_timestamp: 0,
    }
}

fn world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        5,
        ChunkGenerator::Empty(EmptyChunkProvider),
        env(0),
    );
    w.make_deterministic(5);
    for dz in -1..=1 {
        for dx in -1..=1 {
            w.get_chunk_from_chunk_coords(dx, dz);
        }
    }
    for x in -8..8 {
        for z in -8..8 {
            w.set_block(x, 60, z, STONE);
        }
    }
    w
}

fn sp_mut(e: &mut Entity) -> &mut PlayerSpData {
    match &mut e.kind {
        EntityKindData::PlayerSp(_, _, s) => s,
        _ => unreachable!(),
    }
}

/// The client's player at `x, y, z` (`client_additions` on).
fn client_player(w: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut p = new_entity_player_sp(w);
    p.base.set_location_and_angles(x, y, z, 0.0, 0.0);
    p.kind.player_mut().unwrap().username = Some("Player".into());
    sp_mut(&mut p).client_additions = true;
    p
}

/// The harness's player (`HarnessPlayer`, an `EntityPlayer`).
fn harness_player(w: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut p = client_player(w, x, y, z);
    sp_mut(&mut p).client_additions = false;
    p
}

fn requests(w: &mut World) -> Vec<ClientRequest> {
    w.client_requests.drain()
}

/// BlockBed.java:70-80: in daytime `sleepInBedAt` is `NOT_POSSIBLE_NOW` and
/// `addChatMessage("tile.bed.noSleep")`; the harness player says nothing.
#[test]
fn a_bed_in_daytime_says_no_sleep() {
    let mut w = world();
    w.set_block_and_metadata(0, 61, 0, BED, 0);
    w.set_block_and_metadata(0, 61, 1, BED, 8);
    assert!(w.is_daytime());
    let mut p = client_player(&mut w, 0.5, 61.0, 2.5);
    assert!(bd::block_activated(&mut w, BED, 0, 61, 1, &mut p));
    assert_eq!(
        requests(&mut w),
        vec![ClientRequest::ChatMessage {
            key: "tile.bed.noSleep"
        }]
    );
    let mut h = harness_player(&mut w, 0.5, 61.0, 2.5);
    assert!(bd::block_activated(&mut w, BED, 0, 61, 1, &mut h));
    assert!(w.client_requests.is_empty());
}

/// BlockBed.java:49-65: an occupied bed with a player asleep in it says
/// `tile.bed.occupied` and stops there.
#[test]
fn an_occupied_bed_says_occupied() {
    let mut w = world();
    w.set_block_and_metadata(0, 61, 0, BED, 0);
    w.set_block_and_metadata(0, 61, 1, BED, 8);
    w.set_world_time(14000);
    w.skylight_subtracted = w.calculate_skylight_subtracted(1.0);
    assert!(!w.is_daytime());
    let sleeper = client_player(&mut w, 0.5, 61.0, 2.5);
    let sid = sleeper.id();
    w.spawn_entity_in_world(sleeper);
    w.with_entity_mut(sid, |w, s| {
        assert!(bd::block_activated(w, BED, 0, 61, 1, s));
    });
    assert!(requests(&mut w).is_empty(), "the first one sleeps");
    let mut p = client_player(&mut w, 1.5, 61.0, 2.5);
    assert!(bd::block_activated(&mut w, BED, 0, 61, 1, &mut p));
    assert_eq!(
        requests(&mut w),
        vec![ClientRequest::ChatMessage {
            key: "tile.bed.occupied"
        }]
    );
}

/// BlockChest.java:185-221: a single chest opens on its own tile entity; a
/// double chest wraps the -x half as the upper inventory; a normal cube on
/// top opens nothing.
#[test]
fn right_clicking_a_chest_opens_it() {
    let mut w = world();
    w.set_block(0, 61, 0, CHEST);
    let mut p = client_player(&mut w, 0.5, 61.0, 2.5);
    assert!(bd::block_activated(&mut w, CHEST, 0, 61, 0, &mut p));
    let te = w.get_block_tile_entity(0, 61, 0).unwrap();
    assert_eq!(
        requests(&mut w),
        vec![ClientRequest::DisplayGui(PlayerGui::Chest {
            x: 0,
            y: 61,
            z: 0,
            inventory: Some(InventoryRef::TileEntity(te)),
        })]
    );

    w.set_block(-1, 61, 0, CHEST);
    let west = w.get_block_tile_entity(-1, 61, 0).unwrap();
    assert!(bd::block_activated(&mut w, CHEST, 0, 61, 0, &mut p));
    let r = requests(&mut w);
    let [ClientRequest::DisplayGui(PlayerGui::Chest { inventory, .. })] = r.as_slice() else {
        panic!("{r:?}");
    };
    let Some(InventoryRef::LargeChest(l)) = inventory else {
        panic!("{inventory:?}");
    };
    assert_eq!(l.get_inv_name(), "Large chest");
    assert_eq!(l.upper_chest, InventoryRef::TileEntity(west));
    assert_eq!(l.lower_chest, InventoryRef::TileEntity(te));

    w.set_block(0, 62, 0, STONE);
    assert!(bd::block_activated(&mut w, CHEST, 0, 61, 0, &mut p));
    assert!(w.client_requests.is_empty());

    let mut h = harness_player(&mut w, 0.5, 61.0, 2.5);
    w.set_block(0, 62, 0, 0);
    assert!(bd::block_activated(&mut w, CHEST, 0, 61, 0, &mut h));
    assert!(w.client_requests.is_empty());
}

/// BlockFurnace.java:102-110, BlockDispenser.java:74-82,
/// BlockWorkbench.java:19-26: each opens its screen at the clicked block.
#[test]
fn furnace_dispenser_and_workbench_open_their_screens() {
    let mut w = world();
    w.set_block(0, 61, 0, FURNACE);
    w.set_block(1, 61, 0, DISPENSER);
    w.set_block(2, 61, 0, WORKBENCH);
    let mut p = client_player(&mut w, 0.5, 61.0, 2.5);
    assert!(bd::block_activated(&mut w, FURNACE, 0, 61, 0, &mut p));
    assert!(bd::block_activated(&mut w, DISPENSER, 1, 61, 0, &mut p));
    assert!(bd::block_activated(&mut w, WORKBENCH, 2, 61, 0, &mut p));
    let f = w.get_block_tile_entity(0, 61, 0);
    let d = w.get_block_tile_entity(1, 61, 0);
    assert!(f.is_some() && d.is_some());
    assert_eq!(
        requests(&mut w),
        vec![
            ClientRequest::DisplayGui(PlayerGui::Furnace {
                x: 0,
                y: 61,
                z: 0,
                tile_entity: f,
            }),
            ClientRequest::DisplayGui(PlayerGui::Dispenser {
                x: 1,
                y: 61,
                z: 0,
                tile_entity: d,
            }),
            ClientRequest::DisplayGui(PlayerGui::Workbench { x: 2, y: 61, z: 0 }),
        ]
    );
}

/// ItemSign.java:9-53: a sign placed on top of a block opens the sign
/// editor on the new sign's tile entity.
#[test]
fn placing_a_sign_opens_the_editor() {
    let mut w = world();
    let mut p = client_player(&mut w, 0.5, 61.0, 2.5);
    let mut stack = ItemStack::new(SIGN_ITEM, 1, 0);
    assert!(stack.use_item(&mut p, &mut w, 0, 60, 0, 1));
    assert_eq!(w.get_block_id(0, 61, 0), SIGN_POST);
    let te = w.get_block_tile_entity(0, 61, 0).unwrap();
    assert_eq!(
        requests(&mut w),
        vec![ClientRequest::DisplayGui(PlayerGui::EditSign {
            x: 0,
            y: 61,
            z: 0,
            tile_entity: te,
        })]
    );
}

/// EntityMinecart.java:763-791: a chest cart's `interact` opens its
/// inventory, named by the cart's id.
#[test]
fn a_chest_cart_opens_its_inventory() {
    let mut w = world();
    let cart = new_entity_minecart(&mut w, 0.5, 61.0, 0.5, 1);
    let cid = cart.id();
    w.spawn_entity_in_world(cart);
    let mut p = client_player(&mut w, 0.5, 61.0, 2.5);
    w.with_entity_mut(cid, |w, c| assert!(dispatch::interact(c, w, &mut p)));
    assert_eq!(
        requests(&mut w),
        vec![ClientRequest::DisplayGui(PlayerGui::MinecartChest {
            entity_id: cid
        })]
    );
}

/// EntityItem.java:110-130 with EntityPlayerSP.java:133-135: a pickup adds
/// an `EntityPickupFX` of the item (its stack as the pickup left it) to the
/// player, offset -0.5.
#[test]
fn a_pickup_flies_the_item_to_the_player() {
    let mut w = world();
    let mut item = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(STONE, 3, 0));
    item.kind.item_mut().unwrap().delay_before_can_pickup = 0;
    let mut p = client_player(&mut w, 0.5, 61.0, 0.5);
    dispatch::on_collide_with_player(&mut item, &mut w, &mut p);
    assert!(item.base.is_dead);
    let r = requests(&mut w);
    let [ClientRequest::PickupFx(fx)] = r.as_slice() else {
        panic!("{r:?}");
    };
    assert_eq!(fx.player_id, p.id());
    assert_eq!(fx.y_offs, -0.5);
    assert_eq!(fx.picked.id(), item.id());
    assert_eq!(fx.picked.kind.item().unwrap().item.unwrap().stack_size, 0);

    let mut item = new_entity_item(&mut w, 0.5, 61.0, 0.5, ItemStack::new(STONE, 3, 0));
    item.kind.item_mut().unwrap().delay_before_can_pickup = 0;
    let mut h = harness_player(&mut w, 0.5, 61.0, 0.5);
    dispatch::on_collide_with_player(&mut item, &mut w, &mut h);
    assert!(item.base.is_dead);
    assert!(w.client_requests.is_empty());
}

/// EntityPlayerSP.java:35-37: the inventory hint every update until
/// `openInventory` is unlocked.
#[test]
fn the_inventory_hint_until_the_achievement() {
    let mut w = world();
    let mut p = client_player(&mut w, 0.5, 61.0, 0.5);
    dispatch::on_living_update(&mut p, &mut w);
    assert_eq!(
        requests(&mut w),
        vec![ClientRequest::AchievementHint {
            stat_id: al::OPEN_INVENTORY
        }]
    );
    player::trigger_achievement(&mut p, mc_sim::stats::stat(al::OPEN_INVENTORY));
    dispatch::on_living_update(&mut p, &mut w);
    assert!(requests(&mut w).is_empty());
}

/// EntityPlayerSP.java:39-87: the first update in a portal plays
/// `portal.trigger`; the 81st plays `portal.travel` and raises `usePortal`,
/// with the pitches drawn from the player's `rand` at the Java positions,
/// and the update stops there (`inPortal` still set, the cooldown not yet
/// counted, no movement). `sp_on_living_update_after_portal` finishes it:
/// `inPortal` cleared, the cooldown 10 → 9, the sneak `ySize`, then
/// `EntityPlayer.onLivingUpdate` (the player falls).
#[test]
fn the_portal_timer_raises_use_portal_and_the_rest_runs_after() {
    let mut w = world();
    let mut p = client_player(&mut w, 4.5, 63.0, 4.5);
    player::trigger_achievement(&mut p, mc_sim::stats::stat(al::OPEN_INVENTORY));
    p.kind.player_mut().unwrap().time_until_portal = 0;
    let mut expect = p.base.rand.clone();
    let trigger = expect.next_float() * 0.4 + 0.8;
    let travel = expect.next_float() * 0.4 + 0.8;
    let mut all = Vec::new();
    for _ in 0..80 {
        dispatch::set_in_portal(&mut p, &mut w);
        dispatch::on_living_update(&mut p, &mut w);
        all.extend(requests(&mut w));
    }
    assert_eq!(
        all,
        vec![ClientRequest::PlaySoundFx {
            name: "portal.trigger",
            volume: 1.0,
            pitch: trigger,
        }]
    );
    let y_before = p.base.pos_y;
    sp_mut(&mut p).movement_input = MovementInput::from_keys(0.0, 0.0, false, true);
    dispatch::set_in_portal(&mut p, &mut w);
    dispatch::on_living_update(&mut p, &mut w);
    assert_eq!(
        requests(&mut w),
        vec![
            ClientRequest::PlaySoundFx {
                name: "portal.travel",
                volume: 1.0,
                pitch: travel,
            },
            ClientRequest::UsePortal {
                x: 4.5 / 8.0,
                z: 4.5 / 8.0
            },
        ]
    );
    // usePortal's head (Minecraft.java:1215-1230): into the nether, x and z
    // divided by 8, alive again after setEntityDead.
    assert_eq!((p.base.pos_x, p.base.pos_z), (4.5 / 8.0, 4.5 / 8.0));
    assert!(!p.base.is_dead);
    let d = p.kind.player().unwrap();
    assert_eq!(d.dimension, -1);
    assert_eq!(d.time_in_portal, 1.0);
    assert_eq!(
        d.time_until_portal, 10,
        "the cooldown counts after the portal"
    );
    assert!(d.in_portal, ":63 runs after usePortal returns");
    // No movement before the portal; setLocationAndAngles adds yOffset to
    // the y it is given (Entity.java:671).
    assert_eq!(
        p.base.pos_y,
        y_before + p.base.y_offset as f64,
        "no movement before the portal"
    );
    let y_head = p.base.pos_y;
    assert_eq!(p.base.y_size, 0.0);

    player::sp_on_living_update_after_portal(&mut p, &mut w);
    let d = p.kind.player().unwrap();
    assert!(!d.in_portal);
    assert_eq!(d.time_until_portal, 9);
    assert!(p.base.y_size > 0.0, "the sneak dip (decayed by moveEntity)");
    assert!(
        p.base.pos_y < y_head,
        "EntityPlayer.onLivingUpdate moved it"
    );
    assert!(w.client_requests.is_empty());

    // The harness's player: setInPortal only, nothing raised.
    let mut h = harness_player(&mut w, -4.5, 61.0, -4.5);
    h.kind.player_mut().unwrap().time_until_portal = 0;
    for _ in 0..81 {
        dispatch::set_in_portal(&mut h, &mut w);
        dispatch::on_living_update(&mut h, &mut w);
    }
    assert!(w.client_requests.is_empty());
}

/// Minecraft.java:1215-1230 runs inside the player's update
/// (EntityPlayerSP.java:59), within the old world's `updateEntities`: a
/// zombie updated after the player on the travel tick finds it out of
/// `playerEntities` and moved to x/8, z/8, and cannot hit it. Without the
/// portal, the same zombie hits it on that tick.
#[test]
fn a_mob_after_the_player_cannot_hit_it_on_the_travel_tick() {
    let run = |travel: bool| {
        let mut w = world();
        for dz in -3..=3 {
            for dx in -3..=3 {
                w.get_chunk_from_chunk_coords(dx, dz);
            }
        }
        w.difficulty_setting = 2;
        let mut p = client_player(&mut w, 4.5, 61.0, 4.5);
        p.kind.player_mut().unwrap().time_until_portal = 0;
        let pid = p.base.entity_id;
        w.spawn_entity_in_world(p);
        let mut z = zombie::new_entity_zombie(&mut w);
        z.base.set_location_and_angles(5.6, 61.0, 4.5, 0.0, 0.0);
        w.spawn_entity_in_world(z);
        // The first update targets the player (EntityCreature.java:19-24);
        // the second attacks it (EntityCreature.java:25-30, EntityMob.java:49-55).
        w.update_entities();
        requests(&mut w);
        assert_eq!(w.entity(pid).unwrap().kind.living().unwrap().health, 20);
        if travel {
            let d = w.entity_mut(pid).unwrap().kind.player_mut().unwrap();
            d.in_portal = true;
            d.time_in_portal = 0.999;
        }
        w.update_entities();
        let p = w.entity(pid).unwrap();
        let health = p.kind.living().unwrap().health;
        let (x, z) = (p.base.pos_x, p.base.pos_z);
        let in_list = w.player_entities.contains(&pid);
        let used = requests(&mut w)
            .into_iter()
            .any(|r| r == ClientRequest::UsePortal { x, z });
        (health, in_list, used)
    };
    assert_eq!(
        run(false),
        (15, true, false),
        "the zombie hits (5 at normal)"
    );
    assert_eq!(
        run(true),
        (20, false, true),
        "gone before the zombie's turn"
    );
}

/// World.java:103-143: the new world copies `worldInfo` and
/// `lockTimestamp`, takes the given handler, starts the weather from the
/// copied info, and nothing else of the old world; the JVM statics (the
/// entity and tick-entry counters, `Math.random()`, `Item.itemRand`) move
/// over.
#[test]
fn new_for_dimension_copies_the_info_and_keeps_the_statics() {
    let mut old = world();
    old.world_info.world_time = 1234;
    old.world_info.raining = true;
    old.world_info.thundering = true;
    old.world_info.spawn_x = 17;
    old.lock_timestamp = 777;
    let p = client_player(&mut old, 0.5, 61.0, 0.5);
    old.spawn_entity_in_world(p);
    old.next_entity_id = 42;
    old.next_tick_entry_id = 9;
    let mut math = old.math_random.clone();
    let mut item = old.item_rand.clone();
    let info = old.world_info.clone();

    let mut w = World::new_for_dimension(
        &mut old,
        WorldProvider::new(Dimension::Hell),
        Box::new(NoSave),
        env(99),
    );
    assert_eq!(w.world_info, info);
    assert_eq!(w.lock_timestamp, 777);
    assert_eq!(w.world_provider.world_type, -1);
    assert!(!w.is_new_world);
    assert_eq!(w.raining_strength, 1.0);
    assert_eq!(w.thundering_strength, 1.0);
    assert!(w.loaded_entity_list.is_empty());
    assert!(w.player_entities.is_empty());
    assert_eq!(w.loaded_chunk_count(), 0);
    assert_eq!(w.next_entity_id, 42);
    assert_eq!(w.next_tick_entry_id, 9);
    assert_eq!(w.math_random.next_long(), math.next_long());
    assert_eq!(w.item_rand.next_long(), item.next_long());
    // `rand = new Random()` from the environment, not the old world's.
    let mut r = JavaRandom::new(99);
    let _sound_counter = r.next_int_bounded(12000);
    assert_eq!(w.rand.next_long(), r.next_long());
}

/// `seed_clock_randoms` seeds the four clock-seeded streams and leaves the
/// counters alone.
#[test]
fn seed_clock_randoms_leaves_the_counters() {
    let mut w = world();
    w.next_entity_id = 5;
    w.next_tick_entry_id = 6;
    w.seed_clock_randoms(ClockSeeds {
        math_random: 1,
        item_rand: 2,
        entity_rand: 3,
        container_randoms: 4,
    });
    assert_eq!(w.next_entity_id, 5);
    assert_eq!(w.next_tick_entry_id, 6);
    assert_eq!(w.math_random.next_long(), JavaRandom::new(1).next_long());
    assert_eq!(w.item_rand.next_long(), JavaRandom::new(2).next_long());
    assert_eq!(w.entity_rand_seed, 3);
    let mut x = w.block_statics.clone();
    x.seed_container_randoms(4);
    assert_eq!(
        x.chest_random.next_long(),
        w.block_statics.chest_random.next_long()
    );
}

/// EntityPlayerSP.java:108-111: `closeScreen` closes the client's screen;
/// `EntityPlayer`'s (the harness's) does not reach the client.
#[test]
fn close_screen_asks_the_client() {
    let mut w = world();
    let p = client_player(&mut w, 0.5, 61.0, 0.5);
    player::close_screen(&p, &mut w);
    assert_eq!(requests(&mut w), vec![ClientRequest::CloseScreen]);
    let h = harness_player(&mut w, 0.5, 61.0, 0.5);
    player::close_screen(&h, &mut w);
    assert!(requests(&mut w).is_empty());
}
