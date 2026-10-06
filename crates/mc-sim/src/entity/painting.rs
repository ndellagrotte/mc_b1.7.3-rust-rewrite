//! `EntityPainting` (EntityPainting.java) and `EnumArt` (EnumArt.java): a
//! painting hung on a wall.
//!
//! Transcribed: the `(World)` and `(World, x, y, z, direction)`
//! constructors, `setDirection`, `getArtSize`, `onUpdate` (every 100 ticks
//! the wall is checked, `tickCounter1`), `onValidSurface`,
//! `canBeCollidedWith` (in [`super::dispatch::can_be_collided_with`]),
//! `attackEntityFrom`, `moveEntity`, `addVelocity`, the NBT bodies. Not
//! ported: the `(World, x, y, z, direction, title)` constructor
//! (`NetClientHandler` only, multiplayer, D3).
//!
//! The art choice of the position constructor draws `this.rand.nextInt(n)`
//! on the painting's own `rand` (EntityPainting.java:41), before the
//! painting joins the world (tools/oracle/census.md §3: no harness hook for
//! `ItemPainting`; the harness `spawn` op with a `direction` replays it on
//! the seeded `rand`, tests/golden/README.md). It is not a `Math.random()`
//! site.
//!
//! `art` is `null` after the `(World)` constructor until `readEntityFromNBT`
//! sets it; methods that read it would throw `NullPointerException` in
//! Java. The port records that and treats the painting as
//! artless where it can (module function docs).

use mc_core::math::floor_double;
use mc_nbt::Tag;

use super::item::new_entity_item;
use super::{Entity, EntityBase, EntityKindData, dispatch};
use crate::item::ItemStack;
use crate::save::SaveError;
use crate::save::nbt::{self as n, Compound};
use crate::world::World;

/// `EnumArt`: the 25 motives in declaration order (`values()` order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnumArt {
    /// `Kebab`, 16 x 16.
    Kebab,
    /// `Aztec`, 16 x 16.
    Aztec,
    /// `Alban`, 16 x 16.
    Alban,
    /// `Aztec2`, 16 x 16.
    Aztec2,
    /// `Bomb`, 16 x 16.
    Bomb,
    /// `Plant`, 16 x 16.
    Plant,
    /// `Wasteland`, 16 x 16.
    Wasteland,
    /// `Pool`, 32 x 16.
    Pool,
    /// `Courbet`, 32 x 16.
    Courbet,
    /// `Sea`, 32 x 16.
    Sea,
    /// `Sunset`, 32 x 16.
    Sunset,
    /// `Creebet`, 32 x 16.
    Creebet,
    /// `Wanderer`, 16 x 32.
    Wanderer,
    /// `Graham`, 16 x 32.
    Graham,
    /// `Match`, 32 x 32.
    Match,
    /// `Bust`, 32 x 32.
    Bust,
    /// `Stage`, 32 x 32.
    Stage,
    /// `Void`, 32 x 32.
    Void,
    /// `SkullAndRoses`, 32 x 32.
    SkullAndRoses,
    /// `Fighters`, 64 x 32.
    Fighters,
    /// `Pointer`, 64 x 64.
    Pointer,
    /// `Pigscene`, 64 x 64.
    Pigscene,
    /// `BurningSkull`, 64 x 64.
    BurningSkull,
    /// `Skeleton`, 64 x 48.
    Skeleton,
    /// `DonkeyKong`, 64 x 48.
    DonkeyKong,
}

/// One `EnumArt` constant's constructor arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArtDef {
    /// `title` (saved as `Motive`).
    pub title: &'static str,
    /// `sizeX` (pixels; 16 per block).
    pub size_x: i32,
    /// `sizeY`.
    pub size_y: i32,
    /// `offsetX` (texture, render only).
    pub offset_x: i32,
    /// `offsetY` (texture, render only).
    pub offset_y: i32,
}

const fn art(
    title: &'static str,
    size_x: i32,
    size_y: i32,
    offset_x: i32,
    offset_y: i32,
) -> ArtDef {
    ArtDef {
        title,
        size_x,
        size_y,
        offset_x,
        offset_y,
    }
}

