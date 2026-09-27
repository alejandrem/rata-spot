//! State: contadores del sync entre ticks (vive en run_app).

use std::collections::HashMap;
use std::time::Duration;

/// Estado mutable del sync entre ticks (vive en run_app).
pub struct SyncState {
    pub(crate) saved_title: String,
    pub(crate) fail_streak: u32,
    pub(crate) retry_ticks: u32,
    /// Backoff exponencial en ticks (4 ticks = 1s): 4, 8, 16, 32.
    /// Sin esto se martilla COM cada 1s eternamente cuando Brave murio.
    pub(crate) cooldown_ticks: u32,
    pub(crate) backoff_step: u32,
    /// Ultimo estado visto por titulo (para detectar que sesion cambio).
    pub(crate) seen: HashMap<String, bool>,
    pub(crate) scan_ticks: u32,
    pub(crate) last_raw_pos: Duration,
    pub(crate) last_good_dur: Duration,
    pub(crate) last_good_title: String,
}

impl SyncState {
    pub fn new(saved_title: String) -> Self {
        Self {
            saved_title,
            fail_streak: 0,
            retry_ticks: 99,
            cooldown_ticks: 0,
            backoff_step: 0,
            seen: HashMap::new(),
            scan_ticks: 0,
            last_raw_pos: Duration::ZERO,
            last_good_dur: Duration::ZERO,
            last_good_title: String::new(),
        }
    }
}
