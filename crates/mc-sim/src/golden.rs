//! Read-only views of simulation state in the shapes the golden traces use
//! (tests/golden/README.md, "Tier B/C/S scenarios"; the oracle side is
//! `tools/oracle/java/harness/Dumps.java`). `mc-golden` turns these into
//! JSON lines and digests; nothing here loads a chunk ("a dump never loads a
//! chunk").

use crate::entity::{Entity, EntityKindData};
use crate::item::ItemStack;
use crate::world::World;
use crate::world::chunk::Chunk;
use crate::world::lighting::EnumSkyBlock;

/// The five arrays of a `chunk_digest` line, as raw bytes: `Chunk.blocks`
/// (32768), `data.data`, `skylightMap.data`, `blocklightMap.data` (16384
/// each) and `heightMap` (256).
#[derive(Clone, Copy, Debug)]
pub struct ChunkArrays<'a> {
    /// `blocks`.
    pub blocks: &'a [u8],
    /// `data.data`.
    pub data: &'a [u8],
    /// `skylightMap.data`.
    pub skylight: &'a [u8],
    /// `blocklightMap.data`.
    pub blocklight: &'a [u8],
    /// `heightMap`.
    pub heightmap: &'a [u8],
}

impl<'a> ChunkArrays<'a> {
    /// The arrays of one chunk.
    pub fn of(c: &'a Chunk) -> Self {
        ChunkArrays {
            blocks: &c.blocks[..],
            data: &c.data.data,
            skylight: &c.skylight_map.data,
            blocklight: &c.blocklight_map.data,
            heightmap: &c.height_map,
        }
    }
}

/// A `block` dump: `getBlockId`, `getBlockMetadata`, `getSavedLightValue`
/// (Sky, Block).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockDump {
    /// `id`.
    pub id: i32,
    /// `meta`.
    pub meta: i32,
    /// `skylight`.
    pub skylight: i32,
    /// `blocklight`.
    pub blocklight: i32,
}

/// An `rng_state` dump.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RngState {
    /// 48-bit state of `World.rand`.
    pub world_rand: i64,
    /// `field_9437_g`.
    pub lcg: i32,
    /// 48-bit state of the `Math.random()` stream.
    pub math_random: i64,
}

impl World {
    /// The chunk arrays at chunk `(cx, cz)`, or `None` if it is not loaded.
    pub fn chunk_arrays(&self, cx: i32, cz: i32) -> Option<ChunkArrays<'_>> {
        self.loaded_chunk(cx, cz).map(ChunkArrays::of)
    }

    /// The `block` dump at `(x, y, z)`; `None` if `y` is outside 0..127 or the
    /// chunk is not loaded (a scenario error).
    // src: World.java:311-321, :405-420, :646-667
    pub fn block_dump(&self, x: i32, y: i32, z: i32) -> Option<BlockDump> {
        if !(0..128).contains(&y) {
            return None;
        }
        let c = self.loaded_chunk(x >> 4, z >> 4)?;
        Some(BlockDump {
            id: c.get_block_id(x & 15, y, z & 15),
            meta: c.get_block_metadata(x & 15, y, z & 15),
            skylight: self.get_saved_light_value(EnumSkyBlock::Sky, x, y, z),
            blocklight: self.get_saved_light_value(EnumSkyBlock::Block, x, y, z),
        })
    }

    /// The `rng_state` dump.
    pub fn rng_state(&self) -> RngState {
        RngState {
            world_rand: self.rand.state(),
            lcg: self.field_9437_g,
            math_random: self.math_random.state(),
        }
    }

    /// The `scheduled_ticks` digest input: per entry in `TreeSet` order,
    /// `xCoord`, `yCoord`, `zCoord`, `blockID` as i32 LE, `scheduledTime` as
    /// i64 LE (24 bytes).
    pub fn scheduled_ticks_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.scheduled_tick_tree_set.len() * 24);
        for e in self.scheduled_ticks() {
            out.extend_from_slice(&e.x_coord.to_le_bytes());
            out.extend_from_slice(&e.y_coord.to_le_bytes());
            out.extend_from_slice(&e.z_coord.to_le_bytes());
            out.extend_from_slice(&e.block_id.to_le_bytes());
            out.extend_from_slice(&e.scheduled_time.to_le_bytes());
        }
        out
    }
}

