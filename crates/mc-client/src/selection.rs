//! `RenderGlobal.drawSelectionBox` / `drawOutlinedBoundingBox`
//! (RenderGlobal.java:1146-1197): the outline around the block under the
//! cursor — the block's selected bounding box (after
//! `setBlockBoundsBasedOnState`) grown by 0.002, drawn as two line loops and
//! four verticals in black at alpha 0.4, 2 pixels wide, without depth writes.
//!
//! When: `renderWorld` calls it for a player view entity with
//! `objectMouseOver` set (EntityRenderer.java:507-513 in water, before the
//! translucent pass; :549-555 otherwise, after it); it draws only for a
//! block hit (`typeOfHit == TILE`) of a non-air block. Nothing depends on
//! the held item (the `ItemStack` argument is unused).
//!
//! Drawn with [`GlMaterial`] (blend (770, 771), no depth writes, fog
//! `setupFog(0)`) as wide lines ([`GlKey::wide_lines`]: wgpu draws 1-pixel
//! lines only, so each edge is a quad the shader widens to
//! `glLineWidth(2.0F)`'s 2 pixels along its minor screen axis, as GL
//! rasterises aliased wide lines). Sorted by the call site: with the eye in
//! water before WP-3.3's translucent pass ([`order::SELECTION_IN_WATER`]),
//! which then blends over it, otherwise after it and before the weather
//! ([`order::SELECTION`]). (Bevy gizmos were tried: they sort "always on
//! top", before the sky, and write depth, so the sky drawn later left the
//! clear colour in the lines.)

use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::DVec3;
use bevy::prelude::*;
use mc_core::Aabb;
use mc_sim::block::dispatch;
use mc_sim::world::World;
use mc_sim::world::ray_trace::{EnumMovingObjectType, MovingObjectPosition};

use crate::fog::FogState;
use crate::gl_material::{GlBlend, GlKey, GlMaterial, order, wide_line_mesh};
use crate::sky::ViewInfo;

/// The selection box of this frame, if any.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectionFrame {
    /// The expanded box offset by the interpolated `-lastTickPos`, as the
    /// Java draws it (in eye space; drawn at [`SelectionFrame::anchor`]):
    /// min and max corners.
    pub r#box: Option<(DVec3, DVec3)>,
    /// The eye.
    pub anchor: Vec3,
    /// `renderViewEntity.isInsideOfMaterial(Material.water)`: which of
    /// `renderWorld`'s two call sites draws the box.
    // src: EntityRenderer.java:507, :549
    pub in_water: bool,
}

/// `expand(0.002, 0.002, 0.002)` of the selected box.
// src: RenderGlobal.java:1154, :1161
pub const EXPAND: f32 = 0.002;

/// `drawSelectionBox(player, objectMouseOver, 0, item, partial)`'s box.
// src: RenderGlobal.java:1146-1169
pub fn compute(
    world: &mut World,
    view: &ViewInfo,
    mouse_over: Option<&MovingObjectPosition>,
    partial: f32,
) -> SelectionFrame {
    let Some(m) = mouse_over else {
        return SelectionFrame::default();
    };
    if !view.is_player || m.type_of_hit != EnumMovingObjectType::Tile {
        return SelectionFrame::default();
    }
    let (x, y, z) = (m.block_x, m.block_y, m.block_z);
    let var7 = world.get_block_id(x, y, z);
    if var7 <= 0 {
        return SelectionFrame::default();
    }
    dispatch::set_block_bounds_based_on_state(world, var7, x, y, z);
    let a = view.last_tick_interp(partial);
    let var6 = EXPAND as f64;
    let bb: Aabb = dispatch::get_selected_bounding_box_from_pool(world, var7, x, y, z)
        .expand(var6, var6, var6)
        .get_offset_bounding_box(-a.x, -a.y, -a.z);
    SelectionFrame {
        r#box: Some((
            DVec3::new(bb.min_x, bb.min_y, bb.min_z),
            DVec3::new(bb.max_x, bb.max_y, bb.max_z),
        )),
        anchor: Vec3::ZERO,
        in_water: view.in_water,
    }
}

/// `drawOutlinedBoundingBox`' lines: the bottom loop, the top loop
/// (`GL_LINE_STRIP`, 5 points each), the four verticals (`GL_LINES`).
// src: RenderGlobal.java:1171-1197
pub fn outline(min: Vec3, max: Vec3) -> ([Vec3; 5], [Vec3; 5], [(Vec3, Vec3); 4]) {
    let loop_at = |y: f32| {
        [
            Vec3::new(min.x, y, min.z),
            Vec3::new(max.x, y, min.z),
            Vec3::new(max.x, y, max.z),
            Vec3::new(min.x, y, max.z),
            Vec3::new(min.x, y, min.z),
        ]
    };
    let v = |x: f32, z: f32| (Vec3::new(x, min.y, z), Vec3::new(x, max.y, z));
    (
        loop_at(min.y),
        loop_at(max.y),
        [
            v(min.x, min.z),
            v(max.x, min.z),
            v(max.x, max.z),
            v(min.x, max.z),
        ],
    )
}

