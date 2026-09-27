//! Tick: un tick de sync (reconexion con backoff + refresh sin crashear).

use crate::{gsmtc, ui::AppState};

use super::{state::SyncState, switch::switch_if_changed};

/// Un tick: si no hay sesion, reintentar ~1s (4 ticks x 250ms para no
/// martillar COM); si hay, refrescar track sin crashear jamas.
pub async fn tick_gsmtc(
    state: &mut AppState,
    session: &mut Option<gsmtc::Session>,
    sync: &mut SyncState,
) {
    // Conexion inicial / reconexion con backoff: inmediato, 1s, 2s, 4s, 8s.
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
                if fresh.title != sync.saved_title {
                    sync.saved_title = fresh.title.clone();
                    gsmtc::save_last_track(&fresh);
                }
                // Multi-sesion (~1s): si la actual esta pausada y OTRA
                // flippeo a sonando, cambiarse. Con la actual sonando no.
                sync.scan_ticks += 1;
                if sync.scan_ticks >= 4 && !fresh.playing {
                    sync.scan_ticks = 0;
                    switch_if_changed(state, session, sync, &fresh.title).await;
                }
            }
            Err(_) => {
                // Sin metadata momentanea: conservar lo ultimo. Solo si
                // falla seguido ~3s, la sesion murio -> soltar y reconectar.
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
