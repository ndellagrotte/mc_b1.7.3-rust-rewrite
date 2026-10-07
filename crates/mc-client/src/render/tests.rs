//! Tests of the transcribed render data and formulas (WP-3.5b) and of the
//! GL emulation they draw through.

use bevy::math::{Mat4, Vec3};
use mc_sim::entity::EnumArt;
use mc_sim::entity::sheep::FLEECE_COLOR_TABLE;

use super::gl::{EntityGl, Topology, glx};
use super::*;
use crate::gl_material::GlBlend;
use crate::gui::batch::gl;

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

// ---- RenderManager ---------------------------------------------------------------

#[test]
fn yaw_interpolates_without_wrapping() {
    // RenderManager.java:106: prev + (cur - prev) * partial, no ±180 wrap
    assert_eq!(interpolate_yaw(170.0, -170.0, 0.5), 0.0);
    assert_eq!(interpolate_yaw(350.0, 370.0, 0.25), 355.0);
    assert_eq!(interpolate_yaw(10.0, 20.0, 0.0), 10.0);
    assert_eq!(interpolate_yaw(10.0, 20.0, 1.0), 20.0);
}

fn world() -> mc_sim::world::World {
    mc_sim::world::World::new(
        Box::new(mc_sim::save::NoSave),
        "r",
        mc_sim::world::provider::WorldProvider::new(mc_sim::world::provider::Dimension::Surface),
        1,
        mc_sim::world::chunk_provider::ChunkGenerator::Empty(
            mc_sim::world::chunk_provider::EmptyChunkProvider,
        ),
        mc_sim::world::WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    )
}

#[test]
fn renderer_map_falls_back_to_the_superclass() {
    let mut w = world();
    let r = |name: &str, w: &mut mc_sim::world::World| {
        RendererId::of(
            &mc_sim::save::entity_list::create_entity_in_world(name, w)
                .unwrap()
                .kind,
        )
    };
    // EntityMob ("Monster") has no entry: EntityLiving's RenderLiving(ModelBiped)
    assert_eq!(r("Monster", &mut w), RendererId::Living);
    // EntityPigZombie: EntityZombie's renderer
    assert_eq!(r("PigZombie", &mut w), RendererId::Zombie);
    assert_eq!(r("Giant", &mut w), RendererId::GiantZombie);
    assert_eq!(r("Snowball", &mut w), RendererId::Snowball);
    assert_eq!(r("PrimedTnt", &mut w), RendererId::TntPrimed);
    let rs = Renderers::default();
    assert_eq!(rs.living.len(), LIVING_IDS.len());
    // shadow sizes from RenderManager.java:27-41 and the constructors
    assert_eq!(rs.base(RendererId::Spider).shadow_size, 1.0);
    assert_eq!(rs.base(RendererId::Chicken).shadow_size, 0.3);
    assert_eq!(rs.base(RendererId::Slime).shadow_size, 0.25);
    assert_eq!(rs.base(RendererId::GiantZombie).shadow_size, 3.0);
    assert_eq!(rs.base(RendererId::Item).shadow_size, 0.15);
    assert_eq!(rs.base(RendererId::Item).field_194_c, 0.75);
    assert_eq!(rs.base(RendererId::Minecart).shadow_size, 0.5);
    assert_eq!(rs.base(RendererId::Arrow).shadow_size, 0.0);
}

#[test]
fn debug_line_is_render_global_s() {
    let rm = RenderManager {
        counts: (12, 5, 0),
        ..Default::default()
    };
    assert_eq!(rm.get_debug_info_entities(), "E: 5/12. B: 0, I: 7");
}

#[test]
fn viewer_distance_is_squared() {
    let i = RenderInfo {
        viewer_pos: DVec3::new(1.0, 2.0, 3.0),
        ..Default::default()
    };
    assert_eq!(i.func_851_a(4.0, 6.0, 3.0), 25.0);
}

// ---- Render: shadow and fire ---------------------------------------------------

#[test]
fn shadow_opacity_fades_over_sixteen_blocks() {
    assert_eq!(base::shadow_opacity(0.0, 1.0), 1.0);
    assert_eq!(base::shadow_opacity(128.0, 1.0), 0.5);
    assert_eq!(base::shadow_opacity(256.0, 1.0), 0.0);
    assert!(base::shadow_opacity(300.0, 1.0) < 0.0);
    // RenderItem's field_194_c
    assert_eq!(base::shadow_opacity(64.0, 0.75), 0.5625);
}

