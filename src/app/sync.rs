//! Sync GSMTC por tick: reintento de sesion + refresh del track.
//!
//! Incluye los antidotos de barra congelada (sin costo extra):
//! - sesion preferida = la que suena (no una vieja pausada)
//! - posicion que avanza => sonando (status mentiroso)
//! - duracion reutilizada si falta el timeline
//! - sesion muerta (12 fallos ~3s) => soltar y reconectar
//! Contadores en u32: el u8 anterior podia hacer overflow en debug
//! si desconectado >40s (panic por overflow en dev).

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
    // Conexion inicial / reconexion (loop reactivo Fase 3.2).
    if session.is_none() {
        sync.retry_ticks += 1;
        if sync.retry_ticks >= 4 {
            sync.retry_ticks = 0;
            if let Ok(s) = gsmtc::get_brave_session().await {
                *session = Some(s);
                state.connected = true;
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
