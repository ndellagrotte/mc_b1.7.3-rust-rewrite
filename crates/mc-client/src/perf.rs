//! Performance counters (WP-4.3, a project addition with no vanilla
//! counterpart): the frame-time distribution over the last
//! [`WINDOW`] frames, the time of the client's main per-frame phases, the
//! start-up milestones, and the native storage's write counters.
//!
//! - **Frame time** is the wall time between the starts of two consecutive
//!   app updates ([`perf_frame_start`] in `First`): with vsync off
//!   (`mc_b173 --vsync off`, `PresentMode::AutoNoVsync`) it is the real
//!   frame time; with vsync on it is capped at the display's refresh, and
//!   a compositor may still pace a hidden or covered window.
//! - **Update time** is the main schedule's wall time in one frame, from
//!   `First` to `Last` ([`perf_frame_end`]): the CPU work of a frame on the
//!   main thread, without the wait for the render thread and the display
//!   (Bevy renders the previous frame in parallel), so it does not depend
//!   on the pacing.
//! - **Phases** ([`Phase`]) are measured inside the systems that make them
//!   up with a [`PhaseTimer`] guard, so systems running in parallel each
//!   count their own time; [`perf_frame_end`] (in `Last`) moves the frame's
//!   sums into the window. A phase's figure is its CPU time on the main
//!   app's systems this frame, not the GPU's.
//! - **Start-up**: [`mark_process_start`] (called first thing by `mc-app`'s
//!   `main`) and [`perf_milestones`]: the first title-screen frame and the
//!   first in-game frame.
//!
//! The numbers reach the `dump_debug` dev action (`perf`, `storage`) and the
//! F3 screen's right column, under the memory lines, prefixed `[rust]`
//! ([`PerfStats::f3_lines`]). Nothing here changes what the game draws
//! without F3.

#![allow(clippy::disallowed_types)]

use std::collections::VecDeque;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use bevy::prelude::*;

use crate::app::AppState;
use crate::clock;

/// How many frames the statistics cover.
pub const WINDOW: usize = 600;

/// The measured phases of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    /// `game::game_frame`: input, the game ticks (the simulation), the
    /// frame's camera half.
    Sim,
    /// `terrain::terrain_frame` (marks, `updateRenderers`, snapshots,
    /// near meshing, finished jobs, `sortAndRender`) and the crack.
    Terrain,
    /// The entity and tile-entity renderers (`render_entities`,
    /// `render_pickups`, `apply_draws`).
    Entities,
    /// The particles (`compute_particles`, `apply_particles`).
    Particles,
    /// The GUI's drawing calls into the batcher (screens, HUD, F3 graph).
    Gui,
    /// The batcher's flush into meshes (`gui::render::flush_gui`).
    GuiFlush,
    /// Sky, clouds, weather, fog, selection box (`compute_render_frame` and
    /// the `apply_*` systems).
    Sky,
    /// The first-person hand and item (`compute_hand`, `apply_hand`).
    Hand,
}

impl Phase {
    /// Every phase, in report order.
    pub const ALL: [Phase; 8] = [
        Phase::Sim,
        Phase::Terrain,
        Phase::Entities,
        Phase::Particles,
        Phase::Gui,
        Phase::GuiFlush,
        Phase::Sky,
        Phase::Hand,
    ];

    /// The name in `dump_debug`.
    pub fn name(self) -> &'static str {
        match self {
            Phase::Sim => "sim",
            Phase::Terrain => "terrain",
            Phase::Entities => "entities",
            Phase::Particles => "particles",
            Phase::Gui => "gui",
            Phase::GuiFlush => "gui_flush",
            Phase::Sky => "sky",
            Phase::Hand => "hand",
        }
    }

    /// The short label on the F3 line.
    fn short(self) -> &'static str {
        match self {
            Phase::Sim => "sim",
            Phase::Terrain => "ter",
            Phase::Entities => "ent",
            Phase::Particles => "fx",
            Phase::Gui => "gui",
            Phase::GuiFlush => "flush",
            Phase::Sky => "sky",
            Phase::Hand => "hand",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// This frame's nanoseconds per phase (summed by [`PhaseTimer`]s).
static FRAME_ACC: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];

/// Adds the time from its creation to its drop to a [`Phase`].
#[must_use = "the phase is timed until the guard is dropped"]
pub struct PhaseTimer {
    phase: Phase,
    t0: i64,
}

impl Drop for PhaseTimer {
    fn drop(&mut self) {
        let dt = (clock::nano_time() - self.t0).max(0) as u64;
        FRAME_ACC[self.phase.index()].fetch_add(dt, Ordering::Relaxed);
    }
}