#[test]
fn shadow_alpha_by_height_and_light() {
    // standing on the face (feet at the block's y): opacity / 2 × light
    assert_eq!(shadow::shadow_alpha(1.0, 64.0, 64, 0.0, 1.0), Some(0.5));
    // one block above: (1 - 0.5) × 0.5
    assert_eq!(shadow::shadow_alpha(1.0, 65.0, 64, 0.0, 1.0), Some(0.25));
    // three blocks above: below zero, none
    assert_eq!(shadow::shadow_alpha(1.0, 67.0, 64, 0.0, 1.0), None);
    // at most 1
    assert_eq!(shadow::shadow_alpha(4.0, 64.0, 64, 0.0, 1.0), Some(1.0));
    // dark: scaled by the light
    assert_eq!(shadow::shadow_alpha(1.0, 64.0, 64, 0.0, 0.5), Some(0.25));
}

#[test]
fn fire_alternates_two_tiles_and_flips_pairs() {
    let u0 = 240.0 / 256.0;
    let u1 = (240.0 + 15.99) / 256.0;
    let v31 = (16.0, (16.0 + 15.99) / 256.0);
    // step 0: tile 31, u swapped (0 / 2 % 2 == 0)
    assert_eq!(fire::fire_uv(0), (u1, u0, 16.0 / 256.0, v31.1));
    // step 1: tile 47 (one row down), u swapped
    assert_eq!(
        fire::fire_uv(1),
        (u1, u0, 32.0 / 256.0, (32.0 + 15.99) / 256.0)
    );
    // steps 2 and 3: not swapped
    assert_eq!(fire::fire_uv(2), (u0, u1, 16.0 / 256.0, v31.1));
    assert_eq!(fire::fire_uv(3).0, u0);
    assert_eq!(fire::fire_uv(4).0, u1);
}

// ---- RenderLiving and its subclasses ------------------------------------------

#[test]
fn death_tilt_reaches_its_maximum() {
    assert_eq!(living::death_rotation(1, 0.0), 0.0);
    assert!(close(
        living::death_rotation(6, 0.5),
        mc_core::math::sqrt_float(5.5 / 20.0 * 1.6)
    ));
    assert_eq!(living::death_rotation(20, 1.0), 1.0);
}

#[test]
fn fleece_colour_table_is_entity_sheep_s() {
    // src: EntitySheep.java:6
    let java: [[f32; 3]; 16] = [
        [1.0, 1.0, 1.0],
        [0.95, 0.7, 0.2],
        [0.9, 0.5, 0.85],
        [0.6, 0.7, 0.95],
        [0.9, 0.9, 0.2],
        [0.5, 0.8, 0.1],
        [0.95, 0.7, 0.8],
        [0.3, 0.3, 0.3],
        [0.6, 0.6, 0.6],
        [0.3, 0.6, 0.7],
        [0.7, 0.4, 0.9],
        [0.2, 0.4, 0.8],
        [0.5, 0.4, 0.3],
        [0.4, 0.5, 0.2],
        [0.8, 0.3, 0.3],
        [0.1, 0.1, 0.1],
    ];
    assert_eq!(FLEECE_COLOR_TABLE, java);
    for (i, c) in java.iter().enumerate() {
        let w = sheep::wool_color(0.5, i as i32);
        assert_eq!(w, [0.5 * c[0], 0.5 * c[1], 0.5 * c[2]]);
    }
}

#[test]
fn creeper_swell_and_flash() {
    assert_eq!(creeper::creeper_scale(0.0), (1.0, 1.0));
    let (xz, y) = creeper::creeper_scale(1.0);
    let s = 1.0 + mc_core::math::sin(100.0) * 0.01;
    assert!(close(xz, 1.4 * s));
    assert!(close(y, 1.1 / s));
    // even tenths: nothing; odd: white at (int)(t × 0.2 × 255) alpha
    assert_eq!(creeper::creeper_color_multiplier(0.05), 0);
    assert_eq!(creeper::creeper_color_multiplier(0.15), 7 << 24 | 0xFFFFFF);
    assert_eq!(creeper::creeper_color_multiplier(0.95), 48 << 24 | 0xFFFFFF);
}

