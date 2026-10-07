//! The model tables and poses against the vanilla classes run on the
//! oracle JVM (`testdata/java_models.txt`, made by `testdata/ModelDump.java`:
//! every part's rotation point, angles, mirror flag, corners and faces, each
//! face's normal and `Tessellator.setNormal` packing, then the parts after a
//! fixed sequence of pose calls on one shared instance), compared bit for
//! bit.

use super::part::{TEXTURE_HEIGHT, TEXTURE_WIDTH};
use super::*;

const JAVA: &str = include_str!("testdata/java_models.txt");

#[derive(Debug, Default)]
struct JavaPart {
    name: String,
    tex: (i32, i32),
    pose: [f32; 6],
    mirror: bool,
    corners: Vec<f32>,
    faces: Vec<[i32; 8]>,
    /// Per face: `TexturedQuad.draw`'s normal and the packed `normal` int.
    normals: Vec<([f32; 3], i32)>,
}

#[derive(Debug, Default)]
struct JavaDump {
    models: Vec<(String, Vec<JavaPart>)>,
    poses: Vec<(String, String, Vec<[f32; 6]>)>,
    ghast_lengths: Vec<i32>,
    wolf_shading: f32,
}

fn parse() -> JavaDump {
    let mut d = JavaDump::default();
    let f = |s: &str| s.parse::<f32>().unwrap();
    let mut in_pose = false;
    for line in JAVA.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w[0] {
            "model" => {
                d.models.push((w[1].to_string(), Vec::new()));
                in_pose = false;
            }
            "part" => {
                // part NAME tex U V rp X Y Z ang X Y Z mirror M
                d.models.last_mut().unwrap().1.push(JavaPart {
                    name: w[1].to_string(),
                    tex: (w[3].parse().unwrap(), w[4].parse().unwrap()),
                    pose: [f(w[6]), f(w[7]), f(w[8]), f(w[10]), f(w[11]), f(w[12])],
                    mirror: w[14] == "1",
                    ..Default::default()
                });
            }
            "corners" => {
                let p = d.models.last_mut().unwrap().1.last_mut().unwrap();
                p.corners = w[1..].iter().map(|s| f(s)).collect();
            }
            "face" => {
                let p = d.models.last_mut().unwrap().1.last_mut().unwrap();
                let v: Vec<i32> = w[1..].iter().map(|s| s.parse().unwrap()).collect();
                p.faces.push(v.try_into().unwrap());
            }
            "normal" => {
                let p = d.models.last_mut().unwrap().1.last_mut().unwrap();
                p.normals
                    .push(([f(w[1]), f(w[2]), f(w[3])], w[4].parse().unwrap()));
            }
            "wolf_shading" => d.wolf_shading = f(w[1]),
            "pose" => {
                d.poses
                    .push((w[1].to_string(), w[2].to_string(), Vec::new()));
                in_pose = true;
            }
            "ghast_lengths" => {
                d.ghast_lengths = w[1..].iter().map(|s| s.parse().unwrap()).collect();
            }
            _ => {
                assert!(in_pose, "{line}");
                let v: Vec<f32> = w.iter().map(|s| f(s)).collect();
                d.poses.last_mut().unwrap().2.push(v.try_into().unwrap());
            }
        }
    }
    d
}

fn kind(name: &str) -> ModelKind {
    ModelKind::from_name(name).unwrap_or_else(|| panic!("{name}"))
}

fn pose_of(p: &ModelPart) -> [f32; 6] {
    [
        p.rotation_point_x,
        p.rotation_point_y,
        p.rotation_point_z,
        p.rotate_angle_x,
        p.rotate_angle_y,
        p.rotate_angle_z,
    ]
}

fn bits(v: &[f32]) -> Vec<u32> {
    v.iter().map(|x| x.to_bits()).collect()
}

