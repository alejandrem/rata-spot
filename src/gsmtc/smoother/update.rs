//! Update: aplica un `get_track()` fresco sin congelar la barra.
//! Free-run si GSMTC esta clavado (<500ms), resincroniza si hay salto real.

use std::time::{Duration, Instant};

use super::ProgressSmoother;
use super::super::track::TrackInfo;

/// Llamar cada vez que llega un `get_track()` fresco y OK.
///
/// Free-run: si sonando la posicion nueva es ~igual a la guardada
/// (<500ms de diferencia), GSMTC esta congelado (Spotify a veces
/// clava Position en ~40ms y nunca lo mueve) y NO se resetea el reloj:
/// se sigue interpolando con `elapsed()`. Solo saltos reales (>500ms:
/// rafagas normales, seek, cambio de rola) resincronizan.
pub fn apply_update(s: &mut ProgressSmoother, fresh: TrackInfo) {
    let frozen = fresh.playing
        && s.was_playing
        && !fresh.duration.is_zero()
        && diff_ms(fresh.position, s.last_position) < 500;
    if !frozen {
        s.last_position = fresh.position;
        s.last_duration = fresh.duration;
        s.last_update = Instant::now();
    } else if fresh.duration != s.last_duration && !fresh.duration.is_zero() {
        s.last_duration = fresh.duration;
    }
    s.was_playing = fresh.playing;
    s.last_track = Some(fresh);
}

/// Diferencia absoluta en ms entre dos Duration.
pub(crate) fn diff_ms(a: Duration, b: Duration) -> u128 {
    a.as_millis().abs_diff(b.as_millis())
}

impl ProgressSmoother {
    pub fn update(&mut self, fresh: TrackInfo) {
        apply_update(self, fresh);
    }
}
