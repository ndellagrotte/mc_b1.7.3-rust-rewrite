//! The concrete tile entities (`TileEntity*.java`, WP-1.8): per-class data,
//! behaviour (`updateEntity`), the NBT shapes (`readFromNBT` /
//! `writeToNBT`), the id table (`TileEntity.addMapping`,
//! `createAndLoadEntity`) and the `IInventory` methods of the three
//! inventories (chest, furnace, dispenser).
//!
//! The data lives in [`TileEntityKindData`], stored in
//! [`World::tile_entities`] under a [`TileEntityRef`] (see
//! [`crate::world::tile_entity`]). Behaviour that Java runs on `this` and that
//! reaches back into the world (`onInventoryChanged`, `updateEntity`,
//! `BlockFurnace.updateFurnaceBlockState` re-adding the same object) takes the
//! handle and borrows the data for each step, so the object stays in the
//! store, findable through the world, while its code runs, as in Java.
//!
//! | Class | Module | `updateEntity` |
//! |---|---|---|
//! | `TileEntityFurnace` | [`furnace`] (with `FurnaceRecipes`) | smelting |
//! | `TileEntityChest` | [`chest`] | base (none) |
//! | `TileEntityRecordPlayer` | [`record_player`] | base |
//! | `TileEntityDispenser` | [`dispenser`] | base |
//! | `TileEntitySign` | [`sign`] | base |
//! | `TileEntityMobSpawner` | [`mob_spawner`] | spawning (mob constructors are WP-2.3, recorded) |
//! | `TileEntityNote` | [`note`] | base |
//! | `TileEntityPiston` | [`crate::world::tile_entity`] (pistons, WP-1.6) | the move |
//!
//! NBT getters follow `NBTTagCompound` (NBTTagCompound.java:91-129): a
//! missing key reads as 0, `""` or an empty list; a present key of another
//! tag type is a `ClassCastException` ([`JavaException::ClassCast`]).

pub mod chest;
pub mod dispenser;
pub mod furnace;
pub mod mob_spawner;
pub mod note;
pub mod record_player;
pub mod sign;

use std::collections::BTreeMap;

use mc_nbt::{Tag, TagType};

use crate::entity::EntityBase;
use crate::item::{ItemStack, JavaException};
use crate::world::World;
use crate::world::tile_entity::{TileEntity, TileEntityKindData, TileEntityPiston, TileEntityRef};

/// `TileEntity.addMapping` rows in registration order: `(id, class)`.
// src: TileEntity.java:100-109
pub const MAPPINGS: [(&str, &str); 8] = [
    ("Furnace", "TileEntityFurnace"),
    ("Chest", "TileEntityChest"),
    ("RecordPlayer", "TileEntityRecordPlayer"),
    ("Trap", "TileEntityDispenser"),
    ("Sign", "TileEntitySign"),
    ("MobSpawner", "TileEntityMobSpawner"),
    ("Music", "TileEntityNote"),
    ("Piston", "TileEntityPiston"),
];

/// `classToNameMap.get(class)`: the saved id of a tile entity class.
// src: TileEntity.java:8, :15-22
pub fn class_to_name(class: &str) -> Option<&'static str> {
    MAPPINGS.iter().find(|(_, c)| *c == class).map(|(n, _)| *n)
}

/// `nameToClassMap.get(id)`: the class saved under `id`.
// src: TileEntity.java:7, :15-22
pub fn name_to_class(id: &str) -> Option<&'static str> {
    MAPPINGS.iter().find(|(n, _)| *n == id).map(|(_, c)| *c)
}

// ---- NBT getters (NBTTagCompound.java) -----------------------------------------------

/// A compound's children, or `ClassCastException` (a list element cast to
/// `NBTTagCompound`).
pub(crate) fn compound(t: &Tag) -> Result<&BTreeMap<String, Tag>, JavaException> {
    t.as_compound().ok_or(JavaException::ClassCast)
}

