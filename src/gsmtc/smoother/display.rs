//! Lectura: posicion interpolada + ultimo track conocido.

use std::time::Duration;

use super::ProgressSmoother;
use super::super::track::TrackInfo;

impl ProgressSmoother {
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
        use std::time::Instant;
        self.last_position = track.position;
        self.last_duration = track.duration;
        self.last_update = Instant::now();
        self.was_playing = false;
        let mut t = track;
        t.playing = false;
        self.last_track = Some(t);
    }
}
