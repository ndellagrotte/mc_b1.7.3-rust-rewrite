//! WP-2.5a: unit checks of `EntityPlayer`, the non-client `EntityPlayerSP`
//! and `PlayerControllerSP` that the player fixtures
//! (`tests/golden/entity/player-*`) do not reach: the armour arithmetic in
//! isolation, the sleep statuses in the nether and when dead, the NBT round
//! trip of a sleeping player (woken on read), the client-only
//! `onLivingUpdate` body (`pushOutOfBlocks`, the sneak `ySize`, the portal
//! timer), dropping the held item and the cursor stack, the controller's
//! instant break and hit wait, and the respawn check.

use mc_core::java_hash::ChunkCoordinates;
use mc_sim::entity::player::{self, EnumStatus, MovementInput, new_entity_player_sp};
use mc_sim::entity::{Entity, EntityKindData, PlayerData, PlayerSpData, dispatch, nbt};
use mc_sim::inventory::InventoryPlayer;
use mc_sim::item::ItemStack;
use mc_sim::player_controller::PlayerControllerSp;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::client_requests::ClientRequest;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const BED: i32 = 26;
const STONE: i32 = 1;

fn world(dim: Dimension) -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(dim),
        5,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
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

/// Night: the world time and the `skylightSubtracted` `World.tick` would
/// compute from it.
fn night(w: &mut World) {
    w.set_world_time(14000);
    w.skylight_subtracted = w.calculate_skylight_subtracted(1.0);
    assert!(!w.is_daytime());
}

fn player_at(w: &mut World, x: f64, y: f64, z: f64) -> Entity {
    let mut p = new_entity_player_sp(w);
    p.base.set_location_and_angles(x, y, z, 0.0, 0.0);
    p.kind.player_mut().unwrap().username = Some("Player".into());
    p
}

fn pd(e: &Entity) -> &PlayerData {
    e.kind.player().unwrap()
}

fn pd_mut(e: &mut Entity) -> &mut PlayerData {
    e.kind.player_mut().unwrap()
}

fn sp_mut(e: &mut Entity) -> &mut PlayerSpData {
    match &mut e.kind {
        EntityKindData::PlayerSp(_, _, s) => s,
        _ => unreachable!(),
    }
}

/// A bed with direction 0: the half without bit 3 at `z`, the one with it at
/// `z + 1` (BlockBed.headBlockToFootBlockMap[0] = {0, 1}).
fn bed(w: &mut World, x: i32, y: i32, z: i32) {
    w.set_block_and_metadata(x, y, z, BED, 0);
    w.set_block_and_metadata(x, y, z + 1, BED, 8);
}

/// `getTotalArmorValue` and `damageEntity`'s 25ths: full leather (20 points)
/// takes 5/25 of the damage; the remainder carries over.
#[test]
fn armour_absorbs_and_carries_the_remainder() {
    let mut w = world(Dimension::Surface);
    let mut p = player_at(&mut w, 0.5, 61.0, 0.5);
    let inv = &mut pd_mut(&mut p).inventory;
    for (i, id) in [301, 300, 299, 298].into_iter().enumerate() {
        inv.armor_inventory[i] = Some(ItemStack::new(id, 1, 0));
    }
    assert_eq!(pd(&p).inventory.get_total_armor_value(), 20);
    dispatch::damage_entity(&mut p, &mut w, 7);
    // 7 * 5 = 35: 1 point, 10/25 kept.
    assert_eq!(p.kind.living().unwrap().health, 19);
    assert_eq!(pd(&p).damage_remainder, 10);
    // Every piece wore by the full 7.
    assert!(
        pd(&p)
            .inventory
            .armor_inventory
            .iter()
            .all(|s| s.unwrap().item_damage == 7)
    );
    dispatch::damage_entity(&mut p, &mut w, 3);
    // Worn armour counts less: (20 - 1) * remaining / max + 1.
    let a = pd(&p).inventory.get_total_armor_value();
    assert!(a < 20);
    // A nearly broken piece breaks and leaves its slot.
    pd_mut(&mut p).inventory.armor_inventory[0] = Some(ItemStack::new(301, 1, 38));
    dispatch::damage_entity(&mut p, &mut w, 3);
    assert_eq!(pd(&p).inventory.armor_inventory[0], None);
}

/// No `ItemArmor` (a block in an armour slot) counts nothing.
#[test]
fn non_armour_in_armour_slots_counts_nothing() {
    let mut inv = InventoryPlayer::default();
    inv.armor_inventory[3] = Some(ItemStack::new(STONE, 1, 0));
    assert_eq!(inv.get_total_armor_value(), 0);
}

