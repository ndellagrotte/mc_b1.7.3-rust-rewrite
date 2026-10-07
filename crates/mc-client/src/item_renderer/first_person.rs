//! `ItemRenderer.renderItemInFirstPerson(partial)` (ItemRenderer.java:
//! 129-275) as data: the matrices of what it draws, in order, from the
//! modelview `renderHand` hands it (identity, `hurtCameraEffect`,
//! `setupViewBobbing`; EntityRenderer.java:301-316), and the lighting it
//! turns on.
//!
//! Three branches: a map (both arms holding it, the map background and the
//! map itself, [`super::map`]), any other item (the item, tilted by the
//! swing, [`super::render_item`]), and the empty hand (the player model's
//! right arm, `RenderPlayer.drawFirstPersonHand`). `equippedProgress`
//! lowers the hand while an item changes ([`super::ItemRenderer`]).
//!
//! The light: `RenderHelper.enableStandardItemLighting()` is called with
//! the modelview rotated by the player's interpolated pitch and yaw
//! (ItemRenderer.java:133-137), so the two lights are fixed in the world as
//! seen without the `+ 180°` of `orientCamera`: [`FirstPersonFrame::light`]
//! maps them to eye space.

use bevy::math::{Mat4, Vec3};
use mc_core::math::{cos, sin, sqrt_float};

use crate::camera::GlMatrix;
use crate::model::material::standard_light_directions;

/// What `renderItemInFirstPerson` reads of the held stack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldStack {
    /// `itemID`.
    pub item_id: i32,
    /// `getItemDamage()`.
    pub damage: i32,
    /// `Item.itemsList[itemID].getColorFromDamage(damage)`.
    pub color: i32,
    /// `getItem().shouldRotateAroundWhenRendering()`.
    pub rotate_around: bool,
    /// `itemID == Item.mapItem.shiftedIndex`.
    pub is_map: bool,
}

/// `renderItemInFirstPerson`'s inputs (the player's state at `partial`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirstPersonInput {
    /// `prevEquippedProgress + (equippedProgress - prevEquippedProgress) *
    /// partial`.
    pub equipped: f32,
    /// The interpolated `rotationPitch`.
    pub pitch: f32,
    /// The interpolated `rotationYaw`.
    pub yaw: f32,
    /// `thePlayer.getSwingProgress(partial)`.
    pub swing: f32,
    /// `theWorld.getLightBrightness(floor(posX), floor(posY), floor(posZ))`.
    pub light: f32,
    /// `itemToRender`.
    pub stack: Option<HeldStack>,
}

/// One draw of `renderItemInFirstPerson`, with the modelview at the call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FirstPersonDraw {
    /// `RenderPlayer.drawFirstPersonHand()` at this matrix (the right arm
    /// part's own transform comes after it).
    Arm(Mat4),
    /// `renderItem(player, stack)` at this matrix.
    Item(Mat4),
    /// The map: the background quad and `MapItemRenderer.renderMap` at
    /// this matrix (map pixels; ItemRenderer.java:199-217).
    Map(Mat4),
}

/// The result of [`render_item_in_first_person`].
#[derive(Clone, Debug, PartialEq)]
pub struct FirstPersonFrame {
    /// The `glColor4f` set before the draws (ItemRenderer.java:143-151).
    pub color: [f32; 4],
    /// The modelview `enableStandardItemLighting` saw: the light
    /// directions in eye space are `light · L` ([`Self::light_dirs`]).
    pub light: Mat4,
    /// The draws, in order.
    pub draws: Vec<FirstPersonDraw>,
}

impl FirstPersonFrame {
    /// `GL_LIGHT0`/`GL_LIGHT1`'s directions in eye space (`glLight`'s
    /// `GL_POSITION` with w 0 is transformed by the modelview's upper 3×3).
    pub fn light_dirs(&self) -> [Vec3; 2] {
        standard_light_directions().map(|l| self.light.transform_vector3(l).normalize_or_zero())
    }
}

/// `MathHelper.sin(var8 * PI)`, `sin(sqrt(var8) * PI)`,
/// `sin(sqrt(var8) * PI * 2)`: the first translation's swing arcs.
// src: ItemRenderer.java:222-225
pub fn swing_arcs(var8: f32) -> (f32, f32, f32) {
    let var9 = sin(var8 * 3.1415927);
    let var10 = sin(sqrt_float(var8) * 3.1415927);
    let up = sin(sqrt_float(var8) * 3.1415927 * 2.0);
    (var9, var10, up)
}

/// `MathHelper.sin(var8 * var8 * PI)`, `sin(sqrt(var8) * PI)`: the
/// rotations' swing arcs.
// src: ItemRenderer.java:229-231
pub fn swing_rotation_arcs(var8: f32) -> (f32, f32) {
    let var9 = sin(var8 * var8 * 3.1415927);
    let var10 = sin(sqrt_float(var8) * 3.1415927);
    (var9, var10)
}

/// The map branch's pitch tilt: `1 - pitch / 45 + 0.1` clamped to 0..1,
/// then `-cos(t · PI) · 0.5 + 0.5`.
// src: ItemRenderer.java:161-170
#[allow(clippy::manual_clamp)] // the Java's two ifs (NaN passes through)
pub fn map_tilt(var4: f32) -> f32 {
    let mut var8 = 1.0 - var4 / 45.0 + 0.1;
    if var8 < 0.0 {
        var8 = 0.0;
    }
    if var8 > 1.0 {
        var8 = 1.0;
    }
    -cos(var8 * 3.1415927) * 0.5 + 0.5
}

