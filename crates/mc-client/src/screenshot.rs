//! `ScreenShotHelper` (ScreenShotHelper.java): F2 saves the window as
//! `<data dir>/screenshots/<yyyy-MM-dd_HH.mm.ss>[_N].png` (local time, the
//! first free `_2`, `_3`, … suffix) and returns the chat line
//! `"Saved screenshot as <name>"`.
//!
//! The pixels come from Bevy's screenshot API ([`Screenshot`], saved by an
//! observer when the frame is captured) instead of `glReadPixels`; the image
//! is the window's RGB without alpha, as vanilla's (`BufferedImage` type 1,
//! `TYPE_INT_RGB`). The scripted mode's `screenshot` action uses
//! [`take_screenshot`] with its own path.

#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

/// `dateFormat`'s pattern `"yyyy-MM-dd_HH.mm.ss"` for a broken-down local
/// time.
// src: ScreenShotHelper.java:14
pub fn format_date(t: LocalTime) -> String {
    format!(
        "{:04}-{:02}-{:02}_{:02}.{:02}.{:02}",
        t.year, t.month, t.day, t.hour, t.minute, t.second
    )
}

/// A broken-down local time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    /// Year.
    pub year: i32,
    /// Month, 1-12.
    pub month: u32,
    /// Day of the month, 1-31.
    pub day: u32,
    /// Hour, 0-23.
    pub hour: u32,
    /// Minute.
    pub minute: u32,
    /// Second.
    pub second: u32,
}

/// The local time of `millis` since the epoch (`new Date()` formatted in
/// the default time zone). Uses `localtime_r` on Unix; elsewhere UTC.
pub fn local_time(millis: i64) -> LocalTime {
    let secs = millis.div_euclid(1000);
    #[cfg(unix)]
    {
        let t: libc::time_t = secs as libc::time_t;
        // SAFETY: `tm` is plain data, fully written by `localtime_r` on
        // success; both pointers are valid for the call.
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        let ok = unsafe { !libc::localtime_r(&t, &mut tm).is_null() };
        if ok {
            return LocalTime {
                year: tm.tm_year + 1900,
                month: (tm.tm_mon + 1) as u32,
                day: tm.tm_mday as u32,
                hour: tm.tm_hour as u32,
                minute: tm.tm_min as u32,
                second: tm.tm_sec as u32,
            };
        }
    }
    utc_time(secs)
}

/// The UTC broken-down time of `secs` since the epoch (civil-from-days).
pub fn utc_time(secs: i64) -> LocalTime {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    LocalTime {
        year: (if m <= 2 { y + 1 } else { y }) as i32,
        month: m as u32,
        day: d as u32,
        hour: (rem / 3600) as u32,
        minute: (rem % 3600 / 60) as u32,
        second: (rem % 60) as u32,
    }
}

/// `saveScreenshot`'s file choice: `dir/<date>.png`, else
/// `dir/<date>_2.png`, `_3`, … whichever does not exist yet (`var3.mkdir()`
/// is the caller's).
// src: ScreenShotHelper.java:37-41
pub fn screenshot_file(dir: &Path, date: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let mut var6 = 1;
    loop {
        let var5 = dir.join(format!(
            "{date}{}.png",
            if var6 == 1 {
                String::new()
            } else {
                format!("_{var6}")
            }
        ));
        if !exists(&var5) {
            return var5;
        }
        var6 += 1;
    }
}

/// Screenshots requested and not yet written (the scripted mode waits for
/// zero before going on).
#[derive(Resource, Clone, Debug, Default)]
pub struct PendingScreenshots(pub Arc<AtomicUsize>);

impl PendingScreenshots {
    /// How many are pending.
    pub fn count(&self) -> usize {
        self.0.load(Ordering::SeqCst)
    }
}

/// Captures the primary window into the PNG at `path` (parent directories
/// created), counting it in `pending` until written.
pub fn take_screenshot(commands: &mut Commands, pending: &PendingScreenshots, path: PathBuf) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let counter = pending.0.clone();
    counter.fetch_add(1, Ordering::SeqCst);
    commands.spawn(Screenshot::primary_window()).observe(
        move |captured: On<ScreenshotCaptured>| {
            let img = captured.image.clone();
            match img.try_into_dynamic() {
                Ok(dynamic) => {
                    // TYPE_INT_RGB: no alpha.
                    let rgb = dynamic.to_rgb8();
                    match rgb.save_with_format(&path, image::ImageFormat::Png) {
                        Ok(()) => info!("Saved screenshot as {}", path.display()),
                        Err(e) => error!("Failed to save: {e}"),
                    }
                }
                Err(e) => error!("Failed to save: {e:?}"),
            }
            counter.fetch_sub(1, Ordering::SeqCst);
        },
    );
}

/// `ScreenShotHelper.saveScreenshot(mcDataDir, w, h)`'s naming and message
/// for an F2 press at `now_millis`: starts the capture, returns the chat
/// line.
// src: ScreenShotHelper.java:19-64
pub fn save_screenshot(
    commands: &mut Commands,
    pending: &PendingScreenshots,
    data_dir: &Path,
    now_millis: i64,
) -> String {
    let var3 = data_dir.join("screenshots");
    if let Err(e) = std::fs::create_dir_all(&var3) {
        return format!("Failed to save: {e}");
    }
    let var4 = format_date(local_time(now_millis));
    let var5 = screenshot_file(&var3, &var4, |p| p.exists());
    let name = var5
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    take_screenshot(commands, pending, var5);
    format!("Saved screenshot as {name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_format() {
        let t = utc_time(1_791_331_200 + 3 * 3600 + 4 * 60 + 5); // 2026-10-07 03:04:05 UTC
        assert_eq!(format_date(t), "2026-10-07_03.04.05");
        assert_eq!(format_date(utc_time(0)), "1970-01-01_00.00.00");
        assert_eq!(format_date(utc_time(951_782_400)), "2000-02-29_00.00.00");
    }

    #[test]
    fn suffixes_skip_existing_files() {
        let dir = Path::new("/s");
        let taken = ["/s/d.png", "/s/d_2.png"];
        let f = screenshot_file(dir, "d", |p| taken.iter().any(|t| Path::new(t) == p));
        assert_eq!(f, PathBuf::from("/s/d_3.png"));
        let f = screenshot_file(dir, "e", |_| false);
        assert_eq!(f, PathBuf::from("/s/e.png"));
    }
}
