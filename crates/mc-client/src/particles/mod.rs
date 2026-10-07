//! `EffectRenderer` (EffectRenderer.java) and the `EntityFX` family
//! (WP-3.7a): the particles.
//!
//! - [`EffectRenderer`] (a `Resource`, `Minecraft.effectRenderer`): the four
//!   layers (`fxLayers`: 0 `/particles.png`, 1 `/terrain.png`, 2
//!   `/gui/items.png`, 3 drawn by `renderLitParticles`), `addEffect` with
//!   its 4000-per-layer cap, `updateEffects`, `addBlockDestroyEffects`,
//!   `addBlockHitEffects`, `clearEffects`, `getStatistics`.
//! - [`fx`]: `EntityFX` and what it inherits from `Entity`;
//!   [`subclasses`]: every `Entity*FX`; [`spawn`]: `RenderGlobal.spawnParticle`,
//!   the particle halves of `playAuxSFX`, `EntityRenderer.addRainParticles`;
//!   [`render`]: `renderParticles` / `renderLitParticles` as Bevy meshes.
//!
//! # Where particles come from, and when
//!
//! Vanilla adds a particle synchronously, inside the call that asks for it.
//! Here the requests arrive as messages ([`WorldEventMsg`]:
//! `spawnParticle`, `playAuxSFX`; [`ClientEvent`]: `addBlockHitEffects`,
//! the pickup particle, `clearEffects` on a world change). The shell
//! flushes them before each [`RunTickHook`] and at the end of the frame;
//! [`collect_messages`] copies every flush into [`EffectRenderer::pending`]
//! in flush order (the world's events, then the shell's, as flushed), and
//! [`apply_pending`] adds them at the next tick point that touches the
//! particles: [`RunTickHook::UpdateRenderer`] (before `addRainParticles`)
//! and [`RunTickHook::UpdateEffects`] (before `updateEffects`). So every
//! particle of a tick exists before that tick's `updateEffects`, as in
//! vanilla, and none is drawn before its first update (its `prevPos` is 0
//! until then, as in Java). A world change (`clearEffects`) is applied at
//! once even between ticks. Each request still reads the world when it is
//! applied; between the request and the hook the world does not change in
//! the shell's tick order, except that `spawnParticle`'s 16-block range is
//! measured from the view entity's position at the hook (vanilla: at the
//! call; it moves at most one tick's motion).
//!
//! # Randomness
//!
//! `EffectRenderer.rand` (`new Random()`) is [`EffectRenderer::rand`],
//! seeded from the clock. The particles' own streams are [`fx::FxRandom`]
//! (module docs of [`fx`]). `playAuxSFX` 2000's `worldObj.rand` draws come
//! from a client stand-in ([`spawn`] module docs). Nothing here draws from
//! a simulation `Random`, and particles are never counted in
//! `Entity.nextEntityID` (vanilla's `EntityFX` constructor takes an entity
//! id; the oracle, which has no renderer, does not): the simulation's
//! entity ids stay those of the oracle.
//!
//! # For WP-3.5b
//!
//! `EntityPickupFX.renderParticle` draws the picked entity with
//! `RenderManager.renderEntityWithPosYaw`; this module computes its position,
//! yaw and light every frame into [`PickupFxFrame`] and draws nothing for
//! it.

pub mod fx;
pub mod render;
pub mod spawn;
pub mod subclasses;

use bevy::ecs::message::MessageCursor;
use bevy::math::DVec3;
use bevy::prelude::*;
use mc_core::JavaRandom;
use mc_sim::world::World;
use mc_sim::world::client_requests::PickupFx;
use mc_sim::world::events::WorldEvent;

use crate::GameFrameSet;
use crate::clock;
use crate::events::{ClientEvent, WorldEventMsg};
use crate::game::{Game, RunTickHook};
use crate::weather::WeatherRandom;

pub use fx::{Billboard, EntityFx, FxKind, FxRandom};
pub use render::{PickupDraw, PickupFxFrame};

