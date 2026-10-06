//! `BlockRail` (BlockRail.java) and its helper `RailLogic` (RailLogic.java):
//! the rail (id 66), the powered rail (id 27, `isPowered` true) and, as the
//! superclass of `BlockDetectorRail` ([`super::detector_rail`], id 28,
//! `isPowered` true), the shape and support rules of the detector rail.
//!
//! Fully transcribed: `onBlockAdded`, `onNeighborBlockChange`,
//! `refreshTrackShape`, `isNeighborRailPowered`, `isRailPassingPower` (the
//! powered rail's 8-rail propagation), `canPlaceBlockAt`,
//! `setBlockBoundsBasedOnState`, the bounds half of `collisionRayTrace`
//! (the dispatch arm then runs `Block.collisionRayTrace`), the `null`
//! `getCollisionBoundingBoxFromPool` (in the dispatch arm), and the public
//! statics `isRailBlockAt`, `isRailBlock` and `getIsPowered` that
//! `EntityMinecart` (WP-2.4) and `ItemMinecart` call. The data methods
//! (`isOpaqueCube`, `renderAsNormalBlock`, `getRenderType`, `quantityDropped`,
//! `getMobilityFlag`, `getBlockTextureFromSideAndMetadata`) are in
//! [`crate::block`].
//!
//! `RailLogic` is [`RailLogic`] with the world passed to each method instead
//! of held (`worldObj`). Its synthetic `rail` field (the `BlockRail` that
//! built it, RailLogic.java:14) is only ever passed on to the `RailLogic`s it
//! builds and never read: whether a track is a powered one is decided by the
//! block at the track's own position (RailLogic.java:23-30), so it is not
//! ported. `connectedTracks` is an `ArrayList` (RailLogic.java:18): a `Vec`.

use mc_core::java_hash::ChunkPosition;

use super::{BlockBounds, BlockKind, block, dispatch};
use crate::world::World;

/// `Block.rail.blockID`.
// src: Block.java:658
pub const RAIL: i32 = 66;
/// `Block.railPowered.blockID`.
// src: Block.java:619
pub const RAIL_POWERED: i32 = 27;
/// `Block.railDetector.blockID`.
// src: Block.java:620
pub const RAIL_DETECTOR: i32 = 28;

/// `BlockRail.isRailBlockAt(world, x, y, z)` (public static): the block there
/// is one of the three rails. Loads the chunk (`getBlockId`).
// src: BlockRail.java:8-11
pub fn is_rail_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    let var4 = world.get_block_id(x, y, z);
    var4 == RAIL || var4 == RAIL_POWERED || var4 == RAIL_DETECTOR
}

/// `BlockRail.isRailBlock(id)` (public static).
// src: BlockRail.java:13-15
pub const fn is_rail_block(id: i32) -> bool {
    id == RAIL || id == RAIL_POWERED || id == RAIL_DETECTOR
}

/// `((BlockRail) Block.blocksList[id]).getIsPowered()`: `isPowered` of the
/// rail with this id (true for the powered and the detector rail). Every
/// vanilla caller tests [`is_rail_block`] first (EntityMinecart.java:213-224,
/// :506-510, :546-549); for any other id Java throws `ClassCastException`
/// (or `NullPointerException`), here the answer is false.
// src: BlockRail.java:23-25
pub fn get_is_powered(id: i32) -> bool {
    is_powered(id)
}

/// `this.isPowered` of the rail singleton with this id (BlockRail.java:6,
/// :19; BlockDetectorRail.java:8 passes `true`); false for a non-rail id.
fn is_powered(id: i32) -> bool {
    block(id)
        .and_then(|b| b.kind.rail_is_powered())
        .unwrap_or(false)
}

/// `BlockRail.canPlaceBlockAt(world, x, y, z)`: a normal cube below. Unlike
/// `Block.canPlaceBlockAt` it does not look at the target block itself.
// src: BlockRail.java:74-76
pub fn can_place_block_at(world: &mut World, x: i32, y: i32, z: i32) -> bool {
    world.is_block_normal_cube(x, y - 1, z)
}

