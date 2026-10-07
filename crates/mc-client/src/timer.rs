//! `Timer` (Timer.java): turns wall-clock time into whole game ticks (20 per
//! second) and the partial tick the renderer interpolates with.
//!
//! The Java reads two clocks: `System.currentTimeMillis()` (the system clock,
//! which may jump) and `System.nanoTime() / 1000000` (the monotonic clock);
//! every second of system time it re-estimates their ratio
//! (`timeSyncAdjustment`) and scales the monotonic delta by it. Both values
//! are passed in here ([`Timer::update_timer`]); [`crate::game`] reads them
//! from [`crate::clock`].

/// `Timer`.
#[derive(Clone, Debug, PartialEq)]
pub struct Timer {
    /// `ticksPerSecond` (20 for the game, Minecraft.java:114).
    // src: Timer.java:4
    pub ticks_per_second: f32,
    /// `lastHRTime` (private): the last monotonic time, in seconds. Java's
    /// default `0.0` until the first update.
    // src: Timer.java:5
    last_hr_time: f64,
    /// `elapsedTicks`: whole ticks to run this frame, at most 10.
    // src: Timer.java:6
    pub elapsed_ticks: i32,
    /// `renderPartialTicks`: the fraction of a tick since the last one.
    // src: Timer.java:7
    pub render_partial_ticks: f32,
    /// `timerSpeed`.
    // src: Timer.java:8
    pub timer_speed: f32,
    /// `elapsedPartialTicks`: the carried-over fraction.
    // src: Timer.java:9
    pub elapsed_partial_ticks: f32,
    /// `lastSyncSysClock` (private): `currentTimeMillis` at the last update.
    // src: Timer.java:10
    last_sync_sys_clock: i64,
    /// `lastSyncHRClock` (private): the monotonic millis at the last
    /// re-estimate.
    // src: Timer.java:11
    last_sync_hr_clock: i64,
    /// `field_28132_i` (private): system-clock millis accumulated since the
    /// last re-estimate of `timeSyncAdjustment` (docs/mappings.md).
    // src: Timer.java:12
    field_28132_i: i64,
    /// `timeSyncAdjustment` (private): system-clock millis per monotonic
    /// milli, smoothed.
    // src: Timer.java:13
    time_sync_adjustment: f64,
}

impl Timer {
    /// `new Timer(ticksPerSecond)`, created at `now_millis`
    /// (`currentTimeMillis`) and `nano_time` (`nanoTime`).
    // src: Timer.java:15-19
    pub fn new(ticks_per_second: f32, now_millis: i64, nano_time: i64) -> Self {
        Timer {
            ticks_per_second,
            last_hr_time: 0.0,
            elapsed_ticks: 0,
            render_partial_ticks: 0.0,
            timer_speed: 1.0,
            elapsed_partial_ticks: 0.0,
            last_sync_sys_clock: now_millis,
            last_sync_hr_clock: nano_time / 1000000,
            field_28132_i: 0,
            time_sync_adjustment: 1.0,
        }
    }

