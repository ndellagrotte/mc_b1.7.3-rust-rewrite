//! Virtual dispatch for `Entity` methods (module docs of [`crate::entity`]).
//!
//! One function per overridable method that transcribed code calls. Each
//! `match` lists every concrete kind under the class whose body it runs
//! (computed from the vanilla tree, inheritance resolved). Arms for overrides
//! that are not transcribed yet run the nearest transcribed ancestor's body and
//! record `"Class.method"` in [`World::unimplemented_hits`].

use mc_core::Aabb;

use super::{Entity, EntityKindData as K, base, item, living};
use crate::world::World;

/// `onUpdate()`.
pub fn on_update(e: &mut Entity, world: &mut World) {
    match e.kind {
        // src: EntityItem.java:36-77
        K::Item(..) => item::on_update(e, world),
        // EntityLiving.onUpdate (EntityLiving.java:203-290) for the kinds that
        // do not override it further.
        K::Pig(..) | K::Sheep(..) | K::Cow(..) | K::Chicken(..) | K::Squid(..) => {
            living::on_update(e, world)
        }
        K::Mob(..) | K::Skeleton(..) | K::Spider(..) | K::Zombie(..) | K::GiantZombie(..) => {
            world.note_unimplemented("EntityMob.onUpdate");
            living::on_update(e, world);
        }
        K::Creeper(..) => {
            world.note_unimplemented("EntityCreeper.onUpdate");
            living::on_update(e, world);
        }
        K::PigZombie(..) => {
            world.note_unimplemented("EntityPigZombie.onUpdate");
            living::on_update(e, world);
        }
        K::Slime(..) => {
            world.note_unimplemented("EntitySlime.onUpdate");
            living::on_update(e, world);
        }
        K::Ghast(..) => {
            world.note_unimplemented("EntityGhast.onUpdate");
            living::on_update(e, world);
        }
        K::Wolf(..) => {
            world.note_unimplemented("EntityWolf.onUpdate");
            living::on_update(e, world);
        }
        K::PlayerSp(..) => {
            world.note_unimplemented("EntityPlayer.onUpdate");
            living::on_update(e, world);
        }
        K::Arrow(..) => untranscribed_base_update(e, world, "EntityArrow.onUpdate"),
        K::Snowball(..) => untranscribed_base_update(e, world, "EntitySnowball.onUpdate"),
        K::Egg(..) => untranscribed_base_update(e, world, "EntityEgg.onUpdate"),
        K::Fireball(..) => untranscribed_base_update(e, world, "EntityFireball.onUpdate"),
        K::Fish(..) => untranscribed_base_update(e, world, "EntityFish.onUpdate"),
        K::Painting(..) => untranscribed_base_update(e, world, "EntityPainting.onUpdate"),
        K::FallingSand(..) => untranscribed_base_update(e, world, "EntityFallingSand.onUpdate"),
        K::TntPrimed(..) => untranscribed_base_update(e, world, "EntityTNTPrimed.onUpdate"),
        K::Minecart(..) => untranscribed_base_update(e, world, "EntityMinecart.onUpdate"),
        K::Boat(..) => untranscribed_base_update(e, world, "EntityBoat.onUpdate"),
        K::LightningBolt(..) => untranscribed_base_update(e, world, "EntityLightningBolt.onUpdate"),
    }
}

fn untranscribed_base_update(e: &mut Entity, world: &mut World, what: &'static str) {
    world.note_unimplemented(what);
    base::on_update(e, world);
}

/// `onEntityUpdate()`.
pub fn on_entity_update(e: &mut Entity, world: &mut World) {
    if e.kind.living().is_some() {
        // src: EntityLiving.java:105-174
        living::on_entity_update(e, world);
    } else {
        // src: Entity.java:193-269
        base::on_entity_update(e, world);
    }
}

/// `handleWaterMovement()`.
pub fn handle_water_movement(e: &mut Entity, world: &mut World) -> bool {
    match e.kind {
        // src: EntityItem.java:79-81
        K::Item(..) => item::handle_water_movement(e, world),
        // src: Entity.java:584-586
        _ => base::handle_water_movement(e, world),
    }
}

