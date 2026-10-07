//! Map item data (WP-2.6): `MapStorage` (MapStorage.java), `MapDataBase`
//! (MapDataBase.java), `MapData` (MapData.java), `MapInfo` (MapInfo.java)
//! and `MapCoord` (MapCoord.java). The world holds one [`MapStorage`] as
//! `field_28108_z` ([`crate::world::World::field_28108_z`]); `ItemMap`
//! ([`crate::item::behaviour::map`]) reads and draws the [`MapData`] in it.
//!
//! # Shape
//!
//! - `MapDataBase` is abstract with one subclass, `MapData`, the only data
//!   any b1.7.3 caller stores (`loadData(MapData.class, …)`,
//!   ItemMap.java:11, :24). Its two fields (`mapName`, `dirty`) live in
//!   [`MapData`]; the reflective `getConstructor(String).newInstance(name)`
//!   of `loadData` (MapStorage.java:35) is [`MapData::new`].
//! - Java shares one `MapData` object between the storage's map, its list
//!   and every `ItemMap` call; here it is an `Rc<RefCell<MapData>>`
//!   ([`MapDataRef`]), so the item code can hold it while it also borrows
//!   the world (the simulation is single-threaded, PLAN_v4 §3.2).
//! - `MapStorage.loadedDataMap` and `idCounts` are `HashMap`s used for
//!   lookup only (the `idcounts` file is an NBT compound, whose key order is
//!   never compared, PLAN_v4 §4.3), so `BTreeMap`s here; `loadedDataList`
//!   keeps Java's list order (it is the save order of `saveAllData`).
//! - `MapData.playersHashMap` maps a player to its `MapInfo`, which
//!   `playersArrayList` also holds in insertion order; the port keeps the
//!   list only and looks a player up by entity id in it (`MapInfo` holds
//!   the player's entity id, [`crate::entity`] "References between
//!   entities").
//! - The files go through the world's save handler
//!   ([`SaveHandler::read_map_file`], [`SaveHandler::write_map_file`]):
//!   `getMapFile(name)` is `data/<name>.dat` in the world directory
//!   (SaveHandler.java:23, :167-169) and null for `SaveHandlerMP`
//!   ([`SaveHandler::has_map_files`] false), where nothing is read or
//!   written. A map file is gzipped NBT with the `MapData` fields in a
//!   compound `data` (MapStorage.java:41-43, :88-93); `idcounts.dat` is
//!   uncompressed NBT, one `TAG_Short` per key (:110-125, :146-158).
//!
//! # Not ported
//!
//! `MapData.updateMPMapData` (MapData.java:125-148), the multiplayer
//! client's decode of a map packet (colour columns, or the marker list), is
//! not ported (D3), like `ItemMap.getMPMapData`'s caller.
//!
//! # Errors
//!
//! Every file error is caught and printed in Java (`printStackTrace`), so
//! none of these methods fails; the port drops the error at the same
//! place. A map file that exists but does not read still leaves the
//! `MapData` constructed before the read in the storage, partly read
//! (MapStorage.java:34-53).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use mc_nbt::Tag;

use crate::entity::Entity;
use crate::item::{ItemStack, JavaException};
use crate::save::SaveHandler;
use crate::save::nbt::{self as n, Compound};

/// A shared `MapData` object (module docs).
pub type MapDataRef = Rc<RefCell<MapData>>;

/// `MapData.colors.length` for a 128 × 128 map.
// src: MapData.java:13
pub const MAP_COLORS_LEN: usize = 16384;

/// `MapCoord` (MapCoord.java): one player marker on a map. The synthetic
/// outer-instance field `data` is not kept.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapCoord {
    /// `iconSize` (the icon type; 0 for a player).
    pub icon_size: i8,
    /// `centerX`.
    pub center_x: i8,
    /// `centerZ`.
    pub center_z: i8,
    /// `iconRotation`.
    pub icon_rotation: i8,
}