/// `addEffect`'s cap per layer: the oldest particle goes first.
// src: EffectRenderer.java:29-31
pub const MAX_PARTICLES_PER_LAYER: usize = 4000;

/// A particle request waiting for the next tick point (module docs).
#[derive(Clone, Debug)]
pub enum PendingFx {
    /// `RenderGlobal.spawnParticle(name, x, y, z, vx, vy, vz)`.
    Spawn {
        /// Name.
        name: String,
        /// Position.
        pos: DVec3,
        /// Motion arguments.
        motion: DVec3,
    },
    /// `RenderGlobal.playAuxSFX(kind, x, y, z, data)`.
    AuxSfx {
        /// Type.
        kind: i32,
        /// Block.
        x: i32,
        /// Block.
        y: i32,
        /// Block.
        z: i32,
        /// Data.
        data: i32,
    },
    /// `addBlockHitEffects(x, y, z, side)`.
    BlockHit {
        /// Block.
        x: i32,
        /// Block.
        y: i32,
        /// Block.
        z: i32,
        /// Side.
        side: i32,
    },
    /// `addEffect(new EntityPickupFX(world, entity, player, yOffs))`.
    Pickup(PickupFx),
    /// `clearEffects(world)`.
    Clear,
}

/// `EffectRenderer` (module docs).
#[derive(Resource, Debug)]
pub struct EffectRenderer {
    /// `fxLayers`.
    // src: EffectRenderer.java:10
    pub fx_layers: [Vec<EntityFx>; 4],
    /// `rand` (`new Random()`).
    // src: EffectRenderer.java:12
    pub rand: JavaRandom,
    /// The particles' `Math.random()` and their `rand` seeds.
    pub fx_random: FxRandom,
    /// The stand-in for `worldObj.rand` in `playAuxSFX` 2000 ([`spawn`]).
    pub aux_world_rand: JavaRandom,
    /// `EntityFX.interpPosX/Y/Z` (statics): set by `renderParticles` each
    /// frame; `renderLitParticles` runs before it and reads last frame's
    /// (vanilla bug, transcribed: [`render`]).
    // src: EntityFX.java:14-16
    pub interp_pos: DVec3,
    /// Requests not yet applied (module docs).
    pub pending: Vec<PendingFx>,
}

impl Default for EffectRenderer {
    fn default() -> Self {
        EffectRenderer::new(
            JavaRandom::new(clock::new_random_seed()),
            FxRandom::from_clock(),
            JavaRandom::new(clock::new_random_seed()),
        )
    }
}

impl EffectRenderer {
    /// `new EffectRenderer(world, renderEngine)` with the given streams.
    // src: EffectRenderer.java:14-25
    pub fn new(rand: JavaRandom, fx_random: FxRandom, aux_world_rand: JavaRandom) -> Self {
        EffectRenderer {
            fx_layers: Default::default(),
            rand,
            fx_random,
            aux_world_rand,
            interp_pos: DVec3::ZERO,
            pending: Vec::new(),
        }
    }

    /// `addEffect(fx)`: appended to its layer, the layer's oldest removed
    /// first when it holds 4000.
    // src: EffectRenderer.java:27-34
    pub fn add_effect(&mut self, fx: EntityFx) {
        let var2 = fx.get_fx_layer();
        if self.fx_layers[var2].len() >= MAX_PARTICLES_PER_LAYER {
            self.fx_layers[var2].remove(0);
        }
        self.fx_layers[var2].push(fx);
    }

