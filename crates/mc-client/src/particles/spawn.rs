//! Where particles come from: `RenderGlobal.spawnParticle`'s name table,
//! the particle halves of `RenderGlobal.playAuxSFX` (2000 dispenser smoke,
//! 2001 block break) and `EntityRenderer.addRainParticles`.
//!
//! Not in this tree (Beta 1.7.3): `hugeexplosion`, `largeexplode`, the
//! `tilecrack_<id>` prefix and a particle-amount option (`GameSettings` has
//! none); `RenderGlobal.spawnParticle` has no other names than
//! [`particle_class`]'s.
//!
//! # `worldObj.rand`
//!
//! `playAuxSFX` 2000 draws its offsets from `worldObj.rand`, the world's
//! own `Random`, in the middle of the simulation's call. The simulation only
//! queues the event (`WorldEvent::PlayAuxSfx`), so the client draws from a
//! stand-in `Random` ([`super::EffectRenderer::aux_world_rand`]), as the
//! sound half does (`crate::sound::aux_sfx`). The world's stream therefore
//! skips the 50 draws vanilla makes per dispenser shot: an interface change
//! the WP-3.8 report raised for 1003/1004 too.

use bevy::math::DVec3;
use mc_core::JavaRandom;
use mc_core::math::floor_double;
use mc_sim::block::{self, Material};
use mc_sim::item;
use mc_sim::world::World;

use super::EffectRenderer;
use super::fx::{EntityFx, FxRandom};

/// `Item.slimeBall.shiftedIndex` (`new Item(85)`).
// src: Item.java:359
pub const SLIME_BALL: i32 = 256 + 85;

/// The classes `RenderGlobal.spawnParticle` constructs, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleClass {
    /// `"bubble"` → `EntityBubbleFX`.
    Bubble,
    /// `"smoke"` → `EntitySmokeFX` (scale 1).
    Smoke,
    /// `"note"` → `EntityNoteFX`.
    Note,
    /// `"portal"` → `EntityPortalFX`.
    Portal,
    /// `"explode"` → `EntityExplodeFX`.
    Explode,
    /// `"flame"` → `EntityFlameFX`.
    Flame,
    /// `"lava"` → `EntityLavaFX`.
    Lava,
    /// `"footstep"` → `EntityFootStepFX`.
    Footstep,
    /// `"splash"` → `EntitySplashFX`.
    Splash,
    /// `"largesmoke"` → `EntitySmokeFX` with scale 2.5.
    LargeSmoke,
    /// `"reddust"` → `EntityReddustFX` (the motion is the colour).
    Reddust,
    /// `"snowballpoof"` → `EntitySlimeFX` with `Item.snowball`.
    SnowballPoof,
    /// `"snowshovel"` → `EntitySnowShovelFX`.
    SnowShovel,
    /// `"slime"` → `EntitySlimeFX` with `Item.slimeBall`.
    Slime,
    /// `"heart"` → `EntityHeartFX`.
    Heart,
}

/// `RenderGlobal.spawnParticle`'s `equals` chain; `None` for any other
/// name (nothing is spawned).
// src: RenderGlobal.java:1282-1312
pub fn particle_class(name: &str) -> Option<ParticleClass> {
    use ParticleClass as P;
    Some(match name {
        "bubble" => P::Bubble,
        "smoke" => P::Smoke,
        "note" => P::Note,
        "portal" => P::Portal,
        "explode" => P::Explode,
        "flame" => P::Flame,
        "lava" => P::Lava,
        "footstep" => P::Footstep,
        "splash" => P::Splash,
        "largesmoke" => P::LargeSmoke,
        "reddust" => P::Reddust,
        "snowballpoof" => P::SnowballPoof,
        "snowshovel" => P::SnowShovel,
        "slime" => P::Slime,
        "heart" => P::Heart,
        _ => return None,
    })
}