/// `MapInfo` (MapInfo.java): what one player has been sent of a map. The
/// synthetic outer-instance field `mapDataObj` is not kept.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapInfo {
    /// `entityplayerObj`, as the player's entity id.
    pub entityplayer_obj: i32,
    /// `field_28119_b`: per column, the first dirty row (name kept, see
    /// `docs/mappings.md`).
    pub field_28119_b: [i32; 128],
    /// `field_28124_c`: per column, the last dirty row (name kept).
    pub field_28124_c: [i32; 128],
    /// `field_28122_e` (private, never read in the client; name kept).
    pub field_28122_e: i32,
    /// `field_28121_f` (private, never read in the client; name kept).
    pub field_28121_f: i32,
}

impl MapInfo {
    /// `new MapInfo(mapData, player)`: every column clean from row 0 to 127.
    // src: MapInfo.java:12-25
    pub fn new(player: i32) -> Self {
        MapInfo {
            entityplayer_obj: player,
            field_28119_b: [0; 128],
            field_28124_c: [127; 128],
            field_28122_e: 0,
            field_28121_f: 0,
        }
    }
}

/// What `updateVisiblePlayers` reads of a viewer (`MapInfo.entityplayerObj`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapViewer {
    /// `isDead`.
    pub is_dead: bool,
    /// `posX`.
    pub pos_x: f64,
    /// `posZ`.
    pub pos_z: f64,
    /// `EntityPlayer.dimension`.
    pub dimension: i32,
    /// `inventory.hasItemStack(stack)` for the map's stack.
    pub has_item_stack: bool,
}

impl MapViewer {
    /// The viewer state of player `p`, with `hasItemStack` given.
    pub fn of(p: &Entity, has_item_stack: bool) -> Self {
        MapViewer {
            is_dead: p.base.is_dead,
            pos_x: p.base.pos_x,
            pos_z: p.base.pos_z,
            dimension: p
                .kind
                .player()
                .expect("a map viewer is an EntityPlayer")
                .dimension,
            has_item_stack,
        }
    }
}

/// `MapData` (MapData.java) with `MapDataBase`'s fields (MapDataBase.java:4-5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapData {
    /// `MapDataBase.mapName` (`"map_<id>"`).
    pub map_name: String,
    /// `MapDataBase.dirty` (private; `isDirty()` / `setDirty()`).
    dirty: bool,
    /// `xCenter`.
    pub x_center: i32,
    /// `zCenter`.
    pub z_center: i32,
    /// `dimension` (`worldProvider.worldType` as a byte).
    pub dimension: i8,
    /// `scale`: one map pixel is `1 << scale` blocks.
    pub scale: i8,
    /// `colors`: 128 × 128 palette bytes (`colorIndex * 4 + shade`), row
    /// major. Java bytes as their bit patterns. Normally 16384 long; a
    /// 128 × 128 map file stores whatever array it holds (MapData.java:39).
    pub colors: Vec<u8>,
    /// `step`: draw passes so far (`updateMapData` draws the columns
    /// congruent to it mod 16).
    pub step: i32,
    /// `playersArrayList` (with `playersHashMap`, module docs).
    pub players_array_list: Vec<MapInfo>,
    /// `playersVisibleOnMap`: the markers of the last
    /// `updateVisiblePlayers`.
    pub players_visible_on_map: Vec<MapCoord>,
}

impl MapData {
    /// `new MapData(name)`: all fields zero, a blank 16384-byte canvas.
    // src: MapData.java:8-21, MapDataBase.java:7-9
    pub fn new(name: &str) -> Self {
        MapData {
            map_name: name.to_string(),
            dirty: false,
            x_center: 0,
            z_center: 0,
            dimension: 0,
            scale: 0,
            colors: vec![0; MAP_COLORS_LEN],
            step: 0,
            players_array_list: Vec::new(),
            players_visible_on_map: Vec::new(),
        }
    }

    /// The data as the shared object the storage and the items hold.
    pub fn into_ref(self) -> MapDataRef {
        Rc::new(RefCell::new(self))
    }

    /// `MapDataBase.markDirty()`.
    // src: MapDataBase.java:15-17
    pub fn mark_dirty(&mut self) {
        self.set_dirty(true);
    }

