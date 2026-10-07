//! Item behaviour (WP-2.6): the virtual methods of `Item` (Item.java) that
//! act on the world, and every override of them in the `Item*.java`
//! subclasses.
//!
//! # Shape (mirrors [`crate::block::dispatch`])
//!
//! One free function per `Item` behaviour method, dispatched by `match` on
//! the stack's [`ItemKind`]: the arm of the class that overrides the method
//! (the nearest class at or above the item's class that declares it), the
//! base `Item` body otherwise. The bodies are in one module per group of
//! classes:
//!
//! | Module | Classes |
//! |---|---|
//! | [`block_items`] | `ItemBlock` (and its subclasses `ItemCloth`, `ItemLog`, `ItemSlab`, `ItemSapling`, `ItemLeaves`, `ItemPiston`, which only change the placed metadata), `ItemReed`, `ItemDoor`, `ItemBed`, `ItemSign`, `ItemRedstone`, `ItemSeeds` |
//! | [`tools`] | `ItemTool` (`ItemSpade`, `ItemPickaxe`, `ItemAxe`), `ItemSword`, `ItemShears`, `ItemHoe`, `ItemFlintAndSteel` |
//! | [`projectiles`] | `ItemBow`, `ItemSnowball`, `ItemEgg`, `ItemFishingRod` |
//! | [`bucket`] | `ItemBucket` |
//! | [`vehicles`] | `ItemBoat`, `ItemMinecart`, `ItemPainting` |
//! | [`misc`] | `ItemSaddle`, `ItemFood` (`ItemCookie`), `ItemSoup`, `ItemRecord`, `ItemDye` |
//! | [`map`] | `ItemMap` (`ItemMapBase` declares nothing) |
//!
//! `ItemArmor` and `ItemCoal` override no behaviour method. The data
//! methods (`getIconFromDamage`, `getStrVsBlock`, `getDamageVsEntity`,
//! `canHarvestBlock`, `isFull3D`, …) are on [`ItemDef`]
//! (WP-1.2); `crates/mc-sim/tests/item_dispatch_coverage.rs` holds the
//! split ([`DATA_METHODS`]) against the vanilla override set
//! (`tests/data/item_overrides.txt`).
//!
//! The `ItemStack` methods that call these (`useItem`, `useItemRightClick`,
//! `hitEntity`, `onDestroyBlock`, `useItemOnEntity`, `updateAnimation`,
//! `onCrafting`) are on [`ItemStack`] ([`super::stack`]).
//!
//! # Stacks and players
//!
//! Java passes the `ItemStack` object, which also sits in an inventory slot;
//! stacks are values here, so every function takes `&mut ItemStack` and the
//! caller moves the stack out of its slot for the call and back afterwards
//! ([`crate::player_controller::send_place_block`]): what the item does to
//! it lands in the slot, as on Java's shared object. The player (or other
//! entity) is out of its `loadedEntityList` slot too (the take/put-back
//! pattern, [`crate::entity`]), so world queries do not see it; the one
//! vanilla query on this path that would find it, the entity check of
//! `World.canBlockBePlacedAt` in `ItemBlock` / `ItemReed`, is completed by
//! `can_block_be_placed_at` (crate-private).
//!
//! # Randomness
//!
//! `Item.itemRand` is [`World::item_rand`] (a per-run static, seeded
//! `mix(seed, -1)` by the oracle). Entities the items construct draw from
//! their own `rand` in the constructor (arrow, snowball, egg and bobber
//! heading jitter, the painting's art): in the oracle those draws happen
//! before the join hook seeds the entity, so they are unseeded there
//! (tools/oracle/census.md §3) and the port's values (on the stream seeded
//! at construction) cannot be compared; fixtures avoid tracing them.
//!
//! # Out of scope
//!
//! `getMPMapData` (ItemMap.java:9-20, the multiplayer client's map packet)
//! is transcribed for completeness in [`map`] but has no caller.
//! `EntityPlayerSP.displayGUIEditSign` (the sign editor, client GUI) is
//! recorded where `ItemSign` calls it; `EntityPlayer`'s is empty.

pub mod block_items;
pub mod bucket;
pub mod map;
pub mod misc;
pub mod projectiles;
pub mod tools;
pub mod vehicles;

