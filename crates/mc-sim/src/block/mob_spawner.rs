//! `BlockMobSpawner` (BlockMobSpawner.java). Its overrides are data
//! (`idDropped` 0, `quantityDropped` 0, `isOpaqueCube` false, in
//! [`crate::block`]) plus `getBlockEntity` ([`super::container::get_block_entity`]:
//! a `TileEntityMobSpawner`); `onBlockAdded`/`onBlockRemoval` are
//! `BlockContainer`'s. The spawning is the tile entity's
//! ([`crate::tile_entity::mob_spawner`]).