/// Every model's parts (count, names, texture offsets, rotation points,
/// angles, mirror flags, box corners, face corner order and UVs) as the
/// Java constructors leave them.
#[test]
fn constructors_match_the_java_tables() {
    let d = parse();
    assert_eq!(d.models.len(), ModelKind::ALL.len());
    for (name, jparts) in &d.models {
        let m = Model::new(kind(name));
        assert_eq!(m.parts.len(), jparts.len(), "{name}: part count");
        for (p, j) in m.parts.iter().zip(jparts) {
            let ctx = format!("{name}.{}", j.name);
            assert_eq!(p.name, j.name, "{ctx}");
            assert_eq!((p.texture_offset_x, p.texture_offset_y), j.tex, "{ctx}");
            assert_eq!(bits(&pose_of(p)), bits(&j.pose), "{ctx}: pose");
            assert_eq!(p.mirror, j.mirror, "{ctx}");
            let g = p.geometry.as_ref().unwrap();
            let corners: Vec<f32> = g.corners.iter().flat_map(|c| c.pos).collect();
            assert_eq!(bits(&corners), bits(&j.corners), "{ctx}: corners");
            assert_eq!(g.faces.len(), j.faces.len());
            for (fi, (f, jf)) in g.faces.iter().zip(&j.faces).enumerate() {
                let want_idx = [
                    jf[0] as usize,
                    jf[1] as usize,
                    jf[2] as usize,
                    jf[3] as usize,
                ];
                assert_eq!(f.corner, want_idx, "{ctx} face {fi}");
                // TexturedQuad's UVs from the texel rectangle.
                let (u1, v1, u2, v2) = (jf[4] as f32, jf[5] as f32, jf[6] as f32, jf[7] as f32);
                let (i6, i7) = (0.0015625f32, 0.003125f32);
                let mut uv = [
                    (u2 / TEXTURE_WIDTH - i6, v1 / TEXTURE_HEIGHT + i7),
                    (u1 / TEXTURE_WIDTH + i6, v1 / TEXTURE_HEIGHT + i7),
                    (u1 / TEXTURE_WIDTH + i6, v2 / TEXTURE_HEIGHT - i7),
                    (u2 / TEXTURE_WIDTH - i6, v2 / TEXTURE_HEIGHT - i7),
                ];
                if j.mirror {
                    uv.reverse();
                }
                for (k, v) in f.vertex_positions.iter().enumerate() {
                    assert_eq!(
                        (v.u.to_bits(), v.v.to_bits()),
                        (uv[k].0.to_bits(), uv[k].1.to_bits()),
                        "{ctx} face {fi} vertex {k}"
                    );
                    assert_eq!(v.pos, g.corners[f.corner[k]].pos);
                }
                // The face normal (signed zeros included) and what GL reads
                // of the packed bytes.
                let (jn, packed) = j.normals[fi];
                let n = f.face_normal();
                assert_eq!(bits(&n), bits(&jn), "{ctx} face {fi} normal");
                let b = |shift: u32| ((packed >> shift) & 0xFF) as u8 as i8;
                let conv = |c: i8| (2.0 * c as f32 + 1.0) / 255.0;
                let want = [conv(b(0)), conv(b(8)), conv(b(16))];
                assert_eq!(
                    bits(&part::tess_normal(n[0], n[1], n[2])),
                    bits(&want),
                    "{ctx} face {fi} packed normal {packed}"
                );
            }
            assert_eq!(j.normals.len(), 6, "{ctx}");
        }
    }
}

const ARGS: [[f32; 6]; 3] = [
    [0.0, 0.0, 0.0, 0.0, 0.0, 0.0625],
    [1.3, 0.7, 25.5, 30.0, -20.0, 0.0625],
    [7.77, 1.0, 1234.25, -75.5, 45.0, 0.0625],
];

fn wolf_entity() -> mc_sim::entity::Entity {
    use mc_sim::save::NoSave;
    use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
    use mc_sim::world::provider::{Dimension, WorldProvider};
    use mc_sim::world::{World, WorldEnvironment};
    let mut w = World::new(
        Box::new(NoSave),
        "models",
        WorldProvider::new(Dimension::Surface),
        123,
        ChunkGenerator::Empty(EmptyChunkProvider),
        WorldEnvironment {
            rand_seed: 0,
            field_9437_g: 0,
            lock_timestamp: 0,
        },
    );
    mc_sim::entity::wolf::new_entity_wolf(&mut w)
}