/// `BlockRail.setBlockBoundsBasedOnState(blockAccess, x, y, z)`: a sloped
/// rail (meta 2..=5) is 0.625 high, any other 0.125. Writes the singleton's
/// bounds ([`crate::block::BlockStatics::bounds`]).
// src: BlockRail.java:40-48
pub fn set_block_bounds_based_on_state(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    // vanilla bug: the raw metadata is tested, without the `& 7` that
    // onNeighborBlockChange applies for the powered and detector rails
    // (BlockRail.java:89-91), so a powered rail's or a detector rail's slope
    // with bit 3 set (meta 10..=13) gets the flat 0.125 height.
    let var5 = world.get_block_metadata(x, y, z);
    world.block_statics.bounds[(id & 255) as usize] = if (2..=5).contains(&var5) {
        BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.625, 1.0)
    } else {
        BlockBounds::from_floats(0.0, 0.0, 0.0, 1.0, 0.125, 1.0)
    };
}

/// `BlockRail.onBlockAdded(world, x, y, z)`: shapes the new rail and
/// reconnects its neighbours, always writing the metadata (`force`).
// src: BlockRail.java:78-83
pub fn on_block_added(world: &mut World, id: i32, x: i32, y: i32, z: i32) {
    if !world.multiplayer_world {
        refresh_track_shape(world, id, x, y, z, true);
    }
}

/// `BlockRail.onNeighborBlockChange(world, x, y, z, neighborId)`: drops the
/// rail when it lost its support (the block below, or for a slope the block
/// it climbs onto); otherwise a powered rail recomputes its power bit, and a
/// plain rail at a junction of three tracks reshapes itself when the
/// neighbour that changed is a power source.
// src: BlockRail.java:85-140
pub fn on_neighbor_block_change(
    world: &mut World,
    id: i32,
    x: i32,
    y: i32,
    z: i32,
    neighbor_id: i32,
) {
    if world.multiplayer_world {
        return;
    }
    let this_is_powered = is_powered(id);
    let var6 = world.get_block_metadata(x, y, z);
    let mut var7 = var6;
    if this_is_powered {
        var7 = var6 & 7;
    }

    // src: BlockRail.java:93-112
    let mut var8 = false;
    if !world.is_block_normal_cube(x, y - 1, z) {
        var8 = true;
    }
    if var7 == 2 && !world.is_block_normal_cube(x + 1, y, z) {
        var8 = true;
    }
    if var7 == 3 && !world.is_block_normal_cube(x - 1, y, z) {
        var8 = true;
    }
    if var7 == 4 && !world.is_block_normal_cube(x, y, z - 1) {
        var8 = true;
    }
    if var7 == 5 && !world.is_block_normal_cube(x, y, z + 1) {
        var8 = true;
    }

    if var8 {
        // src: BlockRail.java:114-116
        let meta = world.get_block_metadata(x, y, z);
        dispatch::drop_block_as_item(world, id, x, y, z, meta);
        world.set_block_with_notify(x, y, z, 0);
    } else if id == RAIL_POWERED {
        // src: BlockRail.java:117-134 (short-circuit `||` order kept)
        let mut var9 = world.is_block_indirectly_getting_powered(x, y, z)
            || world.is_block_indirectly_getting_powered(x, y + 1, z);
        var9 = var9
            || is_neighbor_rail_powered(world, x, y, z, var6, true, 0)
            || is_neighbor_rail_powered(world, x, y, z, var6, false, 0);
        let mut var10 = false;
        if var9 && (var6 & 8) == 0 {
            world.set_block_metadata_with_notify(x, y, z, var7 | 8);
            var10 = true;
        } else if !var9 && (var6 & 8) != 0 {
            world.set_block_metadata_with_notify(x, y, z, var7);
            var10 = true;
        }

        if var10 {
            world.notify_blocks_of_neighbor_change(x, y - 1, z, id);
            if var7 == 2 || var7 == 3 || var7 == 4 || var7 == 5 {
                world.notify_blocks_of_neighbor_change(x, y + 1, z, id);
            }
        }
    } else if neighbor_id > 0
        && dispatch::can_provide_power(world, neighbor_id)
        && !this_is_powered
        && RailLogic::new(world, x, y, z).get_adjacent_tracks(world) == 3
    {
        // src: BlockRail.java:135-137 (`RailLogic.getNAdjacentTracks`, the
        // synthetic accessor of the private `getAdjacentTracks`)
        refresh_track_shape(world, id, x, y, z, false);
    }
}