/// `getInteger(key)`.
// src: NBTTagCompound.java:99-101
pub(crate) fn get_integer(c: &BTreeMap<String, Tag>, key: &str) -> Result<i32, JavaException> {
    match c.get(key) {
        None => Ok(0),
        Some(Tag::Int(v)) => Ok(*v),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getShort(key)`.
// src: NBTTagCompound.java:95-97
pub(crate) fn get_short(c: &BTreeMap<String, Tag>, key: &str) -> Result<i16, JavaException> {
    match c.get(key) {
        None => Ok(0),
        Some(Tag::Short(v)) => Ok(*v),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getByte(key)`.
// src: NBTTagCompound.java:91-93
pub(crate) fn get_byte(c: &BTreeMap<String, Tag>, key: &str) -> Result<i8, JavaException> {
    match c.get(key) {
        None => Ok(0),
        Some(Tag::Byte(v)) => Ok(*v),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getString(key)`.
// src: NBTTagCompound.java:115-117
pub(crate) fn get_string(c: &BTreeMap<String, Tag>, key: &str) -> Result<String, JavaException> {
    match c.get(key) {
        None => Ok(String::new()),
        Some(Tag::String(v)) => Ok(v.clone()),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// `getTagList(key)`: the items (an absent key is a new, empty list).
// src: NBTTagCompound.java:127-129
pub(crate) fn get_tag_list<'a>(
    c: &'a BTreeMap<String, Tag>,
    key: &str,
) -> Result<&'a [Tag], JavaException> {
    match c.get(key) {
        None => Ok(&[]),
        Some(Tag::List { items, .. }) => Ok(items),
        Some(_) => Err(JavaException::ClassCast),
    }
}

/// A `new NBTTagList()` filled with `setTag`: element type compound, or
/// `TAG_Byte` while empty (the type an empty list is written with,
/// NBTTagList.java:13-18).
// src: NBTTagList.java:50-53
pub(crate) fn compound_list(items: Vec<Tag>) -> Tag {
    let elem = if items.is_empty() {
        TagType::Byte
    } else {
        TagType::Compound
    };
    Tag::List { elem, items }
}

/// The `Items` list of an inventory tile entity's `writeToNBT`: one compound
/// per non-empty slot, in slot order, `Slot` as `(byte)index` plus the
/// stack's own fields (TileEntityChest.java:64-78, TileEntityFurnace.java:
/// 67-83, TileEntityDispenser.java:84-98).
pub(crate) fn write_items(slots: &[Option<ItemStack>]) -> Tag {
    let mut var2 = Vec::new();
    for (var3, s) in slots.iter().enumerate() {
        if let Some(s) = s {
            let mut var4 = BTreeMap::new();
            var4.insert("Slot".to_string(), Tag::Byte(var3 as i8));
            s.write_to_nbt_into(&mut var4);
            var2.push(Tag::Compound(var4));
        }
    }
    compound_list(var2)
}

// ---- TileEntity base -----------------------------------------------------------------

/// `TileEntity.readFromNBT(compound)`: the coordinates.
// src: TileEntity.java:24-28
pub fn read_base_from_nbt(
    te: &mut TileEntity,
    c: &BTreeMap<String, Tag>,
) -> Result<(), JavaException> {
    te.x_coord = get_integer(c, "x")?;
    te.y_coord = get_integer(c, "y")?;
    te.z_coord = get_integer(c, "z")?;
    Ok(())
}

/// `TileEntity.writeToNBT(compound)`: `id` (the class's mapping), `x`, `y`,
/// `z`. Panics with Java's `RuntimeException` message for a class without a
/// mapping (every vanilla class has one).
// src: TileEntity.java:30-40
pub fn write_base_to_nbt(te: &TileEntity, c: &mut BTreeMap<String, Tag>) {
    let class = te.kind.java_class();
    let Some(var2) = class_to_name(class) else {
        panic!("class net.minecraft.src.{class} is missing a mapping! This is a bug!");
    };
    c.insert("id".to_string(), Tag::String(var2.to_string()));
    c.insert("x".to_string(), Tag::Int(te.x_coord));
    c.insert("y".to_string(), Tag::Int(te.y_coord));
    c.insert("z".to_string(), Tag::Int(te.z_coord));
}

/// `tileEntity.writeToNBT(new NBTTagCompound())`, dispatched on the class
/// (each subclass calls `super.writeToNBT` first). `world` only records an
/// untranscribed class's body.
pub fn write_to_nbt(te: &TileEntity, world: &World) -> Tag {
    let mut c = BTreeMap::new();
    write_base_to_nbt(te, &mut c);
    use TileEntityKindData as K;
    match &te.kind {
        K::Untranscribed(_) => world.note_unimplemented("TileEntity*.writeToNBT"),
        K::Furnace(d) => d.write_to_nbt(&mut c),
        K::Chest(d) => d.write_to_nbt(&mut c),
        K::RecordPlayer(d) => d.write_to_nbt(&mut c),
        K::Dispenser(d) => d.write_to_nbt(&mut c),
        K::Sign(d) => d.write_to_nbt(&mut c),
        K::MobSpawner(d) => d.write_to_nbt(&mut c),
        K::Note(d) => d.write_to_nbt(&mut c),
        K::Piston(d) => d.write_to_nbt(&mut c),
    }
    Tag::Compound(c)
}

/// A new tile entity of `class` (`Class.newInstance()`), `None` for a class
/// the port does not know. `world` seeds a dispenser's `Random`
/// ([`dispenser::TileEntityDispenser::new`]).
pub fn new_instance(class: &str, world: &World) -> Option<TileEntityKindData> {
    use TileEntityKindData as K;
    Some(match class {
        "TileEntityFurnace" => K::Furnace(furnace::TileEntityFurnace::default()),
        "TileEntityChest" => K::Chest(chest::TileEntityChest::default()),
        "TileEntityRecordPlayer" => {
            K::RecordPlayer(record_player::TileEntityRecordPlayer::default())
        }
        "TileEntityDispenser" => K::Dispenser(dispenser::TileEntityDispenser::new(world)),
        "TileEntitySign" => K::Sign(sign::TileEntitySign::default()),
        "TileEntityMobSpawner" => K::MobSpawner(mob_spawner::TileEntityMobSpawner::default()),
        "TileEntityNote" => K::Note(note::TileEntityNote::default()),
        // src: TileEntityPiston.java:17-18 (the no-argument constructor)
        "TileEntityPiston" => K::Piston(TileEntityPiston::new(0, 0, 0, false, false)),
        _ => return None,
    })
}

/// `TileEntity.createAndLoadEntity(compound)`: the class saved under `id`,
/// read from the compound; `Ok(None)` for an id without a mapping (Java
/// prints "Skipping TileEntity with id …"). `Err` where Java throws out of the
/// method (a key of the wrong tag type, including `id` itself: the second
/// `getString("id")` at :60 is outside the `try`).
// src: TileEntity.java:45-64
pub fn create_and_load_entity(
    nbt: &Tag,
    world: &World,
) -> Result<Option<TileEntity>, JavaException> {
    let c = compound(nbt)?;
    let id = get_string(c, "id")?;
    let Some(kind) = name_to_class(&id).and_then(|class| new_instance(class, world)) else {
        // src: TileEntity.java:60 (stdout only)
        return Ok(None);
    };
    let mut te = TileEntity::new(kind);
    read_from_nbt(&mut te, c, world)?;
    Ok(Some(te))
}

/// `tileEntity.readFromNBT(compound)`, dispatched on the class.
pub fn read_from_nbt(
    te: &mut TileEntity,
    c: &BTreeMap<String, Tag>,
    world: &World,
) -> Result<(), JavaException> {
    use TileEntityKindData as K;
    if let K::Sign(d) = &mut te.kind {
        // src: TileEntitySign.java:16-17: the flag is cleared before super.readFromNBT.
        d.field_25062_c = false;
    }
    read_base_from_nbt(te, c)?;
    match &mut te.kind {
        K::Untranscribed(_) => {
            world.note_unimplemented("TileEntity*.readFromNBT");
            Ok(())
        }
        K::Furnace(d) => d.read_from_nbt(c),
        K::Chest(d) => d.read_from_nbt(c),
        K::RecordPlayer(d) => d.read_from_nbt(c),
        K::Dispenser(d) => d.read_from_nbt(c),
        K::Sign(d) => d.read_from_nbt(c),
        K::MobSpawner(d) => d.read_from_nbt(c),
        K::Note(d) => d.read_from_nbt(c),
        // src: TileEntityPiston.java:128-135
        K::Piston(d) => {
            d.read_from_nbt(c);
            Ok(())
        }
    }
}

/// `tileEntity.updateEntity()` for the tile entity `r` (called by
/// `World.updateEntities` for every valid loaded tile entity,
/// World.java:1237-1242). Only the furnace, the mob spawner and the moving
/// piston override the empty base (TileEntity.java:42-43).
pub fn update_entity(world: &mut World, r: TileEntityRef) {
    let Some(t) = world.tile_entities.get(&r) else {
        return;
    };
    use TileEntityKindData as K;
    match t.kind {
        K::Untranscribed(_) => world.note_unimplemented("TileEntity*.updateEntity"),
        // src: TileEntityFurnace.java:105-147
        K::Furnace(_) => furnace::update_entity(world, r),
        // src: TileEntityMobSpawner.java:25-87
        K::MobSpawner(_) => mob_spawner::update_entity(world, r),
        // src: TileEntityPiston.java:105-126
        K::Piston(_) => crate::world::tile_entity::piston_update_entity(world, r),
        // src: TileEntity.java:42-43 (not overridden)
        K::Chest(_) | K::RecordPlayer(_) | K::Dispenser(_) | K::Sign(_) | K::Note(_) => {}
    }
}

/// `onInventoryChanged()` (`TileEntity`'s body, not overridden): tells the
/// world the tile entity changed. The `worldObj != null` test always holds
/// for a stored tile entity.
// src: TileEntity.java:70-75
pub fn on_inventory_changed(world: &mut World, r: TileEntityRef) {
    if let Some(t) = world.tile_entities.get(&r) {
        let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
        world.func_698_b(x, y, z);
    }
}

/// `getBlockMetadata()`.
// src: TileEntity.java:66-68
pub fn get_block_metadata(world: &mut World, r: TileEntityRef) -> i32 {
    match world.tile_entities.get(&r) {
        Some(t) => {
            let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
            world.get_block_metadata(x, y, z)
        }
        None => 0,
    }
}

/// `getBlockType()`: `Block.blocksList[getBlockId(x, y, z)]` (null for air
/// or an unregistered id). Its only callers are client code
/// (GuiEditSign.java:75, TileEntitySignRenderer.java:9, Phase 3).
// src: TileEntity.java:84-86
pub fn get_block_type(
    world: &mut World,
    r: TileEntityRef,
) -> Option<&'static crate::block::BlockDef> {
    let t = world.tile_entities.get(&r)?;
    let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
    crate::block::block(world.get_block_id(x, y, z))
}

// ---- IInventory (IInventory.java) ----------------------------------------------------

/// The slots of an `IInventory` tile entity (`None` for the classes that are
/// not inventories).
fn slots(kind: &TileEntityKindData) -> Option<&[Option<ItemStack>]> {
    match kind {
        TileEntityKindData::Chest(d) => Some(&d.chest_contents),
        TileEntityKindData::Furnace(d) => Some(&d.furnace_item_stacks),
        TileEntityKindData::Dispenser(d) => Some(&d.dispenser_contents),
        _ => None,
    }
}

fn slots_mut(kind: &mut TileEntityKindData) -> Option<&mut [Option<ItemStack>]> {
    match kind {
        TileEntityKindData::Chest(d) => Some(&mut d.chest_contents),
        TileEntityKindData::Furnace(d) => Some(&mut d.furnace_item_stacks),
        TileEntityKindData::Dispenser(d) => Some(&mut d.dispenser_contents),
        _ => None,
    }
}

/// `getSizeInventory()`: 27 for a chest (whatever its array holds), the
/// array length (3) for a furnace, 9 for a dispenser; `None` for a class
/// that is not an `IInventory`.
// src: TileEntityChest.java:6-8, TileEntityFurnace.java:9-11, TileEntityDispenser.java:9-11
pub fn get_size_inventory(kind: &TileEntityKindData) -> Option<i32> {
    match kind {
        TileEntityKindData::Chest(_) => Some(chest::SIZE_INVENTORY),
        TileEntityKindData::Furnace(d) => Some(d.furnace_item_stacks.len() as i32),
        TileEntityKindData::Dispenser(_) => Some(dispenser::SIZE_INVENTORY),
        _ => None,
    }
}

/// Java's array access `contents[slot]`; panics where Java throws
/// `ArrayIndexOutOfBoundsException`.
fn index(len: usize, slot: i32) -> usize {
    match usize::try_from(slot) {
        Ok(i) if i < len => i,
        _ => panic!("ArrayIndexOutOfBoundsException: {slot} (length {len})"),
    }
}

/// `getStackInSlot(slot)` (a copy: vanilla returns the shared object; the
/// callers that mutate it write the slot directly).
// src: TileEntityChest.java:10-12, TileEntityFurnace.java:13-15, TileEntityDispenser.java:13-15
pub fn get_stack_in_slot(kind: &TileEntityKindData, slot: i32) -> Option<ItemStack> {
    let s = slots(kind)?;
    s[index(s.len(), slot)]
}

/// `decrStackSize(slot, n)`: takes up to `n` items out of the slot. The
/// chest and the dispenser call `onInventoryChanged`, the furnace does not.
// src: TileEntityChest.java:14-34, TileEntityFurnace.java:17-35, TileEntityDispenser.java:17-37
pub fn decr_stack_size(
    world: &mut World,
    r: TileEntityRef,
    slot: i32,
    n: i32,
) -> Option<ItemStack> {
    let (out, notify) = {
        let t = world.tile_entities.get_mut(&r)?;
        let notify = !matches!(t.kind, TileEntityKindData::Furnace(_));
        let s = slots_mut(&mut t.kind)?;
        let i = index(s.len(), slot);
        let cur = s[i].as_mut()?;
        if cur.stack_size <= n {
            let var3 = *cur;
            s[i] = None;
            (var3, notify)
        } else {
            let var3 = cur.split_stack(n);
            if cur.stack_size == 0 {
                s[i] = None;
            }
            (var3, notify)
        }
    };
    if notify {
        on_inventory_changed(world, r);
    }
    Some(out)
}

/// `setInventorySlotContents(slot, stack)`: a stack larger than the limit
/// (64) is cut down; the chest and the dispenser call `onInventoryChanged`.
// src: TileEntityChest.java:36-43, TileEntityFurnace.java:37-43, TileEntityDispenser.java:56-63
pub fn set_inventory_slot_contents(
    world: &mut World,
    r: TileEntityRef,
    slot: i32,
    stack: Option<ItemStack>,
) {
    let notify = {
        let Some(t) = world.tile_entities.get_mut(&r) else {
            return;
        };
        let notify = !matches!(t.kind, TileEntityKindData::Furnace(_));
        let Some(s) = slots_mut(&mut t.kind) else {
            return;
        };
        let i = index(s.len(), slot);
        let mut stack = stack;
        if let Some(st) = stack.as_mut()
            && st.stack_size > INVENTORY_STACK_LIMIT
        {
            st.stack_size = INVENTORY_STACK_LIMIT;
        }
        s[i] = stack;
        notify
    };
    if notify {
        on_inventory_changed(world, r);
    }
}

/// `getInvName()`.
// src: TileEntityChest.java:45-47, TileEntityFurnace.java:45-47, TileEntityDispenser.java:65-67
pub fn get_inv_name(kind: &TileEntityKindData) -> Option<&'static str> {
    match kind {
        TileEntityKindData::Chest(_) => Some("Chest"),
        TileEntityKindData::Furnace(_) => Some("Furnace"),
        TileEntityKindData::Dispenser(_) => Some("Trap"),
        _ => None,
    }
}

/// `getInventoryStackLimit()`: 64 for all three.
// src: TileEntityChest.java:80-82, TileEntityFurnace.java:85-87, TileEntityDispenser.java:100-102
pub const INVENTORY_STACK_LIMIT: i32 = 64;

/// `canInteractWith(player)`: the tile entity is still the one at its
/// position and the player is within 8 blocks of its centre.
// src: TileEntityChest.java:84-90, TileEntityFurnace.java:204-210, TileEntityDispenser.java:104-110
pub fn can_interact_with(world: &mut World, r: TileEntityRef, player: &EntityBase) -> bool {
    let Some(t) = world.tile_entities.get(&r) else {
        return false;
    };
    let (x, y, z) = (t.x_coord, t.y_coord, t.z_coord);
    if world.get_block_tile_entity(x, y, z) != Some(r) {
        false
    } else {
        player.get_distance_sq(x as f64 + 0.5, y as f64 + 0.5, z as f64 + 0.5) <= 64.0
    }
}
