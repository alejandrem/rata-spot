//! Progreso suave: Spotify Web manda TimelineProperties en rafagas cada
//! varios segundos -> la barra se moveria a saltos. Se interpola con el
//! reloj local (last_position + elapsed cuando esta playing).

use std::time::{Duration, Instant};

use super::track::TrackInfo;

/// Guarda la ultima posicion GSMTC + cuando llego, para interpolar.
#[derive(Debug)]
pub struct ProgressSmoother {
    last_position: Duration,
    last_duration: Duration,
    last_update: Instant,
    was_playing: bool,
    last_track: Option<TrackInfo>,
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

    /// Llamar cada vez que llega un `get_track()` fresco y OK.
    pub fn update(&mut self, fresh: TrackInfo) {
        self.last_position = fresh.position;
        self.last_duration = fresh.duration;
        self.last_update = Instant::now();
        self.was_playing = fresh.playing;
        self.last_track = Some(fresh);
    }

    /// Llamar cuando `get_track()` falla: conserva el ultimo track conocido
    /// (no pantalla en blanco) y congela el progreso si estaba en pausa.
    pub fn last_known(&self) -> TrackInfo {
        self.last_track.clone().unwrap_or_default()
    }

    /// Posicion actual interpolada: last + elapsed (solo si playing).
    pub fn smoothed_position(&self) -> Duration {
        if !self.was_playing {
            return self.last_position;
        }
        let pos = self.last_position + self.last_update.elapsed();
        if !self.last_duration.is_zero() && pos > self.last_duration {
            self.last_duration
        } else {
            pos
        }
    }

    /// Ratio 0.0..=1.0 para el Gauge de Ratatui + label de tiempo.
    pub fn smoothed_progress(&self) -> f64 {
        if self.last_duration.is_zero() {
            return 0.0;
        }
        (self.smoothed_position().as_secs_f64() / self.last_duration.as_secs_f64()).clamp(0.0, 1.0)
    }

    /// Track para mostrar: el ultimo conocido pero con posicion/progreso
    /// interpolados para que la barra fluya suave.
    pub fn display_track(&self) -> TrackInfo {
        let mut t = self.last_known();
        t.position = self.smoothed_position();
        t.duration = self.last_duration;
        t.progress = self.smoothed_progress();
        t.playing = self.was_playing;
        t
    }

    /// Restaura la ultima cancion guardada en disco (para mostrarla
    /// al arrancar aunque aun no haya sesion GSMTC). Queda pausada.
    pub fn restore_last_known(&mut self, track: TrackInfo) {
        self.last_position = track.position;
        self.last_duration = track.duration;
        self.last_update = Instant::now();
        self.was_playing = false;
        let mut t = track;
        t.playing = false;
        self.last_track = Some(t);
    }

    #[allow(dead_code)]
    pub fn has_history(&self) -> bool {
        self.last_track.is_some()
    }
}
