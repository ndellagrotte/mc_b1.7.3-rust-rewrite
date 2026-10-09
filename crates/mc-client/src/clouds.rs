//! `RenderGlobal.renderClouds` / `renderCloudsFancy` (RenderGlobal.java:
//! 724-922) and `updateClouds` (RenderGlobal.java:596-598): the cloud layer
//! at `getCloudHeight()` (108), scrolling west-to-east by 0.03 blocks a tick
//! (`cloudOffsetX`), coloured by `World.drawClouds`.
//!
//! Fast graphics draw one flat textured layer (16 × 16 quads of 32 blocks
//! around the viewer); fancy graphics draw 6 × 6 cells of 8 × 8 texels,
//! scaled 12 × horizontally, as boxes 4 high: a bottom (× 0.7), a top
//! (× 1.0), x sides (× 0.9) and z sides (× 0.8), with the faces facing away
//! from the viewer left out. Fancy clouds draw twice: depth only, then
//! colour where the depth matches, so only the nearest surface shows
//! (`glColorMask` passes, RenderGlobal.java:829-841); both with the alpha
//! test (0.1, so the texture's empty texels write no depth) and fog
//! (`setupFog(0)`, EntityRenderer.java:562-565).
//!
//! The texture (`/environment/clouds.png`, repeat, nearest) is a local
//! handle; it could move to WP-3.2's texture cache.

use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::DVec3;
use bevy::prelude::*;
use mc_core::math::floor_double;
use mc_sim::world::World;

use crate::assets::JarAssets;
use crate::fog::FogState;
use crate::gl_material::{
    GlBlend, GlKey, GlMaterial, TessBatch, anaglyph, jar_texture, order, tess_color,
};
use crate::sky::ViewInfo;

/// `/environment/clouds.png`.
pub const CLOUDS_PNG: &str = "/environment/clouds.png";

/// `RenderGlobal.cloudOffsetX`: ticks of `updateClouds` since the game
/// started (the `RenderGlobal` lives as long as the game). WP-3.3's
/// `sortAndRender` reads it for its occlusion-query stagger
/// (RenderGlobal.java:465).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderGlobalClouds {
    /// `cloudOffsetX`.
    // src: RenderGlobal.java:28
    pub cloud_offset_x: i32,
}

/// `updateClouds()` in `runTick` ([`crate::game::RunTickHook::UpdateClouds`],
/// skipped while paused).
// src: RenderGlobal.java:596-598, Minecraft.java:1151-1153
pub fn update_clouds(mut c: ResMut<RenderGlobalClouds>) {
    c.cloud_offset_x = c.cloud_offset_x.wrapping_add(1);
}

/// The clouds of this frame.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct CloudFrame {
    /// Whether clouds are drawn (a world, not the nether).
    pub visible: bool,
    /// Fancy (`gameSettings.fancyGraphics`).
    pub fancy: bool,
    /// The eye.
    pub anchor: Vec3,
    /// The vertices (fancy: in the 12 × scaled space).
    pub batch: TessBatch,
}

/// What the cloud layer depends on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudInputs {
    /// `world.drawClouds(partial)`, anaglyph applied.
    pub color: [f32; 3],
    /// `renderViewEntity.prevPos + (pos - prevPos) · partial` (x and z).
    pub prev_interp: DVec3,
    /// `lastTickPosY + (posY - lastTickPosY) · partial`, as a float.
    pub eye_y: f32,
    /// `cloudOffsetX`.
    pub cloud_offset_x: i32,
    /// `renderPartialTicks`.
    pub partial: f32,
    /// `worldProvider.getCloudHeight()`.
    pub cloud_height: f32,
}