/// The outline's 12 segments (the `GL_LINE_STRIP`s split into segments).
pub fn segments(min: Vec3, max: Vec3) -> Vec<([f32; 3], [f32; 3])> {
    let (bottom, top, verticals) = outline(min, max);
    let mut v = Vec::with_capacity(12);
    for strip in [bottom, top] {
        for w in strip.windows(2) {
            v.push((w[0].to_array(), w[1].to_array()));
        }
    }
    for (a, b) in verticals {
        v.push((a.to_array(), b.to_array()));
    }
    v
}

/// `glLineWidth(2.0F)`.
// src: RenderGlobal.java:1151
pub const LINE_WIDTH: f32 = 2.0;

/// The box's mesh and material.
#[derive(Resource, Clone, Debug)]
pub struct SelectionHandles {
    /// The line mesh.
    pub mesh: Handle<Mesh>,
    /// The material.
    pub material: Handle<GlMaterial>,
}

/// Marks the selection box entity.
#[derive(Component, Clone, Copy, Debug)]
pub struct SelectionBox;

/// Spawns the (hidden) box: `glColor4f(0, 0, 0, 0.4)`, blend (770, 771),
/// `glLineWidth(2.0F)`, `glDepthMask(false)`, texturing off.
// src: RenderGlobal.java:1148-1153
pub fn spawn_selection(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
) {
    let mesh = meshes.add(wide_line_mesh(&segments(Vec3::ZERO, Vec3::ONE)));
    let mut m = GlMaterial::new(
        GlKey {
            blend: GlBlend::Alpha,
            depth_write: false,
            color_write: true,
            far: false,
            wide_lines: true,
        },
        order::SELECTION,
    );
    m.params.color = Vec4::new(0.0, 0.0, 0.0, 0.4);
    m.params.line_width = LINE_WIDTH;
    let material = materials.add(m);
    commands.spawn((
        Mesh3d(mesh.clone()),
        MeshMaterial3d(material.clone()),
        Transform::IDENTITY,
        Visibility::Hidden,
        NoFrustumCulling,
        SelectionBox,
    ));
    commands.insert_resource(SelectionHandles { mesh, material });
}

/// The box's draw order: `renderWorld` draws it before the translucent
/// terrain pass with the eye in water, after it otherwise.
// src: EntityRenderer.java:507-513, :549-555
pub fn sort_bias(in_water: bool) -> f32 {
    if in_water {
        order::SELECTION_IN_WATER
    } else {
        order::SELECTION
    }
}

/// Applies [`SelectionFrame`].
pub fn apply_selection(
    frame: Res<SelectionFrame>,
    fog: Res<FogState>,
    handles: Option<Res<SelectionHandles>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
    mut q: Query<(&mut Transform, &mut Visibility), With<SelectionBox>>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Sky);
    let Some(h) = handles else {
        return;
    };
    for (mut t, mut vis) in &mut q {
        let want = if frame.r#box.is_some() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        t.translation = frame.anchor;
    }
    let Some((min, max)) = frame.r#box else {
        return;
    };
    if let Some(mut m) = meshes.get_mut(&h.mesh) {
        *m = wide_line_mesh(&segments(min.as_vec3(), max.as_vec3()));
    }
    let bias = sort_bias(frame.in_water);
    let fog_v = fog.world.to_vec4();
    let fog_c = Vec4::new(fog.color[0], fog.color[1], fog.color[2], 1.0);
    let changed = materials.get(&h.material).is_some_and(|m| {
        m.sort_bias != bias || m.params.fog != fog_v || m.params.fog_color != fog_c
    });
    if changed && let Some(mut m) = materials.get_mut(&h.material) {
        m.sort_bias = bias;
        m.params.fog = fog_v;
        m.params.fog_color = fog_c;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_has_twelve_edges() {
        let (b, t, v) = outline(Vec3::ZERO, Vec3::ONE);
        assert_eq!(b[0], b[4]);
        assert_eq!(t[0], t[4]);
        assert!(b.iter().all(|p| p.y == 0.0) && t.iter().all(|p| p.y == 1.0));
        assert_eq!(v[1], (Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0)));
        let l = segments(Vec3::ZERO, Vec3::ONE);
        assert_eq!(l.len(), 12);
        assert_eq!(l[0], ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]));
        assert_eq!(l[11], ([0.0, 0.0, 1.0], [0.0, 1.0, 1.0]));
    }

    #[test]
    fn line_width_and_call_site_order() {
        assert_eq!(LINE_WIDTH, 2.0);
        assert_eq!(sort_bias(false), order::SELECTION);
        assert_eq!(sort_bias(true), order::SELECTION_IN_WATER);
        // In water: after the sky (the void plane is its last part), before
        // the translucent band; otherwise before snow, rain and clouds.
        const { assert!(order::VOID < order::SELECTION_IN_WATER) };
        const { assert!(order::SELECTION_IN_WATER < -0.9e6) };
        const { assert!(order::SELECTION < order::SNOW) };
    }
}
