//! Backend GSMTC (Fase 3).
//!
//! GSMTC = Global System Media Transport Controls, la API nativa de Windows
//! que expone titulo/artista/progreso y play/pause/next/prev.
//! Brave se registra solo, sin configuracion extra.
//!
//! Tareas (1 archivo = 1 tarea):
//! - track: tipos TrackInfo + conversiones de tiempo
//! - session: sesion Brave, lectura y controles de transporte
//! - smoother: progreso suave (interpola entre rafagas de Spotify)
//! - history: ultima cancion persistida en disco (UTF-8)

pub mod history;
pub mod session;
pub mod smoother;
pub mod track;

pub use history::{load_last_track, save_last_track};
pub use session::{get_brave_session, get_track, next, pause, prev, toggle};
pub use smoother::ProgressSmoother;
pub use track::{Session, TrackInfo};
#[allow(unused_imports)]
pub use track::fmt_duration;