/// `renderClouds`' fast layer: 16 × 16 quads, 32 blocks wide, at
/// `cloudHeight - eyeY + 0.33`, texture coordinates 1/2048 per block offset
/// by the viewer's position plus the drift, colour alpha 0.8.
// src: RenderGlobal.java:729-773
pub fn fast_clouds(i: &CloudInputs) -> TessBatch {
    let var2 = i.eye_y;
    let var3 = 32i32;
    let var4 = 256 / var3;
    let [var7, var8, var9] = i.color;
    let var10 = 4.8828125E-4f32;
    let mut var22 = i.prev_interp.x + ((i.cloud_offset_x as f32 + i.partial) * 0.03) as f64;
    let mut var13 = i.prev_interp.z;
    let var15 = floor_double(var22 / 2048.0);
    let var16 = floor_double(var13 / 2048.0);
    var22 -= var15.wrapping_mul(2048) as f64;
    var13 -= var16.wrapping_mul(2048) as f64;
    let var17 = i.cloud_height - var2 + 0.33;
    let var18 = (var22 * var10 as f64) as f32;
    let var19 = (var13 * var10 as f64) as f32;
    let c = tess_color(var7, var8, var9, 0.8);
    let mut b = TessBatch::default();
    let mut var20 = -var3 * var4;
    while var20 < var3 * var4 {
        let mut var21 = -var3 * var4;
        while var21 < var3 * var4 {
            let (x0, x1, z0, z1) = (var20, var20 + var3, var21, var21 + var3);
            let uv = |x: i32, z: i32| [x as f32 * var10 + var18, z as f32 * var10 + var19];
            b.vertex([x0 as f32, var17, z1 as f32], uv(x0, z1), c);
            b.vertex([x1 as f32, var17, z1 as f32], uv(x1, z1), c);
            b.vertex([x1 as f32, var17, z0 as f32], uv(x1, z0), c);
            b.vertex([x0 as f32, var17, z0 as f32], uv(x0, z0), c);
            var21 += var3;
        }
        var20 += var3;
    }
    b
}

/// `renderCloudsFancy`' boxes, in the space `glScalef(12, 1, 12)` scales
/// (one unit = one texel = 12 blocks horizontally).
// src: RenderGlobal.java:785-922
pub fn fancy_clouds(i: &CloudInputs) -> TessBatch {
    let var2 = i.eye_y;
    let var4 = 12.0f32;
    let var5 = 4.0f32;
    let mut var6 =
        (i.prev_interp.x + ((i.cloud_offset_x as f32 + i.partial) * 0.03) as f64) / var4 as f64;
    let mut var8 = i.prev_interp.z / var4 as f64 + 0.33000001311302185;
    let var10 = i.cloud_height - var2 + 0.33;
    let var11 = floor_double(var6 / 2048.0);
    let var12 = floor_double(var8 / 2048.0);
    var6 -= var11.wrapping_mul(2048) as f64;
    var8 -= var12.wrapping_mul(2048) as f64;
    let [var14, var15, var16] = i.color;
    let var19 = 0.00390625f32;
    let var17 = floor_double(var6) as f32 * var19;
    let var18 = floor_double(var8) as f32 * var19;
    let var20 = (var6 - floor_double(var6) as f64) as f32;
    let var21 = (var8 - floor_double(var8) as f64) as f32;
    let var22 = 8i32;
    let var23 = 3i32;
    let var24 = 9.765625E-4f32;
    let fv22 = var22 as f32;
    let mut b = TessBatch::default();
    for var26 in (-var23 + 1)..=var23 {
        for var27 in (-var23 + 1)..=var23 {
            let var28 = (var26 * var22) as f32;
            let var29 = (var27 * var22) as f32;
            let var30 = var28 - var20;
            let var31 = var29 - var21;
            let u = |x: f32| (var28 + x) * var19 + var17;
            let v = |z: f32| (var29 + z) * var19 + var18;
            if var10 > -var5 - 1.0 {
                let c = tess_color(var14 * 0.7, var15 * 0.7, var16 * 0.7, 0.8);
                let y = var10 + 0.0;
                b.vertex([var30 + 0.0, y, var31 + fv22], [u(0.0), v(fv22)], c);
                b.vertex([var30 + fv22, y, var31 + fv22], [u(fv22), v(fv22)], c);
                b.vertex([var30 + fv22, y, var31 + 0.0], [u(fv22), v(0.0)], c);
                b.vertex([var30 + 0.0, y, var31 + 0.0], [u(0.0), v(0.0)], c);
            }
            if var10 <= var5 + 1.0 {
                let c = tess_color(var14, var15, var16, 0.8);
                let y = var10 + var5 - var24;
                b.vertex([var30 + 0.0, y, var31 + fv22], [u(0.0), v(fv22)], c);
                b.vertex([var30 + fv22, y, var31 + fv22], [u(fv22), v(fv22)], c);
                b.vertex([var30 + fv22, y, var31 + 0.0], [u(fv22), v(0.0)], c);
                b.vertex([var30 + 0.0, y, var31 + 0.0], [u(0.0), v(0.0)], c);
            }
            let c = tess_color(var14 * 0.9, var15 * 0.9, var16 * 0.9, 0.8);
            if var26 > -1 {
                for var32 in 0..var22 {
                    let s = var32 as f32;
                    let x = var30 + s + 0.0;
                    let uu = (var28 + s + 0.5) * var19 + var17;
                    b.vertex([x, var10 + 0.0, var31 + fv22], [uu, v(fv22)], c);
                    b.vertex([x, var10 + var5, var31 + fv22], [uu, v(fv22)], c);
                    b.vertex([x, var10 + var5, var31 + 0.0], [uu, v(0.0)], c);
                    b.vertex([x, var10 + 0.0, var31 + 0.0], [uu, v(0.0)], c);
                }
            }
            if var26 <= 1 {
                for var32 in 0..var22 {
                    let s = var32 as f32;
                    let x = var30 + s + 1.0 - var24;
                    let uu = (var28 + s + 0.5) * var19 + var17;
                    b.vertex([x, var10 + 0.0, var31 + fv22], [uu, v(fv22)], c);
                    b.vertex([x, var10 + var5, var31 + fv22], [uu, v(fv22)], c);
                    b.vertex([x, var10 + var5, var31 + 0.0], [uu, v(0.0)], c);
                    b.vertex([x, var10 + 0.0, var31 + 0.0], [uu, v(0.0)], c);
                }
            }
            let c = tess_color(var14 * 0.8, var15 * 0.8, var16 * 0.8, 0.8);
            if var27 > -1 {
                for var32 in 0..var22 {
                    let s = var32 as f32;
                    let z = var31 + s + 0.0;
                    let vv = (var29 + s + 0.5) * var19 + var18;
                    b.vertex([var30 + 0.0, var10 + var5, z], [u(0.0), vv], c);
                    b.vertex([var30 + fv22, var10 + var5, z], [u(fv22), vv], c);
                    b.vertex([var30 + fv22, var10 + 0.0, z], [u(fv22), vv], c);
                    b.vertex([var30 + 0.0, var10 + 0.0, z], [u(0.0), vv], c);
                }
            }
            if var27 <= 1 {
                for var32 in 0..var22 {
                    let s = var32 as f32;
                    let z = var31 + s + 1.0 - var24;
                    let vv = (var29 + s + 0.5) * var19 + var18;
                    b.vertex([var30 + 0.0, var10 + var5, z], [u(0.0), vv], c);
                    b.vertex([var30 + fv22, var10 + var5, z], [u(fv22), vv], c);
                    b.vertex([var30 + fv22, var10 + 0.0, z], [u(fv22), vv], c);
                    b.vertex([var30 + 0.0, var10 + 0.0, z], [u(0.0), vv], c);
                }
            }
        }
    }
    b
}

