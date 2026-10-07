//! Tests of WP-3.7a. Values marked "JVM" were printed by a hand
//! transcription of the Java (EntityFX's constructor and one `onUpdate` in
//! free space, `playAuxSFX` 2000, `addRainParticles`' reseed) run on JDK 25
//! with the same seeded `java.util.Random`s, `Double.toString`.

use bevy::math::DVec3;
use mc_core::JavaRandom;
use mc_sim::block;
use mc_sim::save::NoSave;
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

use super::fx::{Billboard, EntityFx, FxKind, FxRandom, particle_uv, sub_tile_uv};
use super::spawn::{self, ParticleClass, dispenser_smoke, in_spawn_range, particle_class};
use super::subclasses::{grow_in, note_colour, smoke_frame};
use super::{EffectRenderer, MAX_PARTICLES_PER_LAYER, PendingFx};
use crate::gl_material::TessBatch;

fn world() -> World {
    World::new(
        Box::new(NoSave),
        "particles",
        WorldProvider::new(Dimension::Surface),
        123,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    )
}

fn renderer(seed: i64) -> EffectRenderer {
    EffectRenderer::new(
        JavaRandom::new(seed),
        FxRandom::seeded(seed),
        JavaRandom::new(seed + 2),
    )
}

fn d(s: &str) -> f64 {
    s.parse().unwrap()
}

#[test]
fn particle_uv_is_the_tile_just_under_a_sixteenth() {
    // src: EntityFX.java:80-83
    assert_eq!(particle_uv(0), [0.0, 0.0624375, 0.0, 0.0624375]);
    let [u0, u1, v0, v1] = particle_uv(49);
    assert_eq!((u0, v0), (1.0 / 16.0, 3.0 / 16.0));
    assert_eq!(u1, 1.0f32 / 16.0 + 0.0624375);
    assert_eq!(v1, 3.0f32 / 16.0 + 0.0624375);
    // Smoke frame 7, the heart 80 (row 5), the note 64 (row 4).
    assert_eq!(particle_uv(7)[0], 7.0 / 16.0);
    assert_eq!(particle_uv(80)[2], 5.0 / 16.0);
    assert_eq!(particle_uv(64)[2], 4.0 / 16.0);
    // Java's truncating % and / on a negative (mirrored) index.
    assert_eq!(particle_uv(-3)[0], -3.0 / 16.0);
    assert_eq!(particle_uv(-3)[2], 0.0);
}

#[test]
fn digging_uv_is_a_jittered_quarter_tile() {
    // src: EntityDiggingFX.java:34-37: stone (1) with jitter (2, 1).
    let [u0, u1, v0, v1] = sub_tile_uv(1, 2.0, 1.0);
    assert_eq!(u0, (1.0 + 0.5) / 16.0);
    assert_eq!(u1, (1.0f32 + 0.5) / 16.0 + 0.015609375);
    assert_eq!(v0, 0.25 / 16.0);
    assert_eq!(v1, 0.25f32 / 16.0 + 0.015609375);
    // Row 2 (the grass side 3 is row 0; ores at 32+): index 33.
    assert_eq!(sub_tile_uv(33, 0.0, 0.0)[2], 2.0 / 16.0);
}

#[test]
fn constructor_matches_the_jvm() {
    let mut rnd = FxRandom::seeded(42);
    let e = EntityFx::new(10.5, 70.25, -3.75, 0.0, 0.0, 0.0, &mut rnd);
    // JVM: motion, jitter, scale, maxAge
    assert_eq!(e.motion_x, d("0.07595615261086523"));
    assert_eq!(e.motion_y, d("0.1611562883522007"));
    assert_eq!(e.motion_z, d("-0.06384557600135185"));
    assert_eq!(e.particle_texture_jitter_x, 1.6602554);
    assert_eq!(e.particle_texture_jitter_y, 2.9492407);
    assert_eq!(e.particle_scale, 1.8685088);
    // the age formula: (int)(4 / (nextFloat * 0.9 + 0.1))
    assert_eq!(e.particle_max_age, 9);
    assert_eq!(e.particle_age, 0);
    // size 0.2, yOffset 0.1, the box around the feet at y - 0.1
    assert_eq!((e.width, e.height, e.y_offset), (0.2, 0.2, 0.1));
    assert_eq!(e.bounding_box.min_x, 10.5 - 0.1f32 as f64);
    assert_eq!(e.bounding_box.min_y, 70.25 - 0.1f32 as f64);
    assert_eq!(e.bounding_box.max_y, 70.25 - 0.1f32 as f64 + 0.2f32 as f64);
    assert_eq!((e.prev_pos_x, e.prev_pos_y, e.prev_pos_z), (0.0, 0.0, 0.0));
    assert_eq!(e.get_fx_layer(), 0);
}