fn wolf_data(e: &mut mc_sim::entity::Entity) -> &mut mc_sim::entity::WolfData {
    match &mut e.kind {
        mc_sim::entity::EntityKindData::Wolf(_, _, _, w) => w,
        _ => unreachable!(),
    }
}

/// `ModelDump.java`'s pose sequence, replayed; returns (kind, case, parts).
fn replay() -> Vec<(String, String, Vec<[f32; 6]>)> {
    let mut out = Vec::new();
    let snap = |m: &Model| m.parts.iter().map(pose_of).collect::<Vec<_>>();
    let simple = |m: &mut Model, out: &mut Vec<_>| {
        for (i, a) in ARGS.iter().enumerate() {
            m.set_rotation_angles(a[0], a[1], a[2], a[3], a[4], a[5]);
            out.push((m.kind.name().to_string(), format!("args{i}"), snap(m)));
        }
    };
    for k in [
        ModelKind::Biped,
        ModelKind::BipedArmor,
        ModelKind::BipedArmorChestplate,
        ModelKind::Zombie,
        ModelKind::Skeleton,
    ] {
        let mut m = Model::new(k);
        simple(&mut m, &mut out);
        let n = k.name().to_string();
        let a = ARGS[1];
        m.on_ground = 0.3;
        m.is_riding = true;
        m.held_item_right = true;
        m.set_rotation_angles(a[0], a[1], a[2], a[3], a[4], a[5]);
        out.push((n.clone(), "swing_riding_held".into(), snap(&m)));
        let a = ARGS[2];
        m.on_ground = 0.85;
        m.is_riding = false;
        m.held_item_right = false;
        m.held_item_left = true;
        m.is_sneak = true;
        m.set_rotation_angles(a[0], a[1], a[2], a[3], a[4], a[5]);
        out.push((n.clone(), "swing_sneak_left".into(), snap(&m)));
        m.on_ground = 0.0;
        m.held_item_left = false;
        m.is_sneak = false;
        m.set_rotation_angles(a[0], a[1], a[2], a[3], a[4], a[5]);
        out.push((n.clone(), "reset".into(), snap(&m)));
        assert!(m.render_ears(MODEL_SCALE).is_some());
        out.push((n, "ears".into(), snap(&m)));
    }
    for k in [
        ModelKind::Pig,
        ModelKind::PigSaddle,
        ModelKind::Cow,
        ModelKind::Sheep,
        ModelKind::SheepFur,
        ModelKind::Chicken,
        ModelKind::Creeper,
        ModelKind::CreeperArmor,
        ModelKind::Spider,
        ModelKind::Slime,
        ModelKind::SlimeOuter,
        ModelKind::Ghast,
        ModelKind::Squid,
        ModelKind::Minecart,
        ModelKind::Boat,
    ] {
        simple(&mut Model::new(k), &mut out);
    }

    // The wolf, with the inputs read from a simulation EntityWolf in the
    // same states as the Java one.
    let mut m = Model::new(ModelKind::Wolf);
    let mut e = wolf_entity();
    let living = |e: &mc_sim::entity::Entity, partial: f32| {
        LivingInputs::Wolf(wolf::WolfAnimInputs::from_entity(e, partial, 1.0))
    };
    assert_eq!(
        m.set_living_animations(&living(&e, 0.25), 1.3, 0.7, 0.25),
        None
    );
    m.set_rotation_angles(1.3, 0.7, 0.6, 30.0, -20.0, 0.0625);
    out.push(("Wolf".into(), "plain".into(), snap(&m)));
    mc_sim::entity::wolf::set_wolf_sitting(&mut e, true);
    {
        let w = wolf_data(&mut e);
        w.head_rotation_course = 0.8;
        w.head_rotation_course_old = 0.5;
        w.time_wolf_is_shaking = 0.9;
        w.prev_time_wolf_is_shaking = 0.75;
    }
    m.set_living_animations(&living(&e, 0.6), 7.77, 1.0, 0.6);
    m.set_rotation_angles(7.77, 1.0, 1.2, -75.5, 45.0, 0.0625);
    out.push(("Wolf".into(), "sitting_shake".into(), snap(&m)));
    mc_sim::entity::wolf::set_wolf_sitting(&mut e, false);
    mc_sim::entity::wolf::set_wolf_angry(&mut e, true);
    m.set_living_animations(&living(&e, 0.9), 2.5, 0.4, 0.9);
    m.set_rotation_angles(2.5, 0.4, 1.5707964, 10.0, 5.0, 0.0625);
    out.push(("Wolf".into(), "angry".into(), snap(&m)));
    // Shaking: the angles as the Java's, and the glColor3f the Java would
    // set, brightness × getShadingWhileShaking (ModelWolf.java:119-122).
    mc_sim::entity::wolf::set_wolf_angry(&mut e, false);
    {
        let w = wolf_data(&mut e);
        w.is_wolf_shaking = true;
        w.time_wolf_is_shaking = 0.4;
        w.prev_time_wolf_is_shaking = 0.2;
    }
    let inputs = wolf::WolfAnimInputs::from_entity(&e, 0.3, 0.6);
    let shading = parse().wolf_shading;
    assert_eq!(inputs.shading_while_shaking.to_bits(), shading.to_bits());
    let c = 0.6 * shading;
    assert_eq!(
        m.set_living_animations(&LivingInputs::Wolf(inputs), 3.0, 0.5, 0.3),
        Some([c, c, c])
    );
    m.set_rotation_angles(3.0, 0.5, 0.9, -10.0, 15.0, 0.0625);
    out.push(("Wolf".into(), "shaking".into(), snap(&m)));
    out
}

