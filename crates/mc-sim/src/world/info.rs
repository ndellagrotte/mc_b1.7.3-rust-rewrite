//! `WorldInfo` (WorldInfo.java): the `level.dat` fields and their NBT form
//! (the `Data` compound of `level.dat`, WP-1.9b).
//!
//! The copy constructor `WorldInfo(WorldInfo)` (WorldInfo.java:48-64) is
//! `Clone`; the getters and setters (WorldInfo.java:109-221) are the public
//! fields.

use mc_nbt::Tag;

use crate::item::JavaException;
use crate::save::nbt::{self as n, Compound};

/// `WorldInfo`.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldInfo {
    // src: WorldInfo.java:6-20
    /// `randomSeed`.
    pub random_seed: i64,
    /// `spawnX`.
    pub spawn_x: i32,
    /// `spawnY`.
    pub spawn_y: i32,
    /// `spawnZ`.
    pub spawn_z: i32,
    /// `worldTime`.
    pub world_time: i64,
    /// `lastTimePlayed`.
    pub last_time_played: i64,
    /// `sizeOnDisk`.
    pub size_on_disk: i64,
    /// `playerTag`.
    pub player_tag: Option<Tag>,
    /// `dimension`.
    pub dimension: i32,
    /// `levelName`.
    pub level_name: String,
    /// `saveVersion`.
    pub save_version: i32,
    /// `raining`.
    pub raining: bool,
    /// `rainTime`.
    pub rain_time: i32,
    /// `thundering`.
    pub thundering: bool,
    /// `thunderTime`.
    pub thunder_time: i32,
}

impl WorldInfo {
    /// `new WorldInfo(seed, name)`: everything else zero.
    // src: WorldInfo.java:43-46
    pub fn new(random_seed: i64, level_name: &str) -> Self {
        WorldInfo {
            random_seed,
            spawn_x: 0,
            spawn_y: 0,
            spawn_z: 0,
            world_time: 0,
            last_time_played: 0,
            size_on_disk: 0,
            player_tag: None,
            dimension: 0,
            level_name: level_name.to_string(),
            save_version: 0,
            raining: false,
            rain_time: 0,
            thundering: false,
            thunder_time: 0,
        }
    }

    /// `new WorldInfo(NBTTagCompound)`: every field read with the
    /// `NBTTagCompound` getters (a missing key is 0 / `""`; a key of another
    /// type throws `ClassCastException`, which `SaveHandler.loadWorldInfo`
    /// catches, SaveHandler.java:90-92). `dimension` comes from the
    /// `Player` compound and stays 0 without one.
    // src: WorldInfo.java:22-41
    pub fn from_nbt(c: &Compound) -> Result<Self, JavaException> {
        let mut w = WorldInfo::new(0, "");
        w.random_seed = n::get_long(c, "RandomSeed")?;
        w.spawn_x = n::get_integer(c, "SpawnX")?;
        w.spawn_y = n::get_integer(c, "SpawnY")?;
        w.spawn_z = n::get_integer(c, "SpawnZ")?;
        w.world_time = n::get_long(c, "Time")?;
        w.last_time_played = n::get_long(c, "LastPlayed")?;
        w.size_on_disk = n::get_long(c, "SizeOnDisk")?;
        w.level_name = n::get_string(c, "LevelName")?;
        w.save_version = n::get_integer(c, "version")?;
        w.rain_time = n::get_integer(c, "rainTime")?;
        w.raining = n::get_boolean(c, "raining")?;
        w.thunder_time = n::get_integer(c, "thunderTime")?;
        w.thundering = n::get_boolean(c, "thundering")?;
        if n::has_key(c, "Player") {
            let p = n::get_compound_tag(c, "Player")?;
            w.dimension = n::get_integer(&p, "Dimension")?;
            w.player_tag = Some(Tag::Compound(p));
        }
        Ok(w)
    }

    /// `getNBTTagCompound()`: the fields plus the stored `playerTag`.
    /// `now_millis` is `System.currentTimeMillis()` for `LastPlayed` (D9).
    // src: WorldInfo.java:66-70
    pub fn get_nbt_tag_compound(&self, now_millis: i64) -> Compound {
        let mut var1 = Compound::new();
        self.update_tag_compound(&mut var1, self.player_tag.clone(), now_millis);
        var1
    }

    /// `getNBTTagCompoundWithPlayer(players)`: the fields plus `player`, the
    /// first player's `writeToNBT` compound (`None` without players; the
    /// stored `playerTag` is not written then).
    // src: WorldInfo.java:72-87
    pub fn get_nbt_tag_compound_with_player(
        &self,
        player: Option<Tag>,
        now_millis: i64,
    ) -> Compound {
        let mut var2 = Compound::new();
        self.update_tag_compound(&mut var2, player, now_millis);
        var2
    }

    /// `updateTagCompound(compound, playerTag)`. `LastPlayed` is the clock,
    /// not `lastTimePlayed`.
    // src: WorldInfo.java:89-107
    pub fn update_tag_compound(&self, c: &mut Compound, player: Option<Tag>, now_millis: i64) {
        n::set(c, "RandomSeed", Tag::Long(self.random_seed));
        n::set(c, "SpawnX", Tag::Int(self.spawn_x));
        n::set(c, "SpawnY", Tag::Int(self.spawn_y));
        n::set(c, "SpawnZ", Tag::Int(self.spawn_z));
        n::set(c, "Time", Tag::Long(self.world_time));
        n::set(c, "SizeOnDisk", Tag::Long(self.size_on_disk));
        n::set(c, "LastPlayed", Tag::Long(now_millis));
        n::set(c, "LevelName", Tag::String(self.level_name.clone()));
        n::set(c, "version", Tag::Int(self.save_version));
        n::set(c, "rainTime", Tag::Int(self.rain_time));
        n::set_boolean(c, "raining", self.raining);
        n::set(c, "thunderTime", Tag::Int(self.thunder_time));
        n::set_boolean(c, "thundering", self.thundering);
        if let Some(p) = player {
            n::set(c, "Player", p);
        }
    }

    /// `setSpawn(x, y, z)`.
    // src: WorldInfo.java:165-169
    pub fn set_spawn(&mut self, x: i32, y: i32, z: i32) {
        self.spawn_x = x;
        self.spawn_y = y;
        self.spawn_z = z;
    }
}