use mc_core::Vec3D;
use mc_core::math;

use super::{ItemDef, ItemKind, ItemStack};
use crate::block::{self, BlockDef, dispatch as block_dispatch};
use crate::entity::{Entity, player};
use crate::stats::StatBase;
use crate::world::World;
use crate::world::ray_trace::MovingObjectPosition;

/// What `Item.onItemRightClick` returned, standing in for Java's identity
/// test `var5 != var3` in `PlayerController.sendUseItem`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RightClickResult {
    /// The stack object passed in (possibly changed in place).
    Same,
    /// Another object (`ItemBucket`, `ItemSoup`). Never `null` in b1.7.3:
    /// no `onItemRightClick` returns `null`.
    Replaced(ItemStack),
}

/// Overrides transcribed, by the class that declares them: one row per
/// class, `(class, [method, …])`, methods in Java names. Since WP-2.6 every
/// behaviour override of the vanilla tree is here;
/// `tests/item_dispatch_coverage.rs` checks the rows against
/// `tests/data/item_overrides.txt` and that no entry point records a gap.
pub const TRANSCRIBED: &[(&str, &[&str])] = &[
    // src: ItemBed.java:8-42
    ("ItemBed", &["onItemUse"]),
    // src: ItemBlock.java:12-58
    ("ItemBlock", &["onItemUse"]),
    // src: ItemBoat.java:9-46
    ("ItemBoat", &["onItemRightClick"]),
    // src: ItemBow.java:9-18
    ("ItemBow", &["onItemRightClick"]),
    // src: ItemBucket.java:12-99
    ("ItemBucket", &["onItemRightClick"]),
    // src: ItemDoor.java:12-72
    ("ItemDoor", &["onItemUse"]),
    // src: ItemDye.java:21-90
    ("ItemDye", &["onItemUse", "saddleEntity"]),
    // src: ItemEgg.java:9-17
    ("ItemEgg", &["onItemRightClick"]),
    // src: ItemFishingRod.java:18-33
    ("ItemFishingRod", &["onItemRightClick"]),
    // src: ItemFlintAndSteel.java:10-43
    ("ItemFlintAndSteel", &["onItemUse"]),
    // src: ItemFood.java:14-18
    ("ItemFood", &["onItemRightClick"]),
    // src: ItemHoe.java:10-26
    ("ItemHoe", &["onItemUse"]),
    // src: ItemMap.java:213-238
    ("ItemMap", &["onCreated", "onUpdate"]),
    // src: ItemMinecart.java:12-24
    ("ItemMinecart", &["onItemUse"]),
    // src: ItemPainting.java:8-38
    ("ItemPainting", &["onItemUse"]),
    // src: ItemRecord.java:12-25
    ("ItemRecord", &["onItemUse"]),
    // src: ItemRedstone.java:8-45
    ("ItemRedstone", &["onItemUse"]),
    // src: ItemReed.java:11-55
    ("ItemReed", &["onItemUse"]),
    // src: ItemSaddle.java:9-23
    ("ItemSaddle", &["hitEntity", "saddleEntity"]),
    // src: ItemSeeds.java:11-24
    ("ItemSeeds", &["onItemUse"]),
    // src: ItemShears.java:10-16
    ("ItemShears", &["onBlockDestroyed"]),
    // src: ItemSign.java:9-53
    ("ItemSign", &["onItemUse"]),
    // src: ItemSnowball.java:9-17
    ("ItemSnowball", &["onItemRightClick"]),
    // src: ItemSoup.java:8-11
    ("ItemSoup", &["onItemRightClick"]),
    // src: ItemSword.java:17-25
    ("ItemSword", &["hitEntity", "onBlockDestroyed"]),
    // src: ItemTool.java:29-37
    ("ItemTool", &["hitEntity", "onBlockDestroyed"]),
];

/// `Item` methods whose overrides are data, transcribed on
/// [`ItemDef`] (WP-1.2) and pinned per item by
/// `tests/golden/item/item-table`; the coverage test takes them as done.
pub const DATA_METHODS: &[&str] = &[
    "canHarvestBlock",
    "getColorFromDamage",
    "getDamageVsEntity",
    "getIconFromDamage",
    "getItemName",
    "getItemNameIS",
    "getPlacedBlockMetadata",
    "getStrVsBlock",
    "isFull3D",
    "shouldRotateAroundWhenRendering",
];