/// `renderClouds(partial)`'s inputs and vertices (not in the nether).
// src: RenderGlobal.java:724-779
pub fn compute(
    world: &World,
    view: &ViewInfo,
    fancy: bool,
    anaglyph_on: bool,
    cloud_offset_x: i32,
    partial: f32,
) -> CloudFrame {
    if world.world_provider.is_nether {
        return CloudFrame::default();
    }
    let v = world.draw_clouds(partial);
    let mut color = [v.x_coord as f32, v.y_coord as f32, v.z_coord as f32];
    if anaglyph_on {
        color = anaglyph(color);
    }
    let i = CloudInputs {
        color,
        prev_interp: view.prev_interp(partial),
        eye_y: view.last_tick_interp(partial).y as f32,
        cloud_offset_x,
        partial,
        cloud_height: world.world_provider.get_cloud_height(),
    };
    CloudFrame {
        visible: true,
        fancy,
        anchor: Vec3::ZERO,
        batch: if fancy {
            fancy_clouds(&i)
        } else {
            fast_clouds(&i)
        },
    }
}

/// Which cloud entity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudPart {
    /// The fast layer.
    Fast,
    /// Fancy, pass 0 (depth only).
    FancyDepth,
    /// Fancy, pass 1 (colour).
    FancyColor,
}

/// The clouds' mesh and materials.
#[derive(Resource, Clone, Debug)]
pub struct CloudHandles {
    /// The mesh (rebuilt every frame).
    pub mesh: Handle<Mesh>,
    /// Per part, its material.
    pub materials: Vec<(CloudPart, Handle<GlMaterial>)>,
}

