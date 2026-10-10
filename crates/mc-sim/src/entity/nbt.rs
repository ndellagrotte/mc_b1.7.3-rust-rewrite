//! Entity NBT (WP-1.9b): `Entity.addEntityID` / `writeToNBT` /
//! `readFromNBT` (Entity.java:780-833) and the `writeEntityToNBT` /
//! `readEntityFromNBT` bodies of the classes the port has:
//!
//! | Class | Write | Read |
//! |---|---|---|
//! | `EntityItem` | EntityItem.java:97-101 | EntityItem.java:103-108 |
//! | `EntityFallingSand` | EntityFallingSand.java:73-75 | EntityFallingSand.java:77-79 |
//! | `EntityTNTPrimed` | EntityTNTPrimed.java:71-73 | EntityTNTPrimed.java:75-77 |
//! | `EntityArrow` | EntityArrow.java:235-244 | EntityArrow.java:246-255 |
//! | `EntitySnowball` / `EntityEgg` | EntitySnowball.java:204-211, EntityEgg.java:217-224 | EntitySnowball.java:213-220, EntityEgg.java:226-233 |
//! | `EntityFireball` | EntityFireball.java:176-183 | EntityFireball.java:185-192 |
//! | `EntityFish` | EntityFish.java:324-331 | EntityFish.java:333-340 |
//! | `EntityPainting` | EntityPainting.java:214-220 | EntityPainting.java:222-243 |
//! | `EntityMinecart` | EntityMinecart.java:598-619 | EntityMinecart.java:621-640 |
//! | `EntityBoat` | EntityBoat.java:326-327 (empty) | EntityBoat.java:329-330 (empty) |
//! | `EntityLightningBolt` | EntityLightningBolt.java:82-83 (empty) | EntityLightningBolt.java:79-80 (empty) |
//! | `EntityLiving` | EntityLiving.java:557-562 | EntityLiving.java:564-573 |
//! | `EntityAnimal` (`super` only) | EntityAnimal.java:12-14 | EntityAnimal.java:16-18 |
//! | `EntityPig` | EntityPig.java:14-17 | EntityPig.java:19-22 |
//! | `EntitySheep` | EntitySheep.java:55-59 | EntitySheep.java:61-65 |
//! | `EntityCow` (`super` only) | EntityCow.java:10-12 | EntityCow.java:14-16 |
//! | `EntityChicken` (`super` only) | EntityChicken.java:54-56 | EntityChicken.java:58-60 |
//! | `EntityWolf` | EntityWolf.java:42-52 | EntityWolf.java:54-64 |
//! | `EntityMob`, `EntitySkeleton`, `EntitySpider` (`super` only); `EntityZombie`, `EntityGiantZombie`, `EntityFlying`, `EntityGhast` (none) | EntityMob.java:61-63, EntitySkeleton.java:55-57, EntitySpider.java:62-64 | EntityMob.java:65-67, EntitySkeleton.java:59-61, EntitySpider.java:66-68 |
//! | `EntityCreeper` | EntityCreeper.java:18-24 | EntityCreeper.java:26-29 |
//! | `EntityPigZombie` | EntityPigZombie.java:31-34 | EntityPigZombie.java:36-39 |
//! | `EntitySlime` | EntitySlime.java:33-36 | EntitySlime.java:38-41 |
//! | `EntityWaterMob`, `EntitySquid` (`super` only) | EntityWaterMob.java:12-14, EntitySquid.java:26-28 | EntityWaterMob.java:16-18, EntitySquid.java:30-32 |
//! | `EntityPlayer` (+ `InventoryPlayer`) | EntityPlayer.java:326-337, InventoryPlayer.java:199-221 | EntityPlayer.java:308-324, InventoryPlayer.java:223-241 |
//! | `EntityPlayerSP` | EntityPlayerSP.java:98-101 | EntityPlayerSP.java:103-106 |
//!
//! Every entity class of the port has its level here (the squid, WP-2.3c,
//! was the last). The read side's `match` is exhaustive: the kinds read and
//! returned before it (painting, pig, sheep, wolf, creeper, slime, player)
//! have an empty arm (WP-4.8 removed the recorded fallback, Gate 2 R4).
//!
//! Compound children are in `BTreeMap` order (vanilla's `HashMap` order only
//! shows in bytes, never compared, PLAN_v4 §4.3).