impl EnumArt {
    /// `values()`.
    // src: EnumArt.java:4-28
    pub const VALUES: [EnumArt; 25] = [
        EnumArt::Kebab,
        EnumArt::Aztec,
        EnumArt::Alban,
        EnumArt::Aztec2,
        EnumArt::Bomb,
        EnumArt::Plant,
        EnumArt::Wasteland,
        EnumArt::Pool,
        EnumArt::Courbet,
        EnumArt::Sea,
        EnumArt::Sunset,
        EnumArt::Creebet,
        EnumArt::Wanderer,
        EnumArt::Graham,
        EnumArt::Match,
        EnumArt::Bust,
        EnumArt::Stage,
        EnumArt::Void,
        EnumArt::SkullAndRoses,
        EnumArt::Fighters,
        EnumArt::Pointer,
        EnumArt::Pigscene,
        EnumArt::BurningSkull,
        EnumArt::Skeleton,
        EnumArt::DonkeyKong,
    ];

    /// The constant's fields.
    // src: EnumArt.java:4-28
    pub const fn def(self) -> ArtDef {
        use EnumArt as A;
        match self {
            A::Kebab => art("Kebab", 16, 16, 0, 0),
            A::Aztec => art("Aztec", 16, 16, 16, 0),
            A::Alban => art("Alban", 16, 16, 32, 0),
            A::Aztec2 => art("Aztec2", 16, 16, 48, 0),
            A::Bomb => art("Bomb", 16, 16, 64, 0),
            A::Plant => art("Plant", 16, 16, 80, 0),
            A::Wasteland => art("Wasteland", 16, 16, 96, 0),
            A::Pool => art("Pool", 32, 16, 0, 32),
            A::Courbet => art("Courbet", 32, 16, 32, 32),
            A::Sea => art("Sea", 32, 16, 64, 32),
            A::Sunset => art("Sunset", 32, 16, 96, 32),
            A::Creebet => art("Creebet", 32, 16, 128, 32),
            A::Wanderer => art("Wanderer", 16, 32, 0, 64),
            A::Graham => art("Graham", 16, 32, 16, 64),
            A::Match => art("Match", 32, 32, 0, 128),
            A::Bust => art("Bust", 32, 32, 32, 128),
            A::Stage => art("Stage", 32, 32, 64, 128),
            A::Void => art("Void", 32, 32, 96, 128),
            A::SkullAndRoses => art("SkullAndRoses", 32, 32, 128, 128),
            A::Fighters => art("Fighters", 64, 32, 0, 96),
            A::Pointer => art("Pointer", 64, 64, 0, 192),
            A::Pigscene => art("Pigscene", 64, 64, 64, 192),
            A::BurningSkull => art("BurningSkull", 64, 64, 128, 192),
            A::Skeleton => art("Skeleton", 64, 48, 192, 64),
            A::DonkeyKong => art("DonkeyKong", 64, 48, 192, 112),
        }
    }

    /// `maxArtTitleLength`: `"SkullAndRoses".length()`.
    // src: EnumArt.java:30
    pub const MAX_ART_TITLE_LENGTH: usize = "SkullAndRoses".len();
}

/// The fields `EntityPainting` declares.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintingData {
    /// `tickCounter1` (private): the wall is checked when it reaches 100.
    // src: EntityPainting.java:7
    pub tick_counter1: i32,
    /// `direction`: 0 on the -z face of the wall block, 1 on -x, 2 on +z,
    /// 3 on +x (ItemPainting.java:14-25 maps the clicked side 2, 4, 3, 5).
    // src: EntityPainting.java:8
    pub direction: i32,
    /// `xPosition`: the wall block.
    // src: EntityPainting.java:9
    pub x_position: i32,
    /// `yPosition`.
    // src: EntityPainting.java:10
    pub y_position: i32,
    /// `zPosition`.
    // src: EntityPainting.java:11
    pub z_position: i32,
    /// `art` (`null` until set: module docs).
    // src: EntityPainting.java:12
    pub art: Option<EnumArt>,
}

