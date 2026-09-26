# mantenimiento 🐀

Cómo habla rata-spot con Brave (qué se manda y qué regresa) y cómo
arreglarlo cuando Spotify mueva los muebles. Lee esto antes de tocar
cualquier cosa de `src/cdp/`.

---

## 1. Las 3 capas de cada petición

Toda lectura o click viaja por 3 capas. De afuera hacia adentro:

```
TUI (keys.rs / library.rs)
  → 1. HTTP crudo a localhost:9222      (transport.rs)
  → 2. WebSocket de la pestaña          (client.rs)
  → 3. JavaScript dentro de la página   (selectors.rs / api.rs)
```

### Capa 1 — HTTP crudo (`cdp/transport.rs`)

Sin `reqwest` a propósito: solo se necesita `GET` a localhost.

| Endpoint       | Para qué                                    |
|---|---|
| `/json/list`   | Listar pestañas y hallar la de Spotify      |
| `/json/version`| Hallar el WS del navegador (crear pestaña) |

Reglas aprendidas a golpes (ver `bugs-resueltos-uwu.md` #10 y #11):

- Hablar **HTTP/1.1** con `Host:` + `Connection: close`. Con 1.0 el
  servidor cierra con 0 bytes sin decir nada.
- Leer por **`Content-Length`** con fallback a `chunked`. Jamás
  `read_to_end` (a veces no cierra y cuelga eterno).
- Timeout global de **8s**: si cuelga, fallar rápido con mensaje claro.

Lo que regresa `/json/list` (recortado):

```json
[{"type": "page", "url": "https://open.spotify.com/...",
  "webSocketDebuggerUrl": "ws://127.0.0.1:9222/devtools/page/ABC123"}]
```

`spotify_ws_url()` (`cdp/tabs.rs`) filtra `type == "page"` y url con
`open.spotify`, y regresa el `webSocketDebuggerUrl`. Si no hay ninguna:
error `"no hay pestana Spotify"` y `ensure_spotify_tab()` la crea UNA
vez con `Target.createTarget` (sin duplicar).

### Capa 2 — WebSocket genérico (`cdp/client.rs`)

`cdp_call(ws_url, id, method, params)` abre un WS nuevo por llamada,
manda y espera la respuesta con ese `id`:

```json
// lo que SE MANDA
{"id": 41, "method": "Runtime.evaluate",
 "params": {"expression": "<JS>", "returnByValue": true, "awaitPromise": true}}
```

```json
// lo que REGRESA (éxito)
{"id": 41, "result": {"result": {"value": "<string con JSON adentro>"}}}
// lo que REGRESA (fallo CDP)
{"id": 41, "error": {"message": "..."}}
```

En Rust se lee con `v.pointer("/result/result/value")`. Timeouts
anti-cuelgue: **5s** conectando, **12s** leyendo por default
(`cdp_call_t` permite más: biblioteca/tracklist/dashboard usan **30s**,
el fetch de API usa **15s** — los snapshots con scroll tardan segundos
legítimos en máquinas lentas).

Métodos CDP que usamos (y nada más):

| Método                    | Dónde / para qué                                            |
|---|---|
| `Runtime.evaluate`        | Todo: leer DOM, hacer `fetch` dentro de la página, navegar SPA |
| `Page.navigate`           | Fallback cuando la navegación SPA no cambia la ruta en ~2s  |
| `Page.reload`             | Des-wed dear el player fantasma (dice Pausar sin audio ni sesión) |
| `Page.bringToFront`       | Despertar virtualizadores atorados por ventana ocluida      |
| `Input.dispatchKeyEvent`  | **Espacio real** (keyDown+keyUp): lo que haría tu dedo; los clicks `.click()` por JS son *untrusted* y a veces no arrancan audio |
| `Target.createTarget`     | Crear la pestaña Spotify si falta (vía el WS de `/json/version`) |

### Capa 3 — JavaScript dentro de la página

Hay DOS sabores, en este orden de preferencia:

**a) JSON/red (`cdp/api.rs`)** — `fetch` DENTRO de la página (lleva
cookies y login de Brave). Lo que se manda:

```js
fetch(url, {credentials: 'include', headers: {'Accept': 'application/json'}})
// con token: headers['Authorization'] = 'Bearer ' + token
```

y regresa `{status, body}` recortado a 30000 chars. Hoy intenta:

1. `GET https://open.spotify.com/get_access_token?reason=transport&productType=web-player`
   → `{accessToken: "..."}` (actualmente responde **403**: flujo viejo
   retirado; la capa cae al DOM y todo sigue funcionando).
2. `GET https://api.spotify.com/v1/search?q=...&type=track,artist,playlist,album&limit=12`
   con ese token → JSON documentado con `{uri, name, artists}`.

**b) DOM (`cdp/selectors.rs`)** — evaluates que leen el documento y
regresan **string con JSON adentro** (doble parseo: primero el CDP,
luego `serde_json::from_str` en Rust). Si algo falta, el JS regresa
`{error:'no-grid'}` / `{error:'no-tracklist'}` / `{error:'no-track-page'}`
y Rust lo convierte en `anyhow::bail!` legible. Los bloques grandes
(`LIBRARY_JS`, `SEARCH_JS`, `TRACKS_JS`, `DASHBOARD_JS`, `PLAY_CLICK_JS`)
viven TODOS en `selectors.rs`; los módulos solo los importan y parsean.