#[test]
fn spider_eyes_glow_in_the_dark() {
    assert_eq!(spider::eye_alpha(1.0), 0.0);
    assert_eq!(spider::eye_alpha(0.0), 0.5);
}

#[test]
fn ghast_and_slime_scales() {
    // at rest: (8 + 1/1) / 2 both ways
    assert_eq!(ghast::ghast_scale(0.0), (4.5, 4.5));
    let (xz, y) = ghast::ghast_scale(20.0);
    assert!(close(xz, (8.0 + 3.0) / 2.0));
    assert!(close(y, (8.0 + 1.0 / 3.0) / 2.0));
    // a size-4 slime unsquished
    assert_eq!(slime::slime_scale(4, 0.0), (4.0, 4.0));
    let (xz, y) = slime::slime_scale(2, 1.0);
    assert!(close(xz, 2.0 / 1.5));
    assert!(close(y, 1.5 * 2.0));
}

#[test]
fn armour_textures_by_render_index_and_pass() {
    assert_eq!(player::armor_texture(0, 0), Some("/armor/cloth_1.png"));
    assert_eq!(player::armor_texture(2, 2), Some("/armor/iron_2.png"));
    assert_eq!(player::armor_texture(4, 3), Some("/armor/gold_1.png"));
    assert_eq!(player::armor_texture(3, 1), Some("/armor/diamond_1.png"));
    assert_eq!(player::armor_texture(5, 0), None);
    assert_eq!(
        player::ARMOR_FILENAME_PREFIX,
        ["cloth", "chain", "iron", "diamond", "gold"]
    );
}

#[test]
fn living_renderers_own_their_pass_models() {
    use crate::model::ModelKind as M;
    let r = RenderLiving::new(RendererId::Player);
    assert_eq!(r.main_model.kind, M::Biped);
    assert_eq!(
        r.models.iter().map(|m| m.kind).collect::<Vec<_>>(),
        vec![M::BipedArmorChestplate, M::BipedArmor]
    );
    assert_eq!(r.render_pass_model, None);
    let s = RenderLiving::new(RendererId::Spider);
    assert_eq!(s.render_pass_model, Some(0));
    assert_eq!(s.get_death_max_rotation(), 180.0);
    assert_eq!(
        RenderLiving::new(RendererId::Pig).models[0].kind,
        M::PigSaddle
    );
    assert_eq!(
        RenderLiving::new(RendererId::Sheep).models[0].kind,
        M::SheepFur
    );
    assert_eq!(
        RenderLiving::new(RendererId::Creeper).models[0].kind,
        M::CreeperArmor
    );
    assert_eq!(
        RenderLiving::new(RendererId::Slime).models[0].kind,
        M::SlimeOuter
    );
    assert_eq!(RenderLiving::new(RendererId::GiantZombie).scale, 6.0);
    assert_eq!(
        RenderLiving::new(RendererId::Living).main_model.kind,
        M::Biped
    );
}

// ---- the other renderers ---------------------------------------------------------

#[test]
fn dropped_item_copies_bob_and_spin() {
    assert_eq!(item::copies(1), 1);
    assert_eq!(item::copies(2), 2);
    assert_eq!(item::copies(5), 2);
    assert_eq!(item::copies(6), 3);
    assert_eq!(item::copies(20), 3);
    assert_eq!(item::copies(21), 4);
    assert_eq!(item::copies(64), 4);
    assert!(close(item::bob(0, 0.0, 0.0), 0.1));
    assert!(close(item::spin(20, 0.0, 0.0), 57.295776));
    // icon 14 (the snowball): column 14, row 0
    assert_eq!(
        item::icon_uv(14),
        (224.0 / 256.0, 240.0 / 256.0, 0.0, 16.0 / 256.0)
    );
    assert_eq!(item::icon_uv(37).2, 32.0 / 256.0);
}

#[test]
fn arrow_texture_rows() {
    // type 0: the shaft in pixel rows 0-5, the back plate in 5-10 of 32
    assert_eq!(
        arrow::arrow_rows(0),
        (0.0, 5.0 / 32.0, 5.0 / 32.0, 10.0 / 32.0)
    );
    assert_eq!(
        arrow::arrow_rows(1),
        (10.0 / 32.0, 15.0 / 32.0, 15.0 / 32.0, 20.0 / 32.0)
    );
    assert_eq!(arrow::shake_angle(0, 0.5), None);
    let s = arrow::shake_angle(7, 0.0).unwrap();
    assert!(close(s, -mc_core::math::sin(21.0) * 7.0));
}

