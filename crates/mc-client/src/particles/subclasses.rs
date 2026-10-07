//! The `EntityFX` subclasses (every `Entity*FX.java` in the tree): their
//! constructors, `onUpdate` and `renderParticle` / `getEntityBrightness`
//! overrides, dispatched on [`FxKind`].
//!
//! `Particle.java` is not one of them: it is `GuiParticle`'s 2D particle
//! (the GUI, WP-3.6).

use bevy::math::DVec3;
use mc_core::math::{floor_double, sin};
use mc_sim::block::{self, Material, fluid};
use mc_sim::entity::Entity;
use mc_sim::world::World;

use super::fx::{Billboard, EntityFx, FxKind, FxRandom, PickupData};
use crate::gl_material::TessBatch;
use crate::textures::Colorizers;
use crate::textures::colorizer;

/// `Block.blockSnow.blockID` (BlockSnowBlock, read by `EntitySlimeFX` for
/// its gravity).
// src: Block.java:672
const BLOCK_SNOW: i32 = 80;

/// A particle `onUpdate` asked `worldObj.spawnParticle` for (the lava's
/// smoke, EntityLavaFX.java:39-41).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpawnRequest {
    /// The particle name.
    pub name: &'static str,
    /// Position.
    pub pos: DVec3,
    /// Motion.
    pub motion: DVec3,
}

/// `Block.colorMultiplier(world, x, y, z)` of block `id` as the client sees
/// it, with `meta` standing for the `getBlockMetadata(x, y, z)` the
/// overrides read, and the biome colours from the client's colour maps.
///
/// Two reasons it is not `mc_sim::block::dispatch::color_multiplier`: the
/// simulation has no colour map (its lookup answers 0, the headless
/// oracle's empty `grassBuffer`), and `playAuxSFX` 2001 reaches the
/// particles after `sendBlockRemoved` has set the block to air, while
/// vanilla built the debris before (PlayerController.java:22-26): the
/// event's metadata is the one vanilla read. The kinds are those of the
/// dispatch: every override of `colorMultiplier` in the tree.
// src: Block.java:530, BlockGrass.java:23-28, BlockFluid.java:14-16,
// src: BlockLeaves.java:23-35, BlockTallGrass.java:22-37,
// src: BlockRedstoneWire.java:37-39
pub fn block_color_multiplier(
    world: &mut World,
    colorizers: &Colorizers,
    id: i32,
    meta: i32,
    x: i32,
    y: i32,
    z: i32,
) -> i32 {
    const WHITE: i32 = 16777215;
    let Some(b) = block::block(id) else {
        return WHITE;
    };
    use block::BlockKind as K;
    match b.kind {
        K::Grass => biome_color(world, x, z, |t, h| colorizers.get_grass_color(t, h)),
        K::Flowing | K::Stationary => WHITE,
        K::Leaves => {
            if (meta & 1) == 1 {
                colorizer::get_foliage_color_pine()
            } else if (meta & 2) == 2 {
                colorizer::get_foliage_color_birch()
            } else {
                biome_color(world, x, z, |t, h| colorizers.get_foliage_color(t, h))
            }
        }
        K::TallGrass => {
            if meta == 0 {
                return WHITE;
            }
            // (long)(x * 3129871 + z * 6129781 + y): the sum is an int.
            let mut var6 = x
                .wrapping_mul(3129871)
                .wrapping_add(z.wrapping_mul(6129781))
                .wrapping_add(y) as i64;
            var6 = var6
                .wrapping_mul(var6)
                .wrapping_mul(42317861)
                .wrapping_add(var6.wrapping_mul(11));
            let x = (x as i64).wrapping_add(var6 >> 14 & 31) as i32;
            let z = (z as i64).wrapping_add(var6 >> 24 & 31) as i32;
            biome_color(world, x, z, |t, h| colorizers.get_grass_color(t, h))
        }
        K::RedstoneWire => 8388608,
        _ => WHITE,
    }
}

/// `getWorldChunkManager().getBiomeGenBaseAt(x, z, 1, 1)`, then `f` of
/// `temperature[0]`, `humidity[0]`.
// src: BlockGrass.java:24-27
fn biome_color(world: &mut World, x: i32, z: i32, f: impl Fn(f64, f64) -> i32) -> i32 {
    let m = world.get_world_chunk_manager();
    m.get_biome_gen_base_at(x, z, 1, 1);
    f(m.temperature[0], m.humidity[0])
}

/// `(float)(Math.random() * 2.0D - 1.0D) * k` as a double.
fn spread(rnd: &mut FxRandom, k: f32) -> f64 {
    (((rnd.math() * 2.0 - 1.0) as f32) * k) as f64
}

/// `(t + partial) / max · 32`, clamped to 0..1: the grow-in of smoke,
/// reddust, notes, hearts and the snow shovel.
// src: EntitySmokeFX.java:28-35
pub(crate) fn grow_in(age: i32, max: i32, partial: f32) -> f32 {
    let var8 = (age as f32 + partial) / max as f32 * 32.0;
    // the two ifs (NaN stays NaN, as with clamp)
    var8.clamp(0.0, 1.0)
}