/// `attackEntityFrom(attacker, damage)`.
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    attacker: Option<&Entity>,
    damage: i32,
) -> bool {
    let what = match e.kind {
        // src: EntityItem.java:87-95
        K::Item(..) => return item::attack_entity_from(e, world, attacker, damage),
        // src: Entity.java:746-749
        K::Arrow(..)
        | K::Snowball(..)
        | K::Egg(..)
        | K::Fish(..)
        | K::FallingSand(..)
        | K::TntPrimed(..)
        | K::LightningBolt(..) => {
            return base::attack_entity_from(e, world, attacker, damage);
        }
        K::Fireball(..) => "EntityFireball.attackEntityFrom",
        K::Painting(..) => "EntityPainting.attackEntityFrom",
        K::Minecart(..) => "EntityMinecart.attackEntityFrom",
        K::Boat(..) => "EntityBoat.attackEntityFrom",
        K::Mob(..)
        | K::Creeper(..)
        | K::Skeleton(..)
        | K::Spider(..)
        | K::Zombie(..)
        | K::GiantZombie(..) => "EntityMob.attackEntityFrom",
        K::PigZombie(..) => "EntityPigZombie.attackEntityFrom",
        K::Slime(..) | K::Ghast(..) | K::Pig(..) | K::Cow(..) | K::Chicken(..) | K::Squid(..) => {
            "EntityLiving.attackEntityFrom"
        }
        K::Sheep(..) => "EntitySheep.attackEntityFrom",
        K::Wolf(..) => "EntityWolf.attackEntityFrom",
        K::PlayerSp(..) => "EntityPlayer.attackEntityFrom",
    };
    world.note_unimplemented(what);
    base::attack_entity_from(e, world, attacker, damage)
}

/// `kill()`.
pub fn kill(e: &mut Entity, world: &mut World) {
    if e.kind.living().is_some() {
        // src: EntityLiving.java:785-787
        living::kill(e, world);
    } else {
        // src: Entity.java:279-281
        base::kill(e, world);
    }
}

/// `setEntityDead()`.
pub fn set_entity_dead(e: &mut Entity, world: &mut World) {
    let what = match e.kind {
        K::Minecart(..) => "EntityMinecart.setEntityDead",
        K::Slime(..) => "EntitySlime.setEntityDead",
        K::PlayerSp(..) => "EntityPlayer.setEntityDead",
        // src: Entity.java:149-151
        _ => return e.base.set_entity_dead(),
    };
    world.note_unimplemented(what);
    e.base.set_entity_dead();
}

/// `isSneaking()`.
pub fn is_sneaking(e: &Entity, world: &World) -> bool {
    if let K::PlayerSp(..) = e.kind {
        world.note_unimplemented("EntityPlayerSP.isSneaking");
    }
    // src: Entity.java:1067-1069
    e.base.get_entity_flag(1)
}

/// `canTriggerWalking()`.
pub fn can_trigger_walking(e: &Entity) -> bool {
    match e.kind {
        // src: EntityItem.java:23-25, EntityFallingSand.java:26-28, EntityTNTPrimed.java:30-32,
        // EntityMinecart.java:38-40, EntityBoat.java:29-31, EntitySpider.java:15-17,
        // EntityWolf.java:30-32 (all `return false`)
        K::Item(..)
        | K::FallingSand(..)
        | K::TntPrimed(..)
        | K::Minecart(..)
        | K::Boat(..)
        | K::Spider(..)
        | K::Wolf(..) => false,
        // src: Entity.java:542-544
        _ => true,
    }
}

/// `fall(distance)`.
pub fn fall(e: &mut Entity, world: &mut World, distance: f32) {
    let what = match e.kind {
        K::Ghast(..) => "EntityFlying.fall",
        K::Pig(..) => "EntityPig.fall",
        K::Chicken(..) => "EntityChicken.fall",
        K::PlayerSp(..) => "EntityPlayer.fall",
        _ if e.kind.living().is_some() => {
            // src: EntityLiving.java:440-452
            return living::fall(e, world, distance);
        }
        // src: Entity.java:569-574
        _ => return base::fall(e, world, distance),
    };
    world.note_unimplemented(what);
    living::fall(e, world, distance);
}

/// `dealFireDamage(amount)`.
pub fn deal_fire_damage(e: &mut Entity, world: &mut World, amount: i32) {
    match e.kind {
        // src: EntityItem.java:83-85
        K::Item(..) => item::deal_fire_damage(e, world, amount),
        // src: Entity.java:562-567
        _ => base::deal_fire_damage(e, world, amount),
    }
}

/// `getBoundingBox()`: the box other entities collide with (`None` for most).
pub fn get_bounding_box(e: &Entity, world: &World) -> Option<Aabb> {
    match e.kind {
        K::Minecart(..) => world.note_unimplemented("EntityMinecart.getBoundingBox"),
        K::Boat(..) => world.note_unimplemented("EntityBoat.getBoundingBox"),
        _ => {}
    }
    // src: Entity.java:558-560
    None
}