/// `renderItemInFirstPerson(partial)` from the modelview `base`
/// (module docs).
// src: ItemRenderer.java:129-275
pub fn render_item_in_first_person(base: Mat4, i: &FirstPersonInput) -> FirstPersonFrame {
    let var2 = i.equipped;
    let var4 = i.pitch;
    let mut light = GlMatrix(base);
    light.rotate(var4, 1.0, 0.0, 0.0);
    light.rotate(i.yaw, 0.0, 1.0, 0.0);
    let var6 = i.light;
    let color = match i.stack {
        Some(s) => {
            let var7 = s.color;
            let var8 = (var7 >> 16 & 255) as f32 / 255.0;
            let var9 = (var7 >> 8 & 255) as f32 / 255.0;
            let var10 = (var7 & 255) as f32 / 255.0;
            [var6 * var8, var6 * var9, var6 * var10, 1.0]
        }
        None => [var6, var6, var6, 1.0],
    };
    let mut draws = Vec::new();
    let mut m = GlMatrix(base);
    match i.stack {
        Some(s) if s.is_map => {
            // src: ItemRenderer.java:155-218
            let var14 = 0.8f32;
            let var8 = i.swing;
            let (var9, var10, up) = swing_arcs(var8);
            m.translate(-var10 * 0.4, up * 0.2, -var9 * 0.2);
            let var8 = map_tilt(var4);
            m.translate(
                0.0,
                0.0 * var14 - (1.0 - var2) * 1.2 - var8 * 0.5 + 0.04,
                -0.9 * var14,
            );
            m.rotate(90.0, 0.0, 1.0, 0.0);
            m.rotate(var8 * -85.0, 0.0, 0.0, 1.0);
            for var17 in 0..2 {
                let var21 = var17 * 2 - 1;
                let mut a = m;
                a.translate(-0.0, -0.6, 1.1 * var21 as f32);
                a.rotate((-45 * var21) as f32, 1.0, 0.0, 0.0);
                a.rotate(-90.0, 0.0, 0.0, 1.0);
                a.rotate(59.0, 0.0, 0.0, 1.0);
                a.rotate((-65 * var21) as f32, 0.0, 1.0, 0.0);
                let var13 = 1.0f32;
                a.scale(var13, var13, var13);
                draws.push(FirstPersonDraw::Arm(a.0));
            }
            let var9 = i.swing;
            let var10 = sin(var9 * var9 * 3.1415927);
            let var18 = sin(sqrt_float(var9) * 3.1415927);
            m.rotate(-var10 * 20.0, 0.0, 1.0, 0.0);
            m.rotate(-var18 * 20.0, 0.0, 0.0, 1.0);
            m.rotate(-var18 * 80.0, 1.0, 0.0, 0.0);
            let var9 = 0.38f32;
            m.scale(var9, var9, var9);
            m.rotate(90.0, 0.0, 1.0, 0.0);
            m.rotate(180.0, 0.0, 0.0, 1.0);
            m.translate(-1.0, -1.0, 0.0);
            let var10 = 0.015625f32;
            m.scale(var10, var10, var10);
            draws.push(FirstPersonDraw::Map(m.0));
        }
        Some(s) => {
            // src: ItemRenderer.java:219-242
            let var14 = 0.8f32;
            let var8 = i.swing;
            let (var9, var10, up) = swing_arcs(var8);
            m.translate(-var10 * 0.4, up * 0.2, -var9 * 0.2);
            m.translate(
                0.7 * var14,
                -0.65 * var14 - (1.0 - var2) * 0.6,
                -0.9 * var14,
            );
            m.rotate(45.0, 0.0, 1.0, 0.0);
            let (var9, var10) = swing_rotation_arcs(i.swing);
            m.rotate(-var9 * 20.0, 0.0, 1.0, 0.0);
            m.rotate(-var10 * 20.0, 0.0, 0.0, 1.0);
            m.rotate(-var10 * 80.0, 1.0, 0.0, 0.0);
            let var8 = 0.4f32;
            m.scale(var8, var8, var8);
            if s.rotate_around {
                m.rotate(180.0, 0.0, 1.0, 0.0);
            }
            draws.push(FirstPersonDraw::Item(m.0));
        }
        None => {
            // src: ItemRenderer.java:243-271
            let var14 = 0.8f32;
            let var8 = i.swing;
            let var9 = sin(var8 * 3.1415927);
            let var10 = sin(sqrt_float(var8) * 3.1415927);
            m.translate(
                -var10 * 0.3,
                sin(sqrt_float(var8) * 3.1415927 * 2.0) * 0.4,
                -var9 * 0.4,
            );
            m.translate(
                0.8 * var14,
                -0.75 * var14 - (1.0 - var2) * 0.6,
                -0.9 * var14,
            );
            m.rotate(45.0, 0.0, 1.0, 0.0);
            let (var9, var10) = swing_rotation_arcs(i.swing);
            m.rotate(var10 * 70.0, 0.0, 1.0, 0.0);
            m.rotate(-var9 * 20.0, 0.0, 0.0, 1.0);
            m.translate(-1.0, 3.6, 3.5);
            m.rotate(120.0, 0.0, 0.0, 1.0);
            m.rotate(200.0, 1.0, 0.0, 0.0);
            m.rotate(-135.0, 0.0, 1.0, 0.0);
            m.scale(1.0, 1.0, 1.0);
            m.translate(5.6, 0.0, 0.0);
            let var10 = 1.0f32;
            m.scale(var10, var10, var10);
            draws.push(FirstPersonDraw::Arm(m.0));
        }
    }
    FirstPersonFrame {
        color,
        light: light.0,
        draws,
    }
}