#[test]
fn age_formula_under_a_seeded_random() {
    // (int)(4.0F / (rand.nextFloat() * 0.9F + 0.1F)) on the particle's
    // fourth nextFloat (EntityFX.java:35).
    let mut rnd = FxRandom::seeded(7);
    let e = EntityFx::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &mut rnd);
    let mut seeds = JavaRandom::new(8);
    let mut r = JavaRandom::new(seeds.next_long());
    for _ in 0..3 {
        r.next_float();
    }
    let f = r.next_float();
    assert_eq!(e.particle_max_age, (4.0 / (f * 0.9 + 0.1)) as i32);
    assert!((4..=40).contains(&e.particle_max_age));
}

#[test]
fn one_update_in_free_space_matches_the_jvm() {
    let mut w = world();
    let mut rnd = FxRandom::seeded(42);
    let mut e = EntityFx::new(10.5, 70.25, -3.75, 0.0, 0.0, 0.0, &mut rnd);
    e.particle_gravity = 1.0;
    assert_eq!(e.on_update(&mut w, &mut rnd), None);
    // JVM: gravity 0.04, the box moved y, x, z, pos from the box, × 0.98
    assert_eq!(e.pos_x, d("10.575956152610866"));
    assert_eq!(e.pos_y, d("70.3711562883522"));
    assert_eq!(e.pos_z, d("-3.8138455760013517"));
    assert_eq!(e.motion_x, d("0.07443703100739657"));
    assert_eq!(e.motion_y, d("0.11873316489602947"));
    assert_eq!(e.motion_z, d("-0.06256866569908254"));
    assert_eq!((e.prev_pos_x, e.prev_pos_y), (10.5, 70.25));
    assert_eq!(e.particle_age, 1);
    assert!(!e.on_ground && !e.is_dead);
}

#[test]
fn a_falling_particle_lands_and_slides() {
    let mut w = world();
    w.set_block(0, 64, 0, block::ids::STONE);
    let mut rnd = FxRandom::seeded(3);
    let mut e = EntityFx::new(0.5, 65.15, 0.5, 0.0, 0.0, 0.0, &mut rnd);
    e.motion_x = 0.1;
    e.motion_y = -0.5;
    e.motion_z = 0.0;
    e.particle_gravity = 1.0;
    e.on_update(&mut w, &mut rnd);
    // The box bottom (65.05) stops on the stone top (65): posY = 65 + 0.1.
    assert_eq!(e.bounding_box.min_y, 65.0);
    assert_eq!(e.pos_y, 65.0 + 0.1f32 as f64);
    assert!(e.on_ground);
    assert_eq!(e.motion_y, 0.0);
    // x: moved, then × 0.98 × 0.7 on the ground.
    assert_eq!(e.motion_x, 0.1 * 0.9800000190734863 * 0.699999988079071);
}

#[test]
fn ageing_kills_after_max_age_updates() {
    let mut w = world();
    let mut rnd = FxRandom::seeded(5);
    let mut e = EntityFx::new(0.5, 100.0, 0.5, 0.0, 0.0, 0.0, &mut rnd);
    e.particle_max_age = 3;
    for _ in 0..3 {
        e.on_update(&mut w, &mut rnd);
        assert!(!e.is_dead);
    }
    // particleAge++ >= particleMaxAge on the fourth update
    e.on_update(&mut w, &mut rnd);
    assert!(e.is_dead);
}

#[test]
fn smoke_frames_and_grow_in() {
    // 7 - age * 8 / max: frames 7 → 0 over the life (EntitySmokeFX.java:49).
    let frames: Vec<i32> = (0..=8).map(|a| smoke_frame(a, 8)).collect();
    assert_eq!(frames, vec![7, 6, 5, 4, 3, 2, 1, 0, -1]);
    assert_eq!(smoke_frame(1, 0), 7);
    // (age + partial) / max · 32, clamped (EntitySmokeFX.java:28-35).
    assert_eq!(grow_in(0, 8, 0.0), 0.0);
    assert_eq!(grow_in(0, 64, 0.5), 0.25);
    assert_eq!(grow_in(1, 8, 0.0), 1.0);
}