/// `Item.painting.shiftedIndex` (`new ItemPainting(65)`, so 256 + 65).
// src: Item.java:339
const PAINTING_ITEM_ID: i32 = 256 + 65;

fn data(e: &mut Entity) -> &mut PaintingData {
    match &mut e.kind {
        EntityKindData::Painting(d) => d,
        _ => panic!("EntityPainting method on another kind"),
    }
}

fn data_ref(e: &Entity) -> &PaintingData {
    match &e.kind {
        EntityKindData::Painting(d) => d,
        _ => panic!("EntityPainting method on another kind"),
    }
}

/// `new EntityPainting(world)`: the `EntityList` / NBT-load constructor and
/// the harness `spawn` op's plain form: no art, direction 0, size 0.5.
// src: EntityPainting.java:14-20
pub fn new_entity_painting_empty(world: &mut World) -> Entity {
    let mut b = EntityBase::new(world);
    // entityInit() is empty (EntityPainting.java:66-67).
    let d = PaintingData::default();
    b.y_offset = 0.0;
    b.set_size(0.5, 0.5);
    Entity {
        base: b,
        kind: EntityKindData::Painting(d),
    }
}

/// `new EntityPainting(world, x, y, z, direction)` (ItemPainting.java:27):
/// every motive that fits the wall at block `(x, y, z)` facing `direction`
/// is a candidate (`onValidSurface` with that motive placed), and one is
/// drawn with `rand.nextInt(count)` (no draw when none fits: `art` is then
/// the last motive tried, `DonkeyKong`, and `ItemPainting` discards the
/// painting because `onValidSurface` fails).
// src: EntityPainting.java:22-45
pub fn new_entity_painting(world: &mut World, x: i32, y: i32, z: i32, direction: i32) -> Entity {
    let mut e = new_entity_painting_empty(world);
    {
        let d = data(&mut e);
        d.x_position = x;
        d.y_position = y;
        d.z_position = z;
    }
    let mut var6 = Vec::new();
    for var10 in EnumArt::VALUES {
        data(&mut e).art = Some(var10);
        set_direction(&mut e, world, direction);
        if on_valid_surface(&e, world) {
            var6.push(var10);
        }
    }
    if !var6.is_empty() {
        let i = e.base.rand.next_int_bounded(var6.len() as i32);
        data(&mut e).art = Some(var6[i as usize]);
    }
    set_direction(&mut e, world, direction);
    e
}

/// `getArtSize(pixels)` (private): half a block for 32 and 64, else 0.
// src: EntityPainting.java:126-132
#[allow(clippy::if_same_then_else)] // two branches with the same value, as in the Java
fn get_art_size(var1: i32) -> f32 {
    if var1 == 32 {
        0.5
    } else if var1 == 64 {
        0.5
    } else {
        0.0
    }
}

