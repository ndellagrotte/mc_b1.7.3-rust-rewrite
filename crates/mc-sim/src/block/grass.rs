//! `BlockGrass` (BlockGrass.java) and the grass colormap lookup
//! (`ColorizerGrass`, ColorizerGrass.java). `BlockDirt` (BlockDirt.java) has
//! no behaviour of its own: a constructor only.
//!
//! Fully transcribed: `updateTick` (dying in the dark, spreading to dirt in
//! the light), `getBlockTexture` (snowy sides). `colorMultiplier` computes
//! the biome temperature and humidity and the `ColorizerGrass` index exactly;
//! the colour itself comes from `grassBuffer`, a colormap the client
//! installs from `misc/grasscolor.png` of the user's jar
//! (`ColorizerGrass.setGrassBiomeColorizer`, D8, Phase 3). The simulation has
//! no colormap: the lookup answers what Java's `new int[65536]` holds before
//! the client sets it (0, which is also what the headless oracle sees). That
//! is the whole of the Java body for a world without a client, so nothing is
//! recorded (WP-4.8; it recorded `"ColorizerGrass.getGrassColor"` before).
//! The shipping client never calls this: it tints with its own copy and the
//! jar's colormaps (`mc-client` `terrain::blocks::color_multiplier`,
//! `particles::subclasses::block_color_multiplier`, `textures::Colorizers`).

use super::dispatch::RandomSource;
use super::{Material, ids};
use crate::world::World;

/// `Block.lightOpacity[world.getBlockId(x, y, z)]`.
fn opacity_at(world: &mut World, x: i32, y: i32, z: i32) -> i32 {
    let id = world.get_block_id(x, y, z);
    super::light_opacity(id)
}

/// `updateTick(world, x, y, z, rand)`: under an opaque block in light below 4
/// grass turns to dirt (one in four); in light 9 or more it spreads to one
/// random dirt block in a 3×5×3 box below/around it whose top is lit (4) and
/// not opaque (opacity at most 2).
// src: BlockGrass.java:30-49
pub fn update_tick(world: &mut World, x: i32, y: i32, z: i32, rand: RandomSource) {
    if world.multiplayer_world {
        return;
    }
    if world.get_block_light_value(x, y + 1, z) < 4 && opacity_at(world, x, y + 1, z) > 2 {
        if world.random(rand).next_int_bounded(4) != 0 {
            return;
        }
        world.set_block_with_notify(x, y, z, ids::DIRT);
    } else if world.get_block_light_value(x, y + 1, z) >= 9 {
        let var6 = x + world.random(rand).next_int_bounded(3) - 1;
        let var7 = y + world.random(rand).next_int_bounded(5) - 3;
        let var8 = z + world.random(rand).next_int_bounded(3) - 1;
        let var9 = world.get_block_id(var6, var7 + 1, var8);
        if world.get_block_id(var6, var7, var8) == ids::DIRT
            && world.get_block_light_value(var6, var7 + 1, var8) >= 4
            && super::light_opacity(var9) <= 2
        {
            world.set_block_with_notify(var6, var7, var8, ids::GRASS);
        }
    }
}

/// `getBlockTexture(blockAccess, x, y, z, side)`: top 0, bottom 2, sides 3,
/// or 68 (snowy side) under a snow layer or snow block.
// src: BlockGrass.java:12-21
pub fn get_block_texture(world: &mut World, x: i32, y: i32, z: i32, side: i32) -> i32 {
    if side == 1 {
        0
    } else if side == 0 {
        2
    } else {
        let var6 = world.get_block_material(x, y + 1, z);
        if var6 != Material::Snow && var6 != Material::BuiltSnow {
            3
        } else {
            68
        }
    }
}

/// `colorMultiplier(blockAccess, x, y, z)`: the grass colour of the biome at
/// `(x, z)` (module docs).
// src: BlockGrass.java:23-28
pub fn color_multiplier(world: &mut World, x: i32, z: i32) -> i32 {
    biome_grass_color(world, x, z)
}

/// `getWorldChunkManager().getBiomeGenBaseAt(x, z, 1, 1)`, then
/// `ColorizerGrass.getGrassColor(temperature[0], humidity[0])`: shared by
/// `BlockGrass` and `BlockTallGrass` (BlockTallGrass.java:32-35).
// src: BlockGrass.java:24-27
pub(super) fn biome_grass_color(world: &mut World, x: i32, z: i32) -> i32 {
    let m = world.get_world_chunk_manager();
    m.get_biome_gen_base_at(x, z, 1, 1);
    let var5 = m.temperature[0];
    let var7 = m.humidity[0];
    let index = colormap_index(var5, var7);
    colormap_lookup(index)
}

/// The index `ColorizerGrass.getGrassColor` / `ColorizerFoliage.getFoliageColor`
/// read their 256×256 colormap at: `(int)((1 - temperature * humidity) * 255) << 8
/// | (int)((1 - temperature) * 255)`. The two classes share the formula.
// src: ColorizerGrass.java:10-15, ColorizerFoliage.java:10-15
pub fn colormap_index(var0: f64, mut var2: f64) -> usize {
    var2 *= var0;
    let var4 = ((1.0 - var0) * 255.0) as i32;
    let var5 = ((1.0 - var2) * 255.0) as i32;
    (var5 << 8 | var4) as usize
}

/// A colormap entry with no colormap installed: Java's `new int[65536]`
/// (ColorizerGrass.java:4, ColorizerFoliage.java:4) before the client's
/// `set*BiomeColorizer`. An index outside the table would throw
/// `ArrayIndexOutOfBoundsException` in Java; it cannot occur for the
/// normalised temperature and humidity of vanilla biomes.
pub(super) fn colormap_lookup(index: usize) -> i32 {
    let _ = index;
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_index_formula() {
        // temperature 1, humidity 1: (0 << 8) | 0
        assert_eq!(colormap_index(1.0, 1.0), 0);
        // temperature 0: x = 255, y = 255
        assert_eq!(colormap_index(0.0, 0.5), 255 << 8 | 255);
        // (int)((1 - 0.5) * 255) = 127; (int)((1 - 0.25) * 255) = 191
        assert_eq!(colormap_index(0.5, 0.5), 191 << 8 | 127);
    }
}