/// `Item.itemsList[stack.itemID]`, where Java dereferences it: panics where
/// Java throws `NullPointerException` (an id without an item, which a
/// validated world never holds, PLAN_v4 D5).
pub(crate) fn item_of(stack: &ItemStack) -> &'static ItemDef {
    match stack.get_item() {
        Some(i) => i,
        None => panic!(
            "Item.itemsList[{}] is null (Java NullPointerException)",
            stack.item_id
        ),
    }
}

// ---- dispatch ---------------------------------------------------------------------

/// `Item.onItemUse(stack, player, world, x, y, z, side)`: a right click on
/// face `side` of block `(x, y, z)`; true when the click was used (the
/// caller then swings the arm and adds the use stat). Base: false.
// src: Item.java:155-157
#[allow(clippy::too_many_arguments)]
pub fn on_item_use(
    stack: &mut ItemStack,
    player: &mut Entity,
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    side: i32,
) -> bool {
    use ItemKind as K;
    let def = item_of(stack);
    match def.kind {
        K::Block { block_id }
        | K::Cloth { block_id }
        | K::Log { block_id }
        | K::Slab { block_id }
        | K::Sapling { block_id }
        | K::Leaves { block_id }
        | K::Piston { block_id } => {
            block_items::item_block_on_item_use(def, block_id, stack, player, world, x, y, z, side)
        }
        K::Reed { spawn_id } => {
            block_items::reed_on_item_use(spawn_id, stack, player, world, x, y, z, side)
        }
        K::Door { door_material } => {
            block_items::door_on_item_use(door_material, stack, player, world, x, y, z, side)
        }
        K::Bed => block_items::bed_on_item_use(stack, player, world, x, y, z, side),
        K::Sign => block_items::sign_on_item_use(stack, player, world, x, y, z, side),
        K::Redstone => block_items::redstone_on_item_use(stack, world, x, y, z, side),
        K::Seeds { block_type } => {
            block_items::seeds_on_item_use(block_type, stack, world, x, y, z, side)
        }
        K::Hoe => tools::hoe_on_item_use(stack, player, world, x, y, z, side),
        K::FlintAndSteel => tools::flint_and_steel_on_item_use(stack, player, world, x, y, z, side),
        K::Minecart { minecart_type } => {
            vehicles::minecart_on_item_use(minecart_type, stack, world, x, y, z)
        }
        K::Painting => vehicles::painting_on_item_use(stack, world, x, y, z, side),
        K::Record { .. } => misc::record_on_item_use(def, stack, world, x, y, z),
        K::Dye => misc::dye_on_item_use(stack, world, x, y, z),
        // src: Item.java:155-157
        _ => false,
    }
}

/// `Item.onItemRightClick(stack, world, player)`: a right click with the
/// item that hit no block (or used none). Base: the stack itself.
// src: Item.java:163-165
pub fn on_item_right_click(
    stack: &mut ItemStack,
    world: &mut World,
    player: &mut Entity,
) -> RightClickResult {
    use ItemKind as K;
    let def = item_of(stack);
    match def.kind {
        K::Bow => projectiles::bow_on_item_right_click(stack, world, player),
        K::Snowball => projectiles::snowball_on_item_right_click(stack, world, player),
        K::Egg => projectiles::egg_on_item_right_click(stack, world, player),
        K::FishingRod => projectiles::fishing_rod_on_item_right_click(stack, world, player),
        K::Bucket { is_full } => bucket::on_item_right_click(is_full, stack, world, player),
        K::Boat => vehicles::boat_on_item_right_click(stack, world, player),
        K::Food(f) | K::Cookie(f) => misc::food_on_item_right_click(f, stack, player),
        K::Soup(f) => misc::soup_on_item_right_click(f, stack, player),
        // src: Item.java:163-165
        _ => RightClickResult::Same,
    }
}

