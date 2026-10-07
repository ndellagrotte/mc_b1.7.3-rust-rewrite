//! `ItemMap` (ItemMap.java; `ItemMapBase`, ItemMapBase.java, adds nothing):
//! the map item. Its damage value is the map id; the picture is the
//! [`MapData`] `"map_<id>"` in the world's [`MapStorage`]
//! (`World.field_28108_z`, [`crate::world::map_storage`]).
//!
//! Every tick a map in a player's main inventory registers the player as a
//! viewer (`updateVisiblePlayers`), and while it is the selected item it
//! redraws one column in sixteen of the area around the player
//! (`updateMapData`): per map pixel the most common block (by count) of the
//! top surface of the `scale × scale` block square, through the material's
//! `MapColor`, shaded by the height step from the pixel to the north
//! (water by its depth instead).
//!
//! [`MapStorage`]: crate::world::map_storage::MapStorage

use mc_core::math::floor_double;

use super::ItemStack;
use crate::block::{self, Material, material::MapColor};
use crate::entity::Entity;
use crate::world::World;
use crate::world::chunk::Chunk;
use crate::world::map_storage::{MapData, MapDataRef, MapViewer};

/// `Block.dirt.blockID`.
// src: Block.java:595
const DIRT: i32 = 3;
/// `Block.stone.blockID`.
// src: Block.java:593
const STONE: i32 = 1;

/// The `MapData` name of map id `id`: `"map_" + id`.
// src: ItemMap.java:11, :24
pub fn map_name(id: i32) -> String {
    format!("map_{id}")
}

/// `ItemMap.getMPMapData(id, world)` (static; the multiplayer client's map
/// packet handler is its only caller, so nothing in single player reaches
/// it): the loaded or stored `map_<id>`, else a new blank one under a fresh
/// id (`"map_" + getUniqueDataId("map")`, not `map_<id>`).
// src: ItemMap.java:9-20
pub fn get_mp_map_data(var0: i16, var1: &mut World) -> MapDataRef {
    if let Some(var3) = var1.load_item_data(&map_name(i32::from(var0))) {
        return var3;
    }
    let var4 = var1.get_unique_data_id("map");
    let var2 = map_name(var4);
    let var3 = MapData::new(&var2).into_ref();
    var1.set_item_data(&var2, var3.clone());
    var3
}

/// `ItemMap.getMapData(stack, world)`: `map_<damage>`, loaded or read from
/// the save; failing both, the stack gets a fresh id (`getUniqueDataId`)
/// and a new map centred on the world spawn, scale 3, in the world's
/// dimension, marked dirty.
// src: ItemMap.java:22-38
pub fn get_map_data(var1: &mut ItemStack, var2: &mut World) -> MapDataRef {
    if let Some(var4) = var2.load_item_data(&map_name(var1.get_item_damage())) {
        return var4;
    }
    let id = var2.get_unique_data_id("map");
    var1.set_item_damage(id);
    let var3 = map_name(var1.get_item_damage());
    let mut var4 = MapData::new(&var3);
    var4.x_center = var2.world_info.spawn_x;
    var4.z_center = var2.world_info.spawn_z;
    var4.scale = 3;
    var4.dimension = var2.world_provider.world_type as i8;
    var4.mark_dirty();
    let var4 = var4.into_ref();
    var2.set_item_data(&var3, var4.clone());
    var4
}

/// `Chunk.getBlockID(x, y, z)` with Java's array check: the map's surface
/// walk can step below y = 0 in a column of air-coloured blocks, where Java
/// throws `ArrayIndexOutOfBoundsException` (here a panic naming it).
// src: Chunk.java:232-234
fn chunk_block_id(c: &Chunk, x: i32, y: i32, z: i32) -> i32 {
    let i = x << 11 | z << 7 | y;
    if !(0..32768).contains(&i) {
        panic!(
            "Chunk.getBlockID({x}, {y}, {z}): Java ArrayIndexOutOfBoundsException (ItemMap.updateMapData)"
        );
    }
    c.get_block_id(x, y, z)
}

/// `Block.blocksList[id].blockMaterial` for a non-zero id read from a chunk
/// (a null entry would throw `NullPointerException`).
fn material(id: i32) -> Material {
    match block::block(id) {
        Some(b) => b.block_material,
        None => panic!(
            "Block.blocksList[{id}] is null (Java NullPointerException, ItemMap.updateMapData)"
        ),
    }
}

