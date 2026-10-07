//! `TextureFX` (TextureFX.java): the procedural animated tiles, and the
//! registration list of `Minecraft.startGame`.
//!
//! Each FX keeps a 16×16 RGBA tile ([`TextureFx::image_data`], Java's
//! `byte[1024]`) that its `onTick` rewrites; `RenderEngine.updateDynamicTextures`
//! then copies it into `terrain.png` or `gui/items.png` at the FX's icon
//! ([`crate::textures::Textures::update_dynamic_textures`]).
//!
//! The base class's fields are [`TextureFx`]; the subclasses' own state is
//! the closed enum [`FxKind`] (PLAN_v4 D1 rule 5), one module per class.
//!
//! # Randomness
//!
//! The FX that draw random numbers call `Math.random()` (the lava, water and
//! flame FX, and the compass and watch in the Nether), the JVM-wide stream
//! that `java.lang.Math` creates with `new Random()` on first use. The client
//! keeps its own [`JavaRandom`] for it, seeded from the clock
//! ([`crate::clock::new_random_seed`]), owned by
//! [`crate::textures::Textures`] and drawn in the FX's update order. It is
//! never the simulation's `World::math_random` (the FX never touch the
//! simulation, PLAN_v4 §3.4). `TexturePortalFX` seeds its own
//! `new Random(100L)` and is fully deterministic.

use mc_core::java_random::JavaRandom;

use super::compass_fx::TextureCompassFx;
use super::flames_fx::TextureFlamesFx;
use super::lava_flow_fx::TextureLavaFlowFx;
use super::lava_fx::TextureLavaFx;
use super::portal_fx::TexturePortalFx;
use super::watch_fx::TextureWatchFx;
use super::water_flow_fx::TextureWaterFlowFx;
use super::water_fx::TextureWaterFx;

/// `Block.fire.blockIndexInTexture` (`new BlockFire(51, 31)`).
// src: Block.java:643
pub const FIRE_INDEX: i32 = 31;
/// `Block.waterMoving.blockIndexInTexture`: `BlockFluid`'s
/// `(water ? 12 : 14) * 16 + 13` = 205.
// src: BlockFluid.java:7, Block.java:600
pub const WATER_INDEX: i32 = 12 * 16 + 13;
/// `Block.lavaMoving.blockIndexInTexture` = 14 * 16 + 13 = 237.
// src: BlockFluid.java:7, Block.java:602
pub const LAVA_INDEX: i32 = 14 * 16 + 13;
/// `Block.portal.blockIndexInTexture` (`new BlockPortal(90, 14)`).
// src: Block.java:682
pub const PORTAL_INDEX: i32 = 14;
/// `Item.compass.getIconFromDamage(0)`: `setIconCoord(6, 3)` = 6 + 3 * 16.
// src: Item.java:363, :142-145
pub const COMPASS_INDEX: i32 = 6 + 3 * 16;
/// `Item.pocketSundial.getIconFromDamage(0)`: `setIconCoord(6, 4)` =
/// 6 + 4 * 16.
// src: Item.java:365, :142-145
pub const WATCH_INDEX: i32 = 6 + 4 * 16;

/// `tileImage` 0: `/terrain.png`.
// src: TextureFX.java:21-22
pub const TILE_IMAGE_TERRAIN: i32 = 0;
/// `tileImage` 1: `/gui/items.png`.
// src: TextureFX.java:23-24
pub const TILE_IMAGE_ITEMS: i32 = 1;

/// What the compass and the watch read from `Minecraft` each tick
/// (`mc.theWorld`, `mc.thePlayer`): `None` when either is null.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FxWorldView {
    /// `theWorld.getSpawnPoint().x`.
    pub spawn_x: i32,
    /// `theWorld.getSpawnPoint().z`.
    pub spawn_z: i32,
    /// `thePlayer.posX`.
    pub player_x: f64,
    /// `thePlayer.posZ`.
    pub player_z: f64,
    /// `thePlayer.rotationYaw`.
    pub player_yaw: f32,
    /// `theWorld.worldProvider.isNether`.
    pub is_nether: bool,
    /// `theWorld.getCelestialAngle(1.0F)`.
    pub celestial_angle: f32,
}

