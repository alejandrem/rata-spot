mod cdp;
mod gsmtc;
mod launcher;
mod ui;

use std::io::{self, Stdout};
use std::time::Duration;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use ui::AppState;

type Tui = Terminal<CrosstermBackend<Stdout>>;

#[tokio::main]
async fn main() -> Result<()> {
    // Arranque: si YA hay sesion GSMTC de Brave (Spotify sonando en
    // cualquier ventana), reutilizarla y no lanzar nada. Si no, abrir
    // VENTANA NUEVA del mismo Brave (tu perfil logueado), nunca pestaña
    // en tu ventana actual y nunca perfil fresh sin cuentas.
    let reuse_existing = gsmtc::get_brave_session().await.is_ok();
    let (was_already_running, boot_status) = if reuse_existing {
        (
            true,
            "Reutilizando sesion Brave existente (ya sonaba Spotify).".to_string(),
        )
    } else {
        let owned = launcher::launch_brave_spotify().await?;
        let pid = launcher::child_pid()
            .map(|p| p.to_string())
            .unwrap_or_else(|| "?".to_string());
        (
            owned,
            format!(
                "Brave ventana NUEVA visible PID {} | {} | dale play una vez",
                pid,
                launcher::SPOTIFY_URL
            ),
        )
    };

    // 1. Setup terminal (modo raw + pantalla alternativa).
    // NOTA: todo println! anterior queda oculto bajo la TUI; por eso
    // el diagnostico viaja en `boot_status` y se pinta en el centro.
    let mut terminal = setup_terminal()?;

    // 3-4. Loop TUI (conecta GSMTC con reintentos + dibuja + teclas).
    let app_result = run_app(&mut terminal, was_already_running, boot_status).await;

    // 5. Restaurar terminal SIEMPRE, aun si hubo error.
    restore_terminal(&mut terminal);

    // 6. Cleanup ya ocurrio dentro de run_app (pause + kill si lo lanzamos).
    if let Err(e) = &app_result {
        eprintln!("rata-spot fallo: {:#}", e);
        return app_result;
    }

    println!("bye rata 🐀");
    Ok(())
}

fn setup_terminal() -> Result<Tui> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Tui) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

