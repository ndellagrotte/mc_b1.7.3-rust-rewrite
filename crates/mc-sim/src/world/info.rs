//! `WorldInfo` (WorldInfo.java): the `level.dat` fields.
//!
//! NBT reading and writing (`WorldInfo(NBTTagCompound)`, `updateTagCompound`,
//! WorldInfo.java:22-41, :89-107) belong to the save work (WP-1.9) and are not
//! here yet.

use mc_nbt::Tag;

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

    /// `setSpawn(x, y, z)`.
    // src: WorldInfo.java:165-169
    pub fn set_spawn(&mut self, x: i32, y: i32, z: i32) {
        self.spawn_x = x;
        self.spawn_y = y;
        self.spawn_z = z;
    }
}