use mc_core::java_hash::ChunkCoordinates;
use mc_nbt::{Tag, TagType};

use super::{Entity, EntityKindData, LivingData, PlayerData};
use crate::inventory::InventoryPlayer;
use crate::item::ItemStack;
use crate::save::SaveError;
use crate::save::entity_list::get_entity_string;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;

/// `addEntityID(compound)`: `id` (the class's `EntityList` name) and
/// `writeToNBT`, unless the entity is dead or its class has no name; `None`
/// then (Java returns false and the chunk does not list it).
// src: Entity.java:780-789
pub fn add_entity_id(e: &Entity, world: &World) -> Option<Compound> {
    let var2 = get_entity_string(&e.kind);
    match var2 {
        Some(name) if !e.base.is_dead => {
            let mut c = Compound::new();
            n::set(&mut c, "id", Tag::String(name.to_string()));
            write_to_nbt_into(e, world, &mut c);
            Some(c)
        }
        _ => None,
    }
}

/// `writeToNBT(new NBTTagCompound())`.
// src: Entity.java:791-800
pub fn write_to_nbt(e: &Entity, world: &World) -> Compound {
    let mut c = Compound::new();
    write_to_nbt_into(e, world, &mut c);
    c
}

/// `newDoubleNBTList(values)`.
// src: Entity.java:843-854
fn new_double_nbt_list(v: &[f64]) -> Tag {
    n::list_of(TagType::Double, v.iter().map(|d| Tag::Double(*d)).collect())
}

/// `newFloatNBTList(values)`.
// src: Entity.java:856-867
fn new_float_nbt_list(v: &[f32]) -> Tag {
    n::list_of(TagType::Float, v.iter().map(|f| Tag::Float(*f)).collect())
}

/// `writeToNBT(compound)`: the `Entity` fields, then `writeEntityToNBT` of
/// the class.
// src: Entity.java:791-800
pub fn write_to_nbt_into(e: &Entity, world: &World, c: &mut Compound) {
    let b = &e.base;
    n::set(
        c,
        "Pos",
        new_double_nbt_list(&[b.pos_x, b.pos_y + b.y_size as f64, b.pos_z]),
    );
    n::set(
        c,
        "Motion",
        new_double_nbt_list(&[b.motion_x, b.motion_y, b.motion_z]),
    );
    n::set(
        c,
        "Rotation",
        new_float_nbt_list(&[b.rotation_yaw, b.rotation_pitch]),
    );
    n::set(c, "FallDistance", Tag::Float(b.fall_distance));
    n::set(c, "Fire", Tag::Short(b.fire as i16));
    n::set(c, "Air", Tag::Short(b.air as i16));
    n::set_boolean(c, "OnGround", b.on_ground);
    write_entity_to_nbt(e, world, c);
}