/// `7 - particleAge * 8 / particleMaxAge` (int arithmetic): the smoke
/// frames count down from 7. A zero `particleMaxAge` would throw in Java;
/// no constructor makes one.
pub(crate) fn smoke_frame(age: i32, max: i32) -> i32 {
    7 - age.wrapping_mul(8).checked_div(max).unwrap_or(0)
}

impl EntityFx {
    // ---- constructors --------------------------------------------------------------

    /// `new EntityBubbleFX(world, x, y, z, vx, vy, vz)`.
    // src: EntityBubbleFX.java:4-16
    pub fn bubble(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, vx, vy, vz, rnd);
        e.kind = FxKind::Bubble;
        e.particle_red = 1.0;
        e.particle_green = 1.0;
        e.particle_blue = 1.0;
        e.particle_texture_index = 32;
        e.set_size(0.02, 0.02);
        e.particle_scale *= e.rand.next_float() * 0.6 + 0.2;
        e.motion_x = vx * 0.20000000298023224 + spread(rnd, 0.02);
        e.motion_y = vy * 0.20000000298023224 + spread(rnd, 0.02);
        e.motion_z = vz * 0.20000000298023224 + spread(rnd, 0.02);
        e.particle_max_age = (8.0 / (rnd.math() * 0.8 + 0.2)) as i32;
        e
    }

    /// `new EntityDiggingFX(world, x, y, z, vx, vy, vz, block, side, meta)`:
    /// the block's bottom texture for `meta`, its particle gravity, grey
    /// 0.6, half size. `None` where the Java would throw (an id without a
    /// block, or a texture lookup out of range).
    // src: EntityDiggingFX.java:7-15
    #[allow(clippy::too_many_arguments)]
    pub fn digging(
        world: &World,
        x: f64,
        y: f64,
        z: f64,
        vx: f64,
        vy: f64,
        vz: f64,
        block_id: i32,
        side: i32,
        meta: i32,
        rnd: &mut FxRandom,
    ) -> Option<Self> {
        let def = block::block(block_id)?;
        let tex =
            block::get_block_texture_from_side_and_metadata(def, 0, meta, &world.block_statics)?;
        let mut e = EntityFx::new(x, y, z, vx, vy, vz, rnd);
        e.kind = FxKind::Digging { block_id, side };
        e.particle_texture_index = tex;
        e.particle_gravity = def.block_particle_gravity;
        e.particle_red = 0.6;
        e.particle_green = 0.6;
        e.particle_blue = 0.6;
        e.particle_scale /= 2.0;
        Some(e)
    }

    /// `EntityDiggingFX.applyColourMultiplier(x, y, z)`: tinted by the
    /// block's `colorMultiplier` there, except grass (whose particle shows
    /// the dirt bottom). `meta` is what `colorMultiplier` reads with
    /// `getBlockMetadata(x, y, z)` at the call ([`block_color_multiplier`]).
    // src: EntityDiggingFX.java:17-27
    pub fn apply_colour_multiplier(
        self,
        world: &mut World,
        colorizers: &Colorizers,
        x: i32,
        y: i32,
        z: i32,
        meta: i32,
    ) -> Self {
        let FxKind::Digging { block_id, .. } = self.kind else {
            return self;
        };
        if block_id == block::ids::GRASS {
            return self;
        }
        let var4 = block_color_multiplier(world, colorizers, block_id, meta, x, y, z);
        self.apply_colour(var4)
    }

    /// `applyColourMultiplier`'s tint with its multiplier `var4` already
    /// computed (the grass exception kept).
    // src: EntityDiggingFX.java:17-27
    pub fn apply_colour(mut self, var4: i32) -> Self {
        let FxKind::Digging { block_id, .. } = self.kind else {
            return self;
        };
        if block_id == block::ids::GRASS {
            return self;
        }
        self.particle_red *= (var4 >> 16 & 255) as f32 / 255.0;
        self.particle_green *= (var4 >> 8 & 255) as f32 / 255.0;
        self.particle_blue *= (var4 & 255) as f32 / 255.0;
        self
    }

    /// `new EntityExplodeFX(world, x, y, z, vx, vy, vz)`.
    // src: EntityExplodeFX.java:4-12
    pub fn explode(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, vx, vy, vz, rnd);
        e.kind = FxKind::Explode;
        e.motion_x = vx + spread(rnd, 0.05);
        e.motion_y = vy + spread(rnd, 0.05);
        e.motion_z = vz + spread(rnd, 0.05);
        let g = e.rand.next_float() * 0.3 + 0.7;
        e.particle_red = g;
        e.particle_green = g;
        e.particle_blue = g;
        e.particle_scale = e.rand.next_float() * e.rand.next_float() * 6.0 + 1.0;
        e.particle_max_age = (16.0 / (e.rand.next_float() as f64 * 0.8 + 0.2)) as i32 + 2;
        e
    }

    /// `new EntityFlameFX(world, x, y, z, vx, vy, vz)`. The three jittered
    /// positions it computes are discarded (`var10000`), but their six
    /// `rand.nextFloat()` draws happen.
    // src: EntityFlameFX.java:6-19
    pub fn flame(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, vx, vy, vz, rnd);
        e.motion_x = e.motion_x * 0.009999999776482582 + vx;
        e.motion_y = e.motion_y * 0.009999999776482582 + vy;
        e.motion_z = e.motion_z * 0.009999999776482582 + vz;
        // vanilla bug: the jittered spawn position is computed and thrown
        // away (EntityFlameFX.java:11-13); only the draws remain.
        for _ in 0..3 {
            let _ = e.rand.next_float() - e.rand.next_float();
        }
        e.kind = FxKind::Flame {
            flame_scale: e.particle_scale,
        };
        e.particle_red = 1.0;
        e.particle_green = 1.0;
        e.particle_blue = 1.0;
        e.particle_max_age = (8.0 / (rnd.math() * 0.8 + 0.2)) as i32 + 4;
        e.no_clip = true;
        e.particle_texture_index = 48;
        e
    }

    /// `new EntityFootStepFX(renderEngine, world, x, y, z)`: still, 200
    /// ticks.
    // src: EntityFootStepFX.java:10-15
    pub fn footstep(x: f64, y: f64, z: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.motion_x = 0.0;
        e.motion_y = 0.0;
        e.motion_z = 0.0;
        e.kind = FxKind::FootStep {
            footstep_age: 0,
            footstep_max_age: 200,
        };
        e
    }

    /// `new EntityHeartFX(world, x, y, z, vx, vy, vz, scale)` (the
    /// 7-argument constructor passes 2.0); the motion arguments are ignored.
    // src: EntityHeartFX.java:6-22
    pub fn heart(x: f64, y: f64, z: f64, var14: f32, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.motion_x *= 0.009999999776482582;
        e.motion_y *= 0.009999999776482582;
        e.motion_z *= 0.009999999776482582;
        e.motion_y += 0.1;
        e.particle_scale *= 0.75;
        e.particle_scale *= var14;
        e.kind = FxKind::Heart {
            particle_scale_over_time: e.particle_scale,
        };
        e.particle_max_age = 16;
        e.no_clip = false;
        e.particle_texture_index = 80;
        e
    }

    /// `new EntityLavaFX(world, x, y, z)`.
    // src: EntityLavaFX.java:6-18
    pub fn lava(x: f64, y: f64, z: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.motion_x *= 0.800000011920929;
        e.motion_y *= 0.800000011920929;
        e.motion_z *= 0.800000011920929;
        e.motion_y = (e.rand.next_float() * 0.4 + 0.05) as f64;
        e.particle_red = 1.0;
        e.particle_green = 1.0;
        e.particle_blue = 1.0;
        e.particle_scale *= e.rand.next_float() * 2.0 + 0.2;
        e.kind = FxKind::Lava {
            lava_particle_scale: e.particle_scale,
        };
        e.particle_max_age = (16.0 / (rnd.math() * 0.8 + 0.2)) as i32;
        e.no_clip = false;
        e.particle_texture_index = 49;
        e
    }

    /// `new EntityNoteFX(world, x, y, z, note, vy, vz, scale)` (the
    /// 7-argument constructor passes 2.0): the colour is three sines of the
    /// note (`vx`, the pitch / 24) a third of a turn apart; the motion
    /// arguments are otherwise ignored.
    // src: EntityNoteFX.java:6-25
    pub fn note(x: f64, y: f64, z: f64, var8: f64, var14: f32, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.motion_x *= 0.009999999776482582;
        e.motion_y *= 0.009999999776482582;
        e.motion_z *= 0.009999999776482582;
        e.motion_y += 0.2;
        let (r, g, b) = note_colour(var8);
        e.particle_red = r;
        e.particle_green = g;
        e.particle_blue = b;
        e.particle_scale *= 0.75;
        e.particle_scale *= var14;
        e.kind = FxKind::Note {
            note_particle_scale: e.particle_scale,
        };
        e.particle_max_age = 6;
        e.no_clip = false;
        e.particle_texture_index = 64;
        e
    }

    /// `new EntityPickupFX(world, picked, player, yOffs)`: the base
    /// constructor at the picked entity's position and motion (its draws
    /// happen), then 3 ticks.
    // src: EntityPickupFX.java:12-18
    pub fn pickup(picked: Box<Entity>, player_id: i32, y_offs: f32, rnd: &mut FxRandom) -> Self {
        let b = &picked.base;
        let mut e = EntityFx::new(
            b.pos_x, b.pos_y, b.pos_z, b.motion_x, b.motion_y, b.motion_z, rnd,
        );
        e.kind = FxKind::Pickup(Box::new(PickupData {
            entity_to_pick_up: picked,
            entity_picking_up: player_id,
            age: 0,
            max_age: 3,
            y_offs,
        }));
        e
    }

    /// `new EntityPortalFX(world, x, y, z, vx, vy, vz)`: purple, flies from
    /// `pos + motion` back to `pos`.
    // src: EntityPortalFX.java:9-25
    pub fn portal(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, vx, vy, vz, rnd);
        e.motion_x = vx;
        e.motion_y = vy;
        e.motion_z = vz;
        e.pos_x = x;
        e.pos_y = y;
        e.pos_z = z;
        let var14 = e.rand.next_float() * 0.6 + 0.4;
        let s = e.rand.next_float() * 0.2 + 0.5;
        e.particle_scale = s;
        e.kind = FxKind::Portal {
            portal_particle_scale: s,
            portal_pos: DVec3::new(x, y, z),
        };
        let c = 1.0 * var14;
        e.particle_red = c;
        e.particle_green = c;
        e.particle_blue = c;
        e.particle_green *= 0.3;
        e.particle_red *= 0.9;
        e.particle_max_age = (rnd.math() * 10.0) as i32 + 40;
        e.no_clip = true;
        e.particle_texture_index = (rnd.math() * 8.0) as i32;
        e
    }

    /// `new EntityRainFX(world, x, y, z)`: a splash drop, frames 19-22.
    // src: EntityRainFX.java:4-16
    pub fn rain(x: f64, y: f64, z: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.kind = FxKind::Rain;
        e.motion_x *= 0.30000001192092896;
        e.motion_y = (rnd.math() as f32 * 0.2 + 0.1) as f64;
        e.motion_z *= 0.30000001192092896;
        e.particle_red = 1.0;
        e.particle_green = 1.0;
        e.particle_blue = 1.0;
        e.particle_texture_index = 19 + e.rand.next_int_bounded(4);
        e.set_size(0.01, 0.01);
        e.particle_gravity = 0.06;
        e.particle_max_age = (8.0 / (rnd.math() * 0.8 + 0.2)) as i32;
        e
    }

    /// `new EntitySplashFX(world, x, y, z, vx, vy, vz)`: a rain drop one
    /// frame on, lighter, taking a horizontal motion when given one.
    // src: EntitySplashFX.java:4-14
    pub fn splash(x: f64, y: f64, z: f64, vx: f64, vy: f64, vz: f64, rnd: &mut FxRandom) -> Self {
        let mut e = EntityFx::rain(x, y, z, rnd);
        e.kind = FxKind::Splash;
        e.particle_gravity = 0.04;
        e.particle_texture_index += 1;
        if vy == 0.0 && (vx != 0.0 || vz != 0.0) {
            e.motion_x = vx;
            e.motion_y = vy + 0.1;
            e.motion_z = vz;
        }
        e
    }

    /// `new EntityReddustFX(world, x, y, z, scale, r, g, b)` (the 7-argument
    /// constructor passes scale 1.0, and `spawnParticle` passes the motion
    /// as the colour); a red of 0 becomes 1.
    // src: EntityReddustFX.java:6-29
    #[allow(clippy::too_many_arguments)]
    pub fn reddust(
        x: f64,
        y: f64,
        z: f64,
        var8: f32,
        mut var9: f32,
        var10: f32,
        var11: f32,
        rnd: &mut FxRandom,
    ) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.motion_x *= 0.10000000149011612;
        e.motion_y *= 0.10000000149011612;
        e.motion_z *= 0.10000000149011612;
        if var9 == 0.0 {
            var9 = 1.0;
        }
        let var12 = rnd.math() as f32 * 0.4 + 0.6;
        e.particle_red = ((rnd.math() * 0.20000000298023224) as f32 + 0.8) * var9 * var12;
        e.particle_green = ((rnd.math() * 0.20000000298023224) as f32 + 0.8) * var10 * var12;
        e.particle_blue = ((rnd.math() * 0.20000000298023224) as f32 + 0.8) * var11 * var12;
        e.particle_scale *= 0.75;
        e.particle_scale *= var8;
        e.kind = FxKind::Reddust {
            reddust_particle_scale: e.particle_scale,
        };
        e.particle_max_age = (8.0 / (rnd.math() * 0.8 + 0.2)) as i32;
        e.particle_max_age = (e.particle_max_age as f32 * var8) as i32;
        e.no_clip = false;
        e
    }

    /// `new EntitySlimeFX(world, x, y, z, item)`: the item's icon (damage
    /// 0) from items.png, the snow block's gravity, half size. `None` for an
    /// id without an item.
    // src: EntitySlimeFX.java:4-10
    pub fn slime(x: f64, y: f64, z: f64, item_id: i32, rnd: &mut FxRandom) -> Option<Self> {
        let item = mc_sim::item::item(item_id)?;
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.kind = FxKind::Slime;
        e.particle_texture_index = item.get_icon_from_damage(0);
        e.particle_red = 1.0;
        e.particle_green = 1.0;
        e.particle_blue = 1.0;
        e.particle_gravity = block::block(BLOCK_SNOW).map_or(1.0, |b| b.block_particle_gravity);
        e.particle_scale /= 2.0;
        Some(e)
    }

    /// `new EntitySmokeFX(world, x, y, z, vx, vy, vz, scale)` (the
    /// 7-argument constructor passes 1.0; `largesmoke` 2.5).
    // src: EntitySmokeFX.java:6-25
    #[allow(clippy::too_many_arguments)]
    pub fn smoke(
        x: f64,
        y: f64,
        z: f64,
        vx: f64,
        vy: f64,
        vz: f64,
        var14: f32,
        rnd: &mut FxRandom,
    ) -> Self {
        let mut e = EntityFx::new(x, y, z, 0.0, 0.0, 0.0, rnd);
        e.motion_x *= 0.10000000149011612;
        e.motion_y *= 0.10000000149011612;
        e.motion_z *= 0.10000000149011612;
        e.motion_x += vx;
        e.motion_y += vy;
        e.motion_z += vz;
        let g = (rnd.math() * 0.30000001192092896) as f32;
        e.particle_red = g;
        e.particle_green = g;
        e.particle_blue = g;
        e.particle_scale *= 0.75;
        e.particle_scale *= var14;
        e.kind = FxKind::Smoke {
            smoke_particle_scale: e.particle_scale,
        };
        e.particle_max_age = (8.0 / (rnd.math() * 0.8 + 0.2)) as i32;
        e.particle_max_age = (e.particle_max_age as f32 * var14) as i32;
        e.no_clip = false;
        e
    }

    /// `new EntitySnowShovelFX(world, x, y, z, vx, vy, vz, scale)` (the
    /// 7-argument constructor passes 1.0). Unlike the smoke, the base
    /// constructor gets the motion too.
    // src: EntitySnowShovelFX.java:6-25
    #[allow(clippy::too_many_arguments)]
    pub fn snow_shovel(
        x: f64,
        y: f64,
        z: f64,
        vx: f64,
        vy: f64,
        vz: f64,
        var14: f32,
        rnd: &mut FxRandom,
    ) -> Self {
        let mut e = EntityFx::new(x, y, z, vx, vy, vz, rnd);
        e.motion_x *= 0.10000000149011612;
        e.motion_y *= 0.10000000149011612;
        e.motion_z *= 0.10000000149011612;
        e.motion_x += vx;
        e.motion_y += vy;
        e.motion_z += vz;
        let g = 1.0 - (rnd.math() * 0.30000001192092896) as f32;
        e.particle_red = g;
        e.particle_green = g;
        e.particle_blue = g;
        e.particle_scale *= 0.75;
        e.particle_scale *= var14;
        e.kind = FxKind::SnowShovel {
            snow_dig_particle_scale: e.particle_scale,
        };
        e.particle_max_age = (8.0 / (rnd.math() * 0.8 + 0.2)) as i32;
        e.particle_max_age = (e.particle_max_age as f32 * var14) as i32;
        e.no_clip = false;
        e
    }

    // ---- onUpdate --------------------------------------------------------------------

    /// `onUpdate()` of the particle's class. Returns the particle the lava
    /// asks `worldObj.spawnParticle` for (added by the caller at once, as
    /// `RenderGlobal.spawnParticle` would).
    pub fn on_update(&mut self, world: &mut World, rnd: &mut FxRandom) -> Option<SpawnRequest> {
        match &mut self.kind {
            FxKind::Base | FxKind::Digging { .. } | FxKind::Slime => self.update_base(world),
            FxKind::Bubble => self.update_bubble(world),
            FxKind::Explode => self.update_explode(world),
            FxKind::Flame { .. } => self.update_flame(world),
            // src: EntityFootStepFX.java:46-52
            FxKind::FootStep {
                footstep_age,
                footstep_max_age,
            } => {
                *footstep_age += 1;
                if *footstep_age == *footstep_max_age {
                    self.set_entity_dead();
                }
            }
            FxKind::Heart { .. } | FxKind::Note { .. } => self.update_heart_note(world),
            FxKind::Lava { .. } => return self.update_lava(world),
            // src: EntityPickupFX.java:43-49
            FxKind::Pickup(p) => {
                p.age += 1;
                if p.age == p.max_age {
                    self.set_entity_dead();
                }
            }
            FxKind::Portal { .. } => self.update_portal(),
            FxKind::Rain | FxKind::Splash => self.update_rain(world, rnd),
            FxKind::Reddust { .. } => self.update_reddust(world),
            FxKind::Smoke { .. } => self.update_smoke(world),
            FxKind::SnowShovel { .. } => self.update_snow_shovel(world),
        }
        None
    }

    /// `prevPos = pos`.
    fn keep_prev_pos(&mut self) {
        self.prev_pos_x = self.pos_x;
        self.prev_pos_y = self.pos_y;
        self.prev_pos_z = self.pos_z;
    }

    /// `if (this.particleAge++ >= this.particleMaxAge) setEntityDead()`.
    fn age_one(&mut self) {
        let a = self.particle_age;
        self.particle_age += 1;
        if a >= self.particle_max_age {
            self.set_entity_dead();
        }
    }

    /// `motion *= f`.
    fn damp(&mut self, f: f64) {
        self.motion_x *= f;
        self.motion_y *= f;
        self.motion_z *= f;
    }

    /// `if (onGround) motionX, motionZ *= 0.7`.
    fn ground_friction(&mut self) {
        if self.on_ground {
            self.motion_x *= 0.699999988079071;
            self.motion_z *= 0.699999988079071;
        }
    }

    /// `if (posY == prevPosY) motionX, motionZ *= 1.1`: a particle that hit
    /// a ceiling spreads sideways.
    fn spread_when_stuck(&mut self) {
        if self.pos_y == self.prev_pos_y {
            self.motion_x *= 1.1;
            self.motion_z *= 1.1;
        }
    }

    /// `EntityFX.onUpdate`: age, gravity `0.04 · particleGravity`, move,
    /// friction 0.98, ground friction.
    // src: EntityFX.java:59-77
    fn update_base(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.age_one();
        self.motion_y -= 0.04 * self.particle_gravity as f64;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.9800000190734863);
        self.ground_friction();
    }

    /// `EntityBubbleFX.onUpdate`: rises, dies out of water.
    // src: EntityBubbleFX.java:18-35
    fn update_bubble(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.motion_y += 0.002;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.8500000238418579);
        if world.get_block_material(
            floor_double(self.pos_x),
            floor_double(self.pos_y),
            floor_double(self.pos_z),
        ) != Material::Water
        {
            self.set_entity_dead();
        }
        let m = self.particle_max_age;
        self.particle_max_age -= 1;
        if m <= 0 {
            self.set_entity_dead();
        }
    }

    /// `EntityExplodeFX.onUpdate`: the smoke frames, rising, friction 0.9.
    // src: EntityExplodeFX.java:18-37
    fn update_explode(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.age_one();
        self.particle_texture_index = smoke_frame(self.particle_age, self.particle_max_age);
        self.motion_y += 0.004;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.8999999761581421);
        self.ground_friction();
    }

    /// `EntityFlameFX.onUpdate`: no gravity, friction 0.96 (no clipping).
    // src: EntityFlameFX.java:41-58
    fn update_flame(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.age_one();
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.9599999785423279);
        self.ground_friction();
    }

    /// `EntityHeartFX.onUpdate` (friction 0.86) / `EntityNoteFX.onUpdate`
    /// (friction 0.66).
    // src: EntityHeartFX.java:38-60, EntityNoteFX.java:41-63
    fn update_heart_note(&mut self, world: &mut World) {
        let f = if matches!(self.kind, FxKind::Heart { .. }) {
            0.8600000143051147
        } else {
            0.6600000262260437
        };
        self.keep_prev_pos();
        self.age_one();
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.spread_when_stuck();
        self.damp(f);
        self.ground_friction();
    }

    /// `EntityLavaFX.onUpdate`: smoke trailing less often with age, falls
    /// at 0.03, friction 0.999.
    // src: EntityLavaFX.java:30-53
    fn update_lava(&mut self, world: &mut World) -> Option<SpawnRequest> {
        self.keep_prev_pos();
        self.age_one();
        let var1 = self.particle_age as f32 / self.particle_max_age as f32;
        let smoke = (self.rand.next_float() > var1).then(|| SpawnRequest {
            name: "smoke",
            pos: DVec3::new(self.pos_x, self.pos_y, self.pos_z),
            motion: DVec3::new(self.motion_x, self.motion_y, self.motion_z),
        });
        self.motion_y -= 0.03;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.9990000128746033);
        self.ground_friction();
        smoke
    }

    /// `EntityPortalFX.onUpdate`: no physics; the position is set along the
    /// path from `portalPos + motion` to `portalPos` (plus a rise falling to
    /// 0).
    // src: EntityPortalFX.java:44-59
    fn update_portal(&mut self) {
        let FxKind::Portal { portal_pos, .. } = self.kind else {
            return;
        };
        self.keep_prev_pos();
        let mut var1 = self.particle_age as f32 / self.particle_max_age as f32;
        let var2 = var1;
        var1 = -var1 + var1 * var1 * 2.0;
        var1 = 1.0 - var1;
        self.pos_x = portal_pos.x + self.motion_x * var1 as f64;
        self.pos_y = portal_pos.y + self.motion_y * var1 as f64 + (1.0 - var2) as f64;
        self.pos_z = portal_pos.z + self.motion_z * var1 as f64;
        self.age_one();
    }

    /// `EntityRainFX.onUpdate` (also the splash): falls by
    /// `particleGravity`, half the drops die on landing, dies inside a
    /// liquid or a solid block below its surface.
    // src: EntityRainFX.java:22-52
    fn update_rain(&mut self, world: &mut World, rnd: &mut FxRandom) {
        self.keep_prev_pos();
        self.motion_y -= self.particle_gravity as f64;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.9800000190734863);
        let m = self.particle_max_age;
        self.particle_max_age -= 1;
        if m <= 0 {
            self.set_entity_dead();
        }
        if self.on_ground {
            if rnd.math() < 0.5 {
                self.set_entity_dead();
            }
            self.motion_x *= 0.699999988079071;
            self.motion_z *= 0.699999988079071;
        }
        let (bx, by, bz) = (
            floor_double(self.pos_x),
            floor_double(self.pos_y),
            floor_double(self.pos_z),
        );
        let var1 = world.get_block_material(bx, by, bz);
        if var1.get_is_liquid() || var1.is_solid() {
            let var2 = ((by + 1) as f32
                - fluid::get_percent_air(world.get_block_metadata(bx, by, bz)))
                as f64;
            if self.pos_y < var2 {
                self.set_entity_dead();
            }
        }
    }

    /// `EntityReddustFX.onUpdate`: the smoke frames, friction 0.96.
    // src: EntityReddustFX.java:45-68
    fn update_reddust(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.age_one();
        self.particle_texture_index = smoke_frame(self.particle_age, self.particle_max_age);
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.spread_when_stuck();
        self.damp(0.9599999785423279);
        self.ground_friction();
    }

    /// `EntitySmokeFX.onUpdate`: the smoke frames, rising 0.004, friction
    /// 0.96.
    // src: EntitySmokeFX.java:41-65
    fn update_smoke(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.age_one();
        self.particle_texture_index = smoke_frame(self.particle_age, self.particle_max_age);
        self.motion_y += 0.004;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.spread_when_stuck();
        self.damp(0.9599999785423279);
        self.ground_friction();
    }

    /// `EntitySnowShovelFX.onUpdate`: the smoke frames, falling 0.03,
    /// friction 0.99.
    // src: EntitySnowShovelFX.java:41-60
    fn update_snow_shovel(&mut self, world: &mut World) {
        self.keep_prev_pos();
        self.age_one();
        self.particle_texture_index = smoke_frame(self.particle_age, self.particle_max_age);
        self.motion_y -= 0.03;
        self.move_entity(world, self.motion_x, self.motion_y, self.motion_z);
        self.damp(0.9900000095367432);
        self.ground_friction();
    }

    // ---- getEntityBrightness, renderParticle -------------------------------------------

    /// `getEntityBrightness(partial)` of the particle's class.
    // src: Entity.java:628-643, EntityFlameFX.java:27-39, EntityLavaFX.java:20-22,
    // src: EntityPortalFX.java:36-42
    pub fn get_entity_brightness(&self, world: &mut World, partial: f32) -> f32 {
        match self.kind {
            FxKind::Flame { .. } => {
                let var2 = ((self.particle_age as f32 + partial) / self.particle_max_age as f32)
                    .clamp(0.0, 1.0);
                let var3 = self.entity_brightness_base(world);
                var3 * var2 + (1.0 - var2)
            }
            FxKind::Lava { .. } => 1.0,
            FxKind::Portal { .. } => {
                let var2 = self.entity_brightness_base(world);
                let mut var3 = self.particle_age as f32 / self.particle_max_age as f32;
                var3 *= var3;
                var3 *= var3;
                var2 * (1.0 - var3) + var3
            }
            _ => self.entity_brightness_base(world),
        }
    }

    /// `renderParticle(tessellator, partial, …)` of a layer 0-2 particle
    /// into `b` (relative to `interp`, `EntityFX.interpPos`). Several
    /// classes first set `particleScale` from their age, as in Java (the
    /// field keeps the value).
    // src: EntityFX.java:79-94 and the overrides cited per arm
    pub fn render_particle(
        &mut self,
        world: &mut World,
        b: &mut TessBatch,
        partial: f32,
        bb: &Billboard,
        interp: DVec3,
    ) {
        let (age, max) = (self.particle_age, self.particle_max_age);
        match self.kind {
            // src: EntityFlameFX.java:21-25
            FxKind::Flame { flame_scale } => {
                let var8 = (age as f32 + partial) / max as f32;
                self.particle_scale = flame_scale * (1.0 - var8 * var8 * 0.5);
            }
            // src: EntityLavaFX.java:24-28
            FxKind::Lava {
                lava_particle_scale,
            } => {
                let var8 = (age as f32 + partial) / max as f32;
                self.particle_scale = lava_particle_scale * (1.0 - var8 * var8);
            }
            // src: EntityPortalFX.java:27-34
            FxKind::Portal {
                portal_particle_scale,
                ..
            } => {
                let mut var8 = (age as f32 + partial) / max as f32;
                var8 = 1.0 - var8;
                var8 *= var8;
                var8 = 1.0 - var8;
                self.particle_scale = portal_particle_scale * var8;
            }
            // src: EntityHeartFX.java:24-36
            FxKind::Heart {
                particle_scale_over_time: s,
            }
            // src: EntityNoteFX.java:27-39
            | FxKind::Note {
                note_particle_scale: s,
            }
            // src: EntityReddustFX.java:31-43
            | FxKind::Reddust {
                reddust_particle_scale: s,
            }
            // src: EntitySmokeFX.java:27-39
            | FxKind::Smoke {
                smoke_particle_scale: s,
            }
            // src: EntitySnowShovelFX.java:27-39
            | FxKind::SnowShovel {
                snow_dig_particle_scale: s,
            } => {
                self.particle_scale = s * grow_in(age, max, partial);
            }
            _ => {}
        }
        let brightness = self.get_entity_brightness(world, partial);
        match self.kind {
            FxKind::Digging { .. } | FxKind::Slime => {
                self.render_sub_tile_quad(b, partial, bb, interp, brightness)
            }
            FxKind::FootStep { .. } | FxKind::Pickup(_) => {}
            _ => self.render_base_quad(b, partial, bb, interp, brightness),
        }
    }
}