/// The subclass of an FX and its own fields.
#[derive(Clone, Debug)]
pub enum FxKind {
    /// `TextureFlamesFX`.
    Flames(Box<TextureFlamesFx>),
    /// `TextureLavaFX`.
    Lava(Box<TextureLavaFx>),
    /// `TextureLavaFlowFX`.
    LavaFlow(Box<TextureLavaFlowFx>),
    /// `TextureWaterFX`.
    Water(Box<TextureWaterFx>),
    /// `TextureWaterFlowFX`.
    WaterFlow(Box<TextureWaterFlowFx>),
    /// `TexturePortalFX`.
    Portal(Box<TexturePortalFx>),
    /// `TextureCompassFX`.
    Compass(Box<TextureCompassFx>),
    /// `TextureWatchFX`.
    Watch(Box<TextureWatchFx>),
}

/// `TextureFX`: the base fields and the subclass.
#[derive(Clone, Debug)]
pub struct TextureFx {
    /// `imageData`: the 16×16 tile, RGBA bytes.
    // src: TextureFX.java:6
    pub image_data: [u8; 1024],
    /// `iconIndex`: the tile in the 16×16-tile atlas.
    // src: TextureFX.java:7
    pub icon_index: i32,
    /// `anaglyphEnabled`, copied from the options before every tick.
    // src: TextureFX.java:8
    pub anaglyph_enabled: bool,
    /// `textureId`: never set by any FX in this version (only declared,
    /// TextureFX.java:9), so `updateDynamicTextures`' second loop never runs.
    // src: TextureFX.java:9
    pub texture_id: i32,
    /// `tileSize`: the tile is written `tileSize × tileSize` times.
    // src: TextureFX.java:10
    pub tile_size: i32,
    /// `tileImage`: [`TILE_IMAGE_TERRAIN`] or [`TILE_IMAGE_ITEMS`].
    // src: TextureFX.java:11
    pub tile_image: i32,
    /// The subclass.
    pub kind: FxKind,
}

impl TextureFx {
    /// `new TextureFX(iconIndex)` with its subclass.
    // src: TextureFX.java:13-15
    pub fn new(icon_index: i32, kind: FxKind) -> Self {
        TextureFx {
            image_data: [0; 1024],
            icon_index,
            anaglyph_enabled: false,
            texture_id: 0,
            tile_size: 1,
            tile_image: TILE_IMAGE_TERRAIN,
            kind,
        }
    }

    /// `onTick()`: the subclass's update of [`TextureFx::image_data`].
    /// `math_random` is the client's `Math.random()` stream (module docs);
    /// `view` is what the compass and watch read.
    pub fn on_tick(&mut self, math_random: &mut JavaRandom, view: Option<&FxWorldView>) {
        let anaglyph = self.anaglyph_enabled;
        let data = &mut self.image_data;
        match &mut self.kind {
            FxKind::Flames(f) => f.on_tick(data, anaglyph, math_random),
            FxKind::Lava(f) => f.on_tick(data, anaglyph, math_random),
            FxKind::LavaFlow(f) => f.on_tick(data, anaglyph, math_random),
            FxKind::Water(f) => f.on_tick(data, anaglyph, math_random),
            FxKind::WaterFlow(f) => f.on_tick(data, anaglyph, math_random),
            FxKind::Portal(f) => f.on_tick(data, anaglyph),
            FxKind::Compass(f) => f.on_tick(data, anaglyph, math_random, view),
            FxKind::Watch(f) => f.on_tick(data, anaglyph, math_random, view),
        }
    }

