//! Backend GSMTC (Fase 3).
//!
//! GSMTC = Global System Media Transport Controls, la API nativa de Windows
//! que expone titulo/artista/progreso y play/pause/next/prev.
//! Brave se registra solo, sin configuracion extra.
//!
//! Incluye la solucion al "progreso a saltos" de Spotify Web:
//! `ProgressSmoother` interpola con `Instant::elapsed()` entre updates.

use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession,
    GlobalSystemMediaTransportControlsSessionManager as SessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};

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

fn timespan_to_duration(ticks_100ns: i64) -> Duration {
    if ticks_100ns <= 0 {
        return Duration::ZERO;
    }
    // 1 tick = 100ns -> 10_000_000 ticks = 1s
    Duration::from_micros((ticks_100ns / 10) as u64)
}

/// Busca la sesion GSMTC cuya app sea Brave.
/// Retorna error si Brave aun no registro sesion (no listo todavia).
pub async fn get_brave_session() -> Result<Session> {
    // windows 0.58: IAsyncOperation no es Future -> usar .get() bloqueante.
    // Son llamadas COM locales rapidas (ms), aceptable en este loop 1s/250ms.
    // Si algun dia se congela la TUI, mover a spawn_blocking.
    let manager = SessionManager::RequestAsync()
        .context("RequestAsync GSMTC fallo (¿servicio Windows inactivo?)")?
        .get()
        .context("RequestAsync.get() fallo")?;

    let sessions = manager
        .GetSessions()
        .context("GetSessions fallo (falta feature Foundation_Collections?)")?;

    for i in 0..sessions.Size().unwrap_or(0) {
        let session = sessions.GetAt(i).context("GetAt sesion fallo")?;
        let app_id = session
            .SourceAppUserModelId()
            .map(|h| h.to_string())
            .unwrap_or_default()
            .to_lowercase();
        if app_id.contains("brave") {
            return Ok(session);
        }
    }

    anyhow::bail!("sesion Brave no encontrada todavia")
}

/// Lee titulo/artista/album + playing/paused + progreso.
/// Nunca deberia crashear al llamador: los campos faltantes van a default.
pub async fn get_track(session: &Session) -> Result<TrackInfo> {
    let props = session
        .TryGetMediaPropertiesAsync()
        .context("TryGetMediaPropertiesAsync fallo")?
        .get()
        .context("media properties .get() fallo (¿sin metadata ahora mismo?)")?;

    let title = props.Title().map(|h| h.to_string()).unwrap_or_default();
    let artist = props.Artist().map(|h| h.to_string()).unwrap_or_default();
    let album = props
        .AlbumTitle()
        .map(|h| h.to_string())
        .unwrap_or_default();

    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);

    let (position, duration) = session
        .GetTimelineProperties()
        .map(|tl| {
            let pos = tl.Position().map(|t| timespan_to_duration(t.Duration)).unwrap_or(Duration::ZERO);
            let end = tl.EndTime().map(|t| timespan_to_duration(t.Duration)).unwrap_or(Duration::ZERO);
            (pos, end)
        })
        .unwrap_or((Duration::ZERO, Duration::ZERO));

    let progress = if duration.is_zero() {
        0.0
    } else {
        (position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
    };

    Ok(TrackInfo {
        title: if title.is_empty() { "—".to_string() } else { title },
        artist: if artist.is_empty() { "—".to_string() } else { artist },
        album,
        playing,
        position,
        duration,
        progress,
    })
}

pub async fn play(session: &Session) -> Result<()> {
    session
        .TryPlayAsync()
        .context("TryPlayAsync fallo")?
        .get()
        .context("play .get() fallo")?;
    Ok(())
}

pub async fn pause(session: &Session) -> Result<()> {
    session
        .TryPauseAsync()
        .context("TryPauseAsync fallo")?
        .get()
        .context("pause .get() fallo")?;
    Ok(())
}

pub async fn next(session: &Session) -> Result<()> {
    session
        .TrySkipNextAsync()
        .context("TrySkipNextAsync fallo")?
        .get()
        .context("next .get() fallo")?;
    Ok(())
}

pub async fn prev(session: &Session) -> Result<()> {
    session
        .TrySkipPreviousAsync()
        .context("TrySkipPreviousAsync fallo")?
        .get()
        .context("prev .get() fallo")?;
    Ok(())
}

/// Si esta playing -> pause. Si esta paused (o cualquier otro) -> play.
pub async fn toggle(session: &Session) -> Result<()> {
    let is_playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);

    if is_playing {
        pause(session).await
    } else {
        play(session).await
    }
}

/// Loop reactivo (Fase 3.2): reintenta cada 1s indefinidamente.
/// No usa delay fijo de arranque; el llamador muestra "Conectando..."
/// mientras esto pende. Solo regresa cuando hay sesion Brave.
#[allow(dead_code)]
pub async fn wait_for_brave_session() -> Session {
    loop {
        match get_brave_session().await {
            Ok(session) => break session,
            Err(_) => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Progreso suave: Spotify Web manda TimelineProperties en rafagas cada varios
// segundos -> la barra se moveria a saltos. Interpolamos con el reloj local.
// ---------------------------------------------------------------------------

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

/// Ruta del historial (UTF-8, emojis incluidos sin problema).
fn last_track_path() -> std::path::PathBuf {
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    std::path::PathBuf::from(base).join("rata-spot-last-track.txt")
}

/// Guarda la ultima cancion en disco. Formato simple de 6 lineas.
/// Se llama solo cuando cambia de cancion (no cada tick).
pub fn save_last_track(track: &TrackInfo) {
    // Los titulos con salto de linea romperian el formato -> aplanar.
    let flat = |s: &str| s.replace('\n', " ").replace('\r', " ");
    let content = format!(
        "{}\n{}\n{}\n{}\n{}\n{}\n",
        flat(&track.title),
        flat(&track.artist),
        flat(&track.album),
        track.position.as_secs(),
        track.duration.as_secs(),
        if track.playing { "1" } else { "0" },
    );
    let _ = std::fs::write(last_track_path(), content);
}

/// Carga la ultima cancion guardada, si existe.
pub fn load_last_track() -> Option<TrackInfo> {
    let content = std::fs::read_to_string(last_track_path()).ok()?;
    let mut lines = content.lines();
    let title = lines.next()?.to_string();
    let artist = lines.next().unwrap_or("—").to_string();
    let album = lines.next().unwrap_or("").to_string();
    let position = lines
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::ZERO);
    let duration = lines
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::ZERO);
    if title.is_empty() || title == "—" {
        return None;
    }
    let progress = if duration.is_zero() {
        0.0
    } else {
        (position.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0)
    };
    Some(TrackInfo {
        title,
        artist,
        album,
        playing: false, // al arrancar siempre pausado (historial)
        position,
        duration,
        progress,
    })
}
