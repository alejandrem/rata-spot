# arquitectura 🐀

Mapa completo de archivos y carpetas de rata-spot. Regla del repo:
**1 carpeta = 1 dominio, 1 archivo = 1 tarea**. Los `mod.rs` solo
declaran submódulos y re-exportan; la lógica vive en los archivos hoja.

Flujo en 1 línea: `main` → `app/boot` (sesión Brave) → `app/run`
(loop: dibuja `ui` + teclas `app/keys` + sync `app/sync` por tick) →
`launcher` + `cdp` + `gsmtc` por debajo. Al salir (`q`): pausa GSMTC +
`launcher::cleanup`.

```
src/
  main.rs                        # Punto de entrada delgado: boot → terminal → run_app → cleanup.
  app/                           # Orquestación del loop TUI.
    mod.rs                       # Índice: terminal/boot/keys/sync/library/run.
    boot.rs                      # Arranque en 3 estados + sala de espera (P2) si Brave está sordo.
    run.rs                       # El loop: dibuja + teclado + sync GSMTC por tick + cleanup al salir.
    terminal.rs                  # Setup/restore del terminal (raw + pantalla alternativa).
    library.rs                   # Carga la biblioteca del DOM en fondo (sin bloquear el arranque).
    keys/                        # Teclado crossterm → acciones (filtra Press/Repeat/Release).
      mod.rs                     # Dispatcher: search_active → search_mode, si no por tecla.
      search_mode.rs             # Modo escritura: todo va al query, Enter busca vía cdp::search.
      playback.rs                # space/n/p (solo Press): toggle/next/prev o primer play por CDP.
      navigate.rs                # j/k/flechas con throttle 120ms + l recarga la biblioteca.
      open.rs                    # →/Enter: abre playlist, toca rola o toca resultado + dashboard.
      view_keys.rs               # / enfoca buscador, ←/Esc vuelve, q sale del loop.
    sync/                        # Sync GSMTC por tick (reconexión + refresh sin crashear).
      mod.rs                     # Índice: state/tick/switch.
      state.rs                   # SyncState: reintentos, backoff, seen multi-sesión, últimos buenos.
      tick.rs                    # Un tick: reconexión con backoff + refresh + persistir al cambiar rola.
      switch.rs                  # Cambia a otra sesión solo si flippeó a sonando (anti-oscilación).
  cdp/                           # Control de la página Spotify vía Chrome DevTools Protocol (sin API).
    mod.rs                       # Índice + re-exports públicos (search, library, tracks, health...).
    transport.rs                 # HTTP/1.1 crudo a /json/* (Content-Length/chunked, timeout 8s, sin reqwest).
    client.rs                    # WS genérico {id, method, params} con timeouts (5s conectar / 12s leer).
    tabs.rs                      # Halla/crea la pestaña Spotify + spa_navigate (no mata el audio).
    playback.rs                  # Primer play: click al Play visible ES/EN hasta 15s (flujo de [space]).
    library.rs                   # Lee "Tu biblioteca" virtualizada + diag crudo del DOM.
    search.rs                    # Buscador: API primero, DOM (/search + SEARCH_JS) como fallback.
    track_page.rs                # Dashboard de la rola: header + letra desde section[track-page].
    api/                         # Capa JSON/red dentro de la página (con fallback al DOM).
      mod.rs                     # Índice: search_via_api + api_diag públicos, resto pub(crate).
      fetch.rs                   # fetch DENTRO de la página (cookies/login) → (status, body).
      token.rs                   # Token web del player + encode mínimo para URLs.
      search.rs                  # Search /v1/search con token: tracks primero, cap 24.
      probe.rs                   # net_probe (qué endpoint existe hoy) + api_diag (nunca revienta).
      tests.rs                   # Tests puros: encode, escape de fetch, parse de /v1/search.
    selectors/                   # TODO el DOM en un solo lugar + health-check.
      mod.rs                     # Índice + re-exports (health_summary público, JS pub(crate)).
      names.rs                   # Consts canónicas (data-testid/aria/ids: se cambian AQUÍ).
      player.rs                  # PLAYER_ARIA_JS + PLAY_CLICK_JS + click a fila exacta por track id.
      tracklist.rs               # TRACKLIST_EXISTS/SCROLL_TOP/SCROLL + TRACKS_JS (playlist Y álbum).
      library_js.rs              # LIBRARY_JS (scroll sidebar + dedupe por URI) + LIBRARY_DIAG_JS.
      search_js.rs               # SEARCH_JS: anchors /track|artist|playlist|album fuera del sidebar.
      dashboard_js.rs            # DASHBOARD_JS: header + letra del track-page.
      page.rs                    # PAGE_STATE_JS: título + hay player + hay login.
      health.rs                  # HEALTH_JS (6 sondas) + health_summary + summarize_health (1 línea).
      tests.rs                   # Test de sincronía: el JS debe mencionar sus consts + resumen health.
    tracks/                      # Canciones de la colección + tocar por track.
      mod.rs                     # Índice + struct TrackItem (n/title/artist/duration/id).
      open.rs                    # open_page (SPA sin matar audio) + open_playlist (espera tracklist 10s).
      read.rs                    # playlist_tracks: snapshot con scroll del grid virtualizado (30s).
      play_uri.rs                # play_uri: navega al URI + ESPACIO real trusted + espera GSMTC.
      play_track.rs              # play_track: click a LA fila exacta por id (40 intentos con scroll).
    tests/                       # Pruebas vivas/diag contra el Brave real (ver mantenimiento.md).
      mod.rs                     # Índice de suites vivas + diag.
      playback_live.rs           # space_inicia_musica + track_se_reproduce (RUIDOSOS, #[ignore]).
      search_live.rs             # busqueda_funciona + busqueda_no_corta_musica (RUIDOSOS, #[ignore]).
      dashboard_live.rs          # track_detail_se_lee: header + letra (#[ignore]).
      library_live.rs            # playlists_se_listan + playlist_tracks_se_leen (leen, no tocan).
      diag_dom.rs                # diag_estado + diag_selectores + diag_api (imprimen, siempre pasan).
      diag_media.rs              # diag_media + diag_space + diag_play_buttons + diag_gsmtc (imprimen).
  gsmtc/                         # Backend Windows GSMTC (título/progreso + play/pause/next/prev).
    mod.rs                       # Índice + re-exports (sesiones, track, smoother, historial).
    track.rs                     # Tipos TrackInfo/Session + timespan→Duration + format_time.
    history.rs                   # Guarda/carga la última rola en disco (UTF-8, se muestra al arrancar).
    session/                     # Sesión Brave vía windows-rs 0.58 (.get() bloqueante, ver bug #2).
      mod.rs                     # Índice: find/pick/read/control.
      find.rs                    # get_brave_session (prefiere la SONANDO) + brave_sessions (todas).
      pick.rs                    # pick_session: hint del historial > sonando > primera (anti-Facebook).
      read.rs                    # get_track: título/artista/álbum + playing + posición/duración→progreso.
      control.rs                 # play/pause/next/prev/toggle sobre una sesión.
    smoother/                    # Progreso suave (interpola ráfagas GSMTC con el reloj local).
      mod.rs                     # Índice + struct ProgressSmoother + new.
      update.rs                  # update: free-run si GSMTC clavado <500ms, resincroniza si salto real.
      display.rs                 # last_known + smoothed_position/progress + display_track + restore.
      tests.rs                   # Tests: free-run con 39ms clavados, pausa congela, salto resincroniza.
  launcher/                      # Brave minimizado con flags de dieta + cierre quirúrgico.
    mod.rs                       # Índice: config/ports/process/window.
    config/                      # URL, flags y localización del navegador (sin C:\ quemado).
      mod.rs                     # SPOTIFY_URL + BRAVE_FLAGS_BASE + KNOWN_EXES + exe_bases().
      find.rs                    # find_browser_exe: env → where → registro → conocidas → portable (Brave primero).
      lookup.rs                  # where_lookup (PATH) + reg_app_path (HKLM/HKCU App Paths, sin crates).
      candidates.rs              # candidates_from_env: %ProgramFiles%/%LOCALAPPDATA% + scoop + portable ./.
      running.rs                 # is_brave_running SOLO brave.exe (boot) + is_browser_running (diag). Ver bug #28.
      dirs.rs                    # data_dir (LOCALAPPDATA→USERPROFILE→temp) + perfil aislado (reservado).
    ports.rs                     # Puerto CDP efímero + origins a 127.0.0.1 (P0/P1) + RATA_SPOT_PORT.
    process.rs                   # launch_brave_spotify (--new-window + debug_flags + HWND) + cleanup al salir.
    window.rs                    # Snapshot EnumWindows (Chrome_WidgetWin_1) + WM_CLOSE solo a lo nuestro.
  ui/                            # TUI Ratatui: sidebar + player + centro por vista.
    mod.rs                       # render raíz: split 32% sidebar | reproductor.
    state/                       # Estado + navegación con wrap/throttle.
      mod.rs                     # Índice + impls pl/tr/sr/lyr que delegan a cada nav.
      view.rs                    # View (Library/Tracks/Search/Detail) + AppState::new + nav_ok + backs.
      library_nav.rs             # pl_move/pl_selected (sidebar, j/k con wrap).
      tracks_nav.rs              # tr_move/tr_selected (canciones de la playlist).
      search_nav.rs              # sr_move/sr_selected (resultados del buscador).
      detail_nav.rs              # lyr_move (scroll de la letra con tope).
    sidebar.rs                   # Panel izquierdo: Tu biblioteca (j/k + Enter).
    player.rs                    # Header + Gauge de progreso + centro + footer de teclas.
    search.rs                    # Barra SIEMPRE visible + lista de resultados (tracks primero).
    tracks.rs                    # Centro en vista Tracks: tracklist de la playlist abierta.
    detail.rs                    # Centro en vista Detail: header del track + letra con scroll.

docs/
  plan de implementacion.md      # Plan original Fase 1-5 (setup, Brave, GSMTC, TUI, RAM).
  bugs-resueltos-uwu.md          # Bitácora #1-28 (el #28 es la sala fantasma por Edge, anti-regresión).
  mantenimiento.md               # Las 3 capas CDP + flujos + runbook cuando Spotify mueva el DOM.
  arquitectura.md                # Este archivo: mapa 1-carpeta-1-dominio, 1-archivo-1-tarea.
```

Notas que evitan los próximos bugs:

- `boot.rs` SOLO se bloquea por `brave.exe` (`is_brave_running`); Chrome/Edge
  van por `is_browser_running` y jamás entran al boot (bug #28).
- El DOM vive en `cdp/selectors/`; si Spotify rediseña, se toca UNA const en
  `names.rs` y `cargo test` grita si el JS quedó divorciado (`selectors/tests.rs`).
- `cdp/api/` es experimental con kill-switch `RATA_SPOT_API=0`; si el token o
  el endpoint mueren, todo cae al DOM y la música sigue (ver `mantenimiento.md`).
- `Input.dispatchKeyEvent` (Espacio real trusted) sí arranca audio; los
  `.click()` por JS son untrusted y a veces no suenan (bugs #9/#18).
- windows-rs 0.58: `IAsyncOperation` NO es `Future`, se usa `.get()` (bug #2);
  `GetSessions()` exige el feature `Foundation_Collections` (bug #3).