#[test]
fn spawn_name_table() {
    use ParticleClass as P;
    for (n, c) in [
        ("bubble", P::Bubble),
        ("smoke", P::Smoke),
        ("note", P::Note),
        ("portal", P::Portal),
        ("explode", P::Explode),
        ("flame", P::Flame),
        ("lava", P::Lava),
        ("footstep", P::Footstep),
        ("splash", P::Splash),
        ("largesmoke", P::LargeSmoke),
        ("reddust", P::Reddust),
        ("snowballpoof", P::SnowballPoof),
        ("snowshovel", P::SnowShovel),
        ("slime", P::Slime),
        ("heart", P::Heart),
    ] {
        assert_eq!(particle_class(n), Some(c), "{n}");
    }
    // Not in Beta 1.7.3.
    for n in ["hugeexplosion", "largeexplode", "tilecrack_1", "Smoke", ""] {
        assert_eq!(particle_class(n), None, "{n}");
    }
}

#[test]
fn spawned_classes_layers_and_textures() {
    let mut rnd = FxRandom::seeded(11);
    let p = DVec3::new(0.5, 70.0, 0.5);
    let z = DVec3::ZERO;
    let mk = |c, rnd: &mut FxRandom| spawn::construct(c, p, z, rnd).unwrap();
    let flame = mk(ParticleClass::Flame, &mut rnd);
    assert_eq!((flame.particle_texture_index, flame.no_clip), (48, true));
    let lava = mk(ParticleClass::Lava, &mut rnd);
    assert_eq!(lava.particle_texture_index, 49);
    let heart = mk(ParticleClass::Heart, &mut rnd);
    assert_eq!(
        (heart.particle_texture_index, heart.particle_max_age),
        (80, 16)
    );
    let note = mk(ParticleClass::Note, &mut rnd);
    assert_eq!(
        (note.particle_texture_index, note.particle_max_age),
        (64, 6)
    );
    let bubble = mk(ParticleClass::Bubble, &mut rnd);
    assert_eq!(bubble.particle_texture_index, 32);
    let rain = EntityFx::rain(0.5, 70.0, 0.5, &mut rnd);
    assert!((19..23).contains(&rain.particle_texture_index));
    assert_eq!(rain.particle_gravity, 0.06);
    let splash = mk(ParticleClass::Splash, &mut rnd);
    assert!((20..24).contains(&splash.particle_texture_index));
    assert_eq!(splash.particle_gravity, 0.04);
    let portal = mk(ParticleClass::Portal, &mut rnd);
    assert!((0..8).contains(&portal.particle_texture_index));
    assert!((40..50).contains(&portal.particle_max_age));
    assert_eq!(portal.particle_red, portal.particle_blue * 0.9);
    // snowballpoof / slime: items.png icons 14 + 0·16 and 14 + 1·16
    // (Item.java:350, :359), layer 2.
    let snow = mk(ParticleClass::SnowballPoof, &mut rnd);
    assert_eq!((snow.particle_texture_index, snow.get_fx_layer()), (14, 2));
    let slime = mk(ParticleClass::Slime, &mut rnd);
    assert_eq!(slime.particle_texture_index, 30);
    let large = mk(ParticleClass::LargeSmoke, &mut rnd);
    let FxKind::Smoke {
        smoke_particle_scale,
    } = large.kind
    else {
        panic!("smoke")
    };
    assert_eq!(smoke_particle_scale, large.particle_scale);
    let foot = mk(ParticleClass::Footstep, &mut rnd);
    assert_eq!(foot.get_fx_layer(), 3);
    // reddust: the motion is the colour; red 0 becomes 1.
    let red = spawn::construct(
        ParticleClass::Reddust,
        p,
        DVec3::new(0.0, 0.0, 0.0),
        &mut rnd,
    )
    .unwrap();
    assert!(red.particle_red > 0.4 && red.particle_green == 0.0 && red.particle_blue == 0.0);
}