/// `writeEntityToNBT(compound)`, dispatched on the class.
fn write_entity_to_nbt(e: &Entity, world: &World, c: &mut Compound) {
    use EntityKindData as K;
    match &e.kind {
        K::Item(d) => {
            // src: EntityItem.java:97-101 (the short of the byte cast of health)
            n::set(c, "Health", Tag::Short(d.health as i8 as i16));
            n::set(c, "Age", Tag::Short(d.age as i16));
            match d.item {
                Some(s) => n::set(c, "Item", s.write_to_nbt()),
                // `this.item.writeToNBT` on a null item: only an
                // `EntityItem(World)` not yet read from NBT has none.
                None => world.note_unimplemented(
                    "EntityItem.writeEntityToNBT with a null item (Java NullPointerException)",
                ),
            }
        }
        K::FallingSand(d) => super::falling_sand::write_entity_to_nbt(d, c),
        K::TntPrimed(d) => super::tnt_primed::write_entity_to_nbt(d, c),
        K::Arrow(d) => super::arrow::write_entity_to_nbt(d, c),
        K::Snowball(d) | K::Egg(d) => super::snowball::write_entity_to_nbt(d, c),
        K::Fireball(d) => super::fireball::write_entity_to_nbt(d, c),
        K::Fish(d) => super::fish::write_entity_to_nbt(d, c),
        K::Painting(d) => super::painting::write_entity_to_nbt(d, c, world),
        K::Minecart(d) => super::minecart::write_entity_to_nbt(d, c),
        // src: EntityBoat.java:326-327 (empty)
        K::Boat(..) => {}
        // src: EntityLightningBolt.java:82-83 (empty)
        K::LightningBolt(..) => {}
        K::Pig(l, ..) => {
            write_living(l, c);
            // src: EntityPig.java:14-17 (EntityAnimal adds nothing, :12-14)
            n::set_boolean(c, "Saddle", super::pig::get_saddled(e));
        }
        K::Sheep(l, ..) => {
            write_living(l, c);
            // src: EntitySheep.java:55-59 (EntityAnimal adds nothing, :12-14)
            n::set_boolean(c, "Sheared", super::sheep::get_sheared(e));
            n::set(
                c,
                "Color",
                Tag::Byte(super::sheep::get_fleece_color(e) as i8),
            );
        }
        // src: EntityCow.java:10-12, EntityChicken.java:54-56 (super only)
        K::Cow(l, ..) | K::Chicken(l, ..) => write_living(l, c),
        K::Wolf(l, ..) => {
            write_living(l, c);
            // src: EntityWolf.java:42-52 (getWolfOwner() is never null: the
            // watcher string starts as "" and setWolfOwner(null) throws)
            n::set_boolean(c, "Angry", super::wolf::is_wolf_angry(e));
            n::set_boolean(c, "Sitting", super::wolf::is_wolf_sitting(e));
            n::set(
                c,
                "Owner",
                Tag::String(super::wolf::get_wolf_owner(e).to_string()),
            );
        }
        // src: EntityMob.java:61-63, EntitySkeleton.java:55-57,
        // EntitySpider.java:62-64 (`super` only); EntityZombie,
        // EntityGiantZombie, EntityFlying and EntityGhast do not override it.
        K::Mob(l, ..)
        | K::Zombie(l, ..)
        | K::GiantZombie(l, ..)
        | K::Skeleton(l, ..)
        | K::Spider(l, ..)
        | K::Ghast(l, ..) => write_living(l, c),
        K::Creeper(l, ..) => {
            write_living(l, c);
            // src: EntityCreeper.java:18-24
            if super::creeper::get_powered(e) {
                n::set_boolean(c, "powered", true);
            }
        }
        K::PigZombie(l, .., d) => {
            write_living(l, c);
            // src: EntityPigZombie.java:31-34
            n::set(c, "Anger", Tag::Short(d.anger_level as i16));
        }
        K::Slime(l, _) => {
            write_living(l, c);
            // src: EntitySlime.java:33-36
            n::set(c, "Size", Tag::Int(super::slime::get_slime_size(e) - 1));
        }
        // src: EntitySquid.java:26-28, EntityWaterMob.java:12-14 (`super`
        // only; EntityCreature has no override)
        K::Squid(l, ..) => write_living(l, c),
        K::PlayerSp(l, p, _) => {
            write_living(l, c);
            write_player(p, c);
            // src: EntityPlayerSP.java:98-101
            n::set(c, "Score", Tag::Int(p.score));
        }
    }
}

/// `EntityLiving.writeEntityToNBT`.
// src: EntityLiving.java:557-562
fn write_living(l: &LivingData, c: &mut Compound) {
    n::set(c, "Health", Tag::Short(l.health as i16));
    n::set(c, "HurtTime", Tag::Short(l.hurt_time as i16));
    n::set(c, "DeathTime", Tag::Short(l.death_time as i16));
    n::set(c, "AttackTime", Tag::Short(l.attack_time as i16));
}

/// `EntityPlayer.writeEntityToNBT` (after `super`).
// src: EntityPlayer.java:326-337
fn write_player(p: &PlayerData, c: &mut Compound) {
    n::set(c, "Inventory", inventory_write_to_nbt(&p.inventory));
    n::set(c, "Dimension", Tag::Int(p.dimension));
    n::set_boolean(c, "Sleeping", p.sleeping);
    n::set(c, "SleepTimer", Tag::Short(p.sleep_timer as i16));
    if let Some(s) = p.player_spawn_coordinate {
        n::set(c, "SpawnX", Tag::Int(s.pos_x));
        n::set(c, "SpawnY", Tag::Int(s.pos_y));
        n::set(c, "SpawnZ", Tag::Int(s.pos_z));
    }
}

