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
| `BlockRedstoneRepeater.field_22023_b` | `block::REPEATER_DELAYS` | Static `{1, 2, 3, 4}` (BlockRedstoneRepeater.java:7), indexed by the delay setting `(meta & 12) >> 2`; the scheduled tick delay is twice the entry (BlockRedstoneRepeater.java:36-37, :93-97); the redstone task of WP-1.6 reads it |

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

## Real names changed for Rust

Not obfuscated, listed because a reader searching for the Java name would miss them.

| Java | Rust | Why |
|---|---|---|
| `World.TickUpdates` | `World::tick_updates` | snake_case |
| `World.entityJoinedWorld` | `World::spawn_entity_in_world` | the b1.7.3 name of later versions' `spawnEntityInWorld`; the doc comment keeps the Java name |
| `World.worldAccesses` | `World::events` | the listener list becomes an event queue (PLAN_v4 §3.2) |
| `Entity.mountEntity(null)` | `entity::base::dismount` | only the null case is ported so far |
| `Entity.nextEntityID` (static) | `World::next_entity_id` | per-world, reset per scenario (PLAN_v4 §4.2) |
| `NextTickListEntry.nextTickEntryID` (static) | `World::next_tick_entry_id` | as above |
| `World.lightingUpdatesScheduled` (static) | `World::lighting_updates_scheduled` | as above |
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
| `BlockFire.chanceToEncourageFire` / `abilityToCatchFire` (instance fields of `Block.fire`) | `block::CHANCE_TO_ENCOURAGE_FIRE` / `block::ABILITY_TO_CATCH_FIRE` | written once by `initializeBlock` (BlockFire.java:14-24), so immutable data |
| `BlockLeaves.baseIndexInPNG`, `BlockBreakable.localFlag`, `BlockRail.isPowered` (detector rail) | `BlockDef::base_index_in_png`, `BlockKind::breakable_local_flag`, `BlockKind::rail_is_powered` | constructor arguments that are constant per kind, derived instead of stored |
| `Block.blockConstructorCalled` | `BlockDef::block_constructor_called` | snake_case; always true, never read |
| `PistonBlockTextures.faceToSide`, `ModelBed.bedDirection` | `block::FACE_TO_SIDE`, `block::BED_DIRECTION` (private) | the two tables the block texture functions read; the rest of `PistonBlockTextures` is the pistons task's |