/// `EntityNoteFX`'s colour of note `var8` (`pitch / 24`): `sin((n + k/3) ·
/// 2π) · 0.65 + 0.35` for k = 0, 1, 2 (MathHelper's table).
// src: EntityNoteFX.java:16-18
pub fn note_colour(var8: f64) -> (f32, f32, f32) {
    let n = var8 as f32;
    (
        sin((n + 0.0) * 3.1415927 * 2.0) * 0.65 + 0.35,
        sin((n + 0.33333334) * 3.1415927 * 2.0) * 0.65 + 0.35,
        sin((n + 0.6666667) * 3.1415927 * 2.0) * 0.65 + 0.35,
    )
}

/// What `EntityPickupFX.renderParticle` hands `RenderManager`: the entity
/// at `pos` (relative to `interpPos`, `(double)(float)` as in the call),
/// its `rotationYaw`, the partial tick, and `glColor4f(b, b, b, 1)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickupPose {
    /// `renderEntityWithPosYaw`'s x, y, z.
    pub pos: DVec3,
    /// `entityToPickUp.rotationYaw`.
    pub yaw: f32,
    /// The light brightness at the drawn position (`glColor4f`).
    pub brightness: f32,
}

impl EntityFx {
    /// `EntityFootStepFX.renderParticle`: a flat 0.25-block footprint at the
    /// particle (not interpolated), alpha `min(1, 2 − 2t²) · 0.2` with `t`
    /// the age fraction, coloured by the light there (`/misc/footprint.png`,
    /// blended 770/771, lighting off).
    // src: EntityFootStepFX.java:17-44
    pub fn render_footstep(
        &self,
        world: &mut World,
        b: &mut TessBatch,
        partial: f32,
        interp: DVec3,
    ) {
        let FxKind::FootStep {
            footstep_age,
            footstep_max_age,
        } = self.kind
        else {
            return;
        };
        let mut var8 = (footstep_age as f32 + partial) / footstep_max_age as f32;
        var8 *= var8;
        let mut var9 = 2.0 - var8 * 2.0;
        if var9 > 1.0 {
            var9 = 1.0;
        }
        var9 *= 0.2;
        let var10 = 0.125f32;
        let var11 = (self.pos_x - interp.x) as f32;
        let var12 = (self.pos_y - interp.y) as f32;
        let var13 = (self.pos_z - interp.z) as f32;
        let var14 = world.get_light_brightness(
            floor_double(self.pos_x),
            floor_double(self.pos_y),
            floor_double(self.pos_z),
        );
        let c = crate::gl_material::tess_color(var14, var14, var14, var9);
        b.vertex([var11 - var10, var12, var13 + var10], [0.0, 1.0], c);
        b.vertex([var11 + var10, var12, var13 + var10], [1.0, 1.0], c);
        b.vertex([var11 + var10, var12, var13 - var10], [1.0, 0.0], c);
        b.vertex([var11 - var10, var12, var13 - var10], [0.0, 0.0], c);
    }

