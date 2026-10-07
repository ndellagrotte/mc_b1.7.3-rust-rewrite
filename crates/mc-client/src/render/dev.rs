//! The `spawn_entity` dev action (project-owned, not vanilla; WP-3.5b's
//! smoke test): puts an entity into the world as the simulation would
//! spawn it (`EntityList.createEntityInWorld`, or the class's own
//! constructor for an item, a painting, a minecart type, a lightning bolt,
//! a snowball, an egg), with a few render-visible fields set.

use mc_sim::entity::data_watcher::WatchValue;
use mc_sim::entity::{
    Entity, EntityKindData as K, egg, fireball, item as eitem, lightning_bolt, minecart, painting,
    pig, sheep, slime, snowball, wolf,
};
use mc_sim::item::ItemStack;
use mc_sim::save::entity_list;
use serde::Deserialize;

use crate::game::Game;

/// A `spawn_entity` request (the script action's fields).
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct SpawnEntitySpec {
    /// The `EntityList` name (`"Pig"`, `"Sheep"`, `"Creeper"`, `"Skeleton"`,
    /// `"Arrow"`, `"Item"`, `"Minecart"`, `"Boat"`, `"PrimedTnt"`,
    /// `"FallingSand"`, `"Painting"`, …) or `"LightningBolt"`,
    /// `"Fireball"`, `"Egg"`.
    pub kind: String,
    /// x (or the offset with `ahead`).
    #[serde(default)]
    pub x: f64,
    /// y (feet).
    #[serde(default)]
    pub y: f64,
    /// z.
    #[serde(default)]
    pub z: f64,
    /// Blocks ahead of the player, horizontally, at its feet: `x, y, z`
    /// are offsets from there.
    #[serde(default)]
    pub ahead: Option<f64>,
    /// `rotationYaw` (and the previous yaw, the body yaw).
    #[serde(default)]
    pub yaw: f32,
    /// `rotationPitch`.
    #[serde(default)]
    pub pitch: f32,
    /// The item stack of an `"Item"`: `[id, count, damage]`.
    #[serde(default)]
    pub item: Option<[i32; 3]>,
    /// The minecart type (0 plain, 1 chest, 2 furnace).
    #[serde(default)]
    pub minecart_type: i32,
    /// The painting's direction (0-3); its block is `x, y, z` rounded down.
    #[serde(default)]
    pub direction: i32,
    /// The falling block's id.
    #[serde(default)]
    pub block_id: Option<i32>,
    /// `fire` ticks.
    #[serde(default)]
    pub fire: i32,
    /// `hurtTime` (the red tint while it counts down).
    #[serde(default)]
    pub hurt_time: i32,
    /// A creeper: powered.
    #[serde(default)]
    pub powered: bool,
    /// A pig: saddled.
    #[serde(default)]
    pub saddled: bool,
    /// A sheep: sheared.
    #[serde(default)]
    pub sheared: bool,
    /// A sheep: the fleece colour.
    #[serde(default)]
    pub fleece: Option<i32>,
    /// A slime: its size.
    #[serde(default)]
    pub size: Option<i32>,
    /// A wolf: angry.
    #[serde(default)]
    pub angry: bool,
    /// A wolf: tamed.
    #[serde(default)]
    pub tamed: bool,
    /// An arrow: `arrowShake`.
    #[serde(default)]
    pub shake: i32,
    /// Primed TNT: the fuse (`EntityList`'s constructor leaves 0; the
    /// block's is 80).
    #[serde(default)]
    pub fuse: Option<i32>,
}

/// Spawns `s` (module docs); logs and returns false when it cannot.
pub fn spawn_entity(game: &mut Game, s: &SpawnEntitySpec) -> bool {
    let base = base_point(game, s.ahead);
    let (Some((bx, by, bz)), Some(world)) = (base, game.the_world.as_mut()) else {
        return false;
    };
    let (x, y, z) = (bx + s.x, by + s.y, bz + s.z);
    let mut e: Entity = match s.kind.as_str() {
        "Item" => {
            let [id, n, d] = s.item.unwrap_or([1, 1, 0]);
            eitem::new_entity_item(world, x, y, z, ItemStack::new(id, n, d))
        }
        "Minecart" => minecart::new_entity_minecart(world, x, y, z, s.minecart_type),
        "Painting" => {
            let p = painting::new_entity_painting(
                world,
                x.floor() as i32,
                y.floor() as i32,
                z.floor() as i32,
                s.direction,
            );
            return world.spawn_entity_in_world(p);
        }
        "LightningBolt" => {
            let l = lightning_bolt::new_entity_lightning_bolt(world, x, y, z);
            return world.add_weather_effect(l);
        }
        "Snowball" => snowball::new_entity_snowball_at(world, x, y, z),
        "Egg" => egg::new_entity_egg_at(world, x, y, z),
        "Fireball" => fireball::new_entity_fireball_empty(world),
        name => match entity_list::create_entity_in_world(name, world) {
            Some(e) => e,
            None => {
                bevy::log::error!("script: spawn_entity: cannot make {name:?}");
                return false;
            }
        },
    };
    if !matches!(e.kind, K::Item(..) | K::Snowball(..) | K::Egg(..)) {
        e.base.set_location_and_angles(x, y, z, s.yaw, s.pitch);
    }
    e.base.prev_rotation_yaw = s.yaw;
    e.base.rotation_yaw = s.yaw;
    e.base.prev_rotation_pitch = s.pitch;
    e.base.rotation_pitch = s.pitch;
    e.base.fire = s.fire;
    if let Some(l) = e.kind.living_mut() {
        l.render_yaw_offset = s.yaw;
        l.prev_render_yaw_offset = s.yaw;
        l.hurt_time = s.hurt_time;
        l.max_hurt_time = s.hurt_time.max(l.max_hurt_time);
    }
    match &mut e.kind {
        K::Creeper(..) if s.powered => {
            e.base.data_watcher.update_object(17, WatchValue::Byte(1));
        }
        K::Pig(..) if s.saddled => pig::set_saddled(&mut e, true),
        K::Sheep(..) => {
            if let Some(c) = s.fleece {
                sheep::set_fleece_color(&mut e, c);
            }
            if s.sheared {
                sheep::set_sheared(&mut e, true);
            }
        }
        K::Slime(..) => {
            if let Some(n) = s.size {
                slime::set_slime_size(&mut e, n);
            }
        }
        K::Wolf(..) => {
            if s.angry {
                wolf::set_wolf_angry(&mut e, true);
            }
            if s.tamed {
                wolf::set_wolf_tamed(&mut e, true);
            }
        }
        K::Arrow(a) => a.arrow_shake = s.shake,
        K::FallingSand(f) => f.block_id = s.block_id.unwrap_or(12),
        K::TntPrimed(t) => t.fuse = s.fuse.unwrap_or(80),
        _ => {}
    }
    let id = e.base.entity_id;
    let ok = world.spawn_entity_in_world(e);
    if !ok {
        bevy::log::error!("script: spawn_entity: {} {id} not spawned", s.kind);
    }
    ok
}

