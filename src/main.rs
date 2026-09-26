mod launcher;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("rata-spot 🐀 — Fase 1+2: lanzador Brave");

    match launcher::find_brave_exe() {
        Some(p) => println!("brave.exe: {}", p.display()),
        None => println!("brave.exe: NO encontrado en rutas comunes"),
    }

    println!(
        "Brave ya corriendo: {}",
        if launcher::is_brave_running() {
            "si"
        } else {
            "no"
        }
    );

    let was_already_running = launcher::ensure_brave_running()?;
    if was_already_running {
        println!("Brave ya estaba abierto, no se lanzo nada nuevo.");
        println!("(Fase 3 conectara GSMTC aqui y mostrara 'Conectando...')");
    } else {
        println!("Brave lanzado con flags optimizados -> {}", launcher::SPOTIFY_URL);
        println!("(Sin delay fijo: la Fase 3 reintentara GSMTC hasta conectar)");
    }

    println!("\nPresiona Enter para salir y limpiar (si lo lanzamos nosotros)...");
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);

    launcher::cleanup(was_already_running);
    println!("bye rata 🐀");
    Ok(())
}