/// Spawns the cloud entities (hidden until a frame shows them).
pub fn spawn_clouds(
    mut commands: Commands,
    jar: Res<JarAssets>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
) {
    let tex = match jar_texture(&jar, CLOUDS_PNG, false) {
        Ok(i) => Some(images.add(i)),
        Err(e) => {
            error!("{e}");
            None
        }
    };
    let mesh = meshes.add(TessBatch::placeholder_mesh());
    // glEnable(GL_BLEND), (770, 771); glDepthMask(true) since
    // EntityRenderer.java:546; GL_ALPHA_TEST on at 0.1.
    let make = |color_write: bool, bias: f32| {
        let m = GlMaterial::new(
            GlKey {
                blend: GlBlend::Alpha,
                depth_write: true,
                color_write,
                far: false,
                wide_lines: false,
            },
            bias,
        )
        .with_alpha_test(0.1);
        match &tex {
            Some(t) => m.with_texture(t.clone()),
            None => m,
        }
    };
    let parts = [
        (CloudPart::Fast, make(true, order::CLOUDS)),
        (CloudPart::FancyDepth, make(false, order::CLOUDS_DEPTH)),
        (CloudPart::FancyColor, make(true, order::CLOUDS)),
    ];
    let mut handles = Vec::new();
    for (part, mat) in parts {
        let mh = materials.add(mat);
        handles.push((part, mh.clone()));
        commands.spawn((
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mh),
            Transform::IDENTITY,
            Visibility::Hidden,
            NoFrustumCulling,
            part,
        ));
    }
    commands.insert_resource(CloudHandles {
        mesh,
        materials: handles,
    });
}

/// Applies [`CloudFrame`].
pub fn apply_clouds(
    frame: Res<CloudFrame>,
    fog: Res<FogState>,
    handles: Option<Res<CloudHandles>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlMaterial>>,
    mut parts: Query<(&CloudPart, &mut Transform, &mut Visibility)>,
) {
    let _perf = crate::perf::time(crate::perf::Phase::Sky);
    let Some(h) = handles else {
        return;
    };
    let show = frame.visible && !frame.batch.is_empty();
    for (part, mut t, mut vis) in &mut parts {
        let shown = show
            && match part {
                CloudPart::Fast => !frame.fancy,
                _ => frame.fancy,
            };
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *vis != want {
            *vis = want;
        }
        t.translation = frame.anchor;
        // glScalef(12, 1, 12) (RenderGlobal.java:827).
        t.scale = if frame.fancy {
            Vec3::new(12.0, 1.0, 12.0)
        } else {
            Vec3::ONE
        };
    }
    if !show {
        return;
    }
    if let Some(mut m) = meshes.get_mut(&h.mesh) {
        frame.batch.write_into(&mut m, frame.batch.quad_indices());
    }
    for (_, mh) in &h.materials {
        if let Some(mut m) = materials.get_mut(mh) {
            m.params.fog = fog.world.to_vec4();
            m.params.fog_color = Vec4::new(fog.color[0], fog.color[1], fog.color[2], 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs() -> CloudInputs {
        CloudInputs {
            color: [1.0, 1.0, 1.0],
            prev_interp: DVec3::new(10.0, 70.0, -20.0),
            eye_y: 71.62,
            cloud_offset_x: 100,
            partial: 0.5,
            cloud_height: 108.0,
        }
    }

    #[test]
    fn fast_layer() {
        let b = fast_clouds(&inputs());
        assert_eq!(b.len(), 16 * 16 * 4);
        // y = 108 - 71.62 + 0.33 (floats)
        let y = 108.0f32 - 71.62 + 0.33;
        assert_eq!(b.positions[0], [-256.0, y, -224.0]);
        // u = x / 2048 + (10 + (100 + 0.5) · 0.03) / 2048
        let off = (10.0 + ((100.0f32 + 0.5) * 0.03) as f64) * 4.8828125E-4f32 as f64;
        assert_eq!(b.uvs[0][0], -256.0 * 4.8828125E-4 + off as f32);
        assert_eq!(b.colors[0], tess_color(1.0, 1.0, 1.0, 0.8));
        // The drift: one tick moves the texture by 0.03 / 2048.
        let mut i = inputs();
        i.cloud_offset_x += 1;
        let d = fast_clouds(&i).uvs[0][0] - b.uvs[0][0];
        assert!((d - 0.03 / 2048.0).abs() < 1e-7, "{d}");
    }

    #[test]
    fn fancy_cells_and_faces() {
        // Viewer below the clouds (var10 = 36.71 > 5): bottoms, no tops.
        let b = fancy_clouds(&inputs());
        // 36 cells × (bottom 4 + x sides (16 or 8 quads) + z sides).
        // Per axis: cells -2..=3; "> -1" for 0..=3 (4), "<= 1" for -2..=1 (4).
        let quads_per_axis_row = 4 * 8 + 4 * 8; // per 6 cells of a row
        let sides = 6 * quads_per_axis_row * 2;
        assert_eq!(b.len() / 4, 36 + sides);
        // Viewer inside the layer (var10 = 0): both bottom and top.
        let mut i = inputs();
        i.eye_y = 108.33;
        assert_eq!(fancy_clouds(&i).len() / 4, 72 + sides);
        // Viewer above (var10 < -5): tops only.
        i.eye_y = 200.0;
        assert_eq!(fancy_clouds(&i).len() / 4, 36 + sides);
    }
}
