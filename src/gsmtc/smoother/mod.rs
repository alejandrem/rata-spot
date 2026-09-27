//! Progreso suave: Spotify Web manda TimelineProperties en rafagas cada
//! varios segundos -> la barra se moveria a saltos. Se interpola con el
//! reloj local (last_position + elapsed cuando esta playing).

mod display;
#[cfg(test)]
mod tests;
mod update;

#[allow(unused_imports)]
pub use display::*;
#[allow(unused_imports)]
pub use update::*;

use std::time::{Duration, Instant};

use super::track::TrackInfo;

/// Guarda la ultima posicion GSMTC + cuando llego, para interpolar.
#[derive(Debug)]
pub struct ProgressSmoother {
    pub(crate) last_position: Duration,
    pub(crate) last_duration: Duration,
    pub(crate) last_update: Instant,
    pub(crate) was_playing: bool,
    pub(crate) last_track: Option<TrackInfo>,
}

impl Default for ProgressSmoother {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressSmoother {
    pub fn new() -> Self {
        Self {
            last_position: Duration::ZERO,
            last_duration: Duration::ZERO,
            last_update: Instant::now(),
            was_playing: false,
            last_track: None,
        }
    }
}
