//! Tests of the transcribed parts of `ItemRenderer` (no GPU): the
//! equipped-progress tick, the swing arcs and transforms against the Java's
//! GL call sequences written out with glam, the extruded icon, the overlays'
//! quads and texture rectangles, the map's pixels.

use bevy::math::{Mat4, Quat, Vec3};
use mc_core::math::{sin, sqrt_float};
use mc_sim::item::ItemStack;

use super::first_person::{
    FirstPersonDraw, FirstPersonInput, HeldStack, map_tilt, render_item_in_first_person,
    swing_arcs, swing_rotation_arcs,
};
use super::map::{icon_quads, map_pixels};
use super::overlays::{
    FIRE_TEXTURE, InsideBlock, OverlayInput, OverlayKind, inside_block_texture, inside_block_uv,
    render_fire_in_first_person, render_inside_of_block, render_overlays,
    render_warped_texture_overlay,
};
use super::render_item::{
    self, ItemAtlas, extruded_icon, flat_item_matrix, get_color_from_damage, is_full_3d,
    should_rotate_around_when_rendering,
};
use super::{ItemRenderer, same_object};

const DIRT: i32 = 3;
const PLANKS: i32 = 5;
const STICK: i32 = 280;
const IRON_SWORD: i32 = 267;
const IRON_PICKAXE: i32 = 257;

fn stack(id: i32, n: i32, dmg: i32) -> Option<ItemStack> {
    Some(ItemStack::new(id, n, dmg))
}

fn approx(a: Mat4, b: Mat4) -> bool {
    a.abs_diff_eq(b, 1e-5)
}

/// `glRotatef(angle, axis)` as glam builds it (independent of `GlMatrix`).
fn rot(angle: f32, x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_quat(Quat::from_axis_angle(
        Vec3::new(x, y, z).normalize(),
        angle.to_radians(),
    ))
}

fn tr(x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_translation(Vec3::new(x, y, z))
}

fn sc(s: f32) -> Mat4 {
    Mat4::from_scale(Vec3::splat(s))
}

// ---- updateEquippedItem -------------------------------------------------

#[test]
fn empty_hand_rises_by_0_4_a_tick() {
    let mut ir = ItemRenderer::default();
    let mut seen = Vec::new();
    for _ in 0..4 {
        ir.update_equipped_item(0, None);
        seen.push(ir.equipped_progress);
    }
    // 0 → 0.4 → 0.8 → 1.0 (the step is clamped to ±0.4)
    assert_eq!(seen, vec![0.4, 0.8, 1.0, 1.0]);
    assert_eq!(ir.prev_equipped_progress, 1.0);
    // The lerp of renderItemInFirstPerson.
    let mut ir = ItemRenderer::default();
    ir.update_equipped_item(0, None);
    assert_eq!(ir.equipped(0.5), 0.2);
}

#[test]
fn switching_items_lowers_then_raises() {
    let mut ir = ItemRenderer::default();
    for _ in 0..3 {
        ir.update_equipped_item(0, stack(DIRT, 10, 0));
    }
    // Slot 0 held dirt from the start: none → dirt is a change (var4 false
    // until the progress drops under 0.1, then the dirt is taken).
    assert_eq!(ir.item_to_render, stack(DIRT, 10, 0));
    for _ in 0..3 {
        ir.update_equipped_item(0, stack(DIRT, 10, 0));
    }
    assert_eq!(ir.equipped_progress, 1.0);
    // Wheel to slot 1 with planks: down by 0.4 a tick, the planks are taken
    // once the progress is under 0.1, then up again.
    let mut trace = Vec::new();
    for _ in 0..6 {
        ir.update_equipped_item(1, stack(PLANKS, 10, 0));
        trace.push((ir.equipped_progress, ir.item_to_render.map(|s| s.item_id)));
    }
    let p: Vec<f32> = trace.iter().map(|t| t.0).collect();
    // The step is toward the target 0: 1 → 0.6 → 0.2 → 0 (not -0.2).
    let expect = [0.6f32, 0.6 - 0.4, 0.0];
    assert_eq!(&p[..3], &expect);
    assert_eq!(trace[0].1, Some(DIRT));
    assert_eq!(trace[1].1, Some(DIRT));
    assert_eq!(trace[2].1, Some(PLANKS));
    assert_eq!(ir.equipped_item_slot, 1);
    assert!(p[3] > p[2] && p[5] == 1.0);
}