/// A field value of an `entity` line. Floats stay floats; the bit-pattern
/// encoding is `mc-golden`'s job.
#[derive(Clone, Debug, PartialEq)]
pub enum StateValue {
    /// A Java `double`.
    F64(f64),
    /// A Java `float`.
    F32(f32),
    /// A Java `boolean`.
    Bool(bool),
    /// A Java `int` or `long`.
    Int(i64),
    /// A string (the class name).
    Str(&'static str),
    /// An item stack, or `null`.
    Stack(Option<ItemStack>),
    /// Raw bytes that the trace hashes (`inventory`).
    Bytes(Vec<u8>),
    /// A nested object (`state`).
    Object(Vec<(&'static str, StateValue)>),
}

/// The fields of an `entity` line after `idx` (the list index, which the
/// caller knows), in README order: `id`, `kind`, positions, motions,
/// rotations, `onGround`, the two collision flags, `fallDistance`, `isDead`,
/// `ticksExisted`, `rand`, `state`.
///
/// `kind` is the Rust port's class name: the local player is
/// `EntityPlayerSP`, where the oracle's harness player reports
/// `HarnessPlayer`; `mc-golden` maps one to the other.
pub fn entity_state_fields(e: &Entity) -> Vec<(&'static str, StateValue)> {
    use StateValue as V;
    let b = &e.base;
    vec![
        ("id", V::Int(b.entity_id as i64)),
        ("kind", V::Str(e.kind.java_class())),
        ("posX", V::F64(b.pos_x)),
        ("posY", V::F64(b.pos_y)),
        ("posZ", V::F64(b.pos_z)),
        ("prevPosX", V::F64(b.prev_pos_x)),
        ("prevPosY", V::F64(b.prev_pos_y)),
        ("prevPosZ", V::F64(b.prev_pos_z)),
        ("motionX", V::F64(b.motion_x)),
        ("motionY", V::F64(b.motion_y)),
        ("motionZ", V::F64(b.motion_z)),
        ("rotationYaw", V::F32(b.rotation_yaw)),
        ("rotationPitch", V::F32(b.rotation_pitch)),
        ("onGround", V::Bool(b.on_ground)),
        (
            "isCollidedHorizontally",
            V::Bool(b.is_collided_horizontally),
        ),
        ("isCollidedVertically", V::Bool(b.is_collided_vertically)),
        ("fallDistance", V::F32(b.fall_distance)),
        ("isDead", V::Bool(b.is_dead)),
        ("ticksExisted", V::Int(b.ticks_existed as i64)),
        ("rand", V::Int(b.rand.state())),
        ("state", V::Object(entity_kind_state(e))),
    ]
}

/// The `state` object of an `entity` line: the kind-specific fields.
pub fn entity_kind_state(e: &Entity) -> Vec<(&'static str, StateValue)> {
    use StateValue as V;
    let mut s = Vec::new();
    if let Some(l) = e.kind.living() {
        s.push(("health", V::Int(l.health as i64)));
        s.push(("attackTime", V::Int(l.attack_time as i64)));
        s.push(("hurtTime", V::Int(l.hurt_time as i64)));
        s.push(("deathTime", V::Int(l.death_time as i64)));
        s.push(("moveForward", V::F32(l.move_forward)));
        s.push(("moveStrafing", V::F32(l.move_strafing)));
        s.push(("isJumping", V::Bool(l.is_jumping)));
    }
    if let Some(p) = e.kind.player() {
        s.push(("currentItem", V::Int(p.inventory.current_item as i64)));
        s.push(("inventory", V::Bytes(inventory_bytes(&p.inventory))));
    }
    match &e.kind {
        EntityKindData::Item(d) => {
            s.push(("item", V::Stack(d.item)));
            s.push(("age", V::Int(d.age as i64)));
            s.push((
                "delayBeforeCanPickup",
                V::Int(d.delay_before_can_pickup as i64),
            ));
            s.push(("health", V::Int(d.health as i64)));
        }
        EntityKindData::Arrow(d) => {
            s.push(("inGround", V::Bool(d.in_ground)));
            s.push(("ticksInAir", V::Int(d.ticks_in_air as i64)));
            s.push(("arrowShake", V::Int(d.arrow_shake as i64)));
        }
        _ => {}
    }
    s
}

/// `mainInventory[0..35]` then `armorInventory[0..3]`, 12 bytes each:
/// `itemID`, `stackSize`, `itemDamage` as i32 LE; empty = (-1, 0, 0).
pub fn inventory_bytes(inv: &crate::inventory::InventoryPlayer) -> Vec<u8> {
    let mut out = Vec::with_capacity(40 * 12);
    for slot in inv.main_inventory.iter().chain(inv.armor_inventory.iter()) {
        let (id, n, d) = match slot {
            Some(s) => (s.item_id, s.stack_size, s.item_damage),
            None => (-1, 0, 0),
        };
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(&d.to_le_bytes());
    }
    out
}