#[test]
fn painting_art_table_is_enum_art() {
    // src: EnumArt.java:4-28 (title, sizeX, sizeY, offsetX, offsetY)
    let java: [(&str, i32, i32, i32, i32); 25] = [
        ("Kebab", 16, 16, 0, 0),
        ("Aztec", 16, 16, 16, 0),
        ("Alban", 16, 16, 32, 0),
        ("Aztec2", 16, 16, 48, 0),
        ("Bomb", 16, 16, 64, 0),
        ("Plant", 16, 16, 80, 0),
        ("Wasteland", 16, 16, 96, 0),
        ("Pool", 32, 16, 0, 32),
        ("Courbet", 32, 16, 32, 32),
        ("Sea", 32, 16, 64, 32),
        ("Sunset", 32, 16, 96, 32),
        ("Creebet", 32, 16, 128, 32),
        ("Wanderer", 16, 32, 0, 64),
        ("Graham", 16, 32, 16, 64),
        ("Match", 32, 32, 0, 128),
        ("Bust", 32, 32, 32, 128),
        ("Stage", 32, 32, 64, 128),
        ("Void", 32, 32, 96, 128),
        ("SkullAndRoses", 32, 32, 128, 128),
        ("Fighters", 64, 32, 0, 96),
        ("Pointer", 64, 64, 0, 192),
        ("Pigscene", 64, 64, 64, 192),
        ("BurningSkull", 64, 64, 128, 192),
        ("Skeleton", 64, 48, 192, 64),
        ("DonkeyKong", 64, 48, 192, 112),
    ];
    for (a, j) in EnumArt::VALUES.iter().zip(java.iter()) {
        let d = a.def();
        assert_eq!((d.title, d.size_x, d.size_y, d.offset_x, d.offset_y), *j);
    }
    // Kebab's only tile: the texture rectangle right to left (u flipped)
    assert_eq!(
        painting::tile_uv(16, 16, 0, 0, 0, 0),
        (16.0 / 256.0, 0.0, 16.0 / 256.0, 0.0)
    );
    // Fighters' third column, second row
    assert_eq!(
        painting::tile_uv(64, 32, 0, 96, 2, 1),
        (32.0 / 256.0, 16.0 / 256.0, 112.0 / 256.0, 96.0 / 256.0)
    );
    // the light of a tile: along x for direction 0, -z for 1, -x for 2, +z for 3
    let p = [10.5, 64.5, 20.5];
    assert_eq!(painting::tile_light_pos(p, 0, 16.0, 0.0), [11, 64, 20]);
    assert_eq!(painting::tile_light_pos(p, 1, 16.0, 0.0), [10, 64, 19]);
    assert_eq!(painting::tile_light_pos(p, 2, 16.0, 0.0), [9, 64, 20]);
    assert_eq!(painting::tile_light_pos(p, 3, 16.0, -16.0), [10, 63, 21]);
}

#[test]
fn lightning_offsets_match_java_random() {
    // `java Bolt 123456789` on the oracle JVM (RenderLightningBolt.java:13-24)
    let (v11, v12, v13, v15) = lightning_bolt::bolt_offsets(123456789);
    let xs: Vec<i32> = v11.iter().map(|v| *v as i32).collect();
    let zs: Vec<i32> = v12.iter().map(|v| *v as i32).collect();
    assert_eq!(xs, vec![1, 1, -3, -5, -3, -3, 0, 0]);
    assert_eq!(zs, vec![6, 6, 2, -1, -5, -2, 1, 0]);
    assert_eq!((v13 as i32, v15 as i32), (4, 8));
    let (v11, _, v13, v15) = lightning_bolt::bolt_offsets(-42);
    assert_eq!(
        v11.iter().map(|v| *v as i32).collect::<Vec<_>>(),
        vec![-5, -7, -5, -2, -5, -1, 1, 0]
    );
    assert_eq!((v13 as i32, v15 as i32), (-7, 3));
}