    /// `updateEffects()`: every particle's `onUpdate`, layer by layer in
    /// list order, a dead one removed at once. A particle added during the
    /// loop (the lava's smoke, through `spawnParticle` with the view at
    /// `view`) is updated in the same loop when it lands after the cursor.
    ///
    /// Vanilla bug, transcribed: when that add drops the layer's oldest
    /// particle (the 4000 cap), the list shifts under the loop index, and a
    /// dead updater removes the particle now at its old index — the next
    /// one — while it stays in the list.
    // src: EffectRenderer.java:36-47
    pub fn update_effects(&mut self, world: &mut World, view: Option<DVec3>) {
        for var1 in 0..4 {
            let mut var2 = 0usize;
            while var2 < self.fx_layers[var1].len() {
                let (dead, spawned) = {
                    let fx = &mut self.fx_layers[var1][var2];
                    let s = fx.on_update(world, &mut self.fx_random);
                    (fx.is_dead, s)
                };
                if let Some(s) = spawned {
                    self.spawn_particle(view, s.name, s.pos, s.motion);
                }
                if dead {
                    if var2 < self.fx_layers[var1].len() {
                        self.fx_layers[var1].remove(var2);
                    }
                    // var2-- then ++var2
                } else {
                    var2 += 1;
                }
            }
        }
    }

    /// `clearEffects(world)`: every layer emptied.
    // src: EffectRenderer.java:102-109
    pub fn clear_effects(&mut self) {
        for l in &mut self.fx_layers {
            l.clear();
        }
    }

    /// `addBlockDestroyEffects(x, y, z, blockId, meta)`: 4 × 4 × 4 digging
    /// particles through the block, each moving out from its centre, with
    /// a random `nextInt(6)` side. Nothing for air (or an id without a
    /// block, where Java would throw).
    // src: EffectRenderer.java:111-129
    pub fn add_block_destroy_effects(
        &mut self,
        world: &mut World,
        x: i32,
        y: i32,
        z: i32,
        var4: i32,
        var5: i32,
    ) {
        if var4 == 0 || mc_sim::block::block(var4).is_none() {
            return;
        }
        let var7 = 4;
        for var8 in 0..var7 {
            for var9 in 0..var7 {
                for var10 in 0..var7 {
                    let var11 = x as f64 + (var8 as f64 + 0.5) / var7 as f64;
                    let var13 = y as f64 + (var9 as f64 + 0.5) / var7 as f64;
                    let var15 = z as f64 + (var10 as f64 + 0.5) / var7 as f64;
                    let var17 = self.rand.next_int_bounded(6);
                    if let Some(fx) = EntityFx::digging(
                        world,
                        var11,
                        var13,
                        var15,
                        var11 - x as f64 - 0.5,
                        var13 - y as f64 - 0.5,
                        var15 - z as f64 - 0.5,
                        var4,
                        var17,
                        var5,
                        &mut self.fx_random,
                    ) {
                        let fx = fx.apply_colour_multiplier(world, x, y, z);
                        self.add_effect(fx);
                    }
                }
            }
        }
    }

    /// `addBlockHitEffects(x, y, z, side)`: one digging particle just
    /// outside the face `side` of the block (its current bounds, inset by
    /// 0.1 on the other axes), slowed to 0.2 and scaled 0.6. Nothing for
    /// air.
    // src: EffectRenderer.java:131-165
    pub fn add_block_hit_effects(&mut self, world: &mut World, x: i32, y: i32, z: i32, var4: i32) {
        let var5 = world.get_block_id(x, y, z);
        if var5 == 0 || mc_sim::block::block(var5).is_none() {
            return;
        }
        // Block.blocksList[var5].minX…: the singleton's current bounds.
        let b = world.block_statics.bounds[var5 as usize];
        let var7 = 0.1f32;
        let mut var8 = x as f64
            + self.rand.next_double() * (b.max_x - b.min_x - (var7 * 2.0) as f64)
            + var7 as f64
            + b.min_x;
        let mut var10 = y as f64
            + self.rand.next_double() * (b.max_y - b.min_y - (var7 * 2.0) as f64)
            + var7 as f64
            + b.min_y;
        let mut var12 = z as f64
            + self.rand.next_double() * (b.max_z - b.min_z - (var7 * 2.0) as f64)
            + var7 as f64
            + b.min_z;
        match var4 {
            0 => var10 = y as f64 + b.min_y - var7 as f64,
            1 => var10 = y as f64 + b.max_y + var7 as f64,
            2 => var12 = z as f64 + b.min_z - var7 as f64,
            3 => var12 = z as f64 + b.max_z + var7 as f64,
            4 => var8 = x as f64 + b.min_x - var7 as f64,
            5 => var8 = x as f64 + b.max_x + var7 as f64,
            _ => {}
        }
        let meta = world.get_block_metadata(x, y, z);
        if let Some(fx) = EntityFx::digging(
            world,
            var8,
            var10,
            var12,
            0.0,
            0.0,
            0.0,
            var5,
            var4,
            meta,
            &mut self.fx_random,
        ) {
            let fx = fx
                .apply_colour_multiplier(world, x, y, z)
                .multiply_velocity(0.2)
                .multiple_particle_scale_by(0.6);
            self.add_effect(fx);
        }
    }