/// The particle `class` at `(x, y, z)` with the call's motion arguments.
/// `None` only for an item without a definition (cannot happen for the two
/// items used).
// src: RenderGlobal.java:1282-1312
pub fn construct(class: ParticleClass, p: DVec3, v: DVec3, rnd: &mut FxRandom) -> Option<EntityFx> {
    use ParticleClass as P;
    let (x, y, z, vx, vy, vz) = (p.x, p.y, p.z, v.x, v.y, v.z);
    Some(match class {
        P::Bubble => EntityFx::bubble(x, y, z, vx, vy, vz, rnd),
        P::Smoke => EntityFx::smoke(x, y, z, vx, vy, vz, 1.0, rnd),
        P::Note => EntityFx::note(x, y, z, vx, 2.0, rnd),
        P::Portal => EntityFx::portal(x, y, z, vx, vy, vz, rnd),
        P::Explode => EntityFx::explode(x, y, z, vx, vy, vz, rnd),
        P::Flame => EntityFx::flame(x, y, z, vx, vy, vz, rnd),
        P::Lava => EntityFx::lava(x, y, z, rnd),
        P::Footstep => EntityFx::footstep(x, y, z, rnd),
        P::Splash => EntityFx::splash(x, y, z, vx, vy, vz, rnd),
        // src: RenderGlobal.java:1301
        P::LargeSmoke => EntityFx::smoke(x, y, z, vx, vy, vz, 2.5, rnd),
        // src: RenderGlobal.java:1303 ((float) casts of the motion)
        P::Reddust => EntityFx::reddust(x, y, z, 1.0, vx as f32, vy as f32, vz as f32, rnd),
        P::SnowballPoof => EntityFx::slime(x, y, z, item::ids::SNOWBALL, rnd)?,
        P::SnowShovel => EntityFx::snow_shovel(x, y, z, vx, vy, vz, 1.0, rnd),
        P::Slime => EntityFx::slime(x, y, z, SLIME_BALL, rnd)?,
        // src: EntityHeartFX.java:6-8 (the 7-argument constructor: scale 2)
        P::Heart => EntityFx::heart(x, y, z, 2.0, rnd),
    })
}

/// `RenderGlobal.spawnParticle`'s range test: within 16 blocks of the view
/// entity (`posX/Y/Z`), compared squared.
// src: RenderGlobal.java:1277-1281
pub fn in_spawn_range(view: DVec3, p: DVec3) -> bool {
    let var14 = view.x - p.x;
    let var16 = view.y - p.y;
    let var18 = view.z - p.z;
    let var20 = 16.0;
    var14 * var14 + var16 * var16 + var18 * var18 <= var20 * var20
}

/// The offsets and motions of `playAuxSFX(2000, x, y, z, data)`'s ten
/// smoke puffs (a dispenser shooting toward `data`'s direction:
/// `data % 3 - 1` in x, `data / 3 % 3 - 1` in z), drawn from `rand` (the
/// world's `Random` in vanilla; module docs): `(pos, motion)` each.
// src: RenderGlobal.java:1388-1406
pub fn dispenser_smoke(
    x: i32,
    y: i32,
    z: i32,
    data: i32,
    var7: &mut JavaRandom,
) -> Vec<(DVec3, DVec3)> {
    let var8 = data % 3 - 1;
    let var9 = data / 3 % 3 - 1;
    let var10 = x as f64 + var8 as f64 * 0.6 + 0.5;
    let var12 = y as f64 + 0.5;
    let var14 = z as f64 + var9 as f64 * 0.6 + 0.5;
    let mut out = Vec::with_capacity(10);
    for _ in 0..10 {
        let var31 = var7.next_double() * 0.2 + 0.01;
        let var19 = var10 + var8 as f64 * 0.01 + (var7.next_double() - 0.5) * var9 as f64 * 0.5;
        let var21 = var12 + (var7.next_double() - 0.5) * 0.5;
        let var23 = var14 + var9 as f64 * 0.01 + (var7.next_double() - 0.5) * var8 as f64 * 0.5;
        let var25 = var8 as f64 * var31 + var7.next_gaussian() * 0.01;
        let var27 = -0.03 + var7.next_gaussian() * 0.01;
        let var29 = var9 as f64 * var31 + var7.next_gaussian() * 0.01;
        out.push((
            DVec3::new(var19, var21, var23),
            DVec3::new(var25, var27, var29),
        ));
    }
    out
}

impl EffectRenderer {
    /// `RenderGlobal.spawnParticle(name, x, y, z, vx, vy, vz)`: nothing
    /// without a view entity (`view` = its `posX/Y/Z`) or out of range,
    /// else the named particle added.
    // src: RenderGlobal.java:1275-1315
    pub fn spawn_particle(&mut self, view: Option<DVec3>, name: &str, p: DVec3, v: DVec3) {
        let Some(view) = view else {
            return;
        };
        if !in_spawn_range(view, p) {
            return;
        }
        let Some(class) = particle_class(name) else {
            return;
        };
        if let Some(fx) = construct(class, p, v, &mut self.fx_random) {
            self.add_effect(fx);
        }
    }

    /// The particle half of `RenderGlobal.playAuxSFX(player, kind, x, y, z,
    /// data)`: 2000 the dispenser's smoke, 2001 the block-break debris
    /// (`data & 255` the block, `data >> 8 & 255` its metadata). The sound
    /// half is `crate::sound::aux_sfx` (WP-3.8).
    // src: RenderGlobal.java:1358-1416
    #[allow(clippy::too_many_arguments)]
    pub fn play_aux_sfx(
        &mut self,
        world: &mut World,
        view: Option<DVec3>,
        kind: i32,
        x: i32,
        y: i32,
        z: i32,
        data: i32,
    ) {
        match kind {
            2000 => {
                for (p, v) in dispenser_smoke(x, y, z, data, &mut self.aux_world_rand) {
                    self.spawn_particle(view, "smoke", p, v);
                }
            }
            // src: RenderGlobal.java:1414
            2001 => self.add_block_destroy_effects(world, x, y, z, data & 255, data >> 8 & 255),
            _ => {}
        }
    }
}