/// `BlockRail.refreshTrackShape(world, x, y, z, force)` (private). The
/// `RailLogic` is built before the power query runs, as Java evaluates the
/// receiver before the argument.
// src: BlockRail.java:142-146
fn refresh_track_shape(world: &mut World, id: i32, x: i32, y: i32, z: i32, force: bool) {
    let _ = id;
    if !world.multiplayer_world {
        let mut logic = RailLogic::new(world, x, y, z);
        let powered = world.is_block_indirectly_getting_powered(x, y, z);
        logic.refresh_track_shape(world, powered, force);
    }
}

/// `BlockRail.isNeighborRailPowered(world, x, y, z, meta, forward, depth)`
/// (private): whether the powered rail one step along the track (`forward`
/// picks the direction; a slope also looks one up on its rising side) passes
/// power to this one, at most 8 rails deep.
// src: BlockRail.java:148-220
#[allow(clippy::too_many_arguments)]
fn is_neighbor_rail_powered(
    world: &mut World,
    mut x: i32,
    mut y: i32,
    mut z: i32,
    meta: i32,
    forward: bool,
    depth: i32,
) -> bool {
    if depth >= 8 {
        return false;
    }
    let mut var8 = meta & 7;
    let mut var9 = true;
    match var8 {
        0 => {
            if forward {
                z += 1;
            } else {
                z -= 1;
            }
        }
        1 => {
            if forward {
                x -= 1;
            } else {
                x += 1;
            }
        }
        2 => {
            if forward {
                x -= 1;
            } else {
                x += 1;
                y += 1;
                var9 = false;
            }
            var8 = 1;
        }
        3 => {
            if forward {
                x -= 1;
                y += 1;
                var9 = false;
            } else {
                x += 1;
            }
            var8 = 1;
        }
        4 => {
            if forward {
                z += 1;
            } else {
                z -= 1;
                y += 1;
                var9 = false;
            }
            var8 = 0;
        }
        5 => {
            if forward {
                z += 1;
                y += 1;
                var9 = false;
            } else {
                z -= 1;
            }
            var8 = 0;
        }
        // src: BlockRail.java:154 (no default: shapes 6..=7 keep x, y, z)
        _ => {}
    }

    // src: BlockRail.java:214-218
    if is_rail_passing_power(world, x, y, z, forward, depth, var8) {
        true
    } else {
        var9 && is_rail_passing_power(world, x, y - 1, z, forward, depth, var8)
    }
}

/// `BlockRail.isRailPassingPower(world, x, y, z, forward, depth, axis)`
/// (private): a powered rail along the same axis (`axis` 0 = north-south,
/// 1 = east-west) that is on and either powered itself or fed by the next
/// rail along.
// src: BlockRail.java:222-245
#[allow(clippy::too_many_arguments)]
fn is_rail_passing_power(
    world: &mut World,
    x: i32,
    y: i32,
    z: i32,
    forward: bool,
    depth: i32,
    axis: i32,
) -> bool {
    let var8 = world.get_block_id(x, y, z);
    if var8 == RAIL_POWERED {
        let var9 = world.get_block_metadata(x, y, z);
        let var10 = var9 & 7;
        if axis == 1 && (var10 == 0 || var10 == 4 || var10 == 5) {
            return false;
        }
        if axis == 0 && (var10 == 1 || var10 == 2 || var10 == 3) {
            return false;
        }
        if (var9 & 8) != 0 {
            if !world.is_block_indirectly_getting_powered(x, y, z)
                && !world.is_block_indirectly_getting_powered(x, y + 1, z)
            {
                return is_neighbor_rail_powered(world, x, y, z, var9, forward, depth + 1);
            }
            return true;
        }
    }
    false
}

