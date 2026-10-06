//! The vanilla Beta 1.7.3 singleplayer simulation as plain Rust (PLAN_v4 D1).
//!
//! Everything here is transcribed from
//! `.reference/mc_b1.7.3_release/1.7.3-LTS/src/minecraft` (D2) with
//! `// src: File.java:line` citations. Bevy never appears in this crate; the
//! client drives it from one system and reads its state back.
//!
//! # Map (PLAN_v4 §3.1-3.2)
//!
//! | Module | Vanilla | Shape |
//! |---|---|---|
//! | [`block`] | `Block`, `Material`, `StepSound` + subclasses | const table [`block::BLOCKS`], `BlockKind` enum, behaviour in [`block::dispatch`] |
//! | [`item`] | `Item` + subclasses, `EnumToolMaterial`, `ItemStack` | item table done (WP-1.2): const table [`item::ITEMS`] / [`item::RECORDS`] of `ItemDef` + `ItemKind` enum, data methods as `impl ItemDef`; `ItemStack` value type with NBT; item behaviour is WP-2.6; block texture lookups live in `block::block_texture` |
//! | [`world`] | `World`, `Chunk`, `NibbleArray`, `ChunkProvider`, `WorldInfo`, `WorldProvider`, `TileEntity`, `NextTickListEntry`, `IWorldAccess`, `MovingObjectPosition` | lighting done (WP-1.3); [`world::World`] owns everything; client side effects are [`world::events`] |
//! | [`tick`] | `World.tick` and its block systems, `World.randomDisplayUpdates` | `impl World` |
//! | [`tile_entity`] | `TileEntity*` subclasses, `FurnaceRecipes`, `IInventory` | data in `world::tile_entity::TileEntityKindData`, behaviour and NBT as free functions over a `TileEntityRef` (WP-1.8) |
//! | [`spawner`] | `SpawnerAnimals`, `EnumCreatureType` | free functions over `&mut World` |
//! | [`entity`] | `Entity` + subclasses | `Entity { base, kind }`, per-class modules, [`entity::dispatch`], take/put-back driver |
//! | [`inventory`] | `InventoryPlayer` | layout only (WP-2.5) |
//! | [`save`] | `ISaveHandler`, `IChunkLoader`, [`save::Storage`] (D9) | McRegion saves: `SaveOldDir`/`SaveHandler`, `McRegionChunkLoader`, chunk NBT, `EntityList`, import validation (WP-1.9a/b) |
//! | [`worldgen`] | `ChunkProviderGenerate`, noise, biomes | noise, biomes and terrain shape (`provideChunk`) done (WP-1.4), trees (WP-1.6b), population ([`worldgen::populate`], [`worldgen::features`]) done (WP-1.7) |
//! | [`golden`] | — | read-only views in the golden-trace shapes |
//!
//! `worldgen` is the plan's `gen`, renamed because `gen` is a reserved keyword
//! in Rust 2024.
//!
//! # Gaps are visible, never silent
//!
//! Untranscribed code never panics: world systems record their Java name in
//! [`world::World::skipped_systems`], block/entity overrides in
//! [`world::World::unimplemented_hits`], each where the Java code would run.
//!
//! Browser-ready: no `std::fs`, threads, clocks, environment access or `rand`
//! (D9). Storage goes through [`save::Storage`]; time is passed in. No
//! `unsafe`.

#![forbid(unsafe_code)]

pub mod block;
pub mod entity;
pub mod golden;
pub mod inventory;
pub mod item;
pub mod save;
pub mod spawner;
pub mod tick;
pub mod tile_entity;
pub mod world;
pub mod worldgen;