#[test]
fn lightning_strips_match_java_random() {
    let strips = lightning_bolt::bolt_strips(123456789, 0.0, 0.0, 0.0);
    // 4 layers × (8 + 3 + 3) segments
    assert_eq!(strips.len(), 56);
    assert!(strips.iter().all(|s| s.len() == 10));
    // layer 1 (the second width), from `java Bolt 123456789`: per strip
    // (branch, segment, var22, var24, var27, var29)
    let java = [
        (0, 7, -4, -7, -4, -8),
        (0, 6, -7, -10, -4, -7),
        (0, 0, 0, 0, -3, -2),
        (1, 6, 6, 5, -4, -7),
        (1, 4, 25, 17, 12, 9),
        (2, 5, 4, -10, -7, -10),
        (2, 3, 5, 6, 13, -6),
    ];
    let index = |b: i32, seg: i32| -> usize {
        // layer 1 starts at 14; branch 0 has segments 7..0, 1: 6..4, 2: 5..3
        14 + match b {
            0 => (7 - seg) as usize,
            1 => 8 + (6 - seg) as usize,
            _ => 11 + (5 - seg) as usize,
        }
    };
    for (b, seg, v22, v24, v27, v29) in java {
        let s = &strips[index(b, seg)];
        let expect = lightning_bolt::strip_vertices(
            0.0,
            0.0,
            0.0,
            1,
            b,
            seg,
            (v22 as f64, v24 as f64),
            (v27 as f64, v29 as f64),
        );
        assert_eq!(s, &expect, "branch {b} segment {seg}");
    }
    // the main channel ends at the strike point: layer 0's last segment
    // starts at the block centre, `0.1 × ((0 - 1) × 0.1 + 1)` off it
    let last = &strips[7];
    let w = 0.1 * ((0 - 1) as f64 * 0.1 + 1.0);
    assert_eq!(last[0], [0.5 - w, 0.0, 0.5 - w]);
    assert_eq!(last[1][1], 16.0);
}

#[test]
fn sign_rotations_per_metadata() {
    for m in 0..16 {
        assert_eq!(tile_entity::sign::sign_rotation(true, m), m as f32 * 22.5);
    }
    assert_eq!(tile_entity::sign::sign_rotation(false, 2), 180.0);
    assert_eq!(tile_entity::sign::sign_rotation(false, 3), 0.0);
    assert_eq!(tile_entity::sign::sign_rotation(false, 4), 90.0);
    assert_eq!(tile_entity::sign::sign_rotation(false, 5), -90.0);
}

#[test]
fn sign_model_boxes() {
    let m = model_sign::SignModel::default();
    let b = m.sign_board.geometry.as_ref().unwrap();
    // addBox(-12, -14, -1, 24, 12, 2): corner 0 at the box's minimum
    assert_eq!(b.corners[0].pos, [-12.0, -14.0, -1.0]);
    let s = m.sign_stick.geometry.as_ref().unwrap();
    assert_eq!(s.corners[0].pos, [-1.0, -2.0, -1.0]);
    assert_eq!(
        (m.sign_stick.texture_offset_x, m.sign_stick.texture_offset_y),
        (0, 14)
    );
}

#[test]
fn tnt_swells_and_flashes() {
    assert_eq!(tnt_primed::tnt_scale(80, 0.0), None);
    assert_eq!(tnt_primed::tnt_scale(0, 1.0), Some(1.3));
    assert!(close(
        tnt_primed::tnt_scale(4, 0.0).unwrap(),
        1.0 + 0.5f32.powi(4) * 0.3
    ));
    assert!(close(
        tnt_primed::tnt_flash_alpha(80, 0.0),
        (1.0 - 81.0 / 100.0) * 0.8
    ));
}

#[test]
fn minecart_rock() {
    assert_eq!(minecart::rock_angle(0, 10, 1, 0.5), None);
    let a = minecart::rock_angle(10, 20, -1, 0.0).unwrap();
    assert!(close(a, -(mc_core::math::sin(10.0) * 10.0 * 20.0 / 10.0)));
}

#[test]
fn fish_line_sags() {
    let d = [1.0, -2.0, 3.0];
    assert_eq!(fish::line_vertex(0.0, 0.0, 0.0, d, 0), [0.0, 0.25, 0.0]);
    assert_eq!(
        fish::line_vertex(0.0, 0.0, 0.0, d, 16),
        [1.0, -2.0 + 0.25, 3.0]
    );
    let m = fish::line_vertex(0.0, 0.0, 0.0, d, 8);
    assert_eq!(m, [0.5, -2.0 * 0.75 * 0.5 + 0.25, 1.5]);
}

// ---- the GL emulation ------------------------------------------------------------

