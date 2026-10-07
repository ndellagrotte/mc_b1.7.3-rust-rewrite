//! Item behaviour (WP-2.6) the golden fixtures do not reach: the map files
//! through `MapStorage` and a save handler (the oracle's Tier S lines do not
//! carry `data/`), `getMPMapData` (not called in a single-player
//! session by any op), `ItemMap.onCreated` called directly (the crafted map
//! is pinned by `tick/map-craft`), thrown eggs, and the fishing rod whose
//! bobber is gone. The rest is covered by `tests/golden/tick/*` (README
//! "Committed Tier B/C/S scenarios", WP-2.6).

use mc_nbt::Tag;
use mc_sim::entity::player::new_entity_player_sp;
use mc_sim::item::behaviour::{self, map};
use mc_sim::item::{ItemStack, RightClickResult};
use mc_sim::save::{FixedClock, MemoryStorage, NoSave, SaveOldDir, SaveStore, SharedSaveStore};
use mc_sim::world::chunk_provider::{ChunkGenerator, EmptyChunkProvider};
use mc_sim::world::map_storage::MapData;
use mc_sim::world::provider::{Dimension, WorldProvider};
use mc_sim::world::{World, WorldEnvironment};

const ENV: WorldEnvironment = WorldEnvironment {
    rand_seed: 0,
    field_9437_g: 0,
    lock_timestamp: 0,
};

/// `Item.mapItem.shiftedIndex`.
// src: Item.java:376 (new ItemMap(102): 256 + 102)
const MAP: i32 = 358;

fn store() -> SharedSaveStore {
    SaveStore::shared(
        Box::new(MemoryStorage::default()),
        Box::new(FixedClock(1_700_000_000_000)),
    )
}

/// A new world on `world/` of `store` (empty terrain).
fn world_on(store: &SharedSaveStore) -> World {
    let handler = SaveOldDir::new(store.clone(), "", "world").expect("session lock");
    let mut w = World::new(
        Box::new(handler),
        "world",
        WorldProvider::new(Dimension::Surface),
        5,
        ChunkGenerator::Empty(EmptyChunkProvider),
        ENV,
    );
    w.make_deterministic(5);
    w
}

fn empty_world() -> World {
    let mut w = World::new(
        Box::new(NoSave),
        "harness",
        WorldProvider::new(Dimension::Surface),
        5,
        ChunkGenerator::Empty(EmptyChunkProvider),
        ENV,
    );
    w.make_deterministic(5);
    w
}

fn read(store: &SharedSaveStore, path: &str) -> Option<Vec<u8>> {
    store.borrow_mut().storage.read(path).unwrap()
}

/// `getUniqueDataId` counts per key from 0 and writes every count to
/// `data/idcounts.dat` (uncompressed, one short per key); a new storage on
/// the same save reads them back (MapStorage.java:103-167).
#[test]
fn unique_ids_persist_in_idcounts() {
    let s = store();
    let mut w = world_on(&s);
    assert_eq!(w.get_unique_data_id("map"), 0);
    assert_eq!(w.get_unique_data_id("map"), 1);
    assert_eq!(w.get_unique_data_id("other"), 0);
    let bytes = read(&s, "world/data/idcounts.dat").expect("idcounts.dat written");
    let (name, root) = mc_nbt::read_named(&bytes).unwrap();
    assert_eq!(name, "");
    assert_eq!(root.get("map"), Some(&Tag::Short(1)));
    assert_eq!(root.get("other"), Some(&Tag::Short(0)));
    drop(w);
    let mut w2 = world_on(&s);
    assert_eq!(w2.get_unique_data_id("map"), 2);
}

/// Without map files (`SaveHandlerMP`) ids still count, nothing is read or
/// written, and `loadItemData` finds only what was set.
#[test]
fn no_files_without_a_map_directory() {
    let mut w = empty_world();
    assert_eq!(w.get_unique_data_id("map"), 0);
    assert!(w.load_item_data("map_0").is_none());
    let d = MapData::new("map_0").into_ref();
    w.set_item_data("map_0", d.clone());
    assert!(std::rc::Rc::ptr_eq(&w.load_item_data("map_0").unwrap(), &d));
}

