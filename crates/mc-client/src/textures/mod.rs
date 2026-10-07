//! `RenderEngine` (RenderEngine.java): the texture cache, the `TextureFX`
//! updates written into the terrain and items atlases every tick, and the
//! colour maps ([`colorizer`]).
//!
//! # Vanilla → here
//!
//! | Java | Here |
//! |---|---|
//! | `getTexture(name)` → GL texture name, cached in `textureMap` | [`Textures::get_texture`] → `Handle<Image>`, cached by name; [`TextureLoader::get`] in systems |
//! | `func_28149_a(name)` → `int[]` ARGB, cached in `field_28151_c` | [`Textures::get_texture_contents`] |
//! | the `##` (`unwrapImageByColumns`), `%clamp%`, `%blur%` name prefixes | the same, in both lookups |
//! | `setupTexture`: `GL_NEAREST` min and mag (`useMipmaps` is always false), `GL_REPEAT`, or `GL_CLAMP` for `clampTexture`, `GL_LINEAR` for `blurTexture`; the anaglyph colour transform on upload | [`setup_texture`]: an RGBA8 sRGB Bevy [`Image`] with one mip level and that sampler |
//! | `missingTextureImage`: 64×64 white, "missingtex" in black at (1, 10) | [`missing_texture_image`] (the glyphs are a project-owned stand-in for AWT's default font, which is platform-dependent anyway) |
//! | `allocateAndSetupTexture`, `deleteTexture`, `func_28150_a` (upload an `int[]` into a texture), `bindTexture` | [`Textures::allocate_and_setup_texture`], [`Textures::delete_texture`], [`Textures::upload_argb`]; binding is the renderers' business |
//! | `registerTextureFX`, `updateDynamicTextures` | [`Textures::register_texture_fx`], [`Textures::update_dynamic_textures`] (run in [`RunTickHook::UpdateDynamicTextures`]) |
//! | `refreshTextures` (the anaglyph option, Minecraft.java via GameSettings.java:139-142) | [`Textures::refresh_textures`], on [`SettingsEffect::RefreshTextures`] |
//! | `getTextureForDownloadableImage`, `obtainImageData`, `releaseImageData`, `urlToImageDataMap` | deferred (D2: skins and cloaks are defunct online services) |
//! | `texturePack.selectedTexturePack` (`TexturePackList`, `TexturePackCustom`, `GuiTexturePacks`) | deferred (D3: texture packs); always the default pack, i.e. the jar ([`JarAssets`], TexturePackBase.java:28-30) |
//! | the mipmap code (`useMipmaps`, `averageColor`, `weightedAverageColor`) | not ported: `useMipmaps` is a static `false` that nothing sets (RenderEngine.java:20) |
//!
//! Each cached texture keeps a CPU copy of what was uploaded
//! ([`TextureEntry::rgba`]); for the atlases it includes the FX tiles, so the
//! terrain mesher (WP-3.3) can sample it and `dump_texture` can write it out.
//!
//! # Errors
//!
//! `ImageIO.read` failing with an `IOException` is caught by vanilla and
//! gives the missing texture. A resource that is absent under a prefix
//! (`%clamp%`, `%blur%`, `##`) or during `refreshTextures` makes vanilla
//! throw an uncaught `IllegalArgumentException` (`ImageIO.read(null)`); the
//! port logs an error and uses the missing texture instead.

pub mod colorizer;
pub mod compass_fx;
pub mod flames_fx;
pub mod lava_flow_fx;
pub mod lava_fx;
pub mod portal_fx;
pub mod texture_fx;
pub mod watch_fx;
pub mod water_flow_fx;
pub mod water_fx;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use bevy::asset::{Assets, Handle, RenderAssetUsages};
use bevy::ecs::system::SystemParam;
use bevy::image::{Image, ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use mc_core::java_random::JavaRandom;

use crate::GameFrameSet;
use crate::assets::{self, JarAssets};
use crate::clock;
use crate::events::ClientEvent;
use crate::game::{Game, RunTickHook};
use crate::settings::SettingsEffect;

pub use colorizer::Colorizers;
pub use texture_fx::{FxWorldView, TextureFx};

/// The terrain atlas (`getTexture("/terrain.png")`, Minecraft.java:967).
pub const TERRAIN_PNG: &str = assets::TERRAIN_PNG;
/// The items atlas (`tileImage` 1, TextureFX.java:24).
pub const ITEMS_PNG: &str = "/gui/items.png";
/// The watch's dial (TextureWatchFX.java:25).
pub const DIAL_PNG: &str = "/misc/dial.png";

/// The `##` prefix: `unwrapImageByColumns`.
// src: RenderEngine.java:53, :108
const PREFIX_UNWRAP: &str = "##";
/// The `%clamp%` prefix: `clampTexture`.
// src: RenderEngine.java:55, :110
const PREFIX_CLAMP: &str = "%clamp%";
/// The `%blur%` prefix: `blurTexture`.
// src: RenderEngine.java:59, :114
const PREFIX_BLUR: &str = "%blur%";

/// An ARGB image as `BufferedImage.getRGB` returns it (`0xAARRGGBB` ints,
/// row-major).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArgbImage {
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// Pixels.
    pub pixels: Vec<i32>,
}

impl ArgbImage {
    /// A transparent image (`new BufferedImage(w, h, TYPE_INT_ARGB)`).
    pub fn new(width: u32, height: u32) -> Self {
        ArgbImage {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    /// From 8-bit RGBA bytes.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Self {
        let pixels = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| (p[3] as i32) << 24 | (p[0] as i32) << 16 | (p[1] as i32) << 8 | p[2] as i32)
            .collect();
        ArgbImage {
            width,
            height,
            pixels,
        }
    }

    /// `(width, height, pixels)`, as the FX constructors take it.
    pub fn view(&self) -> (u32, u32, &[i32]) {
        (self.width, self.height, &self.pixels)
    }
}