/// `sleepInBedAt`: the nether (`NOT_POSSIBLE_HERE`), a dead player
/// (`OTHER_PROBLEM`), and in the overworld at night the 0.2 box on the bed,
/// shifted toward its head; waking puts the player on a free spot beside it.
#[test]
fn sleep_statuses_position_and_wake_up() {
    let mut w = world(Dimension::Hell);
    night(&mut w);
    bed(&mut w, 0, 61, 2);
    let mut p = player_at(&mut w, 0.5, 61.0, 0.5);
    assert_eq!(
        player::sleep_in_bed_at(&mut p, &mut w, 0, 61, 3),
        EnumStatus::NotPossibleHere
    );

    let mut w = world(Dimension::Surface);
    night(&mut w);
    bed(&mut w, 0, 61, 2);
    let mut p = player_at(&mut w, 0.5, 61.0, 0.5);
    p.kind.living_mut().unwrap().health = 0;
    assert_eq!(
        player::sleep_in_bed_at(&mut p, &mut w, 0, 61, 3),
        EnumStatus::OtherProblem
    );
    p.kind.living_mut().unwrap().health = 20;
    assert_eq!(
        player::sleep_in_bed_at(&mut p, &mut w, 0, 61, 3),
        EnumStatus::Ok
    );
    assert_eq!(sp_mut(&mut p).last_sleep_status, Some(EnumStatus::Ok));
    // Direction 0: var7 = 0.9 (EntityPlayer.java:567-570).
    assert_eq!(p.base.pos_x, 0.5);
    assert_eq!(p.base.pos_y, (61.0f32 + 0.9375) as f64);
    assert_eq!(p.base.pos_z, (3.0f32 + 0.9) as f64);
    assert_eq!(
        (p.base.width, p.base.height, p.base.y_offset),
        (0.2, 0.2, 0.2)
    );
    assert!(dispatch::is_movement_blocked(&p, &w));
    assert!(!dispatch::is_entity_inside_opaque_block(&p, &mut w));
    assert_eq!(player::get_bed_orientation_in_degrees(&p, &mut w), 90.0);
    // Sneak input does not sneak a sleeper.
    sp_mut(&mut p).movement_input = MovementInput::from_keys(0.0, 0.0, false, true);
    assert!(!dispatch::is_sneaking(&p, &w));

    w.set_block_metadata(0, 61, 3, 8 | 4);
    player::wake_up_player(&mut p, &mut w, false, true, true);
    assert!(!pd(&p).sleeping);
    assert_eq!(pd(&p).sleep_timer, 100);
    assert_eq!(w.get_block_metadata(0, 61, 3), 8, "occupied bit cleared");
    assert_eq!(
        pd(&p).get_player_spawn_coordinate(),
        Some(ChunkCoordinates::new(0, 61, 3))
    );
    // getNearestEmptyChunkCoordinates' first free spot of the 3x3 around the
    // bed half: (-1, 61, 2) (x outer, z inner).
    assert_eq!(p.base.pos_x, (-1.0f32 + 0.5) as f64);
    assert_eq!(p.base.pos_z, (2.0f32 + 0.5) as f64);
    assert_eq!(p.base.pos_y, (61.0f32 + 1.62 + 0.1) as f64);
    assert_eq!(p.base.height, 1.8);
}

/// A player saved asleep is woken by `readEntityFromNBT` at its feet
/// (`bedChunkCoordinates` there, `wakeUpPlayer(true, true, false)`), keeps the
/// saved spawn and reads `Score`.
#[test]
fn nbt_round_trip_wakes_a_sleeper() {
    let mut w = world(Dimension::Surface);
    night(&mut w);
    bed(&mut w, 0, 61, 2);
    let mut p = player_at(&mut w, 0.5, 61.0, 0.5);
    assert_eq!(
        player::sleep_in_bed_at(&mut p, &mut w, 0, 61, 3),
        EnumStatus::Ok
    );
    pd_mut(&mut p).sleep_timer = 42;
    pd_mut(&mut p).score = 7;
    pd_mut(&mut p).set_player_spawn_coordinate(Some(ChunkCoordinates::new(4, 5, 6)));
    pd_mut(&mut p).inventory.main_inventory[3] = Some(ItemStack::new(4, 12, 0));
    let c = nbt::write_to_nbt(&p, &w);
    let mut q = new_entity_player_sp(&mut w);
    nbt::read_from_nbt(&mut q, &c, &mut w).unwrap();
    let d = pd(&q);
    assert!(!d.sleeping);
    assert_eq!(d.sleep_timer, 0);
    assert_eq!(
        d.bed_chunk_coordinates,
        Some(ChunkCoordinates::new(0, 61, 3))
    );
    assert_eq!(
        d.get_player_spawn_coordinate(),
        Some(ChunkCoordinates::new(4, 5, 6))
    );
    assert_eq!(d.score, 7);
    assert_eq!(
        d.inventory.main_inventory[3],
        Some(ItemStack::new(4, 12, 0))
    );
    assert_eq!(q.base.height, 1.8);
}