/// `InventoryPlayer.writeToNBT(new NBTTagList())`: the non-empty main slots
/// (`Slot` 0–35), then the armour slots (`Slot` 100–103).
// src: InventoryPlayer.java:199-221
pub fn inventory_write_to_nbt(inv: &InventoryPlayer) -> Tag {
    let mut var1 = Vec::new();
    for (var2, s) in inv.main_inventory.iter().enumerate() {
        if let Some(s) = s {
            let mut var3 = Compound::new();
            n::set(&mut var3, "Slot", Tag::Byte(var2 as i8));
            s.write_to_nbt_into(&mut var3);
            var1.push(Tag::Compound(var3));
        }
    }
    for (var2, s) in inv.armor_inventory.iter().enumerate() {
        if let Some(s) = s {
            let mut var3 = Compound::new();
            n::set(&mut var3, "Slot", Tag::Byte((var2 + 100) as i8));
            s.write_to_nbt_into(&mut var3);
            var1.push(Tag::Compound(var3));
        }
    }
    n::list_of(TagType::Compound, var1)
}

/// `InventoryPlayer.readFromNBT(list)`: fresh arrays, then each compound's
/// stack into `Slot & 255` (0–35 main, 100–103 armour) if its item exists.
// src: InventoryPlayer.java:223-241
pub fn inventory_read_from_nbt(inv: &mut InventoryPlayer, list: &[Tag]) -> Result<(), SaveError> {
    inv.main_inventory = [None; 36];
    inv.armor_inventory = [None; 4];
    for t in list {
        let var3 = n::as_compound(t)?;
        let var4 = n::get_byte(var3, "Slot")? as i32 & 255;
        let var5 = ItemStack::from_nbt(t)?;
        if var5.get_item().is_some() {
            if (0..36).contains(&var4) {
                inv.main_inventory[var4 as usize] = Some(var5);
            }
            if (100..104).contains(&var4) {
                inv.armor_inventory[(var4 - 100) as usize] = Some(var5);
            }
        }
    }
    Ok(())
}

/// `readFromNBT(compound)`: the `Entity` fields (motion over 10 per axis is
/// zeroed; `setPosition` and `setRotation` apply, so the yaw and pitch come
/// back `% 360`), then `readEntityFromNBT` of the class. `Err` where Java
/// throws: a list too short (`IndexOutOfBoundsException`) or a tag of the
/// wrong type (`ClassCastException`).
// src: Entity.java:802-833
pub fn read_from_nbt(e: &mut Entity, c: &Compound, world: &mut World) -> Result<(), SaveError> {
    let var2 = n::get_tag_list(c, "Pos")?;
    let var3 = n::get_tag_list(c, "Motion")?;
    let var4 = n::get_tag_list(c, "Rotation")?;
    let b = &mut e.base;
    b.motion_x = as_double(n::tag_at(var3, 0, "Motion")?)?;
    b.motion_y = as_double(n::tag_at(var3, 1, "Motion")?)?;
    b.motion_z = as_double(n::tag_at(var3, 2, "Motion")?)?;
    // src: Entity.java:809-819 (Math.abs)
    if b.motion_x.abs() > 10.0 {
        b.motion_x = 0.0;
    }
    if b.motion_y.abs() > 10.0 {
        b.motion_y = 0.0;
    }
    if b.motion_z.abs() > 10.0 {
        b.motion_z = 0.0;
    }
    let x = as_double(n::tag_at(var2, 0, "Pos")?)?;
    b.prev_pos_x = x;
    b.last_tick_pos_x = x;
    b.pos_x = x;
    let y = as_double(n::tag_at(var2, 1, "Pos")?)?;
    b.prev_pos_y = y;
    b.last_tick_pos_y = y;
    b.pos_y = y;
    let z = as_double(n::tag_at(var2, 2, "Pos")?)?;
    b.prev_pos_z = z;
    b.last_tick_pos_z = z;
    b.pos_z = z;
    let yaw = as_float(n::tag_at(var4, 0, "Rotation")?)?;
    b.prev_rotation_yaw = yaw;
    b.rotation_yaw = yaw;
    let pitch = as_float(n::tag_at(var4, 1, "Rotation")?)?;
    b.prev_rotation_pitch = pitch;
    b.rotation_pitch = pitch;
    b.fall_distance = n::get_float(c, "FallDistance")?;
    b.fire = n::get_short(c, "Fire")? as i32;
    b.air = n::get_short(c, "Air")? as i32;
    b.on_ground = n::get_boolean(c, "OnGround")?;
    b.set_position(b.pos_x, b.pos_y, b.pos_z);
    b.set_rotation(b.rotation_yaw, b.rotation_pitch);
    read_entity_from_nbt(e, c, world)
}