/// `setRotationAngles` (and the wolf's `setLivingAnimations`, the biped's
/// `renderEars`) for every model against the Java, call by call on one
/// instance.
#[test]
fn poses_match_the_java() {
    let d = parse();
    let ours = replay();
    assert_eq!(ours.len(), d.poses.len());
    for ((k, c, parts), (jk, jc, jparts)) in ours.iter().zip(&d.poses) {
        assert_eq!((k, c), (jk, jc));
        assert_eq!(parts.len(), jparts.len(), "{k} {c}");
        for (i, (p, j)) in parts.iter().zip(jparts).enumerate() {
            assert_eq!(bits(p), bits(j), "{k} {c} part {i}: {p:?} vs {j:?}");
        }
    }
}

/// `new Random(1660L).nextInt(7) + 8`, nine times, as the Java drew them,
/// and the tentacle boxes that long.
#[test]
fn ghast_tentacle_lengths() {
    let d = parse();
    let ours = ghast::tentacle_lengths();
    assert_eq!(ours.to_vec(), d.ghast_lengths);
    assert_eq!(ours, [8, 13, 9, 11, 11, 10, 12, 9, 12]);
    let m = Model::new(ModelKind::Ghast);
    for (i, len) in ours.iter().enumerate() {
        let g = m.parts[ghast::TENTACLE0 + i].geometry.as_ref().unwrap();
        assert_eq!(g.corners[6].pos[1] - g.corners[0].pos[1], *len as f32);
    }
}