/// Why `readTextureImage` produced nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
enum ReadError {
    /// `getResourceAsStream` returned null.
    Missing,
    /// `ImageIO.read` threw an `IOException`.
    Io(String),
}

/// `readTextureImage(texturePack.getResourceAsStream(name))`.
// src: RenderEngine.java:535-539, TexturePackBase.java:28-30
fn read_texture_image(jar: &JarAssets, name: &str) -> Result<ArgbImage, ReadError> {
    if jar.get(name).is_none() {
        return Err(ReadError::Missing);
    }
    jar.rgba(name)
        .map(|(w, h, px)| ArgbImage::from_rgba(w, h, &px))
        .map_err(|e| ReadError::Io(e.to_string()))
}

/// `unwrapImageByColumns`: the image cut into 16-pixel columns stacked
/// top to bottom (`drawImage` onto a transparent `TYPE_INT_ARGB` image;
/// a fully transparent source pixel stays 0).
// src: RenderEngine.java:140-151
pub fn unwrap_image_by_columns(img: &ArgbImage) -> ArgbImage {
    let var2 = img.width / 16;
    let mut out = ArgbImage::new(16, img.height * var2);
    for var5 in 0..var2 {
        for y in 0..img.height {
            for x in 0..16 {
                let p = img.pixels[(y * img.width + var5 * 16 + x) as usize];
                let p = if p >> 24 & 255 == 0 { 0 } else { p };
                out.pixels[((var5 * img.height + y) * 16 + x) as usize] = p;
            }
        }
    }
    out
}

/// The glyphs of the missing texture's "missingtex" (project-owned pixel
/// font; `drawString`'s font is AWT's platform default): each glyph is its
/// top row (relative to the baseline at y 10) and its rows.
const MISSING_GLYPHS: [(i32, &[&str]); 10] = [
    (-5, &["##.#.", "#.#.#", "#.#.#", "#.#.#", "#.#.#"]),
    (-7, &["#", ".", "#", "#", "#", "#", "#"]),
    (-5, &[".###", "#...", ".##.", "...#", "###."]),
    (-5, &[".###", "#...", ".##.", "...#", "###."]),
    (-7, &["#", ".", "#", "#", "#", "#", "#"]),
    (-5, &["###.", "#..#", "#..#", "#..#", "#..#"]),
    (
        -5,
        &[".###", "#..#", "#..#", "#..#", ".###", "...#", "###."],
    ),
    (-7, &[".#.", ".#.", "###", ".#.", ".#.", ".#.", "..#"]),
    (-5, &[".##.", "#..#", "####", "#...", ".###"]),
    (-5, &["#...#", ".#.#.", "..#..", ".#.#.", "#...#"]),
];

/// `missingTextureImage`: 64×64 white, "missingtex" drawn in black at
/// (1, 10).
// src: RenderEngine.java:32, :37-42
pub fn missing_texture_image() -> ArgbImage {
    const WHITE: i32 = -1; // Color.WHITE, opaque
    const BLACK: i32 = 0xff00_0000_u32 as i32; // Color.BLACK, opaque
    let mut img = ArgbImage::new(64, 64);
    img.pixels.fill(WHITE);
    let mut x = 1;
    for (top, rows) in MISSING_GLYPHS {
        let w = rows[0].len() as i32;
        for (dy, row) in rows.iter().enumerate() {
            for (dx, c) in row.bytes().enumerate() {
                if c == b'#' {
                    let px = x + dx as i32;
                    let py = 10 + top + dy as i32;
                    img.pixels[(py * 64 + px) as usize] = BLACK;
                }
            }
        }
        x += w + 1;
    }
    img
}

/// The sampler `setupTexture` leaves on the texture: `GL_NEAREST` (no
/// mipmaps), `GL_LINEAR` for `blurTexture`, `GL_REPEAT` or, for
/// `clampTexture`, `GL_CLAMP` (clamp to edge: with nearest sampling GL's
/// border never shows).
// src: RenderEngine.java:163-183
pub fn sampler(blur: bool, clamp: bool) -> ImageSampler {
    let mut d = ImageSamplerDescriptor::nearest();
    if blur {
        d.set_filter(ImageFilterMode::Linear);
    }
    d.set_address_mode(if clamp {
        ImageAddressMode::ClampToEdge
    } else {
        ImageAddressMode::Repeat
    });
    ImageSampler::Descriptor(d)
}

/// `setupTexture`'s pixel conversion: ARGB ints to RGBA bytes, with the
/// anaglyph transform when the option is on.
// src: RenderEngine.java:185-217
pub fn argb_to_rgba(pixels: &[i32], anaglyph: bool) -> Vec<u8> {
    let mut var6 = vec![0u8; pixels.len() * 4];
    for (var7, &p) in pixels.iter().enumerate() {
        let var8 = p >> 24 & 255;
        let mut var9 = p >> 16 & 255;
        let mut var10 = p >> 8 & 255;
        let mut var11 = p & 255;
        if anaglyph {
            (var9, var10, var11) = texture_fx::anaglyph(var9, var10, var11);
        }
        var6[var7 * 4] = var9 as u8;
        var6[var7 * 4 + 1] = var10 as u8;
        var6[var7 * 4 + 2] = var11 as u8;
        var6[var7 * 4 + 3] = var8 as u8;
    }
    var6
}

/// The Bevy image of an upload: RGBA8 sRGB, one mip level, the sampler.
pub fn setup_texture(width: u32, height: u32, rgba: Vec<u8>, blur: bool, clamp: bool) -> Image {
    let mut img = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    img.sampler = sampler(blur, clamp);
    img
}

