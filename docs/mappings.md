# Obfuscated-name mappings (PLAN_v4 §6, Definition of Done item 4)

The vanilla tree still has MCP's unresolved names (`field_NNNN_x`, `func_NNNN_x`).
The port keeps a Java name, converted to snake_case, unless its usage proves what it
is; then it gets a descriptive name and a row here with that evidence. Kept names are
listed too, with what they do, so the next transcriber does not have to rediscover it.
Citations are into `.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft/net/minecraft/src`.

Snake-case rule for kept names: `field_9293_aM` → `field_9293_a_m`, `field_9355_A` →
`field_9355_a`, `func_27163_E` → `func_27163_e` (rustc's case lints would reject the
mixed-case originals).

## Renamed

| Java | Rust | Evidence |
|---|---|---|
| `StepSound.field_1678_a` | `StepSound::step_sound_name` | Only read as `"step." + field_1678_a` by `stepSoundDir()`/`stepSoundDir2()` (StepSound.java:22-28); values `"stone"`, `"wood"`, `"gravel"`, … (Block.java:7-15) |
| `StepSound.field_1677_b` | `StepSound::step_sound_volume` | Returned by `getVolume()` (StepSound.java:14-16); callers pass it as a sound volume (Entity.java:493, EntityLiving.java:448) |
| `StepSound.field_1679_c` | `StepSound::step_sound_pitch` | Returned by `getPitch()` (StepSound.java:18-20); passed as the pitch (Entity.java:493) |
| `MetadataChunkBlock.field_1298_b` | `MetadataChunkBlock::min_x` | Constructor's first coordinate (MetadataChunkBlock.java:14); `func_4127_a` loops x from it up to `field_1302_e` (:35) and sizes the box as `field_1302_e - field_1298_b + 1` (:23). Split from WP-0.5's grouped `bounds[0]` |
| `MetadataChunkBlock.field_1304_c` | `MetadataChunkBlock::min_y` | Second coordinate (:15); the y loop starts there (:65) after clamping it to `>= 0` (:57-59) |
| `MetadataChunkBlock.field_1303_d` | `MetadataChunkBlock::min_z` | Third coordinate (:16); the z loop starts there (:36) |
| `MetadataChunkBlock.field_1302_e` | `MetadataChunkBlock::max_x` | Fourth coordinate (:17); inclusive end of the x loop (:35); the `+x` neighbour is notified only at `x + 1 >= field_1302_e` (:135) |
| `MetadataChunkBlock.field_1301_f` | `MetadataChunkBlock::max_y` | Fifth coordinate (:18); clamped to `<= 127` (:61-63), inclusive end of the y loop (:65) |
| `MetadataChunkBlock.field_1300_g` | `MetadataChunkBlock::max_z` | Sixth coordinate (:19); inclusive end of the z loop (:36) |
| `MetadataChunkBlock.func_4127_a(World)` | `MetadataChunkBlock::update_light` | The light update itself: recomputes every block of the box from its neighbours, emission (`Block.lightValue`) or sky access, and opacity, writes it with `setLightValue` and queues neighbours (MetadataChunkBlock.java:22-153); its only caller is `World.updatingLighting` (World.java:1674) |
| `MetadataChunkBlock.func_866_a(int×6)` | `MetadataChunkBlock::try_merge` | Called by `scheduleLightingUpdate_do` on the five newest queued updates of the same type (World.java:1708-1713): true when the new box is inside this one, or when this one could be grown to include it at a cost of at most 2 in `(max-min)` volume, in which case it is grown in place (MetadataChunkBlock.java:155-206) |
| `BlockStep.field_22037_a` | `block::STEP_TYPE_NAMES` | Static `{"stone", "sand", "wood", "cobble"}` (BlockStep.java:6), indexed by slab metadata: `ItemSlab.getItemNameIS` returns `super.getItemName() + "." + field_22037_a[damage]` (ItemSlab.java:18-20), and the four entries match the four slab textures of `BlockStep.getBlockTextureFromSideAndMetadata` (BlockStep.java:19-33). Static, so not in the `block_table` dump |
| `BlockRedstoneOre.func_320_h` | `block::redstone_ore::glow` (private) | `sparkle(...)`, then the plain ore (id 73) becomes the glowing ore (id 74) by `setBlockWithNotify` (BlockRedstoneOre.java:36-42); the common body of `onBlockClicked`, `onEntityWalking` and `blockActivated` (:21-34), the three ways a player touches the ore |
| `BlockRedstoneRepeater.field_22023_b` | `block::REPEATER_DELAYS` | Static `{1, 2, 3, 4}` (BlockRedstoneRepeater.java:7), indexed by the delay setting `(meta & 12) >> 2`; the scheduled tick delay is twice the entry (BlockRedstoneRepeater.java:36-37, :93-97); the redstone task of WP-1.6 reads it |
| `NoiseGeneratorPerlin.func_4110_a` | `NoiseGeneratorPerlin::grad_2d` | `grad(hash, x, y, z)` with `y = 0`: same selector on `hash & 15`, only x and z inputs (NoiseGeneratorPerlin.java:76-81); the 2D branch of `func_805_a` calls it where the 3D branch calls `grad` (:141 vs :204) |
| `NoiseGeneratorPerlin.func_801_a` | `NoiseGeneratorPerlin::generate_noise_2d` | Returns `generateNoise(x, y, 0.0)` (NoiseGeneratorPerlin.java:90-92) |
| `NoiseGeneratorPerlin.func_805_a` | `NoiseGeneratorPerlin::populate_noise_array` | Adds `noise * (1 / amplitude)` for each grid sample into the passed array, index running x, z, y (NoiseGeneratorPerlin.java:94-220); `generateNoiseOctaves` calls it once per octave (NoiseGeneratorOctaves.java:43) |
| `NoiseGeneratorOctaves.field_1191_b` | `NoiseGeneratorOctaves::octaves` | The constructor's octave count, bound of every octave loop (NoiseGeneratorOctaves.java:10-14, :23, :42) |
| `NoiseGeneratorOctaves.func_806_a` | `NoiseGeneratorOctaves::noise_2d` | `sum(perlin_i.func_801_a(x*f, z*f) / f)`, `f` halving: a single 2D octave-noise sample (NoiseGeneratorOctaves.java:19-29); `populate` uses it as `mobSpawnerNoise` (ChunkProviderGenerate.java:411) |
| `NoiseGeneratorOctaves.func_4109_a` | `NoiseGeneratorOctaves::generate_noise_octaves_2d` | `generateNoiseOctaves(array, x, 10.0, z, xSize, 1, zSize, xScale, 1.0, zScale)`: one y layer; its last parameter is never read (NoiseGeneratorOctaves.java:50-52) |
| `NoiseGenerator2.field_4296_d` | `simplex::GRAD3` | The twelve `{±1, ±1, 0}`-style gradient vectors indexed by `perm % 12` (NoiseGenerator2.java:6, :78-80, :87) |
| `NoiseGenerator2.field_4295_e` | `NoiseGenerator2::permutations` | Built and shuffled exactly like `NoiseGeneratorPerlin.permutations` (NoiseGenerator2.java:19-34) |
| `NoiseGenerator2.field_4292_a`, `field_4291_b`, `field_4297_c` | `NoiseGenerator2::x_coord`, `y_coord`, `z_coord` | Drawn as `nextDouble() * 256` like Perlin's `xCoord`/`yCoord`/`zCoord` (NoiseGenerator2.java:20-22); the first two are added to the two input coordinates (:50, :53), the third is never read |
| `NoiseGenerator2.field_4294_f` | `simplex::skew_f2()` | `0.5 * (sqrt(3) - 1)`, the 2D simplex skew factor applied to `x + y` (NoiseGenerator2.java:11, :54) |
| `NoiseGenerator2.field_4293_g` | `simplex::unskew_g2()` | `(3 - sqrt(3)) / 6`, the 2D unskew factor (NoiseGenerator2.java:12, :57, :72-75) |
| `NoiseGenerator2.func_4156_a` | `NoiseGenerator2::dot2` | `g[0] * x + g[1] * y` (NoiseGenerator2.java:42-44) |
| `NoiseGenerator2.func_4157_a` | `NoiseGenerator2::add_noise` | Adds `70 * (n0 + n1 + n2) * amplitude` per grid sample into the passed array (NoiseGenerator2.java:46-113); called once per octave (NoiseGeneratorOctaves2.java:38) |
| `NoiseGeneratorOctaves2.field_4234_a` | `NoiseGeneratorOctaves2::generators` | The octave array, one `NoiseGenerator2` per octave (NoiseGeneratorOctaves2.java:9-16) |
| `NoiseGeneratorOctaves2.field_4233_b` | `NoiseGeneratorOctaves2::octaves` | The octave count (NoiseGeneratorOctaves2.java:10, :37) |
| `WorldChunkManager.field_4194_e` | `ChunkManagerKind::Noise::temperature_noise` | Its output becomes `temperature` (WorldChunkManager.java:32, :46, :78) |
| `WorldChunkManager.field_4193_f` | `ChunkManagerKind::Noise::humidity_noise` | Its output becomes `humidity` (WorldChunkManager.java:79) |
| `WorldChunkManagerHell.field_4201_e` | `ChunkManagerKind::Hell::biome` | Returned by `getBiomeGenAt` and filled into the biome array (WorldChunkManagerHell.java:20-22, :52) |
| `WorldChunkManagerHell.field_4200_f` | `ChunkManagerKind::Hell::temperature` | Returned by `getTemperature`, filled into `temperature` (WorldChunkManagerHell.java:24-26, :54); `WorldProviderHell` passes 1.0 |
| `WorldChunkManagerHell.field_4199_g` | `ChunkManagerKind::Hell::humidity` | Filled into `humidity` (WorldChunkManagerHell.java:53); `WorldProviderHell` passes 0.0 |
| `RegionFile.field_22217_e` | `RegionFile::chunk_timestamps` | `int[1024]` read from the second header sector (RegionFile.java:77-80); `func_22208_b` sets entry `x + z * 32` and the int at `4096 + (x + z * 32) * 4` (:265-269), called by `write` with `(int)(System.currentTimeMillis() / 1000L)` (:232) |
| `RegionFile.func_22209_a` | `RegionFile::take_size_delta` | Returns `sizeDelta` and resets it to 0 (RegionFile.java:87-91); `RegionFileCache.getSizeDelta` returns it (RegionFileCache.java:63-66) and `saveChunk` adds it to `sizeOnDisk` (McRegionChunkLoader.java:54) |
| `RegionFile.func_22202_c` | `RegionFile::has_chunk` | `getOffset(x, z) != 0` (RegionFile.java:255-257); `SaveConverterMcRegion` skips chunks for which it is true (SaveConverterMcRegion.java:131) |
| `RegionFile.func_22208_b` | `RegionFile::set_chunk_timestamp` (private) | Writes `field_22217_e[x + z * 32]` and its header int (RegionFile.java:265-269), the timestamp counterpart of `setOffset` (:259-263) |
| `RegionFile.func_22196_b` | `RegionFile::close` | `dataFile.close()` (RegionFile.java:271-273); called by `RegionFileCache.func_22192_a` on every cached file (RegionFileCache.java:53) |
| `RegionFileCache.func_22193_a` | `RegionFileCache::create_or_load_region_file` | Returns the cached `RegionFile` for `region/r.<x>.<z>.mcr` or constructs and caches a new one (RegionFileCache.java:19-42); every accessor goes through it (:64, :69, :74) |
| `RegionFileCache.func_22192_a` | `RegionFileCache::clear_region_file_references` | Closes every cached file and clears the map (RegionFileCache.java:44-61); called at 256 entries (:35-37) and around world conversion (SaveConverterMcRegion.java:50, :154) |
| `RegionFileChunkBuffer.field_22283_b`, `field_22285_c` | `RegionFileChunkBuffer::chunk_x`, `chunk_z` | The constructor's second and third arguments (RegionFileChunkBuffer.java:11-16), passed back as the chunk coordinates of `RegionFile.write(x, z, buf, count)` on `close` (:18-20); `getChunkDataOutputStream(x, z)` creates it (RegionFile.java:162) |
| `RegionFileChunkBuffer.field_22284_a` | the `region` argument of `RegionFileChunkBuffer::close` | The synthetic outer `RegionFile` (RegionFileChunkBuffer.java:9, :13), only used to call `write` on close (:19); a Rust value cannot hold it, so `close` takes it |