#[test]
fn a_tool_worn_in_place_stays_raised() {
    let mut ir = ItemRenderer::default();
    for _ in 0..8 {
        ir.update_equipped_item(2, stack(IRON_SWORD, 1, 0));
    }
    assert_eq!(ir.equipped_progress, 1.0);
    // The same object, damaged in place: identity holds.
    ir.update_equipped_item(2, stack(IRON_SWORD, 1, 5));
    assert_eq!(ir.equipped_progress, 1.0);
    assert_eq!(ir.item_to_render, stack(IRON_SWORD, 1, 5));
    // Placing a block from a stack (size changes in place) is the same object.
    let mut ir = ItemRenderer::default();
    for _ in 0..8 {
        ir.update_equipped_item(0, stack(DIRT, 10, 0));
    }
    ir.update_equipped_item(0, stack(DIRT, 9, 0));
    assert_eq!(ir.equipped_progress, 1.0);
    // The last block placed: the slot empties and the hand lowers.
    ir.update_equipped_item(0, None);
    assert!((ir.equipped_progress - 0.6).abs() < 1e-6);
}

#[test]
fn an_equal_stack_in_another_slot_dips_one_tick_late() {
    // ItemRenderer.java:415-423: another object with the same id and damage
    // is accepted at once (line 420), but the next tick the slot check
    // fails while the objects are identical, and the hand dips.
    let mut ir = ItemRenderer::default();
    for _ in 0..8 {
        ir.update_equipped_item(0, stack(DIRT, 10, 0));
    }
    ir.update_equipped_item(1, stack(DIRT, 5, 0));
    assert_eq!(ir.equipped_progress, 1.0);
    assert_eq!(ir.item_to_render, stack(DIRT, 5, 0));
    assert_eq!(ir.equipped_item_slot, 0);
    ir.update_equipped_item(1, stack(DIRT, 5, 0));
    assert!((ir.equipped_progress - 0.6).abs() < 1e-6);
    ir.update_equipped_item(1, stack(DIRT, 5, 0));
    ir.update_equipped_item(1, stack(DIRT, 5, 0));
    assert_eq!(ir.equipped_item_slot, 1);
}

#[test]
fn reset_drops_the_progress_to_zero() {
    let mut ir = ItemRenderer::default();
    for _ in 0..4 {
        ir.update_equipped_item(0, stack(DIRT, 10, 0));
    }
    ir.reset_equipped_progress();
    assert_eq!(ir.equipped_progress, 0.0);
    ir.update_equipped_item(0, stack(DIRT, 10, 0));
    assert!((ir.equipped_progress - 0.4).abs() < 1e-6);
}

#[test]
fn identity_stand_in() {
    let a = ItemStack::new(DIRT, 3, 0);
    let b = ItemStack::new(DIRT, 7, 0);
    assert!(same_object(&a, 0, &b, 0));
    assert!(!same_object(&a, 0, &b, 1));
    // Wool of another colour is another object.
    let w0 = ItemStack::new(35, 1, 0);
    let w1 = ItemStack::new(35, 1, 4);
    assert!(!same_object(&w0, 0, &w1, 0));
    // A damageable tool keeps its identity across damage.
    let p0 = ItemStack::new(IRON_PICKAXE, 1, 0);
    let p1 = ItemStack::new(IRON_PICKAXE, 1, 30);
    assert!(same_object(&p0, 3, &p1, 3));
}

// ---- swing arcs and transforms --------------------------------------------

#[test]
fn swing_arcs_are_the_java_formulas() {
    for &p in &[0.0f32, 0.125, 0.25, 0.5, 0.75, 0.875] {
        let (a, b, c) = swing_arcs(p);
        assert_eq!(a, sin(p * 3.1415927));
        assert_eq!(b, sin(sqrt_float(p) * 3.1415927));
        assert_eq!(c, sin(sqrt_float(p) * 3.1415927 * 2.0));
        let (d, e) = swing_rotation_arcs(p);
        assert_eq!(d, sin(p * p * 3.1415927));
        assert_eq!(e, b);
    }
    assert_eq!(swing_arcs(0.0), (0.0, 0.0, 0.0));
    // The table's quarter turn is 1.
    assert!((swing_arcs(0.5).0 - 1.0).abs() < 1e-6);
}