/// What `addRainParticles` reads of the view entity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RainView {
    /// `posX/Y/Z`.
    pub pos: DVec3,
}

/// `EntityRenderer.addRainParticles()` (every unpaused tick, after
/// `updateRenderer`'s counter): with rain strength `s` (halved on fast
/// graphics), `(int)(100 s²)` tries at random columns within 10 blocks;
/// a column within 10 blocks of the viewer's level in a rainy biome gets a
/// smoke puff on lava or a rain splash on anything else, on its top solid
/// block. Then, at random, the rain sound at one of the splashes: quieter
/// and lower when it is above the viewer's head and the viewer is under a
/// roof. `random` is `EntityRenderer.random`, reseeded from the counter
/// (the client's own; it never touches the world's `Random`); the sound is
/// queued on the world as `World.playSoundEffect` does (WP-3.8 plays it).
// src: EntityRenderer.java:580-634
#[allow(clippy::too_many_arguments)]
pub fn add_rain_particles(
    er: &mut EffectRenderer,
    world: &mut World,
    view: RainView,
    fancy_graphics: bool,
    renderer_update_count: i32,
    random: &mut JavaRandom,
    rain_sound_counter: &mut i32,
) {
    let mut var1 = world.get_rain_strength(1.0);
    if !fancy_graphics {
        var1 /= 2.0;
    }
    if var1 == 0.0 {
        return;
    }
    // src: EntityRenderer.java:587 ((long)count * 312987231L)
    random.set_seed((renderer_update_count as i64).wrapping_mul(312987231));
    let var4 = floor_double(view.pos.x);
    let var5 = floor_double(view.pos.y);
    let var6 = floor_double(view.pos.z);
    let var7 = 10;
    let mut var8 = 0.0f64;
    let mut var10 = 0.0f64;
    let mut var12 = 0.0f64;
    let mut var14 = 0;
    let tries = (100.0 * var1 * var1) as i32;
    for _ in 0..tries {
        let var16 = var4 + random.next_int_bounded(var7) - random.next_int_bounded(var7);
        let var17 = var6 + random.next_int_bounded(var7) - random.next_int_bounded(var7);
        let var18 = world.find_top_solid_block(var16, var17);
        let var19 = world.get_block_id(var16, var18 - 1, var17);
        if var18 <= var5 + var7
            && var18 >= var5 - var7
            && world
                .get_world_chunk_manager()
                .get_biome_gen_at(var16, var17)
                .can_spawn_lightning_bolt()
        {
            let var20 = random.next_float();
            let var21 = random.next_float();
            if var19 > 0 {
                let Some(def) = block::block(var19) else {
                    continue;
                };
                // Block.blocksList[var19].minY: the singleton's current bounds.
                let min_y = world.block_statics.bounds[var19 as usize].min_y;
                let px = (var16 as f32 + var20) as f64;
                let py = (var18 as f32 + 0.1) as f64 - min_y;
                let pz = (var17 as f32 + var21) as f64;
                if def.block_material == Material::Lava {
                    // src: EntityRenderer.java:609
                    let fx = EntityFx::smoke(px, py, pz, 0.0, 0.0, 0.0, 1.0, &mut er.fx_random);
                    er.add_effect(fx);
                } else {
                    var14 += 1;
                    if random.next_int_bounded(var14) == 0 {
                        var8 = px;
                        var10 = py;
                        var12 = pz;
                    }
                    // src: EntityRenderer.java:618
                    let fx = EntityFx::rain(px, py, pz, &mut er.fx_random);
                    er.add_effect(fx);
                }
            }
        }
    }
    // src: EntityRenderer.java:624-631
    if var14 > 0 && random.next_int_bounded(3) < post_inc(rain_sound_counter) {
        *rain_sound_counter = 0;
        if var10 > view.pos.y + 1.0
            && world.find_top_solid_block(floor_double(view.pos.x), floor_double(view.pos.z))
                > floor_double(view.pos.y)
        {
            world.play_sound_effect(var8, var10, var12, "ambient.weather.rain", 0.1, 0.5);
        } else {
            world.play_sound_effect(var8, var10, var12, "ambient.weather.rain", 0.2, 1.0);
        }
    }
}

/// `x++`.
fn post_inc(x: &mut i32) -> i32 {
    let v = *x;
    *x = x.wrapping_add(1);
    v
}