/// `RailLogic`: a rail position with the two track positions its shape
/// connects to (see the module docs for what is not ported).
#[derive(Clone, Debug)]
pub struct RailLogic {
    /// `trackX`.
    track_x: i32,
    /// `trackY`.
    track_y: i32,
    /// `trackZ`.
    track_z: i32,
    /// `isPoweredRail`: the block at the position is a powered or detector
    /// rail (`BlockRail.isPowered`), whose shape bit 3 is the power flag and
    /// which never curves.
    is_powered_rail: bool,
    /// `connectedTracks`.
    connected_tracks: Vec<ChunkPosition>,
}

impl RailLogic {
    /// `new RailLogic(rail, world, x, y, z)`: reads the block there and its
    /// shape (metadata, without the power bit for a powered rail). Every
    /// vanilla call site has a rail at `(x, y, z)` (Java would throw on any
    /// other block, RailLogic.java:25); here a non-rail counts as unpowered.
    // src: RailLogic.java:16-33
    pub fn new(world: &mut World, x: i32, y: i32, z: i32) -> Self {
        let var6 = world.get_block_id(x, y, z);
        let mut var7 = world.get_block_metadata(x, y, z);
        // src: RailLogic.java:25 (`BlockRail.isPoweredBlockRail`, the synthetic
        // accessor of `isPowered`, BlockRail.java:251-254)
        let is_powered_rail = matches!(
            block(var6).map(|b| b.kind),
            Some(BlockKind::Rail { is_powered: true } | BlockKind::DetectorRail)
        );
        if is_powered_rail {
            var7 &= -9;
        }
        let mut l = RailLogic {
            track_x: x,
            track_y: y,
            track_z: z,
            is_powered_rail,
            connected_tracks: Vec::new(),
        };
        l.set_connections(var7);
        l
    }

    /// `setConnections(shape)` (private): the two positions a shape joins
    /// (0 north-south, 1 east-west, 2..=5 slopes rising to +x, -x, -z, +z,
    /// 6..=9 curves). Any other value leaves the list empty.
    // src: RailLogic.java:35-69
    fn set_connections(&mut self, var1: i32) {
        let (x, y, z) = (self.track_x, self.track_y, self.track_z);
        let p = ChunkPosition::new;
        self.connected_tracks.clear();
        let pair = match var1 {
            0 => Some((p(x, y, z - 1), p(x, y, z + 1))),
            1 => Some((p(x - 1, y, z), p(x + 1, y, z))),
            2 => Some((p(x - 1, y, z), p(x + 1, y + 1, z))),
            3 => Some((p(x - 1, y + 1, z), p(x + 1, y, z))),
            4 => Some((p(x, y + 1, z - 1), p(x, y, z + 1))),
            5 => Some((p(x, y, z - 1), p(x, y + 1, z + 1))),
            6 => Some((p(x + 1, y, z), p(x, y, z + 1))),
            7 => Some((p(x - 1, y, z), p(x, y, z + 1))),
            8 => Some((p(x - 1, y, z), p(x, y, z - 1))),
            9 => Some((p(x + 1, y, z), p(x, y, z - 1))),
            _ => None,
        };
        if let Some((a, b)) = pair {
            self.connected_tracks.push(a);
            self.connected_tracks.push(b);
        }
    }

