//! `RenderBiped` (RenderBiped.java): the zombie's and the skeleton's
//! renderer (and the pig zombie's, through its superclass), with the held
//! item of `getHeldItem()` in the right hand: the skeleton's bow, the pig
//! zombie's golden sword (EntitySkeleton.java:83-89,
//! EntityPigZombie.java:89-95); zombies hold nothing.

use mc_sim::entity::{Entity, EntityKindData as K, pig_zombie, skeleton};
use mc_sim::item::ItemStack;

use super::Ctx;
use super::living::RenderLiving;
use crate::model::biped;

/// `entity.getHeldItem()`.
// src: EntityLiving.java:846-848 (null; EntityPlayer does not override it, so
// a player gets null here: RenderPlayer reads inventory.getCurrentItem()
// itself, RenderPlayer.java:179), EntitySkeleton.java:83-89,
// EntityPigZombie.java:89-95
pub fn get_held_item(e: &Entity) -> Option<ItemStack> {
    match &e.kind {
        K::Skeleton(..) => Some(skeleton::get_held_item()),
        K::PigZombie(..) => Some(pig_zombie::get_held_item()),
        _ => None,
    }
}

/// `renderEquippedItems(entity, partial)`: the held item at the right arm.
// src: RenderBiped.java:13-46
pub fn render_equipped_items(r: &mut RenderLiving, cx: &mut Ctx, var1: &Entity) {
    let Some(var3) = get_held_item(var1) else {
        return;
    };
    cx.gl.push_matrix();
    if let Some(t) = r.main_model.post_render(biped::RIGHT_ARM, 0.0625) {
        cx.gl.mult_transform(&t);
    }
    cx.gl.translatef(-0.0625, 0.4375, 0.0625);
    // RenderBiped has no shouldRotateAroundWhenRendering branch.
    super::player::held_item_transform(cx, &var3, false);
    cx.render_item(var1, &var3);
    cx.gl.pop_matrix();
}