/// RenderBlocks' texture rectangle of a whole 16×16 tile of a 256×256
/// atlas, `[u_min, v_min, u_max, v_max]`, as `renderBottomFace` & co.
/// compute it for a full-size block face (`minX = minZ = 0`, `maxX = maxZ =
/// 1`): `x = (index & 15) << 4`, `y = index & 240`, `u = x / 256`,
/// `u' = (x + 16 - 0.01) / 256`, the same for v. In `f64`, as the Java.
// src: RenderBlocks.java:2660-2665
pub fn tile_uv_f64(icon_index: i32) -> [f64; 4] {
    let var10 = (icon_index & 15) << 4;
    let var11 = icon_index & 240;
    [
        (var10 as f64 + 0.0 * 16.0) / 256.0,
        (var11 as f64 + 0.0 * 16.0) / 256.0,
        (var10 as f64 + 1.0 * 16.0 - 0.01) / 256.0,
        (var11 as f64 + 1.0 * 16.0 - 0.01) / 256.0,
    ]
}

/// [`tile_uv_f64`] as `f32` (the vertex attribute).
// src: RenderBlocks.java:2660-2665
pub fn tile_uv(icon_index: i32) -> [f32; 4] {
    tile_uv_f64(icon_index).map(|v| v as f32)
}

/// One texture the engine uploaded.
#[derive(Clone, Debug)]
pub struct TextureEntry {
    /// The Bevy image.
    pub handle: Handle<Image>,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
    /// The CPU copy of the uploaded pixels (RGBA8, after the anaglyph
    /// transform, with the FX tiles written in).
    pub rgba: Vec<u8>,
    /// `blurTexture` at upload.
    pub blur: bool,
    /// `clampTexture` at upload.
    pub clamp: bool,
    /// The source of an `allocateAndSetupTexture` texture
    /// (`textureNameToImageMap`), re-uploaded by `refreshTextures`.
    pub source: Option<ArgbImage>,
    /// The CPU copy changed since the Bevy image was last written.
    dirty: bool,
}

/// `RenderEngine` (module docs).
#[derive(Resource, Debug)]
pub struct Textures {
    /// `textureMap`: by name (with its prefix).
    // src: RenderEngine.java:21
    texture_map: BTreeMap<String, TextureEntry>,
    /// `field_28151_c`: `func_28149_a`'s `int[]` cache, by name.
    // src: RenderEngine.java:22
    texture_contents: BTreeMap<String, Arc<[i32]>>,
    /// `textureNameToImageMap`: the `allocateAndSetupTexture` textures.
    // src: RenderEngine.java:23
    allocated: Vec<TextureEntry>,
    /// `textureList`: the FX, in registration (= update) order.
    // src: RenderEngine.java:26
    pub texture_list: Vec<TextureFx>,
    /// `options.anaglyph` as last seen (kept in step with the game's
    /// settings by the texture systems).
    pub anaglyph: bool,
    /// `clampTexture`.
    // src: RenderEngine.java:29
    clamp_texture: bool,
    /// `blurTexture`.
    // src: RenderEngine.java:30
    blur_texture: bool,
    /// The client's `Math.random()` stream for the FX ([`texture_fx`]
    /// module docs).
    pub math_random: JavaRandom,
}

impl Textures {
    /// `new RenderEngine(texturePackList, gameSettings)` with an empty FX
    /// list and a clock-seeded `Math.random()` stream.
    // src: RenderEngine.java:34-43
    pub fn new(anaglyph: bool) -> Self {
        Textures {
            texture_map: BTreeMap::new(),
            texture_contents: BTreeMap::new(),
            allocated: Vec::new(),
            texture_list: Vec::new(),
            anaglyph,
            clamp_texture: false,
            blur_texture: false,
            math_random: JavaRandom::new(clock::new_random_seed()),
        }
    }

    /// Reads a resource under a name's prefix: `(image, blur, clamp)` with
    /// `getTexture`'s flag handling. The flags are set before the read and
    /// reset after the upload; an `IOException` skips the reset, so the
    /// flag stays on for the next texture.
    // src: RenderEngine.java:108-125
    fn read_prefixed(&mut self, jar: &JarAssets, name: &str) -> Result<ArgbImage, ReadError> {
        if let Some(rest) = name.strip_prefix(PREFIX_UNWRAP) {
            read_texture_image(jar, rest).map(|i| unwrap_image_by_columns(&i))
        } else if let Some(rest) = name.strip_prefix(PREFIX_CLAMP) {
            self.clamp_texture = true;
            read_texture_image(jar, rest)
        } else if let Some(rest) = name.strip_prefix(PREFIX_BLUR) {
            self.blur_texture = true;
            read_texture_image(jar, rest)
        } else {
            match read_texture_image(jar, name) {
                // src: RenderEngine.java:119-121
                Err(ReadError::Missing) => Ok(missing_texture_image()),
                r => r,
            }
        }
    }

    /// Resets the flag the prefix set (after a successful upload).
    fn reset_prefix_flags(&mut self, name: &str) {
        if name.starts_with(PREFIX_CLAMP) {
            self.clamp_texture = false;
        } else if name.starts_with(PREFIX_BLUR) {
            self.blur_texture = false;
        }
    }

    /// Uploads an image as a new texture with the current flags.
    fn new_entry(&self, img: &ArgbImage, images: &mut Assets<Image>) -> TextureEntry {
        let rgba = argb_to_rgba(&img.pixels, self.anaglyph);
        let handle = images.add(setup_texture(
            img.width,
            img.height,
            rgba.clone(),
            self.blur_texture,
            self.clamp_texture,
        ));
        TextureEntry {
            handle,
            width: img.width,
            height: img.height,
            rgba,
            blur: self.blur_texture,
            clamp: self.clamp_texture,
            source: None,
            dirty: false,
        }
    }

    /// `setupTexture(image, name)` on an existing texture: replaces its
    /// pixels, size and sampler.
    // src: RenderEngine.java:162-244
    fn setup_existing(
        anaglyph: bool,
        blur: bool,
        clamp: bool,
        e: &mut TextureEntry,
        img: &ArgbImage,
        images: &mut Assets<Image>,
    ) {
        e.rgba = argb_to_rgba(&img.pixels, anaglyph);
        e.width = img.width;
        e.height = img.height;
        e.blur = blur;
        e.clamp = clamp;
        e.dirty = false;
        let new = setup_texture(img.width, img.height, e.rgba.clone(), blur, clamp);
        if let Some(mut i) = images.get_mut(&e.handle) {
            *i = new;
        } else {
            let _ = images.insert(&e.handle, new);
        }
    }