    /// `func_785_b()` (private), "refresh connected tracks": replaces each
    /// connected position by the actual rail found there (same y, one up or
    /// one down) when that rail connects back, and drops the others.
    // src: RailLogic.java:71-81
    fn refresh_connected_tracks(&mut self, world: &mut World) {
        let mut var1 = 0;
        while var1 < self.connected_tracks.len() {
            let var2 = self.get_minecart_track_logic(world, self.connected_tracks[var1]);
            match var2 {
                Some(var2) if var2.is_connected_to(self) => {
                    self.connected_tracks[var1] =
                        ChunkPosition::new(var2.track_x, var2.track_y, var2.track_z);
                    var1 += 1;
                }
                // src: RailLogic.java:77 (`remove(var1--)`, then `++var1`)
                _ => {
                    self.connected_tracks.remove(var1);
                }
            }
        }
    }

    /// `isMinecartTrack(x, y, z)` (private): a rail at that position, one
    /// above or one below.
    // src: RailLogic.java:83-91
    fn is_minecart_track(&self, world: &mut World, x: i32, y: i32, z: i32) -> bool {
        is_rail_block_at(world, x, y, z)
            || is_rail_block_at(world, x, y + 1, z)
            || is_rail_block_at(world, x, y - 1, z)
    }

    /// `getMinecartTrackLogic(pos)` (private): the `RailLogic` of the rail at
    /// `pos`, one above or one below (first found), or `None`.
    // src: RailLogic.java:93-101
    fn get_minecart_track_logic(&self, world: &mut World, var1: ChunkPosition) -> Option<Self> {
        if is_rail_block_at(world, var1.x, var1.y, var1.z) {
            Some(RailLogic::new(world, var1.x, var1.y, var1.z))
        } else if is_rail_block_at(world, var1.x, var1.y + 1, var1.z) {
            Some(RailLogic::new(world, var1.x, var1.y + 1, var1.z))
        } else if is_rail_block_at(world, var1.x, var1.y - 1, var1.z) {
            Some(RailLogic::new(world, var1.x, var1.y - 1, var1.z))
        } else {
            None
        }
    }

    /// `isConnectedTo(other)` (private): one of the connected positions has
    /// `other`'s x and z (y is ignored).
    // src: RailLogic.java:103-112
    fn is_connected_to(&self, var1: &RailLogic) -> bool {
        self.connected_tracks
            .iter()
            .any(|var3| var3.x == var1.track_x && var3.z == var1.track_z)
    }

    /// `isInTrack(x, y, z)` (private): one of the connected positions has
    /// this x and z (y is ignored).
    // src: RailLogic.java:114-123
    fn is_in_track(&self, var1: i32, var2: i32, var3: i32) -> bool {
        let _ = var2;
        self.connected_tracks
            .iter()
            .any(|var5| var5.x == var1 && var5.z == var3)
    }

    /// `getAdjacentTracks()` (private; `BlockRail` calls it through the
    /// synthetic `getNAdjacentTracks`, RailLogic.java:358-361): how many of
    /// the four horizontal neighbours hold a track ([`Self::is_minecart_track`]),
    /// in the order -z, +z, -x, +x.
    // src: RailLogic.java:125-144
    pub fn get_adjacent_tracks(&self, world: &mut World) -> i32 {
        let (x, y, z) = (self.track_x, self.track_y, self.track_z);
        let mut var1 = 0;
        if self.is_minecart_track(world, x, y, z - 1) {
            var1 += 1;
        }
        if self.is_minecart_track(world, x, y, z + 1) {
            var1 += 1;
        }
        if self.is_minecart_track(world, x - 1, y, z) {
            var1 += 1;
        }
        if self.is_minecart_track(world, x + 1, y, z) {
            var1 += 1;
        }
        var1
    }