/// `getCollisionBox(other)`: the box `e` presents to `other` (`None` for most).
pub fn get_collision_box(e: &Entity, other: &Entity, world: &World) -> Option<Aabb> {
    let _ = other;
    match e.kind {
        K::Minecart(..) => world.note_unimplemented("EntityMinecart.getCollisionBox"),
        K::Boat(..) => world.note_unimplemented("EntityBoat.getCollisionBox"),
        _ => {}
    }
    // src: Entity.java:912-914
    None
}

/// `canBeCollidedWith()`.
pub fn can_be_collided_with(e: &Entity, world: &World) -> bool {
    let what = match e.kind {
        // src: EntityLiving.java:81-83
        _ if e.kind.living().is_some() => return !e.base.is_dead,
        K::Fireball(..) => "EntityFireball.canBeCollidedWith",
        K::Painting(..) => "EntityPainting.canBeCollidedWith",
        K::FallingSand(..) => "EntityFallingSand.canBeCollidedWith",
        K::TntPrimed(..) => "EntityTNTPrimed.canBeCollidedWith",
        K::Minecart(..) => "EntityMinecart.canBeCollidedWith",
        K::Boat(..) => "EntityBoat.canBeCollidedWith",
        // src: Entity.java:751-753
        _ => return false,
    };
    world.note_unimplemented(what);
    false
}

/// `canBePushed()`.
pub fn can_be_pushed(e: &Entity, world: &World) -> bool {
    let what = match e.kind {
        // src: EntityLiving.java:85-87
        _ if e.kind.living().is_some() => return !e.base.is_dead,
        K::Minecart(..) => "EntityMinecart.canBePushed",
        K::Boat(..) => "EntityBoat.canBePushed",
        // src: Entity.java:755-757
        _ => return false,
    };
    world.note_unimplemented(what);
    false
}

/// `getEyeHeight()`.
pub fn get_eye_height(e: &Entity, world: &World) -> f32 {
    match e.kind {
        K::Wolf(..) => world.note_unimplemented("EntityWolf.getEyeHeight"),
        K::PlayerSp(..) => world.note_unimplemented("EntityPlayer.getEyeHeight"),
        _ => {}
    }
    if e.kind.living().is_some() {
        // src: EntityLiving.java:89-91
        e.base.height * 0.85
    } else {
        // src: Entity.java:603-605
        0.0
    }
}

/// `getYOffset()`.
pub fn get_y_offset(e: &Entity, world: &World) -> f64 {
    if let K::PlayerSp(..) = e.kind {
        world.note_unimplemented("EntityPlayer.getYOffset");
    }
    // src: Entity.java:974-976
    e.base.get_y_offset()
}

/// `getMountedYOffset()`.
pub fn get_mounted_y_offset(e: &Entity, world: &World) -> f64 {
    match e.kind {
        K::Boat(..) => world.note_unimplemented("EntityBoat.getMountedYOffset"),
        K::Minecart(..) => world.note_unimplemented("EntityMinecart.getMountedYOffset"),
        K::Spider(..) => world.note_unimplemented("EntitySpider.getMountedYOffset"),
        _ => {}
    }
    // src: Entity.java:978-980
    base::get_mounted_y_offset(e)
}

/// `moveEntity(dx, dy, dz)`.
pub fn move_entity(e: &mut Entity, world: &mut World, dx: f64, dy: f64, dz: f64) {
    match e.kind {
        K::Painting(..) => world.note_unimplemented("EntityPainting.moveEntity"),
        K::PlayerSp(..) => world.note_unimplemented("EntityPlayerSP.moveEntity"),
        _ => {}
    }
    // src: Entity.java:293-540
    base::move_entity(e, world, dx, dy, dz);
}

/// `isEntityAlive()`.
pub fn is_entity_alive(e: &Entity) -> bool {
    match e.kind.living() {
        // src: EntityLiving.java:575-577
        Some(l) => !e.base.is_dead && l.health > 0,
        // src: Entity.java:888-890
        None => !e.base.is_dead,
    }
}

/// `updateRidden()`.
pub fn update_ridden(e: &mut Entity, world: &mut World) {
    match e.kind {
        K::PlayerSp(..) => {
            world.note_unimplemented("EntityPlayer.updateRidden");
            living::update_ridden(e, world);
        }
        // src: EntityLiving.java:187-191
        _ if e.kind.living().is_some() => living::update_ridden(e, world),
        // src: Entity.java:916-968
        _ => base::update_ridden(e, world),
    }
}