#[test]
fn note_colours_cycle_with_the_pitch() {
    // sin(0) · 0.65 + 0.35 for red at note 0 (MathHelper's table).
    let (r, g, b) = note_colour(0.0);
    assert_eq!(r, 0.35);
    assert!(g > 0.9 && b < 0.0, "{g} {b}");
    // A full turn later the same colour.
    assert_eq!(
        note_colour(1.0).0,
        mc_core::math::sin(3.1415927 * 2.0) * 0.65 + 0.35
    );
}

#[test]
fn spawn_range_is_sixteen_blocks() {
    let v = DVec3::new(0.0, 64.0, 0.0);
    assert!(in_spawn_range(v, DVec3::new(16.0, 64.0, 0.0)));
    assert!(!in_spawn_range(v, DVec3::new(16.0, 64.0, 0.01)));
    let mut er = renderer(1);
    er.spawn_particle(Some(v), "smoke", DVec3::new(20.0, 64.0, 0.0), DVec3::ZERO);
    er.spawn_particle(None, "smoke", DVec3::new(1.0, 64.0, 0.0), DVec3::ZERO);
    er.spawn_particle(Some(v), "nope", DVec3::new(1.0, 64.0, 0.0), DVec3::ZERO);
    assert_eq!(er.get_statistics(), "0");
    er.spawn_particle(Some(v), "smoke", DVec3::new(1.0, 64.0, 0.0), DVec3::ZERO);
    er.spawn_particle(
        Some(v),
        "snowballpoof",
        DVec3::new(1.0, 64.0, 0.0),
        DVec3::ZERO,
    );
    er.spawn_particle(Some(v), "footstep", DVec3::new(1.0, 64.0, 0.0), DVec3::ZERO);
    // getStatistics counts layers 0-2 only.
    assert_eq!(er.get_statistics(), "2");
    assert_eq!(er.fx_layers[3].len(), 1);
}

#[test]
fn dispenser_smoke_matches_the_jvm() {
    let mut r = JavaRandom::new(7);
    let s = dispenser_smoke(0, 64, 0, 2, &mut r);
    assert_eq!(s.len(), 10);
    // JVM, the first two puffs
    assert_eq!(
        s[0].0,
        DVec3::new(
            d("0.9854151984331836"),
            d("64.42415485151562"),
            d("0.08863857137105237")
        )
    );
    assert_eq!(
        s[0].1,
        DVec3::new(
            d("0.1694945450943238"),
            d("-0.0394997893726461"),
            d("-0.1501493094902301")
        )
    );
    assert_eq!(
        s[1].0,
        DVec3::new(
            d("1.123512696395698"),
            d("64.3772108823011"),
            d("-0.16868303310750293")
        )
    );
    assert_eq!(
        s[1].1,
        DVec3::new(
            d("0.13983926655854"),
            d("-0.02305340474800147"),
            d("-0.13498023003318482")
        )
    );
}

#[test]
fn rain_reseeds_from_the_update_count() {
    // setSeed((long)rendererUpdateCount * 312987231L); JVM for count 100.
    let mut r = JavaRandom::new(0);
    r.set_seed(100i64.wrapping_mul(312987231));
    let v: Vec<i32> = (0..4).map(|_| r.next_int_bounded(10)).collect();
    assert_eq!(v, vec![0, 3, 5, 1]);
}

#[test]
fn rain_splashes_land_on_the_ground_when_it_rains() {
    let mut w = world();
    for x in -12..12 {
        for z in -12..12 {
            w.set_block(x, 63, z, block::ids::STONE);
        }
    }
    let mut er = renderer(9);
    let mut random = JavaRandom::new(0);
    let mut counter = 0;
    let view = spawn::RainView {
        pos: DVec3::new(0.5, 65.62, 0.5),
    };
    // No rain: nothing.
    spawn::add_rain_particles(&mut er, &mut w, view, true, 1, &mut random, &mut counter);
    assert!(er.fx_layers[0].is_empty());
    w.set_rain_strength(1.0);
    spawn::add_rain_particles(&mut er, &mut w, view, true, 1, &mut random, &mut counter);
    // (int)(100 · 1²) tries; columns in rainy biomes get a splash on the
    // stone (y 64 + 0.1 - minY 0).
    let n = er.fx_layers[0].len();
    assert!(n > 0 && n <= 100, "{n}");
    for fx in &er.fx_layers[0] {
        assert!(matches!(fx.kind, FxKind::Rain));
        assert_eq!(fx.pos_y, (64.0f32 + 0.1) as f64);
    }
}