#[test]
fn map_tilt_clamps_and_eases() {
    // pitch 49.5 → 1 - 1.1 + 0.1 = 0 → -cos(0)·0.5 + 0.5 = 0
    assert_eq!(map_tilt(49.5), 0.0);
    // pitch -90 → clamped to 1 → -cos(π)·0.5 + 0.5 = 1
    assert!((map_tilt(-90.0) - 1.0).abs() < 1e-6);
    assert!(
        (map_tilt(27.0)
            - (-mc_core::math::cos((1.0f32 - 27.0 / 45.0 + 0.1) * 3.1415927) * 0.5 + 0.5))
            .abs()
            < 1e-7
    );
}

fn input(stack: Option<HeldStack>, swing: f32, equipped: f32) -> FirstPersonInput {
    FirstPersonInput {
        equipped,
        pitch: 10.0,
        yaw: 30.0,
        swing,
        light: 0.8,
        stack,
    }
}

fn held(id: i32) -> HeldStack {
    HeldStack {
        item_id: id,
        damage: 0,
        color: 0xffffff,
        rotate_around: id == render_item::FISHING_ROD,
        is_map: id == render_item::MAP_ITEM,
    }
}

#[test]
fn item_transform_is_the_java_call_sequence() {
    // ItemRenderer.java:220-238 with swing s and equip e, from identity.
    for &(s, e) in &[(0.0f32, 1.0f32), (0.3, 1.0), (0.0, 0.4), (0.7, 0.6)] {
        let f = render_item_in_first_person(Mat4::IDENTITY, &input(Some(held(DIRT)), s, e));
        let [FirstPersonDraw::Item(m)] = f.draws[..] else {
            panic!("{:?}", f.draws);
        };
        let v14 = 0.8f32;
        let (a9, a10) = (sin(s * 3.1415927), sin(sqrt_float(s) * 3.1415927));
        let up = sin(sqrt_float(s) * 3.1415927 * 2.0);
        let r9 = sin(s * s * 3.1415927);
        let expect = tr(-a10 * 0.4, up * 0.2, -a9 * 0.2)
            * tr(0.7 * v14, -0.65 * v14 - (1.0 - e) * 0.6, -0.9 * v14)
            * rot(45.0, 0.0, 1.0, 0.0)
            * rot(-r9 * 20.0, 0.0, 1.0, 0.0)
            * rot(-a10 * 20.0, 0.0, 0.0, 1.0)
            * rot(-a10 * 80.0, 1.0, 0.0, 0.0)
            * sc(0.4);
        assert!(approx(m, expect), "s {s} e {e}\n{m}\n{expect}");
        // glColor: light × the item's colour.
        assert_eq!(f.color, [0.8, 0.8, 0.8, 1.0]);
    }
    // At rest the block's centre sits at (0.56, -0.52, -0.72) in eye space.
    let f = render_item_in_first_person(Mat4::IDENTITY, &input(Some(held(DIRT)), 0.0, 1.0));
    let FirstPersonDraw::Item(m) = f.draws[0] else {
        panic!()
    };
    let c = m.transform_point3(Vec3::ZERO);
    assert!((c - Vec3::new(0.56, -0.52, -0.72)).length() < 1e-6, "{c}");
    // The fishing rod turns around (ItemRenderer.java:237-239).
    let g = render_item_in_first_person(
        Mat4::IDENTITY,
        &input(Some(held(render_item::FISHING_ROD)), 0.0, 1.0),
    );
    let FirstPersonDraw::Item(r) = g.draws[0] else {
        panic!()
    };
    assert!(approx(r, m * rot(180.0, 0.0, 1.0, 0.0)));
}

