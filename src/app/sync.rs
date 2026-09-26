//! Sync GSMTC por tick: reintento de sesion + refresh del track.
//!
//! Incluye los antidotos de barra congelada (sin costo extra):
//! - sesion preferida = la que suena (no una vieja pausada)
//! - posicion que avanza => sonando (status mentiroso)
//! - duracion reutilizada si falta el timeline
//! - sesion muerta (12 fallos ~3s) => soltar y reconectar
//! Contadores en u32: el u8 anterior podia hacer overflow en debug
//! si desconectado >40s (panic por overflow en dev).

use std::collections::HashMap;
use std::time::Duration;

use crate::{
    gsmtc,
    ui::AppState,
};

/// Estado mutable del sync entre ticks (vive en run_app).
pub struct SyncState {
    saved_title: String,
    fail_streak: u32,
    retry_ticks: u32,
    /// Backoff exponencial en ticks (4 ticks = 1s): 4, 8, 16, 32.
    /// Sin esto se martilla COM cada 1s eternamente cuando Brave murio.
    cooldown_ticks: u32,
    backoff_step: u32,
    /// Ultimo estado visto por titulo (para detectar que sesion cambio).
    seen: HashMap<String, bool>,
    scan_ticks: u32,
    last_raw_pos: Duration,
    last_good_dur: Duration,
    last_good_title: String,
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

/// Un tick: si no hay sesion, reintentar ~1s (4 ticks x 250ms para no
/// martillar COM); si hay, refrescar track sin crashear jamas.
pub async fn tick_gsmtc(
    state: &mut AppState,
    session: &mut Option<gsmtc::Session>,
    sync: &mut SyncState,
) {
    // Conexion inicial / reconexion con backoff exponencial:
    // intento inmediato, luego 1s, 2s, 4s, 8s (tope). Sin backoff se
    // martilla COM cada 1s eternamente cuando Brave murio.
    if session.is_none() {
        if sync.cooldown_ticks > 0 {
            sync.cooldown_ticks -= 1;
            return;
        }
        sync.retry_ticks += 1;
        if sync.retry_ticks >= 4 {
            sync.retry_ticks = 0;
            if let Ok(s) = gsmtc::get_brave_session().await {
                *session = Some(s);
                state.connected = true;
                sync.cooldown_ticks = 0;
                sync.backoff_step = 0;
            } else {
                sync.cooldown_ticks = 4u32.saturating_mul(1 << sync.backoff_step.min(3));
                sync.backoff_step = sync.backoff_step.saturating_add(1);
            }
        }
        return;
    }

    // Refresh track (si falla: ultimo conocido, sin crashear).
    // Si la sesion murio (Brave cerrado a mano), intentar reconectar.
    if let Some(ref s) = session {
        match gsmtc::get_track(s).await {
            Ok(mut fresh) => {
                sync.fail_streak = 0;
                if fresh.position > sync.last_raw_pos {
                    fresh.playing = true;
                }
                sync.last_raw_pos = fresh.position;
                if fresh.duration.is_zero()
                    && fresh.title == sync.last_good_title
                    && !sync.last_good_dur.is_zero()
                {
                    fresh.duration = sync.last_good_dur;
                    fresh.progress = (fresh.position.as_secs_f64()
                        / fresh.duration.as_secs_f64())
                    .clamp(0.0, 1.0);
                }
                if !fresh.duration.is_zero() {
                    sync.last_good_dur = fresh.duration;
                    sync.last_good_title = fresh.title.clone();
                }
                state.smoother.update(fresh.clone());
                // Persistir solo al cambiar de cancion (no cada tick).
                if fresh.title != sync.saved_title {
                    sync.saved_title = fresh.title.clone();
                    gsmtc::save_last_track(&fresh);
                }
                // Multi-sesion (~1s): si la actual esta pausada y OTRA
                // cambio a sonando (ej. diste Enter y empezo Spotify
                // mientras un video sonaba), cambiarse a ella. No oscila:
                // con la actual sonando jamas se cambia.
                sync.scan_ticks += 1;
                if sync.scan_ticks >= 4 && !fresh.playing {
                    sync.scan_ticks = 0;
                    switch_if_changed(state, session, sync, &fresh.title).await;
                }
            }
            Err(_) => {
                // Sin metadata momentanea (cambio de cancion): conservar
                // lo ultimo. Solo si falla seguido ~3s, la sesion murio
                // (cerraste la ventana) -> soltar y reconectar.
                sync.fail_streak += 1;
                if sync.fail_streak >= 12 {
                    *session = None;
                    state.connected = false;
                    state.status = "sesion perdida, reintentando...".to_string();
                    sync.fail_streak = 0;
                }
            }
        }
    }
}

/// Cambia a otra sesion Brave solo si FLIPPEO a sonando y no es la actual.
/// Anti-oscilacion: con la actual sonando jamas se cambia; el mapa `seen`
/// recuerda playing por titulo entre escaneos (~1s).
async fn switch_if_changed(
    state: &mut AppState,
    session: &mut Option<gsmtc::Session>,
    sync: &mut SyncState,
    cur_title: &str,
) {
    let all = match gsmtc::brave_sessions().await {
        Ok(a) if a.len() > 1 => a,
        _ => return, // 0-1 sesiones: nada que elegir
    };
    let cur_key = cur_title.to_lowercase();
    let mut candidate: Option<gsmtc::Session> = None;
    for s in &all {
        let (playing, title) = match gsmtc::get_track(s).await {
            Ok(t) => (t.playing, t.title.to_lowercase()),
            Err(_) => continue,
        };
        let was = sync.seen.get(&title).copied();
        sync.seen.insert(title.clone(), playing);
        if playing && was != Some(true) && title != cur_key {
            candidate = Some(s.clone());
        }
    }
    if sync.seen.len() > 40 {
        sync.seen.clear();
    }
    if let Some(s) = candidate {
        *session = Some(s);
        state.connected = true;
    }
}
