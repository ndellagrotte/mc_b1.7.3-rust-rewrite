//! The `IWorldAccess` fan-out as an event queue (PLAN_v4 §3.2).
//!
//! Vanilla `World` keeps a list of `IWorldAccess` listeners (`worldAccesses`,
//! World.java:40) — in practice `RenderGlobal` — and loops over it at each
//! notification site. The simulation never calls the client, so each such
//! call site pushes one [`WorldEvent`] instead; `mc-client` drains the queue
//! each frame. Every variant mirrors one `IWorldAccess` method
//! (IWorldAccess.java:3-23), in interface order, with the same arguments. An
//! entity argument becomes its `entityId`, a tile entity its position.
//!
//! Nothing in the simulation reads these events back, so they have no
//! gameplay effect; a headless runner (`mc-golden`) may simply clear the queue.

use std::borrow::Cow;

/// One `IWorldAccess` call.
#[derive(Clone, Debug, PartialEq)]
pub enum WorldEvent {
    /// `markBlockAndNeighborsNeedsUpdate(x, y, z)`.
    MarkBlockAndNeighborsNeedsUpdate { x: i32, y: i32, z: i32 },
    /// `markBlockRangeNeedsUpdate(x0, y0, z0, x1, y1, z1)`.
    MarkBlockRangeNeedsUpdate {
        x0: i32,
        y0: i32,
        z0: i32,
        x1: i32,
        y1: i32,
        z1: i32,
    },
    /// `playSound(name, x, y, z, volume, pitch)`.
    PlaySound {
        name: Cow<'static, str>,
        x: f64,
        y: f64,
        z: f64,
        volume: f32,
        pitch: f32,
    },
    /// `spawnParticle(name, x, y, z, vx, vy, vz)`.
    SpawnParticle {
        name: Cow<'static, str>,
        x: f64,
        y: f64,
        z: f64,
        vx: f64,
        vy: f64,
        vz: f64,
    },
    /// `obtainEntitySkin(entity)`: an entity joined the world.
    ObtainEntitySkin { entity_id: i32 },
    /// `releaseEntitySkin(entity)`: an entity left the world.
    ReleaseEntitySkin { entity_id: i32 },
    /// `updateAllRenderers()`: the sky light level changed.
    UpdateAllRenderers,
    /// `playRecord(name, x, y, z)` (`None` = stop).
    PlayRecord {
        name: Option<Cow<'static, str>>,
        x: i32,
        y: i32,
        z: i32,
    },
    /// `doNothingWithTileEntity(x, y, z, tileEntity)`.
    DoNothingWithTileEntity { x: i32, y: i32, z: i32 },
    /// `playAuxSFX(player, type, x, y, z, data)`; `player` is the entity id of
    /// the player argument, if any.
    PlayAuxSfx {
        player: Option<i32>,
        kind: i32,
        x: i32,
        y: i32,
        z: i32,
        data: i32,
    },
}

/// The pending [`WorldEvent`]s, in call order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventQueue(pub Vec<WorldEvent>);

impl EventQueue {
    /// Appends one event (one `IWorldAccess` call).
    pub fn push(&mut self, e: WorldEvent) {
        self.0.push(e);
    }

    /// Removes and returns every pending event, oldest first.
    pub fn drain(&mut self) -> Vec<WorldEvent> {
        std::mem::take(&mut self.0)
    }

    /// Number of pending events.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no event is pending.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_returns_in_order_and_empties() {
        let mut q = EventQueue::default();
        q.push(WorldEvent::UpdateAllRenderers);
        q.push(WorldEvent::MarkBlockAndNeighborsNeedsUpdate { x: 1, y: 2, z: 3 });
        let v = q.drain();
        assert_eq!(
            v,
            vec![
                WorldEvent::UpdateAllRenderers,
                WorldEvent::MarkBlockAndNeighborsNeedsUpdate { x: 1, y: 2, z: 3 }
            ]
        );
        assert!(q.is_empty());
        assert!(q.drain().is_empty());
    }
}