/// ModelBiped.setRotationAngles at rest and at a known walk phase, by hand
/// (ModelBiped.java:69-162): with limb swing 0 the legs and arms hang
/// straight; at `limbSwing = π/0.6662/2` (cos 0) the arms reach ±amount.
#[test]
fn biped_pose_by_hand() {
    use mc_core::math::{cos, sin};
    let mut m = Model::new(ModelKind::Biped);
    m.set_rotation_angles(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE);
    let p = &m.parts;
    assert_eq!(p[biped::RIGHT_LEG].rotate_angle_x, 0.0);
    assert_eq!(p[biped::LEFT_LEG].rotate_angle_x, 0.0);
    // Idle sway at ticks 0: z = cos(0) * 0.05 + 0.05 = 0.1, x = sin(0) * 0.05.
    assert_eq!(p[biped::RIGHT_ARM].rotate_angle_z, 0.1);
    assert_eq!(p[biped::LEFT_ARM].rotate_angle_z, -0.1);
    assert_eq!(p[biped::RIGHT_ARM].rotation_point_x, -5.0);
    assert_eq!(p[biped::LEFT_ARM].rotation_point_x, 5.0);
    assert_eq!(p[biped::RIGHT_ARM].rotate_angle_x, 0.0);

    // Head look: degrees / 57.295776.
    m.set_rotation_angles(0.0, 0.0, 0.0, 45.0, -30.0, MODEL_SCALE);
    assert_eq!(m.parts[biped::HEAD].rotate_angle_y, 45.0 / 57.295776);
    assert_eq!(m.parts[biped::HEADWEAR].rotate_angle_x, -30.0 / 57.295776);

    // Walking: limbSwing 1, amount 0.5.
    let (s, a) = (1.0f32, 0.5f32);
    m.set_rotation_angles(s, a, 10.0, 0.0, 0.0, MODEL_SCALE);
    let p = &m.parts;
    assert_eq!(
        p[biped::RIGHT_LEG].rotate_angle_x,
        cos(s * 0.6662) * 1.4 * a
    );
    assert_eq!(
        p[biped::LEFT_LEG].rotate_angle_x,
        cos(s * 0.6662 + 3.1415927) * 1.4 * a
    );
    assert_eq!(
        p[biped::LEFT_ARM].rotate_angle_x,
        cos(s * 0.6662) * 2.0 * a * 0.5 - sin(10.0 * 0.067) * 0.05
    );
    assert!(p[biped::RIGHT_LEG].rotate_angle_x > 0.0);
    assert!(p[biped::LEFT_LEG].rotate_angle_x < 0.0);
}

/// Conditional parts: the sheep's wool model inflates every box, the
/// sheared model does not; `ModelSlime(0)` has only the outer cube; the
/// creeper's headwear is built but never drawn; the wolf draws its head
/// with `renderWithRotation`; the minecart's inner floor follows `ticks`;
/// hidden parts are not drawn.
#[test]
fn conditional_parts_and_draw_lists() {
    let wool = Model::new(ModelKind::SheepFur);
    let body = Model::new(ModelKind::Sheep);
    let size = |m: &Model, i: usize| {
        let g = m.parts[i].geometry.as_ref().unwrap();
        g.corners[6].pos[0] - g.corners[0].pos[0]
    };
    assert_eq!(size(&wool, quadruped::BODY), 8.0 + 2.0 * 1.75);
    assert_eq!(size(&body, quadruped::BODY), 8.0);
    assert_eq!(size(&wool, quadruped::HEAD), 6.0 + 2.0 * 0.6);

    assert_eq!(Model::new(ModelKind::SlimeOuter).parts.len(), 1);
    let mut slime = Model::new(ModelKind::Slime);
    assert_eq!(slime.render(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE).len(), 4);
    let mut outer = Model::new(ModelKind::SlimeOuter);
    assert_eq!(outer.render(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE).len(), 1);

    let mut creeper = Model::new(ModelKind::Creeper);
    let d = creeper.render(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE);
    assert_eq!(d.len(), 6);
    assert!(d.iter().all(|x| x.part != creeper::UNUSED_HEADWEAR));

    let mut wolf = Model::new(ModelKind::Wolf);
    let d = wolf.render(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE);
    assert_eq!(d[0].part, wolf::HEAD);
    assert_eq!(d[0].mode, RenderMode::WithRotation);
    assert_eq!(d[1].mode, RenderMode::Render);

    let mut cart = Model::new(ModelKind::Minecart);
    let d = cart.render(0.0, 0.0, -0.1, 0.0, 0.0, MODEL_SCALE);
    assert_eq!(d.len(), 6);
    assert_eq!(cart.parts[minecart::INNER].rotation_point_y, 4.0 - -0.1);

    let mut player = Model::new(ModelKind::BipedArmor);
    player.parts[biped::HEAD].show_model = false;
    player.parts[biped::BODY].is_hidden = true;
    let d = player.render(0.0, 0.0, 0.0, 0.0, 0.0, MODEL_SCALE);
    assert_eq!(d.len(), 5);
    assert!(player.post_render(biped::HEAD, MODEL_SCALE).is_none());
    assert!(player.post_render(biped::RIGHT_ARM, MODEL_SCALE).is_some());
    assert!(player.render_cloak(MODEL_SCALE).is_some());
    assert!(
        Model::new(ModelKind::Pig)
            .render_cloak(MODEL_SCALE)
            .is_none()
    );
}

