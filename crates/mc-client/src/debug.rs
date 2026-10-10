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
    /// freeMemory`), in bytes. There is no JVM: [`publish_debug_text`]
    /// fills them from the process ([`process_memory`]: max = a quarter of
    /// the physical memory, the JVM's default `-Xmx`; total = the resident
    /// set; free = its file-backed and shared part, so "used" is the
    /// anonymous memory, the heap's stand-in).
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

/// `kB` value of a `/proc` line `"<key>:   <n> kB"`, in bytes.
pub fn proc_kb(text: &str, key: &str) -> Option<i64> {
    text.lines().find_map(|l| {
        let rest = l.strip_prefix(key)?.strip_prefix(':')?;
        let n: i64 = rest.split_whitespace().next()?.parse().ok()?;
        Some(n * 1024)
    })
}

/// The stand-in for `Runtime.maxMemory()`, `totalMemory()` and
/// `freeMemory()` (project-owned; P5 of docs/PARITY_CHECKLIST.md) from
/// `/proc/self/status` and `/proc/meminfo`:
///
/// - max: `MemTotal / 4`, Java 8's default maximum heap on a machine with
///   that much memory (vanilla, run with no `-Xmx`, showed `of 6932MB` on
///   the test machine's 27.7 GB);
/// - total ("Allocated"): `VmRSS`, the process's resident memory;
/// - free: `VmRSS - RssAnon`, the resident file-backed and shared pages,
///   so "Used" (`total - free`) is the anonymous memory (heap, stacks,
///   buffers), the nearest thing to a JVM's live heap.
pub fn memory_from_proc(status: &str, meminfo: &str) -> Option<(i64, i64, i64)> {
    let max = proc_kb(meminfo, "MemTotal")? / 4;
    let total = proc_kb(status, "VmRSS")?;
    let anon = proc_kb(status, "RssAnon").unwrap_or(total);
    Some((max, total, (total - anon).max(0)))
}

/// [`memory_from_proc`] for this process; `None` where there is no
/// `/proc` (the lines then read 0, as before).
pub fn process_memory() -> Option<(i64, i64, i64)> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let meminfo = std::fs::read_to_string("/proc/meminfo").ok()?;
    memory_from_proc(&status, &meminfo)
}

/// The right column's first row of the project's lines: row 5 is y = 52,
/// under the left column's lines at y = 2 to 42.
// src: GuiIngame.java:172-176
pub const PERF_FIRST_ROW: usize = 5;

/// Fills [`DebugText`] every frame. With F3 on, the right column also
/// gets the project's performance lines ([`crate::perf::PerfStats::f3_lines`],
/// WP-4.3) from row [`PERF_FIRST_ROW`] down, below the rows of the left
/// column's five lines, so no row holds both.
pub fn publish_debug_text(
    game: NonSend<Game>,
    mut src: ResMut<DebugSources>,
    perf: Option<Res<crate::perf::PerfStats>>,
    mut out: ResMut<DebugText>,
) {
    // Runtime.getRuntime()'s numbers, read each frame the lines are shown
    // (GuiIngame.java:178-183).
    if game.game_settings.show_debug_info
        && game.the_world.is_some()
        && let Some((max, total, free)) = process_memory()
        && (src.max_memory, src.total_memory, src.free_memory) != (max, total, free)
    {
        src.max_memory = max;
        src.total_memory = total;
        src.free_memory = free;
    }
    let mut t = debug_text(&game, &src);
    if !t.right.is_empty()
        && let Some(p) = perf
    {
        // Rows 2 to 4 stay empty (an empty string draws no glyph).
        t.right
            .resize(PERF_FIRST_ROW.max(t.right.len()), String::new());
        t.right.extend(p.f3_lines());
    }
    if *out != t {
        *out = t;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_lines_from_proc() {
        let status =
            "Name:\tmc_b173\nVmRSS:\t  409600 kB\nRssAnon:\t  307200 kB\nRssFile:\t 102400 kB\n";
        let meminfo = "MemTotal:       28385280 kB\nMemFree:  1 kB\n";
        let (max, total, free) = memory_from_proc(status, meminfo).unwrap();
        assert_eq!(max, 28385280 * 1024 / 4);
        assert_eq!(total, 400 * 1024 * 1024);
        assert_eq!(free, 100 * 1024 * 1024);
        // GuiIngame.java:178-183's format: used = total - free.
        let used = total - free;
        assert_eq!(
            format!(
                "Used memory: {}% ({}MB) of {}MB",
                used * 100 / max,
                used / 1024 / 1024,
                max / 1024 / 1024
            ),
            "Used memory: 4% (300MB) of 6930MB"
        );
        assert_eq!(proc_kb("VmRSSx: 5 kB\n", "VmRSS"), None);
        assert!(memory_from_proc("", meminfo).is_none());
    }

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