/// A `set_block` request: a block (or a box of them, to `x2, y2, z2`)
/// set as `World.setBlockAndMetadataWithNotify` sets it, and a sign's text.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct SetBlockSpec {
    /// x (or the offset with `ahead`), rounded down.
    #[serde(default)]
    pub x: f64,
    /// y (feet level with `ahead`).
    #[serde(default)]
    pub y: f64,
    /// z.
    #[serde(default)]
    pub z: f64,
    /// The far corner of a box, same frame as `x, y, z`.
    #[serde(default)]
    pub to: Option<[f64; 3]>,
    /// Blocks ahead of the player, horizontally, at its feet.
    #[serde(default)]
    pub ahead: Option<f64>,
    /// The block id.
    pub id: i32,
    /// The metadata.
    #[serde(default)]
    pub meta: i32,
    /// A sign's four lines.
    #[serde(default)]
    pub text: Option<[String; 4]>,
    /// WP-3.3d: `setBlockAndMetadata` (no neighbour notification), then the
    /// metadata forced with `setBlockMetadata` and `markBlockNeedsUpdate`.
    #[serde(default)]
    pub raw: bool,
    /// WP-3.7b: `x, y, z` are offsets from the player's block (`floor(posX)`,
    /// `floor(posY)`, `floor(posZ)`; `posY` is the eye).
    #[serde(default)]
    pub rel: bool,
}

/// The point `ahead` blocks in front of the player at its feet, or the
/// origin.
fn base_point(game: &Game, ahead: Option<f64>) -> Option<(f64, f64, f64)> {
    match ahead {
        Some(d) => game.player().map(|p| {
            let b = &p.base;
            let yaw = (b.rotation_yaw as f64).to_radians();
            (
                b.pos_x - yaw.sin() * d,
                b.pos_y - b.y_offset as f64,
                b.pos_z + yaw.cos() * d,
            )
        }),
        None => Some((0.0, 0.0, 0.0)),
    }
}

/// Sets the blocks of `s` (module docs of [`SetBlockSpec`]).
pub fn set_block(game: &mut Game, s: &SetBlockSpec) -> bool {
    let base = if s.rel {
        game.player().map(|p| {
            let b = &p.base;
            (b.pos_x.floor(), b.pos_y.floor(), b.pos_z.floor())
        })
    } else {
        base_point(game, s.ahead)
    };
    let Some((bx, by, bz)) = base else {
        return false;
    };
    let Some(world) = game.the_world.as_mut() else {
        return false;
    };
    let a = [
        (bx + s.x).floor() as i32,
        (by + s.y).floor() as i32,
        (bz + s.z).floor() as i32,
    ];
    let b = s.to.map_or(a, |t| {
        [
            (bx + t[0]).floor() as i32,
            (by + t[1]).floor() as i32,
            (bz + t[2]).floor() as i32,
        ]
    });
    for x in a[0].min(b[0])..=a[0].max(b[0]) {
        for y in a[1].min(b[1])..=a[1].max(b[1]) {
            for z in a[2].min(b[2])..=a[2].max(b[2]) {
                if s.raw {
                    world.set_block_and_metadata(x, y, z, s.id, s.meta);
                    world.set_block_metadata(x, y, z, s.meta);
                    world.mark_block_needs_update(x, y, z);
                } else {
                    world.set_block_and_metadata_with_notify(x, y, z, s.id, s.meta);
                }
                if let Some(text) = &s.text
                    && let Some(r) = world.get_block_tile_entity(x, y, z)
                    && let Some(te) = world.tile_entities.get_mut(&r)
                    && let mc_sim::world::tile_entity::TileEntityKindData::Sign(sign) = &mut te.kind
                {
                    sign.sign_text = text.clone();
                }
            }
        }
    }
    true
}