#[test]
fn empty_hand_transform_is_the_java_call_sequence() {
    // ItemRenderer.java:244-269.
    let (s, e) = (0.4f32, 0.8f32);
    let f = render_item_in_first_person(Mat4::IDENTITY, &input(None, s, e));
    let [FirstPersonDraw::Arm(m)] = f.draws[..] else {
        panic!("{:?}", f.draws);
    };
    let v14 = 0.8f32;
    let (a9, a10) = (sin(s * 3.1415927), sin(sqrt_float(s) * 3.1415927));
    let up = sin(sqrt_float(s) * 3.1415927 * 2.0);
    let r9 = sin(s * s * 3.1415927);
    let expect = tr(-a10 * 0.3, up * 0.4, -a9 * 0.4)
        * tr(0.8 * v14, -0.75 * v14 - (1.0 - e) * 0.6, -0.9 * v14)
        * rot(45.0, 0.0, 1.0, 0.0)
        * rot(a10 * 70.0, 0.0, 1.0, 0.0)
        * rot(-r9 * 20.0, 0.0, 0.0, 1.0)
        * tr(-1.0, 3.6, 3.5)
        * rot(120.0, 0.0, 0.0, 1.0)
        * rot(200.0, 1.0, 0.0, 0.0)
        * rot(-135.0, 0.0, 1.0, 0.0)
        * tr(5.6, 0.0, 0.0);
    assert!(approx(m, expect), "{m}\n{expect}");
    assert_eq!(f.color, [0.8, 0.8, 0.8, 1.0]);
}

#[test]
fn map_transform_and_two_arms() {
    let f = render_item_in_first_person(
        Mat4::IDENTITY,
        &input(Some(held(render_item::MAP_ITEM)), 0.0, 1.0),
    );
    assert_eq!(f.draws.len(), 3);
    let (FirstPersonDraw::Arm(a0), FirstPersonDraw::Arm(a1), FirstPersonDraw::Map(m)) =
        (f.draws[0], f.draws[1], f.draws[2])
    else {
        panic!("{:?}", f.draws);
    };
    let t = map_tilt(10.0);
    let base = tr(0.0, 0.0, 0.0)
        * tr(0.0, 0.0 * 0.8 - 0.0 * 1.2 - t * 0.5 + 0.04, -0.9 * 0.8)
        * rot(90.0, 0.0, 1.0, 0.0)
        * rot(t * -85.0, 0.0, 0.0, 1.0);
    for (k, a) in [(-1.0f32, a0), (1.0, a1)] {
        let e = base
            * tr(-0.0, -0.6, 1.1 * k)
            * rot(-45.0 * k, 1.0, 0.0, 0.0)
            * rot(-90.0, 0.0, 0.0, 1.0)
            * rot(59.0, 0.0, 0.0, 1.0)
            * rot(-65.0 * k, 0.0, 1.0, 0.0);
        assert!(approx(a, e), "arm {k}");
    }
    let e = base
        * sc(0.38)
        * rot(90.0, 0.0, 1.0, 0.0)
        * rot(180.0, 0.0, 0.0, 1.0)
        * tr(-1.0, -1.0, 0.0)
        * sc(0.015625);
    assert!(approx(m, e));
}

#[test]
fn hurt_and_bobbing_matrix_is_the_base() {
    let base = tr(0.01, -0.02, 0.0) * rot(3.0, 0.0, 0.0, 1.0);
    let a = render_item_in_first_person(base, &input(Some(held(STICK)), 0.2, 1.0));
    let b = render_item_in_first_person(Mat4::IDENTITY, &input(Some(held(STICK)), 0.2, 1.0));
    let (FirstPersonDraw::Item(ma), FirstPersonDraw::Item(mb)) = (a.draws[0], b.draws[0]) else {
        panic!()
    };
    assert!(approx(ma, base * mb));
    // The light sees the base turned by pitch and yaw (ItemRenderer.java:133-137).
    assert!(approx(
        a.light,
        base * rot(10.0, 1.0, 0.0, 0.0) * rot(30.0, 0.0, 1.0, 0.0)
    ));
    let [l0, l1] = a.light_dirs();
    assert!((l0.length() - 1.0).abs() < 1e-5 && (l1.length() - 1.0).abs() < 1e-5);
}

#[test]
fn flat_item_matrix_is_render_items() {
    // ItemRenderer.java:42-47.
    let e = tr(-0.0, -0.3, 0.0)
        * sc(1.5)
        * rot(50.0, 0.0, 1.0, 0.0)
        * rot(335.0, 0.0, 0.0, 1.0)
        * tr(-0.9375, -0.0625, 0.0);
    assert!(approx(flat_item_matrix(), e));
}