#[test]
fn lighting_is_gl_s_formula() {
    let g = EntityGl::new(false);
    // straight up: both lights at 0.8085…; clamped
    assert_eq!(g.light([1.0; 4], Vec3::Y), [1.0, 1.0, 1.0, 1.0]);
    // facing away from both lights: the ambient 0.4
    assert_eq!(
        g.light([0.5, 1.0, 0.0, 0.3], Vec3::NEG_Y),
        [0.2, 0.4, 0.0, 0.3]
    );
}

/// Draws one quad with the arrow's normal handling (`glNormal3f(s, 0, 0)`
/// under `glScalef(s)`) and returns its colour.
fn arrow_like_colour(normalize: bool) -> [f32; 4] {
    let mut g = EntityGl::new(normalize);
    g.enable(gl::RESCALE_NORMAL);
    g.color3f(1.0, 1.0, 1.0);
    let s = 0.05625;
    g.scalef(s, s, s);
    g.rotatef(90.0, 0.0, 0.0, 1.0); // +x up
    g.normal3f(s, 0.0, 0.0);
    g.start_drawing_quads();
    for _ in 0..4 {
        g.add_vertex_with_uv(0.0, 0.0, 0.0, 0.0, 0.0);
    }
    g.draw();
    g.out.tess[0].colors[0]
}

#[test]
fn rescale_normal_keeps_a_short_normal_short() {
    let lit_up = 0.4 + 0.6 * 2.0 * standard_dir_y();
    // GL_RESCALE_NORMAL: |n| stays 0.05625 (RenderArrow's dim arrows)
    let c = arrow_like_colour(false);
    assert!(
        close(c[0], 0.4 + 0.6 * 2.0 * standard_dir_y() * 0.05625),
        "{c:?}"
    );
    // GL_NORMALIZE (latched by a slime): fully lit
    let c = arrow_like_colour(true);
    assert!(close(c[0], lit_up.min(1.0)), "{c:?}");
}

fn standard_dir_y() -> f32 {
    crate::model::material::standard_light_directions()[0].y
}

#[test]
fn quads_become_triangles_with_the_provoking_colour() {
    let mut g = EntityGl::new(false);
    g.disable(gl::LIGHTING);
    g.start_drawing_quads();
    for i in 0..4 {
        g.set_color_opaque(i * 10, 0, 0);
        g.add_vertex_with_uv(i as f64, 0.0, 0.0, 0.0, 0.0);
    }
    g.draw();
    let d = &g.out.tess[0];
    assert_eq!(d.spec.topology, Topology::Triangles);
    let xs: Vec<f32> = d.positions.iter().map(|p| p[0]).collect();
    assert_eq!(xs, vec![0.0, 1.0, 2.0, 0.0, 2.0, 3.0]);
    // GL_FLAT: (0,1,2) takes vertex 2's colour, (0,2,3) vertex 3's
    let reds: Vec<f32> = d.colors.iter().map(|c| (c[0] * 255.0).round()).collect();
    assert_eq!(reds, vec![20.0, 20.0, 20.0, 30.0, 30.0, 30.0]);
    // GL_SMOOTH keeps each vertex's
    let mut g = EntityGl::new(false);
    g.disable(gl::LIGHTING);
    g.shade_model(gl::SMOOTH);
    g.start_drawing_quads();
    for i in 0..4 {
        g.set_color_opaque(i * 10, 0, 0);
        g.add_vertex_with_uv(i as f64, 0.0, 0.0, 0.0, 0.0);
    }
    g.draw();
    let reds: Vec<f32> = g.out.tess[0]
        .colors
        .iter()
        .map(|c| (c[0] * 255.0).round())
        .collect();
    assert_eq!(reds, vec![0.0, 10.0, 20.0, 0.0, 20.0, 30.0]);
}