    /// `getTexture(name)`: the cached texture, or the resource read (with
    /// the name's prefix) and uploaded. A missing resource without a
    /// prefix, or an unreadable one, gives the missing texture.
    // src: RenderEngine.java:98-138
    pub fn get_texture(
        &mut self,
        name: &str,
        jar: &JarAssets,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        if let Some(e) = self.texture_map.get(name) {
            return e.handle.clone();
        }
        let e = match self.read_prefixed(jar, name) {
            Ok(img) => {
                let e = self.new_entry(&img, images);
                self.reset_prefix_flags(name);
                e
            }
            Err(ReadError::Io(msg)) => {
                // src: RenderEngine.java:129-135 (printStackTrace; the
                // prefix's flag stays set: vanilla bug, transcribed)
                error!("RenderEngine.getTexture({name}): {msg}");
                self.new_entry(&missing_texture_image(), images)
            }
            Err(ReadError::Missing) => {
                // Vanilla: ImageIO.read(null) throws an uncaught
                // IllegalArgumentException (module docs).
                error!("RenderEngine.getTexture({name}): no such resource");
                let e = self.new_entry(&missing_texture_image(), images);
                self.reset_prefix_flags(name);
                e
            }
        };
        let h = e.handle.clone();
        self.texture_map.insert(name.to_string(), e);
        h
    }

    /// The cached texture of `name`, if `getTexture` loaded it.
    pub fn entry(&self, name: &str) -> Option<&TextureEntry> {
        self.texture_map.get(name)
    }

    /// The texture behind a handle (named or allocated).
    pub fn entry_by_handle(&self, handle: &Handle<Image>) -> Option<&TextureEntry> {
        let id = handle.id();
        self.texture_map
            .values()
            .chain(self.allocated.iter())
            .find(|e| e.handle.id() == id)
    }

    /// The terrain atlas's handle (loading it if needed).
    pub fn terrain(&mut self, jar: &JarAssets, images: &mut Assets<Image>) -> Handle<Image> {
        self.get_texture(TERRAIN_PNG, jar, images)
    }

    /// The items atlas's handle (loading it if needed).
    pub fn items(&mut self, jar: &JarAssets, images: &mut Assets<Image>) -> Handle<Image> {
        self.get_texture(ITEMS_PNG, jar, images)
    }

    /// `func_28149_a(name)`: the image as ARGB ints, cached (the colour
    /// maps). A missing or unreadable resource gives the missing texture's
    /// pixels.
    // src: RenderEngine.java:45-81
    pub fn get_texture_contents(&mut self, name: &str, jar: &JarAssets) -> Arc<[i32]> {
        if let Some(v) = self.texture_contents.get(name) {
            return v.clone();
        }
        let px: Arc<[i32]> = match self.read_prefixed(jar, name) {
            Ok(img) => {
                self.reset_prefix_flags(name);
                img.pixels.into()
            }
            Err(e) => {
                // src: RenderEngine.java:74-79
                error!("RenderEngine.func_28149_a({name}): {e:?}");
                if e == ReadError::Missing {
                    self.reset_prefix_flags(name);
                }
                missing_texture_image().pixels.into()
            }
        };
        self.texture_contents.insert(name.to_string(), px.clone());
        px
    }

    /// The colour maps as `startGame` installs them: water, grass, foliage.
    // src: Minecraft.java:242-244
    pub fn load_colorizers(&mut self, jar: &JarAssets) -> Colorizers {
        Colorizers {
            water: self.get_texture_contents(assets::MISC_WATERCOLOR_PNG, jar),
            grass: self.get_texture_contents(assets::MISC_GRASSCOLOR_PNG, jar),
            foliage: self.get_texture_contents(assets::MISC_FOLIAGECOLOR_PNG, jar),
        }
    }

    /// `allocateAndSetupTexture(image)`: a new texture kept with its source
    /// for `refreshTextures` (the font, the map, the pack thumbnail).
    // src: RenderEngine.java:153-160
    pub fn allocate_and_setup_texture(
        &mut self,
        img: ArgbImage,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        let mut e = self.new_entry(&img, images);
        e.source = Some(img);
        let h = e.handle.clone();
        self.allocated.push(e);
        h
    }

    /// `deleteTexture(name)`: forgets an allocated texture (the handle is
    /// dropped; Bevy frees the image when no one else holds it).
    // src: RenderEngine.java:297-303
    pub fn delete_texture(&mut self, handle: &Handle<Image>) {
        let id = handle.id();
        self.allocated.retain(|e| e.handle.id() != id);
    }

    /// `func_28150_a(pixels, w, h, name)`: uploads ARGB ints into the
    /// top-left `w × h` of an existing texture (the map item), with the
    /// current flags and the anaglyph transform. The Bevy image is written
    /// at once, as the `glTexSubImage2D` at the end puts the pixels on the
    /// GPU at once: the caller runs at render time (MapItemRenderer.java:57),
    /// also while the game is paused, when no tick flushes.
    // src: RenderEngine.java:246-295
    pub fn upload_argb(
        &mut self,
        pixels: &[i32],
        w: u32,
        h: u32,
        handle: &Handle<Image>,
        images: &mut Assets<Image>,
    ) {
        let rgba = argb_to_rgba(pixels, self.anaglyph);
        let (blur, clamp) = (self.blur_texture, self.clamp_texture);
        let id = handle.id();
        let Some(e) = self
            .texture_map
            .values_mut()
            .chain(self.allocated.iter_mut())
            .find(|e| e.handle.id() == id)
        else {
            return;
        };
        e.blur = blur;
        e.clamp = clamp;
        write_region(e, 0, 0, w, h, &rgba);
        // src: RenderEngine.java:294 (glTexSubImage2D: on the GPU now)
        Self::flush_entry(e, images);
    }