/// `Item.hitEntity(stack, target, attacker)`: the attacker hit `target`
/// with the stack; true counts a use. Base: false.
// src: Item.java:197-199
pub fn hit_entity(
    stack: &mut ItemStack,
    target: &mut Entity,
    attacker: &mut Entity,
    world: &mut World,
) -> bool {
    use ItemKind as K;
    match item_of(stack).kind {
        K::Spade(_) | K::Pickaxe(_) | K::Axe(_) => tools::tool_hit_entity(stack, attacker),
        K::Sword { .. } => tools::sword_hit_entity(stack, attacker),
        K::Saddle => misc::saddle_hit_entity(stack, target, world),
        // src: Item.java:197-199
        _ => false,
    }
}

/// `Item.onBlockDestroyed(stack, blockId, x, y, z, entity)`: the entity
/// broke the block with the stack; true counts a use. Base: false.
// src: Item.java:201-203
#[allow(clippy::too_many_arguments)]
pub fn on_block_destroyed(
    stack: &mut ItemStack,
    block_id: i32,
    x: i32,
    y: i32,
    z: i32,
    entity: &mut Entity,
    world: &mut World,
) -> bool {
    let _ = (x, y, z, world);
    use ItemKind as K;
    match item_of(stack).kind {
        K::Spade(_) | K::Pickaxe(_) | K::Axe(_) => tools::tool_on_block_destroyed(stack, entity),
        K::Sword { .. } => tools::sword_on_block_destroyed(stack, entity),
        K::Shears => tools::shears_on_block_destroyed(stack, block_id, entity),
        // src: Item.java:201-203
        _ => false,
    }
}

/// `Item.saddleEntity(stack, target)`: the item used on a living entity
/// (`ItemStack.useItemOnEntity`). Base: nothing.
// src: Item.java:213-214
pub fn saddle_entity(stack: &mut ItemStack, target: &mut Entity, world: &mut World) {
    use ItemKind as K;
    match item_of(stack).kind {
        K::Saddle => misc::saddle_saddle_entity(stack, target, world),
        K::Dye => misc::dye_saddle_entity(stack, target, world),
        // src: Item.java:213-214
        _ => {}
    }
}

/// `Item.onUpdate(stack, world, entity, slot, isCurrentItem)`: every tick
/// for each stack in a player's main inventory
/// (`InventoryPlayer.decrementAnimations`). Base: nothing.
// src: Item.java:267-268
pub fn on_update(
    stack: &mut ItemStack,
    world: &mut World,
    entity: &mut Entity,
    slot: i32,
    is_current: bool,
) {
    let _ = slot;
    if let ItemKind::Map = item_of(stack).kind {
        map::on_update(stack, world, entity, is_current);
    }
}

/// `Item.onCreated(stack, world, player)`: the stack was taken from a
/// crafting or furnace output (`ItemStack.onCrafting`). Base: nothing.
// src: Item.java:270-271
pub fn on_created(stack: &mut ItemStack, world: &mut World, player: &Entity) {
    if let ItemKind::Map = item_of(stack).kind {
        map::on_created(stack, world, player);
    }
}

// ---- shared helpers ---------------------------------------------------------------

/// The block coordinates in front of face `side` of `(x, y, z)` (0 −y,
/// 1 +y, 2 −z, 3 +z, 4 −x, 5 +x), the six `if (side == n)` steps every
/// placing item repeats (e.g. ItemBlock.java:16-38).
pub(crate) fn offset_by_side(side: i32, mut x: i32, mut y: i32, mut z: i32) -> (i32, i32, i32) {
    if side == 0 {
        y -= 1;
    }
    if side == 1 {
        y += 1;
    }
    if side == 2 {
        z -= 1;
    }
    if side == 3 {
        z += 1;
    }
    if side == 4 {
        x -= 1;
    }
    if side == 5 {
        x += 1;
    }
    (x, y, z)
}

/// The placement sound of `ItemBlock` / `ItemReed` / `ItemHoe`:
/// `playSoundEffect(x + 0.5F, y + 0.5F, z + 0.5F, stepSoundDir2,
/// (volume + 1) / 2, pitch * 0.8)` of `block`'s step sound.
// src: ItemBlock.java:50, ItemReed.java:48, ItemHoe.java:17
pub(crate) fn play_place_sound(world: &mut World, block: &BlockDef, x: i32, y: i32, z: i32) {
    let s = block.step_sound;
    world.play_sound_effect(
        (x as f32 + 0.5) as f64,
        (y as f32 + 0.5) as f64,
        (z as f32 + 0.5) as f64,
        &s.step_sound_dir2(),
        (s.get_volume() + 1.0) / 2.0,
        s.get_pitch() * 0.8,
    );
}