/// `setDirection(direction)`: the yaw (`direction * 90`), the centre in
/// front of the wall block and the box (the motive's size, half a pixel
/// thick, shrunk by 1/160 of a block on every side). With `art` null Java
/// throws `NullPointerException` after setting `direction` and the yaw:
/// recorded, nothing more is set.
// src: EntityPainting.java:69-124
pub fn set_direction(e: &mut Entity, world: &World, var1: i32) {
    let (art, xp, yp, zp) = {
        let d = data(e);
        d.direction = var1;
        (d.art, d.x_position, d.y_position, d.z_position)
    };
    let b = &mut e.base;
    b.rotation_yaw = (var1 * 90) as f32;
    b.prev_rotation_yaw = b.rotation_yaw;
    let Some(art) = art else {
        world.note_unimplemented(
            "EntityPainting.setDirection with a null art (Java NullPointerException)",
        );
        return;
    };
    let a = art.def();
    let mut var2 = a.size_x as f32;
    let mut var3 = a.size_y as f32;
    let mut var4 = a.size_x as f32;
    if var1 != 0 && var1 != 2 {
        var2 = 0.5;
    } else {
        var4 = 0.5;
    }
    var2 /= 32.0;
    var3 /= 32.0;
    var4 /= 32.0;
    let mut var5 = xp as f32 + 0.5;
    let mut var6 = yp as f32 + 0.5;
    let mut var7 = zp as f32 + 0.5;
    let var8: f32 = 0.5625;
    if var1 == 0 {
        var7 -= var8;
    }
    if var1 == 1 {
        var5 -= var8;
    }
    if var1 == 2 {
        var7 += var8;
    }
    if var1 == 3 {
        var5 += var8;
    }
    if var1 == 0 {
        var5 -= get_art_size(a.size_x);
    }
    if var1 == 1 {
        var7 += get_art_size(a.size_x);
    }
    if var1 == 2 {
        var5 += get_art_size(a.size_x);
    }
    if var1 == 3 {
        var7 -= get_art_size(a.size_x);
    }
    var6 += get_art_size(a.size_y);
    b.set_position(var5 as f64, var6 as f64, var7 as f64);
    let var9: f32 = -0.00625;
    b.bounding_box.set_bounds(
        (var5 - var2 - var9) as f64,
        (var6 - var3 - var9) as f64,
        (var7 - var4 - var9) as f64,
        (var5 + var2 + var9) as f64,
        (var6 + var3 + var9) as f64,
        (var7 + var4 + var9) as f64,
    );
}

/// `EntityPainting.onUpdate()`: no `super.onUpdate()`; on the 101st update
/// (and every 101 after: the post-increment compares the old value, then
/// the counter is reset to 0) a painting whose wall no longer holds it dies
/// and drops a painting item.
// src: EntityPainting.java:134-143
pub fn on_update(e: &mut Entity, world: &mut World) {
    let d = data(e);
    let was = d.tick_counter1;
    d.tick_counter1 += 1;
    if was == 100 && !world.multiplayer_world {
        data(e).tick_counter1 = 0;
        if !on_valid_surface(e, world) {
            dispatch::set_entity_dead(e, world);
            drop_painting_item(e, world);
        }
    }
}

/// `worldObj.entityJoinedWorld(new EntityItem(worldObj, posX, posY, posZ,
/// new ItemStack(Item.painting)))`: the item, with no pick-up delay.
// src: EntityPainting.java:139, :208, :248, :256
fn drop_painting_item(e: &Entity, world: &mut World) {
    let item = new_entity_item(
        world,
        e.base.pos_x,
        e.base.pos_y,
        e.base.pos_z,
        ItemStack::new(PAINTING_ITEM_ID, 1, 0),
    );
    world.spawn_entity_in_world(item);
}

/// `onValidSurface()`: nothing solid inside the painting's box, every block
/// behind it of a solid material, and no other painting overlapping it.
/// With `art` null the Java throws once the box is clear (recorded, false).
// src: EntityPainting.java:145-198
pub fn on_valid_surface(e: &Entity, world: &mut World) -> bool {
    let bb = e.base.bounding_box;
    if !world.get_colliding_bounding_boxes(e, &bb).is_empty() {
        return false;
    }
    let d = data_ref(e);
    let Some(art) = d.art else {
        world.note_unimplemented(
            "EntityPainting.onValidSurface with a null art (Java NullPointerException)",
        );
        return false;
    };
    let a = art.def();
    let var1 = a.size_x / 16;
    let var2 = a.size_y / 16;
    let mut var3 = d.x_position;
    let mut var5 = d.z_position;
    let b = &e.base;
    if d.direction == 0 {
        var3 = floor_double(b.pos_x - (a.size_x as f32 / 32.0) as f64);
    }
    if d.direction == 1 {
        var5 = floor_double(b.pos_z - (a.size_x as f32 / 32.0) as f64);
    }
    if d.direction == 2 {
        var3 = floor_double(b.pos_x - (a.size_x as f32 / 32.0) as f64);
    }
    if d.direction == 3 {
        var5 = floor_double(b.pos_z - (a.size_x as f32 / 32.0) as f64);
    }
    let var4 = floor_double(b.pos_y - (a.size_y as f32 / 32.0) as f64);
    for var6 in 0..var1 {
        for var7 in 0..var2 {
            let var8 = if d.direction != 0 && d.direction != 2 {
                world.get_block_material(d.x_position, var4 + var7, var5 + var6)
            } else {
                world.get_block_material(var3 + var6, var4 + var7, d.z_position)
            };
            if !var8.is_solid() {
                return false;
            }
        }
    }
    let var9 = world.get_entities_within_aabb_excluding_entity(Some(b.entity_id), &bb);
    for id in var9 {
        if let Some(o) = world.entity(id)
            && matches!(o.kind, EntityKindData::Painting(..))
        {
            return false;
        }
    }
    true
}