/// `((NBTTagDouble) tag).doubleValue`.
fn as_double(t: &Tag) -> Result<f64, SaveError> {
    match t {
        Tag::Double(v) => Ok(*v),
        _ => Err(crate::item::JavaException::ClassCast.into()),
    }
}

/// `((NBTTagFloat) tag).floatValue`.
fn as_float(t: &Tag) -> Result<f32, SaveError> {
    match t {
        Tag::Float(v) => Ok(*v),
        _ => Err(crate::item::JavaException::ClassCast.into()),
    }
}

/// `readEntityFromNBT(compound)`, dispatched on the class.
fn read_entity_from_nbt(e: &mut Entity, c: &Compound, world: &mut World) -> Result<(), SaveError> {
    use EntityKindData as K;
    if let K::Painting(..) = e.kind {
        // src: EntityPainting.java:222-243 (it calls setDirection on itself)
        return super::painting::read_entity_from_nbt(e, c, world);
    }
    if let K::Pig(l, ..) = &mut e.kind {
        read_living(l, c)?;
        // src: EntityPig.java:19-22 (EntityAnimal adds nothing, :16-18)
        let saddle = n::get_boolean(c, "Saddle")?;
        super::pig::set_saddled(e, saddle);
        return Ok(());
    }
    if let K::Sheep(l, ..) = &mut e.kind {
        read_living(l, c)?;
        // src: EntitySheep.java:61-65
        let sheared = n::get_boolean(c, "Sheared")?;
        super::sheep::set_sheared(e, sheared);
        let color = n::get_byte(c, "Color")?;
        super::sheep::set_fleece_color(e, color as i32);
        return Ok(());
    }
    if let K::Wolf(l, ..) = &mut e.kind {
        read_living(l, c)?;
        // src: EntityWolf.java:54-64
        let angry = n::get_boolean(c, "Angry")?;
        super::wolf::set_wolf_angry(e, angry);
        let sitting = n::get_boolean(c, "Sitting")?;
        super::wolf::set_wolf_sitting(e, sitting);
        let var2 = n::get_string(c, "Owner")?;
        if !var2.is_empty() {
            super::wolf::set_wolf_owner(e, Some(&var2));
            super::wolf::set_wolf_tamed(e, true);
        }
        return Ok(());
    }
    if let K::Creeper(l, ..) = &mut e.kind {
        read_living(l, c)?;
        // src: EntityCreeper.java:26-29
        let powered = n::get_boolean(c, "powered")?;
        e.base
            .data_watcher
            .update_object(17, super::data_watcher::WatchValue::Byte(i8::from(powered)));
        return Ok(());
    }
    if let K::Slime(l, _) = &mut e.kind {
        read_living(l, c)?;
        // src: EntitySlime.java:38-41
        // vanilla bug: setSlimeSize resets health to size², so a hurt slime
        // reloads healed.
        let size = n::get_integer(c, "Size")?.wrapping_add(1);
        super::slime::set_slime_size(e, size);
        return Ok(());
    }
    if let K::PlayerSp(l, p, _) = &mut e.kind {
        read_living(l, c)?;
        read_player(p, c)?;
        // src: EntityPlayer.java:315-318 (a player saved asleep is woken)
        super::player::read_entity_from_nbt_tail(e, world);
        let Some(p) = e.kind.player_mut() else {
            unreachable!("the PlayerSp arm above")
        };
        read_player_spawn(p, c)?;
        // src: EntityPlayerSP.java:103-106
        p.score = n::get_integer(c, "Score")?;
        return Ok(());
    }
    match &mut e.kind {
        K::Item(d) => {
            // src: EntityItem.java:103-108
            d.health = n::get_short(c, "Health")? as i32 & 255;
            d.age = n::get_short(c, "Age")? as i32;
            let var2 = n::get_compound_tag(c, "Item")?;
            d.item = Some(ItemStack::from_nbt(&Tag::Compound(var2))?);
        }
        K::FallingSand(d) => super::falling_sand::read_entity_from_nbt(d, c),
        K::TntPrimed(d) => super::tnt_primed::read_entity_from_nbt(d, c)?,
        K::Arrow(d) => super::arrow::read_entity_from_nbt(d, c)?,
        K::Snowball(d) | K::Egg(d) => super::snowball::read_entity_from_nbt(d, c)?,
        K::Fireball(d) => super::fireball::read_entity_from_nbt(d, c)?,
        K::Fish(d) => super::fish::read_entity_from_nbt(d, c)?,
        K::Minecart(d) => super::minecart::read_entity_from_nbt(d, c)?,
        // src: EntityBoat.java:329-330 (empty)
        K::Boat(..) => {}
        // src: EntityLightningBolt.java:79-80 (empty)
        K::LightningBolt(..) => {}
        // src: EntityCow.java:14-16, EntityChicken.java:58-60 (super only)
        K::Cow(l, ..) | K::Chicken(l, ..) => read_living(l, c)?,
        // src: EntityMob.java:65-67, EntitySkeleton.java:59-61,
        // EntitySpider.java:66-68 (`super` only)
        K::Mob(l, ..)
        | K::Zombie(l, ..)
        | K::GiantZombie(l, ..)
        | K::Skeleton(l, ..)
        | K::Spider(l, ..)
        | K::Ghast(l, ..) => read_living(l, c)?,
        K::PigZombie(l, .., d) => {
            read_living(l, c)?;
            // src: EntityPigZombie.java:36-39
            d.anger_level = n::get_short(c, "Anger")? as i32;
        }
        // src: EntitySquid.java:30-32, EntityWaterMob.java:16-18 (`super`
        // only)
        K::Squid(l, ..) => read_living(l, c)?,
        // Read and returned above.
        K::Painting(..)
        | K::Pig(..)
        | K::Sheep(..)
        | K::Wolf(..)
        | K::Creeper(..)
        | K::Slime(..)
        | K::PlayerSp(..) => {}
    }
    Ok(())
}