    /// `registerTextureFX(fx)`: appends it and runs its first `onTick`
    /// (with `anaglyphEnabled` still false and no tile written).
    // src: RenderEngine.java:350-353
    pub fn register_texture_fx(&mut self, mut fx: TextureFx, view: Option<&FxWorldView>) {
        fx.on_tick(&mut self.math_random, view);
        self.texture_list.push(fx);
    }

    /// `updateDynamicTextures()`: each FX in order takes the anaglyph
    /// option, ticks, and has its tile written `tileSize × tileSize` times
    /// at its icon in its atlas (`bindImage` loads the atlas if needed).
    /// The second loop (FX with their own `textureId`) never runs: no FX
    /// sets one. The changed atlases are written to their Bevy images.
    // src: RenderEngine.java:355-433
    pub fn update_dynamic_textures(
        &mut self,
        jar: &JarAssets,
        images: &mut Assets<Image>,
        view: Option<&FxWorldView>,
    ) {
        for var1 in 0..self.texture_list.len() {
            let fx = &mut self.texture_list[var1];
            fx.anaglyph_enabled = self.anaglyph;
            fx.on_tick(&mut self.math_random, view);
            let Some(path) = self.texture_list[var1].bind_image_path() else {
                continue;
            };
            self.get_texture(path, jar, images);
            let fx = &self.texture_list[var1];
            let Some(e) = self.texture_map.get_mut(path) else {
                continue;
            };
            for var3 in 0..fx.tile_size {
                for var4 in 0..fx.tile_size {
                    let (x, y) = texture_fx::tile_origin(fx.icon_index, var3, var4);
                    if x >= 0 && y >= 0 {
                        write_region(e, x as u32, y as u32, 16, 16, &fx.image_data);
                    }
                }
            }
        }
        self.flush(images);
    }

    /// Writes every changed CPU copy into its Bevy image (marking the asset
    /// modified, so the render world uploads it again).
    pub fn flush(&mut self, images: &mut Assets<Image>) {
        for e in self
            .texture_map
            .values_mut()
            .chain(self.allocated.iter_mut())
        {
            Self::flush_entry(e, images);
        }
    }

    /// Writes one CPU copy into its Bevy image if it changed.
    fn flush_entry(e: &mut TextureEntry, images: &mut Assets<Image>) {
        if !e.dirty {
            return;
        }
        e.dirty = false;
        if let Some(mut img) = images.get_mut(&e.handle) {
            img.data = Some(e.rgba.clone());
            img.sampler = sampler(e.blur, e.clamp);
        }
    }

    /// `refreshTextures()`: every texture uploaded again from its source
    /// (the anaglyph option changed). The allocated ones from their kept
    /// images; the named ones read again with their prefixes; the
    /// `func_28149_a` arrays read again (the caller re-installs the colour
    /// maps from them, [`Textures::load_colorizers`]). The prefix flags are
    /// set before each read and reset after its upload; an `IOException`
    /// skips the reset (vanilla bug, transcribed). The downloadable images'
    /// `textureSetupComplete` reset is deferred (skins, D2). Order: the
    /// named textures by name (Java iterates a `HashMap`).
    // src: RenderEngine.java:463-533
    pub fn refresh_textures(&mut self, jar: &JarAssets, images: &mut Assets<Image>) {
        let anaglyph = self.anaglyph;
        for e in &mut self.allocated {
            if let Some(src) = e.source.clone() {
                let (b, c) = (self.blur_texture, self.clamp_texture);
                Self::setup_existing(anaglyph, b, c, e, &src, images);
            }
        }
        let names: Vec<String> = self.texture_map.keys().cloned().collect();
        for name in &names {
            let img = match self.read_prefixed(jar, name) {
                Ok(img) => img,
                Err(ReadError::Io(msg)) => {
                    error!("RenderEngine.refreshTextures({name}): {msg}");
                    continue;
                }
                Err(ReadError::Missing) => {
                    error!("RenderEngine.refreshTextures({name}): no such resource");
                    missing_texture_image()
                }
            };
            let (b, c) = (self.blur_texture, self.clamp_texture);
            if let Some(e) = self.texture_map.get_mut(name) {
                Self::setup_existing(anaglyph, b, c, e, &img, images);
            }
            self.blur_texture = false;
            self.clamp_texture = false;
        }
        let names: Vec<String> = self.texture_contents.keys().cloned().collect();
        for name in &names {
            let img = match self.read_prefixed(jar, name) {
                Ok(img) => img,
                Err(e) => {
                    error!("RenderEngine.refreshTextures({name}): {e:?}");
                    if e == ReadError::Missing {
                        self.blur_texture = false;
                        self.clamp_texture = false;
                    }
                    continue;
                }
            };
            // `func_28147_a` writes into the same array.
            self.texture_contents
                .insert(name.clone(), img.pixels.into());
            self.blur_texture = false;
            self.clamp_texture = false;
        }
    }
}

/// `glTexSubImage2D` into a CPU copy: `w × h` RGBA pixels at `(x, y)`. A
/// region outside the texture is an error in GL and writes nothing.
fn write_region(e: &mut TextureEntry, x: u32, y: u32, w: u32, h: u32, rgba: &[u8]) {
    if x + w > e.width || y + h > e.height || rgba.len() < (w * h * 4) as usize {
        return;
    }
    for row in 0..h {
        let dst = (((y + row) * e.width + x) * 4) as usize;
        let src = (row * w * 4) as usize;
        e.rgba[dst..dst + (w * 4) as usize].copy_from_slice(&rgba[src..src + (w * 4) as usize]);
    }
    e.dirty = true;
}

/// `Textures` with `JarAssets` and `Assets<Image>`, for systems that load
/// textures by name (`renderEngine.getTexture(name)`).
#[derive(SystemParam)]
pub struct TextureLoader<'w> {
    /// The engine.
    pub textures: ResMut<'w, Textures>,
    /// The jar.
    pub jar: Res<'w, JarAssets>,
    /// The images.
    pub images: ResMut<'w, Assets<Image>>,
}

