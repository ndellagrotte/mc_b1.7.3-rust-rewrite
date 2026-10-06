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
| `MetadataChunkBlock.field_1298_b`, `field_1304_c`, `field_1303_d`, `field_1302_e`, `field_1301_f`, `field_1300_g` | `MetadataChunkBlock::bounds[0..6]` | Assigned from the constructor's x0, y0, z0, x1, y1, z1 in that order (MetadataChunkBlock.java:12-20); `func_4127_a` uses them as box min/max (`field_1302_e - field_1298_b + 1`, :22-25). Grouped into one array; WP-1.3 may split them when it transcribes `func_4127_a` |

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
| `World.field_1012_M` | — (local `Vec`) | Scratch list returned by `getEntitiesWithinAABBExcludingEntity` (World.java:55, :2013-2029) |
| `Entity.field_9293_aM` | `EntityBase::field_9293_a_m` | When false, a collision on any axis zeroes the whole move (Entity.java:348-377, :398-428); initialised true (:83) and never cleared in vanilla |
| `EntityItem.field_803_e` | `ItemData::field_803_e` | Incremented every tick (EntityItem.java:71), never read |
| `EntityLiving.field_9365_p`, `field_9363_r`, `field_9362_u`, `field_9361_v`, `field_9360_w`, `field_9359_x`, `field_9358_y`, `field_9355_A`, `field_9349_D`, `field_9345_F`, `field_9326_T`, `field_9325_U`, `field_9348_ae`, `field_9346_af` | `LivingData::` same, snake_case | Fields only in WP-0.5 (EntityLiving.java:7-49). `field_9325_U` and `field_9363_r`, `field_9365_p` draw `Math.random()` at construction (:38, :63, :65); `field_9362_u`/`field_9361_v` are reset by `updateRidden` (:187-191). WP-2.2 names them from their uses |
| `BlockRedstoneOre.field_468_a` | `BlockKind::RedstoneOre { field_468_a }` | True for the glowing ore (id 74): the constructor enables random ticks only then (BlockRedstoneOre.java:8-15); pinned per block by `block/block-table` (`fields.field_468_a`) |
| `BlockStep.field_22037_a` | — (not ported yet) | Static `{"stone", "sand", "wood", "cobble"}`, the slab type names by metadata (BlockStep.java:6); read by `ItemSlab` for item names (WP-1.2). Static, so not in the `block_table` dump |
| `BlockRedstoneRepeater.field_22023_b` | — (not ported yet) | Static `{1, 2, 3, 4}`: repeater delay setting by `(meta & 12) >> 2`; the scheduled tick delay is twice the entry (BlockRedstoneRepeater.java:7, :36-37, :93-97); redstone task of WP-1.6 |
| `TileEntity.field_31007_h` | `TileEntity::field_31007_h` | The "invalid" flag: `isInvalid()` returns it, `invalidate()`/`validate()` set it (TileEntity.java:88-98) |
| `MetadataChunkBlock.field_1299_a` | `MetadataChunkBlock::field_1299_a` | Which light (`EnumSkyBlock`) the update is for (MetadataChunkBlock.java:4, :13) |

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
| `EntityPlayerSP` (class) | `EntityKindData::PlayerSp` | Rust casing |
| `Block.<instance field>` of a mutable singleton (`BlockFlowing.numAdjacentSources`/`isOptimalFlowDirection`/`flowCost`, `BlockLeaves.adjacentTreeBlocks`, `BlockPistonBase.ignoreUpdates`, `BlockPistonExtension.headTexture`, `BlockRedstoneWire.wiresProvidePower`/`blocksNeedingUpdate`) | `BlockStatics::<singleton>_<field>` (`water_moving`/`lava_moving: FlowingScratch`, `leaves_adjacent_tree_blocks`, `piston_sticky_base_ignore_updates`, `piston_base_ignore_updates`, `piston_extension_head_texture`, `redstone_wire_wires_provide_power`, `redstone_wire_blocks_needing_update`) | run-time state of a singleton is world state (`block` module docs); the prefix is the `Block` static that holds the singleton |
| `BlockFire.chanceToEncourageFire` / `abilityToCatchFire` (instance fields of `Block.fire`) | `block::CHANCE_TO_ENCOURAGE_FIRE` / `block::ABILITY_TO_CATCH_FIRE` | written once by `initializeBlock` (BlockFire.java:14-24), so immutable data |
| `BlockLeaves.baseIndexInPNG`, `BlockBreakable.localFlag`, `BlockRail.isPowered` (detector rail) | `BlockDef::base_index_in_png`, `BlockKind::breakable_local_flag`, `BlockKind::rail_is_powered` | constructor arguments that are constant per kind, derived instead of stored |
| `Block.blockConstructorCalled` | `BlockDef::block_constructor_called` | snake_case; always true, never read |
| `PistonBlockTextures.faceToSide`, `ModelBed.bedDirection` | `block::FACE_TO_SIDE`, `block::BED_DIRECTION` (private) | the two tables the block texture functions read; the rest of `PistonBlockTextures` is the pistons task's |
