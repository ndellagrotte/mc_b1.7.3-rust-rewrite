//! `HarnessSaveHandler` (the save handler of `"save_format": "mcregion"` and
//! `"world": "load"` tick scenarios) keeps `SaveOldDir`'s map files: the
//! oracle's `SaveOldDir` inherits `SaveHandler.getMapFile`, which is never
//! null (SaveHandler.java:23-24, :167-169), so `MapStorage` reads and writes
//! `data/<name>.dat` there.

use mc_golden::tick_driver::HarnessSaveHandler;
use mc_sim::save::{FixedClock, MemoryStorage, SaveHandler, SaveOldDir, SaveStore};

#[test]
fn map_files_reach_the_save_directory() {
    let store = SaveStore::shared(
        Box::new(MemoryStorage::default()),
        Box::new(FixedClock(1_700_000_000_000)),
    );
    let dir = SaveOldDir::new(store.clone(), "", "world").expect("session lock");
    let mut h = HarnessSaveHandler(dir);
    assert!(h.has_map_files());
    assert_eq!(h.read_map_file("idcounts").unwrap(), None);
    h.write_map_file("idcounts", &[1, 2, 3]).unwrap();
    assert_eq!(
        store
            .borrow_mut()
            .storage
            .read("world/data/idcounts.dat")
            .unwrap(),
        Some(vec![1, 2, 3])
    );
    assert_eq!(h.read_map_file("idcounts").unwrap(), Some(vec![1, 2, 3]));
}
