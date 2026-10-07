//! `ColorizerGrass`, `ColorizerFoliage`, `ColorizerWater`
//! (ColorizerGrass.java, ColorizerFoliage.java, ColorizerWater.java): the
//! biome colour maps, 256×256 ARGB tables from `/misc/grasscolor.png`,
//! `/misc/foliagecolor.png` and `/misc/watercolor.png`
//! (`RenderEngine.func_28149_a`, Minecraft.java:242-244), looked up by
//! temperature and rainfall.
//!
//! Vanilla keeps them in static fields; here they are the [`Colorizers`]
//! resource (cheap to clone into the mesher's tasks). Until set, each table
//! is `new int[65536]` of zeros. A missing PNG yields the 64×64 missing
//! texture's 4096 entries (`func_28149_a`), and a lookup past it fails as
//! Java's `ArrayIndexOutOfBoundsException` does (a panic).

use std::sync::Arc;

use bevy::prelude::Resource;

/// The three colour maps.
#[derive(Resource, Clone, Debug)]
pub struct Colorizers {
    /// `ColorizerWater.waterBuffer` (set, never read in this version).
    // src: ColorizerWater.java:4
    pub water: Arc<[i32]>,
    /// `ColorizerGrass.grassBuffer`.
    // src: ColorizerGrass.java:4
    pub grass: Arc<[i32]>,
    /// `ColorizerFoliage.foliageBuffer`.
    // src: ColorizerFoliage.java:4
    pub foliage: Arc<[i32]>,
}

impl Default for Colorizers {
    /// `new int[65536]` each.
    fn default() -> Self {
        let zeros: Arc<[i32]> = vec![0; 65536].into();
        Colorizers {
            water: zeros.clone(),
            grass: zeros.clone(),
            foliage: zeros,
        }
    }
}

/// The table index of a temperature and rainfall: `var2 *= var0`,
/// `x = (int)((1 - var0) * 255)`, `y = (int)((1 - var2) * 255)`, index
/// `y << 8 | x`.
// src: ColorizerGrass.java:10-15, ColorizerFoliage.java:10-15
pub fn colorizer_index(var0: f64, var2: f64) -> usize {
    let var2 = var2 * var0;
    let var4 = ((1.0 - var0) * 255.0) as i32;
    let var5 = ((1.0 - var2) * 255.0) as i32;
    let i = var5 << 8 | var4;
    usize::try_from(i).unwrap_or_else(|_| panic!("colorizer index {i} out of bounds"))
}

impl Colorizers {
    /// `ColorizerGrass.getGrassColor(temperature, rainfall)`.
    // src: ColorizerGrass.java:10-15
    pub fn get_grass_color(&self, var0: f64, var2: f64) -> i32 {
        self.grass[colorizer_index(var0, var2)]
    }

    /// `ColorizerFoliage.getFoliageColor(temperature, rainfall)`.
    // src: ColorizerFoliage.java:10-15
    pub fn get_foliage_color(&self, var0: f64, var2: f64) -> i32 {
        self.foliage[colorizer_index(var0, var2)]
    }
}

/// `ColorizerFoliage.getFoliageColorPine()`: `0x619961`.
// src: ColorizerFoliage.java:17-19
pub const fn get_foliage_color_pine() -> i32 {
    6396257
}

/// `ColorizerFoliage.getFoliageColorBirch()`: `0x80a755`.
// src: ColorizerFoliage.java:21-23
pub const fn get_foliage_color_birch() -> i32 {
    8431445
}

/// `ColorizerFoliage.getFoliageColorBasic()`: `0x48b518`.
// src: ColorizerFoliage.java:25-27
pub const fn get_foliage_color_basic() -> i32 {
    4764952
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants() {
        assert_eq!(get_foliage_color_pine(), 0x619961);
        assert_eq!(get_foliage_color_birch(), 0x80a755);
        assert_eq!(get_foliage_color_basic(), 0x48b518);
    }

    /// Hand-checked index arithmetic: hot and wet (1, 1) is the top-left
    /// corner; cold (0, anything) is x 255, y 255 (`var2 * 0 = 0`);
    /// (0.5, 0.5): x = (int)127.5 = 127, rain 0.25 → y = (int)191.25 = 191.
    #[test]
    fn index() {
        assert_eq!(colorizer_index(1.0, 1.0), 0);
        assert_eq!(colorizer_index(0.0, 0.7), 255 << 8 | 255);
        assert_eq!(colorizer_index(0.5, 0.5), 191 << 8 | 127);
        assert_eq!(
            colorizer_index(0.8, 0.4),
            (((1.0 - 0.32f64) * 255.0) as usize) << 8 | 50
        );
    }

    #[test]
    fn default_is_zeros() {
        let c = Colorizers::default();
        assert_eq!(c.get_grass_color(0.5, 0.5), 0);
        assert_eq!(c.grass.len(), 65536);
    }

    /// Against the jar's maps when it is present (D8: never committed).
    /// The expected pixels are read with an independent decode of the PNG
    /// (the `image` crate straight to ARGB) at the hand-computed index.
    #[test]
    fn lookups_in_the_real_maps() {
        let path = crate::paths::default_jar();
        if !path.exists() {
            return;
        }
        let jar = crate::assets::JarAssets::open(&path).unwrap();
        let mut t = crate::textures::Textures::new(false);
        let c = t.load_colorizers(&jar);
        let argb_at = |name: &str, x: u32, y: u32| {
            let (w, _, px) = jar.rgba(name).unwrap();
            let i = ((y * w + x) * 4) as usize;
            (px[i + 3] as i32) << 24
                | (px[i] as i32) << 16
                | (px[i + 1] as i32) << 8
                | px[i + 2] as i32
        };
        // Plains (temperature 0.8, rainfall 0.4): x 50, y 173.
        assert_eq!(colorizer_index(0.8, 0.4), 173 << 8 | 50);
        assert_eq!(
            c.get_grass_color(0.8, 0.4),
            argb_at("/misc/grasscolor.png", 50, 173)
        );
        assert_eq!(
            c.get_foliage_color(0.8, 0.4),
            argb_at("/misc/foliagecolor.png", 50, 173)
        );
        // Opaque, and greenish: green is the largest channel.
        let g = c.get_grass_color(0.8, 0.4);
        assert_eq!(g >> 24 & 255, 255);
        assert!(
            (g >> 8 & 255) > (g >> 16 & 255) && (g >> 8 & 255) > (g & 255),
            "{g:#x}"
        );
        // Hot and wet corner, and the cold corner.
        assert_eq!(
            c.get_grass_color(1.0, 1.0),
            argb_at("/misc/grasscolor.png", 0, 0)
        );
        assert_eq!(
            c.get_grass_color(0.0, 0.0),
            argb_at("/misc/grasscolor.png", 255, 255)
        );
        assert_eq!(c.water.len(), 65536);
    }
}