/// `EntityPainting.attackEntityFrom(attacker, damage)`: any hit takes it
/// down (dead, attacked, a painting item); always true.
// src: EntityPainting.java:204-212
pub fn attack_entity_from(
    e: &mut Entity,
    world: &mut World,
    _attacker: Option<&Entity>,
    _damage: i32,
) -> bool {
    if !e.base.is_dead && !world.multiplayer_world {
        dispatch::set_entity_dead(e, world);
        e.base.set_been_attacked();
        drop_painting_item(e, world);
    }
    true
}

/// `EntityPainting.moveEntity(dx, dy, dz)`: it never moves; any non-zero
/// move takes it down.
// src: EntityPainting.java:245-251
pub fn move_entity(e: &mut Entity, world: &mut World, var1: f64, var3: f64, var5: f64) {
    if !world.multiplayer_world && var1 * var1 + var3 * var3 + var5 * var5 > 0.0 {
        dispatch::set_entity_dead(e, world);
        drop_painting_item(e, world);
    }
}

/// `EntityPainting.addVelocity(dx, dy, dz)`: as `moveEntity`.
// src: EntityPainting.java:253-259
pub fn add_velocity(e: &mut Entity, world: &mut World, var1: f64, var3: f64, var5: f64) {
    if !world.multiplayer_world && var1 * var1 + var3 * var3 + var5 * var5 > 0.0 {
        dispatch::set_entity_dead(e, world);
        drop_painting_item(e, world);
    }
}

/// `EntityPainting.writeEntityToNBT(compound)`. With `art` null Java throws
/// (`art.title`): recorded, `Motive` left out.
// src: EntityPainting.java:214-220
pub fn write_entity_to_nbt(d: &PaintingData, c: &mut Compound, world: &World) {
    n::set(c, "Dir", Tag::Byte(d.direction as i8));
    match d.art {
        Some(a) => n::set(c, "Motive", Tag::String(a.def().title.to_string())),
        None => world.note_unimplemented(
            "EntityPainting.writeEntityToNBT with a null art (Java NullPointerException)",
        ),
    }
    n::set(c, "TileX", Tag::Int(d.x_position));
    n::set(c, "TileY", Tag::Int(d.y_position));
    n::set(c, "TileZ", Tag::Int(d.z_position));
}

/// `EntityPainting.readEntityFromNBT(compound)`: the motive by title (the
/// last match; `Kebab` when none), then `setDirection`.
// src: EntityPainting.java:222-243
pub fn read_entity_from_nbt(e: &mut Entity, c: &Compound, world: &World) -> Result<(), SaveError> {
    let direction = n::get_byte(c, "Dir")? as i32;
    let x = n::get_integer(c, "TileX")?;
    let y = n::get_integer(c, "TileY")?;
    let z = n::get_integer(c, "TileZ")?;
    let var2 = n::get_string(c, "Motive")?;
    let d = data(e);
    d.direction = direction;
    d.x_position = x;
    d.y_position = y;
    d.z_position = z;
    for var6 in EnumArt::VALUES {
        if var6.def().title == var2 {
            d.art = Some(var6);
        }
    }
    if d.art.is_none() {
        d.art = Some(EnumArt::Kebab);
    }
    let dir = d.direction;
    set_direction(e, world, dir);
    Ok(())
}