#[test]
fn add_effect_keeps_at_most_4000_per_layer() {
    let mut er = renderer(2);
    let mut rnd = FxRandom::seeded(2);
    for i in 0..MAX_PARTICLES_PER_LAYER + 5 {
        let mut fx = EntityFx::new(i as f64, 0.0, 0.0, 0.0, 0.0, 0.0, &mut rnd);
        fx.particle_max_age = i as i32;
        er.add_effect(fx);
    }
    assert_eq!(er.fx_layers[0].len(), MAX_PARTICLES_PER_LAYER);
    // The oldest five went first.
    assert_eq!(er.fx_layers[0][0].particle_max_age, 5);
}

#[test]
fn update_effects_drops_the_dead() {
    let mut w = world();
    let mut er = renderer(4);
    let mut rnd = FxRandom::seeded(4);
    for age in [0, 5, 0] {
        let mut fx = EntityFx::new(0.5, 100.0, 0.5, 0.0, 0.0, 0.0, &mut rnd);
        fx.particle_age = age;
        fx.particle_max_age = 3;
        er.add_effect(fx);
    }
    er.update_effects(&mut w, None);
    assert_eq!(er.fx_layers[0].len(), 2);
    assert!(er.fx_layers[0].iter().all(|f| f.particle_age == 1));
    er.clear_effects();
    assert_eq!(er.get_statistics(), "0");
}

#[test]
fn block_destroy_effects_are_4x4x4_digging_particles() {
    let mut w = world();
    let mut er = renderer(6);
    er.add_block_destroy_effects(&mut w, 2, 64, 3, block::ids::STONE, 0);
    assert_eq!(er.fx_layers[1].len(), 64);
    let fx = &er.fx_layers[1][0];
    // The first at (x + 0.125, y + 0.125, z + 0.125), stone's texture 1,
    // grey 0.6 (stone has no tint), gravity 1, half the base scale.
    assert_eq!((fx.pos_x, fx.pos_y, fx.pos_z), (2.125, 64.125, 3.125));
    assert_eq!(fx.particle_texture_index, 1);
    assert_eq!(fx.particle_red, 0.6);
    assert_eq!(fx.particle_gravity, 1.0);
    assert!(fx.particle_scale <= 1.0);
    let FxKind::Digging { block_id, side } = fx.kind else {
        panic!("digging")
    };
    assert_eq!(block_id, block::ids::STONE);
    assert!((0..6).contains(&side));
    // Air: nothing.
    er.add_block_destroy_effects(&mut w, 2, 64, 3, 0, 0);
    assert_eq!(er.fx_layers[1].len(), 64);
}

#[test]
fn block_hit_effects_sit_just_outside_the_face() {
    let mut w = world();
    w.set_block(0, 64, 0, block::ids::STONE);
    let mut er = renderer(8);
    // Top face (side 1): y = 64 + maxY 1 + 0.1.
    er.add_block_hit_effects(&mut w, 0, 64, 0, 1);
    // East face (side 5): x = 0 + 1 + 0.1.
    er.add_block_hit_effects(&mut w, 0, 64, 0, 5);
    // Air: nothing.
    er.add_block_hit_effects(&mut w, 0, 65, 0, 1);
    assert_eq!(er.fx_layers[1].len(), 2);
    let top = &er.fx_layers[1][0];
    assert_eq!(top.pos_y, 64.0 + 1.0 + 0.1f32 as f64);
    assert!(top.pos_x > 0.1 && top.pos_x < 0.9);
    let east = &er.fx_layers[1][1];
    assert_eq!(east.pos_x, 0.0 + 1.0 + 0.1f32 as f64);
    // multipleParticleScaleBy(0.6): the size 0.12.
    assert_eq!((top.width, top.height), (0.2 * 0.6, 0.2 * 0.6));
}

#[test]
fn multiply_velocity_scales_around_the_lift() {
    let mut rnd = FxRandom::seeded(1);
    let mut e = EntityFx::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &mut rnd);
    e.motion_x = 1.0;
    e.motion_y = 0.5;
    e.motion_z = -1.0;
    let e = e.multiply_velocity(0.2);
    assert_eq!(e.motion_x, 0.2f32 as f64);
    assert_eq!(
        e.motion_y,
        (0.5 - 0.10000000149011612) * 0.2f32 as f64 + 0.10000000149011612
    );
    assert_eq!(e.motion_z, -(0.2f32 as f64));
}