/// `ItemMap.updateMapData(world, entity, data)`: in the map's dimension,
/// advances `step` and redraws the columns `x ≡ step (mod 16)` within
/// `128 / 2^scale` pixels of the entity (half that in the nether, where
/// every pixel is a hash-chosen dirt or stone at height 100). Changed
/// pixels inside the visible disc (its rim on a checkerboard) are written
/// and their rows marked dirty for the viewers. Loads (and generates) the
/// chunks it samples.
// src: ItemMap.java:40-211
// Java's branch-per-condition shape is kept.
#[allow(clippy::if_same_then_else)]
pub fn update_map_data(var1: &mut World, var2: &Entity, var3: &mut MapData) {
    if var1.world_provider.world_type != i32::from(var3.dimension) {
        return;
    }
    let var4: i32 = 128;
    let var5: i32 = 128;
    let var6 = 1i32.wrapping_shl(var3.scale as i32 as u32);
    let var7 = var3.x_center;
    let var8 = var3.z_center;
    let var9 = (floor_double(var2.base.pos_x - var7 as f64) / var6).wrapping_add(var4 / 2);
    let var10 = (floor_double(var2.base.pos_z - var8 as f64) / var6).wrapping_add(var5 / 2);
    let mut var11 = 128 / var6;
    let has_no_sky = var1.world_provider.has_no_sky;
    if has_no_sky {
        var11 /= 2;
    }
    var3.step = var3.step.wrapping_add(1);
    let mut var12 = var9.wrapping_sub(var11).wrapping_add(1);
    while var12 < var9.wrapping_add(var11) {
        if (var12 & 15) == (var3.step & 15) {
            let mut var13: i32 = 255;
            let mut var14: i32 = 0;
            let mut var15: f64 = 0.0;
            let mut var17 = var10.wrapping_sub(var11).wrapping_sub(1);
            while var17 < var10.wrapping_add(var11) {
                if var12 >= 0 && var17 >= -1 && var12 < var4 && var17 < var5 {
                    let var18 = var12 - var9;
                    let var19 = var17 - var10;
                    let d2 = var18
                        .wrapping_mul(var18)
                        .wrapping_add(var19.wrapping_mul(var19));
                    let var20 = d2 > (var11 - 2).wrapping_mul(var11 - 2);
                    let var21 = (var7 / var6 + var12 - var4 / 2).wrapping_mul(var6);
                    let var22 = (var8 / var6 + var17 - var5 / 2).wrapping_mul(var6);
                    let mut var26 = [0i32; 256];
                    let var27 = var1.get_chunk_from_block_coords(var21, var22);
                    let var28 = var21 & 15;
                    let var29 = var22 & 15;
                    let mut var30: i32 = 0;
                    let mut var31: f64 = 0.0;
                    if has_no_sky {
                        // src: ItemMap.java:82-94
                        let mut var33 = var21.wrapping_add(var22.wrapping_mul(231871));
                        var33 = var33
                            .wrapping_mul(var33)
                            .wrapping_mul(31287121)
                            .wrapping_add(var33.wrapping_mul(11));
                        if (var33 >> 20 & 1) == 0 {
                            var26[DIRT as usize] += 10;
                        } else {
                            var26[STONE as usize] += 10;
                        }
                        var31 = 100.0;
                    } else {
                        // src: ItemMap.java:96-140
                        for var33 in 0..var6 {
                            for var34 in 0..var6 {
                                let cx = var33 + var28;
                                let cz = var34 + var29;
                                let mut var35 = var27.get_height_value(cx, cz) + 1;
                                let mut var36: i32 = 0;
                                if var35 > 1 {
                                    'label164: loop {
                                        let mut var37 = true;
                                        var36 = chunk_block_id(var27, cx, var35 - 1, cz);
                                        if var36 == 0 {
                                            var37 = false;
                                        } else if var35 > 0
                                            && var36 > 0
                                            && material(var36).material_map_color()
                                                == MapColor::AIR_COLOR
                                        {
                                            var37 = false;
                                        }
                                        if !var37 {
                                            var35 -= 1;
                                            var36 = chunk_block_id(var27, cx, var35 - 1, cz);
                                        }
                                        if var37 {
                                            if var36 == 0 || !material(var36).get_is_liquid() {
                                                break;
                                            }
                                            let mut var38 = var35 - 1;
                                            loop {
                                                let var43 = chunk_block_id(var27, cx, var38, cz);
                                                var38 -= 1;
                                                var30 += 1;
                                                if var38 <= 0
                                                    || var43 == 0
                                                    || !material(var43).get_is_liquid()
                                                {
                                                    break 'label164;
                                                }
                                            }
                                        }
                                    }
                                }
                                var31 += var35 as f64 / var6.wrapping_mul(var6) as f64;
                                var26[var36 as usize] += 1;
                            }
                        }
                    }
                    // src: ItemMap.java:143-146 (var23..var25 / (var6 * var6): dead)
                    var30 /= var6.wrapping_mul(var6);
                    let mut var33 = 0;
                    let mut var34 = 0usize;
                    for (var35, &n) in var26.iter().enumerate() {
                        if n > var33 {
                            var34 = var35;
                            var33 = n;
                        }
                    }
                    let parity = (var12.wrapping_add(var17) & 1) as f64;
                    let mut var41 =
                        (var31 - var15) * 4.0 / (var6 + 4) as f64 + (parity - 0.5) * 0.4;
                    let mut var42: i32 = 1;
                    if var41 > 0.6 {
                        var42 = 2;
                    }
                    if var41 < -0.6 {
                        var42 = 0;
                    }
                    let mut var38: i32 = 0;
                    if var34 > 0 {
                        let var44 = material(var34 as i32).material_map_color();
                        if var44 == MapColor::WATER_COLOR {
                            var41 = var30 as f64 * 0.1 + parity * 0.2;
                            var42 = 1;
                            if var41 < 0.5 {
                                var42 = 2;
                            }
                            if var41 > 0.9 {
                                var42 = 0;
                            }
                        }
                        var38 = var44.color_index;
                    }
                    var15 = var31;
                    if var17 >= 0
                        && d2 < var11.wrapping_mul(var11)
                        && (!var20 || (var12.wrapping_add(var17) & 1) != 0)
                    {
                        let idx = var12 + var17 * var4;
                        let Some(slot) = usize::try_from(idx)
                            .ok()
                            .and_then(|i| var3.colors.get_mut(i))
                        else {
                            panic!(
                                "MapData.colors[{idx}]: Java ArrayIndexOutOfBoundsException (ItemMap.updateMapData)"
                            );
                        };
                        let var45 = *slot as i8;
                        let var40 = var38.wrapping_mul(4).wrapping_add(var42) as i8;
                        if var45 != var40 {
                            if var13 > var17 {
                                var13 = var17;
                            }
                            if var14 < var17 {
                                var14 = var17;
                            }
                            *slot = var40 as u8;
                        }
                    }
                }
                var17 += 1;
            }
            if var13 <= var14 {
                var3.set_column_dirty(var12, var13, var14);
            }
        }
        var12 += 1;
    }
}