/// A dirty map is written by the save (`saveLevel` → `saveAllData`) as
/// `data/map_<id>.dat`, gzipped `{"data": {…}}`, and marked clean; a fresh
/// world on the same save loads it (MapStorage.java:25-101).
#[test]
fn dirty_maps_save_and_load_back() {
    let s = store();
    let mut w = world_on(&s);
    let mut stack = ItemStack::new(MAP, 1, 0);
    let d = map::get_map_data(&mut stack, &mut w);
    assert_eq!(stack.item_damage, 0);
    {
        let mut m = d.borrow_mut();
        assert!(m.is_dirty());
        assert_eq!((m.scale, m.x_center, m.z_center, m.dimension), (3, 0, 0, 0));
        m.colors[1000] = 0x2D;
    }
    w.try_save_world(true).unwrap();
    assert!(!d.borrow().is_dirty());
    let bytes = read(&s, "world/data/map_0.dat").expect("map_0.dat written");
    let (_, root) = mc_nbt::read_gzip(&bytes).unwrap();
    let data = root.get("data").expect("data compound");
    assert_eq!(data.get("scale"), Some(&Tag::Byte(3)));
    assert_eq!(data.get("width"), Some(&Tag::Short(128)));
    drop(w);
    let mut w2 = world_on(&s);
    let back = w2.load_item_data("map_0").expect("read from the file");
    assert_eq!(back.borrow().colors[1000], 0x2D);
    assert_eq!(back.borrow().scale, 3);
    // A clean map is not written again: delete the file, save, still gone.
    s.borrow_mut()
        .storage
        .delete("world/data/map_0.dat")
        .unwrap();
    w2.try_save_world(true).unwrap();
    assert!(read(&s, "world/data/map_0.dat").is_none());
}

/// `ItemMap.onCreated` (a crafted map): a fresh id, centred on the player,
/// scale 3, dirty (ItemMap.java:228-238); `ItemStack.onCrafting` calls it.
#[test]
fn on_created_centres_on_the_player() {
    let mut w = empty_world();
    w.get_unique_data_id("map");
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_position(-20.5, 70.0, 33.9);
    let mut stack = ItemStack::new(MAP, 1, 0);
    stack.on_crafting(&mut w, &mut p);
    assert_eq!(stack.item_damage, 1);
    let d = w.load_item_data("map_1").expect("map_1");
    let d = d.borrow();
    assert_eq!((d.x_center, d.z_center, d.scale), (-21, 33, 3));
    assert!(d.is_dirty());
}

/// `ItemMap.getMPMapData(id)` makes the new map under a fresh id, not
/// `map_<id>` (ItemMap.java:9-20).
#[test]
fn get_mp_map_data_uses_a_fresh_name() {
    let mut w = empty_world();
    let d = map::get_mp_map_data(7, &mut w);
    assert_eq!(d.borrow().map_name, "map_0");
    assert!(w.load_item_data("map_7").is_none());
}

/// A thrown egg: one off the stack, an `EntityEgg` joins, `Item.itemRand`
/// drew the pitch (ItemEgg.java:9-17).
#[test]
fn egg_throw() {
    let mut w = empty_world();
    w.get_chunk_from_chunk_coords(0, 0);
    let mut p = new_entity_player_sp(&mut w);
    p.base.set_location_and_angles(0.5, 70.0, 0.5, 0.0, 0.0);
    let before = w.item_rand.state();
    let mut s = ItemStack::new(344, 3, 0);
    assert_eq!(
        behaviour::on_item_right_click(&mut s, &mut w, &mut p),
        RightClickResult::Same
    );
    assert_eq!(s.stack_size, 2);
    assert_ne!(w.item_rand.state(), before);
    assert_eq!(w.count_entities("EntityEgg"), 1);
}

/// The fishing rod with a `fishEntity` that is not in the world any more:
/// recorded, the reference cleared, no wear (the port cannot reach the
/// bobber object).
#[test]
fn fishing_rod_with_a_lost_bobber() {
    let mut w = empty_world();
    let mut p = new_entity_player_sp(&mut w);
    p.kind.player_mut().unwrap().fish_entity = Some(99);
    let mut s = ItemStack::new(346, 1, 0);
    behaviour::on_item_right_click(&mut s, &mut w, &mut p);
    assert_eq!(s.item_damage, 0);
    assert_eq!(p.kind.player().unwrap().fish_entity, None);
    assert!(
        w.unimplemented_hits
            .borrow()
            .contains_key("EntityFish.catchFish: bobber not in loadedEntityList")
    );
}
