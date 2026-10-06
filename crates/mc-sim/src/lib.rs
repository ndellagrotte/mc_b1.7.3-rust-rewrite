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
//! | [`item`] | `Item` + subclasses, `EnumToolMaterial`, `ItemStack` | const table [`item::ITEMS`] / [`item::RECORDS`] of `ItemDef` + `ItemKind` enum, data methods as `impl ItemDef`; `ItemStack` value type with NBT; block texture lookups in `item::block_texture` |
//! | [`world`] | `World`, `Chunk`, `NibbleArray`, `ChunkProvider`, `WorldInfo`, `WorldProvider`, `TileEntity`, `NextTickListEntry`, `IWorldAccess` | [`world::World`] owns everything; client side effects are [`world::events`] |
//! | [`tick`] | `World.tick` and its block systems | `impl World` |
//! | [`entity`] | `Entity` + subclasses | `Entity { base, kind }`, per-class modules, [`entity::dispatch`], take/put-back driver |
//! | [`inventory`] | `InventoryPlayer` | layout only (WP-2.5) |
//! | [`save`] | `ISaveHandler`, `IChunkLoader`, [`save::Storage`] (D9) | traits + `NoSave`; McRegion is WP-1.9 |
//! | [`worldgen`] | `ChunkProviderGenerate`, noise, biomes | empty (WP-1.4, WP-1.7) |
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
pub mod tick;
pub mod world;
pub mod worldgen;