    /// `MapDataBase.setDirty(dirty)`.
    // src: MapDataBase.java:19-21
    pub fn set_dirty(&mut self, v: bool) {
        self.dirty = v;
    }

    /// `MapDataBase.isDirty()`.
    // src: MapDataBase.java:23-25
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// `readFromNBT(compound)`: `dimension`, `xCenter`, `zCenter`, `scale`
    /// (clamped to 0..=4), then the colours: a 128 × 128 file's array as
    /// stored, any other size centred on a blank canvas. Fields are
    /// assigned in Java's order, so an `Err` (what Java throws: a key of
    /// the wrong tag type, an array index out of bounds) leaves the fields
    /// read so far set, as the caught exception does (MapStorage.java:45).
    // src: MapData.java:23-59
    #[allow(clippy::manual_clamp)] // Java's two ifs, kept
    pub fn read_from_nbt(&mut self, var1: &Compound) -> Result<(), JavaException> {
        self.dimension = n::get_byte(var1, "dimension")?;
        self.x_center = n::get_integer(var1, "xCenter")?;
        self.z_center = n::get_integer(var1, "zCenter")?;
        self.scale = n::get_byte(var1, "scale")?;
        if self.scale < 0 {
            self.scale = 0;
        }
        if self.scale > 4 {
            self.scale = 4;
        }
        let var2 = i32::from(n::get_short(var1, "width")?);
        let var3 = i32::from(n::get_short(var1, "height")?);
        if var2 == 128 && var3 == 128 {
            self.colors = n::get_byte_array(var1, "colors")?.to_vec();
        } else {
            let var4 = n::get_byte_array(var1, "colors")?.to_vec();
            self.colors = vec![0; MAP_COLORS_LEN];
            let var5 = (128 - var2) / 2;
            let var6 = (128 - var3) / 2;
            for var7 in 0..var3 {
                let var8 = var7 + var6;
                // vanilla bug: `||` where `&&` was meant (always true), so
                // rows and columns outside the canvas are not skipped and
                // throw (ArrayIndexOutOfBoundsException) or wrap into
                // another row.
                #[allow(clippy::nonminimal_bool, clippy::overly_complex_bool_expr)]
                if var8 >= 0 || var8 < 128 {
                    for var9 in 0..var2 {
                        let var10 = var9 + var5;
                        #[allow(clippy::nonminimal_bool, clippy::overly_complex_bool_expr)]
                        if var10 >= 0 || var10 < 128 {
                            let src =
                                java_index(&var4, var9.wrapping_add(var7.wrapping_mul(var2)))?;
                            let dst = var10.wrapping_add(var8.wrapping_mul(128));
                            *java_index_mut(&mut self.colors, dst)? = src;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// `writeToNBT(compound)`: the fields, with `width` and `height` 128.
    // src: MapData.java:61-69
    pub fn write_to_nbt(&self, var1: &mut Compound) {
        n::set(var1, "dimension", Tag::Byte(self.dimension));
        n::set(var1, "xCenter", Tag::Int(self.x_center));
        n::set(var1, "zCenter", Tag::Int(self.z_center));
        n::set(var1, "scale", Tag::Byte(self.scale));
        n::set(var1, "width", Tag::Short(128));
        n::set(var1, "height", Tag::Short(128));
        n::set(var1, "colors", Tag::ByteArray(self.colors.clone()));
    }

    /// `updateVisiblePlayers(player, stack)`: registers `player` (in hand,
    /// out of its slot) as a viewer, then rebuilds the markers: every
    /// viewer that is alive and still carries the map (`hasItemStack`) and
    /// stands within 64 pixels of the centre gets a marker (in the nether
    /// the arrow spins with `step`); the others are dropped. Other viewers
    /// are looked up with `others(id, stack)`, which answers their
    /// `hasItemStack(stack)` too (`None`: gone, counted as dead).
    ///
    /// For `player` itself `inventory.hasItemStack(stack)` is true without
    /// a search: `stack` is one of its main slots (the caller,
    /// `InventoryPlayer.decrementAnimations`, takes it out of the slot while
    /// the item runs, so a search would miss it), and `isStackEqual` of a
    /// stack with itself holds.
    // src: MapData.java:71-107
    pub fn update_visible_players(
        &mut self,
        var1: &Entity,
        var2: &ItemStack,
        others: impl Fn(i32, &ItemStack) -> Option<MapViewer>,
    ) {
        let me = var1.base.entity_id;
        if !self
            .players_array_list
            .iter()
            .any(|i| i.entityplayer_obj == me)
        {
            // src: MapData.java:72-76
            self.players_array_list.push(MapInfo::new(me));
        }
        self.players_visible_on_map.clear();
        let mut var14 = 0usize;
        while var14 < self.players_array_list.len() {
            let pid = self.players_array_list[var14].entityplayer_obj;
            let viewer = if pid == me {
                Some(MapViewer::of(var1, true))
            } else {
                others(pid, var2)
            };
            match viewer {
                Some(MapViewer {
                    is_dead: false,
                    pos_x: px,
                    pos_z: pz,
                    dimension: dim,
                    has_item_stack: true,
                }) => {
                    // src: MapData.java:83-99
                    let shift = 1i32.wrapping_shl(self.scale as i32 as u32) as f32;
                    let var5 = (px - self.x_center as f64) as f32 / shift;
                    let var6 = (pz - self.z_center as f64) as f32 / shift;
                    let var7: i8 = 64;
                    let var8: i8 = 64;
                    if var5 >= -(var7 as f32)
                        && var6 >= -(var8 as f32)
                        && var5 <= var7 as f32
                        && var6 <= var8 as f32
                    {
                        let var9: i8 = 0;
                        let var10 = ((var5 * 2.0) as f64 + 0.5) as i32 as i8;
                        let var11 = ((var6 * 2.0) as f64 + 0.5) as i32 as i8;
                        let mut var12 =
                            ((var1.base.rotation_yaw * 16.0 / 360.0) as f64 + 0.5) as i32 as i8;
                        if self.dimension < 0 {
                            let var13 = self.step / 10;
                            var12 = (var13
                                .wrapping_mul(var13)
                                .wrapping_mul(34187121)
                                .wrapping_add(var13.wrapping_mul(121))
                                >> 15
                                & 15) as i8;
                        }
                        if dim == self.dimension as i32 {
                            self.players_visible_on_map.push(MapCoord {
                                icon_size: var9,
                                center_x: var10,
                                center_z: var11,
                                icon_rotation: var12,
                            });
                        }
                    }
                }
                _ => {
                    // src: MapData.java:102-103. vanilla bug: the removal shifts
                    // the next viewer into index var14, which the loop's
                    // ++var14 then skips this pass.
                    self.players_array_list.remove(var14);
                }
            }
            var14 += 1;
        }
    }

    /// `setColumnDirty(x, y0, y1)`: marks the data dirty and widens every
    /// viewer's dirty range of column `x`.
    // src: MapData.java:109-123
    pub fn set_column_dirty(&mut self, var1: i32, var2: i32, var3: i32) {
        self.mark_dirty();
        let x = var1 as usize;
        for var5 in &mut self.players_array_list {
            if var5.field_28119_b[x] < 0 || var5.field_28119_b[x] > var2 {
                var5.field_28119_b[x] = var2;
            }
            if var5.field_28124_c[x] < 0 || var5.field_28124_c[x] < var3 {
                var5.field_28124_c[x] = var3;
            }
        }
    }
}

/// `array[i]` with Java's bounds check.
fn java_index(a: &[u8], i: i32) -> Result<u8, JavaException> {
    usize::try_from(i)
        .ok()
        .and_then(|i| a.get(i).copied())
        .ok_or(JavaException::ArrayIndexOutOfBounds)
}

/// `array[i] = …` with Java's bounds check.
fn java_index_mut(a: &mut [u8], i: i32) -> Result<&mut u8, JavaException> {
    usize::try_from(i)
        .ok()
        .and_then(|i| a.get_mut(i))
        .ok_or(JavaException::ArrayIndexOutOfBounds)
}

/// `MapStorage` (MapStorage.java), the world's `field_28108_z`.
#[derive(Clone, Debug, Default)]
pub struct MapStorage {
    /// `loadedDataMap` (lookup only, module docs).
    loaded_data_map: BTreeMap<String, MapDataRef>,
    /// `loadedDataList`, in insertion order.
    loaded_data_list: Vec<MapDataRef>,
    /// `idCounts`: the last id handed out per key.
    id_counts: BTreeMap<String, i16>,
}

impl MapStorage {
    /// `new MapStorage(saveHandler)`: reads `idcounts.dat`.
    // src: MapStorage.java:20-23
    pub fn new(handler: &mut dyn SaveHandler) -> Self {
        let mut s = MapStorage::default();
        s.load_id_counts(handler);
        s
    }

    /// The loaded data called `name`, without reading any file (the
    /// harness's `map_data` dump; `loadedDataMap.get(name)`).
    pub fn loaded(&self, name: &str) -> Option<&MapDataRef> {
        self.loaded_data_map.get(name)
    }

    /// `loadedDataList`, in order.
    pub fn loaded_list(&self) -> &[MapDataRef] {
        &self.loaded_data_list
    }

    /// `loadData(MapData.class, name)`: the loaded data, else the file
    /// `name` read into a new `MapData` (kept even when the read fails
    /// after the file was found, module docs "Errors"), else `None`.
    // src: MapStorage.java:25-57
    pub fn load_data(&mut self, handler: &mut dyn SaveHandler, var2: &str) -> Option<MapDataRef> {
        if let Some(var3) = self.loaded_data_map.get(var2) {
            return Some(var3.clone());
        }
        let mut var3: Option<MapData> = None;
        if handler.has_map_files() {
            // getMapFile(name).exists(): a storage error reads as a file that
            // exists but cannot be opened (the FileInputStream throws after the
            // MapData was constructed).
            let bytes = handler.read_map_file(var2);
            match bytes {
                Ok(None) => {}
                Ok(Some(bytes)) => {
                    let mut d = MapData::new(var2);
                    // Any exception from here on is caught and printed
                    // (MapStorage.java:45-47); `d` stays as far as it got.
                    if let Ok((_, root)) = mc_nbt::read_gzip(&bytes)
                        && let Ok(c) = n::as_compound(&root)
                        && let Ok(data) = n::get_compound_tag(c, "data")
                    {
                        let _ = d.read_from_nbt(&data);
                    }
                    var3 = Some(d);
                }
                Err(_) => var3 = Some(MapData::new(var2)),
            }
        }
        let var3 = var3.map(MapData::into_ref);
        if let Some(d) = &var3 {
            self.loaded_data_map.insert(var2.to_string(), d.clone());
            self.loaded_data_list.push(d.clone());
        }
        var3
    }

    /// `setData(name, data)`: replaces any data loaded under `name` (removed
    /// from the list; the new one goes at the end).
    // src: MapStorage.java:59-70
    pub fn set_data(&mut self, var1: &str, var2: MapDataRef) {
        if let Some(old) = self.loaded_data_map.remove(var1) {
            // loadedDataList.remove(Object): the first element equal to it,
            // which for MapData (no equals override) is the same object.
            if let Some(i) = self
                .loaded_data_list
                .iter()
                .position(|d| Rc::ptr_eq(d, &old))
            {
                self.loaded_data_list.remove(i);
            }
        }
        self.loaded_data_map.insert(var1.to_string(), var2.clone());
        self.loaded_data_list.push(var2);
    }

    /// `saveAllData()`: writes every dirty data in list order and marks it
    /// clean (also when there is nowhere to write it).
    // src: MapStorage.java:72-81
    pub fn save_all_data(&mut self, handler: &mut dyn SaveHandler) {
        for var2 in &self.loaded_data_list {
            let mut d = var2.borrow_mut();
            if d.is_dirty() {
                Self::save_data(handler, &d);
                d.set_dirty(false);
            }
        }
    }

    /// `saveData(data)` (private): `{"data": <fields>}` gzipped to the map
    /// file. Errors are printed in Java and dropped here.
    // src: MapStorage.java:83-101
    fn save_data(handler: &mut dyn SaveHandler, var1: &MapData) {
        if handler.has_map_files() {
            let mut var3 = Compound::new();
            var1.write_to_nbt(&mut var3);
            let mut var4 = Compound::new();
            n::set(&mut var4, "data", Tag::Compound(var3));
            if let Ok(bytes) = mc_nbt::write_gzip("", &Tag::Compound(var4)) {
                let _ = handler.write_map_file(&var1.map_name, &bytes);
            }
        }
    }

    /// `loadIdCounts()` (private): every `TAG_Short` of `idcounts.dat`.
    // src: MapStorage.java:103-131
    fn load_id_counts(&mut self, handler: &mut dyn SaveHandler) {
        self.id_counts.clear();
        if !handler.has_map_files() {
            return;
        }
        let Ok(Some(bytes)) = handler.read_map_file("idcounts") else {
            return;
        };
        // CompressedStreamTools.read: an uncompressed named compound.
        let Ok((_, Tag::Compound(var3))) = mc_nbt::read_named(&bytes) else {
            return;
        };
        for (var7, var5) in &var3 {
            if let Tag::Short(var8) = var5 {
                self.id_counts.insert(var7.clone(), *var8);
            }
        }
    }

    /// `getUniqueDataId(key)`: the next id for `key` (0 first; a `short`, so
    /// it wraps after 32767), then every count written to `idcounts.dat`.
    // src: MapStorage.java:133-167
    pub fn get_unique_data_id(&mut self, handler: &mut dyn SaveHandler, var1: &str) -> i32 {
        let var2: i16 = match self.id_counts.get(var1) {
            None => 0,
            Some(v) => v.wrapping_add(1),
        };
        self.id_counts.insert(var1.to_string(), var2);
        if handler.has_map_files() {
            let mut var4 = Compound::new();
            for (var6, var7) in &self.id_counts {
                n::set(&mut var4, var6, Tag::Short(*var7));
            }
            // CompressedStreamTools.write: uncompressed; errors printed.
            if let Ok(bytes) = mc_nbt::write_named("", &Tag::Compound(var4)) {
                let _ = handler.write_map_file("idcounts", &bytes);
            }
        }
        i32::from(var2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nbt_round_trip_and_clamped_scale() {
        let mut d = MapData::new("map_3");
        d.x_center = -40;
        d.z_center = 7;
        d.dimension = -1;
        d.scale = 3;
        d.colors[5] = 0xAB;
        let mut c = Compound::new();
        d.write_to_nbt(&mut c);
        let mut r = MapData::new("map_3");
        r.read_from_nbt(&c).unwrap();
        assert_eq!(r, d);
        c.insert("scale".into(), Tag::Byte(9));
        r.read_from_nbt(&c).unwrap();
        assert_eq!(r.scale, 4);
    }

    #[test]
    fn small_canvas_is_centred() {
        // A 2 × 2 canvas lands at (63, 63)..(64, 64) (MapData.java:41-55).
        let mut c = Compound::new();
        c.insert("width".into(), Tag::Short(2));
        c.insert("height".into(), Tag::Short(2));
        c.insert("colors".into(), Tag::ByteArray(vec![1, 2, 3, 4]));
        let mut r = MapData::new("map_0");
        r.read_from_nbt(&c).unwrap();
        assert_eq!(r.colors.len(), MAP_COLORS_LEN);
        assert_eq!(r.colors[63 + 63 * 128], 1);
        assert_eq!(r.colors[64 + 63 * 128], 2);
        assert_eq!(r.colors[63 + 64 * 128], 3);
        assert_eq!(r.colors[64 + 64 * 128], 4);
    }

    #[test]
    fn wrong_tag_type_keeps_the_fields_read_before() {
        let mut c = Compound::new();
        c.insert("dimension".into(), Tag::Byte(-1));
        c.insert("xCenter".into(), Tag::Short(1));
        let mut r = MapData::new("map_0");
        assert_eq!(r.read_from_nbt(&c), Err(JavaException::ClassCast));
        assert_eq!(r.dimension, -1);
        assert_eq!(r.x_center, 0);
    }
}