## Kept (meaning documented)

| Java | Rust | Meaning, from usage |
|---|---|---|
| `World.field_9437_g` | `World::field_9437_g` | Random-tick LCG state: `g = g * 3 + 1013904223`, then `g >> 2` picks x/z/y (World.java:1903, :1921, :1934, :1953); seeded from an unseeded `new Random().nextInt()` (:75). Drives cave sounds, lightning, snow and the 80 random ticks per chunk, so "random tick seed" would undersell it; the traces call it `lcg` (tests/golden/README.md `rng_state`) |
| `World.field_9436_h` | `World::field_9436_h` | `final int = 1013904223`, the LCG increment; never read (the update sites use the literal) (World.java:26, :76) |
| `World.field_1019_F` | `World::field_1019_f` | `16777215L`, the cloud colour read by `drawClouds` (World.java:1088-1090); client only |
| `World.field_27168_F` | `World::field_27168_f` | Set to 2 when lightning strikes (World.java:1928), counted down in `updateWeather` (:1794-1796) |
| `World.field_27172_i` | `World::field_27172_i` | Sky flash timer: set to 2 by `EntityLightningBolt` (EntityLightningBolt.java:71), read by `getSkyColor` (World.java:1058), decremented by `Minecraft.runTick` (Minecraft.java:1155-1157) |
| `World.field_30900_E` | `World::field_30900_e` | Tile entities added while the tile-entity pass runs; merged after it (World.java:1254-1274, :1278-1285, :1604-1611) |
| `World.field_31055_L` | `World::field_31055_l` | True while `updateEntities` iterates `loadedTileEntityList` (World.java:1235, :1253) |
| `World.func_27163_E` | `World::func_27163_e` | Starts rain/thunder strength at 1 if `WorldInfo` says it is raining (World.java:1782-1790) |
| `World.func_698_b` | `World::func_698_b` | A tile entity's inventory changed: marks the chunk modified and notifies listeners (World.java:2053-2062; called from `TileEntity.onInventoryChanged`, TileEntity.java:70-74) |
| `World.func_6466_a(EntityPlayer, int, int, int)` | `World::func_6466_a` | Whether the player may mine the block at x, y, z (later versions' `canMineBlock`): always `true` in `World` (World.java:2311-2313); its only caller, `ItemBucket.onItemRightClick`, returns early when it is false, before touching the targeted block (ItemBucket.java:36). Kept: one caller that only gates an interaction does not pin the meaning down |
| `World.func_687_d` | `World::func_687_d` | Debug-screen string `"All: " + loadedEntityList.size()` (World.java:1591-1593) |
| `World.func_21119_g` | `World::func_21119_g` | Debug-screen string: the chunk provider's `makeString()`, `"ServerChunkCache: <chunks> Drop: <dropped>"` (World.java:1595-1597, ChunkProvider.java:185-187) |
| `World.func_656_j` | `World::func_656_j` | `while (chunkProvider.unload100OldestChunks());`: unloads until the provider has nothing to drop (World.java:2090-2094) |
| `World.func_4085_a(Class)` | — (not ported) | Returns `null` (World.java:1587-1589); no caller in the client |
| `World.field_28108_z` | — (not ported yet) | `MapStorage` for map items (World.java:46); WP-2.6 |
| `BlockStep.field_22037_a` | `item::BLOCK_STEP_FIELD_22037_A` | `static final String[] {"stone", "sand", "wood", "cobble"}`: the slab type names by metadata, read only by `ItemSlab.getItemNameIS` as `"tile.stoneSlab." + field_22037_a[damage]` (BlockStep.java:6, ItemSlab.java:18-20). Kept because the block module may want it under its own name (WP-1.1); the item module needs it for item names |
| `World.field_1012_M` | — (local `Vec`) | Scratch list returned by `getEntitiesWithinAABBExcludingEntity` (World.java:55, :2013-2029) |
| `Entity.field_9293_aM` | `EntityBase::field_9293_a_m` | When false, a collision on any axis zeroes the whole move (Entity.java:348-377, :398-428); initialised true (:83) and never cleared in vanilla |
| `EntityItem.field_803_e` | `ItemData::field_803_e` | Incremented every tick (EntityItem.java:71), never read |
| `EntityLiving.field_9365_p`, `field_9363_r`, `field_9362_u`, `field_9361_v`, `field_9360_w`, `field_9359_x`, `field_9358_y`, `field_9355_A`, `field_9349_D`, `field_9345_F`, `field_9326_T`, `field_9325_U`, `field_9348_ae`, `field_9346_af` | `LivingData::` same, snake_case | Fields only in WP-0.5 (EntityLiving.java:7-49). `field_9325_U` and `field_9363_r`, `field_9365_p` draw `Math.random()` at construction (:38, :63, :65); `field_9362_u`/`field_9361_v` are reset by `updateRidden` (:187-191). WP-2.2 names them from their uses |
| `BlockRedstoneOre.field_468_a` | `BlockKind::RedstoneOre { field_468_a }` | True for the glowing ore (id 74): the constructor enables random ticks only then (BlockRedstoneOre.java:8-15); pinned per block by `block/block-table` (`fields.field_468_a`) |
| `TileEntity.field_31007_h` | `TileEntity::field_31007_h` | The "invalid" flag: `isInvalid()` returns it, `invalidate()`/`validate()` set it (TileEntity.java:88-98) |
| `MetadataChunkBlock.field_1299_a` | `MetadataChunkBlock::field_1299_a` | Which light (`EnumSkyBlock`) the update is for (MetadataChunkBlock.java:4, :13) |
| `Chunk.func_1014_a` | — (not ported) | Empty in `Chunk` and `EmptyChunk` (Chunk.java:64-65, EmptyChunk.java:25-26); called after a chunk is read from disk (ChunkLoader.java:173), so its call site needs nothing |
| `Chunk.func_4143_d` | — (not ported) | Empty in `Chunk` and `EmptyChunk` (Chunk.java:133-134, EmptyChunk.java:34-35); called right after a chunk enters the cache (ChunkProvider.java:48, ChunkProviderLoadOrGenerate.java:71) |
| `NoiseGeneratorOctaves2.func_4112_a` | `NoiseGeneratorOctaves2::func_4112_a` | `func_4111_a(..., frequencyStep, 0.5)` (NoiseGeneratorOctaves2.java:19-21); the biome manager's only entry point |
| `NoiseGeneratorOctaves2.func_4111_a` | `NoiseGeneratorOctaves2::func_4111_a` | Octave sum of `NoiseGenerator2`: scales divided by 1.5, octave i at `scale * frequencyStep^i` with amplitude `0.55 / amplitudeStep^i`; reuses the array (zeroing all of it) when long enough (NoiseGeneratorOctaves2.java:23-44) |
| `WorldChunkManager.field_4192_g` | `ChunkManagerKind::Noise::field_4192_g` | 2-octave noise (seed × 543321) whose output, `field_4196_c`, is blended into temperature (weight 0.01) and humidity (0.002) (WorldChunkManager.java:20, :47, :80-91) |
| `WorldChunkManager.field_4196_c` | `WorldChunkManager::field_4196_c` | The last `field_4192_g` array (WorldChunkManager.java:11, :47, :80); read only as `value * 1.1 + 0.5` in the blend |
| `ChunkProviderGenerate.field_912_k`, `field_911_l`, `field_910_m` | `ChunkProviderGenerate::` same | 16-, 16- and 8-octave 3D noises: two density fields (`field_4184_e`, `field_4183_f`) and their blend factor (`field_4185_d`) (ChunkProviderGenerate.java:33-35, :217-219, :280-289) |
| `ChunkProviderGenerate.field_909_n`, `field_908_o` | `ChunkProviderGenerate::` same | 4-octave noises: sand/gravel beaches and the surface-layer depth (ChunkProviderGenerate.java:36-37, :116-118) |
| `ChunkProviderGenerate.field_922_a`, `field_921_b` | `ChunkProviderGenerate::` same | 10- and 16-octave 2D noises: density falloff scale (`field_4182_g`) and height offset (`field_4181_h`) (ChunkProviderGenerate.java:38-39, :215-216, :235-262) |
| `ChunkProviderGenerate.field_4180_q` | `ChunkProviderGenerate::field_4180_q` | The 5×17×5 density grid `generateTerrain` interpolates (ChunkProviderGenerate.java:49-62) |
| `ChunkProviderGenerate.field_4185_d`, `field_4184_e`, `field_4183_f`, `field_4182_g`, `field_4181_h` | same, snake_case | Scratch arrays of the five noises above (ChunkProviderGenerate.java:215-219) |
| `ChunkProviderGenerate.field_902_u` | `ChunkProviderGenerate::field_902_u` | The `MapGenCaves` instance (ChunkProviderGenerate.java:20, :201) |
| `ChunkProviderGenerate.field_914_i` | — (not ported) | `int[32][32]`, never read or written after construction (ChunkProviderGenerate.java:27) |
| `ChunkProviderGenerate.func_4061_a` | `ChunkProviderGenerate::func_4061_a` | Fills the density grid from the noises and the biome temperature × humidity (ChunkProviderGenerate.java:206-304) |
| `ChunkProviderHell.field_4169_i`, `field_4168_j`, `field_4167_k` | `ChunkProviderHell::` same | As `field_912_k`/`field_911_l`/`field_910_m` for the nether (ChunkProviderHell.java:29-31, :199-201) |
| `ChunkProviderHell.field_4166_l`, `field_4165_m` | `ChunkProviderHell::` same | Soul-sand/gravel noise and surface-layer depth noise (ChunkProviderHell.java:32-33, :106-108) |
| `ChunkProviderHell.field_4177_a`, `field_4176_b` | `ChunkProviderHell::` same | 2D noises whose values the nether density computes but never uses (ChunkProviderHell.java:34-35, :197-198, :222-252) |
| `ChunkProviderHell.field_4163_o`, `field_4162_p`, `field_4160_r`, `field_4175_c`, `field_4174_d`, `field_4173_e`, `field_4172_f`, `field_4171_g` | same, snake_case | Scratch arrays: density grid, soul-sand noise, depth noise, and the five density noises (ChunkProviderHell.java:15-24) |
| `ChunkProviderHell.func_4058_b` | `ChunkProviderHell::func_4058_b` | The nether's `replaceBlocksForBiome`: bedrock top and bottom, soul sand, gravel (ChunkProviderHell.java:103-174) |
| `ChunkProviderHell.func_4057_a` | `ChunkProviderHell::func_4057_a` | The nether's density grid (ChunkProviderHell.java:190-296) |
| `RegionFile.field_22214_h` | not ported | `fileName.lastModified()` at construction (RegionFile.java:22, :30-32), never read; file metadata and clocks stay out of the simulation (D9) |
| `RegionFile.func_22211_a`, `func_22199_a`, `func_22197_a`, `debugln` | not ported | `func_22211_a` has an empty body (RegionFile.java:93-94); the other three only build messages for it (:96-110). The write path's message names ("rewrite", "reuse", "grow") are returned as `RegionWrite` |
| `WorldGenBigTree.field_882_a` | `worldgen::trees::FIELD_882_A` (private) | Static `{2, 0, 0, 1, 2, 1}`: for a main axis `a`, the two other axes are `[a]` and `[a + 3]` (WorldGenBigTree.java:6, :92-93, :180-181, :273-274) |
| `WorldGenBigTree.field_881_b` | `WorldGenBigTree::field_881_b` | The tree's own `Random`, reseeded with `rand.nextLong()` at every `generate` (WorldGenBigTree.java:7, :333-334); draws the height limit and the branch directions |
| `WorldGenBigTree.field_878_e` | `WorldGenBigTree::field_878_e` | Height limit: `5 + nextInt(field_870_m)` when 0 (:338-340), lowered to the first obstruction by `func_519_e` (:315); every size derives from it |
| `WorldGenBigTree.field_876_g` | `WorldGenBigTree::field_876_g` | 0.618, trunk height = `(int)(field_878_e * field_876_g)` (:12, :23) |
| `WorldGenBigTree.field_875_h` | `WorldGenBigTree::field_875_h` | 1.0, never read (:13) |
| `WorldGenBigTree.field_874_i` | `WorldGenBigTree::field_874_i` | 0.381, how far below a node its branch meets the trunk, per block of horizontal distance (:14, :62) |
| `WorldGenBigTree.field_873_j` | `WorldGenBigTree::field_873_j` | Branch length scale (1.0; `func_517_a`'s second argument) (:15, :53, :327) |
| `WorldGenBigTree.field_872_k` | `WorldGenBigTree::field_872_k` | Leaf-node count scale (1.0; `func_517_a`'s third argument): nodes per layer `(int)(1.382 + (k * e / 13)^2)` (:16, :28, :328) |
| `WorldGenBigTree.field_871_l` | `WorldGenBigTree::field_871_l` | Trunk width, 1; 2 makes `func_529_c` draw a 2×2 trunk (:17, :228-238); never set to 2 in this tree |
| `WorldGenBigTree.field_870_m` | `WorldGenBigTree::field_870_m` | Bound of the height-limit draw, 12, or `(int)(a * 12)` from `func_517_a` (:18, :322, :339) |
| `WorldGenBigTree.field_869_n` | `WorldGenBigTree::field_869_n` | Height of a leaf cluster, 4 (5 when `func_517_a`'s first argument is above 0.5) (:19, :150-151, :160, :323-325) |
| `WorldGenBigTree.field_868_o` | `WorldGenBigTree::field_868_o` (`Vec<[i32; 4]>`) | Leaf nodes `[x, y, z, branch base y]` built by `func_521_a` (:20, :70-73, :84-85) |
| `WorldGenBigTree.func_521_a` | `WorldGenBigTree::func_521_a` | Builds the leaf nodes layer by layer from the top (:22-88) |
| `WorldGenBigTree.func_523_a` | `WorldGenBigTree::func_523_a` | Fills a disc of a block id perpendicular to an axis, replacing only air and leaves (:90-127) |
| `WorldGenBigTree.func_528_a` | `WorldGenBigTree::func_528_a` | Branch spread of a layer: negative below 30 % of the height limit, else half a circle chord (:129-147) |
| `WorldGenBigTree.func_526_b` | `WorldGenBigTree::func_526_b` | Leaf-cluster disc radius by layer: 2, 3, …, 3, 2, -1 outside (:149-155) |
| `WorldGenBigTree.func_520_a` | `WorldGenBigTree::func_520_a` | One leaf cluster at a node (:157-165) |
| `WorldGenBigTree.func_522_a` | `WorldGenBigTree::func_522_a` | Places a line of a block id between two points (trunk, branches) (:167-202) |
| `WorldGenBigTree.func_518_b` | `WorldGenBigTree::func_518_b` | All leaf clusters (:204-214) |
| `WorldGenBigTree.func_527_c` | `WorldGenBigTree::func_527_c` | Whether a branch base is high enough for a branch (20 % of the height limit) (:216-218) |
| `WorldGenBigTree.func_529_c` | `WorldGenBigTree::func_529_c` | The trunk (:220-240) |
| `WorldGenBigTree.func_525_d` | `WorldGenBigTree::func_525_d` | The branches (:242-256) |
| `WorldGenBigTree.func_524_a` | `WorldGenBigTree::func_524_a` | Walks a line: -1 if it is clear (air and leaves), else the distance to the first obstruction (:258-300) |
| `WorldGenBigTree.func_519_e` | `WorldGenBigTree::func_519_e` | Site check: grass or dirt below, at least 6 clear blocks up (:302-319) |
| `WorldGenerator.func_517_a`, `WorldGenBigTree.func_517_a` | `WorldGen::func_517_a`, `WorldGenBigTree::func_517_a` | Scale hook, empty except in the big tree (height-limit bound, branch length, node count); population calls it with `(1, 1, 1)` (WorldGenerator.java:8-9, WorldGenBigTree.java:321-329, ChunkProviderGenerate.java:450) |

## Real names changed for Rust

Not obfuscated, listed because a reader searching for the Java name would miss them.

| Java | Rust | Why |
|---|---|---|
| `World.TickUpdates` | `World::tick_updates` | snake_case |
| `WorldGenerator` (abstract class) and its tree subclasses | `worldgen::trees::WorldGen` (enum; `WorldGenBigTree` as a struct in its variant) | a closed enum like `BlockKind`; `generate` takes a `RandomSource` (`block::dispatch` convention) and the world as a parameter instead of `WorldGenBigTree.worldObj` |
| `BlockFlower.canThisPlantGrowOnThisBlockID` and its overrides (`BlockCrops`, `BlockDeadBush`, `BlockMushroom`) | `block::flower::can_this_plant_grow_on_this_block_id(kind, id)` | one function matching on the kind |
| `ColorizerGrass.getGrassColor`, `ColorizerFoliage.getFoliageColor` | `block::grass::colormap_index` (+ a lookup that has no colormap) | the colormaps are client textures (D8); the index formula is the shared part |
| `World.entityJoinedWorld` | `World::spawn_entity_in_world` | the b1.7.3 name of later versions' `spawnEntityInWorld`; the doc comment keeps the Java name |
| `World.worldAccesses` | `World::events` | the listener list becomes an event queue (PLAN_v4 §3.2) |
| `Entity.mountEntity(null)` | `entity::base::dismount` | only the null case is ported so far |
| `Entity.nextEntityID` (static) | `World::next_entity_id` | per-world, reset per scenario (PLAN_v4 §4.2) |
| `NextTickListEntry.nextTickEntryID` (static) | `World::next_tick_entry_id` | as above |
| `World.lightingUpdatesScheduled` (static) | `World::lighting_updates_scheduled` | as above |
| `SpawnerAnimals.eligibleChunksForSpawning` (static) | `World::eligible_chunks_for_spawning` | per-world, like the other statics; same `HashSet` order (spawner module docs) |
| `World.randomDisplayUpdates`' local `new Random()` | `World::display_random` | one world-owned stream instead of a clock-seeded `Random` per call (D9); particles only |
| `Class` arguments of `getEntitiesWithinAABB` / `countEntities` | `&str` Java class or interface name, `world::is_instance_of` | no `Class` objects; the kind's class chain answers `isAssignableFrom` |
| `Block.blocksList` | `block::BLOCKS` / `block::block(id)` | immutable table; run-time-mutable block state is `World::block_statics` |
| `Item.itemsList` | `item::ITEMS` (ids 0..360), `item::RECORDS` (2256, 2257), `item::item(id)` | immutable table built at compile time (WP-1.2); the 32000-slot array is not materialised |
| `Item` subclass instance fields (`ItemTool.toolMaterial`, `ItemFood.healAmount`, `ItemBlock.blockID`, …) | `item::ItemKind` variant fields (`ToolData`, `FoodData`, `block_id`, …) | one variant per instantiated class, as `BlockKind` |
| `Item.itemName` | `ItemDef::item_name` (the `setItemName` argument) | stored without the `"item."` prefix, like `BlockDef::block_name`; `get_item_name()` adds it |
| `Item.containerItem` | `ItemDef::container_item: Option<i32>` | the container's `shiftedIndex` instead of the `Item` reference |
| `Item.getStrVsBlock(stack, block)`, `Item.getDamageVsEntity(entity)` | `ItemDef::get_str_vs_block(block)`, `ItemDef::get_damage_vs_entity()` | the unused stack/entity parameters are dropped (no override reads them) |
| `ItemStack(NBTTagCompound)`, `ItemStack.writeToNBT(c)` | `ItemStack::from_nbt`, `ItemStack::write_to_nbt_into(map)` / `write_to_nbt()` | Rust NBT is a value tree (`mc_nbt::Tag`); `ClassCastException` is `JavaException::ClassCast` |
| `ItemStack.damageItem(n, entity)` | `ItemStack::damage_item(n) -> bool` | returns "broke"; the caller adds `StatList.objectBreakStats` for players (ItemStack.java:128-130) |
| `ItemStack.toString()` | `ItemStack::java_to_string()` | not `Display`, to keep the Java format explicit |
| `Block.getBlockTextureFromSide`, `Block.getBlockTextureFromSideAndMetadata` | `item::block_texture::get_block_texture_from_side(id, side)`, `…_and_metadata(id, side, meta)` | free functions over the block id, in the item module until the block module has them (WP-1.2 report) |
| `EntityPlayerSP` (class) | `EntityKindData::PlayerSp` | Rust casing |
| `Block.<instance field>` of a mutable singleton (`BlockFlowing.numAdjacentSources`/`isOptimalFlowDirection`/`flowCost`, `BlockLeaves.adjacentTreeBlocks`, `BlockPistonBase.ignoreUpdates`, `BlockPistonExtension.headTexture`, `BlockRedstoneWire.wiresProvidePower`/`blocksNeedingUpdate`) | `BlockStatics::<singleton>_<field>` (`water_moving`/`lava_moving: FlowingScratch`, `leaves_adjacent_tree_blocks`, `piston_sticky_base_ignore_updates`, `piston_base_ignore_updates`, `piston_extension_head_texture`, `redstone_wire_wires_provide_power`, `redstone_wire_blocks_needing_update`) | run-time state of a singleton is world state (`block` module docs); the prefix is the `Block` static that holds the singleton |
| `BlockRedstoneTorch.torchUpdates` (static `List` of `RedstoneUpdateInfo`) | `BlockStatics::redstone_torch_updates: Vec<redstone_torch::RedstoneUpdateInfo>` | a Java static is per world here, like the other statics; the oracle clears it per scenario (census §6) |
| `ModelBed.footInvisibleFaceRemap` (read by `BlockRedstoneWire.isPowerProviderOrWire`, BlockRedstoneWire.java:378) | `block::redstone_wire::FOOT_INVISIBLE_FACE_REMAP` (private copy) | a client model class; the simulation keeps its own copy of the four ints (ModelBed.java:5) |
| `World.getEntitiesWithinAABBExcludingEntity(null, box)` as `BlockPressurePlate` calls it | `block::pressure_plate::get_entities_within_aabb_excluding_entity` (private, with `in_hand`) | `World::get_entities_within_aabb_excluding_entity` has no `in_hand` parameter; the plate is called from inside the colliding entity's `moveEntity`, so the held entity must be visible to the scan |
| `BlockFlowing`, `BlockStationary` (classes) | `block::fluid::flowing`, `block::fluid::stationary` (inline modules of `block/fluid.rs`, whose top level is `BlockFluid`) | the three fluid classes share helpers (`getFlowDecay`, `triggerLavaMixEffects`); one file, no new `mod` lines in `block/mod.rs` |
| `BlockFlowing.getOptimalFlowDirections()` returning `this.isOptimalFlowDirection` | `fluid::flowing::get_optimal_flow_directions` (private, returns nothing); `updateTick` reads `BlockStatics::{water,lava}_moving.is_optimal_flow_direction[k]` before each `flowIntoBlock` | Java returns the singleton's own array, which a re-entrant `updateTick` (immediate scheduled updates during population, WorldGenLiquids.java:56-58) overwrites while the caller iterates it (BlockFlowing.java:87-111) |
| `BlockFluid.getFlowDirection(blockAccess, x, y, z, material)` (static) | `block::fluid::get_flow_direction(world, x, y, z, material) -> Option<f64>` | `None` where Java throws a `NullPointerException` (a material other than water or lava, BlockFluid.java:227-236) |
| `super.shouldSideBeRendered` in `BlockFluid.shouldSideBeRendered` | a `base: fn(&mut World, id, x, y, z, side) -> bool` argument, passed by the dispatch | the base body is private to `block::dispatch` |
| `BlockFire.chanceToEncourageFire` / `abilityToCatchFire` (instance fields of `Block.fire`) | `block::CHANCE_TO_ENCOURAGE_FIRE` / `block::ABILITY_TO_CATCH_FIRE` | written once by `initializeBlock` (BlockFire.java:14-24), so immutable data |
| `BlockLeaves.baseIndexInPNG`, `BlockBreakable.localFlag`, `BlockRail.isPowered` (detector rail) | `BlockDef::base_index_in_png`, `BlockKind::breakable_local_flag`, `BlockKind::rail_is_powered` | constructor arguments that are constant per kind, derived instead of stored |
| `Block.blockConstructorCalled` | `BlockDef::block_constructor_called` | snake_case; always true, never read |
| `PistonBlockTextures.faceToSide`, `ModelBed.bedDirection` | `block::FACE_TO_SIDE`, `block::BED_DIRECTION` (private) | the two tables the block texture functions read; the rest of `PistonBlockTextures` is the pistons task's |
| `World.getWorldChunkManager()` / `WorldProvider.worldChunkMgr` | `World::get_world_chunk_manager` / `WorldProvider::world_chunk_mgr` (`Option`) | built on first use for the surface, because `WorldProvider::new` runs before the world seed is known (`world/provider.rs` module docs) |
| `WorldProvider.getChunkProvider()` | `WorldProvider::get_chunk_provider(seed)` | the world is not built yet when the generator is; the seed is passed |
| `BiomeGenBase.rainforest`, … (static singletons) | `worldgen::biome::Biome` + `BIOMES` | identity enum + post-static-init data rows; `BiomeGenSky`/`sky` not ported (D3) |
| `BiomeGenBase.biomeLookupTable` | `worldgen::biome::BIOME_LOOKUP_TABLE` | const-evaluated with the static initialiser's loop |
| `WorldChunkManagerHell` (class) | `ChunkManagerKind::Hell` | one struct for both classes |
| `MapGenBase`, `MapGenCaves`, `MapGenCavesHell` (classes) | `worldgen::caves::MapGenCaves` + `CaveKind` | one struct; per-class constants selected by kind |
| `ChunkProviderHell.hellRNG` | `ChunkProviderHell::hell_rng` | snake_case |
| `RegionFile.getChunkDataInputStream(x, z)` | `RegionFile::get_chunk_data_input_stream(x, z) -> Result<Option<Vec<u8>>, _>` | returns the decompressed bytes instead of a lazy `DataInputStream` (`save/region.rs`, "Errors") |
| `RegionFile.getChunkDataOutputStream(x, z)` | `RegionFile::get_chunk_data_output_stream(x, z) -> Option<ChunkDataOutputStream>` | the `DataOutputStream(DeflaterOutputStream(RegionFileChunkBuffer))` chain is one `io::Write` value; its `close` takes the region, storage and time |
| `RegionFile.write(x, z, buf, count)` (protected) | `RegionFile::write(storage, x, z, data, now_millis) -> Result<RegionWrite, _>` | public for tests and the cache; storage and `System.currentTimeMillis()` passed in (D9) |
| `RegionFileCache` (static class) | `save::RegionFileCache` (a value) | owned by the save handler; no soft references (`save/region_cache.rs` docs) |
| `RegionFileCache.getChunkInputStream`/`getChunkOutputStream`/`getSizeDelta` | `RegionFileCache::get_chunk_input_stream`/`get_chunk_output_stream`/`get_size_delta` (+ `close_chunk_output_stream`, `write_chunk`) | snake_case; storage passed in, world directory a storage path |