/// `EntityPlayerSP.onLivingUpdate` with the client parts on: a player whose
/// feet are in a stone block is pushed toward the open side, the sneak dips
/// `ySize`, and the portal timer reaches `Minecraft.usePortal` (raised as a
/// client request, the update finished by
/// `sp_on_living_update_after_portal`) after 81 ticks in a portal; off (the
/// harness), none of it.
#[test]
fn client_additions_push_sneak_and_portal() {
    let mut w = world(Dimension::Surface);
    w.set_block(0, 61, 0, STONE);
    w.set_block(0, 62, 0, STONE);
    let mut p = player_at(&mut w, 0.2, 61.0, 0.5);
    sp_mut(&mut p).client_additions = true;
    sp_mut(&mut p).movement_input = MovementInput::from_keys(0.0, 0.0, false, true);
    dispatch::on_living_update(&mut p, &mut w);
    // pushOutOfBlocks leaves motionX at -0.1 (the -x side is nearest, open),
    // then moveEntityWithHeading moves and damps it.
    assert!(p.base.pos_x < 0.2, "pushed toward -x: {}", p.base.pos_x);
    assert!(p.base.y_size > 0.0, "the sneak dip, decayed by moveEntity");

    let mut q = player_at(&mut w, 4.5, 61.0, 4.5);
    sp_mut(&mut q).client_additions = true;
    pd_mut(&mut q).time_until_portal = 0;
    // 80 steps of 0.0125F sum to 0.99999934 in float: the 81st reaches 1.
    for _ in 0..80 {
        dispatch::set_in_portal(&mut q, &mut w);
        dispatch::on_living_update(&mut q, &mut w);
    }
    assert!(pd(&q).time_in_portal < 1.0);
    let use_portal = |w: &World| {
        w.client_requests
            .iter()
            .any(|r| *r == ClientRequest::UsePortal)
    };
    assert!(!use_portal(&w));
    dispatch::set_in_portal(&mut q, &mut w);
    dispatch::on_living_update(&mut q, &mut w);
    assert_eq!(pd(&q).time_in_portal, 1.0);
    assert!(use_portal(&w));
    assert_eq!(
        pd(&q).time_until_portal,
        10,
        "the update stopped at usePortal"
    );
    player::sp_on_living_update_after_portal(&mut q, &mut w);
    assert_eq!(
        pd(&q).time_until_portal,
        9,
        "10, then the countdown of the same update"
    );
    // The harness player: setInPortal only resets the timer, nothing counts.
    let mut h = player_at(&mut w, -4.5, 61.0, -4.5);
    dispatch::set_in_portal(&mut h, &mut w);
    dispatch::on_living_update(&mut h, &mut w);
    assert_eq!(pd(&h).time_until_portal, 10);
    assert!(!pd(&h).in_portal);
}

/// `dropCurrentItem`: one item of the held stack, thrown forward with pick-up
/// delay 40; `setEntityDead` drops the cursor stack (`Container.onCraftGuiClosed`).
#[test]
fn drops_the_held_item_and_the_cursor_stack() {
    let mut w = world(Dimension::Surface);
    let p = player_at(&mut w, 0.5, 61.0, 0.5);
    let id = p.id();
    w.spawn_entity_in_world(p);
    w.with_entity_mut(id, |w, p| {
        pd_mut(p).inventory.main_inventory[0] = Some(ItemStack::new(4, 3, 0));
        player::drop_current_item(p, w);
        assert_eq!(
            pd(p).inventory.main_inventory[0],
            Some(ItemStack::new(4, 2, 0))
        );
        pd_mut(p).inventory.item_stack = Some(ItemStack::new(5, 1, 0));
        dispatch::set_entity_dead(p, w);
        assert_eq!(pd(p).inventory.item_stack, None);
        assert!(p.base.is_dead);
    });
    let items: Vec<_> = w
        .loaded_entity_list
        .iter()
        .filter_map(|s| s.entity.as_deref())
        .filter_map(|e| e.kind.item().map(|d| (e, d)))
        .collect();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].1.item, Some(ItemStack::new(4, 1, 0)));
    assert_eq!(items[0].1.delay_before_can_pickup, 40);
    // Thrown along yaw 0 (+z) from the eyes: posY - 0.3 + 0.12.
    assert!(items[0].0.base.motion_z > 0.2);
    assert_eq!(items[1].1.item, Some(ItemStack::new(5, 1, 0)));
}