/// `world.canBlockBePlacedAt(id, x, y, z, ignoreCollision, side)` with the
/// placing player in hand: `checkIfAABBIsClear` (World.java:1357-1368)
/// would see the player, a living entity with `preventEntitySpawning`
/// (EntityLiving.java:62), when its box intersects the block's collision
/// box, and refuse; the world's scan cannot see it while it is out of its
/// slot (module docs), so it is tested here first. The rest is
/// [`World::can_block_be_placed_at`] (the same box, the other entities, the
/// replaceable blocks, `canPlaceBlockOnSide`).
// src: World.java:2096-2114
#[allow(clippy::too_many_arguments)]
pub(crate) fn can_block_be_placed_at(
    world: &mut World,
    player: &Entity,
    block_id: i32,
    x: i32,
    y: i32,
    z: i32,
    ignore_collision: bool,
    side: i32,
) -> bool {
    if !ignore_collision
        && block::block(block_id).is_some()
        && let Some(bb) =
            block_dispatch::get_collision_bounding_box_from_pool(world, block_id, x, y, z)
        && !player.base.is_dead
        && player.base.prevent_entity_spawning
        && player.base.bounding_box.intersects_with(&bb)
    {
        return false;
    }
    world.can_block_be_placed_at(block_id, x, y, z, ignore_collision, side)
}

/// The player's eye ray of `ItemBucket` / `ItemBoat`, five blocks long
/// along the look direction (with the partial tick 1, so the current
/// position and angles), traced with `rayTraceBlocks_do(start, end,
/// stop_on_liquid)`. Returns the start position (`var7`, `var9`, `var11`)
/// and the hit.
// src: ItemBucket.java:13-28, ItemBoat.java:10-25
pub(crate) fn player_look_trace(
    world: &mut World,
    player: &Entity,
    stop_on_liquid: bool,
) -> ([f64; 3], Option<MovingObjectPosition>) {
    let b = &player.base;
    let var4: f32 = 1.0;
    let var5 = b.prev_rotation_pitch + (b.rotation_pitch - b.prev_rotation_pitch) * var4;
    let var6 = b.prev_rotation_yaw + (b.rotation_yaw - b.prev_rotation_yaw) * var4;
    let var7 = b.prev_pos_x + (b.pos_x - b.prev_pos_x) * var4 as f64;
    let var9 = b.prev_pos_y + (b.pos_y - b.prev_pos_y) * var4 as f64 + 1.62 - b.y_offset as f64;
    let var11 = b.prev_pos_z + (b.pos_z - b.prev_pos_z) * var4 as f64;
    let mut var13 = Vec3D::create_vector(var7, var9, var11);
    let var14 = math::cos(-var6 * 0.017453292 - 3.1415927);
    let var15 = math::sin(-var6 * 0.017453292 - 3.1415927);
    let var16 = -math::cos(-var5 * 0.017453292);
    let var17 = math::sin(-var5 * 0.017453292);
    let var18 = var15 * var16;
    let var20 = var14 * var16;
    let var21 = 5.0f64;
    let var23 = var13.add_vector(
        var18 as f64 * var21,
        var17 as f64 * var21,
        var20 as f64 * var21,
    );
    let hit = world.ray_trace_blocks_do(&mut var13, &var23, stop_on_liquid);
    ([var7, var9, var11], hit)
}

/// `EntityPlayer.addStat(stat, n)` for the player `e`
/// ([`player::add_stat`], `EntityPlayerSP`'s stat file), or nothing when
/// `e` is not a player (`ItemStack.damageItem`'s `instanceof EntityPlayer`).
pub(crate) fn add_stat_if_player(e: &mut Entity, stat: Option<&'static StatBase>, n: i32) {
    if e.kind.is_player() {
        player::add_stat(e, stat, n);
    }
}
