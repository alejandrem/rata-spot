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
    ///
    /// Free-run: si sonando la posicion nueva es ~igual a la guardada
    /// (<500ms de diferencia), GSMTC esta congelado (Spotify a veces
    /// clava Position en ~40ms y nunca lo mueve) y NO se resetea el reloj:
    /// se sigue interpolando con `elapsed()`. Solo saltos reales (>500ms:
    /// rafagas normales, seek, cambio de rola) resincronizan.
    pub fn update(&mut self, fresh: TrackInfo) {
        let frozen = fresh.playing
            && self.was_playing
            && !fresh.duration.is_zero()
            && diff_ms(fresh.position, self.last_position) < 500;
        if !frozen {
            self.last_position = fresh.position;
            self.last_duration = fresh.duration;
            self.last_update = Instant::now();
        } else if fresh.duration != self.last_duration && !fresh.duration.is_zero() {
            self.last_duration = fresh.duration;
        }
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
}

/// Diferencia absoluta en ms entre dos Duration.
fn diff_ms(a: Duration, b: Duration) -> u128 {
    a.as_millis().abs_diff(b.as_millis())
}

#[cfg(test)]
mod tests {
    use super::super::track::TrackInfo;
    use super::*;

    fn mk(playing: bool) -> TrackInfo {
        TrackInfo {
            title: "t".to_string(),
            artist: "a".to_string(),
            album: String::new(),
            playing,
            position: Duration::from_millis(39),
            duration: Duration::from_secs(202),
            progress: 0.0,
        }
    }

    /// Posicion GSMTC clavada en 39ms (caso real): la barra debe avanzar
    /// con el reloj local en vez de quedarse en 0:00.
    #[test]
    fn free_run_con_posicion_congelada() {
        let mut s = ProgressSmoother::new();
        s.update(mk(true));
        std::thread::sleep(Duration::from_millis(300));
        s.update(mk(true)); // mismo valor congelado
        let d = s.display_track();
        assert!(d.playing);
        assert!(
            d.position >= Duration::from_millis(250),
            "barra congelada: {:?}",
            d.position
        );
        assert!(d.progress > 0.0);
    }

    /// Pausa real congela la barra (no confundir con freeze de GSMTC).
    #[test]
    fn pausa_congela() {
        let mut s = ProgressSmoother::new();
        s.update(mk(true));
        s.update(mk(false));
        let p1 = s.display_track().position;
        std::thread::sleep(Duration::from_millis(200));
        let p2 = s.display_track().position;
        assert_eq!(p1, p2);
    }

    /// Salto real (>500ms) resincroniza en vez de free-run.
    #[test]
    fn salto_resincroniza() {
        let mut s = ProgressSmoother::new();
        s.update(mk(true));
        let mut jump = mk(true);
        jump.position = Duration::from_secs(60);
        s.update(jump);
        let p = s.display_track().position;
        assert!(
            p >= Duration::from_secs(60) && p < Duration::from_secs(61),
            "no resincronizo: {p:?}"
        );
    }
}