/// `ItemMap.onUpdate(stack, world, entity, slot, isCurrentItem)`: not on a
/// multiplayer client: the map data (created if missing), the entity
/// registered as a viewer when it is a player, and the picture redrawn
/// while the map is the selected item.
// src: ItemMap.java:213-226
pub fn on_update(var1: &mut ItemStack, var2: &mut World, var3: &mut Entity, var5: bool) {
    if var2.multiplayer_world {
        return;
    }
    let var6 = get_map_data(var1, var2);
    if var3.kind.is_player() {
        let w: &World = var2;
        var6.borrow_mut()
            .update_visible_players(var3, var1, |id, s| {
                w.entity(id).and_then(|p| {
                    let inv = &p.kind.player()?.inventory;
                    Some(MapViewer::of(p, inv.has_item_stack(s)))
                })
            });
    }
    if var5 {
        update_map_data(var2, var3, &mut var6.borrow_mut());
    }
}

/// `ItemMap.onCreated(stack, world, player)`: a crafted map gets a fresh id
/// and a new map centred on the player (floored), scale 3, in the world's
/// dimension, marked dirty.
// src: ItemMap.java:228-238
pub fn on_created(var1: &mut ItemStack, var2: &mut World, var3: &Entity) {
    let id = var2.get_unique_data_id("map");
    var1.set_item_damage(id);
    let var4 = map_name(var1.get_item_damage());
    let var5 = MapData::new(&var4).into_ref();
    var2.set_item_data(&var4, var5.clone());
    let mut d = var5.borrow_mut();
    d.x_center = floor_double(var3.base.pos_x);
    d.z_center = floor_double(var3.base.pos_z);
    d.scale = 3;
    d.dimension = var2.world_provider.world_type as i8;
    d.mark_dirty();
}