    /// `EntityPickupFX.renderParticle` up to the `RenderManager` call: the
    /// picked entity moves from where it was picked to the picker's
    /// interpolated position plus `yOffs`, along `t²` of the age fraction,
    /// lit by the brightness there. `picker` is the picker's
    /// `lastTickPos` and `pos`.
    // src: EntityPickupFX.java:20-41
    pub fn pickup_pose(
        &self,
        world: &mut World,
        picker: (DVec3, DVec3),
        partial: f32,
        interp: DVec3,
    ) -> Option<PickupPose> {
        let FxKind::Pickup(p) = &self.kind else {
            return None;
        };
        let mut var8 = (p.age as f32 + partial) / p.max_age as f32;
        var8 *= var8;
        let e = &p.entity_to_pick_up.base;
        let (var9, var11, var13) = (e.pos_x, e.pos_y, e.pos_z);
        let (last, pos) = picker;
        let pt = partial as f64;
        let var15 = last.x + (pos.x - last.x) * pt;
        let var17 = last.y + (pos.y - last.y) * pt + p.y_offs as f64;
        let var19 = last.z + (pos.z - last.z) * pt;
        let mut var21 = var9 + (var15 - var9) * var8 as f64;
        let mut var23 = var11 + (var17 - var11) * var8 as f64;
        let mut var25 = var13 + (var19 - var13) * var8 as f64;
        let var27 = floor_double(var21);
        let var28 = floor_double(var23 + (self.y_offset / 2.0) as f64);
        let var29 = floor_double(var25);
        let var30 = world.get_light_brightness(var27, var28, var29);
        var21 -= interp.x;
        var23 -= interp.y;
        var25 -= interp.z;
        Some(PickupPose {
            pos: DVec3::new(
                var21 as f32 as f64,
                var23 as f32 as f64,
                var25 as f32 as f64,
            ),
            yaw: e.rotation_yaw,
            brightness: var30,
        })
    }
}
