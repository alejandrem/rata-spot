//! Tests del smoother: free-run, pausa y salto real.

use std::time::Duration;

use super::super::track::TrackInfo;
use super::ProgressSmoother;

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