#[test]
fn kinds_round_trip_and_textures() {
    for k in ModelKind::ALL {
        assert_eq!(ModelKind::from_name(k.name()), Some(k));
        assert!(k.default_texture().ends_with(".png"));
        // Every draw-list index is a part.
        let m = Model::new(k);
        assert!(k.render_list().iter().all(|&(i, _)| i < m.parts.len()));
    }
    assert_eq!(ModelKind::from_name("sheep_fur"), Some(ModelKind::SheepFur));
    assert_eq!(ModelKind::from_name("bed"), None);
}

#[test]
fn mesh_cache_builds_once() {
    let mut meshes = Assets::<Mesh>::default();
    let mut cache = ModelMeshes::default();
    let a = cache.get(ModelKind::Pig, &mut meshes).to_vec();
    let b = cache.get(ModelKind::Pig, &mut meshes).to_vec();
    assert_eq!(a.len(), 6);
    assert_eq!(a, b);
    assert_eq!(meshes.len(), 6);
}

/// A part's transform puts its box where the Java matrix stack does: the
/// pig's head (rotation point (0, 12, -6), box -4..4 × -4..4 × -8..0) at
/// rest spans z -14..-6 texels, scaled by 0.0625.
#[test]
fn part_transform_places_the_box() {
    let m = Model::new(ModelKind::Pig);
    let t = part_transform(&m.parts[quadruped::HEAD]);
    let near = t.transform_point(Vec3::new(0.0, 0.0, 0.0));
    let far = t.transform_point(Vec3::new(0.0, 0.0, -8.0 * MODEL_SCALE));
    assert_eq!(near, Vec3::new(0.0, 12.0, -6.0) * MODEL_SCALE);
    assert!((far - Vec3::new(0.0, 12.0, -14.0) * MODEL_SCALE).length() < 1e-6);
}