    /// `handleKeyPress(other)` (private; an MCP misnomer, later versions'
    /// `canConnectTo`): whether this track can take `other` as a
    /// connection: already connected, or not yet connected at both ends.
    // src: RailLogic.java:146-157
    fn handle_key_press(&self, var1: &RailLogic) -> bool {
        if self.is_connected_to(var1) {
            true
        } else if self.connected_tracks.len() == 2 {
            false
        } else if self.connected_tracks.is_empty() {
            true
        } else {
            // vanilla bug: RailLogic.java:154-155 computes
            // `var1.trackY == trackY && var2.y == trackY ? true : true`: the
            // test (and the read of the first connection) decides nothing.
            let var2 = self.connected_tracks[0];
            let _ = var1.track_y == self.track_y && var2.y == self.track_y;
            true
        }
    }

    /// `func_788_d(other)` (private), "connect to neighbour": adds `other` as
    /// a connection and rewrites this rail's metadata to the shape its
    /// connections now make (a curve only for a plain rail; a slope when a
    /// rail sits one up at an end), keeping a powered rail's power bit.
    // src: RailLogic.java:159-222
    fn connect_to_neighbor(&mut self, world: &mut World, var1: &RailLogic) {
        let (x, y, z) = (self.track_x, self.track_y, self.track_z);
        self.connected_tracks
            .push(ChunkPosition::new(var1.track_x, var1.track_y, var1.track_z));
        let var2 = self.is_in_track(x, y, z - 1);
        let var3 = self.is_in_track(x, y, z + 1);
        let var4 = self.is_in_track(x - 1, y, z);
        let var5 = self.is_in_track(x + 1, y, z);
        let mut var6: i32 = -1;
        if var2 || var3 {
            var6 = 0;
        }
        if var4 || var5 {
            var6 = 1;
        }

        if !self.is_powered_rail {
            if var3 && var5 && !var2 && !var4 {
                var6 = 6;
            }
            if var3 && var4 && !var2 && !var5 {
                var6 = 7;
            }
            if var2 && var4 && !var3 && !var5 {
                var6 = 8;
            }
            if var2 && var5 && !var3 && !var4 {
                var6 = 9;
            }
        }

        if var6 == 0 {
            if is_rail_block_at(world, x, y + 1, z - 1) {
                var6 = 4;
            }
            if is_rail_block_at(world, x, y + 1, z + 1) {
                var6 = 5;
            }
        }

        if var6 == 1 {
            if is_rail_block_at(world, x + 1, y + 1, z) {
                var6 = 2;
            }
            if is_rail_block_at(world, x - 1, y + 1, z) {
                var6 = 3;
            }
        }

        if var6 < 0 {
            var6 = 0;
        }

        let mut var7 = var6;
        if self.is_powered_rail {
            var7 = (world.get_block_metadata(x, y, z) & 8) | var6;
        }
        world.set_block_metadata_with_notify(x, y, z, var7);
    }

    /// `func_786_c(x, y, z)` (private), "can connect from": the rail at (or
    /// one above or below) that position, after refreshing its connections,
    /// would accept this rail as a connection.
    // src: RailLogic.java:224-232
    fn can_connect_from(&self, world: &mut World, var1: i32, var2: i32, var3: i32) -> bool {
        match self.get_minecart_track_logic(world, ChunkPosition::new(var1, var2, var3)) {
            None => false,
            Some(mut var4) => {
                var4.refresh_connected_tracks(world);
                var4.handle_key_press(self)
            }
        }
    }

