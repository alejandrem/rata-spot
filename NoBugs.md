# NoBugs owo — críticas que no son bugs 🐀

> Respuestas sinceras y riales de la rata a las críticas. No son bugs, son decisiones.
> Si algo de aquí te arde, es comportamiento esperado uwu.

---

## 1. Es scraping con traje bonito. Todo depende del DOM de open.spotify.com.

**Crítica original:**
> Tienes blindaje (`cdp/selectors/names.rs` + `health.rs`), pero si Spotify hace deploy mañana, mueres. Es deuda eterna. No es tu culpa, es el diseño.

**Respuesta rata owo:**
Sí, y lo sé. Es comportamiento esperado, y la verdad no me importa unu.

rata-spot nació para NO usar la API de Spotify. Si quisiera estabilidad 100% usaría la API oficial o la app nativa. Prefiero leer el DOM real por CDP y asumir el mantenimiento.

Para eso ya existe blindaje:
- todo el DOM vive en `src/cdp/selectors/` (una sola caja),
- `health_summary()` canta en el boot qué sonda murió (`DOM 6/6 ok` o `fallan: library_grid`),
- `cargo test diag_selectores -- --nocapture` te dice cuál mover,
- y `src/cdp/api/` con kill-switch `RATA_SPOT_API=0` cae a DOM si la red muere.

Si Spotify rediseña, se cambia UNA const en `names.rs` y listo. Deuda eterna aceptada owo.

---

## 2. Ahorro modesto para tanta complejidad. 4933 líneas + 86 archivos + Tokio + CDP + GSMTC para ahorrar ~100MB de RAM.

**Crítica original:**
> Funciona, pero el costo de mantenimiento es alto. Bus factor 1.

**Respuesta rata uwu:**
Vale la pena. 100MB de RAM por escuchar música vale oro owo.

- Web de Spotify: 300-400MB en Brave normal.
- App de Spotify: +1GB de RAM a veces.
- rata-spot: ~137MB total (`rata-spot.exe` ~17MB + Brave con dieta ~120MB).

Esos 100-200MB liberados se sienten en una PC humilde. Y las 4933 líneas están partidas en `1 carpeta = 1 dominio, 1 archivo = 1 tarea`, así que mantener no duele tanto. Bus factor 1, sí, pero documentado para que cualquier rata lo agarre (`docs/arquitectura.md`, `mantenimiento.md`, `bugs-resueltos-uwu.md`).

---

*Fin — no son bugs, son decisiones 🐀 owo*