impl TextureLoader<'_> {
    /// `getTexture(name)`.
    pub fn get(&mut self, name: &str) -> Handle<Image> {
        let TextureLoader {
            textures,
            jar,
            images,
        } = self;
        textures.get_texture(name, jar, images)
    }
}

/// What the compass and the watch see of the game this tick: `None` unless
/// `theWorld` and `thePlayer` are both set.
// src: TextureCompassFX.java:52-60, TextureWatchFX.java:35-41
pub fn fx_world_view(game: &Game) -> Option<FxWorldView> {
    let w = game.the_world.as_ref()?;
    let p = game.player()?;
    let spawn = w.get_spawn_point();
    Some(FxWorldView {
        spawn_x: spawn.pos_x,
        spawn_z: spawn.pos_z,
        player_x: p.base.pos_x,
        player_z: p.base.pos_z,
        player_yaw: p.base.rotation_yaw,
        is_nether: w.world_provider.is_nether,
        celestial_angle: w.get_celestial_angle(1.0),
    })
}

/// The texture systems run after these, in `Update`; renderers that read
/// [`Textures`] or [`Colorizers`] in the same frame order after it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TexturesSet;

/// Installs [`Textures`] (with the FX registered in `startGame`'s order)
/// and [`Colorizers`], and the systems.
pub struct TexturesPlugin {
    /// The jar (the FX constructors read `items.png` and `dial.png` from it).
    pub jar: JarAssets,
}

impl Plugin for TexturesPlugin {
    fn build(&self, app: &mut App) {
        let anaglyph = app
            .world()
            .get_non_send::<Game>()
            .is_some_and(|g| g.game_settings.anaglyph);
        let mut t = Textures::new(anaglyph);
        // src: Minecraft.java:242-244
        let colorizers = t.load_colorizers(&self.jar);
        // src: Minecraft.java:275-283 (the FX constructors read the jar's
        // own images, TextureCompassFX.java:20, TextureWatchFX.java:21, :25)
        let read = |name: &str| match self.jar.rgba(name) {
            Ok((w, h, px)) => Some(ArgbImage::from_rgba(w, h, &px)),
            Err(e) => {
                error!("{e}");
                None
            }
        };
        let items = read(ITEMS_PNG);
        let dial = read(DIAL_PNG);
        for fx in texture_fx::startup_fx_list(
            items.as_ref().map(|i| i.view()),
            dial.as_ref().map(|i| i.view()),
        ) {
            t.register_texture_fx(fx, None);
        }
        app.insert_resource(t)
            .insert_resource(colorizers)
            .add_systems(Startup, load_atlases)
            .add_systems(
                RunTickHook::UpdateDynamicTextures,
                update_dynamic_textures_system,
            )
            .add_systems(
                Update,
                refresh_on_settings.in_set(TexturesSet).after(GameFrameSet),
            );
    }
}

/// Loads the two atlases at start-up (vanilla loads them on first bind:
/// the first frame), with the options' anaglyph setting.
fn load_atlases(
    game: NonSend<Game>,
    mut textures: ResMut<Textures>,
    jar: Res<JarAssets>,
    images: Option<ResMut<Assets<Image>>>,
) {
    let Some(mut images) = images else {
        return;
    };
    textures.anaglyph = game.game_settings.anaglyph;
    textures.terrain(&jar, &mut images);
    textures.items(&jar, &mut images);
}

/// `renderEngine.updateDynamicTextures()` inside `runTick` (the hook runs
/// only when the game is not paused, Minecraft.java:968-970).
// src: Minecraft.java:967-970
fn update_dynamic_textures_system(
    game: NonSend<Game>,
    mut textures: ResMut<Textures>,
    jar: Res<JarAssets>,
    images: Option<ResMut<Assets<Image>>>,
) {
    let Some(mut images) = images else {
        return;
    };
    textures.anaglyph = game.game_settings.anaglyph;
    let view = fx_world_view(&game);
    textures.update_dynamic_textures(&jar, &mut images, view.as_ref());
}

/// `renderEngine.refreshTextures()` after the anaglyph option changed
/// (GameSettings.java:139-142), and the colour maps installed again (their
/// arrays are refreshed in place in vanilla).
fn refresh_on_settings(
    mut events: MessageReader<ClientEvent>,
    game: NonSend<Game>,
    mut textures: ResMut<Textures>,
    mut colorizers: ResMut<Colorizers>,
    jar: Res<JarAssets>,
    images: Option<ResMut<Assets<Image>>>,
) {
    let refresh = events
        .read()
        .any(|e| matches!(e, ClientEvent::Settings(SettingsEffect::RefreshTextures)));
    textures.anaglyph = game.game_settings.anaglyph;
    if !refresh {
        return;
    }
    let Some(mut images) = images else {
        return;
    };
    textures.refresh_textures(&jar, &mut images);
    *colorizers = textures.load_colorizers(&jar);
}