/// `EntityLiving.readEntityFromNBT`: `Health` defaults to 10 when absent.
// src: EntityLiving.java:564-573
fn read_living(l: &mut LivingData, c: &Compound) -> Result<(), SaveError> {
    l.health = n::get_short(c, "Health")? as i32;
    if !n::has_key(c, "Health") {
        l.health = 10;
    }
    l.hurt_time = n::get_short(c, "HurtTime")? as i32;
    l.death_time = n::get_short(c, "DeathTime")? as i32;
    l.attack_time = n::get_short(c, "AttackTime")? as i32;
    Ok(())
}

/// `EntityPlayer.readEntityFromNBT` (after `super`), up to the sleep
/// state; a player saved asleep is then woken by
/// [`super::player::read_entity_from_nbt_tail`] (EntityPlayer.java:315-318),
/// and [`read_player_spawn`] reads the rest.
// src: EntityPlayer.java:308-314
fn read_player(p: &mut PlayerData, c: &Compound) -> Result<(), SaveError> {
    let var2 = n::get_tag_list(c, "Inventory")?;
    inventory_read_from_nbt(&mut p.inventory, var2)?;
    p.dimension = n::get_integer(c, "Dimension")?;
    p.sleeping = n::get_boolean(c, "Sleeping")?;
    p.sleep_timer = n::get_short(c, "SleepTimer")? as i32;
    Ok(())
}

/// The end of `EntityPlayer.readEntityFromNBT`: the bed spawn, when all
/// three keys are present.
// src: EntityPlayer.java:320-322
fn read_player_spawn(p: &mut PlayerData, c: &Compound) -> Result<(), SaveError> {
    if n::has_key(c, "SpawnX") && n::has_key(c, "SpawnY") && n::has_key(c, "SpawnZ") {
        p.player_spawn_coordinate = Some(ChunkCoordinates::new(
            n::get_integer(c, "SpawnX")?,
            n::get_integer(c, "SpawnY")?,
            n::get_integer(c, "SpawnZ")?,
        ));
    }
    Ok(())
}
