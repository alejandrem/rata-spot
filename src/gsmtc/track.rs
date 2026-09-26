//! Tipos del track + conversiones de tiempo WinRT.

use std::time::Duration;

use windows::Media::Control::GlobalSystemMediaTransportControlsSession;

pub type Session = GlobalSystemMediaTransportControlsSession;

/// Info de la cancion actual. `progress` siempre en 0.0..=1.0.
#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub playing: bool,
    pub position: Duration,
    pub duration: Duration,
    pub progress: f64,
}

impl Default for TrackInfo {
    fn default() -> Self {
        Self {
            title: "—".to_string(),
            artist: "—".to_string(),
            album: String::new(),
            playing: false,
            position: Duration::ZERO,
            duration: Duration::ZERO,
            progress: 0.0,
        }
    }
}

impl TrackInfo {
    pub fn format_time(&self) -> String {
        format!("{} / {}", fmt_duration(self.position), fmt_duration(self.duration))
    }
}

pub fn fmt_duration(d: Duration) -> String {
    let s = d.as_secs();
    format!("{}:{:02}", s / 60, s % 60)
}

/// TimeSpan WinRT (ticks de 100ns) -> Duration.
pub(crate) fn timespan_to_duration(ticks_100ns: i64) -> Duration {
    if ticks_100ns <= 0 {
        return Duration::ZERO;
    }
    // 1 tick = 100ns -> 10_000_000 ticks = 1s
    Duration::from_micros((ticks_100ns / 10) as u64)
}