    /// `updateTimer()` at `now_millis` / `nano_time`: sets
    /// [`Timer::elapsed_ticks`] (capped at 10) and
    /// [`Timer::render_partial_ticks`]. A system-clock jump of more than a
    /// second, or backwards, resets the monotonic reference, so that frame
    /// advances nothing; a monotonic step is clamped to one second.
    // src: Timer.java:21-64
    #[allow(clippy::manual_clamp, clippy::if_same_then_else)] // the Java's branches, kept
    pub fn update_timer(&mut self, now_millis: i64, nano_time: i64) {
        let var1 = now_millis;
        let var3 = var1.wrapping_sub(self.last_sync_sys_clock);
        let var5 = nano_time / 1000000;
        let var7 = var5 as f64 / 1000.0;
        if var3 > 1000 {
            self.last_hr_time = var7;
        } else if var3 < 0 {
            self.last_hr_time = var7;
        } else {
            self.field_28132_i = self.field_28132_i.wrapping_add(var3);
            if self.field_28132_i > 1000 {
                let var9 = var5.wrapping_sub(self.last_sync_hr_clock);
                // A zero `var9` divides to Infinity in Java (no exception).
                let var11 = self.field_28132_i as f64 / var9 as f64;
                self.time_sync_adjustment +=
                    (var11 - self.time_sync_adjustment) * 0.20000000298023224;
                self.last_sync_hr_clock = var5;
                self.field_28132_i = 0;
            }
            if self.field_28132_i < 0 {
                self.last_sync_hr_clock = var5;
            }
        }
        self.last_sync_sys_clock = var1;
        let mut var13 = (var7 - self.last_hr_time) * self.time_sync_adjustment;
        self.last_hr_time = var7;
        if var13 < 0.0 {
            var13 = 0.0;
        }
        if var13 > 1.0 {
            var13 = 1.0;
        }
        self.elapsed_partial_ticks = (self.elapsed_partial_ticks as f64
            + var13 * self.timer_speed as f64 * self.ticks_per_second as f64)
            as f32;
        self.elapsed_ticks = self.elapsed_partial_ticks as i32;
        self.elapsed_partial_ticks -= self.elapsed_ticks as f32;
        if self.elapsed_ticks > 10 {
            self.elapsed_ticks = 10;
        }
        self.render_partial_ticks = self.elapsed_partial_ticks;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: i64 = 1_000_000;

    #[test]
    fn first_update_after_construction_runs_ten_ticks() {
        // lastHRTime starts at 0: the first monotonic step is the whole
        // uptime, clamped to one second = 20 ticks, capped at 10.
        let mut t = Timer::new(20.0, 1_000_000, 5_000 * MS);
        t.update_timer(1_000_016, 5_016 * MS);
        assert_eq!(t.elapsed_ticks, 10);
        assert_eq!(t.render_partial_ticks, 0.0);
    }

    #[test]
    fn steady_frames_accumulate_partial_ticks() {
        let mut t = Timer::new(20.0, 0, 0);
        t.update_timer(0, 0); // lastHRTime = 0 → step 0
        assert_eq!(t.elapsed_ticks, 0);
        // 30 ms: 0.03 s * 20 = 0.6 ticks
        t.update_timer(30, 30 * MS);
        assert_eq!(t.elapsed_ticks, 0);
        assert_eq!(t.render_partial_ticks, 0.6);
        // +30 ms: 1.2 → one tick, 0.2 left (f32 arithmetic)
        t.update_timer(60, 60 * MS);
        assert_eq!(t.elapsed_ticks, 1);
        assert_eq!(t.render_partial_ticks, (1.2f64 as f32) - 1.0);
        // +500 ms: 10 + 0.2 → 10 ticks
        t.update_timer(560, 560 * MS);
        assert_eq!(t.elapsed_ticks, 10);
    }

    #[test]
    fn a_long_stall_is_clamped_and_capped() {
        let mut t = Timer::new(20.0, 0, 0);
        t.update_timer(0, 0);
        // 900 ms of system time and of monotonic time: 18 ticks → 10.
        t.update_timer(900, 900 * MS);
        assert_eq!(t.elapsed_ticks, 10);
        // The 8 ticks over the cap are lost, not carried (elapsedPartialTicks
        // already had the 18 subtracted).
        assert!(t.render_partial_ticks.abs() < 1e-4);
    }

    #[test]
    fn a_system_clock_jump_resets_the_reference() {
        let mut t = Timer::new(20.0, 0, 0);
        t.update_timer(0, 0);
        // The system clock jumps by 5 s while 50 ms of monotonic time pass.
        t.update_timer(5_000, 50 * MS);
        assert_eq!(t.elapsed_ticks, 0);
        assert_eq!(t.render_partial_ticks, 0.0);
        // Backwards jump: also no step.
        t.update_timer(4_000, 100 * MS);
        assert_eq!(t.elapsed_ticks, 0);
    }

    #[test]
    fn time_sync_adjustment_tracks_the_clock_ratio() {
        let mut t = Timer::new(20.0, 0, 0);
        t.update_timer(0, 0);
        // The system clock runs twice as fast as the monotonic one for a bit
        // over a second: the adjustment moves 20 % of the way to 2.
        let mut now = 0;
        let mut mono = 0;
        for _ in 0..21 {
            now += 100;
            mono += 50;
            t.update_timer(now, mono * MS);
        }
        // After 1100 ms of system time the ratio 1100/550 = 2 was blended in.
        assert_eq!(
            t.time_sync_adjustment,
            1.0 + (2.0 - 1.0) * 0.20000000298023224
        );
    }

    #[test]
    fn timer_speed_scales_the_step() {
        let mut t = Timer::new(20.0, 0, 0);
        t.update_timer(0, 0);
        t.timer_speed = 0.5;
        t.update_timer(100, 100 * MS);
        // 0.1 s * 0.5 * 20 = 1 tick
        assert_eq!(t.elapsed_ticks, 1);
    }
}