// ---- the extruded icon ---------------------------------------------------

#[test]
fn extruded_icon_has_the_java_quads() {
    let t = extruded_icon(STICK_ICON);
    // front, back, 4 × 16 slabs
    assert_eq!(t.positions.len(), 66 * 4);
    // The front face at z 0 and the back at z -1/16.
    assert_eq!(t.positions[0], [0.0, 0.0, 0.0]);
    assert_eq!(t.positions[4], [0.0, 1.0, -0.0625]);
    // Normals: +z, -z, then the -x slabs, the +x slabs (packed: the same
    // bytes as -x, vanilla bug), +y, -y (sign-extended into z).
    assert_eq!(t.normals[0], [0, 0, 127]);
    assert_eq!(t.normals[4], [0, 0, -127]);
    assert_eq!(t.normals[8], [-128, -1, -1]);
    assert_eq!(t.normals[8 + 64], [-128, -1, -1]);
    assert_eq!(t.normals[8 + 128], [0, 127, 0]);
    assert_eq!(t.normals[8 + 192], [0, -127, -1]);
}

/// `Item.stick`'s icon (`setIconCoord(5, 3)`).
const STICK_ICON: i32 = 5 + 3 * 16;

#[test]
fn every_slab_samples_the_texel_column_or_row_it_bounds() {
    // For a synthetic icon at index i: slab k of the -x and +x sides sits
    // on an edge of texel column 15 - k's cell on the front face (x = 0 is
    // the icon's right edge: the front face mirrors it), and samples that
    // column; the y slabs likewise with rows (y up, v down).
    for icon in [0, 17, STICK_ICON, 255] {
        let t = extruded_icon(icon);
        let col0 = (icon % 16 * 16) as f32;
        let row0 = (icon / 16 * 16) as f32;
        let texel_u = |u: f32| (u * 256.0 - col0).floor() as i32;
        let texel_v = |v: f32| (v * 256.0 - row0).floor() as i32;
        for k in 0..16 {
            let nx = 8 + k * 4;
            let px = 8 + 64 + k * 4;
            let py = 8 + 128 + k * 4;
            let ny = 8 + 192 + k * 4;
            let c = 15 - k as i32;
            assert_eq!(texel_u(t.uvs[nx][0]), c, "icon {icon} -x slab {k}");
            assert_eq!(texel_u(t.uvs[px][0]), c, "icon {icon} +x slab {k}");
            // The -x slab is the cell's left edge, the +x slab its right.
            assert_eq!(t.positions[nx][0], k as f32 / 16.0);
            assert_eq!(t.positions[px][0], k as f32 / 16.0 + 0.0625);
            assert_eq!(texel_v(t.uvs[py][1]), c, "icon {icon} +y slab {k}");
            assert_eq!(texel_v(t.uvs[ny][1]), c, "icon {icon} -y slab {k}");
            assert_eq!(t.positions[py][1], k as f32 / 16.0 + 0.0625);
            assert_eq!(t.positions[ny][1], k as f32 / 16.0);
        }
        // The front face: x 0 ↔ the right edge (u + 15.99/256).
        assert_eq!(t.uvs[0][0], (col0 + 15.99) / 256.0);
        assert_eq!(t.uvs[1][0], col0 / 256.0);
    }
}

#[test]
fn a_synthetic_alpha_mask_shows_its_outline_on_the_slabs() {
    // A 16×16 mask: an L of opaque texels. With the alpha test, a side slab
    // shows a cell where its texel is opaque; every opaque texel (column c,
    // row r) shows on both x slabs of column c and both y slabs of row r,
    // at the cell (15 - c, 15 - r)'s edges of the front face.
    let mut mask = [[false; 16]; 16];
    for row in &mut mask[2..14] {
        row[3] = true;
    }
    for cell in &mut mask[13][3..12] {
        *cell = true;
    }
    let t = extruded_icon(0);
    let mut shown = 0;
    // Slab quads after the two faces; each spans its whole edge (16 cells
    // along it), sampling a fixed column (x slabs) or row (y slabs).
    for q in 0..64 {
        let i = 8 + q * 4;
        let x_slab = q < 32;
        for cell in 0..16 {
            // The texel at the cell's centre along the slab.
            let f = (cell as f32 + 0.5) / 16.0;
            let (col, row) = if x_slab {
                let u = t.uvs[i][0];
                // y runs from v8 (y 0) to v7 (y 1)
                let (v_lo, v_hi) = (t.uvs[i][1], t.uvs[i + 2][1]);
                let v = v_lo + (v_hi - v_lo) * f;
                ((u * 256.0).floor() as usize, (v * 256.0).floor() as usize)
            } else {
                let v = t.uvs[i][1];
                let (u_lo, u_hi) = (t.uvs[i][0], t.uvs[i + 1][0]);
                let u = u_lo + (u_hi - u_lo) * f;
                ((u * 256.0).floor() as usize, (v * 256.0).floor() as usize)
            };
            if mask[row][col] {
                shown += 1;
            }
        }
    }
    let opaque = mask.iter().flatten().filter(|&&b| b).count();
    // Each opaque texel shows on four slabs (two x, two y).
    assert_eq!(shown, opaque * 4);
}