#[test]
fn brightness_reads_two_thirds_up_the_box() {
    let mut rnd = FxRandom::seeded(1);
    let mut e = EntityFx::new(3.5, 70.25, -0.5, 0.0, 0.0, 0.0, &mut rnd);
    // floor(70.25 - 0.1 + 0.2 · 0.66)
    assert_eq!(e.brightness_sample_pos(), (3, 70, -1));
    e.set_position(3.5, 70.97, -0.5);
    assert_eq!(e.brightness_sample_pos(), (3, 71, -1));
    // Chunks not loaded: entityBrightness (0).
    let mut w = world();
    assert_eq!(e.get_entity_brightness(&mut w, 0.0), 0.0);
    // Loaded: the light brightness of that block.
    let l = w.get_light_brightness(3, 71, -1);
    assert_eq!(e.get_entity_brightness(&mut w, 0.0), l);
    // The lava particle is always full bright; the flame fades to full.
    let mut rnd = FxRandom::seeded(2);
    let lava = EntityFx::lava(3.5, 70.97, -0.5, &mut rnd);
    assert_eq!(lava.get_entity_brightness(&mut w, 0.0), 1.0);
}

#[test]
fn the_billboard_and_the_quad() {
    // yaw 0, pitch 0: the quad spans x (rot_x) and y (rot_xz).
    let bb = Billboard::from_rotation(0.0, 0.0);
    assert_eq!((bb.rot_x, bb.rot_xz, bb.rot_z), (1.0, 1.0, 0.0));
    assert_eq!((bb.rot_yz, bb.rot_xy), (0.0, 0.0));
    let mut rnd = FxRandom::seeded(1);
    let mut e = EntityFx::new(1.0, 2.0, 3.0, 0.0, 0.0, 0.0, &mut rnd);
    e.prev_pos_x = 1.0;
    e.prev_pos_y = 2.0;
    e.prev_pos_z = 3.0;
    e.particle_scale = 2.0;
    e.particle_texture_index = 0;
    e.particle_red = 0.5;
    let mut b = TessBatch::default();
    e.render_base_quad(&mut b, 0.5, &bb, DVec3::new(1.0, 1.0, 1.0), 1.0);
    // Half size 0.1 · 2 = 0.2, around (0, 1, 2) relative to interpPos.
    assert_eq!(
        b.positions,
        vec![
            [-0.2, 0.8, 2.0],
            [-0.2, 1.2, 2.0],
            [0.2, 1.2, 2.0],
            [0.2, 0.8, 2.0]
        ]
    );
    assert_eq!(b.uvs[0], [0.0624375, 0.0624375]);
    assert_eq!(b.uvs[2], [0.0, 0.0]);
    // setColorOpaque_F: bytes (int)(0.5 · 255) = 127.
    assert_eq!(b.colors[0], [127.0 / 255.0, 1.0, 1.0, 1.0]);
}

#[test]
fn pending_clear_drops_what_came_before() {
    let mut er = renderer(3);
    let mut rnd = FxRandom::seeded(3);
    er.add_effect(EntityFx::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &mut rnd));
    er.pending.push(PendingFx::BlockHit {
        x: 0,
        y: 0,
        z: 0,
        side: 0,
    });
    er.pending.push(PendingFx::Clear);
    er.pending.push(PendingFx::AuxSfx {
        kind: 2000,
        x: 0,
        y: 64,
        z: 0,
        data: 4,
    });
    er.apply_pending_clear();
    assert_eq!(er.get_statistics(), "0");
    assert_eq!(er.pending.len(), 1);
    // Then the dispenser smoke lands within range of a view at the block.
    let mut w = world();
    er.apply_pending(&mut w, Some(DVec3::new(0.5, 64.5, 0.5)));
    assert_eq!(er.fx_layers[0].len(), 10);
    assert!(er.pending.is_empty());
}

/// Colour maps with one colour each (grass 0x336699, foliage 0x55aa11).
fn flat_colorizers() -> crate::textures::Colorizers {
    crate::textures::Colorizers {
        water: vec![0; 65536].into(),
        grass: vec![0x336699; 65536].into(),
        foliage: vec![0x55aa11; 65536].into(),
    }
}

