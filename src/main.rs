//! rata-spot 🐀 — Spotify liviano: Brave minimizado + TUI Ratatui.
//!
//! Punto de entrada delgado: el trabajo vive en `app/` por tareas.
//! Flujo: boot (sesion Brave) -> terminal -> run_app (loop) -> cleanup.

mod app;
mod cdp;
mod gsmtc;
mod launcher;
mod ui;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 2. Lanzar Brave si no hay sesion (o reutilizar la existente).
    let (was_already_running, boot_status) = app::boot::boot().await?;

    // 1. Setup terminal (modo raw + pantalla alternativa).
    // NOTA: todo println! anterior queda oculto bajo la TUI; por eso
    // el diagnostico viaja en `boot_status` y se pinta en el centro.
    let mut terminal = app::terminal::setup_terminal()?;

    // 3-4. Loop TUI (conecta GSMTC con reintentos + dibuja + teclas).
    let app_result = app::run::run_app(&mut terminal, was_already_running, boot_status).await;

    // 5. Restaurar terminal SIEMPRE, aun si hubo error.
    app::terminal::restore_terminal(&mut terminal);

    // 6. Cleanup ya ocurrio dentro de run_app (pause + kill si lo lanzamos).
    if let Err(e) = &app_result {
        eprintln!("rata-spot fallo: {:#}", e);
        return app_result;
    }

    println!("bye rata 🐀");
    Ok(())
}