#[test]
fn item_methods() {
    let def = |id| mc_sim::item::item(id).unwrap();
    assert!(is_full_3d(def(STICK)));
    assert!(is_full_3d(def(352))); // bone, setFull3D
    assert!(is_full_3d(def(IRON_SWORD)));
    assert!(is_full_3d(def(IRON_PICKAXE)));
    assert!(is_full_3d(def(render_item::FISHING_ROD)));
    assert!(!is_full_3d(def(262))); // arrow
    assert!(!is_full_3d(def(DIRT)));
    assert!(should_rotate_around_when_rendering(def(
        render_item::FISHING_ROD
    )));
    assert!(!should_rotate_around_when_rendering(def(STICK)));
    // ItemLeaves: spruce, birch, oak; everything else white.
    assert_eq!(get_color_from_damage(def(18), 1), 0x619961);
    assert_eq!(get_color_from_damage(def(18), 2), 0x80a755);
    assert_eq!(get_color_from_damage(def(18), 0), 0x48b518);
    assert_eq!(get_color_from_damage(def(STICK), 0), 0xffffff);
    assert_eq!(ItemAtlas::of(DIRT), ItemAtlas::Terrain);
    assert_eq!(ItemAtlas::of(STICK), ItemAtlas::Items);
    // 3D: cubes (0), stairs (10), fence (11), cactus (13), piston (16);
    // flat: torch (2), sapling (1), crops (6), every item.
    assert!(render_item::is_block_3d(DIRT));
    assert!(render_item::is_block_3d(53));
    assert!(render_item::is_block_3d(85));
    assert!(render_item::is_block_3d(81));
    assert!(render_item::is_block_3d(33));
    assert!(!render_item::is_block_3d(50));
    assert!(!render_item::is_block_3d(6));
    assert!(!render_item::is_block_3d(STICK));
}

// ---- overlays --------------------------------------------------------------

#[test]
fn inside_block_uv_misses_the_tile_size() {
    // Stone (side texture 1): u from 1/256 - 1/128, not 16/256 (vanilla bug).
    let (u0, u1, v0, v1) = inside_block_uv(1);
    assert_eq!(u0, 1.0 / 256.0 - 0.0078125);
    assert_eq!(u1, (1.0 + 15.99) / 256.0 + 0.0078125);
    assert_eq!(v0, 0.0 / 256.0 - 0.0078125);
    assert_eq!(v1, 15.99 / 256.0 + 0.0078125);
    // Grass side (3) and sand (18): the window's corner is texel (i % 16, i / 16).
    let (u0, _, v0, _) = inside_block_uv(18);
    assert_eq!(u0, 2.0 / 256.0 - 0.0078125);
    assert_eq!(v0, 1.0 / 256.0 - 0.0078125);
    // The quad: z -0.5, (-1, -1)..(1, 1), u flipped against x.
    let d = render_inside_of_block(1);
    assert_eq!(d.kind, OverlayKind::InsideBlock);
    assert_eq!(d.color, [0.1, 0.1, 0.1, 0.5]);
    assert_eq!(d.batch.positions[0], [-1.0, -1.0, -0.5]);
    assert_eq!(d.batch.positions[2], [1.0, 1.0, -0.5]);
    let (u0, u1, v0, v1) = inside_block_uv(1);
    assert_eq!(d.batch.uvs, vec![[u1, v1], [u0, v1], [u0, v0], [u1, v0]]);
}