/// The 0.6 grey of a digging particle times a multiplier's channels.
fn tinted(m: i32) -> (f32, f32, f32) {
    (
        0.6 * ((m >> 16 & 255) as f32 / 255.0),
        0.6 * ((m >> 8 & 255) as f32 / 255.0),
        0.6 * ((m & 255) as f32 / 255.0),
    )
}

fn rgb(fx: &EntityFx) -> (f32, f32, f32) {
    (fx.particle_red, fx.particle_green, fx.particle_blue)
}

#[test]
fn destroy_effects_tint_with_the_event_metadata() {
    // sendBlockRemoved: playAuxSFX(2001, id + meta * 256), then the block
    // becomes air (PlayerController.java:22-26); the debris is applied
    // after that and must still see the tall grass's metadata.
    let mut w = world();
    let mut er = renderer(10);
    er.colorizers = flat_colorizers();
    let (x, y, z) = (4, 64, 7);
    w.set_block_and_metadata(x, y, z, block::ids::TALL_GRASS, 1);
    w.set_block(x, y, z, 0);
    er.play_aux_sfx(&mut w, None, 2001, x, y, z, block::ids::TALL_GRASS + 256);
    assert_eq!(er.fx_layers[1].len(), 64);
    assert!(er.fx_layers[1].iter().all(|f| rgb(f) == tinted(0x336699)));
    // Meta 0 (the dead shrub): white, the 0.6 grey.
    er.clear_effects();
    er.play_aux_sfx(&mut w, None, 2001, x, y, z, block::ids::TALL_GRASS);
    assert!(er.fx_layers[1].iter().all(|f| rgb(f) == (0.6, 0.6, 0.6)));
    // Leaves: pine (meta 1), birch (meta 2), else the foliage map.
    for (meta, m) in [(1, 0x619961), (2, 0x80a755), (0, 0x55aa11), (8, 0x55aa11)] {
        er.clear_effects();
        er.play_aux_sfx(&mut w, None, 2001, x, y, z, block::ids::LEAVES + meta * 256);
        assert_eq!(rgb(&er.fx_layers[1][0]), tinted(m), "leaves meta {meta}");
    }
    // Grass: never tinted.
    er.clear_effects();
    er.play_aux_sfx(&mut w, None, 2001, x, y, z, block::ids::GRASS);
    assert_eq!(rgb(&er.fx_layers[1][0]), (0.6, 0.6, 0.6));
}

#[test]
fn hit_effects_tint_with_the_world_metadata() {
    let mut w = world();
    let mut er = renderer(11);
    er.colorizers = flat_colorizers();
    w.set_block_and_metadata(1, 64, 1, block::ids::TALL_GRASS, 2);
    er.add_block_hit_effects(&mut w, 1, 64, 1, 1);
    w.set_block_and_metadata(2, 64, 1, block::ids::LEAVES, 1);
    er.add_block_hit_effects(&mut w, 2, 64, 1, 1);
    assert_eq!(rgb(&er.fx_layers[1][0]), tinted(0x336699));
    assert_eq!(rgb(&er.fx_layers[1][1]), tinted(0x619961));
}

#[test]
fn block_color_multiplier_table() {
    use super::subclasses::block_color_multiplier;
    let mut w = world();
    let c = flat_colorizers();
    let mut m = |id, meta| block_color_multiplier(&mut w, &c, id, meta, 0, 64, 0);
    assert_eq!(m(block::ids::STONE, 0), 0xffffff);
    assert_eq!(m(block::ids::GRASS, 0), 0x336699);
    assert_eq!(m(block::ids::WATER_MOVING, 0), 0xffffff);
    assert_eq!(m(block::ids::REDSTONE_WIRE, 15), 0x800000);
    assert_eq!(m(block::ids::TALL_GRASS, 0), 0xffffff);
    assert_eq!(m(block::ids::TALL_GRASS, 1), 0x336699);
    assert_eq!(m(block::ids::LEAVES, 3), 0x619961);
}

#[test]
fn publish_statistics_writes_the_count_for_f3() {
    use bevy::prelude::*;
    let mut w = world();
    let mut er = renderer(12);
    er.add_block_destroy_effects(&mut w, 0, 64, 0, block::ids::STONE, 0);
    let mut app = App::new();
    app.init_resource::<crate::debug::DebugSources>()
        .insert_resource(er)
        .add_systems(Update, super::publish_statistics);
    app.update();
    assert_eq!(
        app.world()
            .resource::<crate::debug::DebugSources>()
            .particles,
        "64"
    );
}