    /// `getStatistics()`: the particle count of layers 0-2 (F3's `P:`).
    // src: EffectRenderer.java:167-169
    pub fn get_statistics(&self) -> String {
        (self.fx_layers[0].len() + self.fx_layers[1].len() + self.fx_layers[2].len()).to_string()
    }

    /// Applies the pending requests in order ([`apply_pending`]).
    pub fn apply_pending(&mut self, world: &mut World, view: Option<DVec3>) {
        for p in std::mem::take(&mut self.pending) {
            match p {
                PendingFx::Spawn { name, pos, motion } => {
                    self.spawn_particle(view, &name, pos, motion)
                }
                PendingFx::AuxSfx {
                    kind,
                    x,
                    y,
                    z,
                    data,
                } => self.play_aux_sfx(world, view, kind, x, y, z, data),
                PendingFx::BlockHit { x, y, z, side } => {
                    self.add_block_hit_effects(world, x, y, z, side)
                }
                // src: EntityPlayerSP.java:133-135
                PendingFx::Pickup(p) => {
                    let fx = EntityFx::pickup(p.picked, p.player_id, p.y_offs, &mut self.fx_random);
                    self.add_effect(fx);
                }
                PendingFx::Clear => self.clear_effects(),
            }
        }
    }

    /// Takes the requests up to the last `clearEffects` out of the queue
    /// and clears (vanilla would have added them and then cleared them).
    pub fn apply_pending_clear(&mut self) {
        if let Some(i) = self
            .pending
            .iter()
            .rposition(|p| matches!(p, PendingFx::Clear))
        {
            self.pending.drain(..=i);
            self.clear_effects();
        }
    }
}

/// The message cursors of [`collect_messages`] (shared by every schedule
/// it runs in).
#[derive(Resource, Default)]
pub struct FxMessageCursors {
    world: MessageCursor<WorldEventMsg>,
    client: MessageCursor<ClientEvent>,
}

/// Copies the newly flushed messages into [`EffectRenderer::pending`]: the
/// world's events, then the shell's (their order within a flush).
pub fn collect_messages(
    mut er: ResMut<EffectRenderer>,
    mut cursors: ResMut<FxMessageCursors>,
    world_msgs: Res<Messages<WorldEventMsg>>,
    client_msgs: Res<Messages<ClientEvent>>,
) {
    let FxMessageCursors { world, client } = &mut *cursors;
    for WorldEventMsg(e) in world.read(&world_msgs) {
        match e {
            WorldEvent::SpawnParticle {
                name,
                x,
                y,
                z,
                vx,
                vy,
                vz,
            } => er.pending.push(PendingFx::Spawn {
                name: name.to_string(),
                pos: DVec3::new(*x, *y, *z),
                motion: DVec3::new(*vx, *vy, *vz),
            }),
            WorldEvent::PlayAuxSfx {
                kind,
                x,
                y,
                z,
                data,
                ..
            } if matches!(kind, 2000 | 2001) => er.pending.push(PendingFx::AuxSfx {
                kind: *kind,
                x: *x,
                y: *y,
                z: *z,
                data: *data,
            }),
            _ => {}
        }
    }
    for e in client.read(&client_msgs) {
        match e {
            ClientEvent::BlockHitEffects { x, y, z, side } => {
                er.pending.push(PendingFx::BlockHit {
                    x: *x,
                    y: *y,
                    z: *z,
                    side: *side,
                })
            }
            ClientEvent::PickupFx(p) => er.pending.push(PendingFx::Pickup(p.clone())),
            // src: Minecraft.java:1308-1310
            ClientEvent::WorldChanged { .. } => er.pending.push(PendingFx::Clear),
            _ => {}
        }
    }
}