/// Times the rest of the enclosing scope as `phase`.
pub fn time(phase: Phase) -> PhaseTimer {
    PhaseTimer {
        phase,
        t0: clock::nano_time(),
    }
}

/// The process start (module docs, "Start-up").
static PROCESS_START: OnceLock<Instant> = OnceLock::new();

/// Records the process start; `mc-app` calls it first thing in `main`.
/// Later calls do nothing.
pub fn mark_process_start() {
    PROCESS_START.get_or_init(Instant::now);
}

/// Milliseconds since [`mark_process_start`] (or since the first call of
/// this function, if it was never called).
pub fn ms_since_process_start() -> f64 {
    PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_secs_f64()
        * 1e3
}

/// Minimum, mean, 95th percentile and maximum of a sample, milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Summary {
    /// Number of samples.
    pub n: usize,
    /// Smallest.
    pub min: f64,
    /// Mean.
    pub avg: f64,
    /// 95th percentile (nearest rank).
    pub p95: f64,
    /// Largest.
    pub max: f64,
}

impl Summary {
    /// The summary of `xs` (milliseconds); all zero when empty.
    pub fn of(xs: impl Iterator<Item = f64>) -> Summary {
        let mut v: Vec<f64> = xs.collect();
        if v.is_empty() {
            return Summary::default();
        }
        v.sort_by(f64::total_cmp);
        let n = v.len();
        // Nearest rank: the smallest value with at least 95 % of the
        // sample at or below it.
        let rank = (n * 95).div_ceil(100).max(1);
        Summary {
            n,
            min: v[0],
            avg: v.iter().sum::<f64>() / n as f64,
            p95: v[rank - 1],
            max: v[n - 1],
        }
    }

    /// As JSON.
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "n": self.n,
            "min_ms": self.min,
            "avg_ms": self.avg,
            "p95_ms": self.p95,
            "max_ms": self.max,
        })
    }
}

/// The window of frame and phase times, and the start-up milestones.
#[derive(Resource, Debug, Default)]
pub struct PerfStats {
    /// Frame times of the last [`WINDOW`] frames, milliseconds.
    pub frames: VecDeque<f64>,
    /// Update times (module docs) of the last [`WINDOW`] frames,
    /// milliseconds.
    pub updates: VecDeque<f64>,
    /// Per phase ([`Phase::ALL`] order), the last [`WINDOW`] frames'
    /// times, milliseconds.
    pub phases: [VecDeque<f64>; 8],
    /// `nano_time` at the start of the previous app update.
    last_start: Option<i64>,
    /// Milliseconds from the process start to the first frame that showed
    /// the title screen.
    pub title_ms: Option<f64>,
    /// Milliseconds from the process start to the first in-game frame.
    pub in_game_ms: Option<f64>,
    /// Milliseconds from the process start to the end of the plugin's
    /// build (jar opened, settings and textures set up).
    pub plugin_built_ms: Option<f64>,
}

fn push(q: &mut VecDeque<f64>, v: f64) {
    if q.len() == WINDOW {
        q.pop_front();
    }
    q.push_back(v);
}

impl PerfStats {
    /// The frame-time summary.
    pub fn frame_summary(&self) -> Summary {
        Summary::of(self.frames.iter().copied())
    }

    /// The update-time summary.
    pub fn update_summary(&self) -> Summary {
        Summary::of(self.updates.iter().copied())
    }

    /// A phase's summary.
    pub fn phase_summary(&self, p: Phase) -> Summary {
        Summary::of(self.phases[p.index()].iter().copied())
    }

    /// The `perf` object of `dump_debug`.
    pub fn to_json(&self) -> serde_json::Value {
        let mut phases = serde_json::Map::new();
        for p in Phase::ALL {
            phases.insert(p.name().into(), self.phase_summary(p).to_json());
        }
        serde_json::json!({
            "window_frames": WINDOW,
            "frame": self.frame_summary().to_json(),
            "update": self.update_summary().to_json(),
            "phases": phases,
            "startup": {
                "plugin_built_ms": self.plugin_built_ms,
                "title_ms": self.title_ms,
                "in_game_ms": self.in_game_ms,
            },
        })
    }