    /// `refreshTrackShape(powered, force)`: picks this rail's shape from the
    /// neighbours that would connect (curves only for a plain rail, their
    /// preference order depending on `powered`; slopes toward a rail one up),
    /// writes it when it changed or `force`, and then lets each connected
    /// neighbour that accepts this rail reshape itself toward it.
    // src: RailLogic.java:234-356
    pub fn refresh_track_shape(&mut self, world: &mut World, var1: bool, var2: bool) {
        let (x, y, z) = (self.track_x, self.track_y, self.track_z);
        let var3 = self.can_connect_from(world, x, y, z - 1);
        let var4 = self.can_connect_from(world, x, y, z + 1);
        let var5 = self.can_connect_from(world, x - 1, y, z);
        let var6 = self.can_connect_from(world, x + 1, y, z);
        let mut var7: i32 = -1;
        if (var3 || var4) && !var5 && !var6 {
            var7 = 0;
        }
        if (var5 || var6) && !var3 && !var4 {
            var7 = 1;
        }

        if !self.is_powered_rail {
            if var4 && var6 && !var3 && !var5 {
                var7 = 6;
            }
            if var4 && var5 && !var3 && !var6 {
                var7 = 7;
            }
            if var3 && var5 && !var4 && !var6 {
                var7 = 8;
            }
            if var3 && var6 && !var4 && !var5 {
                var7 = 9;
            }
        }

        // src: RailLogic.java:266-310 (three or four candidates: straight
        // first, then the curves in an order that depends on `powered`)
        if var7 == -1 {
            if var3 || var4 {
                var7 = 0;
            }
            if var5 || var6 {
                var7 = 1;
            }

            if !self.is_powered_rail {
                if var1 {
                    if var4 && var6 {
                        var7 = 6;
                    }
                    if var5 && var4 {
                        var7 = 7;
                    }
                    if var6 && var3 {
                        var7 = 9;
                    }
                    if var3 && var5 {
                        var7 = 8;
                    }
                } else {
                    if var3 && var5 {
                        var7 = 8;
                    }
                    if var6 && var3 {
                        var7 = 9;
                    }
                    if var5 && var4 {
                        var7 = 7;
                    }
                    if var4 && var6 {
                        var7 = 6;
                    }
                }
            }
        }

        if var7 == 0 {
            if is_rail_block_at(world, x, y + 1, z - 1) {
                var7 = 4;
            }
            if is_rail_block_at(world, x, y + 1, z + 1) {
                var7 = 5;
            }
        }

        if var7 == 1 {
            if is_rail_block_at(world, x + 1, y + 1, z) {
                var7 = 2;
            }
            if is_rail_block_at(world, x - 1, y + 1, z) {
                var7 = 3;
            }
        }

        if var7 < 0 {
            var7 = 0;
        }

        self.set_connections(var7);
        let mut var8 = var7;
        if self.is_powered_rail {
            var8 = (world.get_block_metadata(x, y, z) & 8) | var7;
        }

        // src: RailLogic.java:342-354 (`connectedTracks.size()` is re-read
        // each pass; nothing in the loop changes this rail's list)
        if var2 || world.get_block_metadata(x, y, z) != var8 {
            world.set_block_metadata_with_notify(x, y, z, var8);

            let mut var9 = 0;
            while var9 < self.connected_tracks.len() {
                let var10 = self.get_minecart_track_logic(world, self.connected_tracks[var9]);
                if let Some(mut var10) = var10 {
                    var10.refresh_connected_tracks(world);
                    if var10.handle_key_press(self) {
                        var10.connect_to_neighbor(world, self);
                    }
                }
                var9 += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rail_ids_and_powered_flags() {
        assert!(is_rail_block(RAIL) && is_rail_block(RAIL_POWERED) && is_rail_block(RAIL_DETECTOR));
        assert!(!is_rail_block(0) && !is_rail_block(1));
        assert!(!get_is_powered(RAIL));
        assert!(get_is_powered(RAIL_POWERED));
        assert!(get_is_powered(RAIL_DETECTOR));
        assert!(!get_is_powered(1));
        assert_eq!(
            block(RAIL).map(|b| b.kind),
            Some(BlockKind::Rail { is_powered: false })
        );
        assert_eq!(
            block(RAIL_DETECTOR).map(|b| b.kind),
            Some(BlockKind::DetectorRail)
        );
    }
}
