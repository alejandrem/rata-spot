//! Puerto CDP dinámico + orígenes acotados (P0 + P1).
//!
//! Problemas que resuelve:
//! - Puerto 9222 fijo: colisiona con otras herramientas y es un objetivo
//!   conocido (DNS-rebinding contra un puerto fijo y famoso).
//! - `--remote-allow-origins=*`: anula la defensa de Origin de Chromium;
//!   cualquier web visitada podría manejar el Brave por CDP.
//!
//! Estrategia: al lanzar se aparta un puerto libre de verdad (bind a
//! 127.0.0.1:0), se pasa explícito + orígenes acotados SOLO a ese
//! origen. El transporte prueba [nuestro puerto, 9222]: el segundo cubre
//! al Brave ya abierto con flags viejos (o a mano con 9222).
//! Kill-switch/debug: `RATA_SPOT_PORT=9333` fija el puerto.

use std::sync::atomic::{AtomicU16, Ordering};

use anyhow::{Context, Result};

/// Fallback para Brave depurable preexistente (flags viejos / a mano).
pub const DEFAULT_CDP_PORT: u16 = 9222;

/// Puerto elegido en este arranque (0 = aún no elegido → default).
static CHOSEN_PORT: AtomicU16 = AtomicU16::new(0);

/// Fija el puerto de este arranque (lo llama el lanzador antes del spawn).
pub fn set_cdp_port(port: u16) {
    CHOSEN_PORT.store(port, Ordering::SeqCst);
}

/// Puerto efectivo: env > elegido > default.
pub fn cdp_port() -> u16 {
    if let Ok(raw) = std::env::var("RATA_SPOT_PORT") {
        if let Ok(p) = raw.trim().parse::<u16>() {
            if p != 0 {
                return p;
            }
        }
    }
    match CHOSEN_PORT.load(Ordering::SeqCst) {
        0 => DEFAULT_CDP_PORT,
        p => p,
    }
}

/// Candidatos a probar, en orden (puro, testeable).
pub(crate) fn candidates(configured: u16) -> Vec<u16> {
    if configured == DEFAULT_CDP_PORT {
        vec![DEFAULT_CDP_PORT]
    } else {
        vec![configured, DEFAULT_CDP_PORT]
    }
}

/// Candidatos vivos para el transporte.
pub(crate) fn live_candidates() -> Vec<u16> {
    candidates(cdp_port())
}

/// Aparta un puerto libre en 127.0.0.1 (se suelta al instante; la ventana
/// de reuso es de ms y el uso es inmediato en el spawn).
pub fn pick_free_port() -> Result<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").context("sin loopback libre")?;
    listener
        .local_addr()
        .map(|a| a.port())
        .context("puerto libre ilegible")
}

/// Puerto a usar al lanzar: env válido o uno libre.
pub fn wanted_cdp_port() -> Result<u16> {
    if let Ok(raw) = std::env::var("RATA_SPOT_PORT") {
        let p = raw
            .trim()
            .parse::<u16>()
            .with_context(|| format!("RATA_SPOT_PORT invalido: '{raw}' (usa 1-65535 o quítalo)"))?;
        if p == 0 {
            anyhow::bail!("RATA_SPOT_PORT invalido: '0' (usa 1-65535 o quítalo)");
        }
        return Ok(p);
    }
    pick_free_port()
}

/// Flags de depuración para el spawn: puerto explícito + orígenes
/// acotados a loopback. NUNCA `*` (ver módulo).
pub fn debug_flags(port: u16) -> [String; 2] {
    [
        format!("--remote-debugging-port={port}"),
        format!("--remote-allow-origins=http://127.0.0.1:{port}"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidatos_sin_duplicar() {
        assert_eq!(candidates(9222), vec![9222]);
        assert_eq!(candidates(9333), vec![9333, 9222]);
    }

    #[test]
    fn flags_acotados_sin_wildcard() {
        let [port, origins] = debug_flags(9333);
        assert_eq!(port, "--remote-debugging-port=9333");
        assert_eq!(origins, "--remote-allow-origins=http://127.0.0.1:9333");
        assert!(!origins.contains('*'));
    }

    #[test]
    fn puerto_libre_es_enlazable() {
        let p = pick_free_port().expect("pick");
        assert!(p != 0);
        // Recién soltado: se puede enlazar de nuevo (ventana de reuso sana).
        let _l = std::net::TcpListener::bind(("127.0.0.1", p)).expect("rebind");
    }
}