/// Every model's `render` draw list (which parts, in which order, with
/// `render` or `renderWithRotation`), against the Java `render` methods'
/// call sequences written out by hand (the oracle cannot run them: they
/// need GL).
#[test]
fn draw_lists_match_the_java_render_methods() {
    const R: RenderMode = RenderMode::Render;
    const W: RenderMode = RenderMode::WithRotation;
    // src: ModelBiped.java:60-66
    let biped: &[(&str, RenderMode)] = &[
        ("bipedHead", R),
        ("bipedBody", R),
        ("bipedRightArm", R),
        ("bipedLeftArm", R),
        ("bipedRightLeg", R),
        ("bipedLeftLeg", R),
        ("bipedHeadwear", R),
    ];
    // src: ModelQuadruped.java:33-38
    let quad: &[(&str, RenderMode)] = &[
        ("head", R),
        ("body", R),
        ("leg1", R),
        ("leg2", R),
        ("leg3", R),
        ("leg4", R),
    ];
    // src: ModelCow.java:39-42 (super.render first)
    let cow: Vec<(&str, RenderMode)> = quad
        .iter()
        .copied()
        .chain([("horn1", R), ("horn2", R), ("udders", R)])
        .collect();
    // src: ModelChicken.java:43-50
    let chicken: &[(&str, RenderMode)] = &[
        ("head", R),
        ("bill", R),
        ("chin", R),
        ("body", R),
        ("rightLeg", R),
        ("leftLeg", R),
        ("rightWing", R),
        ("leftWing", R),
    ];
    // src: ModelCreeper.java:43-48 (no unusedCreeperHeadwear)
    let creeper: &[(&str, RenderMode)] = quad;
    // src: ModelSpider.java:56-66
    let spider: Vec<(&str, RenderMode)> = [
        "spiderHead",
        "spiderNeck",
        "spiderBody",
        "spiderLeg1",
        "spiderLeg2",
        "spiderLeg3",
        "spiderLeg4",
        "spiderLeg5",
        "spiderLeg6",
        "spiderLeg7",
        "spiderLeg8",
    ]
    .map(|n| (n, R))
    .to_vec();
    // src: ModelWolf.java:59-69
    let wolf: &[(&str, RenderMode)] = &[
        ("wolfHeadMain", W),
        ("wolfBody", R),
        ("wolfLeg1", R),
        ("wolfLeg2", R),
        ("wolfLeg3", R),
        ("wolfLeg4", R),
        ("wolfRightEar", W),
        ("wolfLeftEar", W),
        ("wolfSnout", W),
        ("wolfTail", W),
        ("wolfMane", R),
    ];
    // src: ModelSlime.java:30-35
    let slime: &[(&str, RenderMode)] = &[
        ("slimeBodies", R),
        ("slimeRightEye", R),
        ("slimeLeftEye", R),
        ("slimeMouth", R),
    ];
    let indexed = |first: &'static str, array: &str, n: usize| {
        let mut v = vec![(first.to_string(), R)];
        v.extend((0..n).map(|i| (format!("{array}[{i}]"), R)));
        v
    };
    let owned = |l: &[(&str, RenderMode)]| -> Vec<(String, RenderMode)> {
        l.iter().map(|&(n, m)| (n.to_string(), m)).collect()
    };
    let want = |k: ModelKind| -> Vec<(String, RenderMode)> {
        match k {
            ModelKind::Biped
            | ModelKind::BipedArmor
            | ModelKind::BipedArmorChestplate
            | ModelKind::Zombie
            | ModelKind::Skeleton => owned(biped),
            ModelKind::Pig | ModelKind::PigSaddle | ModelKind::Sheep | ModelKind::SheepFur => {
                owned(quad)
            }
            ModelKind::Cow => owned(&cow),
            ModelKind::Chicken => owned(chicken),
            ModelKind::Creeper | ModelKind::CreeperArmor => owned(creeper),
            ModelKind::Spider => owned(&spider),
            ModelKind::Wolf => owned(wolf),
            ModelKind::Slime => owned(slime),
            ModelKind::SlimeOuter => owned(&slime[..1]),
            // src: ModelGhast.java:39-43
            ModelKind::Ghast => indexed("body", "tentacles", 9),
            // src: ModelSquid.java:38-42
            ModelKind::Squid => indexed("squidBody", "squidTentacles", 8),
            // src: ModelMinecart.java:39-41 (sideModels[0..6))
            ModelKind::Minecart => (0..6).map(|i| (format!("sideModels[{i}]"), R)).collect(),
            // src: ModelBoat.java:33-35
            ModelKind::Boat => (0..5).map(|i| (format!("boatSides[{i}]"), R)).collect(),
        }
    };
    for k in ModelKind::ALL {
        let mut m = Model::new(k);
        let got: Vec<(String, RenderMode)> = m
            .render(1.3, 0.7, 25.5, 30.0, -20.0, MODEL_SCALE)
            .iter()
            .map(|d| (m.parts[d.part].name.to_string(), d.mode))
            .collect();
        assert_eq!(got, want(k), "{}", k.name());
    }
}