    /// `bindImage`'s texture name: `/terrain.png` for `tileImage` 0,
    /// `/gui/items.png` for 1, none otherwise. For another value Java binds
    /// nothing, and the upload goes to the texture the previous FX bound
    /// (terrain.png for the first FX, Minecraft.java:967). No FX has
    /// another value, so that branch is unreachable; the caller's
    /// `continue` (write nothing) differs from vanilla there.
    // src: TextureFX.java:20-27
    pub fn bind_image_path(&self) -> Option<&'static str> {
        match self.tile_image {
            TILE_IMAGE_TERRAIN => Some(super::TERRAIN_PNG),
            TILE_IMAGE_ITEMS => Some(super::ITEMS_PNG),
            _ => None,
        }
    }

    /// The FX's class name (for logs and dumps).
    pub fn class_name(&self) -> &'static str {
        match self.kind {
            FxKind::Flames(_) => "TextureFlamesFX",
            FxKind::Lava(_) => "TextureLavaFX",
            FxKind::LavaFlow(_) => "TextureLavaFlowFX",
            FxKind::Water(_) => "TextureWaterFX",
            FxKind::WaterFlow(_) => "TextureWaterFlowFX",
            FxKind::Portal(_) => "TexturePortalFX",
            FxKind::Compass(_) => "TextureCompassFX",
            FxKind::Watch(_) => "TextureWatchFX",
        }
    }
}

/// The top-left pixel of the `(var3, var4)`-th copy of an FX tile:
/// `glTexSubImage2D(…, iconIndex % 16 * 16 + var3 * 16, iconIndex / 16 * 16
/// + var4 * 16, 16, 16, …)`.
// src: RenderEngine.java:377-379
pub fn tile_origin(icon_index: i32, var3: i32, var4: i32) -> (i32, i32) {
    (
        icon_index % 16 * 16 + var3 * 16,
        icon_index / 16 * 16 + var4 * 16,
    )
}

/// The anaglyph colour transform every FX and `setupTexture` apply:
/// `(r*30 + g*59 + b*11) / 100`, `(r*30 + g*70) / 100`, `(r*30 + b*70) / 100`.
// src: RenderEngine.java:204-211, TextureFlamesFX.java:61-68
#[inline]
pub fn anaglyph(r: i32, g: i32, b: i32) -> (i32, i32, i32) {
    let var9 = (r * 30 + g * 59 + b * 11) / 100;
    let var10 = (r * 30 + g * 70) / 100;
    let var11 = (r * 30 + b * 70) / 100;
    (var9, var10, var11)
}

/// Builds the FX list in `startGame`'s registration order (which is also
/// the update order): `textureLavaFX`, `textureWaterFX`, `TexturePortalFX`,
/// `TextureCompassFX`, `TextureWatchFX`, `TextureWaterFlowFX`,
/// `TextureLavaFlowFX`, `TextureFlamesFX(0)`, `TextureFlamesFX(1)`.
/// `items_argb` is `/gui/items.png` and `dial_argb` `/misc/dial.png` as
/// `getRGB` ARGB (`None` when the read failed: Java prints the
/// `IOException` and keeps the all-zero array).
// src: Minecraft.java:275-283, :154-155
pub fn startup_fx_list(
    items_argb: Option<(u32, u32, &[i32])>,
    dial_argb: Option<(u32, u32, &[i32])>,
) -> Vec<TextureFx> {
    vec![
        TextureLavaFx::new_fx(),
        TextureWaterFx::new_fx(),
        TexturePortalFx::new_fx(),
        TextureCompassFx::new_fx(items_argb),
        TextureWatchFx::new_fx(items_argb, dial_argb),
        TextureWaterFlowFx::new_fx(),
        TextureLavaFlowFx::new_fx(),
        TextureFlamesFx::new_fx(0),
        TextureFlamesFx::new_fx(1),
    ]
}