async fn run_app(terminal: &mut Tui, was_already_running: bool, boot_status: String) -> Result<()> {
    let mut state = AppState::new();
    state.status = boot_status;
    // Historial: mostrar ultima cancion escuchada aunque aun no haya sesion.
    if let Some(last) = gsmtc::load_last_track() {
        state.smoother.restore_last_known(last);
    }
    let mut session: Option<gsmtc::Session> = None;
    // Reintento de conexion ~1s (4 ticks x 250ms) para no martillar COM.
    let mut ticks_since_retry: u8 = 99;
    let mut saved_title = state.smoother.last_known().title;
    // Fallos seguidos de get_track: si la ventana se cerro a mano, la
    // sesion queda muerta y hay que soltarla para reconectar (12 ticks ~3s;
    // los huecos de cambio de cancion son mas cortos y no la disparan).
    let mut track_fail_streak: u8 = 0;

    // Biblioteca en FONDO: leer el DOM tarda segundos (scroll virtualizado);
    // no bloquear el arranque. La sidebar muestra "cargando..." mientras.
    let (pl_tx, mut pl_rx) =
        tokio::sync::mpsc::channel::<Result<Vec<cdp::LibraryItem>, String>>(1);
    tokio::spawn(async move {
        for _ in 0..20 {
            if cdp::ensure_spotify_tab().await.is_ok() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let res = cdp::library_items()
            .await
            .map_err(|e| e.to_string());
        let _ = pl_tx.send(res).await;
    });

    loop {
        // Recoger biblioteca cuando el fondo termine (sin bloquear).
        if let Ok(res) = pl_rx.try_recv() {
            match res {
                Ok(items) => {
                    state.library = items;
                    state.pl_index = 0;
                    state.pl_state.select(Some(0));
                    state.pl_msg.clear();
                }
                Err(e) => state.pl_msg = format!("biblioteca: {e}"),
            }
        }

        // a. Dibujar (si no conectado, el centro muestra "Conectando...").
        terminal.draw(|f| ui::render(f, &mut state))?;

        // b. Poll 250ms: balance responsividad/CPU (no <100ms ni >1000ms).
        // c. Teclado.
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                // Windows manda Press + Repeat (sostenida) + Release por tecla.
                // Sin filtrar, 1 tap = Press+Release = 2 toggles (pausa fantasma
                // que se deshace sola) y sostener = flicker aleatorio.
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                // Solo flechas aceptan Repeat (con throttle 120ms); las
                // acciones (space/n/p/enter/l) solo responden a Press.
                let is_repeat = key.kind == KeyEventKind::Repeat;
                match key.code {
                    KeyCode::Char(' ') => {
                        if is_repeat {
                            continue;
                        }
                        if let Some(ref s) = session {
                            // Ya suena o pausado con contexto -> GSMTC basta.
                            let _ = gsmtc::toggle(s).await;
                        } else {
                            // Pagina fresca sin sesion: GSMTC no puede;
                            // click al Play via CDP para arrancar sin mouse.
                            state.status = "▶ iniciando Spotify...".to_string();
                            // Redibujar ANTES del await (el click tarda segundos
                            // y el loop se queda esperando aqui).
                            let _ = terminal.draw(|f| ui::render(f, &mut state));
                            match cdp::play_from_scratch().await {
                                Ok(_) => {
                                    state.status =
                                        "▶ play enviado, conectando...".to_string()
                                }
                                Err(e) => {
                                    // Mensaje especifico segun la causa:
                                    // el generico "dale play" no decia nada.
                                    let msg = e.to_string();
                                    state.status = if msg.contains("sin puerto CDP") {
                                        "cierra Brave por completo y haz cargo run".to_string()
                                    } else if msg.contains("no hay pestana") {
                                        "abre la pestana Spotify en Brave".to_string()
                                    } else {
                                        "dale play una vez en Brave".to_string()
                                    };
                                }
                            }
                        }
                    }
                    KeyCode::Char('n') => {
                        if is_repeat {
                            continue;
                        }
                        if let Some(ref s) = session {
                            let _ = gsmtc::next(s).await;
                        }
                    }
                    KeyCode::Char('p') => {
                        if is_repeat {
                            continue;
                        }
                        if let Some(ref s) = session {
                            let _ = gsmtc::prev(s).await;
                        }
                    }
                    // Biblioteca (sidebar) y canciones (centro).
                    // Flechas con throttle: sostenida avanza legible.
                    KeyCode::Char('j') | KeyCode::Down => {
                        if state.nav_ok() {
                            match state.view {
                                ui::View::Library => state.pl_move(1),
                                ui::View::Tracks => state.tr_move(1),
                            }
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if state.nav_ok() {
                            match state.view {
                                ui::View::Library => state.pl_move(-1),
                                ui::View::Tracks => state.tr_move(-1),
                            }
                        }
                    }
                    KeyCode::Char('l') => {
                        if is_repeat {
                            continue;
                        }
                        // Recargar biblioteca a mano (por si se abrio tarde).
                        state.pl_msg = "cargando biblioteca...".to_string();
                        let _ = terminal.draw(|f| ui::render(f, &mut state));
                        match cdp::library_items().await {
                            Ok(items) => {
                                state.library = items;
                                state.pl_index = 0;
                                state.pl_state.select(Some(0));
                                state.pl_msg.clear();
                            }
                            Err(e) => state.pl_msg = format!("biblioteca: {e}"),
                        }
                    }
                    // → o Enter: en biblioteca abre las canciones de la
                    // playlist; en canciones toca la rola elegida.
                    KeyCode::Right | KeyCode::Enter => {
                        if is_repeat {
                            continue;
                        }
                        match state.view {
                            ui::View::Library => {
                                if let Some(item) = state.pl_selected().cloned() {
                                    state.tr_playlist = item.name.clone();
                                    state.tracks.clear();
                                    state.tr_index = 0;
                                    state.tr_state.select(Some(0));
                                    state.tr_msg = "abriendo playlist...".to_string();
                                    state.view = ui::View::Tracks;
                                    let _ = terminal.draw(|f| ui::render(f, &mut state));
                                    match cdp::open_playlist(&item.uri).await {
                                        Ok(()) => match cdp::playlist_tracks().await {
                                            Ok(tracks) => {
                                                state.tracks = tracks;
                                                state.tr_index = 0;
                                                state.tr_state.select(Some(0));
                                                state.tr_msg.clear();
                                            }
                                            Err(e) => {
                                                state.tr_msg = format!("canciones: {e:.80}")
                                            }
                                        },
                                        Err(e) => {
                                            state.tr_msg = format!("no abre: {e:.80}")
                                        }
                                    }
                                }
                            }
                            ui::View::Tracks => {
                                if let Some(t) = state.tr_selected().cloned() {
                                    state.status = format!("▶ tocando {}...", t.title);
                                    let _ = terminal.draw(|f| ui::render(f, &mut state));
                                    match cdp::play_track(&t.id).await {
                                        Ok(_) => {
                                            state.status =
                                                "▶ reproduciendo, conectando...".to_string()
                                        }
                                        Err(e) => {
                                            state.status = format!("no sono: {e:.60}");
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // ← o Esc: volver a la biblioteca.
                    KeyCode::Left | KeyCode::Esc => state.back_to_library(),
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        // Conexion inicial / reconexion (loop reactivo Fase 3.2).
        if session.is_none() {
            ticks_since_retry += 1;
            if ticks_since_retry >= 4 {
                ticks_since_retry = 0;
                if let Ok(s) = gsmtc::get_brave_session().await {
                    session = Some(s);
                    state.connected = true;
                }
            }
            continue;
        }

        // d. Refresh track (si falla: ultimo conocido, sin crashear).
        // Si la sesion murio (Brave cerrado a mano), intentar reconectar.
        if let Some(ref s) = session {
            match gsmtc::get_track(s).await {
                Ok(fresh) => {
                    track_fail_streak = 0;
                    state.smoother.update(fresh.clone());
                    // Persistir solo al cambiar de cancion (no cada tick).
                    if fresh.title != saved_title {
                        saved_title = fresh.title.clone();
                        gsmtc::save_last_track(&fresh);
                    }
                }
                Err(_) => {
                    // Sin metadata momentanea (cambio de cancion): conservar
                    // lo ultimo. Solo si falla seguido ~3s, la sesion murio
                    // (cerraste la ventana) -> soltar y reconectar.
                    track_fail_streak += 1;
                    if track_fail_streak >= 12 {
                        session = None;
                        state.connected = false;
                        state.status = "sesion perdida, reintentando...".to_string();
                        track_fail_streak = 0;
                    }
                }
            }
        }
    }

    // Parar la musica al salir (q): pausa best-effort + cleanup.
    // Si Brave lo lanzamos nosotros, cleanup() ademas lo mata.
    // Si ya estaba abierto, lo dejamos vivo pero pausado.
    if let Some(ref s) = session {
        let _ = gsmtc::pause(s).await;
    }
    launcher::cleanup(was_already_running);
    Ok(())
}