/// `decrStackSize`: the whole stack when it is not larger than the count;
/// slots 36-39 are the armour.
#[test]
fn decr_stack_size_covers_both_arrays() {
    let mut inv = InventoryPlayer::default();
    inv.main_inventory[1] = Some(ItemStack::new(4, 2, 0));
    assert_eq!(inv.decr_stack_size(1, 5), Some(ItemStack::new(4, 2, 0)));
    assert_eq!(inv.main_inventory[1], None);
    inv.armor_inventory[2] = Some(ItemStack::new(300, 1, 0));
    assert_eq!(inv.decr_stack_size(38, 1), Some(ItemStack::new(300, 1, 0)));
    assert_eq!(inv.decr_stack_size(38, 1), None);
}

/// `PlayerControllerSP`: a flower breaks on the click; stone held bare-handed
/// accumulates `1 / 1.5 / 100` per tick from the second tick; after a break
/// the controller waits 5 ticks; `resetBlockRemoving` zeroes the damage.
#[test]
fn controller_instant_break_damage_and_wait() {
    let mut w = world(Dimension::Surface);
    w.set_block(2, 60, 4, 3);
    w.set_block(2, 61, 4, 37);
    w.set_block(0, 61, 4, STONE);
    let p = player_at(&mut w, 0.5, 61.0, 0.5);
    let id = p.id();
    w.spawn_entity_in_world(p);
    w.with_entity_mut(id, |w, p| {
        // The player must stand on the ground for the full strength.
        p.base.on_ground = true;
        let mut c = PlayerControllerSp::new();
        assert!(c.click_block(w, p, 2, 61, 4, 2));
        assert_eq!(w.get_block_id(2, 61, 4), 0);

        assert!(!c.send_block_removing(w, p, 0, 61, 4, 2));
        assert_eq!(c.cur_block_damage, 0.0);
        assert_eq!(
            (c.current_block_x, c.current_block_y, c.current_block_z),
            (0, 61, 4)
        );
        assert!(!c.send_block_removing(w, p, 0, 61, 4, 2));
        assert_eq!(c.cur_block_damage, 1.0 / 1.5 / 100.0);
        assert_eq!(
            c.sounds.len(),
            1,
            "the step sound on the first damaging tick"
        );
        c.reset_block_removing();
        assert_eq!(c.cur_block_damage, 0.0);

        c.cur_block_damage = 0.999;
        assert!(c.send_block_removing(w, p, 0, 61, 4, 2));
        assert_eq!(w.get_block_id(0, 61, 4), 0);
        assert_eq!(c.block_hit_wait, 5);
        for left in (0..5).rev() {
            assert!(!c.send_block_removing(w, p, 0, 61, 4, 2));
            assert_eq!(c.block_hit_wait, left);
        }
        assert_eq!(c.get_block_reach_distance(), 4.0);
        c.update_controller();
        assert_eq!(c.damage_partial_time(0.5), 0.0);
    });
}

/// `func_25060_a`: the free spot next to a bed, `None` once it is gone.
#[test]
fn verify_respawn_coordinates_finds_the_spot() {
    let mut w = world(Dimension::Surface);
    bed(&mut w, 0, 61, 2);
    let c = ChunkCoordinates::new(0, 61, 3);
    assert_eq!(
        player::verify_respawn_coordinates(&mut w, c),
        Some(ChunkCoordinates::new(-1, 61, 2))
    );
    w.set_block(0, 61, 3, 0);
    assert_eq!(player::verify_respawn_coordinates(&mut w, c), None);
}

/// `getYOffset` is `yOffset - 0.5`; `getEyeHeight` is 0.12.
#[test]
fn offsets() {
    let mut w = world(Dimension::Surface);
    let p = player_at(&mut w, 0.5, 61.0, 0.5);
    assert_eq!(dispatch::get_y_offset(&p, &w), (1.62f32 - 0.5) as f64);
    assert_eq!(dispatch::get_eye_height(&p, &w), 0.12);
}