#[test]
fn inside_block_choice() {
    // In a normal cube: its side texture, drawn twice.
    let r = inside_block_texture(
        [0.5, 64.62, 0.5],
        0.6,
        1.8,
        |_, _, _| (1, true),
        |id| Some(id * 10),
    );
    assert_eq!(
        r,
        Some(InsideBlock {
            texture: 10,
            draws: 2
        })
    );
    // Not in one: the last of the 8 samples (around the block coordinates,
    // ±0.27 sideways, ±0.18 up and down) that is a normal cube.
    let r = inside_block_texture(
        [0.5, 64.62, 0.5],
        0.6,
        1.8,
        |x, y, z| match (x, y, z) {
            (0, 64, 0) => (0, false),
            (-1, 63, -1) => (4, true),
            (-1, 63, 0) => (5, true),
            _ => (0, false),
        },
        |id| (id != 0).then_some(id * 10),
    );
    // var7 = 4: (x -0.27 → -1, y -0.18 → 63, z +0.27 → 0) is the last cube.
    assert_eq!(
        r,
        Some(InsideBlock {
            texture: 50,
            draws: 1
        })
    );
    // Nothing found and air at the player: no overlay.
    let r = inside_block_texture(
        [0.5, 64.62, 0.5],
        0.6,
        1.8,
        |_, _, _| (0, false),
        |id| (id != 0).then_some(id),
    );
    assert_eq!(r, None);
}

#[test]
fn fire_overlay_quads() {
    let d = render_fire_in_first_person();
    assert_eq!(d.color, [1.0, 1.0, 1.0, 0.9]);
    assert_eq!(d.batch.len(), 8);
    // Tile 31 then 47 (the one below).
    let u = |i: i32| (((i & 15) << 4) as f32 / 256.0, ((i & 240) as f32) / 256.0);
    assert_eq!(
        d.batch.uvs[1],
        [
            u(FIRE_TEXTURE).0,
            (u(FIRE_TEXTURE).1 * 256.0 + 15.99) / 256.0
        ]
    );
    assert_eq!(
        d.batch.uvs[5][1],
        (u(FIRE_TEXTURE + 16).1 * 256.0 + 15.99) / 256.0
    );
    // The first quad: translate(0.24, -0.3, 0), rotate(-10°, y) of (-0.5, -0.5, -0.5).
    let m = tr(0.24, -0.3, 0.0) * rot(-10.0, 0.0, 1.0, 0.0);
    let p = m.transform_point3(Vec3::new(-0.5, -0.5, -0.5));
    assert!((Vec3::from(d.batch.positions[0]) - p).length() < 1e-6);
    let m = tr(-0.24, -0.3, 0.0) * rot(10.0, 0.0, 1.0, 0.0);
    let p = m.transform_point3(Vec3::new(0.5, 0.5, -0.5));
    assert!((Vec3::from(d.batch.positions[6]) - p).length() < 1e-6);
}

#[test]
fn water_overlay_scrolls_with_the_rotation() {
    let d = render_warped_texture_overlay(0.7, 90.0, -32.0);
    assert_eq!(d.color, [0.7, 0.7, 0.7, 0.5]);
    let (du, dv) = (-90.0f32 / 64.0, -32.0f32 / 64.0);
    assert_eq!(
        d.batch.uvs,
        vec![
            [4.0 + du, 4.0 + dv],
            [du, 4.0 + dv],
            [du, dv],
            [4.0 + du, dv]
        ]
    );
}

#[test]
fn overlays_in_order() {
    let all = render_overlays(&OverlayInput {
        burning: true,
        inside: Some(InsideBlock {
            texture: 1,
            draws: 2,
        }),
        in_water: true,
        brightness: 1.0,
        yaw: 0.0,
        pitch: 0.0,
    });
    let kinds: Vec<_> = all.iter().map(|d| d.kind).collect();
    assert_eq!(
        kinds,
        vec![
            OverlayKind::Fire,
            OverlayKind::InsideBlock,
            OverlayKind::InsideBlock,
            OverlayKind::Water
        ]
    );
}

// ---- the map ----------------------------------------------------------------