/// The view entity's `posX/Y/Z` (`mc.renderViewEntity`), if any.
pub fn view_pos(game: &Game) -> Option<DVec3> {
    game.render_view()
        .map(|e| DVec3::new(e.base.pos_x, e.base.pos_y, e.base.pos_z))
}

/// Applies the pending requests at a tick point (module docs).
pub fn apply_pending(mut game: NonSendMut<Game>, mut er: ResMut<EffectRenderer>) {
    let view = view_pos(&game);
    match game.the_world.as_mut() {
        Some(w) => er.apply_pending(w, view),
        None => er.apply_pending_clear(),
    }
}

/// At the end of the frame: a world change clears at once; the rest waits
/// for the next tick point.
pub fn apply_pending_clear(mut er: ResMut<EffectRenderer>) {
    er.apply_pending_clear();
}

/// `effectRenderer.updateEffects()` (the end of an unpaused `runTick`).
// src: Minecraft.java:1172-1174
pub fn update_effects(mut game: NonSendMut<Game>, mut er: ResMut<EffectRenderer>) {
    let view = view_pos(&game);
    if let Some(w) = game.the_world.as_mut() {
        er.update_effects(w, view);
    }
}

/// `EntityRenderer.rainSoundCounter`.
// src: EntityRenderer.java:44
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct RainSoundCounter(pub i32);

/// `entityRenderer.updateRenderer()`'s `addRainParticles()` (after the
/// counter, EntityRenderer.java:74-76), with `EntityRenderer.random`
/// (WP-3.4's [`WeatherRandom`]: the same `Random` `renderRainSnow` reseeds).
// src: EntityRenderer.java:76
pub fn add_rain_particles(
    mut game: NonSendMut<Game>,
    mut er: ResMut<EffectRenderer>,
    mut random: ResMut<WeatherRandom>,
    mut counter: ResMut<RainSoundCounter>,
) {
    let Some(pos) = view_pos(&game) else {
        return;
    };
    let fancy = game.game_settings.fancy_graphics;
    let count = game.entity_renderer.renderer_update_count;
    let Some(w) = game.the_world.as_mut() else {
        return;
    };
    spawn::add_rain_particles(
        &mut er,
        w,
        spawn::RainView { pos },
        fancy,
        count,
        &mut random.0,
        &mut counter.0,
    );
}

/// Registers WP-3.7a: the resources, the tick systems and the renderer.
pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EffectRenderer>()
            .init_resource::<FxMessageCursors>()
            .init_resource::<RainSoundCounter>()
            .init_resource::<WeatherRandom>()
            .init_resource::<PickupFxFrame>();
        for hook in RunTickHook::ALL {
            match hook {
                RunTickHook::UpdateRenderer => {
                    app.add_systems(
                        hook,
                        (collect_messages, apply_pending, add_rain_particles).chain(),
                    );
                }
                RunTickHook::UpdateEffects => {
                    app.add_systems(
                        hook,
                        (collect_messages, apply_pending, update_effects).chain(),
                    );
                }
                _ => {
                    app.add_systems(hook, collect_messages);
                }
            }
        }
        app.add_systems(
            Update,
            (collect_messages, apply_pending_clear)
                .chain()
                .after(GameFrameSet),
        );
        render::build(app);
    }
}

#[cfg(test)]
mod tests;