/// The `dump_texture` dev action: writes the CPU copy of a texture (with
/// the FX tiles) to a PNG. `name` is `"terrain"`, `"items"` or a jar path
/// (with its prefix, as `getTexture` takes it); the texture is loaded if it
/// was not.
pub fn dump_texture(world: &mut World, name: &str, path: &Path) -> Result<(), String> {
    let name = match name {
        "terrain" => TERRAIN_PNG,
        "items" => ITEMS_PNG,
        n => n,
    };
    let (w, h, rgba) = world.resource_scope(|world, mut textures: Mut<Textures>| {
        world.resource_scope(|world, mut images: Mut<Assets<Image>>| {
            let jar = world.resource::<JarAssets>();
            textures.get_texture(name, jar, &mut images);
            textures
                .entry(name)
                .map(|e| (e.width, e.height, e.rgba.clone()))
                .ok_or_else(|| format!("no texture {name}"))
        })
    })?;
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("{}: {e}", d.display()))?;
    }
    image::save_buffer_with_format(
        path,
        &rgba,
        w,
        h,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// FNV-1a over the frames, for the FX regression checksums.
    #[derive(Debug)]
    pub(crate) struct Fnv(pub u64);

    impl Default for Fnv {
        fn default() -> Self {
            Fnv(0xcbf2_9ce4_8422_2325)
        }
    }

    impl Fnv {
        pub(crate) fn add(&mut self, bytes: &[u8]) {
            for &b in bytes {
                self.0 ^= b as u64;
                self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
            }
        }
    }

    // Regression checksums of the FX tests (taken from this implementation,
    // not from Java: there is no oracle for the client).
    pub(crate) const FLAMES_CHECKSUM: u64 = 0xfe65_2bfa_bcdc_e868;
    pub(crate) const LAVA_CHECKSUM: u64 = 0x315a_2468_97f2_7b7d;
    pub(crate) const LAVA_FLOW_CHECKSUM: u64 = 0xdd99_237d_ca22_06cd;
    pub(crate) const WATER_CHECKSUM: u64 = 0x1e61_0f32_0368_96df;
    pub(crate) const WATER_FLOW_CHECKSUM: u64 = 0x38ff_35f3_3253_8003;
    pub(crate) const PORTAL_CHECKSUM: u64 = 0x4895_0b6d_6218_b13b;
    pub(crate) const COMPASS_CHECKSUM: u64 = 0x672c_a3f1_82e0_c6e1;
    pub(crate) const WATCH_CHECKSUM: u64 = 0x72de_aeba_4920_cc9a;

    /// Hand-checked against RenderBlocks.java:2660-2665: tile 0 is
    /// `[0, 0, 15.99/256, 15.99/256]`; tile 205 (water) is column 13, row
    /// 12: `x = 208`, `y = 192`.
    #[test]
    fn tile_uvs() {
        assert_eq!(tile_uv_f64(0), [0.0, 0.0, 15.99 / 256.0, 15.99 / 256.0]);
        let uv = tile_uv_f64(205);
        assert_eq!(uv[0], 208.0 / 256.0);
        assert_eq!(uv[1], 192.0 / 256.0);
        assert_eq!(uv[2], (208.0 + 16.0 - 0.01) / 256.0);
        assert_eq!(uv[3], (192.0 + 16.0 - 0.01) / 256.0);
        assert_eq!(tile_uv(255)[2], ((240.0f64 + 16.0 - 0.01) / 256.0) as f32);
    }

    #[test]
    fn argb_conversion_and_anaglyph() {
        let px = [0x80ff_1400_u32 as i32, 0x0000_ff00];
        assert_eq!(
            argb_to_rgba(&px, false),
            [0xff, 0x14, 0x00, 0x80, 0, 0xff, 0, 0]
        );
        // (255*30 + 20*59)/100 = 88, (255*30 + 20*70)/100 = 90, 255*30/100 = 76.
        assert_eq!(&argb_to_rgba(&px, true)[0..4], [88, 90, 76, 0x80]);
    }

    /// The `dump_texture` script action parses.
    #[test]
    fn dump_texture_action_parses() {
        let s = crate::script::Script::parse(
            r#"[{"action": "dump_texture", "name": "terrain", "path": "build/screens/t.png", "tick": 40}]"#,
        )
        .unwrap();
        assert_eq!(s.steps[0].tick, Some(40));
        assert_eq!(
            s.steps[0].action,
            crate::script::Action::DumpTexture {
                name: "terrain".into(),
                path: "build/screens/t.png".into(),
            }
        );
    }

    #[test]
    fn missing_texture() {
        let m = missing_texture_image();
        assert_eq!((m.width, m.height), (64, 64));
        assert_eq!(m.pixels[0], -1);
        assert_eq!(m.pixels[63 * 64 + 63], -1);
        let black = m
            .pixels
            .iter()
            .filter(|&&p| p == 0xff00_0000_u32 as i32)
            .count();
        assert!(black > 50, "{black}");
        // Nothing drawn above row 3 or below row 11.
        assert!(m.pixels[..3 * 64].iter().all(|&p| p == -1));
        assert!(m.pixels[12 * 64..].iter().all(|&p| p == -1));
    }

    #[test]
    fn unwrap_columns() {
        let mut img = ArgbImage::new(32, 2);
        for (i, p) in img.pixels.iter_mut().enumerate() {
            *p = 0xff00_0000_u32 as i32 | i as i32;
        }
        let u = unwrap_image_by_columns(&img);
        assert_eq!((u.width, u.height), (16, 4));
        // Row 2 is the first row of the second column.
        assert_eq!(u.pixels[2 * 16] & 0xffff, 16);
        assert_eq!(u.pixels[3 * 16 + 15] & 0xffff, 32 + 31);
    }

    #[test]
    fn samplers() {
        let ImageSampler::Descriptor(d) = sampler(false, false) else {
            panic!()
        };
        assert_eq!(d.mag_filter, ImageFilterMode::Nearest);
        assert_eq!(d.min_filter, ImageFilterMode::Nearest);
        assert_eq!(d.address_mode_u, ImageAddressMode::Repeat);
        let ImageSampler::Descriptor(d) = sampler(true, true) else {
            panic!()
        };
        assert_eq!(d.mag_filter, ImageFilterMode::Linear);
        assert_eq!(d.address_mode_v, ImageAddressMode::ClampToEdge);
    }

    /// With the real jar (skipped when absent, D8): the atlases load, an
    /// FX update writes the fire, water, lava and portal tiles of
    /// terrain.png and the compass and watch tiles of items.png, the Bevy
    /// image follows the CPU copy, and a missing name gives the missing
    /// texture.
    /// `func_28150_a` puts the pixels on the GPU at once (its
    /// `glTexSubImage2D`, RenderEngine.java:294): the Bevy image matches the
    /// CPU copy with no tick (no `update_dynamic_textures`) in between.
    #[test]
    fn upload_argb_reaches_the_image_at_once() {
        let mut images = Assets::<Image>::default();
        let mut t = Textures::new(false);
        let h = t.allocate_and_setup_texture(ArgbImage::new(4, 4), &mut images);
        let px: Vec<i32> = (0..4)
            .map(|i| (0xff00_0000_u32 as i32) | (i * 0x0010_2030))
            .collect();
        t.upload_argb(&px, 2, 2, &h, &mut images);
        let e = t.entry_by_handle(&h).unwrap();
        assert_eq!(&e.rgba[0..8], &argb_to_rgba(&px[0..2], false)[..]);
        assert_eq!(&e.rgba[16..24], &argb_to_rgba(&px[2..4], false)[..]);
        assert_eq!(images.get(&h).unwrap().data.as_ref().unwrap(), &e.rgba);
        // Anaglyph: the transform applies on upload too.
        t.anaglyph = true;
        t.upload_argb(&px, 2, 2, &h, &mut images);
        let e = t.entry_by_handle(&h).unwrap();
        assert_eq!(&e.rgba[0..8], &argb_to_rgba(&px[0..2], true)[..]);
        assert_eq!(images.get(&h).unwrap().data.as_ref().unwrap(), &e.rgba);
    }

    #[test]
    fn engine_with_the_real_jar() {
        let path = crate::paths::default_jar();
        if !path.exists() {
            return;
        }
        let jar = JarAssets::open(&path).unwrap();
        let mut images = Assets::<Image>::default();
        let mut t = Textures::new(false);
        t.math_random = JavaRandom::new(5);
        let items = jar
            .rgba(ITEMS_PNG)
            .map(|(w, h, p)| ArgbImage::from_rgba(w, h, &p))
            .ok();
        let dial = jar
            .rgba(DIAL_PNG)
            .map(|(w, h, p)| ArgbImage::from_rgba(w, h, &p))
            .ok();
        for fx in texture_fx::startup_fx_list(
            items.as_ref().map(|i| i.view()),
            dial.as_ref().map(|i| i.view()),
        ) {
            t.register_texture_fx(fx, None);
        }
        let terrain = t.terrain(&jar, &mut images);
        let before = t.entry(TERRAIN_PNG).unwrap().rgba.clone();
        let items_before = t.items(&jar, &mut images);
        let items_rgba = t.entry(ITEMS_PNG).unwrap().rgba.clone();
        for _ in 0..3 {
            t.update_dynamic_textures(&jar, &mut images, None);
        }
        let e = t.entry(TERRAIN_PNG).unwrap();
        assert_eq!((e.width, e.height), (256, 256));
        let tile = |rgba: &[u8], idx: i32| -> Vec<u8> {
            let (x, y) = texture_fx::tile_origin(idx, 0, 0);
            (0..16)
                .flat_map(|r| {
                    let o = (((y + r) * 256 + x) * 4) as usize;
                    rgba[o..o + 64].to_vec()
                })
                .collect()
        };
        for idx in [31, 47, 205, 206, 207, 222, 223, 237, 238, 239, 254, 255, 14] {
            assert_ne!(tile(&e.rgba, idx), tile(&before, idx), "tile {idx}");
        }
        // A tile no FX touches is unchanged (stone, 1).
        assert_eq!(tile(&e.rgba, 1), tile(&before, 1));
        // The FX's own tile is what landed in the atlas.
        let water = &t.texture_list[1];
        assert_eq!(tile(&e.rgba, 205), water.image_data.to_vec());
        // The Bevy image follows.
        assert_eq!(
            images.get(&terrain).unwrap().data.as_ref().unwrap(),
            &e.rgba
        );
        let ie = t.entry(ITEMS_PNG).unwrap();
        assert_eq!(ie.handle, items_before);
        assert_ne!(tile(&ie.rgba, 54), tile(&items_rgba, 54));
        // The watch tile without a world: the dial at angle ~0 replaces the
        // magenta pixels (red == blue, green 0, blue > 0); others are the
        // icon's (TextureWatchFX.java:65-96).
        let (watch_now, watch_icon) = (tile(&ie.rgba, 70), tile(&items_rgba, 70));
        assert_ne!(watch_now, watch_icon, "watch");
        let mut kept = 0;
        for (now, icon) in watch_now.chunks(4).zip(watch_icon.chunks(4)) {
            if !(icon[0] == icon[2] && icon[1] == 0 && icon[2] > 0) {
                assert_eq!(now, icon, "a non-magenta watch pixel");
                kept += 1;
            }
        }
        assert!(kept > 0 && kept < 256);
        // A tile no FX touches is unchanged.
        assert_eq!(tile(&ie.rgba, 1), tile(&items_rgba, 1));
        // Missing: the 64×64 missing texture, cached.
        let m = t.get_texture("/no/such.png", &jar, &mut images);
        assert_eq!(t.entry("/no/such.png").unwrap().width, 64);
        assert_eq!(t.get_texture("/no/such.png", &jar, &mut images), m);
        // A prefixed name: clamp, and the flag is reset afterwards.
        t.get_texture("%clamp%/misc/shadow.png", &jar, &mut images);
        assert!(t.entry("%clamp%/misc/shadow.png").unwrap().clamp);
        t.get_texture("/misc/dial.png", &jar, &mut images);
        assert!(!t.entry("/misc/dial.png").unwrap().clamp);
        // Refresh with anaglyph: terrain is re-read (FX tiles gone until
        // the next update) and converted.
        t.anaglyph = true;
        t.refresh_textures(&jar, &mut images);
        let e = t.entry(TERRAIN_PNG).unwrap();
        assert_eq!(
            tile(&e.rgba, 31),
            tile(
                &argb_to_rgba(
                    &ArgbImage::from_rgba(256, 256, &jar.rgba(TERRAIN_PNG).unwrap().2).pixels,
                    true,
                ),
                31
            )
        );
        assert_eq!(
            images.get(&terrain).unwrap().data.as_ref().unwrap(),
            &e.rgba
        );
    }
}