    /// The project's F3 lines (module docs): the frame-time summary and the
    /// phases' means, milliseconds.
    pub fn f3_lines(&self) -> Vec<String> {
        let f = self.frame_summary();
        let u = self.update_summary();
        let mut phases = String::from("[rust] ms");
        for p in [
            Phase::Sim,
            Phase::Terrain,
            Phase::Entities,
            Phase::Particles,
            Phase::Gui,
            Phase::GuiFlush,
        ] {
            phases.push_str(&format!(" {} {:.2}", p.short(), self.phase_summary(p).avg));
        }
        vec![
            format!(
                "[rust] frame ms {:.1}/{:.1}/{:.1}/{:.1} (min/avg/p95/max of {})",
                f.min, f.avg, f.p95, f.max, f.n
            ),
            format!(
                "[rust] update ms {:.1}/{:.1}/{:.1}/{:.1}",
                u.min, u.avg, u.p95, u.max
            ),
            phases,
        ]
    }
}

/// `First`: the frame time since the previous update.
pub fn perf_frame_start(mut stats: ResMut<PerfStats>) {
    let now = clock::nano_time();
    if let Some(prev) = stats.last_start {
        push(&mut stats.frames, (now - prev).max(0) as f64 / 1e6);
    }
    stats.last_start = Some(now);
}

/// `Last`: the update time and this frame's phase sums into the window.
pub fn perf_frame_end(mut stats: ResMut<PerfStats>) {
    if let Some(start) = stats.last_start {
        let ms = (clock::nano_time() - start).max(0) as f64 / 1e6;
        push(&mut stats.updates, ms);
    }
    for p in Phase::ALL {
        let ns = FRAME_ACC[p.index()].swap(0, Ordering::Relaxed);
        push(&mut stats.phases[p.index()], ns as f64 / 1e6);
    }
}

/// The start-up milestones (module docs).
pub fn perf_milestones(
    mut stats: ResMut<PerfStats>,
    state: Res<State<AppState>>,
    game: NonSend<crate::game::Game>,
) {
    if stats.title_ms.is_none()
        && game
            .current_screen
            .as_ref()
            .is_some_and(|s| s.kind == crate::app::ScreenKind::MainMenu)
    {
        let ms = ms_since_process_start();
        stats.title_ms = Some(ms);
        info!("perf: title screen after {ms:.0} ms");
    }
    if stats.in_game_ms.is_none() && *state.get() == AppState::InGame {
        let ms = ms_since_process_start();
        stats.in_game_ms = Some(ms);
        info!("perf: in game after {ms:.0} ms");
    }
}

/// Registers the counters ([`crate::ClientPlugin`]).
pub fn build(app: &mut App) {
    let stats = PerfStats {
        plugin_built_ms: Some(ms_since_process_start()),
        ..Default::default()
    };
    app.insert_resource(stats)
        .add_systems(First, perf_frame_start)
        .add_systems(Last, (perf_milestones, perf_frame_end).chain());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_nearest_rank() {
        let s = Summary::of((1..=100).map(f64::from));
        assert_eq!(s.n, 100);
        assert_eq!(s.min, 1.0);
        assert_eq!(s.max, 100.0);
        assert_eq!(s.p95, 95.0);
        assert!((s.avg - 50.5).abs() < 1e-12);
        let one = Summary::of([7.0].into_iter());
        assert_eq!((one.min, one.p95, one.max), (7.0, 7.0, 7.0));
        assert_eq!(Summary::of(std::iter::empty()), Summary::default());
        // 20 samples: rank ceil(19) = 19.
        let s = Summary::of((1..=20).map(f64::from));
        assert_eq!(s.p95, 19.0);
    }

    #[test]
    fn window_is_bounded() {
        let mut q = VecDeque::new();
        for i in 0..(WINDOW + 10) {
            push(&mut q, i as f64);
        }
        assert_eq!(q.len(), WINDOW);
        assert_eq!(q.front().copied(), Some(10.0));
    }

    #[test]
    fn phase_timer_accumulates() {
        // Uses Hand, which no other unit test times.
        FRAME_ACC[Phase::Hand.index()].store(0, Ordering::Relaxed);
        {
            let _t = time(Phase::Hand);
            std::hint::black_box((0..1000).sum::<u64>());
        }
        let mut stats = PerfStats::default();
        let mut world = bevy::ecs::world::World::new();
        world.insert_resource(PerfStats::default());
        world.run_system_cached(perf_frame_end).unwrap();
        std::mem::swap(&mut stats, &mut *world.resource_mut::<PerfStats>());
        assert_eq!(stats.phases[Phase::Hand.index()].len(), 1);
        assert_eq!(FRAME_ACC[Phase::Hand.index()].load(Ordering::Relaxed), 0);
        let lines = stats.f3_lines();
        assert!(lines[0].starts_with("[rust] frame ms"));
        assert!(lines[1].starts_with("[rust] update ms"));
        assert!(lines[2].starts_with("[rust] ms sim "));
    }
}