#[test]
fn map_pixels_follow_map_item_renderer() {
    let mut colors = vec![0u8; 16384];
    // grass (index 1), shade 2 → ×255/255; shade 0 → ×180/255; shade 1, 3 → 220
    colors[0] = 4 + 2;
    colors[1] = 4;
    colors[2] = 4 + 1;
    let px = map_pixels(&colors, false);
    let grass = 8368696i32;
    let ch = |c: i32, s: i32| ((c * s) / 255) as u8;
    let (r, g, b) = (grass >> 16 & 255, grass >> 8 & 255, grass & 255);
    assert_eq!(&px[0..4], &[ch(r, 255), ch(g, 255), ch(b, 255), 255]);
    assert_eq!(&px[4..8], &[ch(r, 180), ch(g, 180), ch(b, 180), 255]);
    assert_eq!(&px[8..12], &[ch(r, 220), ch(g, 220), ch(b, 220), 255]);
    // Unexplored: a checkerboard of alpha 16 and 24, black.
    assert_eq!(&px[12..16], &[0, 0, 0, 24]);
    assert_eq!(&px[16..20], &[0, 0, 0, 16]);
    // Row 1 starts shifted: pixel 128 is (128 + 1) & 1 = 1 → 24.
    assert_eq!(&px[128 * 4..128 * 4 + 4], &[0, 0, 0, 24]);
}

#[test]
fn map_icons_are_placed_as_the_java() {
    use mc_sim::world::map_storage::MapCoord;
    let b = icon_quads(&[MapCoord {
        icon_size: 0,
        center_x: 10,
        center_z: -20,
        icon_rotation: 4,
    }]);
    let m = tr(5.0 + 64.0, -10.0 + 64.0, -0.02)
        * rot(4.0 * 360.0 / 16.0, 0.0, 0.0, 1.0)
        * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
        * tr(-0.125, 0.125, 0.0);
    let p = m.transform_point3(Vec3::new(-1.0, 1.0, 0.0));
    assert!((Vec3::from(b.positions[0]) - p).length() < 1e-4);
    assert_eq!(b.uvs[0], [0.0, 0.0]);
    assert_eq!(b.uvs[2], [0.25, 0.25]);
}

// ---- renderItem through the hook ----------------------------------------------

#[test]
fn render_item_hook_draws_blocks_in_3d_and_items_flat() {
    use bevy::prelude::Assets;
    let mut meshes = Assets::<bevy::prelude::Mesh>::default();
    let mut cache = render_item::ItemMeshCache::default();
    let hook = render_item::HeldItemHook::default();
    let statics = mc_sim::block::BlockStatics::default();
    let colorizers = crate::textures::Colorizers::default();
    let args = |id: i32, icon: i32| render_item::RenderItemArgs {
        item_id: id,
        damage: 0,
        icon,
        brightness: 0.5,
        statics: &statics,
        colorizers: &colorizers,
    };
    // Dirt: renderBlockOnInventory, glColor = white × brightness.
    let d = (hook.render_item)(&mut cache, &mut meshes, &args(DIRT, 2)).unwrap();
    assert_eq!(d.atlas, ItemAtlas::Terrain);
    assert_eq!(d.matrix, Mat4::IDENTITY);
    assert_eq!(d.gl_color, Some([0.5, 0.5, 0.5, 1.0]));
    let again = (hook.render_item)(&mut cache, &mut meshes, &args(DIRT, 2)).unwrap();
    assert_eq!(again.mesh, d.mesh);
    // A stick: the extruded icon from items.png, renderItem's matrix, no colour.
    let s = (hook.render_item)(&mut cache, &mut meshes, &args(STICK, STICK_ICON)).unwrap();
    assert_eq!(s.atlas, ItemAtlas::Items);
    assert!(approx(s.matrix, flat_item_matrix()));
    assert_eq!(s.gl_color, None);
    // A torch (render type 2): flat, from terrain.png.
    let t = (hook.render_item)(&mut cache, &mut meshes, &args(50, 80)).unwrap();
    assert_eq!(t.atlas, ItemAtlas::Terrain);
    assert!(approx(t.matrix, flat_item_matrix()));
    // No such item.
    assert!((hook.render_item)(&mut cache, &mut meshes, &args(4000, 0)).is_none());
}
