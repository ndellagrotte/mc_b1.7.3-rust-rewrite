//! The F3 screen's text (`GuiIngame.renderGameOverlay`'s debug block,
//! GuiIngame.java:166-190), as strings; drawing them is WP-3.6's.
//!
//! The parts that other renderers own are filled in by them through
//! [`DebugSources`]: `RenderGlobal.getDebugInfoRenders` /
//! `getDebugInfoEntities` (WP-3.3, WP-3.5) and `EffectRenderer.getStatistics`
//! (WP-3.7); until then they read as vanilla's with zero counts.

use bevy::prelude::*;
use mc_core::math::floor_double;

use crate::game::Game;
use crate::jfmt::double_to_string;

/// The other renderers' debug strings (module docs).
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct DebugSources {
    /// `mc.debugInfoRenders()` = `renderGlobal.getDebugInfoRenders()`.
    pub renders: String,
    /// `mc.func_6262_n()` = `renderGlobal.getDebugInfoEntities()`.
    pub entities: String,
    /// `effectRenderer.getStatistics()`: the particle count.
    pub particles: String,
    /// The Java heap lines' numbers (`Runtime.maxMemory/totalMemory/
    /// freeMemory`): there is no JVM; the client may report process memory
    /// here. Bytes.
    pub max_memory: i64,
    /// See [`DebugSources::max_memory`].
    pub total_memory: i64,
    /// See [`DebugSources::max_memory`].
    pub free_memory: i64,
}

impl Default for DebugSources {
    // src: RenderGlobal.java:309-315, EffectRenderer.java:167-169
    fn default() -> Self {
        DebugSources {
            renders: "C: 0/0. F: 0, O: 0, E: 0".into(),
            entities: "E: 0/0. B: 0, I: 0".into(),
            particles: "0".into(),
            max_memory: 1,
            total_memory: 0,
            free_memory: 0,
        }
    }
}

/// The F3 lines.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct DebugText {
    /// The left column, top down: the version line, renders, entities,
    /// particles/entities, chunk cache (at y 2, 12, 22, 32, 42; white).
    pub left: Vec<String>,
    /// The right column: the two memory lines (y 2, 12; right-aligned).
    pub right: Vec<String>,
    /// The position block: `x:`, `y:`, `z:`, `f:` (y 64, 72, 80, 88).
    pub position: Vec<String>,
}

/// The lines for the current game (empty without F3 or a world).
// src: GuiIngame.java:166-190, Minecraft.java:1402-1416
pub fn debug_text(g: &Game, src: &DebugSources) -> DebugText {
    let (Some(world), Some(p)) = (g.the_world.as_ref(), g.player()) else {
        return DebugText::default();
    };
    if !g.game_settings.show_debug_info {
        return DebugText::default();
    }
    let left = vec![
        format!("Minecraft Beta 1.7.3 ({})", g.debug),
        src.renders.clone(),
        src.entities.clone(),
        // debugInfoEntities(): "P: " + effectRenderer.getStatistics() +
        // ". T: " + theWorld.func_687_d()
        format!("P: {}. T: {}", src.particles, world.func_687_d()),
        // func_21002_o(): theWorld.func_21119_g()
        world.func_21119_g(),
    ];
    let (max, total, free) = (src.max_memory.max(1), src.total_memory, src.free_memory);
    let used = total - free;
    let right = vec![
        format!(
            "Used memory: {}% ({}MB) of {}MB",
            used * 100 / max,
            used / 1024 / 1024,
            max / 1024 / 1024
        ),
        format!(
            "Allocated memory: {}% ({}MB)",
            total * 100 / max,
            total / 1024 / 1024
        ),
    ];
    let b = &p.base;
    let position = vec![
        format!("x: {}", double_to_string(b.pos_x)),
        format!("y: {}", double_to_string(b.pos_y)),
        format!("z: {}", double_to_string(b.pos_z)),
        format!(
            "f: {}",
            floor_double((b.rotation_yaw * 4.0 / 360.0) as f64 + 0.5) & 3
        ),
    ];
    DebugText {
        left,
        right,
        position,
    }
}

/// Fills [`DebugText`] every frame.
pub fn publish_debug_text(game: NonSend<Game>, src: Res<DebugSources>, mut out: ResMut<DebugText>) {
    let t = debug_text(&game, &src);
    if *out != t {
        *out = t;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn facing_formula() {
        // f: floor_double(yaw * 4 / 360 + 0.5) & 3 — 0 south, 1 west, 2
        // north, 3 east; -180 (flipPlayer) is north.
        let f = |yaw: f32| mc_core::math::floor_double((yaw * 4.0 / 360.0) as f64 + 0.5) & 3;
        assert_eq!(f(0.0), 0);
        assert_eq!(f(90.0), 1);
        assert_eq!(f(-180.0), 2);
        assert_eq!(f(-90.0), 3);
        assert_eq!(f(44.0), 0);
        assert_eq!(f(46.0), 1);
    }
}