#[test]
fn strips_expand_and_consecutive_draws_merge() {
    let mut g = EntityGl::new(false);
    g.disable(gl::LIGHTING);
    g.start_drawing(glx::LINE_STRIP);
    for i in 0..3 {
        g.add_vertex(i as f64, 0.0, 0.0);
    }
    g.draw();
    assert_eq!(g.out.tess[0].spec.topology, Topology::Lines);
    assert_eq!(g.out.tess[0].positions.len(), 4);
    g.start_drawing(glx::TRIANGLE_STRIP);
    for i in 0..4 {
        g.add_vertex(i as f64, (i % 2) as f64, 0.0);
    }
    g.draw();
    // a new batch (triangles), two triangles, the odd one rewound
    assert_eq!(g.out.tess.len(), 2);
    let xs: Vec<f32> = g.out.tess[1].positions.iter().map(|p| p[0]).collect();
    assert_eq!(xs, vec![0.0, 1.0, 2.0, 2.0, 1.0, 3.0]);
    // the same state again: merged
    g.start_drawing(glx::TRIANGLE_STRIP);
    for i in 0..3 {
        g.add_vertex(i as f64, 0.0, 0.0);
    }
    g.draw();
    assert_eq!(g.out.tess.len(), 2);
    assert_eq!(g.out.tess[1].positions.len(), 9);
}

#[test]
fn matrices_apply_in_gl_order() {
    let mut g = EntityGl::new(false);
    g.push_matrix();
    g.translatef(1.0, 2.0, 3.0);
    g.rotatef(90.0, 0.0, 1.0, 0.0);
    g.scalef(2.0, 2.0, 2.0);
    let p = g.matrix().transform_point3(Vec3::X);
    // scale, then rotate +x to -z, then translate
    assert!((p - Vec3::new(1.0, 2.0, 1.0)).length() < 1e-5, "{p:?}");
    g.pop_matrix();
    assert_eq!(g.matrix(), Mat4::IDENTITY);
}

#[test]
fn model_state_maps_blend_functions() {
    let mut g = EntityGl::new(false);
    assert_eq!(g.model_spec().blend, GlBlend::Off);
    g.enable(gl::BLEND);
    g.blend_func(gl::ONE, gl::ONE);
    let s = g.model_spec();
    assert_eq!((s.blend, s.src_one), (GlBlend::Additive, true));
    g.blend_func(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
    assert_eq!(g.model_spec().blend, GlBlend::Alpha);
    // the GL_TEXTURE matrix's translation is the uv offset
    g.matrix_mode(glx::TEXTURE);
    g.load_identity();
    g.translatef(0.25, 0.5, 0.0);
    g.matrix_mode(glx::MODELVIEW);
    assert_eq!(g.model_spec().uv_offset, [0.25, 0.5]);
    g.depth_func(glx::EQUAL);
    assert!(g.model_spec().depth_equal);
}

#[test]
fn in_range_overrides() {
    use mc_sim::entity::EntityKindData as K;
    let mut w = world();
    let mut e = mc_sim::entity::snowball::new_entity_snowball_at(&mut w, 0.0, 0.0, 0.0);
    // 0.25-wide box: 0.25 × 4 × 64 = 64 blocks
    assert!(is_in_range_to_render_vec3d(&e, DVec3::new(63.0, 0.0, 0.0)));
    assert!(!is_in_range_to_render_vec3d(&e, DVec3::new(65.0, 0.0, 0.0)));
    e.base.pos_x = 0.0;
    let mut l = mc_sim::entity::lightning_bolt::new_entity_lightning_bolt(&mut w, 0.0, 0.0, 0.0);
    assert!(is_in_range_to_render_vec3d(
        &l,
        DVec3::new(1000.0, 0.0, 0.0)
    ));
    if let K::LightningBolt(_, d) = &mut l.kind {
        d.lightning_state = -1;
    }
    assert!(!is_in_range_to_render_vec3d(&l, DVec3::ZERO));
    assert_eq!(shadow::get_shadow_size(&e), 0.0);
}

#[test]
fn dev_actions_parse() {
    let s = crate::script::Script::parse(
        r#"{"actions": [
            {"action": "spawn_entity", "kind": "Pig", "ahead": 3, "saddled": true},
            {"action": "set_block", "ahead": 2, "id": 63, "meta": 8, "text": ["a", "b", "c", "d"]}
        ]}"#,
    )
    .unwrap();
    let crate::script::Action::SpawnEntity(a) = &s.steps[0].action else {
        panic!("spawn_entity");
    };
    assert_eq!(
        (a.kind.as_str(), a.ahead, a.saddled),
        ("Pig", Some(3.0), true)
    );
    let crate::script::Action::SetBlock(b) = &s.steps[1].action else {
        panic!("set_block");
    };
    assert_eq!(
        (b.id, b.meta, b.text.as_ref().map(|t| t[1].as_str())),
        (63, 8, Some("b"))
    );
}