/// `var2.getRGB(x, y, 16, 16, out, 0, 16)`: a 16×16 block of an ARGB image
/// (zeros where it is outside the image; Java would throw, which the FX
/// constructors do not catch, but the jar's images are large enough).
pub fn get_rgb_16(img: Option<(u32, u32, &[i32])>, x: i32, y: i32) -> [i32; 256] {
    let mut out = [0i32; 256];
    let Some((w, h, px)) = img else {
        return out;
    };
    for dy in 0..16 {
        for dx in 0..16 {
            let sx = x + dx;
            let sy = y + dy;
            if sx >= 0 && sy >= 0 && (sx as u32) < w && (sy as u32) < h {
                out[(dy * 16 + dx) as usize] = px[(sy as u32 * w + sx as u32) as usize];
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_placement_arithmetic() {
        // Fire (31) is column 15, row 1.
        assert_eq!(tile_origin(31, 0, 0), (240, 16));
        // Fire's second frame (47): column 15, row 2.
        assert_eq!(tile_origin(47, 0, 0), (240, 32));
        // Water (205): column 13, row 12.
        assert_eq!(tile_origin(205, 0, 0), (208, 192));
        // The flowing water (206, tileSize 2) covers 206, 207, 222, 223.
        assert_eq!(tile_origin(206, 0, 0), (224, 192));
        assert_eq!(tile_origin(206, 1, 0), (240, 192));
        assert_eq!(tile_origin(206, 0, 1), (224, 208));
        assert_eq!(tile_origin(206, 1, 1), (240, 208));
        // Lava (237), portal (14), compass (54), watch (70).
        assert_eq!(tile_origin(237, 0, 0), (208, 224));
        assert_eq!(tile_origin(14, 0, 0), (224, 0));
        assert_eq!(tile_origin(54, 0, 0), (96, 48));
        assert_eq!(tile_origin(70, 0, 0), (96, 64));
    }

    /// The icon indices agree with the simulation's block and item tables.
    #[test]
    fn indices_match_the_registries() {
        use mc_sim::block::block;
        use mc_sim::item::item;
        assert_eq!(FIRE_INDEX, 31);
        assert_eq!(WATER_INDEX, 205);
        assert_eq!(LAVA_INDEX, 237);
        assert_eq!(PORTAL_INDEX, 14);
        assert_eq!(COMPASS_INDEX, 54);
        assert_eq!(WATCH_INDEX, 70);
        assert_eq!(block(51).unwrap().block_index_in_texture, FIRE_INDEX);
        assert_eq!(block(8).unwrap().block_index_in_texture, WATER_INDEX);
        assert_eq!(block(10).unwrap().block_index_in_texture, LAVA_INDEX);
        assert_eq!(block(90).unwrap().block_index_in_texture, PORTAL_INDEX);
        assert_eq!(item(345).unwrap().get_icon_from_damage(0), COMPASS_INDEX);
        assert_eq!(item(347).unwrap().get_icon_from_damage(0), WATCH_INDEX);
    }

    #[test]
    fn registration_order() {
        let list = startup_fx_list(None, None);
        let names: Vec<_> = list.iter().map(|f| f.class_name()).collect();
        assert_eq!(
            names,
            [
                "TextureLavaFX",
                "TextureWaterFX",
                "TexturePortalFX",
                "TextureCompassFX",
                "TextureWatchFX",
                "TextureWaterFlowFX",
                "TextureLavaFlowFX",
                "TextureFlamesFX",
                "TextureFlamesFX",
            ]
        );
        let icons: Vec<_> = list.iter().map(|f| f.icon_index).collect();
        assert_eq!(icons, [237, 205, 14, 54, 70, 206, 238, 31, 47]);
        let sizes: Vec<_> = list.iter().map(|f| f.tile_size).collect();
        assert_eq!(sizes, [1, 1, 1, 1, 1, 2, 2, 1, 1]);
        let images: Vec<_> = list.iter().map(|f| f.tile_image).collect();
        assert_eq!(images, [0, 0, 0, 1, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn anaglyph_formula() {
        assert_eq!(anaglyph(100, 100, 100), (100, 100, 100));
        // (255*30 + 20*59 + 20*11)/100 = 90, (255*30 + 20*70)/100 = 90,
        // (255*30 + 20*70)/100 = 90.
        assert_eq!(anaglyph(255, 20, 20), (90, 90, 90));
        assert_eq!(anaglyph(0, 255, 0), (150, 178, 0));
    }
}