---

## 2. Flujos completos (qué viaja en cada acción)

- **Buscar (`/` + Enter):** `search()` → 1) `api::search_via_api`
  (12s max, sin navegar, sin tocar la música) → si hay items, listo.
  2) Fallback DOM: `spa_navigate(/search/...)` (`history.pushState` +
  `popstate`, sin recargar para no matar el audio) + `SEARCH_JS` con
  reintentos 8s. Respuesta: hasta 24 items `{uri, name, detail, kind}`
  (tracks primero).
- **Biblioteca (fondo al arrancar):** `library_items()` → `LIBRARY_JS`
  (scroll del sidebar virtualizado, dedupe por URI, 30s max, 3 intentos).
- **Abrir playlist (→):** `open_playlist()` espera el contenedor
  `[data-testid="playlist-tracklist"]` (10s) → `playlist_tracks()` lee
  filas por `a[data-testid="internal-track-link"]` (30s max).
- **Tocar rola (Enter):** en tracklist, click al botón de LA fila exacta
  (por track id, nada de lotería); en resultados, `play_uri()`: navega a
  la página del URI + **Espacio real** + espera sesión GSMTC (3s entre
  intentos para no pausar lo recién prendido).
- **Primer play ([space] sin sesión):** `play_from_scratch()` =
  `ensure_spotify_tab()` + `PLAY_CLICK_JS` (player-bar primero, luego
  héroes ES/EN, solo botones visibles) hasta 15s.
- **Dashboard (Enter en resultado):** `track_detail()` lee
  `section[data-testid="track-page"]` (header + letra, 30s max).
- **Health-check (en `boot.rs`, best-effort 4s):** `health_summary()`
  corre UN evaluate (`HEALTH_JS`, 6 sondas, cero navegación) y pinta
  `DOM 6/6 ok` o `DOM 5/6 ok, fallan: library_grid` en el status de la
  TUI. Sin pestaña aún → `DOM check pendiente` (no es error).

---

## 3. Mantenimiento a futuro (runbook cuando algo se rompa)

### Si un selector murió (lo más común: rediseño de Spotify)

1. Corre el health, que te dice CUÁL murió sin adivinar:
   ```powershell
   cargo test diag_selectores -- --nocapture
   # DIAG-SEL: DOM 5/6 ok, fallan: library_grid
   ```
2. Mira el DOM real con las sondas (solo imprimen, siempre pasan):
   ```powershell
   cargo test diag_estado -- --nocapture        # grid, rowcount, sidebar, url
   cargo test diag_play_buttons -- --nocapture  # inventario de botones Play
   ```
3. Cambia UNA const en `src/cdp/selectors.rs` (ahí vive todo) y corre:
   ```powershell
   cargo test  # js_menciona_sus_consts grita si el JS quedó divorciado de la const
   ```

Reglas al elegir el reemplazo (orden de estabilidad):

1. `data-testid` → 2. `role` + `href` con IDs (`/track/{id}` es contrato
   de URL) → 3. `aria-label` por substring ES/EN → 4. **clases CSS con
   hash: prohibidas**, mueren en cada deploy.

### Si el search por API falla (token o endpoint nuevo)

1. Corre el diagnóstico, que dice la verdad:
   ```powershell
   cargo test diag_api -- --nocapture
   # DIAG-API: API: sin token (token web status 403) | red: {...}
   ```
   El campo `red` muestra los endpoints que la página SÍ usa hoy
   (`pathfinder/vX/query`, `*.spclient.spotify.com`, `hosts`, keys de
   `localStorage`): ahí está la pista del flujo nuevo, sin adivinar.
2. Actualiza `src/cdp/api.rs` (`web_token()` / `search_with_token()` /
   `parse_search_api()`) y valida con sus tests puros + `diag_api`.
3. Mientras tanto todo sigue funcionando por el fallback DOM; y con
   `RATA_SPOT_API=0` fuerzas DOM directo para depurar.

### Kill-switches y comandos

| Para qué | Comando |
|---|---|
| Suite silenciosa (default) | `cargo test` |
| Tests vivos (suenan 1 min, manos fuera) | `cargo test -- --include-ignored` |
| Apagar capa API | `$env:RATA_SPOT_API=0; cargo run` |
| Brave visible para depurar | quitar `RATA_SPOT_HIDDEN=1` / ver `abrir-spotify.ps1` |

### Mapa rápido (qué vive dónde)

```
src/cdp/selectors.rs  # TODO el DOM + HEALTH_JS + tests de sincronía
src/cdp/api.rs        # fetch en página, token, search JSON, net_probe + tests puros
src/cdp/client.rs     # WS genérico + timeouts (5s conectar / 12s leer)
src/cdp/transport.rs  # HTTP/1.1 + Content-Length/chunked + timeout 8s
src/cdp/tabs.rs       # hallar/crear pestaña + spa_navigate (no mata audio)
src/cdp/search.rs     # API primero, DOM después
src/cdp/tests.rs      # diag_* (imprimen, no revientan) + vivos #[ignore]
src/app/boot.rs       # health_summary() en el status de la TUI
```

Y si todo falla: `diag_media` + `diag_gsmtc` te dicen si el problema es
la página (sin `<audio>`, player wedged) o el sistema (sin sesión
GSMTC). Buena suerte rata 🐀
